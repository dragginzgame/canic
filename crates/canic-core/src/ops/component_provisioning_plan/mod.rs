//! Module: ops::component_provisioning_plan
//!
//! Responsibility: validate and hash one canonical Fleet Component provisioning plan.
//! Does not own: root selection, durable journals, lifecycle effects, publication, or receipts.
//! Boundary: checked-in deployment authority and one exact Fleet Registry version constrain every
//! root, placement, member and protected limit before the plan can become durable intent.

mod canonical;
mod scale_out;
#[cfg(test)]
mod tests;

pub use scale_out::{
    ComponentProvisioningPlacementAuthority, ComponentProvisioningScaleOutAuthority,
};

use crate::{
    InternalError,
    config::{
        ComponentDeploymentConfiguration, ComponentDeploymentPurpose, ComponentGroupDeploymentSpec,
        ComponentGroupDeploymentTopology, ComponentTopology, ConfigModel,
        FlattenedComponentGroupDeploymentMember, FleetServiceTopology,
        MAX_COMPONENT_GROUP_DEPLOYMENT_MEMBERS,
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
        ComponentSpecAdmission, ComponentSpecId, FleetRegistryAuthority, FleetServiceId,
        FleetSubnetRootBinding,
    },
    ops::{OpsError, fleet_registry::FleetRegistryOps},
};
use candid::Principal;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error as ThisError;

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

    #[error("provisioning plan root batch does not match one exact active Fleet Registry root")]
    RootBindingMismatch,

    #[error("provisioning plan root release set differs from the Fleet Registry root")]
    RootReleaseSetMismatch,

    #[error("provisioning plan selected root is absent from Directory confirmation roots")]
    SelectedRootNotConfirmed,

    #[error("fresh-install Directory confirmation roots are not the complete active root set")]
    FreshInstallConfirmationRootSetMismatch,

    #[error("fresh-install root batches are not the complete active root set")]
    FreshInstallBatchRootSetMismatch,

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

    #[error("scale-out committed placement authority is not strictly canonical")]
    NonCanonicalCommittedPlacements,

    #[error("scale-out eligible root authority is not strictly canonical")]
    NonCanonicalEligibleRoots,

    #[error("scale-out committed or selected root is outside the installed root set")]
    ScaleOutRootIneligible,

    #[error("scale-out desired counts do not form one bounded monotonic increase")]
    ScaleOutCountMismatch,

    #[error("scale-out placement IDs do not equal the next reserved ordinal range")]
    ScaleOutPlacementSetMismatch,

    #[error("scale-out plan contains a placement for a different deployment")]
    ScaleOutDeploymentMismatch,

    #[error("scale-out deployment contains an Authority occurrence")]
    ScaleOutAuthorityDeployment,

    #[error("scale-out placement assignment violates deployment density or spread")]
    ScaleOutPlacementPolicyMismatch,

    #[error("scale-out service assignment violates service density or spread")]
    ScaleOutServicePlacementPolicyMismatch,

    #[error("scale-out Directory confirmation roots differ from selected and affected roots")]
    ScaleOutConfirmationRootSetMismatch,

    #[error("scale-out root batch must contain at least one new placement")]
    EmptyScaleOutBatch,

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

    /// Validate one scale-out plan against its durable placement and installed-root authority.
    pub fn validate_scale_out_compiled(
        configuration: &ComponentDeploymentConfiguration,
        registry: &FleetRegistry,
        plan: &FleetComponentProvisioningPlan,
        authority: ComponentProvisioningScaleOutAuthority<'_>,
    ) -> Result<(), InternalError> {
        validate_scale_out_compiled_configuration(configuration, registry, plan, authority)
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

    /// Hash one scale-out plan after validating its durable placement authority.
    pub fn hash_scale_out_compiled(
        configuration: &ComponentDeploymentConfiguration,
        registry: &FleetRegistry,
        plan: &FleetComponentProvisioningPlan,
        authority: ComponentProvisioningScaleOutAuthority<'_>,
    ) -> Result<[u8; 32], InternalError> {
        validate_scale_out_compiled_configuration(configuration, registry, plan, authority)
            .map_err(OpsError::from)
            .map_err(InternalError::from)?;
        Self::hash_for_exact_retry(plan)
    }

    /// Hash one bounded canonical plan solely for comparison with prior validated authority.
    ///
    /// This function does not validate or authorize a new plan. Callers may use it only to
    /// identify an exact retry after the original validation context has advanced.
    pub fn hash_for_exact_retry(
        plan: &FleetComponentProvisioningPlan,
    ) -> Result<[u8; 32], InternalError> {
        let bytes = canonical::plan_bytes(plan)
            .map_err(OpsError::from)
            .map_err(InternalError::from)?;
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
        canonical::root_batch_bytes(batch)
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
    validate_compiled_configuration_with_scale_out(configuration, registry, plan, None)
}

fn validate_scale_out_compiled_configuration(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
    authority: ComponentProvisioningScaleOutAuthority<'_>,
) -> Result<(), ComponentProvisioningPlanOpsError> {
    validate_compiled_configuration_with_scale_out(configuration, registry, plan, Some(authority))
}

fn validate_compiled_configuration_with_scale_out(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
    scale_out: Option<ComponentProvisioningScaleOutAuthority<'_>>,
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

    let mut ledger = PlanValidationLedger::new();
    if let Some(authority) = scale_out {
        scale_out::seed_authority(&mut ledger, &configuration.deployment_topology, authority)?;
    }
    let mut previous_root = None;
    for batch in &plan.batches {
        let root = batch.root.fleet_subnet_root;
        if previous_root.is_some_and(|previous| previous >= root) {
            return Err(ComponentProvisioningPlanOpsError::NonCanonicalBatchOrder);
        }
        previous_root = Some(root);
        if scale_out.is_some() && batch.placements.is_empty() {
            return Err(ComponentProvisioningPlanOpsError::EmptyScaleOutBatch);
        }
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
    validate_confirmation_roots(registry, plan)?;
    match (&plan.operation, scale_out) {
        (FleetComponentProvisioningOperation::FreshInstall, None) => validate_fresh_install(
            &configuration.deployment_topology,
            &configuration.fleet_service_topology,
            &ledger,
        ),
        (FleetComponentProvisioningOperation::ScaleOut { .. }, Some(authority)) => {
            scale_out::validate(
                registry,
                plan,
                &configuration.deployment_topology,
                &configuration.fleet_service_topology,
                &ledger,
                authority,
            )
        }
        (FleetComponentProvisioningOperation::ScaleOut { .. }, None)
        | (FleetComponentProvisioningOperation::FreshInstall, Some(_)) => {
            Err(ComponentProvisioningPlanOpsError::ScaleOutStateUnavailable)
        }
    }
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
        let batch_roots = plan
            .batches
            .iter()
            .map(|batch| batch.root.fleet_subnet_root)
            .collect::<Vec<_>>();
        if batch_roots != expected {
            return Err(ComponentProvisioningPlanOpsError::FreshInstallBatchRootSetMismatch);
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
    ledger.record(&placement.group_placement, deployment, root)
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
    root_component_counts: BTreeMap<Principal, u32>,
    root_spec_counts: BTreeMap<Principal, BTreeMap<ComponentSpecId, u32>>,
}

impl PlanValidationLedger {
    const fn new() -> Self {
        Self {
            placements: BTreeSet::new(),
            placement_roots: BTreeMap::new(),
            service_roots: BTreeMap::new(),
            root_component_counts: BTreeMap::new(),
            root_spec_counts: BTreeMap::new(),
        }
    }

    fn record(
        &mut self,
        placement: &crate::ids::ComponentGroupPlacementId,
        deployment: &ComponentGroupDeploymentSpec,
        root: Principal,
    ) -> Result<(), ComponentProvisioningPlanOpsError> {
        self.placement_roots.insert(placement.clone(), root);
        let component_count = self.root_component_counts.entry(root).or_default();
        *component_count = component_count
            .checked_add(
                u32::try_from(deployment.members.len())
                    .map_err(|_| ComponentProvisioningPlanOpsError::CountOverflow)?,
            )
            .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
        let spec_counts = self.root_spec_counts.entry(root).or_default();
        for member in &deployment.members {
            let spec_count = spec_counts
                .entry(member.component_spec.clone())
                .or_default();
            *spec_count = spec_count
                .checked_add(1)
                .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
            if let ComponentDeploymentPurpose::FleetServiceMember { service, .. } = &member.purpose
            {
                let service_count = self
                    .service_roots
                    .entry(service.clone())
                    .or_default()
                    .entry(root)
                    .or_default();
                *service_count = service_count
                    .checked_add(1)
                    .ok_or(ComponentProvisioningPlanOpsError::CountOverflow)?;
            }
        }
        Ok(())
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
    canonical::plan_bytes(plan)
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
