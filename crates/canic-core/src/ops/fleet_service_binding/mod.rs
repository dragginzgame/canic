//! Module: ops::fleet_service_binding
//!
//! Responsibility: derive the complete initial Fleet-service member set from exact root receipts.
//! Does not own: Coordinator persistence, Fleet Registry mutation, or Directory publication.
//! Boundary: Coordinator workflow supplies one validated plan and every authenticated root receipt.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    config::{
        ComponentDeploymentConfiguration, ComponentDeploymentPurpose, ComponentTopology,
        ConfigModel, FleetServiceMemberPurpose, FleetServiceTarget, FleetServiceTargetMode,
    },
    dto::{
        component_provisioning::{
            ComponentGroupPlanEntry, FleetComponentProvisioningPlan,
            FleetSubnetRootProvisioningBatch, RootComponentProvisioningPhase,
            RootComponentProvisioningResult, RootComponentProvisioningStatusResponse,
            RootProvisionedGroupMember,
        },
        fleet_registry::{
            FleetRegistry, FleetServiceBinding, FleetServiceComponentBinding, FleetServiceMode,
        },
    },
    ids::{ComponentGroupPlacementId, ComponentInstanceId, FleetServiceId},
    ops::{
        OpsError,
        component_provisioning_plan::ComponentProvisioningPlanOps,
        component_provisioning_receipt::{
            RootComponentProvisioningProvisionedReceiptAuthority,
            RootComponentProvisioningReceiptOps,
        },
    },
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use candid::Principal;
use thiserror::Error as ThisError;

/// Typed rejection while deriving exact configured Fleet-service bindings.
#[derive(Debug, ThisError)]
pub enum FleetServiceBindingOpsError {
    #[error("Fleet-service binding compilation configuration failed: {0}")]
    Configuration(String),

    #[error("Fleet-service binding compilation reused Component identity {component}")]
    DuplicateComponentIdentity { component: ComponentInstanceId },

    #[error("Fleet-service binding compilation reused Canister principal {canister_id}")]
    DuplicateComponentPrincipal { canister_id: Principal },

    #[error("Fleet service '{service}' has more than one Authority member")]
    DuplicateServiceAuthority { service: FleetServiceId },

    #[error("Fleet-service binding compilation operation ID must be nonzero")]
    EmptyOperationId,

    #[error("Fleet service '{service}' has no configured initial members")]
    EmptyService { service: FleetServiceId },

    #[error("Fleet service '{service}' does not contain its configured Authority occurrence")]
    InvalidServiceAuthority { service: FleetServiceId },

    #[error("Fleet service '{service}' has a member incompatible with its configured mode")]
    InvalidServiceMemberPurpose { service: FleetServiceId },

    #[error("Fleet service '{service}' violates its configured density or spread policy")]
    InvalidServicePlacement { service: FleetServiceId },

    #[error("Fleet-service binding compilation plan failed validation: {0}")]
    Plan(String),

    #[error("root Provisioned receipt count {actual} differs from planned root count {expected}")]
    RootReceiptCountMismatch { actual: usize, expected: usize },

    #[error("root Provisioned receipt counts differ from its exact planned batch")]
    RootReceiptCountsMismatch,

    #[error(
        "root Provisioned receipt does not match its operation, plan, Registry, configuration or root"
    )]
    RootReceiptIdentityMismatch,

    #[error("root Provisioned receipt content hash is invalid")]
    RootReceiptInvalidHash,

    #[error("root Provisioned receipt result differs from its exact planned batch")]
    RootReceiptResultMismatch,

    #[error("root Provisioned receipt is not in the exact terminal Provisioned phase")]
    RootReceiptStateMismatch,

    #[error("root Provisioned receipt time evidence is invalid")]
    RootReceiptTimeMismatch,

    #[error("Fleet-service binding count arithmetic overflowed")]
    CountOverflow,

    #[error("Fleet-service binding compilation produced undeclared service '{service}'")]
    UnexpectedService { service: FleetServiceId },
}

/// Pure compiler for the complete configured initial Fleet-service binding set.
pub struct FleetServiceBindingOps;

impl FleetServiceBindingOps {
    /// Compile every initial service from one canonical plan and all exact root receipts.
    pub fn compile_initial(
        config: &ConfigModel,
        registry: &FleetRegistry,
        plan: &FleetComponentProvisioningPlan,
        operation_id: [u8; 32],
        root_receipts: &[RootComponentProvisioningStatusResponse],
    ) -> Result<Vec<FleetServiceBinding>, InternalError> {
        let configuration = config
            .compile_component_deployment_configuration()
            .map_err(|error| FleetServiceBindingOpsError::Configuration(error.to_string()))
            .map_err(OpsError::from)
            .map_err(InternalError::from)?;
        Self::compile_initial_compiled(&configuration, registry, plan, operation_id, root_receipts)
    }

    /// Compile initial services from one exact decoded provisioning configuration.
    pub fn compile_initial_compiled(
        configuration: &ComponentDeploymentConfiguration,
        registry: &FleetRegistry,
        plan: &FleetComponentProvisioningPlan,
        operation_id: [u8; 32],
        root_receipts: &[RootComponentProvisioningStatusResponse],
    ) -> Result<Vec<FleetServiceBinding>, InternalError> {
        compile_initial_compiled_configuration(
            configuration,
            registry,
            plan,
            operation_id,
            root_receipts,
        )
        .map_err(OpsError::from)
        .map_err(InternalError::from)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct RootReceiptIdentity<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    fleet_registry: &'a crate::dto::fleet_registry::FleetRegistryVersion,
    configuration_digest: crate::ids::ComponentDeploymentConfigurationDigest,
    fleet_subnet_root: Principal,
}

struct BindingCompilationAuthority<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    plan: &'a FleetComponentProvisioningPlan,
    component_topology: &'a ComponentTopology,
}

#[derive(Default)]
struct BindingCompilationLedger {
    candidates: BTreeMap<FleetServiceId, Vec<FleetServiceComponentBinding>>,
    components: BTreeSet<ComponentInstanceId>,
    principals: BTreeSet<Principal>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct RootReceiptCounts {
    placement_count: u32,
    component_count: u32,
    reserved_component_count: u32,
    claimed_component_count: u32,
    installed_component_count: u32,
    registry_committed_component_count: u32,
}

#[derive(Eq, PartialEq)]
struct ResultPlacementAuthority<'a> {
    group_placement: &'a ComponentGroupPlacementId,
    component_group: &'a crate::ids::ComponentGroupSpecId,
    member_count: usize,
}

#[derive(Eq, PartialEq)]
struct ResultMemberAuthority<'a> {
    member_path: &'a crate::ids::ComponentGroupMemberPath,
    component_spec: &'a crate::ids::ComponentSpecId,
    purpose: &'a ComponentDeploymentPurpose,
    limits: &'a crate::config::ComponentDeploymentLimits,
    binding_authority: &'a crate::ids::FleetRegistryAuthority,
    binding_component_spec: &'a crate::ids::ComponentSpecId,
    binding_spec_hash: [u8; 32],
    binding_role: &'a crate::ids::CanisterRole,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct ResultMemberPlacementAuthority {
    placement_subnet: crate::ids::SubnetId,
    fleet_subnet_root: Principal,
}

fn compile_initial_compiled_configuration(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
    operation_id: [u8; 32],
    root_receipts: &[RootComponentProvisioningStatusResponse],
) -> Result<Vec<FleetServiceBinding>, FleetServiceBindingOpsError> {
    if operation_id == [0; 32] {
        return Err(FleetServiceBindingOpsError::EmptyOperationId);
    }
    ComponentProvisioningPlanOps::validate_compiled(configuration, registry, plan)
        .map_err(|error| FleetServiceBindingOpsError::Plan(error.to_string()))?;
    let plan_hash = ComponentProvisioningPlanOps::hash_compiled(configuration, registry, plan)
        .map_err(|error| FleetServiceBindingOpsError::Plan(error.to_string()))?;
    if root_receipts.len() != plan.batches.len() {
        return Err(FleetServiceBindingOpsError::RootReceiptCountMismatch {
            actual: root_receipts.len(),
            expected: plan.batches.len(),
        });
    }
    let authority = BindingCompilationAuthority {
        operation_id,
        plan_hash,
        plan,
        component_topology: &configuration.component_topology,
    };
    let mut ledger = BindingCompilationLedger::default();

    for (batch, receipt) in plan.batches.iter().zip(root_receipts) {
        validate_root_receipt(batch, receipt, &authority, &mut ledger)?;
    }

    let services = configuration
        .fleet_service_topology
        .targets
        .iter()
        .map(|target| compile_service(target, &mut ledger.candidates))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(service) = ledger.candidates.into_keys().next() {
        return Err(FleetServiceBindingOpsError::UnexpectedService { service });
    }
    Ok(services)
}

#[cfg(test)]
fn compile_initial(
    config: &ConfigModel,
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
    operation_id: [u8; 32],
    root_receipts: &[RootComponentProvisioningStatusResponse],
) -> Result<Vec<FleetServiceBinding>, FleetServiceBindingOpsError> {
    let configuration = config
        .compile_component_deployment_configuration()
        .map_err(|error| FleetServiceBindingOpsError::Configuration(error.to_string()))?;
    compile_initial_compiled_configuration(
        &configuration,
        registry,
        plan,
        operation_id,
        root_receipts,
    )
}

fn validate_root_receipt(
    batch: &FleetSubnetRootProvisioningBatch,
    receipt: &RootComponentProvisioningStatusResponse,
    authority: &BindingCompilationAuthority<'_>,
    ledger: &mut BindingCompilationLedger,
) -> Result<(), FleetServiceBindingOpsError> {
    let expected_identity = RootReceiptIdentity {
        operation_id: authority.operation_id,
        plan_hash: authority.plan_hash,
        fleet_registry: &authority.plan.fleet_registry,
        configuration_digest: authority.plan.configuration_digest,
        fleet_subnet_root: batch.root.fleet_subnet_root,
    };
    let actual_identity = RootReceiptIdentity {
        operation_id: receipt.operation_id,
        plan_hash: receipt.plan_hash,
        fleet_registry: &receipt.fleet_registry,
        configuration_digest: receipt.configuration_digest,
        fleet_subnet_root: receipt.fleet_subnet_root,
    };
    if actual_identity != expected_identity {
        return Err(FleetServiceBindingOpsError::RootReceiptIdentityMismatch);
    }
    if receipt.phase != RootComponentProvisioningPhase::Provisioned {
        return Err(FleetServiceBindingOpsError::RootReceiptStateMismatch);
    }
    let result = receipt
        .result
        .as_ref()
        .ok_or(FleetServiceBindingOpsError::RootReceiptStateMismatch)?;
    let provisioned_at_ns = receipt
        .provisioned_at_ns
        .ok_or(FleetServiceBindingOpsError::RootReceiptTimeMismatch)?;
    if receipt.accepted_at_ns == 0 || provisioned_at_ns < receipt.accepted_at_ns {
        return Err(FleetServiceBindingOpsError::RootReceiptTimeMismatch);
    }
    validate_receipt_counts(batch, receipt)?;
    let expected_hash = RootComponentProvisioningReceiptOps::provisioned_content_hash(
        RootComponentProvisioningProvisionedReceiptAuthority {
            operation_id: authority.operation_id,
            plan_hash: authority.plan_hash,
            fleet_registry: &authority.plan.fleet_registry,
            configuration_digest: authority.plan.configuration_digest,
            root: &batch.root,
            result,
            accepted_at_ns: receipt.accepted_at_ns,
            provisioned_at_ns,
        },
    )
    .map_err(|error| FleetServiceBindingOpsError::Plan(error.to_string()))?;
    if receipt.receipt_content_hash != expected_hash {
        return Err(FleetServiceBindingOpsError::RootReceiptInvalidHash);
    }
    collect_result_members(batch, result, authority.component_topology, ledger)
}

fn validate_receipt_counts(
    batch: &FleetSubnetRootProvisioningBatch,
    receipt: &RootComponentProvisioningStatusResponse,
) -> Result<(), FleetServiceBindingOpsError> {
    let placement_count = u32::try_from(batch.placements.len())
        .map_err(|_| FleetServiceBindingOpsError::CountOverflow)?;
    let component_count = batch
        .placements
        .iter()
        .try_fold(0_u32, |total, placement| {
            let members = u32::try_from(placement.entries.len())
                .map_err(|_| FleetServiceBindingOpsError::CountOverflow)?;
            total
                .checked_add(members)
                .ok_or(FleetServiceBindingOpsError::CountOverflow)
        })?;
    let expected = RootReceiptCounts {
        placement_count,
        component_count,
        reserved_component_count: component_count,
        claimed_component_count: component_count,
        installed_component_count: component_count,
        registry_committed_component_count: component_count,
    };
    let actual = RootReceiptCounts {
        placement_count: receipt.placement_count,
        component_count: receipt.component_count,
        reserved_component_count: receipt.reserved_component_count,
        claimed_component_count: receipt.claimed_component_count,
        installed_component_count: receipt.installed_component_count,
        registry_committed_component_count: receipt.registry_committed_component_count,
    };
    if actual != expected {
        return Err(FleetServiceBindingOpsError::RootReceiptCountsMismatch);
    }
    Ok(())
}

fn collect_result_members(
    batch: &FleetSubnetRootProvisioningBatch,
    result: &RootComponentProvisioningResult,
    component_topology: &ComponentTopology,
    ledger: &mut BindingCompilationLedger,
) -> Result<(), FleetServiceBindingOpsError> {
    if result.placements.len() != batch.placements.len() {
        return Err(FleetServiceBindingOpsError::RootReceiptResultMismatch);
    }
    for (planned, provisioned) in batch.placements.iter().zip(&result.placements) {
        let expected = ResultPlacementAuthority {
            group_placement: &planned.group_placement,
            component_group: &planned.component_group,
            member_count: planned.entries.len(),
        };
        let actual = ResultPlacementAuthority {
            group_placement: &provisioned.group_placement,
            component_group: &provisioned.component_group,
            member_count: provisioned.members.len(),
        };
        if actual != expected {
            return Err(FleetServiceBindingOpsError::RootReceiptResultMismatch);
        }
        for (entry, member) in planned.entries.iter().zip(&provisioned.members) {
            validate_result_member(batch, entry, member, component_topology)?;
            if !ledger.components.insert(member.binding.component) {
                return Err(FleetServiceBindingOpsError::DuplicateComponentIdentity {
                    component: member.binding.component,
                });
            }
            if !ledger.principals.insert(member.binding.canister_id) {
                return Err(FleetServiceBindingOpsError::DuplicateComponentPrincipal {
                    canister_id: member.binding.canister_id,
                });
            }
            collect_service_candidate(
                &planned.group_placement,
                entry,
                member,
                &mut ledger.candidates,
            );
        }
    }
    Ok(())
}

fn validate_result_member(
    batch: &FleetSubnetRootProvisioningBatch,
    entry: &ComponentGroupPlanEntry,
    member: &RootProvisionedGroupMember,
    component_topology: &ComponentTopology,
) -> Result<(), FleetServiceBindingOpsError> {
    let spec = component_topology
        .get(&entry.component_spec)
        .ok_or(FleetServiceBindingOpsError::RootReceiptResultMismatch)?;
    let binding = &member.binding;
    let expected = ResultMemberAuthority {
        member_path: &entry.member_path,
        component_spec: &entry.component_spec,
        purpose: &entry.purpose,
        limits: &entry.limits,
        binding_authority: &batch.root.authority,
        binding_component_spec: &entry.component_spec,
        binding_spec_hash: entry.spec_hash,
        binding_role: &spec.component_role,
    };
    let actual = ResultMemberAuthority {
        member_path: &member.member_path,
        component_spec: &member.component_spec,
        purpose: &member.purpose,
        limits: &member.limits,
        binding_authority: &binding.authority,
        binding_component_spec: &binding.component_spec,
        binding_spec_hash: binding.spec_hash,
        binding_role: &binding.role,
    };
    if actual != expected {
        return Err(FleetServiceBindingOpsError::RootReceiptResultMismatch);
    }
    let expected_placement = ResultMemberPlacementAuthority {
        placement_subnet: batch.root.placement_subnet,
        fleet_subnet_root: batch.root.fleet_subnet_root,
    };
    let actual_placement = ResultMemberPlacementAuthority {
        placement_subnet: binding.placement_subnet,
        fleet_subnet_root: binding.fleet_subnet_root,
    };
    if actual_placement != expected_placement {
        return Err(FleetServiceBindingOpsError::RootReceiptResultMismatch);
    }
    if !result_member_identity_is_qualified(member) {
        return Err(FleetServiceBindingOpsError::RootReceiptResultMismatch);
    }
    Ok(())
}

fn result_member_identity_is_qualified(member: &RootProvisionedGroupMember) -> bool {
    if member.binding.component.as_bytes() == &[0; 32] {
        return false;
    }
    if member.binding.canister_id == Principal::anonymous() {
        return false;
    }
    if member.component_registry_revision == 0 {
        return false;
    }
    member.component_registry_content_hash != [0; 32]
}

fn collect_service_candidate(
    group_placement: &ComponentGroupPlacementId,
    entry: &ComponentGroupPlanEntry,
    member: &RootProvisionedGroupMember,
    candidates: &mut BTreeMap<FleetServiceId, Vec<FleetServiceComponentBinding>>,
) {
    let ComponentDeploymentPurpose::FleetServiceMember {
        service,
        member_purpose,
    } = &entry.purpose
    else {
        return;
    };
    candidates
        .entry(service.clone())
        .or_default()
        .push(FleetServiceComponentBinding {
            member_purpose: *member_purpose,
            component: member.binding.component,
            fleet_subnet_root: member.binding.fleet_subnet_root,
            canister_id: member.binding.canister_id,
            group_placement: group_placement.clone(),
            member_path: member.member_path.clone(),
        });
}

fn compile_service(
    target: &FleetServiceTarget,
    candidates: &mut BTreeMap<FleetServiceId, Vec<FleetServiceComponentBinding>>,
) -> Result<FleetServiceBinding, FleetServiceBindingOpsError> {
    let mut members = candidates.remove(&target.service).ok_or_else(|| {
        FleetServiceBindingOpsError::EmptyService {
            service: target.service.clone(),
        }
    })?;
    if members.is_empty() {
        return Err(FleetServiceBindingOpsError::EmptyService {
            service: target.service.clone(),
        });
    }
    members.sort_by(compare_members);
    let mode = validate_service_mode(target, &members)?;
    validate_service_placement(target, &members)?;
    Ok(FleetServiceBinding {
        service: target.service.clone(),
        role: target.role.clone(),
        component_spec: target.component_spec.clone(),
        mode,
        placement: target.placement,
        members,
    })
}

fn validate_service_mode(
    target: &FleetServiceTarget,
    members: &[FleetServiceComponentBinding],
) -> Result<FleetServiceMode, FleetServiceBindingOpsError> {
    match &target.mode {
        FleetServiceTargetMode::AuthorityReplica {
            authority_deployment,
            authority_member,
        } => {
            if members
                .iter()
                .any(|member| member.member_purpose == FleetServiceMemberPurpose::PoolMember)
            {
                return Err(FleetServiceBindingOpsError::InvalidServiceMemberPurpose {
                    service: target.service.clone(),
                });
            }
            let authorities = members
                .iter()
                .filter(|member| member.member_purpose == FleetServiceMemberPurpose::Authority)
                .collect::<Vec<_>>();
            if authorities.len() > 1 {
                return Err(FleetServiceBindingOpsError::DuplicateServiceAuthority {
                    service: target.service.clone(),
                });
            }
            let authority = authorities.first().ok_or_else(|| {
                FleetServiceBindingOpsError::InvalidServiceAuthority {
                    service: target.service.clone(),
                }
            })?;
            if authority.group_placement.deployment != *authority_deployment
                || authority.member_path != *authority_member
            {
                return Err(FleetServiceBindingOpsError::InvalidServiceAuthority {
                    service: target.service.clone(),
                });
            }
            Ok(FleetServiceMode::AuthorityReplica)
        }
        FleetServiceTargetMode::ActivePool => {
            if members
                .iter()
                .any(|member| member.member_purpose != FleetServiceMemberPurpose::PoolMember)
            {
                return Err(FleetServiceBindingOpsError::InvalidServiceMemberPurpose {
                    service: target.service.clone(),
                });
            }
            Ok(FleetServiceMode::ActivePool)
        }
    }
}

fn validate_service_placement(
    target: &FleetServiceTarget,
    members: &[FleetServiceComponentBinding],
) -> Result<(), FleetServiceBindingOpsError> {
    let mut root_counts = BTreeMap::<Principal, u32>::new();
    for member in members {
        let count = root_counts.entry(member.fleet_subnet_root).or_default();
        *count = count
            .checked_add(1)
            .ok_or(FleetServiceBindingOpsError::CountOverflow)?;
    }
    if root_counts
        .values()
        .any(|count| *count > target.placement.maximum_members_per_root)
    {
        return Err(FleetServiceBindingOpsError::InvalidServicePlacement {
            service: target.service.clone(),
        });
    }
    let member_count =
        u32::try_from(members.len()).map_err(|_| FleetServiceBindingOpsError::CountOverflow)?;
    let required_roots = member_count.min(target.placement.minimum_distinct_roots) as usize;
    if root_counts.len() < required_roots {
        return Err(FleetServiceBindingOpsError::InvalidServicePlacement {
            service: target.service.clone(),
        });
    }
    Ok(())
}

fn compare_members(
    left: &FleetServiceComponentBinding,
    right: &FleetServiceComponentBinding,
) -> Ordering {
    service_member_purpose_tag(left.member_purpose)
        .cmp(&service_member_purpose_tag(right.member_purpose))
        .then_with(|| left.group_placement.cmp(&right.group_placement))
        .then_with(|| left.member_path.cmp(&right.member_path))
        .then_with(|| left.component.cmp(&right.component))
}

const fn service_member_purpose_tag(purpose: FleetServiceMemberPurpose) -> u8 {
    match purpose {
        FleetServiceMemberPurpose::Authority => 0,
        FleetServiceMemberPurpose::Replica => 1,
        FleetServiceMemberPurpose::PoolMember => 2,
    }
}
