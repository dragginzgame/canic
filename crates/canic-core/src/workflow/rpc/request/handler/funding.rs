use crate::{
    InternalError,
    cdk::types::Principal,
    config::schema::{CanisterTopup, ConfigModel},
    ids::CanisterRole,
    ops::config::ConfigOps,
};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static MINT_FUNDING_LEDGER: RefCell<HashMap<Principal, MintFundingLedger>> =
        RefCell::new(HashMap::new());
}

///
/// MintFundingPolicy
/// Effective parent funding limits for a single child role.
///

#[derive(Clone, Copy, Debug)]
pub(super) struct MintFundingPolicy {
    pub max_per_request: u128,
    pub max_per_child: u128,
    pub cooldown_secs: u64,
}

impl MintFundingPolicy {
    // Evaluate a child request against role policy and child runtime ledger.
    pub(super) fn evaluate(
        self,
        child: Principal,
        requested_cycles: u128,
        now_secs: u64,
    ) -> Result<(), MintFundingPolicyViolation> {
        if requested_cycles > self.max_per_request {
            return Err(MintFundingPolicyViolation::MaxPerRequest {
                requested: requested_cycles,
                max_per_request: self.max_per_request,
            });
        }

        let ledger = child_ledger(child);
        if self.cooldown_secs > 0 {
            let earliest_next = ledger.last_granted_at.saturating_add(self.cooldown_secs);
            if now_secs < earliest_next {
                return Err(MintFundingPolicyViolation::CooldownActive {
                    retry_after_secs: earliest_next.saturating_sub(now_secs),
                });
            }
        }

        let remaining_budget = self.max_per_child.saturating_sub(ledger.granted_total);
        if requested_cycles > remaining_budget {
            return Err(MintFundingPolicyViolation::MaxPerChild {
                requested: requested_cycles,
                max_per_child: self.max_per_child,
                remaining_budget,
            });
        }

        Ok(())
    }
}

///
/// MintFundingPolicyViolation
/// Pure policy violations for mint-cycle authorization.
///

#[derive(Clone, Copy, Debug)]
pub(super) enum MintFundingPolicyViolation {
    MaxPerRequest {
        requested: u128,
        max_per_request: u128,
    },
    MaxPerChild {
        requested: u128,
        max_per_child: u128,
        remaining_budget: u128,
    },
    CooldownActive {
        retry_after_secs: u64,
    },
}

#[derive(Clone, Copy, Debug, Default)]
struct MintFundingLedger {
    granted_total: u128,
    last_granted_at: u64,
}

// Resolve the effective policy for a child role from current config.
pub(super) fn policy_for_child_role(
    child_role: &CanisterRole,
) -> Result<MintFundingPolicy, InternalError> {
    let config = ConfigOps::get()?;
    Ok(resolve_policy_from_config(&config, child_role).unwrap_or(default_policy()))
}

// Record a successful grant for cooldown and child-budget accounting.
pub(super) fn record_child_grant(child: Principal, granted_cycles: u128, now_secs: u64) {
    MINT_FUNDING_LEDGER.with_borrow_mut(|ledger| {
        let entry = ledger.entry(child).or_default();
        entry.granted_total = entry.granted_total.saturating_add(granted_cycles);
        entry.last_granted_at = now_secs;
    });
}

// Return the current ledger state for a child.
fn child_ledger(child: Principal) -> MintFundingLedger {
    MINT_FUNDING_LEDGER.with_borrow(|ledger| ledger.get(&child).copied().unwrap_or_default())
}

// Resolve role policy by scanning configured subnets for the role definition.
fn resolve_policy_from_config(
    config: &ConfigModel,
    child_role: &CanisterRole,
) -> Option<MintFundingPolicy> {
    config
        .subnets
        .values()
        .find_map(|subnet| subnet.canisters.get(child_role))
        .and_then(|cfg| cfg.topup.as_ref())
        .map(policy_from_topup)
}

// Convert topup config into an effective mint-cycle policy.
const fn policy_from_topup(topup: &CanisterTopup) -> MintFundingPolicy {
    MintFundingPolicy {
        max_per_request: topup.max_per_request.to_u128(),
        max_per_child: topup.max_per_child.to_u128(),
        cooldown_secs: topup.cooldown_secs,
    }
}

// Fail-open defaults preserve existing behavior when role policy is absent.
const fn default_policy() -> MintFundingPolicy {
    MintFundingPolicy {
        max_per_request: u128::MAX,
        max_per_child: u128::MAX,
        cooldown_secs: 0,
    }
}

#[cfg(test)]
pub(super) fn reset_for_tests() {
    MINT_FUNDING_LEDGER.with_borrow_mut(HashMap::clear);
}
