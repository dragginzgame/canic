///
/// IcpRefillPolicyInput
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IcpRefillPolicyInput {
    pub requested_amount_e8s: u64,
    pub observed_xdr_permyriad_per_icp: Option<u64>,
    pub observed_fee_e8s: Option<u64>,
    pub observed_source_balance_e8s: Option<u64>,
    pub window_reserved_e8s: u64,
    pub active_for_key: bool,
    pub cycles_funding_enabled: bool,
}

///
/// IcpRefillPolicyRules
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRefillPolicyRules {
    pub max_refill_e8s_per_call: u64,
    pub maximum_refill_e8s: u64,
    pub minimum_icp_balance_e8s: u64,
    pub min_xdr_permyriad_per_icp: Option<u64>,
}

/// Non-renewing automatic ICP authority retained beside rolling-window usage.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AutomaticIcpRefillUsage {
    pub completed_refills: u32,
    pub completed_refill_e8s: u64,
}

/// Additional immutable rules that exist only for timer-triggered refill.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutomaticIcpRefillRules {
    pub maximum_automatic_refills: u32,
    pub maximum_automatic_refill_e8s: u64,
}

/// Typed failure while deriving the exact ICP amount needed by an automatic refill.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutomaticIcpRefillAmountError {
    AmountOverflow { required_e8s: u128 },
    RateZero,
    TargetSatisfied,
}

///
/// IcpRefillPolicyViolation
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IcpRefillPolicyViolation {
    NotConfigured,
    CyclesFundingDisabled,
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
    WindowBudgetExhausted {
        requested_e8s: u64,
        remaining_e8s: u64,
    },
    BalanceFloorUnavailable {
        observed_balance_e8s: u64,
        required_e8s: u64,
    },
    AmountAndFeeOverflow,
    AutomaticRefillCountExhausted {
        completed_refills: u32,
        maximum_refills: u32,
    },
    AutomaticRefillSpendExhausted {
        requested_e8s: u64,
        remaining_e8s: u64,
    },
    ConcurrentRefill,
}

/// Evaluate an operator-triggered canister-side refill request.
///
/// Manual refills are constrained by the configured cap, rate gate, funding
/// controls, and concurrency key.
pub fn evaluate_manual_refill(
    policy: Option<&IcpRefillPolicyRules>,
    input: IcpRefillPolicyInput,
) -> Result<(), IcpRefillPolicyViolation> {
    evaluate_common_option(policy, input)
}

/// Evaluate one timer-triggered refill after its exact amount is derived.
pub fn evaluate_automatic_refill(
    policy: Option<&IcpRefillPolicyRules>,
    automatic: Option<&AutomaticIcpRefillRules>,
    usage: AutomaticIcpRefillUsage,
    input: IcpRefillPolicyInput,
) -> Result<(), IcpRefillPolicyViolation> {
    let Some(automatic) = automatic else {
        return Err(IcpRefillPolicyViolation::NotConfigured);
    };
    evaluate_common_option(policy, input)?;
    if usage.completed_refills >= automatic.maximum_automatic_refills {
        return Err(IcpRefillPolicyViolation::AutomaticRefillCountExhausted {
            completed_refills: usage.completed_refills,
            maximum_refills: automatic.maximum_automatic_refills,
        });
    }
    let remaining_e8s = automatic
        .maximum_automatic_refill_e8s
        .saturating_sub(usage.completed_refill_e8s);
    if input.requested_amount_e8s > remaining_e8s {
        return Err(IcpRefillPolicyViolation::AutomaticRefillSpendExhausted {
            requested_e8s: input.requested_amount_e8s,
            remaining_e8s,
        });
    }
    Ok(())
}

/// Derive the minimum ICP e8s that reaches the exact automatic cycle target.
pub fn automatic_refill_amount_e8s(
    current_cycles: u128,
    target_cycles: u128,
    xdr_permyriad_per_icp: u64,
) -> Result<u64, AutomaticIcpRefillAmountError> {
    let Some(deficit) = target_cycles.checked_sub(current_cycles) else {
        return Err(AutomaticIcpRefillAmountError::TargetSatisfied);
    };
    if deficit == 0 {
        return Err(AutomaticIcpRefillAmountError::TargetSatisfied);
    }
    if xdr_permyriad_per_icp == 0 {
        return Err(AutomaticIcpRefillAmountError::RateZero);
    }

    let rate = u128::from(xdr_permyriad_per_icp);
    let quotient = deficit / rate;
    let required_e8s = quotient
        .checked_add(u128::from(deficit % rate != 0))
        .ok_or(AutomaticIcpRefillAmountError::AmountOverflow {
            required_e8s: u128::MAX,
        })?;
    u64::try_from(required_e8s)
        .map_err(|_| AutomaticIcpRefillAmountError::AmountOverflow { required_e8s })
}

fn evaluate_common_option(
    policy: Option<&IcpRefillPolicyRules>,
    input: IcpRefillPolicyInput,
) -> Result<(), IcpRefillPolicyViolation> {
    let Some(policy) = policy else {
        return Err(IcpRefillPolicyViolation::NotConfigured);
    };
    evaluate_common(policy, input)
}

fn evaluate_common(
    policy: &IcpRefillPolicyRules,
    input: IcpRefillPolicyInput,
) -> Result<(), IcpRefillPolicyViolation> {
    if !input.cycles_funding_enabled {
        return Err(IcpRefillPolicyViolation::CyclesFundingDisabled);
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
    let remaining_e8s = policy
        .maximum_refill_e8s
        .saturating_sub(input.window_reserved_e8s);
    if input.requested_amount_e8s > remaining_e8s {
        return Err(IcpRefillPolicyViolation::WindowBudgetExhausted {
            requested_e8s: input.requested_amount_e8s,
            remaining_e8s,
        });
    }
    if let (Some(fee_e8s), Some(observed_balance_e8s)) =
        (input.observed_fee_e8s, input.observed_source_balance_e8s)
    {
        let Some(required_e8s) = input
            .requested_amount_e8s
            .checked_add(fee_e8s)
            .and_then(|debit| debit.checked_add(policy.minimum_icp_balance_e8s))
        else {
            return Err(IcpRefillPolicyViolation::AmountAndFeeOverflow);
        };
        if observed_balance_e8s < required_e8s {
            return Err(IcpRefillPolicyViolation::BalanceFloorUnavailable {
                observed_balance_e8s,
                required_e8s,
            });
        }
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> IcpRefillPolicyRules {
        IcpRefillPolicyRules {
            max_refill_e8s_per_call: 100_000_000,
            maximum_refill_e8s: 200_000_000,
            minimum_icp_balance_e8s: 10_000_000,
            min_xdr_permyriad_per_icp: Some(40_000),
        }
    }

    fn input() -> IcpRefillPolicyInput {
        IcpRefillPolicyInput {
            requested_amount_e8s: 50_000_000,
            observed_xdr_permyriad_per_icp: Some(45_000),
            observed_fee_e8s: Some(10_000),
            observed_source_balance_e8s: Some(200_000_000),
            window_reserved_e8s: 25_000_000,
            active_for_key: false,
            cycles_funding_enabled: true,
        }
    }

    #[test]
    fn manual_refill_allows_configured_request() {
        evaluate_manual_refill(Some(&policy()), input()).expect("manual refill");
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
    fn refill_reserves_the_window_and_retains_amount_fee_and_floor() {
        let mut window = input();
        window.window_reserved_e8s = 175_000_001;
        assert_eq!(
            evaluate_manual_refill(Some(&policy()), window),
            Err(IcpRefillPolicyViolation::WindowBudgetExhausted {
                requested_e8s: 50_000_000,
                remaining_e8s: 24_999_999,
            })
        );

        let mut balance = input();
        balance.observed_source_balance_e8s = Some(60_009_999);
        assert_eq!(
            evaluate_manual_refill(Some(&policy()), balance),
            Err(IcpRefillPolicyViolation::BalanceFloorUnavailable {
                observed_balance_e8s: 60_009_999,
                required_e8s: 60_010_000,
            })
        );
    }

    #[test]
    fn automatic_refill_caps_are_nonrenewing_and_independent_of_the_window() {
        let automatic = AutomaticIcpRefillRules {
            maximum_automatic_refills: 4,
            maximum_automatic_refill_e8s: 180_000_000,
        };
        evaluate_automatic_refill(
            Some(&policy()),
            Some(&automatic),
            AutomaticIcpRefillUsage {
                completed_refills: 3,
                completed_refill_e8s: 125_000_000,
            },
            input(),
        )
        .expect("last admitted automatic refill");

        assert_eq!(
            evaluate_automatic_refill(
                Some(&policy()),
                Some(&automatic),
                AutomaticIcpRefillUsage {
                    completed_refills: 4,
                    completed_refill_e8s: 125_000_000,
                },
                input(),
            ),
            Err(IcpRefillPolicyViolation::AutomaticRefillCountExhausted {
                completed_refills: 4,
                maximum_refills: 4,
            })
        );

        assert_eq!(
            evaluate_automatic_refill(
                Some(&policy()),
                Some(&automatic),
                AutomaticIcpRefillUsage {
                    completed_refills: 3,
                    completed_refill_e8s: 130_000_001,
                },
                input(),
            ),
            Err(IcpRefillPolicyViolation::AutomaticRefillSpendExhausted {
                requested_e8s: 50_000_000,
                remaining_e8s: 49_999_999,
            })
        );
    }

    #[test]
    fn automatic_refill_amount_uses_exact_ceiling_division() {
        assert_eq!(
            automatic_refill_amount_e8s(100, 200, 25).expect("exact division"),
            4
        );
        assert_eq!(
            automatic_refill_amount_e8s(100, 201, 25).expect("rounded division"),
            5
        );
    }

    #[test]
    fn automatic_refill_amount_fails_closed_for_invalid_inputs() {
        assert_eq!(
            automatic_refill_amount_e8s(200, 200, 25),
            Err(AutomaticIcpRefillAmountError::TargetSatisfied)
        );
        assert_eq!(
            automatic_refill_amount_e8s(200, 199, 25),
            Err(AutomaticIcpRefillAmountError::TargetSatisfied)
        );
        assert_eq!(
            automatic_refill_amount_e8s(100, 200, 0),
            Err(AutomaticIcpRefillAmountError::RateZero)
        );
        assert!(matches!(
            automatic_refill_amount_e8s(0, u128::MAX, 1),
            Err(AutomaticIcpRefillAmountError::AmountOverflow { .. })
        ));
    }
}
