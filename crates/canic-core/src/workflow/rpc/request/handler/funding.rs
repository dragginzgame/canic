use crate::{cdk::types::Principal, ids::CanisterRole};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static FUNDING_LEDGER: RefCell<HashMap<Principal, FundingLedger>> =
        RefCell::new(HashMap::new());
}

///
/// FundingPolicy
/// Effective parent funding limits for a single child role.
///

#[derive(Clone, Copy, Debug)]
pub(super) struct FundingPolicy {
    pub max_per_request: u128,
    pub max_per_child: u128,
    pub cooldown_secs: u64,
}

impl FundingPolicy {
    // Evaluate a child request against role policy and child runtime ledger.
    pub(super) fn evaluate(
        self,
        child: Principal,
        requested_cycles: u128,
        now_secs: u64,
    ) -> Result<FundingDecision, FundingPolicyViolation> {
        let ledger = child_ledger(child);
        if self.cooldown_secs > 0 {
            let earliest_next = ledger.last_granted_at.saturating_add(self.cooldown_secs);
            if now_secs < earliest_next {
                return Err(FundingPolicyViolation::CooldownActive {
                    retry_after_secs: earliest_next.saturating_sub(now_secs),
                });
            }
        }

        let remaining_budget = self.max_per_child.saturating_sub(ledger.granted_total);
        if remaining_budget == 0 {
            return Err(FundingPolicyViolation::MaxPerChild {
                requested: requested_cycles,
                max_per_child: self.max_per_child,
                remaining_budget,
            });
        }

        let mut approved_cycles = requested_cycles;
        let mut clamped_max_per_request = false;
        let mut clamped_max_per_child = false;

        if approved_cycles > self.max_per_request {
            approved_cycles = self.max_per_request;
            clamped_max_per_request = true;
        }
        if approved_cycles > remaining_budget {
            approved_cycles = remaining_budget;
            clamped_max_per_child = true;
        }

        Ok(FundingDecision {
            approved_cycles,
            clamped_max_per_request,
            clamped_max_per_child,
        })
    }
}

///
/// FundingDecision
/// Effective approved grant amount after applying policy clamping.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FundingDecision {
    pub approved_cycles: u128,
    pub clamped_max_per_request: bool,
    pub clamped_max_per_child: bool,
}

///
/// FundingPolicyViolation
/// Pure policy violations for funding authorization.
///

#[derive(Clone, Copy, Debug)]
pub(super) enum FundingPolicyViolation {
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
struct FundingLedger {
    granted_total: u128,
    last_granted_at: u64,
}

// Resolve the effective policy for a child role from current config.
pub(super) const fn policy_for_child_role(_child_role: &CanisterRole) -> FundingPolicy {
    default_policy()
}

// Record a successful grant for cooldown and child-budget accounting.
pub(super) fn record_child_grant(child: Principal, granted_cycles: u128, now_secs: u64) {
    FUNDING_LEDGER.with_borrow_mut(|ledger| {
        let entry = ledger.entry(child).or_default();
        entry.granted_total = entry.granted_total.saturating_add(granted_cycles);
        entry.last_granted_at = now_secs;
    });
}

// Return the current ledger state for a child.
fn child_ledger(child: Principal) -> FundingLedger {
    FUNDING_LEDGER.with_borrow(|ledger| ledger.get(&child).copied().unwrap_or_default())
}

// Fail-open defaults preserve existing behavior when role policy is absent.
const fn default_policy() -> FundingPolicy {
    FundingPolicy {
        max_per_request: u128::MAX,
        max_per_child: u128::MAX,
        cooldown_secs: 0,
    }
}

#[cfg(test)]
pub(super) fn reset_for_tests() {
    FUNDING_LEDGER.with_borrow_mut(HashMap::clear);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn evaluate_clamps_to_max_per_request() {
        reset_for_tests();
        let child = p(1);
        let policy = FundingPolicy {
            max_per_request: 100,
            max_per_child: 1_000,
            cooldown_secs: 0,
        };

        let decision = policy.evaluate(child, 250, 10).expect("must clamp");
        assert_eq!(decision.approved_cycles, 100);
        assert!(decision.clamped_max_per_request);
        assert!(!decision.clamped_max_per_child);
    }

    #[test]
    fn evaluate_clamps_to_remaining_budget() {
        reset_for_tests();
        let child = p(2);
        record_child_grant(child, 950, 5);
        let policy = FundingPolicy {
            max_per_request: 200,
            max_per_child: 1_000,
            cooldown_secs: 0,
        };

        let decision = policy.evaluate(child, 200, 10).expect("must clamp");
        assert_eq!(decision.approved_cycles, 50);
        assert!(!decision.clamped_max_per_request);
        assert!(decision.clamped_max_per_child);
    }

    #[test]
    fn evaluate_denies_when_child_budget_exhausted() {
        reset_for_tests();
        let child = p(3);
        record_child_grant(child, 1_000, 5);
        let policy = FundingPolicy {
            max_per_request: 200,
            max_per_child: 1_000,
            cooldown_secs: 0,
        };

        let err = policy
            .evaluate(child, 1, 10)
            .expect_err("budget exhaustion must deny");
        match err {
            FundingPolicyViolation::MaxPerChild {
                requested,
                max_per_child,
                remaining_budget,
            } => {
                assert_eq!(requested, 1);
                assert_eq!(max_per_child, 1_000);
                assert_eq!(remaining_budget, 0);
            }
            FundingPolicyViolation::CooldownActive { .. } => {
                panic!("expected child-budget violation")
            }
        }
    }

    #[test]
    fn evaluate_denies_when_cooldown_active() {
        reset_for_tests();
        let child = p(4);
        record_child_grant(child, 100, 50);
        let policy = FundingPolicy {
            max_per_request: 1_000,
            max_per_child: 10_000,
            cooldown_secs: 20,
        };

        let err = policy
            .evaluate(child, 10, 60)
            .expect_err("cooldown must deny");
        match err {
            FundingPolicyViolation::CooldownActive { retry_after_secs } => {
                assert_eq!(retry_after_secs, 10);
            }
            FundingPolicyViolation::MaxPerChild { .. } => panic!("expected cooldown violation"),
        }
    }
}
