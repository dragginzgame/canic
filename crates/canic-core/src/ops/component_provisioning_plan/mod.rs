//! Module: ops::component_provisioning_plan
//!
//! Responsibility: validate and hash one canonical Fleet Component provisioning plan.
//! Does not own: root selection, durable journals, lifecycle effects, publication, or receipts.
//! Boundary: checked-in deployment authority and one exact Fleet Registry version constrain every
//! root, placement, member and protected limit before the plan can become durable intent.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    cdk::types::Cycles,
    config::{
        ComponentDeploymentConfiguration, ComponentDeploymentLimits, ComponentDeploymentPurpose,
        ComponentGroupDeploymentSpec, ComponentGroupDeploymentTopology, ComponentTopology,
        ConfigModel, FlattenedComponentGroupDeploymentMember, FleetServiceMemberPurpose,
        FleetServiceTopology, MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS,
    },
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, ComponentGroupPlanEntry,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
            FleetSubnetRootProvisioningBatch,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryVersion, FleetSubnetRootEntry, FleetSubnetRootStatus,
        },
    },
    ids::{
        CanisterRole, ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId,
        ComponentGroupMemberPath, ComponentSpecAdmission, ComponentSpecId, FleetRegistryAuthority,
        FleetServiceId, FleetSubnetRootBinding, FleetSubnetRootLimits,
    },
    ops::{OpsError, fleet_registry::FleetRegistryOps},
};
use candid::Principal;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error as ThisError;

const PLAN_DOMAIN: &[u8] = b"canic/fleet-component-provisioning-plan/v1";
const ROOT_BATCH_DOMAIN: &[u8] = b"canic/fleet-subnet-root-provisioning-batch/v1";
const PLAN_SCHEMA_VERSION: u32 = 1;

/// Maximum canonical bytes retained for one initial or scale-out provisioning plan.
pub const MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CANONICAL_BYTES: usize = 8_388_608;
/// Maximum selected roots in one provisioning plan.
pub const MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES: usize = 4_096;
/// Maximum roots in the synchronous Directory-confirmation barrier.
pub const MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CONFIRMATION_ROOTS: usize = 4_096;
/// Maximum new group placements in one provisioning plan.
pub const MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS: usize = 4_096;
/// Maximum new top-level Component occurrences in one provisioning plan.
pub const MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES: usize =
    MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS;
/// Maximum canonical bytes accepted for one root's exact batch.
pub const MAX_FLEET_SUBNET_ROOT_PROVISIONING_BATCH_CANONICAL_BYTES: usize = 8_388_608;
/// Maximum Candid payload bytes for the batch plus its fixed acceptance authority.
pub const MAX_FLEET_SUBNET_ROOT_PROVISIONING_ACCEPTANCE_PAYLOAD_BYTES: usize =
    MAX_FLEET_SUBNET_ROOT_PROVISIONING_BATCH_CANONICAL_BYTES + 65_536;
/// Maximum Candid payload bytes for one compact root Directory-publication command.
pub const MAX_FLEET_SUBNET_ROOT_COMPONENT_PUBLICATION_PAYLOAD_BYTES: usize = 65_536;
/// Maximum Candid payload bytes for one compact root runtime-activation command.
pub const MAX_FLEET_SUBNET_ROOT_COMPONENT_ACTIVATION_PAYLOAD_BYTES: usize = 65_536;

/// Validated local capacity and artifact facts derived from one exact root batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentProvisioningBatchValidation {
    pub placement_count: u32,
    pub component_count: u32,
    pub component_spec_counts: BTreeMap<ComponentSpecId, u32>,
    pub component_roles: BTreeSet<CanisterRole>,
}

/// Typed rejection for a noncanonical or unauthorized provisioning plan.
#[derive(Debug, ThisError)]
pub enum ComponentProvisioningPlanOpsError {
    #[error("provisioning plan canonical bytes exceed bound {maximum_bytes}: {actual_bytes}")]
    CanonicalBytesExceeded {
        actual_bytes: usize,
        maximum_bytes: usize,
    },

    #[error("provisioning plan batch count {actual} exceeds bound {maximum}")]
    BatchBoundExceeded { actual: usize, maximum: usize },

    #[error("provisioning plan Directory confirmation root count {actual} exceeds bound {maximum}")]
    ConfirmationRootBoundExceeded { actual: usize, maximum: usize },

    #[error("provisioning plan placement count {actual} exceeds bound {maximum}")]
    PlacementBoundExceeded { actual: usize, maximum: usize },

    #[error("provisioning plan Component entry count {actual} exceeds bound {maximum}")]
    EntryBoundExceeded { actual: usize, maximum: usize },

    #[error("provisioning plan Fleet does not match its Fleet Registry authority")]
    FleetMismatch,

    #[error("provisioning plan Fleet Registry version is not the exact current Registry version")]
    FleetRegistryVersionMismatch,

    #[error("provisioning plan configuration digest differs from checked-in App authority")]
    ConfigurationDigestMismatch,

    #[error("provisioning plan Directory confirmation roots are not strictly canonical")]
    NonCanonicalDirectoryConfirmationRoots,

    #[error("provisioning plan Directory confirmation root must not be anonymous")]
    AnonymousDirectoryConfirmationRoot,

    #[error("provisioning plan root batches are not in canonical principal order")]
    NonCanonicalBatchOrder,

    #[error("provisioning plan contains an empty root batch")]
    EmptyRootBatch,

    #[error("provisioning plan root batch does not match one exact active Fleet Registry root")]
    RootBindingMismatch,

    #[error("provisioning plan root release set differs from the Fleet Registry root")]
    RootReleaseSetMismatch,

    #[error("provisioning plan selected root is absent from Directory confirmation roots")]
    SelectedRootNotConfirmed,

    #[error("fresh-install Directory confirmation roots are not the complete active root set")]
    FreshInstallConfirmationRootSetMismatch,

    #[error("provisioning plan Directory confirmation root is not an active Registry root")]
    ConfirmationRootNotActive,

    #[error("fresh-install provisioning requires every Registry root to be Active")]
    FreshInstallRootNotActive,

    #[error("provisioning plan placements are not in canonical placement-ID order")]
    NonCanonicalPlacementOrder,

    #[error("provisioning plan repeats Component Group placement '{placement:?}'")]
    DuplicatePlacement {
        placement: crate::ids::ComponentGroupPlacementId,
    },

    #[error("provisioning plan references unknown deployment '{deployment}'")]
    UnknownDeployment {
        deployment: ComponentGroupDeploymentId,
    },

    #[error("provisioning plan placement names a different Component Group than its deployment")]
    ComponentGroupMismatch,

    #[error("provisioning plan placement entries differ from the complete flattened deployment")]
    PlacementEntriesMismatch,

    #[error("provisioning plan root has no admission for Component Spec used by its batch")]
    MissingRootAdmission,

    #[error("provisioning plan root batch exceeds one Component Spec admission")]
    RootAdmissionExceeded,

    #[error("provisioning plan root batch exceeds maximum Component instances")]
    RootComponentCapacityExceeded,

    #[error("provisioning plan root batch exceeds maximum Component Group placements")]
    RootGroupPlacementCapacityExceeded,

    #[error(
        "fresh-install placement count or ordinal set differs from configured initial placements"
    )]
    FreshInstallPlacementSetMismatch,

    #[error("fresh-install placement assignment violates deployment density or spread")]
    FreshInstallPlacementPolicyMismatch,

    #[error("fresh-install service assignment violates service density or spread")]
    FreshInstallServicePlacementPolicyMismatch,

    #[error("root provisioning batch violates local deployment density")]
    RootBatchDeploymentDensityExceeded,

    #[error("root provisioning batch violates local Fleet-service density")]
    RootBatchServiceDensityExceeded,

    #[error("scale-out validation requires the durable Coordinator placement ledger")]
    ScaleOutStateUnavailable,

    #[error("provisioning plan count arithmetic overflowed")]
    CountOverflow,

    #[error("provisioning plan configuration compilation failed: {0}")]
    Configuration(String),

    #[error("provisioning plan Fleet Registry validation failed: {0}")]
    FleetRegistry(String),
}

/// Deterministic validation and hashing boundary for provisioning plans.
pub struct ComponentProvisioningPlanOps;

impl ComponentProvisioningPlanOps {
    /// Validate one plan against checked-in configuration and the exact current Fleet Registry.
    pub fn validate(
        config: &ConfigModel,
        registry: &FleetRegistry,
        plan: &FleetComponentProvisioningPlan,
    ) -> Result<(), InternalError> {
        let compiled = compile_configuration(config)
            .map_err(OpsError::from)
            .map_err(InternalError::from)?;
        Self::validate_compiled(&compiled, registry, plan)
    }

    /// Validate one plan against an exact decoded compiled configuration authority.
    pub fn validate_compiled(
        configuration: &ComponentDeploymentConfiguration,
        registry: &FleetRegistry,
        plan: &FleetComponentProvisioningPlan,
    ) -> Result<(), InternalError> {
        validate_compiled_configuration(configuration, registry, plan)
            .map_err(OpsError::from)
            .map_err(InternalError::from)
    }

    /// Return the bounded canonical bytes covered by the plan hash.
    pub fn canonical_bytes(
        config: &ConfigModel,
        registry: &FleetRegistry,
        plan: &FleetComponentProvisioningPlan,
    ) -> Result<Vec<u8>, InternalError> {
        let compiled = compile_configuration(config)
            .map_err(OpsError::from)
            .map_err(InternalError::from)?;
        Self::canonical_bytes_compiled(&compiled, registry, plan)
    }

    /// Return canonical plan bytes under an exact decoded compiled configuration authority.
    pub fn canonical_bytes_compiled(
        configuration: &ComponentDeploymentConfiguration,
        registry: &FleetRegistry,
        plan: &FleetComponentProvisioningPlan,
    ) -> Result<Vec<u8>, InternalError> {
        canonical_bytes_compiled_configuration(configuration, registry, plan)
            .map_err(OpsError::from)
            .map_err(InternalError::from)
    }

    /// Return the SHA-256 identity of the complete validated canonical plan.
    pub fn hash(
        config: &ConfigModel,
        registry: &FleetRegistry,
        plan: &FleetComponentProvisioningPlan,
    ) -> Result<[u8; 32], InternalError> {
        let compiled = compile_configuration(config)
            .map_err(OpsError::from)
            .map_err(InternalError::from)?;
        Self::hash_compiled(&compiled, registry, plan)
    }

    /// Hash one plan under an exact decoded compiled configuration authority.
    pub fn hash_compiled(
        configuration: &ComponentDeploymentConfiguration,
        registry: &FleetRegistry,
        plan: &FleetComponentProvisioningPlan,
    ) -> Result<[u8; 32], InternalError> {
        let bytes = Self::canonical_bytes_compiled(configuration, registry, plan)?;
        Ok(Sha256::digest(bytes).into())
    }

    /// Validate one Coordinator-selected batch against current local and Registry authority.
    pub fn validate_root_batch(
        config: &ConfigModel,
        registry: &FleetRegistry,
        fleet_registry: &FleetRegistryVersion,
        configuration_digest: ComponentDeploymentConfigurationDigest,
        expected_root: &FleetSubnetRootBinding,
        batch: &FleetSubnetRootProvisioningBatch,
    ) -> Result<RootComponentProvisioningBatchValidation, InternalError> {
        let compiled = compile_configuration(config)
            .map_err(OpsError::from)
            .map_err(InternalError::from)?;
        validate_root_batch_compiled(
            &compiled,
            registry,
            fleet_registry,
            configuration_digest,
            expected_root,
            batch,
        )
        .map_err(OpsError::from)
        .map_err(InternalError::from)
    }

    /// Return the bounded canonical bytes of one already validated root batch.
    pub fn root_batch_canonical_bytes(
        config: &ConfigModel,
        registry: &FleetRegistry,
        fleet_registry: &FleetRegistryVersion,
        configuration_digest: ComponentDeploymentConfigurationDigest,
        expected_root: &FleetSubnetRootBinding,
        batch: &FleetSubnetRootProvisioningBatch,
    ) -> Result<Vec<u8>, InternalError> {
        let compiled = compile_configuration(config)
            .map_err(OpsError::from)
            .map_err(InternalError::from)?;
        validate_root_batch_compiled(
            &compiled,
            registry,
            fleet_registry,
            configuration_digest,
            expected_root,
            batch,
        )
        .map_err(OpsError::from)
        .map_err(InternalError::from)?;
        let mut encoder = CanonicalEncoder::with_domain(ROOT_BATCH_DOMAIN);
        encode_batch(&mut encoder, batch);
        encoder
            .finish_with_bound(MAX_FLEET_SUBNET_ROOT_PROVISIONING_BATCH_CANONICAL_BYTES)
            .map_err(OpsError::from)
            .map_err(InternalError::from)
    }
}

fn compile_configuration(
    config: &ConfigModel,
) -> Result<ComponentDeploymentConfiguration, ComponentProvisioningPlanOpsError> {
    config
        .compile_component_deployment_configuration()
        .map_err(|error| ComponentProvisioningPlanOpsError::Configuration(error.to_string()))
}

#[cfg(test)]
fn validate(
    config: &ConfigModel,
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let configuration = compile_configuration(config)?;
    validate_compiled_configuration(&configuration, registry, plan)
}

#[cfg(test)]
fn validate_root_batch(
    config: &ConfigModel,
    registry: &FleetRegistry,
    fleet_registry: &FleetRegistryVersion,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    expected_root: &FleetSubnetRootBinding,
    batch: &FleetSubnetRootProvisioningBatch,
) -> Result<RootComponentProvisioningBatchValidation, ComponentProvisioningPlanOpsError> {
    let configuration = compile_configuration(config)?;
    validate_root_batch_compiled(
        &configuration,
        registry,
        fleet_registry,
        configuration_digest,
        expected_root,
        batch,
    )
}

fn validate_compiled_configuration(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let expected_digest = configuration
        .digest()
        .map_err(|error| ComponentProvisioningPlanOpsError::Configuration(error.to_string()))?;
    validate_plan_authority(
        registry,
        plan,
        &configuration.component_topology,
        expected_digest,
    )?;
    validate_bounds(plan)?;
    validate_confirmation_roots(registry, plan)?;

    let mut ledger = PlanValidationLedger::new();
    let mut previous_root = None;
    for batch in &plan.batches {
        let root = batch.root.fleet_subnet_root;
        if previous_root.is_some_and(|previous| previous >= root) {
            return Err(ComponentProvisioningPlanOpsError::NonCanonicalBatchOrder);
        }
        previous_root = Some(root);
        let _validation = validate_batch(
            registry,
            batch,
            &configuration.component_topology,
            &configuration.deployment_topology,
            &mut ledger,
        )?;
        if plan
            .directory_confirmation_roots
            .binary_search(&batch.root.fleet_subnet_root)
            .is_err()
        {
            return Err(ComponentProvisioningPlanOpsError::SelectedRootNotConfirmed);
        }
    }

    validate_operation(
        plan,
        &configuration.deployment_topology,
        &configuration.fleet_service_topology,
        &ledger,
    )
}

fn validate_root_batch_compiled(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    fleet_registry: &FleetRegistryVersion,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    expected_root: &FleetSubnetRootBinding,
    batch: &FleetSubnetRootProvisioningBatch,
) -> Result<RootComponentProvisioningBatchValidation, ComponentProvisioningPlanOpsError> {
    let expected_digest = configuration
        .digest()
        .map_err(|error| ComponentProvisioningPlanOpsError::Configuration(error.to_string()))?;
    if configuration_digest != expected_digest {
        return Err(ComponentProvisioningPlanOpsError::ConfigurationDigestMismatch);
    }
    let expected_registry = FleetRegistryOps::version(
        &registry.authority,
        &configuration.component_topology,
        registry,
    )
    .map_err(|error| ComponentProvisioningPlanOpsError::FleetRegistry(error.to_string()))?;
    if fleet_registry != &expected_registry {
        return Err(ComponentProvisioningPlanOpsError::FleetRegistryVersionMismatch);
    }
    if expected_root != &batch.root {
        return Err(ComponentProvisioningPlanOpsError::RootBindingMismatch);
    }
    validate_root_batch_bounds(batch)?;
    let mut ledger = PlanValidationLedger::new();
    let validation = validate_batch(
        registry,
        batch,
        &configuration.component_topology,
        &configuration.deployment_topology,
        &mut ledger,
    )?;
    validate_root_batch_density(
        batch,
        &configuration.deployment_topology,
        &configuration.fleet_service_topology,
    )?;
    Ok(validation)
}

fn validate_plan_authority(
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
    topology: &ComponentTopology,
    expected_digest: ComponentDeploymentConfigurationDigest,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    if plan.fleet != registry.authority.binding.fleet
        || plan.fleet_registry.authority.binding.fleet != plan.fleet
    {
        return Err(ComponentProvisioningPlanOpsError::FleetMismatch);
    }
    let expected_version = FleetRegistryOps::version(&registry.authority, topology, registry)
        .map_err(|error| ComponentProvisioningPlanOpsError::FleetRegistry(error.to_string()))?;
    if plan.fleet_registry != expected_version {
        return Err(ComponentProvisioningPlanOpsError::FleetRegistryVersionMismatch);
    }
    if plan.configuration_digest != expected_digest {
        return Err(ComponentProvisioningPlanOpsError::ConfigurationDigestMismatch);
    }
    Ok(())
}

fn validate_bounds(
    plan: &FleetComponentProvisioningPlan,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    if plan.batches.len() > MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES {
        return Err(ComponentProvisioningPlanOpsError::BatchBoundExceeded {
            actual: plan.batches.len(),
            maximum: MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES,
        });
    }
    if plan.directory_confirmation_roots.len()
        > MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CONFIRMATION_ROOTS
    {
        return Err(
            ComponentProvisioningPlanOpsError::ConfirmationRootBoundExceeded {
                actual: plan.directory_confirmation_roots.len(),
                maximum: MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CONFIRMATION_ROOTS,
            },
        );
    }
    let mut placements = 0_usize;
    let mut entries = 0_usize;
    for batch in &plan.batches {
        placements = placements
            .checked_add(batch.placements.len())
            .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
        for placement in &batch.placements {
            entries = entries
                .checked_add(placement.entries.len())
                .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
        }
    }
    if placements > MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS {
        return Err(ComponentProvisioningPlanOpsError::PlacementBoundExceeded {
            actual: placements,
            maximum: MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS,
        });
    }
    if entries > MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES {
        return Err(ComponentProvisioningPlanOpsError::EntryBoundExceeded {
            actual: entries,
            maximum: MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES,
        });
    }
    Ok(())
}

fn validate_root_batch_bounds(
    batch: &FleetSubnetRootProvisioningBatch,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    if batch.placements.len() > MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS {
        return Err(ComponentProvisioningPlanOpsError::PlacementBoundExceeded {
            actual: batch.placements.len(),
            maximum: MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS,
        });
    }
    let entries = batch
        .placements
        .iter()
        .try_fold(0_usize, |total, placement| {
            total
                .checked_add(placement.entries.len())
                .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)
        })?;
    if entries > MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES {
        return Err(ComponentProvisioningPlanOpsError::EntryBoundExceeded {
            actual: entries,
            maximum: MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES,
        });
    }
    Ok(())
}

fn validate_confirmation_roots(
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let mut previous = None;
    for root in &plan.directory_confirmation_roots {
        if *root == Principal::anonymous() {
            return Err(ComponentProvisioningPlanOpsError::AnonymousDirectoryConfirmationRoot);
        }
        if previous.is_some_and(|previous| previous >= *root) {
            return Err(ComponentProvisioningPlanOpsError::NonCanonicalDirectoryConfirmationRoots);
        }
        previous = Some(*root);
        let is_active = registry.fleet_subnet_roots.iter().any(|entry| {
            entry.fleet_subnet_root == *root && entry.status == FleetSubnetRootStatus::Active
        });
        if !is_active {
            return Err(ComponentProvisioningPlanOpsError::ConfirmationRootNotActive);
        }
    }
    if matches!(
        plan.operation,
        FleetComponentProvisioningOperation::FreshInstall
    ) {
        if registry
            .fleet_subnet_roots
            .iter()
            .any(|root| root.status != FleetSubnetRootStatus::Active)
        {
            return Err(ComponentProvisioningPlanOpsError::FreshInstallRootNotActive);
        }
        let mut expected = registry
            .fleet_subnet_roots
            .iter()
            .map(|root| root.fleet_subnet_root)
            .collect::<Vec<_>>();
        expected.sort_unstable();
        if plan.directory_confirmation_roots != expected {
            return Err(ComponentProvisioningPlanOpsError::FreshInstallConfirmationRootSetMismatch);
        }
    }
    Ok(())
}

fn validate_batch(
    registry: &FleetRegistry,
    batch: &FleetSubnetRootProvisioningBatch,
    component_topology: &ComponentTopology,
    deployment_topology: &ComponentGroupDeploymentTopology,
    ledger: &mut PlanValidationLedger,
) -> Result<RootComponentProvisioningBatchValidation, ComponentProvisioningPlanOpsError> {
    if batch.placements.is_empty() {
        return Err(ComponentProvisioningPlanOpsError::EmptyRootBatch);
    }
    let registry_root = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == batch.root.fleet_subnet_root)
        .ok_or(ComponentProvisioningPlanOpsError::RootBindingMismatch)?;
    validate_root_binding(registry, registry_root, batch, component_topology)?;
    let placement_count = u32::try_from(batch.placements.len())
        .map_err(|_| ComponentProvisioningPlanOpsError::CountOverflow)?;
    if placement_count > batch.root.limits.maximum_group_placements {
        return Err(ComponentProvisioningPlanOpsError::RootGroupPlacementCapacityExceeded);
    }
    let mut component_count = 0_u32;
    let mut spec_counts = BTreeMap::new();
    let mut component_roles = BTreeSet::new();
    let mut previous_placement = None;
    for placement in &batch.placements {
        if previous_placement
            .as_ref()
            .is_some_and(|previous| previous >= &placement.group_placement)
        {
            return Err(ComponentProvisioningPlanOpsError::NonCanonicalPlacementOrder);
        }
        previous_placement = Some(placement.group_placement.clone());
        validate_placement(
            placement,
            deployment_topology,
            batch.root.fleet_subnet_root,
            ledger,
        )?;
        component_count = component_count
            .checked_add(
                u32::try_from(placement.entries.len())
                    .map_err(|_| ComponentProvisioningPlanOpsError::CountOverflow)?,
            )
            .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
        for entry in &placement.entries {
            let count = spec_counts
                .entry(entry.component_spec.clone())
                .or_insert(0_u32);
            *count = count
                .checked_add(1)
                .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
            let role = component_topology
                .get(&entry.component_spec)
                .ok_or(ComponentProvisioningPlanOpsError::MissingRootAdmission)?
                .component_role
                .clone();
            component_roles.insert(role);
        }
    }
    if component_count > batch.root.limits.maximum_component_instances {
        return Err(ComponentProvisioningPlanOpsError::RootComponentCapacityExceeded);
    }
    validate_spec_admissions(&batch.root.component_admissions, &spec_counts)?;
    Ok(RootComponentProvisioningBatchValidation {
        placement_count,
        component_count,
        component_spec_counts: spec_counts,
        component_roles,
    })
}

fn validate_root_binding(
    registry: &FleetRegistry,
    registry_root: &FleetSubnetRootEntry,
    batch: &FleetSubnetRootProvisioningBatch,
    topology: &ComponentTopology,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    if registry_root.status != FleetSubnetRootStatus::Active {
        return Err(ComponentProvisioningPlanOpsError::RootBindingMismatch);
    }
    let expected = registry_root_binding(&registry.authority, registry_root);
    if batch.root != expected {
        return Err(ComponentProvisioningPlanOpsError::RootBindingMismatch);
    }
    topology
        .validate_planned_root(
            &batch.root.component_admissions,
            batch.root.component_topology_digest,
            &batch.root.limits,
        )
        .map_err(|error| ComponentProvisioningPlanOpsError::Configuration(error.to_string()))?;
    if batch.active_release_set != registry_root.active_release_set {
        return Err(ComponentProvisioningPlanOpsError::RootReleaseSetMismatch);
    }
    Ok(())
}

fn registry_root_binding(
    authority: &FleetRegistryAuthority,
    root: &FleetSubnetRootEntry,
) -> crate::ids::FleetSubnetRootBinding {
    crate::ids::FleetSubnetRootBinding {
        authority: authority.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        limits: root.limits.clone(),
    }
}

fn validate_placement(
    placement: &ComponentGroupPlacementPlan,
    topology: &ComponentGroupDeploymentTopology,
    root: Principal,
    ledger: &mut PlanValidationLedger,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    if !ledger.placements.insert(placement.group_placement.clone()) {
        return Err(ComponentProvisioningPlanOpsError::DuplicatePlacement {
            placement: placement.group_placement.clone(),
        });
    }
    let deployment = topology
        .get(&placement.group_placement.deployment)
        .ok_or_else(|| ComponentProvisioningPlanOpsError::UnknownDeployment {
            deployment: placement.group_placement.deployment.clone(),
        })?;
    if placement.component_group != deployment.component_group {
        return Err(ComponentProvisioningPlanOpsError::ComponentGroupMismatch);
    }
    if !entries_match(&placement.entries, &deployment.members) {
        return Err(ComponentProvisioningPlanOpsError::PlacementEntriesMismatch);
    }
    ledger.record(placement, root);
    Ok(())
}

fn entries_match(
    entries: &[ComponentGroupPlanEntry],
    expected: &[FlattenedComponentGroupDeploymentMember],
) -> bool {
    entries.len() == expected.len()
        && entries
            .iter()
            .zip(expected)
            .all(|(entry, expected)| entry_matches(entry, expected))
}

fn entry_matches(
    entry: &ComponentGroupPlanEntry,
    expected: &FlattenedComponentGroupDeploymentMember,
) -> bool {
    let occurrence_is_exact = entry.member_path == expected.member_path
        && entry.component_spec == expected.component_spec;
    let artifact_is_exact = entry.spec_hash == expected.component_spec_hash;
    let policy_is_exact = entry.purpose == expected.purpose
        && entry.labels == expected.labels
        && entry.limits == expected.limits;
    occurrence_is_exact && artifact_is_exact && policy_is_exact
}

fn validate_spec_admissions(
    admissions: &[ComponentSpecAdmission],
    spec_counts: &BTreeMap<crate::ids::ComponentSpecId, u32>,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    for (component_spec, count) in spec_counts {
        let admission = admissions
            .binary_search_by(|admission| admission.component_spec.cmp(component_spec))
            .ok()
            .map(|index| &admissions[index])
            .ok_or(ComponentProvisioningPlanOpsError::MissingRootAdmission)?;
        if *count > admission.maximum_root_instances {
            return Err(ComponentProvisioningPlanOpsError::RootAdmissionExceeded);
        }
    }
    Ok(())
}

fn validate_operation(
    plan: &FleetComponentProvisioningPlan,
    topology: &ComponentGroupDeploymentTopology,
    service_topology: &FleetServiceTopology,
    ledger: &PlanValidationLedger,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    match &plan.operation {
        FleetComponentProvisioningOperation::FreshInstall => {
            validate_fresh_install(topology, service_topology, ledger)
        }
        FleetComponentProvisioningOperation::ScaleOut { .. } => {
            Err(ComponentProvisioningPlanOpsError::ScaleOutStateUnavailable)
        }
    }
}

fn validate_fresh_install(
    topology: &ComponentGroupDeploymentTopology,
    service_topology: &FleetServiceTopology,
    ledger: &PlanValidationLedger,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    for deployment in &topology.component_group_deployments {
        let placements = ledger.for_deployment(&deployment.deployment);
        if placements.len() != deployment.initial_placements as usize {
            return Err(ComponentProvisioningPlanOpsError::FreshInstallPlacementSetMismatch);
        }
        let mut ordinals = placements
            .iter()
            .map(|placement| placement.ordinal)
            .collect::<Vec<_>>();
        ordinals.sort_unstable();
        if ordinals
            .iter()
            .copied()
            .ne(0..deployment.initial_placements)
        {
            return Err(ComponentProvisioningPlanOpsError::FreshInstallPlacementSetMismatch);
        }
        validate_fresh_placement_policy(deployment, ledger)?;
    }
    validate_fresh_service_placement_policy(service_topology, ledger)
}

fn validate_fresh_placement_policy(
    deployment: &ComponentGroupDeploymentSpec,
    ledger: &PlanValidationLedger,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let roots = ledger.root_counts(&deployment.deployment);
    if roots
        .values()
        .any(|count| *count > deployment.placement.maximum_per_root)
    {
        return Err(ComponentProvisioningPlanOpsError::FreshInstallPlacementPolicyMismatch);
    }
    let required_roots = deployment
        .initial_placements
        .min(deployment.placement.minimum_distinct_roots) as usize;
    if roots.len() < required_roots {
        return Err(ComponentProvisioningPlanOpsError::FreshInstallPlacementPolicyMismatch);
    }
    Ok(())
}

fn validate_fresh_service_placement_policy(
    topology: &FleetServiceTopology,
    ledger: &PlanValidationLedger,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    for target in &topology.targets {
        let roots = ledger.service_root_counts(&target.service);
        if roots
            .values()
            .any(|count| *count > target.placement.maximum_members_per_root)
        {
            return Err(
                ComponentProvisioningPlanOpsError::FreshInstallServicePlacementPolicyMismatch,
            );
        }
        let members = roots.values().try_fold(0_u32, |total, count| {
            total
                .checked_add(*count)
                .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)
        })?;
        let required_roots = members.min(target.placement.minimum_distinct_roots) as usize;
        if roots.len() < required_roots {
            return Err(
                ComponentProvisioningPlanOpsError::FreshInstallServicePlacementPolicyMismatch,
            );
        }
    }
    Ok(())
}

fn validate_root_batch_density(
    batch: &FleetSubnetRootProvisioningBatch,
    deployment_topology: &ComponentGroupDeploymentTopology,
    service_topology: &FleetServiceTopology,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    let mut deployment_counts = BTreeMap::<ComponentGroupDeploymentId, u32>::new();
    let mut service_counts = BTreeMap::<FleetServiceId, u32>::new();
    for placement in &batch.placements {
        let deployment_count = deployment_counts
            .entry(placement.group_placement.deployment.clone())
            .or_default();
        *deployment_count = deployment_count
            .checked_add(1)
            .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
        for entry in &placement.entries {
            if let ComponentDeploymentPurpose::FleetServiceMember { service, .. } = &entry.purpose {
                let service_count = service_counts.entry(service.clone()).or_default();
                *service_count = service_count
                    .checked_add(1)
                    .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
            }
        }
    }
    for (deployment, count) in deployment_counts {
        let maximum = deployment_topology
            .get(&deployment)
            .ok_or(ComponentProvisioningPlanOpsError::UnknownDeployment { deployment })?
            .placement
            .maximum_per_root;
        if count > maximum {
            return Err(ComponentProvisioningPlanOpsError::RootBatchDeploymentDensityExceeded);
        }
    }
    for (service, count) in service_counts {
        let maximum = service_topology
            .get(&service)
            .ok_or_else(|| {
                ComponentProvisioningPlanOpsError::Configuration(format!(
                    "root batch references unknown Fleet service '{service}'"
                ))
            })?
            .placement
            .maximum_members_per_root;
        if count > maximum {
            return Err(ComponentProvisioningPlanOpsError::RootBatchServiceDensityExceeded);
        }
    }
    Ok(())
}

struct PlanValidationLedger {
    placements: BTreeSet<crate::ids::ComponentGroupPlacementId>,
    placement_roots: BTreeMap<crate::ids::ComponentGroupPlacementId, Principal>,
    service_roots: BTreeMap<FleetServiceId, BTreeMap<Principal, u32>>,
}

impl PlanValidationLedger {
    const fn new() -> Self {
        Self {
            placements: BTreeSet::new(),
            placement_roots: BTreeMap::new(),
            service_roots: BTreeMap::new(),
        }
    }

    fn record(&mut self, placement: &ComponentGroupPlacementPlan, root: Principal) {
        self.placement_roots
            .insert(placement.group_placement.clone(), root);
        for entry in &placement.entries {
            if let ComponentDeploymentPurpose::FleetServiceMember { service, .. } = &entry.purpose {
                *self
                    .service_roots
                    .entry(service.clone())
                    .or_default()
                    .entry(root)
                    .or_default() += 1;
            }
        }
    }

    fn for_deployment(
        &self,
        deployment: &ComponentGroupDeploymentId,
    ) -> Vec<&crate::ids::ComponentGroupPlacementId> {
        self.placements
            .iter()
            .filter(|placement| &placement.deployment == deployment)
            .collect()
    }

    fn root_counts(&self, deployment: &ComponentGroupDeploymentId) -> BTreeMap<Principal, u32> {
        let mut counts = BTreeMap::new();
        for (placement, root) in &self.placement_roots {
            if &placement.deployment == deployment {
                *counts.entry(*root).or_insert(0) += 1;
            }
        }
        counts
    }

    fn service_root_counts(&self, service: &FleetServiceId) -> BTreeMap<Principal, u32> {
        self.service_roots.get(service).cloned().unwrap_or_default()
    }
}

fn canonical_bytes_compiled_configuration(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
) -> Result<Vec<u8>, ComponentProvisioningPlanOpsError> {
    validate_compiled_configuration(configuration, registry, plan)?;
    let mut encoder = CanonicalEncoder::new();
    encode_plan(&mut encoder, plan);
    encoder.finish()
}

#[cfg(test)]
fn canonical_bytes(
    config: &ConfigModel,
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
) -> Result<Vec<u8>, ComponentProvisioningPlanOpsError> {
    let configuration = compile_configuration(config)?;
    canonical_bytes_compiled_configuration(&configuration, registry, plan)
}

fn encode_plan(encoder: &mut CanonicalEncoder, plan: &FleetComponentProvisioningPlan) {
    encode_fleet(encoder, &plan.fleet);
    encode_registry_version(encoder, &plan.fleet_registry);
    encoder.bytes(plan.configuration_digest.as_bytes());
    encode_operation(encoder, &plan.operation);
    encoder.u64(plan.directory_confirmation_roots.len() as u64);
    for root in &plan.directory_confirmation_roots {
        encoder.bytes(root.as_slice());
    }
    encoder.u64(plan.batches.len() as u64);
    for batch in &plan.batches {
        encode_batch(encoder, batch);
    }
}

fn encode_fleet(encoder: &mut CanonicalEncoder, fleet: &crate::ids::FleetBinding) {
    encoder.bytes(fleet.fleet.canonical_network_id.as_bytes());
    encoder.bytes(fleet.fleet.fleet_id.as_bytes());
    encoder.string(fleet.app.as_str());
}

fn encode_registry_version(encoder: &mut CanonicalEncoder, version: &FleetRegistryVersion) {
    encode_authority(encoder, &version.authority);
    encoder.u64(version.revision);
    encoder.bytes(&version.content_hash);
}

fn encode_authority(encoder: &mut CanonicalEncoder, authority: &FleetRegistryAuthority) {
    encode_fleet(encoder, &authority.binding.fleet);
    encoder.bytes(
        authority
            .binding
            .coordinator_subnet
            .as_principal()
            .as_slice(),
    );
    encoder.bytes(authority.binding.coordinator.as_slice());
    encoder.u64(authority.epoch);
}

fn encode_operation(
    encoder: &mut CanonicalEncoder,
    operation: &FleetComponentProvisioningOperation,
) {
    match operation {
        FleetComponentProvisioningOperation::FreshInstall => encoder.u8(0),
        FleetComponentProvisioningOperation::ScaleOut {
            deployment,
            previous_placements,
            requested_placements,
        } => {
            encoder.u8(1);
            encoder.string(deployment.as_str());
            encoder.u32(*previous_placements);
            encoder.u32(*requested_placements);
        }
    }
}

fn encode_batch(encoder: &mut CanonicalEncoder, batch: &FleetSubnetRootProvisioningBatch) {
    encode_authority(encoder, &batch.root.authority);
    encoder.bytes(batch.root.placement_subnet.as_principal().as_slice());
    encoder.bytes(batch.root.fleet_subnet_root.as_slice());
    encoder.u64(batch.root.component_admissions.len() as u64);
    for admission in &batch.root.component_admissions {
        encoder.string(admission.component_spec.as_str());
        encoder.bytes(&admission.spec_hash);
        encoder.u32(admission.maximum_root_instances);
    }
    encoder.bytes(batch.root.component_topology_digest.as_bytes());
    encode_root_limits(encoder, &batch.root.limits);
    encoder.bytes(batch.active_release_set.release_build_id.as_bytes());
    encoder.bytes(batch.active_release_set.manifest_digest.as_bytes());
    encoder.u64(batch.placements.len() as u64);
    for placement in &batch.placements {
        encode_placement(encoder, placement);
    }
}

fn encode_root_limits(encoder: &mut CanonicalEncoder, limits: &FleetSubnetRootLimits) {
    encoder.u32(limits.maximum_component_instances);
    encoder.u64(limits.maximum_registry_bytes);
    encoder.u64(limits.maximum_wasm_store_bytes);
    encoder.u32(limits.canister_pool.minimum_size);
    encoder.u32(limits.canister_pool.maximum_size);
    encode_cycles(encoder, &limits.canister_pool.canister_cycles);
    encoder.u64(limits.cycles_funding.window_secs);
    encode_cycles(encoder, &limits.cycles_funding.maximum_cycles);
    encoder.u32(limits.maximum_group_placements);
}

fn encode_cycles(encoder: &mut CanonicalEncoder, cycles: &Cycles) {
    encoder.u128(cycles.to_u128());
}

fn encode_placement(encoder: &mut CanonicalEncoder, placement: &ComponentGroupPlacementPlan) {
    encoder.string(placement.group_placement.deployment.as_str());
    encoder.u32(placement.group_placement.ordinal);
    encoder.string(placement.component_group.as_str());
    encoder.u64(placement.entries.len() as u64);
    for entry in &placement.entries {
        encode_entry(encoder, entry);
    }
}

fn encode_entry(encoder: &mut CanonicalEncoder, entry: &ComponentGroupPlanEntry) {
    encode_member_path(encoder, &entry.member_path);
    encoder.string(entry.component_spec.as_str());
    encoder.bytes(&entry.spec_hash);
    encode_purpose(encoder, &entry.purpose);
    encoder.u64(entry.labels.len() as u64);
    for label in &entry.labels {
        encoder.string(label.key.as_str());
        encoder.string(label.value.as_str());
    }
    encode_limits(encoder, &entry.limits);
}

fn encode_member_path(encoder: &mut CanonicalEncoder, path: &ComponentGroupMemberPath) {
    encoder.u64(path.len() as u64);
    for member in path.as_slice() {
        encoder.string(member.as_str());
    }
}

fn encode_purpose(encoder: &mut CanonicalEncoder, purpose: &ComponentDeploymentPurpose) {
    match purpose {
        ComponentDeploymentPurpose::Ordinary => encoder.u8(0),
        ComponentDeploymentPurpose::FleetServiceMember {
            service,
            member_purpose,
        } => {
            encoder.u8(1);
            encoder.string(service.as_str());
            encoder.u8(match member_purpose {
                FleetServiceMemberPurpose::Authority => 0,
                FleetServiceMemberPurpose::Replica => 1,
                FleetServiceMemberPurpose::PoolMember => 2,
            });
        }
    }
}

fn encode_limits(encoder: &mut CanonicalEncoder, limits: &ComponentDeploymentLimits) {
    encoder.u32(limits.maximum_descendants);
    encoder.u64(limits.maximum_registry_bytes);
    encoder.u64(limits.spawn_grant_reductions.len() as u64);
    for grant in &limits.spawn_grant_reductions {
        encoder.string(grant.parent_role.as_str());
        encoder.string(grant.child_role.as_str());
        encoder.u32(grant.maximum_instances_per_parent);
    }
}

struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    fn new() -> Self {
        Self::with_domain(PLAN_DOMAIN)
    }

    fn with_domain(domain: &[u8]) -> Self {
        let mut encoder = Self { bytes: Vec::new() };
        encoder.bytes(domain);
        encoder.u32(PLAN_SCHEMA_VERSION);
        encoder
    }

    fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn finish(self) -> Result<Vec<u8>, ComponentProvisioningPlanOpsError> {
        self.finish_with_bound(MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CANONICAL_BYTES)
    }

    fn finish_with_bound(
        self,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, ComponentProvisioningPlanOpsError> {
        if self.bytes.len() > maximum_bytes {
            return Err(ComponentProvisioningPlanOpsError::CanonicalBytesExceeded {
                actual_bytes: self.bytes.len(),
                maximum_bytes,
            });
        }
        Ok(self.bytes)
    }
}
