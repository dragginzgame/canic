//! Known shortfalls and unavailable balances remain distinct before infrastructure effects.

use super::*;

#[test]
fn import_headroom_reports_native_shortfalls_without_fabricating_unknown_balances() {
    let minimum = 1_000_000_000_000;
    let required = minimum + DEFAULT_IMPORT_SOURCE_DEBIT_CYCLES;
    for (available, shortfall) in [
        (0, required),
        (minimum, DEFAULT_IMPORT_SOURCE_DEBIT_CYCLES),
        (required, 0),
        (required + 1, 0),
    ] {
        assert_eq!(
            import_headroom(minimum, Some(available)).assessment,
            ImportHeadroomAssessment::Observed {
                required_cycles: required,
                available_cycles: available,
                shortfall_cycles: shortfall
            }
        );
    }
    assert_eq!(
        import_headroom(minimum, None).assessment,
        ImportHeadroomAssessment::AwaitingCurrentRootObservation
    );
    assert_eq!(
        import_headroom(u128::MAX, Some(u128::MAX)).assessment,
        ImportHeadroomAssessment::InvalidBounds
    );
}
