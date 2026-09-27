use super::*;
use crate::fleet_ensure::json;

#[test]
fn retirement_evidence_preserves_current_digest_input() {
    let actual = ActualCycleConservation {
        estate_funding_cycles: 0,
        exact_estate_creation_fee_cycles: 0,
        exact_unavoidable_fee_cycles: 10,
        final_controlled_cycles: 600,
        observed_net_cycle_debit_cycles: 10,
        observed_starting_cycles: 500,
        observed_net_cycle_credit_cycles: 0,
        operator_debit_cycles: 120,
        received_new_funding_cycles: 110,
    };
    let record: FleetRetirementConservationRecord =
        serde_json::from_value(json::to_value(&actual).unwrap()).unwrap();
    assert_eq!(
        json::to_vec(&record).unwrap(),
        json::to_vec(&actual).unwrap()
    );
    assert_eq!(
        record,
        FleetRetirementConservationRecord::NetBalance(actual)
    );
}
