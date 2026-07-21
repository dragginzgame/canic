use crate::domain::value::Cycles;

///
/// IcpRefillPolicyInput
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IcpRefillPolicyInput {
    pub hub_cycles: u128,
    pub requested_amount_e8s: u64,
    pub observed_xdr_permyriad_per_icp: Option<u64>,
    pub active_for_key: bool,
    pub cycles_funding_enabled: bool,
    pub funding_cooldown_retry_after_secs: Option<u64>,
}

///
/// IcpRefillPolicyRules
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRefillPolicyRules {
    pub enabled: bool,
    pub min_hub_cycles_before_refill: Cycles,
    pub max_refill_e8s_per_call: u64,
    pub min_xdr_permyriad_per_icp: Option<u64>,
}

///
/// IcpRefillDecision
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IcpRefillDecision {
    pub amount_e8s: u64,
    pub threshold_cycles: u128,
    pub current_cycles: u128,
}

///
/// IcpRefillPolicyViolation
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IcpRefillPolicyViolation {
    NotConfigured,
    Disabled,
    CyclesFundingDisabled,
    FundingCooldownActive {
        retry_after_secs: u64,
    },
    AmountZero,
    MaxRefillPerCall {
        requested_e8s: u64,
        max_e8s: u64,
    },
    RateUnavailable {
        min_xdr_permyriad_per_icp: u64,
    },
    RateGateDenied {
        observed_xdr_permyriad_per_icp: u64,
        min_xdr_permyriad_per_icp: u64,
    },
    HubCyclesAboveThreshold {
        current_cycles: u128,
        threshold_cycles: u128,
    },
    ConcurrentRefill,
}

/// Evaluate an operator-triggered canister-side refill request.
///
/// Manual refills are still constrained by the configured cap, rate gate, and
/// concurrency key, but they do not require the hub to already be below its
/// timer-driven self-refill threshold.
pub const fn evaluate_manual_refill(
    policy: Option<&IcpRefillPolicyRules>,
    input: IcpRefillPolicyInput,
) -> Result<IcpRefillDecision, IcpRefillPolicyViolation> {
    let Some(policy) = policy else {
        return Err(IcpRefillPolicyViolation::NotConfigured);
    };

    evaluate_common(policy, input)
}

/// Evaluate the timer-driven hub self-refill branch.
pub const fn evaluate_hub_self_refill(
    policy: Option<&IcpRefillPolicyRules>,
    input: IcpRefillPolicyInput,
) -> Result<IcpRefillDecision, IcpRefillPolicyViolation> {
    let Some(policy) = policy else {
        return Err(IcpRefillPolicyViolation::NotConfigured);
    };

    let decision = match evaluate_common(policy, input) {
        Ok(decision) => decision,
        Err(err) => return Err(err),
    };
    if input.hub_cycles > decision.threshold_cycles {
        return Err(IcpRefillPolicyViolation::HubCyclesAboveThreshold {
            current_cycles: input.hub_cycles,
            threshold_cycles: decision.threshold_cycles,
        });
    }

    Ok(decision)
}

const fn evaluate_common(
    policy: &IcpRefillPolicyRules,
    input: IcpRefillPolicyInput,
) -> Result<IcpRefillDecision, IcpRefillPolicyViolation> {
    if !input.cycles_funding_enabled {
        return Err(IcpRefillPolicyViolation::CyclesFundingDisabled);
    }
    if !policy.enabled {
        return Err(IcpRefillPolicyViolation::Disabled);
    }
    if input.requested_amount_e8s == 0 {
        return Err(IcpRefillPolicyViolation::AmountZero);
    }
    if input.requested_amount_e8s > policy.max_refill_e8s_per_call {
        return Err(IcpRefillPolicyViolation::MaxRefillPerCall {
            requested_e8s: input.requested_amount_e8s,
            max_e8s: policy.max_refill_e8s_per_call,
        });
    }
    if input.active_for_key {
        return Err(IcpRefillPolicyViolation::ConcurrentRefill);
    }
    if let Some(retry_after_secs) = input.funding_cooldown_retry_after_secs {
        return Err(IcpRefillPolicyViolation::FundingCooldownActive { retry_after_secs });
    }
    if let Some(min_xdr_permyriad_per_icp) = policy.min_xdr_permyriad_per_icp {
        match input.observed_xdr_permyriad_per_icp {
            Some(observed_xdr_permyriad_per_icp)
                if observed_xdr_permyriad_per_icp >= min_xdr_permyriad_per_icp => {}
            Some(observed_xdr_permyriad_per_icp) => {
                return Err(IcpRefillPolicyViolation::RateGateDenied {
                    observed_xdr_permyriad_per_icp,
                    min_xdr_permyriad_per_icp,
                });
            }
            None => {
                return Err(IcpRefillPolicyViolation::RateUnavailable {
                    min_xdr_permyriad_per_icp,
                });
            }
        }
    }

    Ok(IcpRefillDecision {
        amount_e8s: input.requested_amount_e8s,
        threshold_cycles: policy.min_hub_cycles_before_refill.to_u128(),
        current_cycles: input.hub_cycles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value::{Cycles, TC};

    fn policy() -> IcpRefillPolicyRules {
        IcpRefillPolicyRules {
            enabled: true,
            min_hub_cycles_before_refill: Cycles::new(2 * TC),
            max_refill_e8s_per_call: 100_000_000,
            min_xdr_permyriad_per_icp: Some(40_000),
        }
    }

    fn input() -> IcpRefillPolicyInput {
        IcpRefillPolicyInput {
            hub_cycles: TC,
            requested_amount_e8s: 50_000_000,
            observed_xdr_permyriad_per_icp: Some(45_000),
            active_for_key: false,
            cycles_funding_enabled: true,
            funding_cooldown_retry_after_secs: None,
        }
    }

    #[test]
    fn manual_refill_allows_configured_request_without_low_balance_gate() {
        let mut input = input();
        input.hub_cycles = 3 * TC;

        let decision = evaluate_manual_refill(Some(&policy()), input).expect("manual refill");

        assert_eq!(decision.amount_e8s, 50_000_000);
        assert_eq!(decision.current_cycles, 3 * TC);
        assert_eq!(decision.threshold_cycles, 2 * TC);
    }

    #[test]
    fn hub_self_refill_rejects_balance_above_threshold() {
        let mut input = input();
        input.hub_cycles = 2 * TC + 1;

        let err = evaluate_hub_self_refill(Some(&policy()), input).expect_err("threshold gate");

        assert_eq!(
            err,
            IcpRefillPolicyViolation::HubCyclesAboveThreshold {
                current_cycles: 2 * TC + 1,
                threshold_cycles: 2 * TC,
            }
        );
    }

    #[test]
    fn hub_self_refill_accepts_exact_threshold() {
        let mut input = input();
        input.hub_cycles = 2 * TC;

        evaluate_hub_self_refill(Some(&policy()), input).expect("threshold refill");
    }

    #[test]
    fn hub_self_refill_accepts_low_balance_request() {
        let decision = evaluate_hub_self_refill(Some(&policy()), input()).expect("low balance");

        assert_eq!(decision.amount_e8s, 50_000_000);
    }

    #[test]
    fn refill_denies_amount_above_cap() {
        let mut input = input();
        input.requested_amount_e8s = 100_000_001;

        let err = evaluate_manual_refill(Some(&policy()), input).expect_err("cap violation");

        assert_eq!(
            err,
            IcpRefillPolicyViolation::MaxRefillPerCall {
                requested_e8s: 100_000_001,
                max_e8s: 100_000_000,
            }
        );
    }

    #[test]
    fn refill_denies_missing_rate_when_gate_configured() {
        let mut input = input();
        input.observed_xdr_permyriad_per_icp = None;

        let err = evaluate_manual_refill(Some(&policy()), input).expect_err("rate required");

        assert_eq!(
            err,
            IcpRefillPolicyViolation::RateUnavailable {
                min_xdr_permyriad_per_icp: 40_000,
            }
        );
    }

    #[test]
    fn refill_denies_low_rate() {
        let mut input = input();
        input.observed_xdr_permyriad_per_icp = Some(39_999);

        let err = evaluate_manual_refill(Some(&policy()), input).expect_err("rate too low");

        assert_eq!(
            err,
            IcpRefillPolicyViolation::RateGateDenied {
                observed_xdr_permyriad_per_icp: 39_999,
                min_xdr_permyriad_per_icp: 40_000,
            }
        );
    }

    #[test]
    fn refill_denies_concurrent_key() {
        let mut input = input();
        input.active_for_key = true;

        let err = evaluate_manual_refill(Some(&policy()), input).expect_err("concurrent refill");

        assert_eq!(err, IcpRefillPolicyViolation::ConcurrentRefill);
    }

    #[test]
    fn manual_refill_denies_when_cycles_funding_disabled() {
        let mut input = input();
        input.cycles_funding_enabled = false;

        let err = evaluate_manual_refill(Some(&policy()), input).expect_err("kill switch");

        assert_eq!(err, IcpRefillPolicyViolation::CyclesFundingDisabled);
    }

    #[test]
    fn hub_self_refill_denies_when_cycles_funding_disabled() {
        let mut input = input();
        input.cycles_funding_enabled = false;
        let err = evaluate_hub_self_refill(Some(&policy()), input).expect_err("kill switch");

        assert_eq!(err, IcpRefillPolicyViolation::CyclesFundingDisabled);
    }

    #[test]
    fn manual_refill_denies_when_funding_cooldown_active() {
        let mut input = input();
        input.funding_cooldown_retry_after_secs = Some(11);

        let err = evaluate_manual_refill(Some(&policy()), input).expect_err("cooldown");

        assert_eq!(
            err,
            IcpRefillPolicyViolation::FundingCooldownActive {
                retry_after_secs: 11
            }
        );
    }

    #[test]
    fn hub_self_refill_denies_when_funding_cooldown_active() {
        let mut input = input();
        input.funding_cooldown_retry_after_secs = Some(12);
        let err = evaluate_hub_self_refill(Some(&policy()), input).expect_err("cooldown");

        assert_eq!(
            err,
            IcpRefillPolicyViolation::FundingCooldownActive {
                retry_after_secs: 12
            }
        );
    }
}
