use super::*;
use crate::fleet_ensure::json;

// Field order is the historical digest input, even though on-disk JSON sorts keys.
const RECORDED: &str = concat!(
    "{\"estate_funding_cycles\":\"0\",",
    "\"exact_estate_creation_fee_cycles\":\"0\",",
    "\"exact_unavoidable_fee_cycles\":\"10\",",
    "\"final_controlled_cycles\":\"600\",",
    "\"measured_execution_burn_cycles\":\"10\",",
    "\"observed_starting_cycles\":\"500\",",
    "\"observed_settlement_credit_cycles\":\"0\",",
    "\"operator_debit_cycles\":\"120\",",
    "\"received_new_funding_cycles\":\"110\"}"
);

#[test]
fn retirement_evidence_preserves_exact_historical_digest_input() {
    let value: serde_json::Value = serde_json::from_str(RECORDED).unwrap();
    let record: FleetRetirementConservationRecord = serde_json::from_value(value).unwrap();
    assert!(matches!(
        record,
        FleetRetirementConservationRecord::RecordedExecution(_)
    ));
    assert_eq!(json::to_vec(&record).unwrap(), RECORDED.as_bytes());
    assert!(serde_json::from_str::<ActualCycleConservation>(RECORDED).is_err());
}

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

#[test]
fn retirement_evidence_rejects_missing_mixed_and_unrecognized_accounting() {
    let value: serde_json::Value = serde_json::from_str(RECORDED).unwrap();
    for key in value.as_object().unwrap().keys() {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<FleetRetirementConservationRecord>(missing).is_err());
    }
    for (key, replacement) in [
        ("observed_net_cycle_debit_cycles", serde_json::json!("10")),
        ("observed_net_cycle_credit_cycles", serde_json::json!("0")),
        ("unrecognized_accounting", serde_json::json!("0")),
        ("measured_execution_burn_cycles", serde_json::json!(-1)),
        ("operator_debit_cycles", serde_json::Value::Null),
    ] {
        let mut invalid = value.clone();
        invalid[key] = replacement;
        assert!(serde_json::from_value::<FleetRetirementConservationRecord>(invalid).is_err());
    }
}
