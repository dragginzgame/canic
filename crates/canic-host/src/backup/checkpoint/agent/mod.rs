//! Bind original Canic checkpoint reservations to Backup's single-submit Agent owner.
//!
//! Fresh whole-unit authority and durable custody remain mandatory integration inputs.
//! This adapter grants neither Root-paid effects nor automatic live-create admission.

use super::{
    CheckpointIngress, CheckpointObservation, CheckpointProvider, CheckpointRecoveryProvider,
    CheckpointUnitRecord,
};
use ic_backup::model::{
    application_ingress::{
        ApplicationIngressPayload, ApplicationMutationRequest, ApplicationObservationRequest,
    },
    artifacts::ArtifactChecksumRecord as Digest,
    attempt_journal::AttemptJournalRecord,
};
use ic_backup_agent::{
    AgentApplicationIngressProvider, AgentTransport, ApplicationIngressProviderError,
    ApplicationIngressResponse, PreparedUpdate, ReservedUpdate,
};
use ic_platform::agent::RequestId;
use std::future::Future;
use thiserror::Error;

/// Qualify actual network, signer, release, controllers and complete current unit.
///
/// Implementations must refuse stale/changed authority. Any remote admission call
/// requires separately accounted original allowance; this trait supplies none.
/// Transport context declarations and a decoded reply are not admission evidence.
pub trait CheckpointAdmission {
    type Error: std::error::Error + 'static;
    fn admit(
        &mut self,
        unit: &CheckpointUnitRecord,
        payload: &ApplicationIngressPayload,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

/// Original single-submit transport with mandatory fresh admission and durable retainers.
///
/// Ingress retention must synchronously claim the original authority/kind/attempt
/// before dispatch and reject any prior or uncertain witness, including changed
/// signed expiry. Reply retention must synchronously persist exact bytes and ID.
/// Neither callback may issue remote effects. There are no permissive defaults.
pub struct AgentCheckpointProvider<'a, A, I, R> {
    admission: A,
    transport: AgentApplicationIngressProvider<'a, I, R>,
    response: Option<ApplicationIngressResponse>,
}

impl<'a, A, I, R> AgentCheckpointProvider<'a, A, I, R> {
    #[must_use]
    pub const fn new(
        transport: &'a AgentTransport,
        admission: A,
        retain_ingress: I,
        retain_reply: R,
    ) -> Self {
        Self {
            admission,
            transport: AgentApplicationIngressProvider::new(
                transport,
                retain_ingress,
                retain_reply,
            ),
            response: None,
        }
    }

    fn retain_response(&mut self, response: ApplicationIngressResponse) -> Vec<u8> {
        let bytes = response.reply().to_vec();
        self.response = Some(response);
        bytes
    }

    fn qualify_reply<E>(
        &mut self,
        request: &Digest,
        bytes: &[u8],
    ) -> Result<Digest, AgentCheckpointError<E>> {
        let response = self
            .response
            .take()
            .ok_or(AgentCheckpointError::ReplyMismatch)?;
        if response.request() != request || response.reply() != bytes {
            return Err(AgentCheckpointError::ReplyMismatch);
        }
        Ok(response.evidence().clone())
    }
}

/// A refusal or uncertain outcome never authorizes resubmission or budget replacement.
#[derive(Debug, Error)]
pub enum AgentCheckpointError<E> {
    #[error("fresh checkpoint authority admission failed: {0}")]
    Admission(#[source] E),
    #[error(transparent)]
    Transport(#[from] ApplicationIngressProviderError),
    #[error("checkpoint reply differs from the retained authenticated transport response")]
    ReplyMismatch,
}

impl<A, I, R> CheckpointProvider for AgentCheckpointProvider<'_, A, I, R>
where
    A: CheckpointAdmission,
    I: FnMut(
        &ReservedUpdate<'_>,
        &PreparedUpdate<'_>,
    ) -> Result<(), ApplicationIngressProviderError>,
    R: FnMut(&ReservedUpdate<'_>, &RequestId, &[u8]) -> Result<(), ApplicationIngressProviderError>,
{
    type Error = AgentCheckpointError<A::Error>;
    async fn admit(
        &mut self,
        request: &CheckpointIngress,
        unit: &CheckpointUnitRecord,
    ) -> Result<(), Self::Error> {
        self.response = None;
        self.admission
            .admit(unit, request.payload())
            .await
            .map_err(AgentCheckpointError::Admission)
    }
    async fn submit(
        &mut self,
        request: &CheckpointIngress,
        reserved: &ApplicationMutationRequest<'_>,
        journal: &AttemptJournalRecord,
    ) -> Result<Vec<u8>, Self::Error> {
        self.response = None;
        if reserved.payload().digest() != *request.binding() {
            return Err(AgentCheckpointError::ReplyMismatch);
        }
        let response = self.transport.submit_mutation(reserved, journal).await?;
        Ok(self.retain_response(response))
    }
    fn qualify(
        &mut self,
        request: &CheckpointIngress,
        reply: &[u8],
    ) -> impl Future<Output = Result<Digest, Self::Error>> {
        std::future::ready(self.qualify_reply(request.binding(), reply))
    }
}

impl<A, I, R> CheckpointRecoveryProvider for AgentCheckpointProvider<'_, A, I, R>
where
    A: CheckpointAdmission,
    I: FnMut(
        &ReservedUpdate<'_>,
        &PreparedUpdate<'_>,
    ) -> Result<(), ApplicationIngressProviderError>,
    R: FnMut(&ReservedUpdate<'_>, &RequestId, &[u8]) -> Result<(), ApplicationIngressProviderError>,
{
    type Error = AgentCheckpointError<A::Error>;
    async fn admit(
        &mut self,
        request: &CheckpointObservation,
        unit: &CheckpointUnitRecord,
    ) -> Result<(), Self::Error> {
        self.response = None;
        self.admission
            .admit(unit, request.payload())
            .await
            .map_err(AgentCheckpointError::Admission)
    }
    async fn observe(
        &mut self,
        request: &CheckpointObservation,
        reserved: &ApplicationObservationRequest<'_>,
        journal: &AttemptJournalRecord,
    ) -> Result<Vec<u8>, Self::Error> {
        self.response = None;
        if reserved.payload().digest() != *request.binding() {
            return Err(AgentCheckpointError::ReplyMismatch);
        }
        let response = self.transport.submit_observation(reserved, journal).await?;
        Ok(self.retain_response(response))
    }
    fn qualify(
        &mut self,
        request: &CheckpointObservation,
        reply: &[u8],
    ) -> impl Future<Output = Result<Digest, Self::Error>> {
        std::future::ready(self.qualify_reply(request.binding(), reply))
    }
}
