//! Module: workflow::root_admission
//!
//! Responsibility: discover one Root subtree and drive its replay-safe admission convergence.
//! Does not own: Fleet policy choice, stable encoding, target transition policy, or caller auth.
//! Boundary: the exact Coordinator command starts a durable Root journal before bounded calls.

use crate::{
    ops::{
        component_registry::ComponentRegistryOps,
        fleet_registry_mirror::FleetRegistryMirrorOps,
        root_admission::{RootAdmissionOps, RootAdmissionStep},
    },
    view::component_registry::ComponentDirectoryPageSelection,
    workflow::root_authority::validated_root_authority,
};
use candid::{CandidType, Principal};
use canic_core::{
    api::timer::TimerApi,
    control_plane_support::{
        error::InternalError,
        ops::{config::ConfigOps, ic::call::CallOps},
    },
    dto::{
        component_registry::ComponentLifecycleStatus,
        error::Error,
        fleet_admission::{
            FleetAdmissionActivateRootRequest, FleetAdmissionActivateTargetRequest,
            FleetAdmissionOpenRootRequest, FleetAdmissionOpenTargetRequest,
            FleetAdmissionPrepareRootRequest, FleetAdmissionPrepareTargetRequest,
            FleetAdmissionRootReceipt, FleetAdmissionTargetReceipt,
            FleetAdmissionTargetTransitionPhase,
        },
        page::PageRequest,
    },
    ids::{FleetAdmissionPolicy, FleetAdmissionProjection, ManagedCanisterBinding},
    protocol,
    shared_support::fleet_admission_policy::{
        effective_fleet_admission_principals, fleet_admission_target_for_binding,
        materialize_fleet_admission_projection,
    },
};
use serde::Deserialize;
use std::time::Duration;

#[derive(CandidType)]
enum RemoteManagedCommand {
    ActivateFleetAdmission(FleetAdmissionActivateTargetRequest),
    OpenFleetAdmission(FleetAdmissionOpenTargetRequest),
    PrepareFleetAdmission(Box<FleetAdmissionPrepareTargetRequest>),
}

#[derive(CandidType, Deserialize)]
enum RemoteManagedCommandResponse {
    ActivateFleetAdmission(FleetAdmissionTargetReceipt),
    OpenFleetAdmission(FleetAdmissionTargetReceipt),
    PrepareFleetAdmission(FleetAdmissionTargetReceipt),
}

/// Authenticate one Root admission phase command against the exact installed Coordinator.
pub fn authorize_coordinator(caller: Principal) -> Result<(), InternalError> {
    crate::workflow::root_funding::authorize_coordinator(caller)
}

/// Start or replay one Root prepare command and return only after its subtree is fenced.
pub fn prepare(
    request: FleetAdmissionPrepareRootRequest,
) -> Result<FleetAdmissionRootReceipt, InternalError> {
    let (protected, _) = validated_root_authority()?;
    let root = protected.binding;
    let active = FleetRegistryMirrorOps::active_admission(&root)?;
    let projections = if RootAdmissionOps::retains_operation_id(&root, request.operation_id)? {
        Vec::new()
    } else {
        compile_participant_projections(&root, &request.successor)?
    };
    if let Some(receipt) = RootAdmissionOps::prepare(&root, active, request.clone(), projections)? {
        Ok(receipt)
    } else {
        schedule(request.operation_id, Duration::ZERO);
        Err(InternalError::unavailable())
    }
}

/// Start or replay Root activation and return only after every target is successor-fenced.
pub fn activate(
    request: FleetAdmissionActivateRootRequest,
) -> Result<FleetAdmissionRootReceipt, InternalError> {
    let (protected, _) = validated_root_authority()?;
    if let Some(receipt) = RootAdmissionOps::activate(&protected.binding, request.clone())? {
        Ok(receipt)
    } else {
        schedule(request.operation_id, Duration::ZERO);
        Err(InternalError::unavailable())
    }
}

/// Start or replay Root opening and return the retained terminal receipt.
pub fn open(
    request: FleetAdmissionOpenRootRequest,
) -> Result<FleetAdmissionRootReceipt, InternalError> {
    let (protected, _) = validated_root_authority()?;
    if let Some(receipt) = RootAdmissionOps::open(&protected.binding, request.clone())? {
        Ok(receipt)
    } else {
        schedule(request.operation_id, Duration::ZERO);
        Err(InternalError::unavailable())
    }
}

/// Return one bounded protected Root distribution view.
pub fn status(
    request: PageRequest,
) -> Result<canic_core::dto::fleet_admission::FleetAdmissionRootStatusResponse, InternalError> {
    let (protected, _) = validated_root_authority()?;
    let fallback = FleetRegistryMirrorOps::active_admission(&protected.binding)?;
    let projections =
        if RootAdmissionOps::status_requires_live_catalog(&protected.binding, fallback.clone())? {
            let active = RootAdmissionOps::active_policy(&protected.binding, fallback.clone())?;
            compile_participant_projections(&protected.binding, &active)?
        } else {
            Vec::new()
        };
    RootAdmissionOps::status(&protected.binding, fallback, projections, request)
}

/// Resolve the one converged policy used for newly created managed targets.
pub fn current_policy() -> Result<FleetAdmissionPolicy, InternalError> {
    let (protected, _) = validated_root_authority()?;
    let fallback = FleetRegistryMirrorOps::active_admission(&protected.binding)?;
    RootAdmissionOps::active_policy(&protected.binding, fallback)
}

/// Fence any Root operation that would change the transition participant catalog.
pub fn require_catalog_mutation_allowed() -> Result<(), InternalError> {
    let (protected, _) = validated_root_authority()?;
    RootAdmissionOps::require_catalog_mutation_allowed(&protected.binding)
}

fn schedule(operation_id: [u8; 32], delay: Duration) {
    TimerApi::defer_lifecycle_required(delay, "Root Fleet-admission convergence", async move {
        match advance_once(operation_id).await {
            Ok(true) => {}
            Ok(false) => schedule(operation_id, Duration::ZERO),
            Err(_) => schedule(operation_id, Duration::from_secs(1)),
        }
    });
}

async fn advance_once(operation_id: [u8; 32]) -> Result<bool, InternalError> {
    let (protected, _) = validated_root_authority()?;
    let root = protected.binding;
    let (expected, step) = RootAdmissionOps::next_step(&root)?;
    if step == RootAdmissionStep::Waiting {
        return Ok(true);
    }
    let current = expected
        .current_transition
        .as_ref()
        .ok_or_else(InternalError::conflict)?;
    if current.request.operation_id != operation_id {
        return Err(InternalError::conflict());
    }
    match step {
        RootAdmissionStep::Prepare { projection } => {
            let receipt = call_target(
                &projection,
                RemoteManagedCommand::PrepareFleetAdmission(Box::new(
                    FleetAdmissionPrepareTargetRequest {
                        operation_id,
                        expected_generation: current.request.expected_generation,
                        expected_policy_digest: current.request.expected_policy_digest,
                        successor: projection.clone(),
                    },
                )),
                FleetAdmissionTargetTransitionPhase::Prepare,
            )
            .await?;
            RootAdmissionOps::record_target_receipt(
                &root,
                &expected,
                projection,
                FleetAdmissionTargetTransitionPhase::Prepare,
                receipt,
            )?;
            Ok(false)
        }
        RootAdmissionStep::Activate { projection } => {
            let receipt = call_target(
                &projection,
                RemoteManagedCommand::ActivateFleetAdmission(FleetAdmissionActivateTargetRequest {
                    operation_id,
                    expected_generation: current.request.expected_generation,
                    expected_policy_digest: current.request.expected_policy_digest,
                    successor_generation: projection.generation,
                    successor_policy_digest: projection.policy_digest,
                    successor_projection_digest: projection.projection_digest,
                }),
                FleetAdmissionTargetTransitionPhase::Activate,
            )
            .await?;
            RootAdmissionOps::record_target_receipt(
                &root,
                &expected,
                projection,
                FleetAdmissionTargetTransitionPhase::Activate,
                receipt,
            )?;
            Ok(false)
        }
        RootAdmissionStep::Open { projection } => {
            let receipt = call_target(
                &projection,
                RemoteManagedCommand::OpenFleetAdmission(FleetAdmissionOpenTargetRequest {
                    operation_id,
                    generation: projection.generation,
                    policy_digest: projection.policy_digest,
                    projection_digest: projection.projection_digest,
                }),
                FleetAdmissionTargetTransitionPhase::Open,
            )
            .await?;
            RootAdmissionOps::record_target_receipt(
                &root,
                &expected,
                projection,
                FleetAdmissionTargetTransitionPhase::Open,
                receipt,
            )?;
            Ok(false)
        }
        RootAdmissionStep::Complete => {
            RootAdmissionOps::complete(&root, &expected)?;
            Ok(true)
        }
        RootAdmissionStep::Waiting => unreachable!(),
    }
}

async fn call_target(
    projection: &FleetAdmissionProjection,
    command: RemoteManagedCommand,
    expected_phase: FleetAdmissionTargetTransitionPhase,
) -> Result<FleetAdmissionTargetReceipt, InternalError> {
    let target = target_principal(&projection.target);
    let call = CallOps::bounded_wait(target, protocol::CANIC_COMMAND)
        .with_arg(command)?
        .execute()
        .await?;
    let result: Result<RemoteManagedCommandResponse, Error> = call.candid()?;
    let response = result.map_err(InternalError::observed_public)?;
    match (expected_phase, response) {
        (
            FleetAdmissionTargetTransitionPhase::Prepare,
            RemoteManagedCommandResponse::PrepareFleetAdmission(receipt),
        )
        | (
            FleetAdmissionTargetTransitionPhase::Activate,
            RemoteManagedCommandResponse::ActivateFleetAdmission(receipt),
        )
        | (
            FleetAdmissionTargetTransitionPhase::Open,
            RemoteManagedCommandResponse::OpenFleetAdmission(receipt),
        ) => Ok(receipt),
        _ => Err(InternalError::conflict()),
    }
}

fn compile_participant_projections(
    root: &canic_core::ids::FleetSubnetRootBinding,
    policy: &FleetAdmissionPolicy,
) -> Result<Vec<FleetAdmissionProjection>, InternalError> {
    if policy.fleet != root.authority.binding.fleet {
        return Err(InternalError::invariant());
    }
    crate::ops::component_provisioning::RootComponentProvisioningOps::
        require_ordinary_allocation_open()?;
    ComponentRegistryOps::require_admission_catalog_stable()?;
    let mut targets = Vec::new();
    for partition in ComponentRegistryOps::root_component_partitions()? {
        if partition.status != ComponentLifecycleStatus::Active {
            return Err(InternalError::conflict());
        }
        if ConfigOps::role_uses_fleet_admission(&partition.binding.role)? {
            targets.push(ManagedCanisterBinding::Component(partition.binding.clone()));
        }
        let remaining =
            canic_core::shared_support::fleet_admission_root::MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS
                .checked_sub(targets.len())
                .ok_or_else(InternalError::resource_exhausted)?;
        let page = ComponentRegistryOps::directory_page(
            partition.binding.component,
            &ComponentDirectoryPageSelection {
                parent_canister_id: None,
                role: None,
                status: None,
                start_after: None,
            },
            remaining.saturating_add(1),
        )?;
        if page.next_cursor.is_some() || page.entries.len() > remaining {
            return Err(InternalError::resource_exhausted());
        }
        if page
            .entries
            .iter()
            .any(|entry| entry.status != ComponentLifecycleStatus::Active)
        {
            return Err(InternalError::conflict());
        }
        for entry in page.entries {
            if ConfigOps::role_uses_fleet_admission(&entry.binding.role)? {
                targets.push(ManagedCanisterBinding::ComponentChild(entry.binding));
            }
        }
    }
    targets.sort_by(|left, right| {
        target_principal(left)
            .as_slice()
            .cmp(target_principal(right).as_slice())
    });
    if targets
        .windows(2)
        .any(|pair| target_principal(&pair[0]) == target_principal(&pair[1]))
    {
        return Err(InternalError::invariant());
    }
    targets
        .into_iter()
        .map(|target| {
            let selector_target = fleet_admission_target_for_binding(&target);
            let principals = effective_fleet_admission_principals(policy, &selector_target);
            materialize_fleet_admission_projection(policy, target, principals)
                .map_err(|_error| InternalError::invariant())
        })
        .collect()
}

const fn target_principal(target: &ManagedCanisterBinding) -> Principal {
    match target {
        ManagedCanisterBinding::Component(component) => component.canister_id,
        ManagedCanisterBinding::ComponentChild(child) => child.canister_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentChildBinding,
        ComponentInstanceId, ComponentSpecId, FleetBinding, FleetCoordinatorBinding, FleetId,
        FleetKey, FleetRegistryAuthority, SubnetId,
    };

    #[test]
    fn maximum_production_target_prepare_command_fits_update_envelope() {
        let principal = |index: u16| {
            let mut bytes = [0xa5; 29];
            bytes[..2].copy_from_slice(&index.to_be_bytes());
            Principal::from_slice(&bytes)
        };
        let fleet = FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([0xe1; 32]),
            },
            app: AppId::from("a".repeat(40)),
        };
        let authority = FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet,
                coordinator_subnet: SubnetId::from_principal(principal(1)),
                coordinator: principal(2),
            },
            epoch: u64::MAX,
        };
        let component = ComponentBinding {
            authority,
            component: ComponentInstanceId::from_generated_bytes([0xe2; 32]),
            component_spec: ComponentSpecId::try_from("s".repeat(40))
                .expect("maximum Component Spec ID"),
            spec_hash: [0xe3; 32],
            role: CanisterRole::from("r".repeat(40)),
            placement_subnet: SubnetId::from_principal(principal(3)),
            fleet_subnet_root: principal(4),
            canister_id: principal(5),
        };
        let successor = FleetAdmissionProjection {
            schema_version: 1,
            authority: component.authority.binding.clone(),
            target: ManagedCanisterBinding::ComponentChild(ComponentChildBinding {
                component,
                parent_canister_id: principal(6),
                role: CanisterRole::from("c".repeat(40)),
                canister_id: principal(7),
            }),
            generation: u64::MAX,
            policy_digest: [0xe4; 32],
            projection_digest: [0xe5; 32],
            principals: (8_u16..264).map(principal).collect(),
        };
        let command = RemoteManagedCommand::PrepareFleetAdmission(Box::new(
            FleetAdmissionPrepareTargetRequest {
                operation_id: [0xe6; 32],
                expected_generation: u64::MAX - 1,
                expected_policy_digest: [0xe7; 32],
                successor,
            },
        ));

        let bytes = candid::encode_one(command).expect("production target command Candid");
        eprintln!(
            "maximum production target prepare command bytes: {}",
            bytes.len()
        );
        assert!(
            bytes.len() <= canic_core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
            "maximum production target command must fit the frozen 16 KiB envelope"
        );
    }
}
