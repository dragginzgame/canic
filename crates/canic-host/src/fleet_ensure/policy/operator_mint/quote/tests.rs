use super::amount_e8s;

#[test]
fn conversion_quote_rounds_up_with_both_fees_separate() {
    assert_eq!(
        amount_e8s(80_681_069_959_196, 100_000_000, 20_423, 10_000),
        Some(3_950_505_311)
    );
    assert_eq!(amount_e8s(100, 0, 10, 1), Some(10));
    assert_eq!(amount_e8s(101, 9, 10, 1), Some(11));
    assert_eq!(amount_e8s(101, 10, 10, 1), Some(12));
    assert_eq!(amount_e8s(1, 0, 10, 1), Some(1));
}

#[test]
fn conversion_quote_rejects_unrepresentable_or_zero_inputs() {
    for (shortfall, fee, rate, transfer) in [
        (0, 1, 1, 0),
        (1, 0, 0, 0),
        (u128::MAX, 1, 1, 0),
        (u128::MAX, 0, 1, 0),
        (u128::from(u64::MAX), 0, 1, 1),
    ] {
        assert_eq!(amount_e8s(shortfall, fee, rate, transfer), None);
    }
}
