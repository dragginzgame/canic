//! Module: fleet_ensure::view::startup_funding
//!
//! Responsibility: describe configured startup grants separately from native balance evidence.
//! Does not own: runtime grant history, burn forecasts, plan authority or funding effects.
//! Boundary: live Coordinator usage does not turn fresh-child demand into a funding quotation.

use candid::Principal;
use canic_core::ids::{
    CanisterRole, ComponentBinding, ComponentSpecAdmission, ComponentSpecId,
    ComponentTopologyDigest, CyclesFundingBudget, FleetRegistryAuthority,
    FleetSubnetRootFundingAuthority, FleetSubnetRootLimits, FleetSubnetRootReleaseSet, SubnetId,
};
use std::collections::BTreeMap;

/// Retained observation status and recomputed diagnostic demand; neither approves funding.
#[derive(Clone, Debug, serde::Serialize)]
pub struct FundingObservationReport {
    pub review: crate::fleet_ensure::model::funding_observation::FundingObservationReviewRecord,
    pub recovery_demand: Result<
        StartupRecoveryDemand,
        crate::fleet_ensure::model::funding_observation::FundingObservationError,
    >,
}

/// Validated current Coordinator registry evidence, local to one preview invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::fleet_ensure) struct StartupFundingRegistry {
    pub authority: FleetRegistryAuthority,
    pub revision: u64,
    pub content_hash: [u8; 32],
    pub roots: BTreeMap<Principal, StartupFundingPlacement>,
}

/// Current registry placement and policy used to qualify an allocation's funding edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::fleet_ensure) struct StartupFundingPlacement {
    pub active: bool,
    pub placement_subnet: SubnetId,
    pub release_set: FleetSubnetRootReleaseSet,
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub component_topology_digest: ComponentTopologyDigest,
    pub limits: FleetSubnetRootLimits,
    pub funding: FleetSubnetRootFundingAuthority,
}

/// Root's independently observed number of current Workload canisters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::fleet_ensure) struct StartupRootInventory {
    pub workloads: u32,
}

/// Complete current membership qualified independently of funding-ledger availability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StartupInventoryCoverage {
    pub components: usize,
    pub descendants: usize,
}

/// Native balance evidence used for the startup scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupNativeBalance {
    ConfiguredCreation(u128),
    Observed(u128),
}

impl StartupNativeBalance {
    /// Return the balance while retaining its evidence kind in the projection.
    #[must_use]
    pub const fn cycles(self) -> u128 {
        match self {
            Self::ConfiguredCreation(cycles) | Self::Observed(cycles) => cycles,
        }
    }
}

/// Configured startup scenario for one generated Fleet, excluding execution burn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupFundingForecast {
    pub coordinator_usage: StartupCoordinatorUsage,
    pub coordinator_balance: StartupNativeBalance,
    pub coordinator_reserve_cycles: u128,
    pub coordinator_spendable_cycles: u128,
    pub roots: Vec<StartupRootFunding>,
}

/// Protected Coordinator accounting, independent of the fresh-child startup scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StartupCoordinatorUsage {
    Unavailable(StartupUsageUnavailable),
    Observed(StartupCoordinatorAccounting),
}

/// Why generation cannot assert current automatic-funding usage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupUsageUnavailable {
    NotWorkload,
    ParentRelayRequired,
    ParentNotObserved,
    NotObserved,
    SelectedBuildNotInstalled,
    ObservationFailed,
    AuthorityMismatch,
    PolicyTransition,
    InvalidAccounting,
    InventoryIncomplete,
}

/// One current protected status response; window allowance is not spendable native funding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupCoordinatorAccounting {
    pub policy_generation: u64,
    pub funding_enabled: bool,
    pub window_start_secs: u64,
    pub spent_cycles: u128,
    pub reserved_cycles: u128,
    pub window_remaining_cycles: u128,
    pub automatic_grants: u32,
    pub maximum_automatic_grants: u32,
    pub automatic_cycles: u128,
    pub maximum_automatic_cycles: u128,
    pub pending_roots: usize,
}

/// Root-local demand; descendant transfers are already included through their parents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupRootFunding {
    pub recovery_demand: Result<StartupRecoveryDemand, StartupDemandUnavailable>,
    pub relay_quote: Result<StartupRelayQuote, StartupDemandUnavailable>,
    pub inventory: Result<StartupInventoryCoverage, StartupUsageUnavailable>,
    pub child_usage: Vec<StartupChildFundingUsage>,
    pub balance: StartupNativeBalance,
    pub child_grants_cycles: u128,
    pub components: Vec<StartupComponentFunding>,
    pub funding_budget: CyclesFundingBudget,
    pub exceeds_window_budget: bool,
    pub minimum_native_cycles: u128,
    pub request_threshold_cycles: u128,
    pub root: String,
    pub shortfall_cycles: u128,
}

/// One seeded Root-controlled asset's actual parent-local grant accounting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupChildFundingUsage {
    pub allowance: Result<StartupChildFundingAllowance, StartupUsageUnavailable>,
    pub observed_balance_cycles: Option<u128>,
    pub local_demand: Result<StartupChildLocalDemand, StartupDemandUnavailable>,
    pub binding: Option<StartupChildFundingBinding>,
    pub name: String,
    pub child: String,
    pub usage: Result<StartupChildAccounting, StartupUsageUnavailable>,
}

/// A child's own threshold deficit; excludes descendants, execution and parent liquidity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupChildLocalDemand {
    pub observed_balance_cycles: u128,
    pub threshold_cycles: Option<u128>,
    pub shortfall_cycles: u128,
    pub shortfall_beyond_lifetime_allowance_cycles: u128,
    pub next_request_policy_cycles: u128,
}

/// Missing or unresolved evidence must never become a zero funding requirement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupDemandUnavailable {
    Usage(StartupUsageUnavailable),
    BalanceNotObserved,
    PendingGrant,
    InvalidObservationBounds,
    ArithmeticOverflow,
}

/// Live recursive demand, excluding execution burn and never itself authorizing a transfer.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct StartupRecoveryDemand {
    /// Aggregate Root transfers exceed one configured window even before existing usage.
    pub exceeds_root_window_budget: bool,
    /// Only Root-to-Component edges contribute here; nested transfers are not added twice.
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub root_grants_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub minimum_native_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub shortfall_cycles: u128,
    /// Required funding that automatic per-child policy cannot supply.
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub uncovered_cycles: u128,
    pub children: Vec<StartupRecursiveChildDemand>,
}

/// One exact edge's grant requirement after counting its children's transfers.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct StartupRecursiveChildDemand {
    /// Aggregate Component transfers exceed one configured window even before existing usage.
    pub exceeds_component_window_budget: bool,
    pub child: Principal,
    pub parent: Principal,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub outgoing_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub parent_grants_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub uncovered_cycles: u128,
    pub cooldown_remaining_secs: u64,
}

/// Complete allocation identity and its funding edge, independent of physical pool custody.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupChildFundingBinding {
    pub release_set: FleetSubnetRootReleaseSet,
    pub component: ComponentBinding,
    pub canister_id: candid::Principal,
    pub parent: candid::Principal,
    pub parent_role: Option<CanisterRole>,
    pub role: CanisterRole,
}

/// Runtime policy headroom after observed charges; this is neither demand nor spending authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupChildFundingAllowance {
    pub maximum_per_child_cycles: u128,
    pub remaining_after_charges_cycles: u128,
    pub cooldown_remaining_secs: u64,
    /// None means unresolved accounting; zero means this policy currently admits no amount.
    pub next_request_policy_cap_cycles: Option<u128>,
}

/// Charged amounts may include pending grants and must not be subtracted twice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupChildAccounting {
    pub observed_at_ns: u64,
    pub accounted_cycles: u128,
    pub last_accounted_at_secs: u64,
    pub pending_operations: u32,
    pub reserved_cycles: Option<u128>,
}

/// One concrete Component placement and its recursively counted initial roles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupComponentFunding {
    pub component_spec: ComponentSpecId,
    pub deployment: String,
    pub descendant_grants_cycles: u128,
    pub exceeds_window_budget: bool,
    pub funding_budget: CyclesFundingBudget,
    pub ordinal: u32,
    pub roles: Vec<StartupRoleFunding>,
    pub root_grant_cycles: u128,
}

/// One role's per-instance demand after its outgoing child grants, with lifetime clamping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupRoleFunding {
    pub cooldown_secs: u64,
    pub effective_grant_cycles: u128,
    pub grants_per_instance: u128,
    pub initial_balance_cycles: u128,
    pub instances: u32,
    pub maximum_per_child_cycles: u128,
    pub outgoing_grants_per_instance_cycles: u128,
    pub parent_grants_per_instance_cycles: u128,
    pub role: CanisterRole,
    pub threshold_cycles: Option<u128>,
    pub unfunded_per_instance_cycles: u128,
}

/// Proposed single-pass relay allowance, owned by preview policy and never executable authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupRelayQuote {
    /// Exact funding edges; each requests ChildFunding(child) from its immediate parent via Root.
    pub requests: Vec<StartupChildFundingBinding>,
    pub per_attempt_burn_allowance_cycles: u128,
    pub proposed_burn_allowance_cycles: u128,
    pub minimum_recovery_cycles: u128,
    pub required_native_cycles: u128,
    pub observed_native_cycles: u128,
    pub native_shortfall_cycles: u128,
}

/// Shared policy bounds for preview quotation and retained observation review validation.
pub(in crate::fleet_ensure) struct StartupObservationBounds {
    pub per_attempt_cycles: u128,
    pub recovery_floor_cycles: u128,
}
