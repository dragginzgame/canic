//! Module: checkpoint::driver
//!
//! Responsibility: account one original host ingress per frozen Root checkpoint transition.
//! Does not own: signing, authentication, Root-paid management calls or application settlement.
//! Boundary: replies settle host ingress only; pending histories never authorize redispatch.

use crate::backup::checkpoint::{
    CheckpointError, CheckpointIngress, CheckpointUnitRecord, FrozenCheckpoint, model::RECORD_LIMIT,
};

use std::future::Future;

use ic_backup::{
    model::{
        application_ingress::ApplicationMutationRequest,
        artifacts::ArtifactChecksumRecord as Digest,
        attempt_journal::{AttemptJournalRecord, MutationOutcomeRecord, MutationReceiptRequest},
        execution_workflow::{ExecutionStagePredecessorRecord, ExecutionWorkflowRecord},
        operation_plan::OperationPlanRecord,
    },
    ops::persistence::{AttemptJournalGuard, BackupLayoutGuard, ExecutionStageGuard},
    workflow::execution_workflow::ExecutionWorkflowStageDriver,
};
use thiserror::Error;

const INTENT_FILE: &str = "checkpoint-intent.json";

impl FrozenCheckpoint {
    /// Create-only original unit publication before workflow/stage allocation.
    /// Retain this record and every original workflow/journal across interruption.
    pub fn retain(&self, layout: &BackupLayoutGuard) -> Result<(), CheckpointError> {
        let bytes = serde_json::to_vec(&self.record).map_err(CheckpointError::accounting)?;
        if bytes.len() > RECORD_LIMIT {
            return Err(CheckpointError::UnitMismatch);
        }
        ic_host_fs::durable::write_bytes(
            &layout.root().join(INTENT_FILE),
            ic_host_fs::durable::WriteOptions {
                mode: ic_host_fs::durable::PublicationMode::CreateNew,
                permissions: 0o600,
            },
            &bytes,
        )?;
        Ok(())
    }
    /// Reconstruct declarations from the exact original bounded, no-follow record.
    /// This does not create a workflow/journal or restore any consumed allowance.
    pub fn read(layout: &BackupLayoutGuard) -> Result<Self, CheckpointError> {
        let bytes =
            ic_host_fs::read::read_file_no_follow(&layout.root().join(INTENT_FILE), RECORD_LIMIT)
                .map_err(CheckpointError::accounting)?;
        let record: CheckpointUnitRecord =
            serde_json::from_slice(&bytes).map_err(CheckpointError::accounting)?;
        Self::new(record)
    }
    pub(super) fn check(&self, layout: &BackupLayoutGuard) -> Result<(), CheckpointError> {
        let original = Self::read(layout)?;
        if original.record != self.record
            || original.workflow().digest() != self.workflow().digest()
        {
            return Err(CheckpointError::OriginalMismatch);
        }
        Ok(())
    }
}

/// Mandatory integration boundary for independently admitted original host ingress.
///
/// `admit` qualifies actual network/caller/release, controller authentication, exact
/// Root installation and complete current unit. `submit` durably retains the exact
/// signed bytes/request ID or exclusive original command witness before one update;
/// it must not retry, poll, observe, pay for management calls or infer receipts.
/// `qualify` independently authenticates this original reply and durably retains
/// request/reply bytes before awaiting cancellable work. Its digest settles only
/// this host ingress. Root replay/cost and paid-effect evidence remain separate.
/// All additional remote calls need prior original accounting of their own.
pub trait CheckpointProvider {
    type Error: std::error::Error + 'static;
    fn admit(
        &mut self,
        request: &CheckpointIngress,
        unit: &CheckpointUnitRecord,
    ) -> impl Future<Output = Result<(), Self::Error>>;
    fn submit(
        &mut self,
        request: &CheckpointIngress,
        reserved: &ApplicationMutationRequest<'_>,
        journal: &AttemptJournalRecord,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>>;
    fn qualify(
        &mut self,
        request: &CheckpointIngress,
        reply: &[u8],
    ) -> impl Future<Output = Result<Digest, Self::Error>>;
}

/// IC Backup stage driver for one retained complete application checkpoint unit.
/// Use the existing new/retained workflow coordinators; do not add a parallel loop.
pub struct CheckpointDriver<'a, P> {
    layout: &'a BackupLayoutGuard,
    intent: FrozenCheckpoint,
    provider: P,
}
impl<'a, P> CheckpointDriver<'a, P> {
    pub fn new(
        layout: &'a BackupLayoutGuard,
        intent: FrozenCheckpoint,
        provider: P,
    ) -> Result<Self, CheckpointError> {
        intent.check(layout)?;
        Ok(Self {
            layout,
            intent,
            provider,
        })
    }
    pub const fn provider(&self) -> &P {
        &self.provider
    }
}

impl<P: CheckpointProvider> ExecutionWorkflowStageDriver for CheckpointDriver<'_, P> {
    type Error = CheckpointExecutionError<P::Error>;

    fn bind_stage(
        &mut self,
        workflow: &ExecutionWorkflowRecord,
        sequence: u64,
        _: &[ExecutionStagePredecessorRecord],
    ) -> Result<OperationPlanRecord, Self::Error> {
        self.intent.check(self.layout)?;
        if workflow != self.intent.workflow() {
            return Err(CheckpointError::OriginalMismatch.into());
        }
        Ok(self.intent.child(sequence)?)
    }

    #[expect(
        clippy::future_not_send,
        reason = "IC Backup and application providers deliberately impose no executor or Send contract"
    )]
    async fn execute_stage(
        &mut self,
        stage: &ExecutionStageGuard<'_>,
    ) -> Result<Digest, Self::Error> {
        let sequence = stage.binding().stage_sequence();
        self.intent.check(self.layout)?;
        if stage.plan() != &self.intent.child(sequence)?
            || stage.binding().workflow() != &self.intent.workflow().digest()
        {
            return Err(CheckpointError::OriginalMismatch.into());
        }
        let request = self.intent.request(sequence)?;
        let authority = stage
            .plan()
            .attempt_authority(sequence)
            .map_err(CheckpointError::accounting)?;
        request
            .payload()
            .validate_mutation_binding(authority.binding())
            .map_err(CheckpointError::accounting)?;
        let mut journal = AttemptJournalGuard::open(
            stage.layout().map_err(CheckpointError::accounting)?,
            &authority,
        )
        .map_err(CheckpointError::accounting)?;
        let attempt = journal
            .reserve_planned_mutation(&stage.plan().digest())
            .map_err(CheckpointError::accounting)?;
        self.provider
            .admit(request, &self.intent.record)
            .await
            .map_err(CheckpointExecutionError::Admission)?;
        self.intent.check(self.layout)?;
        stage.layout().map_err(CheckpointError::accounting)?;
        let reserved = ApplicationMutationRequest::new(
            stage.plan(),
            sequence,
            journal.record().map_err(CheckpointError::accounting)?,
            request.payload(),
        )
        .map_err(CheckpointError::accounting)?;
        let reply = self
            .provider
            .submit(
                request,
                &reserved,
                journal.record().map_err(CheckpointError::accounting)?,
            )
            .await
            .map_err(CheckpointExecutionError::Submit)?;
        let settle = async {
            request.decode(&reply)?;
            self.intent.check(self.layout)?;
            stage.layout().map_err(CheckpointError::accounting)?;
            let evidence = self
                .provider
                .qualify(request, &reply)
                .await
                .map_err(CheckpointReplyError::Qualification)?;
            self.intent.check(self.layout)?;
            stage.layout().map_err(CheckpointError::accounting)?;
            journal
                .record_mutation(MutationReceiptRequest {
                    attempt,
                    request: request.binding().hash().into(),
                    outcome: MutationOutcomeRecord::Applied,
                    evidence: evidence.hash().into(),
                })
                .map_err(CheckpointError::accounting)?;
            Ok::<_, CheckpointReplyError<P::Error>>(())
        }
        .await;
        if let Err(source) = settle {
            return Err(CheckpointExecutionError::AfterReply { source, reply });
        }
        drop(journal);
        let learned = Digest::from_bytes(&reply);
        stage
            .reconcile_checkpoint(learned.clone())
            .map_err(|error| CheckpointExecutionError::AfterReply {
                source: CheckpointReplyError::Local(CheckpointError::accounting(error)),
                reply,
            })?;
        Ok(learned)
    }
}

/// Failure preserves original spending and returned bytes; it never grants a retry.
#[derive(Debug, Error)]
pub enum CheckpointExecutionError<E> {
    #[error(transparent)]
    Local(#[from] CheckpointError),
    #[error("fresh checkpoint admission failed: {0}")]
    Admission(#[source] E),
    #[error("original checkpoint ingress outcome is indeterminate: {0}")]
    Submit(#[source] E),
    #[error("checkpoint reply could not settle: {source}")]
    AfterReply {
        source: CheckpointReplyError<E>,
        reply: Vec<u8>,
    },
}

/// Returned evidence remains available after qualification/local settlement rejection.
#[derive(Debug, Error)]
pub enum CheckpointReplyError<E> {
    #[error(transparent)]
    Local(#[from] CheckpointError),
    #[error("checkpoint reply qualification failed: {0}")]
    Qualification(#[source] E),
}
