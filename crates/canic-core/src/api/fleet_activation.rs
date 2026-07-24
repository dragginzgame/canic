//! Module: api::fleet_activation
//!
//! Responsibility: expose Fleet activation workflows and canonical evidence hashes.
//! Does not own: evidence encoding, storage projection, phase validation, or authorization.
//! Boundary: maps typed internal failures into Canic's public error contract.

use crate::{
    dto::{
        error::Error,
        fleet_activation::{
            FleetActivationIdentity, FleetActivationRequest, FleetActivationResumeRequest,
            FleetActivationStatusResponse, FleetCascadeActivationEvidence,
            FleetCascadeManifestEntry, FleetCredentialGenerationRef,
            FleetCredentialGenerationRequest, FleetCredentialManifest,
        },
    },
    ops::fleet_activation::FleetActivationEvidenceOps,
    view::fleet_activation::FleetActivationTransition,
    workflow::runtime::fleet_activation::FleetActivationWorkflow,
};

///
/// FleetActivationApi
///

pub struct FleetActivationApi;

impl FleetActivationApi {
    /// Hash one canonical root cascade manifest for host/runtime evidence comparison.
    pub fn cascade_manifest_hash(
        manifest: &[FleetCascadeManifestEntry],
    ) -> Result<[u8; 32], Error> {
        FleetActivationEvidenceOps::cascade_manifest_hash(manifest).map_err(Error::from)
    }

    /// Hash one canonical credential manifest for host/runtime evidence comparison.
    pub fn credential_manifest_hash(manifest: &FleetCredentialManifest) -> Result<[u8; 32], Error> {
        FleetActivationEvidenceOps::credential_manifest_hash(manifest).map_err(Error::from)
    }

    /// Hash one Canister's exact activation identity and accepted evidence.
    pub fn activation_evidence_hash(
        identity: &FleetActivationIdentity,
        cascade: &FleetCascadeActivationEvidence,
        credential: FleetCredentialGenerationRef,
    ) -> Result<[u8; 32], Error> {
        FleetActivationEvidenceOps::activation_evidence_hash(identity, cascade, credential)
            .map_err(Error::from)
    }

    pub fn status() -> Result<FleetActivationStatusResponse, Error> {
        FleetActivationWorkflow::status().map_err(Error::from)
    }

    pub fn require_active() -> Result<(), Error> {
        FleetActivationWorkflow::require_active().map_err(Error::from)
    }

    pub async fn prepare_root() -> Result<FleetActivationStatusResponse, Error> {
        FleetActivationWorkflow::prepare_root()
            .await
            .map_err(Error::from)
    }

    pub async fn resume_root(
        request: FleetActivationResumeRequest,
    ) -> Result<FleetActivationTransition, Error> {
        FleetActivationWorkflow::resume_root(request)
            .await
            .map_err(Error::from)
    }

    pub fn prepare_nonroot_credential_generation(
        request: FleetCredentialGenerationRequest,
    ) -> Result<FleetActivationStatusResponse, Error> {
        FleetActivationWorkflow::prepare_nonroot_credential_generation(request).map_err(Error::from)
    }

    pub fn activate_nonroot(
        request: FleetActivationRequest,
    ) -> Result<FleetActivationTransition, Error> {
        FleetActivationWorkflow::activate_nonroot(request).map_err(Error::from)
    }
}
