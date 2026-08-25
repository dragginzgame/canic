//! Module: install_root::fleet_install_recovery
//!
//! Responsibility: compile one read-only recovery plan from exact retained fresh-install state.
//! Does not own: journal mutation, live verification, release building, or installation effects.
//! Boundary: only validated immutable session, plan, artifact, and role-journal authority may
//! reduce the maximum debit that can still be issued by the recovering host.

use super::{
    coordinator_install_journal::{
        FleetCoordinatorInstallPhase, PlanFleetCoordinatorInstallRequest,
        inspect_fleet_coordinator_install,
    },
    fleet_install_session,
    fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest,
        has_fleet_subnet_root_install_journal, inspect_fleet_subnet_root_install,
    },
};
use crate::{
    fleet_install_plan::{
        CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES, FreshFleetDecisionAuthorityError,
        FreshFleetDecisionAuthorityRequest, FreshFleetDecisionAuthorityV1,
        FreshFleetDeploymentPlanError, FreshFleetDeploymentPlanRequest, FreshFleetDeploymentPlanV1,
        FreshFleetPreflightError, FreshFleetPreflightRequest, FreshFleetPreflightV1,
        PersistedFleetInstallPlan, PlannedCanisterCreationFunding,
        compile_fresh_fleet_deployment_plan_with_operator_debit, compile_retained_fleet_preflight,
        load_fresh_fleet_recovery_decision_authority, load_retained_fleet_install_plan,
    },
    release_set::load_persisted_canic_infrastructure_artifact_manifest,
};
use canic_core::{
    bootstrap::compiled::ConfigModel,
    cdk::utils::hash::hex_bytes,
    ids::{AppId, CanonicalNetworkId, FleetName, ReleaseBuildId},
};
use serde::Serialize;
use std::path::Path;
use thiserror::Error as ThisError;

const RECOVERY_PLAN_SCHEMA_VERSION: u16 = 1;

/// Whether a retained session is still pre-effect or has fenced operator-paid work.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshFleetInstallRecoveryClassificationV1 {
    PreparedResume,
    PaidEffectRecovery,
}

impl FreshFleetInstallRecoveryClassificationV1 {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreparedResume => "prepared_resume",
            Self::PaidEffectRecovery => "paid_effect_recovery",
        }
    }
}

/// Read-only exact replay authority and remaining operator exposure for one install session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshFleetInstallRecoveryPlanV1 {
    pub schema_version: u16,
    pub classification: FreshFleetInstallRecoveryClassificationV1,
    pub fleet_install_operation_id: String,
    pub release_build_id: ReleaseBuildId,
    pub decision_release_build_id: Option<ReleaseBuildId>,
    pub retained_builder_version: String,
    pub fresh_fleet_plan_digest: String,
    pub effects_started: bool,
    pub original_maximum_operator_debit: PlannedCanisterCreationFunding,
    pub remaining_operator_debit: PlannedCanisterCreationFunding,
    pub fenced_operator_creations: u32,
    pub total_operator_creations: u32,
    pub uncertain_creation_outcomes: Vec<String>,
    pub next_replay_phase: String,
}

impl FreshFleetInstallRecoveryPlanV1 {
    /// True when an exact durable creation intent must be observed before any later paid effect.
    #[must_use]
    pub const fn has_uncertain_creation_outcome(&self) -> bool {
        !self.uncertain_creation_outcomes.is_empty()
    }

    /// Require the newly compiled decision to remain the exact retained session authority.
    pub fn require_matching_decision(
        &self,
        plan: &FreshFleetDeploymentPlanV1,
    ) -> Result<(), FreshFleetInstallRecoveryError> {
        if self.fresh_fleet_plan_digest == plan.plan_digest
            && self.original_maximum_operator_debit == plan.maximum_operator_debit
            && plan.preflight.release_build_id == self.decision_release_build_id
        {
            return Ok(());
        }
        Err(FreshFleetInstallRecoveryError::DecisionMismatch)
    }

    /// Load the original decision authority without substituting the current host version.
    pub fn load_decision_authority(
        &self,
        request: FreshFleetDecisionAuthorityRequest<'_>,
    ) -> Result<FreshFleetDecisionAuthorityV1, FreshFleetInstallRecoveryError> {
        if request.release_build_id != self.decision_release_build_id {
            return Err(FreshFleetInstallRecoveryError::DecisionMismatch);
        }
        load_fresh_fleet_recovery_decision_authority(request, &self.retained_builder_version)
            .map_err(Into::into)
    }

    /// Compile only this exact retained decision against its journal-derived remaining debit.
    pub fn compile_decision(
        &self,
        request: FreshFleetDeploymentPlanRequest,
    ) -> Result<FreshFleetDeploymentPlanV1, FreshFleetInstallRecoveryError> {
        let plan = compile_fresh_fleet_deployment_plan_with_operator_debit(
            request,
            &self.remaining_operator_debit,
        )?;
        self.require_matching_decision(&plan)?;
        Ok(plan)
    }

    /// Compile the retained decision under its exact admitted historical pool-policy boundary.
    pub fn compile_preflight(
        &self,
        request: FreshFleetPreflightRequest<'_>,
    ) -> Result<FreshFleetPreflightV1, FreshFleetInstallRecoveryError> {
        require_supported_recovery_builder(
            &self.retained_builder_version,
            env!("CARGO_PKG_VERSION"),
        )?;
        if uses_historical_pool_policy(&self.retained_builder_version, env!("CARGO_PKG_VERSION")) {
            return compile_retained_fleet_preflight(request).map_err(Into::into);
        }
        crate::fleet_install_plan::compile_fresh_fleet_preflight(request).map_err(Into::into)
    }

    pub(super) fn load_install_plan(
        &self,
        root: &Path,
        config: &ConfigModel,
        fleet: &canic_core::ids::FleetBinding,
    ) -> Result<PersistedFleetInstallPlan, FreshFleetInstallRecoveryError> {
        require_supported_recovery_builder(
            &self.retained_builder_version,
            env!("CARGO_PKG_VERSION"),
        )?;
        load_recovery_install_plan(
            root,
            config,
            fleet,
            self.release_build_id,
            &self.retained_builder_version,
        )
    }
}

/// Read-only identity needed to discover one retained fresh-install session.
pub struct InspectFreshFleetInstallRecoveryRequest<'a> {
    pub root: &'a Path,
    pub canonical_network_id: CanonicalNetworkId,
    pub fleet_name: &'a FleetName,
    pub app: &'a AppId,
    pub config: &'a ConfigModel,
}

/// Invalid or inconsistent local recovery evidence.
#[derive(Debug, ThisError)]
pub enum FreshFleetInstallRecoveryError {
    #[error("invalid retained Fleet-install recovery evidence: {0}")]
    InvalidEvidence(String),

    #[error("retained Fleet-install recovery funding is not cycles-only")]
    NonCyclesFunding,

    #[error("retained Fleet-install recovery funding arithmetic overflowed")]
    FundingOverflow,

    #[error("retained Root recovery exists before exact Coordinator verification evidence")]
    RootBeforeCoordinator,

    #[error("retained Fleet-install recovery differs from the recompiled decision authority")]
    DecisionMismatch,

    #[error(transparent)]
    DecisionAuthority(#[from] FreshFleetDecisionAuthorityError),

    #[error(transparent)]
    DecisionPlan(#[from] FreshFleetDeploymentPlanError),

    #[error(transparent)]
    Preflight(#[from] FreshFleetPreflightError),
}

/// Inspect one existing session without creating a lock, journal, report, or other file.
pub fn inspect_fresh_fleet_install_recovery(
    request: InspectFreshFleetInstallRecoveryRequest<'_>,
) -> Result<Option<FreshFleetInstallRecoveryPlanV1>, FreshFleetInstallRecoveryError> {
    let recovered = fleet_install_session::inspect_fleet_install_session_authority(
        request.root,
        request.canonical_network_id,
        request.fleet_name,
        request.app,
    )
    .map_err(invalid)?;
    recovered
        .map(|recovered| compile_recovery_plan(request.root, request.config, &recovered))
        .transpose()
}

pub(super) fn compile_recovery_plan(
    root: &Path,
    config: &ConfigModel,
    recovered: &fleet_install_session::RecoveredFleetInstallAuthority,
) -> Result<FreshFleetInstallRecoveryPlanV1, FreshFleetInstallRecoveryError> {
    let session = &recovered.session;
    let retained_builder_version = &recovered.finalized_release_build.record.builder_version;
    let plan = load_recovery_install_plan(
        root,
        config,
        &session.fleet,
        session.release_build_id,
        retained_builder_version,
    )?;
    if plan.plan.fresh_fleet_plan_digest != session.fresh_fleet_plan_digest {
        return Err(invalid(
            "persisted Fleet plan differs from the retained fresh-Fleet decision digest",
        ));
    }
    let infrastructure_manifest =
        load_persisted_canic_infrastructure_artifact_manifest(root, session.release_build_id)
            .map_err(invalid)?;
    let deployment = config
        .compile_component_deployment_configuration()
        .map_err(invalid)?;
    let coordinator = inspect_fleet_coordinator_install(PlanFleetCoordinatorInstallRequest {
        fleet_install_plan: &plan,
        infrastructure_manifest: &infrastructure_manifest,
        component_deployment_configuration: deployment,
    })
    .map_err(invalid)?;

    let mut funding = RecoveryFunding::new(&plan)?;
    let mut next_replay_phase = None;
    let mut uncertain_creation_outcomes = Vec::new();
    let mut verified_coordinator = None;

    if let Some(current) = coordinator {
        if current.journal.phase != FleetCoordinatorInstallPhase::Planned {
            funding.fence(&current.journal.creation_funding)?;
        }
        if current.journal.phase == FleetCoordinatorInstallPhase::CreationInFlight {
            uncertain_creation_outcomes.push("fleet_coordinator".to_string());
        }
        if current.journal.phase == FleetCoordinatorInstallPhase::Verified {
            verified_coordinator = current.journal.coordinator;
        } else {
            next_replay_phase = Some(coordinator_phase_label(current.journal.phase).to_string());
        }
    } else {
        next_replay_phase = Some("coordinator:creation".to_string());
    }

    if let Some(coordinator) = verified_coordinator {
        inspect_roots(
            RootInspectionAuthority {
                plan: &plan,
                infrastructure_manifest: &infrastructure_manifest,
                config,
                install_operation_id: session.operation_id,
                coordinator,
            },
            &mut funding,
            &mut next_replay_phase,
            &mut uncertain_creation_outcomes,
        )?;
    } else {
        for root_plan in &plan.plan.fleet_subnet_roots {
            if has_fleet_subnet_root_install_journal(&plan.path, root_plan.placement_subnet)
                .map_err(invalid)?
            {
                return Err(FreshFleetInstallRecoveryError::RootBeforeCoordinator);
            }
        }
    }

    let effects_started = funding.fenced_operator_creations > 0;
    Ok(FreshFleetInstallRecoveryPlanV1 {
        schema_version: RECOVERY_PLAN_SCHEMA_VERSION,
        classification: if effects_started {
            FreshFleetInstallRecoveryClassificationV1::PaidEffectRecovery
        } else {
            FreshFleetInstallRecoveryClassificationV1::PreparedResume
        },
        fleet_install_operation_id: hex_bytes(session.operation_id),
        release_build_id: session.release_build_id,
        decision_release_build_id: session.decision_release_build_id,
        retained_builder_version: retained_builder_version.clone(),
        fresh_fleet_plan_digest: session.fresh_fleet_plan_digest.clone(),
        effects_started,
        original_maximum_operator_debit: PlannedCanisterCreationFunding::Cycles {
            cycles: funding.original_cycles,
        },
        remaining_operator_debit: PlannedCanisterCreationFunding::Cycles {
            cycles: funding.remaining_cycles,
        },
        fenced_operator_creations: funding.fenced_operator_creations,
        total_operator_creations: funding.total_operator_creations,
        uncertain_creation_outcomes,
        next_replay_phase: next_replay_phase
            .unwrap_or_else(|| "fleet_component_provisioning".to_string()),
    })
}

const RETAINED_INSTALL_RECOVERY_PREDECESSOR: &str = "0.109.1";
const RETAINED_INSTALL_RECOVERY_SUCCESSORS: &[&str] = &["0.109.2", "0.109.3", "0.109.4", "0.109.5"];

fn load_recovery_install_plan(
    root: &Path,
    config: &ConfigModel,
    fleet: &canic_core::ids::FleetBinding,
    release_build_id: ReleaseBuildId,
    retained_builder_version: &str,
) -> Result<PersistedFleetInstallPlan, FreshFleetInstallRecoveryError> {
    let current = env!("CARGO_PKG_VERSION");
    require_supported_recovery_builder(retained_builder_version, current)?;
    if uses_historical_pool_policy(retained_builder_version, current) {
        return load_retained_fleet_install_plan(root, config, fleet, release_build_id)
            .map_err(invalid);
    }
    crate::fleet_install_plan::load_persisted_fleet_install_plan(
        root,
        config,
        fleet,
        release_build_id,
    )
    .map_err(invalid)
}

fn uses_historical_pool_policy(recorded: &str, current: &str) -> bool {
    recorded == RETAINED_INSTALL_RECOVERY_PREDECESSOR
        && recorded != current
        && RETAINED_INSTALL_RECOVERY_SUCCESSORS.contains(&current)
}

/// Enforce the one narrow host-only cross-patch rescue contract for an exact retained build.
pub fn require_supported_recovery_builder(
    recorded: &str,
    current: &str,
) -> Result<(), FreshFleetInstallRecoveryError> {
    if recorded == current {
        return Ok(());
    }
    if recorded == RETAINED_INSTALL_RECOVERY_PREDECESSOR
        && RETAINED_INSTALL_RECOVERY_SUCCESSORS.contains(&current)
    {
        return Ok(());
    }
    let successors = RETAINED_INSTALL_RECOVERY_SUCCESSORS.join(" or ");
    Err(invalid(format!(
        "interrupted Fleet install release build belongs to Canic {recorded}, not current Canic {current}; only an exact {RETAINED_INSTALL_RECOVERY_PREDECESSOR} release-build session may be resumed by {successors}"
    )))
}

struct RootInspectionAuthority<'a> {
    plan: &'a PersistedFleetInstallPlan,
    infrastructure_manifest: &'a crate::release_set::PersistedCanicInfrastructureArtifactManifest,
    config: &'a ConfigModel,
    install_operation_id: [u8; 32],
    coordinator: candid::Principal,
}

fn inspect_roots(
    authority: RootInspectionAuthority<'_>,
    funding: &mut RecoveryFunding,
    next_replay_phase: &mut Option<String>,
    uncertain_creation_outcomes: &mut Vec<String>,
) -> Result<(), FreshFleetInstallRecoveryError> {
    let topology = authority
        .config
        .compile_component_topology()
        .map_err(invalid)?;
    for root_plan in &authority.plan.plan.fleet_subnet_roots {
        let current = inspect_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan: authority.plan,
            infrastructure_manifest: authority.infrastructure_manifest,
            coordinator: authority.coordinator,
            install_operation_id: authority.install_operation_id,
            component_topology: topology.clone(),
            root_plan,
        })
        .map_err(invalid)?;
        let Some(current) = current else {
            if next_replay_phase.is_none() {
                *next_replay_phase = Some(format!(
                    "fleet_subnet_root:{}:root_creation",
                    root_plan.placement_subnet
                ));
            }
            continue;
        };
        let phase = current.journal.phase;
        if phase != FleetSubnetRootInstallPhase::Planned {
            funding.fence(&root_plan.root_creation_funding)?;
        }
        if phase_at_or_after_wasm_store_creation_intent(phase) {
            funding.fence(&root_plan.wasm_store_creation_funding)?;
        }
        if phase == FleetSubnetRootInstallPhase::RootCreationInFlight {
            uncertain_creation_outcomes
                .push(format!("fleet_subnet_root:{}", root_plan.placement_subnet));
        }
        if phase == FleetSubnetRootInstallPhase::WasmStoreCreationInFlight {
            uncertain_creation_outcomes.push(format!("wasm_store:{}", root_plan.placement_subnet));
        }
        if next_replay_phase.is_none()
            && phase != FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified
        {
            *next_replay_phase = Some(format!(
                "fleet_subnet_root:{}:{}",
                root_plan.placement_subnet,
                root_phase_label(phase)
            ));
        }
    }
    Ok(())
}

struct RecoveryFunding {
    original_cycles: u128,
    remaining_cycles: u128,
    fenced_operator_creations: u32,
    total_operator_creations: u32,
}

impl RecoveryFunding {
    fn new(plan: &PersistedFleetInstallPlan) -> Result<Self, FreshFleetInstallRecoveryError> {
        let total_operator_creations = u32::try_from(
            1_usize
                .checked_add(
                    plan.plan
                        .fleet_subnet_roots
                        .len()
                        .checked_mul(2)
                        .ok_or(FreshFleetInstallRecoveryError::FundingOverflow)?,
                )
                .ok_or(FreshFleetInstallRecoveryError::FundingOverflow)?,
        )
        .map_err(|_| FreshFleetInstallRecoveryError::FundingOverflow)?;
        let mut original_cycles = funding_cycles(&plan.plan.coordinator.creation_funding)?;
        for root in &plan.plan.fleet_subnet_roots {
            original_cycles = checked_add(
                original_cycles,
                funding_cycles(&root.root_creation_funding)?,
            )?;
            original_cycles = checked_add(
                original_cycles,
                funding_cycles(&root.wasm_store_creation_funding)?,
            )?;
        }
        original_cycles = checked_add(
            original_cycles,
            CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES
                .checked_mul(u128::from(total_operator_creations))
                .ok_or(FreshFleetInstallRecoveryError::FundingOverflow)?,
        )?;
        Ok(Self {
            original_cycles,
            remaining_cycles: original_cycles,
            fenced_operator_creations: 0,
            total_operator_creations,
        })
    }

    fn fence(
        &mut self,
        funding: &PlannedCanisterCreationFunding,
    ) -> Result<(), FreshFleetInstallRecoveryError> {
        let debit = checked_add(
            funding_cycles(funding)?,
            CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES,
        )?;
        self.remaining_cycles = self
            .remaining_cycles
            .checked_sub(debit)
            .ok_or(FreshFleetInstallRecoveryError::FundingOverflow)?;
        self.fenced_operator_creations = self
            .fenced_operator_creations
            .checked_add(1)
            .ok_or(FreshFleetInstallRecoveryError::FundingOverflow)?;
        Ok(())
    }
}

const fn funding_cycles(
    funding: &PlannedCanisterCreationFunding,
) -> Result<u128, FreshFleetInstallRecoveryError> {
    match funding {
        PlannedCanisterCreationFunding::Cycles { cycles } => Ok(*cycles),
        PlannedCanisterCreationFunding::Icp { .. } => {
            Err(FreshFleetInstallRecoveryError::NonCyclesFunding)
        }
    }
}

fn checked_add(left: u128, right: u128) -> Result<u128, FreshFleetInstallRecoveryError> {
    left.checked_add(right)
        .ok_or(FreshFleetInstallRecoveryError::FundingOverflow)
}

const fn phase_at_or_after_wasm_store_creation_intent(phase: FleetSubnetRootInstallPhase) -> bool {
    !matches!(
        phase,
        FleetSubnetRootInstallPhase::Planned
            | FleetSubnetRootInstallPhase::RootCreationInFlight
            | FleetSubnetRootInstallPhase::RootCreated
    )
}

const fn coordinator_phase_label(phase: FleetCoordinatorInstallPhase) -> &'static str {
    match phase {
        FleetCoordinatorInstallPhase::Planned => "coordinator:creation",
        FleetCoordinatorInstallPhase::CreationInFlight => "coordinator:creation_observation",
        FleetCoordinatorInstallPhase::Created => "coordinator:install",
        FleetCoordinatorInstallPhase::InstallInFlight => "coordinator:install_observation",
        FleetCoordinatorInstallPhase::Installed => "coordinator:verification",
        FleetCoordinatorInstallPhase::Verified => "fleet_subnet_roots",
    }
}

const fn root_phase_label(phase: FleetSubnetRootInstallPhase) -> &'static str {
    match phase {
        FleetSubnetRootInstallPhase::Planned => "root_creation",
        FleetSubnetRootInstallPhase::RootCreationInFlight => "root_creation_observation",
        FleetSubnetRootInstallPhase::RootCreated => "wasm_store_creation",
        FleetSubnetRootInstallPhase::WasmStoreCreationInFlight => "wasm_store_creation_observation",
        FleetSubnetRootInstallPhase::WasmStoreCreated => "wasm_store_install",
        FleetSubnetRootInstallPhase::WasmStoreInstallInFlight => "wasm_store_install_observation",
        FleetSubnetRootInstallPhase::WasmStoreInstalled => "root_install",
        FleetSubnetRootInstallPhase::RootInstallInFlight => "root_install_observation",
        FleetSubnetRootInstallPhase::RootInstalled => "infrastructure_verification",
        FleetSubnetRootInstallPhase::InfrastructureVerified
        | FleetSubnetRootInstallPhase::StoreStaging => "store_staging",
        FleetSubnetRootInstallPhase::StoreStaged => "store_adoption",
        FleetSubnetRootInstallPhase::StoreAdoptionInFlight => "store_adoption_observation",
        FleetSubnetRootInstallPhase::StoreAdopted => "store_bootstrap",
        FleetSubnetRootInstallPhase::StoreBootstrapInFlight => "store_bootstrap_observation",
        FleetSubnetRootInstallPhase::StoreBootstrapped => "store_bootstrap_verification",
        FleetSubnetRootInstallPhase::StoreVerified => "registry_join",
        FleetSubnetRootInstallPhase::RegistryJoinInFlight => "registry_join_observation",
        FleetSubnetRootInstallPhase::RegistryJoined => "registry_join_verification",
        FleetSubnetRootInstallPhase::RegistryJoinVerified => "registry_sync",
        FleetSubnetRootInstallPhase::RegistrySyncInFlight => "registry_sync_observation",
        FleetSubnetRootInstallPhase::RegistrySynchronized => "registry_sync_verification",
        FleetSubnetRootInstallPhase::RegistrySyncVerified => "registry_mirror_activation",
        FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight => {
            "registry_mirror_activation_observation"
        }
        FleetSubnetRootInstallPhase::RegistryMirrorActivated => {
            "registry_mirror_activation_verification"
        }
        FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified => {
            "component_registry_preparation"
        }
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight => {
            "component_registry_preparation_observation"
        }
        FleetSubnetRootInstallPhase::ComponentRegistryPrepared => {
            "component_registry_preparation_verification"
        }
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => {
            "fleet_component_provisioning"
        }
    }
}

fn invalid(error: impl std::fmt::Display) -> FreshFleetInstallRecoveryError {
    FreshFleetInstallRecoveryError::InvalidEvidence(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fleet_install_plan::{FleetInstallPlan, PlannedFleetCoordinator},
        test_support::{fleet_admission_policy, placement_cost},
    };
    use candid::Principal;
    use canic_core::ids::{AppId, FleetBinding, FleetId, FleetKey, ReleaseBuildNonce, SubnetId};
    use std::path::PathBuf;

    #[test]
    fn exact_creation_intent_removes_amount_and_one_fee_from_remaining_debit() {
        let plan = zero_root_plan();
        let mut funding = RecoveryFunding::new(&plan).expect("compile original exposure");
        assert_eq!(funding.total_operator_creations, 1);
        assert_eq!(funding.original_cycles, 140_000_100_000_000);
        assert_eq!(funding.remaining_cycles, funding.original_cycles);

        funding
            .fence(&plan.plan.coordinator.creation_funding)
            .expect("fence Coordinator creation");

        assert_eq!(funding.remaining_cycles, 0);
        assert_eq!(funding.fenced_operator_creations, 1);
    }

    #[test]
    fn cross_patch_rescue_is_exactly_1091_to_explicit_successors() {
        assert!(require_supported_recovery_builder("0.109.2", "0.109.2").is_ok());
        assert!(require_supported_recovery_builder("0.109.1", "0.109.2").is_ok());
        assert!(require_supported_recovery_builder("0.109.1", "0.109.3").is_ok());
        assert!(require_supported_recovery_builder("0.109.1", "0.109.4").is_ok());
        assert!(require_supported_recovery_builder("0.109.1", "0.109.5").is_ok());
        assert!(require_supported_recovery_builder("0.109.0", "0.109.2").is_err());
        assert!(require_supported_recovery_builder("0.109.1", "0.109.6").is_err());
        assert!(require_supported_recovery_builder("0.109.2", "0.109.3").is_err());
        assert!(require_supported_recovery_builder("0.109.2", "0.109.1").is_err());
        assert!(uses_historical_pool_policy("0.109.1", "0.109.5"));
        assert!(!uses_historical_pool_policy("0.109.5", "0.109.5"));
        assert!(!uses_historical_pool_policy("0.109.2", "0.109.5"));
        assert!(!uses_historical_pool_policy("0.109.1", "0.109.6"));
    }

    fn zero_root_plan() -> PersistedFleetInstallPlan {
        let release_build_id =
            ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([7; 32]));
        let fleet = FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([8; 32]),
            },
            app: AppId::from("demo"),
        };
        let coordinator_subnet = SubnetId::from_principal(Principal::from_slice(&[9; 29]));
        PersistedFleetInstallPlan {
            plan: FleetInstallPlan {
                fleet: fleet.clone(),
                fresh_fleet_plan_digest: "ab".repeat(32),
                release_build_id,
                application_artifact_union_digest: [1; 32],
                admission: fleet_admission_policy(fleet),
                coordinator: PlannedFleetCoordinator {
                    coordinator_subnet,
                    placement_cost: placement_cost(coordinator_subnet),
                    creation_funding: PlannedCanisterCreationFunding::Cycles {
                        cycles: 140_000_000_000_000,
                    },
                    root_funding: None,
                },
                fleet_subnet_roots: Vec::new(),
            },
            digest: [2; 32],
            path: PathBuf::from("plan.json"),
            root_release_sets: Vec::new(),
        }
    }
}
