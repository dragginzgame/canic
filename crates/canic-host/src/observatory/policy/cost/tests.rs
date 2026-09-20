use super::*;

#[test]
fn grant_adjustment_handles_funding_balance_growth_and_full_width_cancellation() {
    for (opening, closing, incoming, outgoing, expected) in [
        (1_000, 500, 0, 500, difference(0, 0)),
        (100, 550, 500, 0, difference(50, 0)),
        (100, 600, 500, 0, difference(0, 0)),
        (100, 700, 500, 0, difference(0, 100)),
        (u128::MAX, 0, u128::MAX, u128::MAX, difference(u128::MAX, 0)),
        (0, u128::MAX, u128::MAX, 0, difference(0, 0)),
        (0, u128::MAX, 0, 0, difference(0, u128::MAX)),
    ] {
        assert_eq!(
            grant_adjusted_decrease(opening, closing, incoming, outgoing),
            Ok(expected)
        );
    }
    assert_eq!(
        grant_adjusted_decrease(u128::MAX, 0, 1, 0),
        Err(CostIntervalIssue::Overflow)
    );
    assert_eq!(
        grant_adjusted_decrease(0, u128::MAX, 0, 1),
        Err(CostIntervalIssue::Overflow)
    );
}

#[test]
fn counters_require_advancing_unsaturated_matching_windows() {
    let before = CostCounterReading {
        value: 9,
        observed_at_ns: 10,
        window_id: 4,
        saturated: false,
    };
    let after = CostCounterReading {
        value: 20,
        observed_at_ns: 30,
        ..before
    };
    assert_eq!(
        counter(before, after),
        Ok(CounterMovement {
            interval: CostInterval {
                start_ns: 10,
                end_ns: 30
            },
            amount: 11
        })
    );
    for (reading, failure) in [
        (
            CostCounterReading { value: 8, ..after },
            CostIntervalIssue::CounterDecreased,
        ),
        (
            CostCounterReading {
                window_id: 5,
                ..after
            },
            CostIntervalIssue::CounterWindowChanged,
        ),
        (
            CostCounterReading {
                saturated: true,
                ..after
            },
            CostIntervalIssue::SaturatedCounter,
        ),
        (
            CostCounterReading {
                observed_at_ns: 10,
                ..after
            },
            CostIntervalIssue::NonAdvancingWindow,
        ),
        (
            CostCounterReading {
                observed_at_ns: 9,
                ..after
            },
            CostIntervalIssue::NonAdvancingWindow,
        ),
    ] {
        assert_eq!(counter(before, reading), Err(failure));
    }
    assert_eq!(
        counter(
            CostCounterReading {
                saturated: true,
                ..before
            },
            after
        ),
        Err(CostIntervalIssue::SaturatedCounter)
    );
}
