//! Module: fleet_ensure::view::startup_funding
//!
//! Responsibility: describe configured startup grants separately from native balance evidence.
//! Does not own: runtime grant history, burn forecasts, plan authority or funding effects.
//! Boundary: live Coordinator usage does not turn fresh-child demand into a funding quotation.

use canic_core::ids::{
    CanisterRole, ComponentBinding, ComponentSpecId, CyclesFundingBudget, FleetSubnetRootReleaseSet,
};

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
    ArithmeticOverflow,
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
