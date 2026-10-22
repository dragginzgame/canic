//! Current Fleet journal decoding at the optional-release boundary.

use super::{FLEET_ENSURE_SCHEMA_VERSION, FleetEnsureCompletion, FleetEnsureJournalRecord};
use std::collections::BTreeMap;

#[test]
fn journal_release_omission_is_neutral_without_weakening_required_fields() {
    let journal = FleetEnsureJournalRecord {
        release: None,
        bootstrap_registration_recovery: None,
        funding_observations: BTreeMap::new(),
        funding_reviews: Vec::new(),
        successor_phases: Vec::new(),
        completion: FleetEnsureCompletion::Converged,
        estate_funding_required: None,
        effects: Vec::new(),
        fleet: "demo".into(),
        initial_controlled_cycles: 0,
        initial_estate_funding_cycles_by_root: BTreeMap::new(),
        initial_operator_cycles: 0,
        operation_id: "11".repeat(32),
        plan_sha256: "22".repeat(32),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        stalled_observations: 0,
    };
    let mut value = serde_json::to_value(&journal).unwrap();
    assert!(value["release"].is_null());
    assert_eq!(
        serde_json::from_value::<FleetEnsureJournalRecord>(value.clone()).unwrap(),
        journal
    );
    value.as_object_mut().unwrap().remove("release");
    assert_eq!(
        serde_json::from_value::<FleetEnsureJournalRecord>(value.clone()).unwrap(),
        journal
    );
    for invalid in [
        serde_json::json!({}),
        serde_json::json!(false),
        serde_json::json!("release"),
    ] {
        let mut invalid_record = value.clone();
        invalid_record["release"] = invalid;
        assert!(
            serde_json::from_value::<FleetEnsureJournalRecord>(invalid_record)
                .unwrap_err()
                .is_data()
        );
    }
    for required in [
        "bootstrap_registration_recovery",
        "estate_funding_required",
        "operation_id",
        "plan_sha256",
    ] {
        let mut incomplete = value.clone();
        incomplete.as_object_mut().unwrap().remove(required);
        assert!(
            serde_json::from_value::<FleetEnsureJournalRecord>(incomplete)
                .unwrap_err()
                .is_data()
        );
    }
    let normalized: FleetEnsureJournalRecord = serde_json::from_value(value).unwrap();
    let encoded = serde_json::to_value(normalized).unwrap();
    assert!(encoded.as_object().unwrap().contains_key("release"));
    assert!(encoded["release"].is_null());
}
