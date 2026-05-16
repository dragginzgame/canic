use crate::ids::CanisterRole;

///
/// FundingPolicy
/// Effective parent funding limits for a single child role.
///

#[derive(Clone, Copy, Debug)]
pub struct FundingPolicy {
    pub max_per_request: u128,
    pub max_per_child: u128,
    pub cooldown_secs: u64,
}

impl FundingPolicy {
    /// Evaluate a child request against role policy and the observed grant ledger.
    pub const fn evaluate(
        self,
        ledger: FundingLedgerSnapshot,
        requested_cycles: u128,
        now_secs: u64,
    ) -> Result<FundingDecision, FundingPolicyViolation> {
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
pub struct FundingDecision {
    pub approved_cycles: u128,
    pub clamped_max_per_request: bool,
    pub clamped_max_per_child: bool,
}

///
/// FundingLedgerSnapshot
/// Read-only child funding ledger input for policy evaluation.
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FundingLedgerSnapshot {
    pub granted_total: u128,
    pub last_granted_at: u64,
}

///
/// FundingPolicyViolation
/// Pure policy violations for funding authorization.
///

#[derive(Clone, Copy, Debug)]
pub enum FundingPolicyViolation {
    MaxPerChild {
        requested: u128,
        max_per_child: u128,
        remaining_budget: u128,
    },
    CooldownActive {
        retry_after_secs: u64,
    },
}

/// Resolve the effective policy for a child role from current config.
#[must_use]
pub const fn policy_for_child_role(_child_role: &CanisterRole) -> FundingPolicy {
    default_policy()
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
mod tests {
    use super::*;

    #[test]
    fn evaluate_clamps_to_max_per_request() {
        let policy = FundingPolicy {
            max_per_request: 100,
            max_per_child: 1_000,
            cooldown_secs: 0,
        };

        let decision = policy
            .evaluate(FundingLedgerSnapshot::default(), 250, 10)
            .expect("must clamp");
        assert_eq!(decision.approved_cycles, 100);
        assert!(decision.clamped_max_per_request);
        assert!(!decision.clamped_max_per_child);
    }

    #[test]
    fn evaluate_clamps_to_remaining_budget() {
        let policy = FundingPolicy {
            max_per_request: 200,
            max_per_child: 1_000,
            cooldown_secs: 0,
        };
        let ledger = FundingLedgerSnapshot {
            granted_total: 950,
            last_granted_at: 5,
        };

        let decision = policy.evaluate(ledger, 200, 10).expect("must clamp");
        assert_eq!(decision.approved_cycles, 50);
        assert!(!decision.clamped_max_per_request);
        assert!(decision.clamped_max_per_child);
    }

    #[test]
    fn evaluate_denies_when_child_budget_exhausted() {
        let policy = FundingPolicy {
            max_per_request: 200,
            max_per_child: 1_000,
            cooldown_secs: 0,
        };
        let ledger = FundingLedgerSnapshot {
            granted_total: 1_000,
            last_granted_at: 5,
        };

        let err = policy
            .evaluate(ledger, 1, 10)
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
        let policy = FundingPolicy {
            max_per_request: 1_000,
            max_per_child: 10_000,
            cooldown_secs: 20,
        };
        let ledger = FundingLedgerSnapshot {
            granted_total: 100,
            last_granted_at: 50,
        };

        let err = policy
            .evaluate(ledger, 10, 60)
            .expect_err("cooldown must deny");
        match err {
            FundingPolicyViolation::CooldownActive { retry_after_secs } => {
                assert_eq!(retry_after_secs, 10);
            }
            FundingPolicyViolation::MaxPerChild { .. } => panic!("expected cooldown violation"),
        }
    }
}
