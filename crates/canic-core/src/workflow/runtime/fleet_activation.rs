//! Module: workflow::runtime::fleet_activation
//!
//! Responsibility: expose the canonical protected Fleet activation status.
//! Does not own: stable conversion, endpoint authorization, or activation mutation.
//! Boundary: the runtime role selects root-only projection before ops validates the record.

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::pure::{
        PolicyError,
        fleet_activation::{
            require_prepared_nonroot_endpoint, require_prepared_root_endpoint,
            require_prepared_store_data_endpoint,
        },
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
        role::{OperationReceipt, OperationStatusRequest},
    },
    ids::{EndpointCall, EndpointCallKind, EndpointId, FleetSubnetWasmStoreAuthority},
    ops::{
        cascade::CascadeOps,
        fleet_activation::FleetActivationEvidenceOps,
        ic::IcOps,
        rpc::RpcOps,
        runtime::{env::EnvOps, fleet_activation::FleetActivationRuntimeOps},
        storage::{StorageOpsError, auth::AuthStateOps, fleet_activation::FleetActivationOps},
    },
    protocol,
    view::fleet_activation::{FleetActivationTransition, FleetActivationWasmStoreView},
    workflow::cascade::{
        snapshot::{StateSnapshotBuilder, adapter::StateSnapshotAdapter},
        state::StateCascadeWorkflow,
        topology::TopologyCascadeWorkflow,
    },
};
use candid::CandidType;
use serde::Deserialize;

#[derive(CandidType)]
enum StoreCommandFragment {
    ActivateFleet(FleetActivationRequest),
    PrepareFleetCredential(FleetCredentialGenerationRequest),
}

#[derive(CandidType, Deserialize)]
enum StoreCommandResponseFragment {
    OperationAccepted(OperationReceipt),
}

#[derive(CandidType)]
enum StoreStatusRequestFragment {
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum StoreStatusResponseFragment {
    Operation(StoreOperationStatusFragment),
}

#[derive(CandidType, Deserialize)]
enum StoreOperationStatusFragment {
    FleetActivation(FleetActivationStatusResponse),
}

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

    pub fn wasm_store_authority() -> Result<FleetSubnetWasmStoreAuthority, InternalError> {
        EnvOps::deny_root()?;
        FleetActivationOps::wasm_store_authority()
            .map_err(StorageOpsError::from)
            .map_err(Into::into)
    }

    pub async fn prepare_root(
        wasm_store: FleetActivationWasmStoreView,
    ) -> Result<FleetActivationStatusResponse, InternalError> {
        EnvOps::require_root()?;
        let current = Self::status()?;
        if current.phase == FleetActivationPhase::Active {
            return Ok(current);
        }
        require_empty_prepared_credential_authority()?;

        let root_pid = IcOps::canister_self();
        require_root_activation_wasm_store(root_pid, wasm_store.pid)?;

        let state_snapshot = StateSnapshotBuilder::new()?.with_fleet_state().build();
        let state_input = StateSnapshotAdapter::to_input(&state_snapshot);
        let state_snapshot_hash = FleetActivationEvidenceOps::state_snapshot_hash(&state_input)?;

        let topology = TopologyCascadeWorkflow::root_wasm_store_snapshot_input(wasm_store.pid)?;
        let topology_snapshot_hash = FleetActivationEvidenceOps::topology_snapshot_hash(&topology)?;
        let cascade_manifest = vec![FleetCascadeManifestEntry {
            principal: wasm_store.pid,
            state_snapshot_hash,
            topology_snapshot_hash,
        }];
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

        StateCascadeWorkflow::root_cascade_state_to(&state_snapshot, &[wasm_store.pid]).await?;
        CascadeOps::send_topology_snapshot(wasm_store.pid, &topology).await?;
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
            return Err(InternalError::invariant());
        }
        let manifest = root_status
            .cascade_manifest
            .clone()
            .ok_or_else(InternalError::invariant)?;

        for entry in &manifest {
            resume_nonroot_activation(entry, &root_status, request).await?;
        }

        let root_cascade = root_status
            .cascade
            .clone()
            .ok_or_else(InternalError::invariant)?;
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
                "Fleet Subnet Root activation could not establish runtime services: {error}"
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
        let credential = root_status
            .credential
            .ok_or_else(InternalError::invariant)?;
        let expected_cascade = FleetCascadeActivationEvidence::Applied {
            state_snapshot_hash: FleetActivationEvidenceOps::state_snapshot_hash(&state)?,
            topology_snapshot_hash: FleetActivationEvidenceOps::topology_snapshot_hash(&topology)?,
        };
        let generation_request = FleetCredentialGenerationRequest {
            operation_id: root_status.identity.operation_id,
            credential,
        };

        let prepared = match submit_store_fleet_command(
            pid,
            StoreCommandFragment::PrepareFleetCredential(generation_request),
            root_status.identity.operation_id,
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
        let activated = match submit_store_fleet_command(
            pid,
            StoreCommandFragment::ActivateFleet(request),
            root_status.identity.operation_id,
        )
        .await
        {
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

    /// Cascade and activate one newly installed root-owned Store without Registry topology.
    pub async fn complete_provisioned_wasm_store_activation(
        wasm_store: Principal,
    ) -> Result<(), InternalError> {
        EnvOps::require_root()?;
        let root = IcOps::canister_self();
        require_root_activation_wasm_store(root, wasm_store)?;

        let state_snapshot = StateSnapshotBuilder::new()?.with_fleet_state().build();
        let state_input = StateSnapshotAdapter::to_input(&state_snapshot);
        let topology = TopologyCascadeWorkflow::root_wasm_store_snapshot_input(wasm_store)?;

        StateCascadeWorkflow::root_cascade_state_to(&state_snapshot, &[wasm_store]).await?;
        CascadeOps::send_topology_snapshot(wasm_store, &topology).await?;
        Self::complete_provisioned_nonroot_activation(wasm_store, state_input, topology).await
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

    /// Enforce the activation phase for a compile-selected Store data-lane endpoint.
    pub fn require_store_data_endpoint_allowed(call: EndpointCall) -> Result<(), InternalError> {
        if FleetActivationRuntimeOps::is_standalone_local() {
            return Ok(());
        }
        let status = FleetActivationOps::status(false)
            .map_err(StorageOpsError::from)
            .map_err(InternalError::from)?;
        match status.phase {
            FleetActivationPhase::Prepared => require_prepared_store_data_endpoint(call)
                .map_err(PolicyError::from)
                .map_err(InternalError::from),
            FleetActivationPhase::Active => Ok(()),
        }
    }

    /// Preserve the prepared-Root fence after role methods collapse many variants into one name.
    pub fn require_root_command_variant_allowed(
        prepared_allowed: bool,
    ) -> Result<(), InternalError> {
        Self::require_root_role_variant_allowed(
            prepared_allowed,
            EndpointCall {
                endpoint: EndpointId::new(protocol::CANIC_COMMAND),
                kind: EndpointCallKind::Update,
            },
        )
    }

    /// Preserve the prepared-Root observation fence for the consolidated status selector.
    pub fn require_root_status_variant_allowed(
        prepared_allowed: bool,
    ) -> Result<(), InternalError> {
        Self::require_root_role_variant_allowed(
            prepared_allowed,
            EndpointCall {
                endpoint: EndpointId::new(protocol::CANIC_STATUS),
                kind: EndpointCallKind::Query,
            },
        )
    }

    fn require_root_role_variant_allowed(
        prepared_allowed: bool,
        call: EndpointCall,
    ) -> Result<(), InternalError> {
        EnvOps::require_root()?;
        let status = FleetActivationOps::status(true)
            .map_err(StorageOpsError::from)
            .map_err(InternalError::from)?;
        if status.phase == FleetActivationPhase::Active || prepared_allowed {
            return Ok(());
        }
        Err(InternalError::from(PolicyError::from(
            crate::domain::policy::pure::fleet_activation::FleetActivationEndpointPolicyError::Fenced {
                endpoint: call.endpoint.name,
                kind: call.kind,
            },
        )))
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
    let prepared: FleetActivationStatusResponse = match submit_store_fleet_command(
        entry.principal,
        StoreCommandFragment::PrepareFleetCredential(FleetCredentialGenerationRequest {
            operation_id: request.operation_id,
            credential: request.credential,
        }),
        request.operation_id,
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
    let activated: FleetActivationStatusResponse = match submit_store_fleet_command(
        entry.principal,
        StoreCommandFragment::ActivateFleet(FleetActivationRequest {
            operation_id: request.operation_id,
            credential: request.credential,
            activation_evidence_hash,
        }),
        request.operation_id,
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
    _operation: &str,
    call_error: InternalError,
) -> Result<FleetActivationStatusResponse, InternalError> {
    let observed: FleetActivationStatusResponse =
        match query_store_fleet_activation_status(pid, root_status.identity.operation_id).await {
            Ok(status) => status,
            Err(_observation_error) => {
                return Err(call_error);
            }
        };
    if let Err(_observation_error) =
        validate_nonroot_activation_status(root_status, &observed, expected_cascade, required_phase)
    {
        return Err(call_error);
    }
    Ok(observed)
}

async fn submit_store_fleet_command(
    pid: Principal,
    command: StoreCommandFragment,
    operation_id: [u8; 32],
) -> Result<FleetActivationStatusResponse, InternalError> {
    let response: StoreCommandResponseFragment =
        RpcOps::call_rpc_result(pid, protocol::CANIC_COMMAND, command).await?;
    let StoreCommandResponseFragment::OperationAccepted(receipt) = response;
    if receipt.operation_id != operation_id {
        return Err(InternalError::conflict());
    }
    query_store_fleet_activation_status(pid, operation_id).await
}

async fn query_store_fleet_activation_status(
    pid: Principal,
    operation_id: [u8; 32],
) -> Result<FleetActivationStatusResponse, InternalError> {
    let response: StoreStatusResponseFragment = RpcOps::call_rpc_result(
        pid,
        protocol::CANIC_STATUS,
        StoreStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
    )
    .await?;
    let StoreStatusResponseFragment::Operation(StoreOperationStatusFragment::FleetActivation(
        status,
    )) = response;
    if status.identity.operation_id != operation_id {
        return Err(InternalError::conflict());
    }
    Ok(status)
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
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn require_empty_prepared_credential_authority() -> Result<(), InternalError> {
    if !AuthStateOps::root_issuer_policies().is_empty()
        || !AuthStateOps::root_issuer_renewal_templates().is_empty()
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn require_root_activation_wasm_store(
    root_pid: crate::cdk::types::Principal,
    wasm_store: Principal,
) -> Result<(), InternalError> {
    if wasm_store == Principal::anonymous() {
        return Err(InternalError::invariant());
    }
    if wasm_store == root_pid {
        return Err(InternalError::invariant());
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

    fn assert_invariant(result: Result<(), InternalError>) {
        let error = result.expect_err("activation inventory must fail");
        assert_eq!(error.code(), crate::diagnostics::codes::STATE_INVALID);
    }

    #[test]
    fn root_activation_requires_a_distinct_non_anonymous_wasm_store() {
        let root = Principal::from_slice(&[1]);
        let store = Principal::from_slice(&[2]);

        assert!(require_root_activation_wasm_store(root, store).is_ok());
        assert_invariant(require_root_activation_wasm_store(
            root,
            Principal::anonymous(),
        ));
        assert_invariant(require_root_activation_wasm_store(root, root));
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
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
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
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
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
