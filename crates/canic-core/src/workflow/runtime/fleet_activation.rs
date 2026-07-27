//! Module: workflow::runtime::fleet_activation
//!
//! Responsibility: expose the canonical protected Fleet activation status.
//! Does not own: stable conversion, endpoint authorization, or activation mutation.
//! Boundary: the runtime role selects root-only projection before ops validates the record.

use crate::{
    InternalError, InternalErrorOrigin,
    domain::policy::pure::{
        PolicyError,
        fleet_activation::{require_prepared_nonroot_endpoint, require_prepared_root_endpoint},
    },
    dto::{
        cascade::{StateSnapshotInput, TopologySnapshotInput},
        fleet_activation::{
            FleetActivationPhase, FleetActivationRequest, FleetActivationResumeRequest,
            FleetActivationStatusResponse, FleetCascadeActivationEvidence,
            FleetCascadeManifestEntry, FleetCredentialGenerationRef,
            FleetCredentialGenerationRequest, FleetCredentialManifest,
        },
        fleet_subnet_root::FleetSubnetRootAuthority,
    },
    ids::EndpointCall,
    ops::{
        cascade::CascadeOps,
        fleet_activation::FleetActivationEvidenceOps,
        ic::IcOps,
        rpc::RpcOps,
        runtime::{env::EnvOps, fleet_activation::FleetActivationRuntimeOps},
        storage::{
            StorageOpsError, auth::AuthStateOps, fleet_activation::FleetActivationOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
    protocol,
    view::fleet_activation::FleetActivationTransition,
    view::topology::RegisteredCanisterView,
    workflow::cascade::{
        snapshot::{StateSnapshotBuilder, adapter::StateSnapshotAdapter},
        state::StateCascadeWorkflow,
        topology::TopologyCascadeWorkflow,
    },
};
use std::collections::BTreeSet;

///
/// FleetActivationWorkflow
///

pub struct FleetActivationWorkflow;

impl FleetActivationWorkflow {
    pub fn status() -> Result<FleetActivationStatusResponse, InternalError> {
        FleetActivationOps::status(EnvOps::is_root())
            .map_err(StorageOpsError::from)
            .map_err(Into::into)
    }

    pub fn require_active() -> Result<(), InternalError> {
        FleetActivationOps::require_active(EnvOps::is_root())
            .map_err(StorageOpsError::from)
            .map_err(Into::into)
    }

    pub fn root_authority() -> Result<FleetSubnetRootAuthority, InternalError> {
        EnvOps::require_root()?;
        FleetActivationOps::root_authority()
            .map_err(StorageOpsError::from)
            .map_err(Into::into)
    }

    pub async fn prepare_root() -> Result<FleetActivationStatusResponse, InternalError> {
        EnvOps::require_root()?;
        let current = Self::status()?;
        if current.phase == FleetActivationPhase::Active {
            return Ok(current);
        }
        require_empty_prepared_credential_authority()?;

        let root_pid = IcOps::canister_self();
        let mut children = SubnetRegistryOps::direct_child_registrations(root_pid);
        children.sort_by(|left, right| left.pid.as_slice().cmp(right.pid.as_slice()));
        require_direct_activation_inventory(root_pid, &children)?;

        let state_snapshot = StateSnapshotBuilder::new()?
            .with_fleet_state()
            .with_fleet_directory()?
            .with_subnet_directory()?
            .build();
        let state_input = StateSnapshotAdapter::to_input(&state_snapshot);
        let state_snapshot_hash = FleetActivationEvidenceOps::state_snapshot_hash(&state_input)?;

        let mut topology_inputs = Vec::with_capacity(children.len());
        let mut cascade_manifest = Vec::with_capacity(children.len());
        for child in &children {
            let topology = TopologyCascadeWorkflow::root_snapshot_input_for_target(child.pid)?;
            let topology_snapshot_hash =
                FleetActivationEvidenceOps::topology_snapshot_hash(&topology)?;
            cascade_manifest.push(FleetCascadeManifestEntry {
                principal: child.pid,
                state_snapshot_hash,
                topology_snapshot_hash,
            });
            topology_inputs.push((child.pid, topology));
        }
        let cascade_manifest_hash =
            FleetActivationEvidenceOps::cascade_manifest_hash(&cascade_manifest)?;

        let credential_manifest = FleetCredentialManifest {
            fleet: current.identity.fleet.fleet,
            activation_id: current.identity.operation_id,
            generation: 1,
            root_policy_set_hash: FleetActivationEvidenceOps::empty_root_policy_set_hash()?,
            renewal_template_set_hash: FleetActivationEvidenceOps::empty_renewal_template_set_hash(
            )?,
            entries: Vec::new(),
        };
        let credential = FleetCredentialGenerationRef {
            generation: credential_manifest.generation,
            manifest_hash: FleetActivationEvidenceOps::credential_manifest_hash(
                &credential_manifest,
            )?,
        };
        FleetActivationOps::prepare_root(
            cascade_manifest,
            cascade_manifest_hash,
            credential,
            credential_manifest,
        )
        .map_err(StorageOpsError::from)?;

        StateCascadeWorkflow::root_cascade_state(&state_snapshot).await?;
        for (pid, topology) in topology_inputs {
            CascadeOps::send_topology_snapshot(pid, topology).await?;
        }
        Self::status()
    }

    pub async fn resume_root(
        request: FleetActivationResumeRequest,
    ) -> Result<FleetActivationTransition, InternalError> {
        EnvOps::require_root()?;
        let root_status = Self::status()?;
        if root_status.identity.operation_id != request.operation_id
            || root_status.credential != Some(request.credential)
        {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "Fleet activation resume identity does not match protected root state",
            ));
        }
        let manifest = root_status.cascade_manifest.clone().ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "Fleet activation resume requires a protected cascade manifest",
            )
        })?;

        for entry in &manifest {
            resume_nonroot_activation(entry, &root_status, request).await?;
        }

        let root_cascade = root_status.cascade.clone().ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "Fleet activation resume requires protected root cascade evidence",
            )
        })?;
        let activation_evidence_hash = FleetActivationEvidenceOps::activation_evidence_hash(
            &root_status.identity,
            &root_cascade,
            request.credential,
        )?;
        let transition = FleetActivationOps::activate(
            FleetActivationRequest {
                operation_id: request.operation_id,
                credential: request.credential,
                activation_evidence_hash,
            },
            true,
            IcOps::now_nanos(),
        )
        .map_err(StorageOpsError::from)?;
        if transition.transitioned
            && let Err(error) = crate::workflow::runtime::RuntimeWorkflow::start_all_root()
        {
            IcOps::trap(format!(
                "Fleet root activation could not establish runtime services: {error}"
            ));
        }
        Ok(transition)
    }

    pub fn prepare_nonroot_credential_generation(
        request: FleetCredentialGenerationRequest,
    ) -> Result<FleetActivationStatusResponse, InternalError> {
        EnvOps::deny_root()?;
        FleetActivationOps::prepare_credential_generation(request)
            .map_err(StorageOpsError::from)
            .map_err(Into::into)
    }

    pub fn activate_nonroot(
        request: FleetActivationRequest,
    ) -> Result<FleetActivationTransition, InternalError> {
        EnvOps::deny_root()?;
        let transition = FleetActivationOps::activate(request, false, IcOps::now_nanos())
            .map_err(StorageOpsError::from)
            .map_err(InternalError::from)?;
        if transition.transitioned
            && let Err(error) = crate::workflow::runtime::RuntimeWorkflow::start_all()
        {
            IcOps::trap(format!(
                "Fleet non-root activation could not establish runtime services: {error}"
            ));
        }
        Ok(transition)
    }

    /// Complete activation handling for one newly provisioned managed non-root.
    ///
    /// Initial bootstrap children remain Prepared for the root's complete
    /// activation manifest. Once the root is Active, the root validates the
    /// exact cascade payloads it just propagated, advances the new child to the
    /// Fleet's frozen credential generation, and observes uncertain call
    /// outcomes through the controller status surface.
    pub(crate) async fn complete_provisioned_nonroot_activation(
        pid: crate::cdk::types::Principal,
        state: StateSnapshotInput,
        topology: TopologySnapshotInput,
    ) -> Result<(), InternalError> {
        EnvOps::require_root()?;
        let root_status = Self::status()?;
        if root_status.phase == FleetActivationPhase::Prepared {
            return Ok(());
        }
        let credential = root_status.credential.ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "active root Fleet is missing its credential generation",
            )
        })?;
        let expected_cascade = FleetCascadeActivationEvidence::Applied {
            state_snapshot_hash: FleetActivationEvidenceOps::state_snapshot_hash(&state)?,
            topology_snapshot_hash: FleetActivationEvidenceOps::topology_snapshot_hash(&topology)?,
        };
        let generation_request = FleetCredentialGenerationRequest {
            operation_id: root_status.identity.operation_id,
            credential,
        };

        let prepared = match RpcOps::call_rpc_result(
            pid,
            protocol::CANIC_PREPARE_FLEET_CREDENTIAL_GENERATION,
            generation_request,
        )
        .await
        {
            Ok(status) => status,
            Err(error) => {
                reconcile_nonroot_activation_status_after_call_error(
                    pid,
                    &root_status,
                    &expected_cascade,
                    None,
                    "credential-generation preparation",
                    error,
                )
                .await?
            }
        };
        validate_nonroot_activation_status(&root_status, &prepared, &expected_cascade, None)?;

        let activation_evidence_hash = FleetActivationEvidenceOps::activation_evidence_hash(
            &prepared.identity,
            &expected_cascade,
            credential,
        )?;
        let request = FleetActivationRequest {
            operation_id: root_status.identity.operation_id,
            credential,
            activation_evidence_hash,
        };
        let activated =
            match RpcOps::call_rpc_result(pid, protocol::CANIC_ACTIVATE_FLEET, request).await {
                Ok(status) => status,
                Err(error) => {
                    reconcile_nonroot_activation_status_after_call_error(
                        pid,
                        &root_status,
                        &expected_cascade,
                        Some(FleetActivationPhase::Active),
                        "activation",
                        error,
                    )
                    .await?
                }
            };
        validate_nonroot_activation_status(
            &root_status,
            &activated,
            &expected_cascade,
            Some(FleetActivationPhase::Active),
        )?;
        Ok(())
    }

    /// Enforce the activation phase before a managed endpoint handler runs.
    pub fn require_endpoint_allowed(call: EndpointCall) -> Result<(), InternalError> {
        if EnvOps::is_fleet_coordinator_runtime() {
            return Ok(());
        }
        let is_root = EnvOps::canister_role()?.is_root();
        if !is_root && FleetActivationRuntimeOps::is_standalone_local() {
            return Ok(());
        }
        let status = FleetActivationOps::status(is_root)
            .map_err(StorageOpsError::from)
            .map_err(InternalError::from)?;

        require_endpoint_for_phase(is_root, status.phase, call).map_err(InternalError::from)
    }
}

async fn resume_nonroot_activation(
    entry: &FleetCascadeManifestEntry,
    root_status: &FleetActivationStatusResponse,
    request: FleetActivationResumeRequest,
) -> Result<(), InternalError> {
    let expected_cascade = FleetCascadeActivationEvidence::Applied {
        state_snapshot_hash: entry.state_snapshot_hash,
        topology_snapshot_hash: entry.topology_snapshot_hash,
    };
    let prepared: FleetActivationStatusResponse = match RpcOps::call_rpc_result(
        entry.principal,
        protocol::CANIC_PREPARE_FLEET_CREDENTIAL_GENERATION,
        FleetCredentialGenerationRequest {
            operation_id: request.operation_id,
            credential: request.credential,
        },
    )
    .await
    {
        Ok(status) => status,
        Err(error) => {
            reconcile_nonroot_activation_status_after_call_error(
                entry.principal,
                root_status,
                &expected_cascade,
                None,
                "credential-generation preparation",
                error,
            )
            .await?
        }
    };
    validate_nonroot_activation_status(root_status, &prepared, &expected_cascade, None)?;

    let activation_evidence_hash = FleetActivationEvidenceOps::activation_evidence_hash(
        &prepared.identity,
        &expected_cascade,
        request.credential,
    )?;
    let activated: FleetActivationStatusResponse = match RpcOps::call_rpc_result(
        entry.principal,
        protocol::CANIC_ACTIVATE_FLEET,
        FleetActivationRequest {
            operation_id: request.operation_id,
            credential: request.credential,
            activation_evidence_hash,
        },
    )
    .await
    {
        Ok(status) => status,
        Err(error) => {
            reconcile_nonroot_activation_status_after_call_error(
                entry.principal,
                root_status,
                &expected_cascade,
                Some(FleetActivationPhase::Active),
                "activation",
                error,
            )
            .await?
        }
    };
    validate_nonroot_activation_status(
        root_status,
        &activated,
        &expected_cascade,
        Some(FleetActivationPhase::Active),
    )
}

async fn reconcile_nonroot_activation_status_after_call_error(
    pid: crate::cdk::types::Principal,
    root_status: &FleetActivationStatusResponse,
    expected_cascade: &FleetCascadeActivationEvidence,
    required_phase: Option<FleetActivationPhase>,
    operation: &str,
    call_error: InternalError,
) -> Result<FleetActivationStatusResponse, InternalError> {
    let observed: FleetActivationStatusResponse = match RpcOps::call_rpc_result(
        pid,
        protocol::CANIC_FLEET_ACTIVATION_STATUS,
        (),
    )
    .await
    {
        Ok(status) => status,
        Err(observation_error) => {
            return Err(call_error.with_diagnostic_context(format!(
                "could not reconcile uncertain child {operation} outcome for {pid}: {observation_error}"
            )));
        }
    };
    if let Err(observation_error) =
        validate_nonroot_activation_status(root_status, &observed, expected_cascade, required_phase)
    {
        return Err(call_error.with_diagnostic_context(format!(
            "child {operation} outcome for {pid} was not established by status: {observation_error}"
        )));
    }
    Ok(observed)
}

fn validate_nonroot_activation_status(
    root_status: &FleetActivationStatusResponse,
    child_status: &FleetActivationStatusResponse,
    expected_cascade: &FleetCascadeActivationEvidence,
    required_phase: Option<FleetActivationPhase>,
) -> Result<(), InternalError> {
    if child_status.identity != root_status.identity
        || child_status.cascade.as_ref() != Some(expected_cascade)
        || child_status.credential != root_status.credential
        || child_status.cascade_manifest.is_some()
        || child_status.credential_manifest.is_some()
        || match child_status.phase {
            FleetActivationPhase::Prepared => child_status.activated_at_ns.is_some(),
            FleetActivationPhase::Active => child_status.activated_at_ns.is_none(),
        }
        || required_phase.is_some_and(|phase| child_status.phase != phase)
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "provisioned non-root status does not match the active root Fleet and propagated evidence",
        ));
    }
    Ok(())
}

fn require_empty_prepared_credential_authority() -> Result<(), InternalError> {
    if !AuthStateOps::root_issuer_policies().is_empty()
        || !AuthStateOps::root_issuer_renewal_templates().is_empty()
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "Fleet activation with configured issuer credentials requires the credential-bundle activation slice",
        ));
    }
    Ok(())
}

fn require_direct_activation_inventory(
    root_pid: crate::cdk::types::Principal,
    children: &[RegisteredCanisterView],
) -> Result<(), InternalError> {
    let direct = children
        .iter()
        .map(|entry| entry.pid)
        .collect::<BTreeSet<_>>();
    let managed = SubnetRegistryOps::registrations()
        .into_iter()
        .filter(|entry| entry.pid != root_pid)
        .map(|entry| entry.pid)
        .collect::<BTreeSet<_>>();
    if direct != managed {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "fresh Fleet activation currently requires every managed non-root to be a direct root child",
        ));
    }
    Ok(())
}

fn require_endpoint_for_phase(
    is_root: bool,
    phase: FleetActivationPhase,
    call: EndpointCall,
) -> Result<(), PolicyError> {
    match phase {
        FleetActivationPhase::Prepared if is_root => {
            require_prepared_root_endpoint(call).map_err(PolicyError::from)
        }
        FleetActivationPhase::Prepared => {
            require_prepared_nonroot_endpoint(call).map_err(PolicyError::from)
        }
        FleetActivationPhase::Active => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{
        AppId, CanonicalNetworkId, EndpointCallKind, EndpointId, FleetBinding, FleetId, FleetKey,
        ReleaseBuildId, ReleaseBuildNonce,
    };

    fn call(name: &'static str, kind: EndpointCallKind) -> EndpointCall {
        EndpointCall {
            endpoint: EndpointId::new(name),
            kind,
        }
    }

    #[test]
    fn active_admits_ordinary_handlers_but_prepared_delegates_to_exact_policy() {
        let ordinary = call("application_update", EndpointCallKind::Update);

        assert!(require_endpoint_for_phase(true, FleetActivationPhase::Active, ordinary).is_ok());
        assert!(matches!(
            require_endpoint_for_phase(true, FleetActivationPhase::Prepared, ordinary),
            Err(PolicyError::FleetActivationPolicy(_))
        ));
        assert!(matches!(
            require_endpoint_for_phase(false, FleetActivationPhase::Prepared, ordinary),
            Err(PolicyError::FleetActivationPolicy(_))
        ));
    }

    fn root_status() -> FleetActivationStatusResponse {
        FleetActivationStatusResponse {
            phase: FleetActivationPhase::Active,
            identity: crate::dto::fleet_activation::FleetActivationIdentity {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::public_ic(),
                        fleet_id: FleetId::from_generated_bytes([1; 32]),
                    },
                    app: AppId::from("toko"),
                },
                operation_id: [2; 32],
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [3; 32],
                )),
            },
            cascade: Some(FleetCascadeActivationEvidence::Source {
                cascade_manifest_hash: [4; 32],
            }),
            cascade_manifest: Some(Vec::new()),
            credential: Some(FleetCredentialGenerationRef {
                generation: 1,
                manifest_hash: [5; 32],
            }),
            credential_manifest: Some(FleetCredentialManifest {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::public_ic(),
                    fleet_id: FleetId::from_generated_bytes([1; 32]),
                },
                activation_id: [2; 32],
                generation: 1,
                root_policy_set_hash: [6; 32],
                renewal_template_set_hash: [7; 32],
                entries: Vec::new(),
            }),
            activated_at_ns: Some(8),
        }
    }

    fn child_status(
        root: &FleetActivationStatusResponse,
        phase: FleetActivationPhase,
        cascade: FleetCascadeActivationEvidence,
    ) -> FleetActivationStatusResponse {
        FleetActivationStatusResponse {
            phase,
            identity: root.identity.clone(),
            cascade: Some(cascade),
            cascade_manifest: None,
            credential: root.credential,
            credential_manifest: None,
            activated_at_ns: (phase == FleetActivationPhase::Active).then_some(9),
        }
    }

    #[test]
    fn nonroot_activation_requires_exact_root_identity_cascade_generation_and_phase_evidence() {
        let root = root_status();
        let expected_cascade = FleetCascadeActivationEvidence::Applied {
            state_snapshot_hash: [10; 32],
            topology_snapshot_hash: [11; 32],
        };
        let prepared = child_status(
            &root,
            FleetActivationPhase::Prepared,
            expected_cascade.clone(),
        );

        validate_nonroot_activation_status(&root, &prepared, &expected_cascade, None)
            .expect("exact prepared child");
        assert!(
            validate_nonroot_activation_status(
                &root,
                &prepared,
                &expected_cascade,
                Some(FleetActivationPhase::Active),
            )
            .is_err()
        );

        let mut wrong_identity = prepared;
        wrong_identity.identity.operation_id = [12; 32];
        assert!(
            validate_nonroot_activation_status(&root, &wrong_identity, &expected_cascade, None,)
                .is_err()
        );

        let mut active_without_timestamp = child_status(
            &root,
            FleetActivationPhase::Active,
            expected_cascade.clone(),
        );
        active_without_timestamp.activated_at_ns = None;
        assert!(
            validate_nonroot_activation_status(
                &root,
                &active_without_timestamp,
                &expected_cascade,
                Some(FleetActivationPhase::Active),
            )
            .is_err()
        );
    }
}
