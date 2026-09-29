//! Target-local funding windows reject overflow without changing generated burn allowances.

use super::*;
use crate::fleet_ensure::policy::CycleBounds;

#[test]
fn bootstrap_funding_window_preserves_floor_and_rejects_overflow() {
    let bounds = CycleBounds {
        observation_burn: 1_000_000_000_000,
        update_burn: 1_000_000_000_000,
        ledger_fee: 100_000_000,
        management_creation_fee: 500_000_000_000,
        material_threshold: 1_000_000,
    };
    let minimum = 270_000_000_000_000;
    let funded = funding_floor(4, minimum, bounds).unwrap();
    assert!(funded > minimum);
    assert!(funded - minimum < 100_000_000_000_000);
    assert!(matches!(
        funding_floor(4, u128::MAX, bounds),
        Err(EnsurePolicyError::ArithmeticOverflow { .. })
    ));
    assert!(matches!(
        funding_floor(
            4,
            0,
            CycleBounds {
                observation_burn: u128::MAX,
                ..bounds
            }
        ),
        Err(EnsurePolicyError::ArithmeticOverflow { .. })
    ));
}
