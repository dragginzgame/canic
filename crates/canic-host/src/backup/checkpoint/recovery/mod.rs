//! Module: checkpoint::recovery
//!
//! Responsibility: reconcile one original Root ingress through its reserved status observation.
//! Does not own: transport authentication, Root-paid effects or generic recovery scheduling.
//! Boundary: observed status never grants new mutation authority or refunds consumed budgets.

use super::{
    CheckpointError, CheckpointIngress, CheckpointReplyError, CheckpointUnitRecord,
    FrozenCheckpoint,
};
use canic_contracts::{
    dto::application_checkpoint::RootCheckpointStatus, protocol::application_checkpoint as protocol,
};
use ic_backup::{
    model::{
        application_ingress::{ApplicationIngressPayload, ApplicationObservationRequest},
        artifacts::ArtifactChecksumRecord as Digest,
        attempt_journal::{
            AttemptJournalRecord, ObservationOutcomeRecord, ObservationReceiptRequest,
        },
    },
    ops::persistence::{AttemptJournalGuard, BackupLayoutGuard, ExecutionStageGuard},
    workflow::execution_workflow::RetainedStageEvidence,
};
use std::future::Future;
use thiserror::Error;

/// Exact read-only status update bound to the original workflow and mutation attempt.
#[derive(Clone, Debug)]
pub struct CheckpointObservation {
    original: CheckpointIngress,
    payload: ApplicationIngressPayload,
    binding: Digest,
    mutation_attempt: u32,
}

impl CheckpointObservation {
    fn new(
        intent: &FrozenCheckpoint,
        sequence: u64,
        mutation_attempt: u32,
    ) -> Result<Self, CheckpointError> {
        let original = intent.request(sequence)?.clone();
        let arguments = ic_wire::native::encode_request(protocol::STATUS, &(), protocol::LIMITS)
            .map_err(CheckpointError::from)?;
        let payload = ApplicationIngressPayload::new(
            original.receiver(),
            original.receiver(),
            "canic_root_checkpoint_status",
            &arguments,
            protocol::LIMITS.bytes,
        )
        .map_err(CheckpointError::accounting)?;
        let binding = payload.digest();
        Ok(Self {
            original,
            payload,
            binding,
            mutation_attempt,
        })
    }

    #[must_use]
    pub fn receiver(&self) -> &str {
        self.original.receiver()
    }
    #[must_use]
    pub const fn method(&self) -> &'static str {
        "canic_root_checkpoint_status"
    }
    #[must_use]
    pub fn arguments(&self) -> &[u8] {
        self.payload.arguments()
    }
    #[must_use]
    pub const fn binding(&self) -> &Digest {
        &self.binding
    }
    #[must_use]
    pub const fn mutation_attempt(&self) -> u32 {
        self.mutation_attempt
    }
    #[must_use]
    pub const fn original(&self) -> &CheckpointIngress {
        &self.original
    }

    /// Exact opaque replicated observation; original authority is bound by its reservation.
    #[must_use]
    pub const fn payload(&self) -> &ApplicationIngressPayload {
        &self.payload
    }

    /// Admit the exact original transition, not merely a current prepared checkpoint.
    /// This checks shape/identity only; the provider must independently qualify evidence.
    pub fn decode(&self, bytes: &[u8]) -> Result<RootCheckpointStatus, CheckpointError> {
        let status: Result<Option<RootCheckpointStatus>, canic_contracts::dto::error::Error> =
            ic_wire::native::decode_response(protocol::STATUS, bytes, protocol::LIMITS)?;
        let status = status
            .map_err(|_| CheckpointError::ReplyMismatch)?
            .ok_or(CheckpointError::ReplyMismatch)?;
        if &status != self.original.expected_status() {
            return Err(CheckpointError::ReplyMismatch);
        }
        Ok(status)
    }
}

/// Mandatory fresh admission, original observation custody and independent reply qualification.
///
/// No method may repeat the mutation. Additional calls need their own original allowance.
/// Observe retains exact request/reply evidence before returning; qualify retains its evidence
/// before cancellable work. Replicated invocation alone does not qualify application attribution.
pub trait CheckpointRecoveryProvider {
    type Error: std::error::Error + 'static;
    fn admit(
        &mut self,
        request: &CheckpointObservation,
        unit: &CheckpointUnitRecord,
    ) -> impl Future<Output = Result<(), Self::Error>>;
    fn observe(
        &mut self,
        request: &CheckpointObservation,
        reserved: &ApplicationObservationRequest<'_>,
        journal: &AttemptJournalRecord,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>>;
    fn qualify(
        &mut self,
        request: &CheckpointObservation,
        reply: &[u8],
    ) -> impl Future<Output = Result<Digest, Self::Error>>;
}

/// Settle one pending original mutation with one separately reserved observation.
///
/// Missing, occupied, exhausted or already applied authority refuses without provider calls.
/// Root-paid effect accounting and terminal/reference release remain separate.
#[expect(
    clippy::future_not_send,
    reason = "Backup providers impose no executor or Send contract"
)]
pub async fn reconcile_checkpoint<P: CheckpointRecoveryProvider>(
    layout: &BackupLayoutGuard,
    intent: &FrozenCheckpoint,
    stage: &ExecutionStageGuard<'_>,
    provider: &mut P,
) -> Result<RetainedStageEvidence, CheckpointRecoveryError<P::Error>> {
    intent.check(layout)?;
    let sequence = stage.binding().stage_sequence();
    if stage.plan() != &intent.child(sequence)?
        || stage.binding().workflow() != &intent.workflow().digest()
    {
        return Err(CheckpointError::OriginalMismatch.into());
    }
    let authority = stage
        .plan()
        .attempt_authority(sequence)
        .map_err(CheckpointError::accounting)?;
    let mut journal = AttemptJournalGuard::open(
        stage.layout().map_err(CheckpointError::accounting)?,
        &authority,
    )
    .map_err(CheckpointError::accounting)?;
    let mutation_attempt = journal
        .record()
        .map_err(CheckpointError::accounting)?
        .view()
        .pending_mutation
        .ok_or(CheckpointError::OriginalMismatch)?;
    let request = CheckpointObservation::new(intent, sequence, mutation_attempt)?;
    request
        .payload()
        .validate_observation_binding(authority.binding())
        .map_err(CheckpointError::accounting)?;
    let attempt = journal
        .reserve_planned_observation(
            &stage.plan().digest(),
            mutation_attempt,
            request.binding().hash(),
        )
        .map_err(CheckpointError::accounting)?;
    provider
        .admit(&request, &intent.record)
        .await
        .map_err(CheckpointRecoveryError::Admission)?;
    intent.check(layout)?;
    stage.layout().map_err(CheckpointError::accounting)?;
    let reserved = ApplicationObservationRequest::new(
        stage.plan(),
        sequence,
        journal.record().map_err(CheckpointError::accounting)?,
        request.payload(),
    )
    .map_err(CheckpointError::accounting)?;
    let reply = provider
        .observe(
            &request,
            &reserved,
            journal.record().map_err(CheckpointError::accounting)?,
        )
        .await
        .map_err(CheckpointRecoveryError::Observation)?;
    let settlement = async {
        request.decode(&reply)?;
        intent.check(layout)?;
        stage.layout().map_err(CheckpointError::accounting)?;
        let evidence = provider
            .qualify(&request, &reply)
            .await
            .map_err(CheckpointReplyError::Qualification)?;
        intent.check(layout)?;
        stage.layout().map_err(CheckpointError::accounting)?;
        journal
            .record_observation(ObservationReceiptRequest {
                attempt,
                request: request.binding().hash().into(),
                outcome: ObservationOutcomeRecord::Applied,
                evidence: evidence.hash().into(),
            })
            .map_err(CheckpointError::accounting)?;
        Ok::<_, CheckpointReplyError<P::Error>>(())
    }
    .await;
    if let Err(source) = settlement {
        return Err(CheckpointRecoveryError::AfterReply { source, reply });
    }
    drop(journal);
    let learned = Digest::from_bytes(&reply);
    stage
        .reconcile_checkpoint(learned.clone())
        .map_err(|error| CheckpointRecoveryError::AfterReply {
            source: CheckpointReplyError::Local(CheckpointError::accounting(error)),
            reply,
        })?;
    Ok(RetainedStageEvidence::new(
        stage.binding().digest(),
        learned,
    ))
}

/// Original uncertainty and returned bytes survive failure; no variant grants reissue.
#[derive(Debug, Error)]
pub enum CheckpointRecoveryError<E> {
    #[error(transparent)]
    Local(#[from] CheckpointError),
    #[error("checkpoint recovery admission failed: {0}")]
    Admission(#[source] E),
    #[error("original checkpoint observation outcome is indeterminate: {0}")]
    Observation(#[source] E),
    #[error("checkpoint observation could not settle: {source}")]
    AfterReply {
        source: CheckpointReplyError<E>,
        reply: Vec<u8>,
    },
}
