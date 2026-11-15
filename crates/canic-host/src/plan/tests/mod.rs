use super::*;

#[test]
fn retained_actions_preserve_approval_digests() {
    #[derive(Deserialize)]
    struct Fixture {
        digest: String,
        plan: Plan,
    }
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("fixtures/retained.json")).unwrap();
    for fixture in fixtures {
        assert_eq!(fixture.plan.digest().unwrap(), fixture.digest);
        if let Some(owner) = fixture.plan.management() {
            assert!(owner.execute_at_ns > fixture.plan.created_at_ns);
            assert_eq!(
                admission_deadline(
                    fixture.plan.created_at_ns,
                    owner.execute_at_ns,
                    fixture.plan.expires_at_ns
                ),
                owner.execute_at_ns
            );
        }
        let encoded = serde_json::to_vec(&fixture.plan).unwrap();
        let decoded: Plan = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, fixture.plan);
    }
}

#[test]
fn ready_admission_ends_at_original_expiry_and_scheduled_admission_stays_earlier() {
    assert_eq!(admission_deadline(100, 100, 200), 200);
    assert_eq!(admission_deadline(100, 150, 200), 150);
}

#[test]
fn incomplete_or_conflicting_record_authority_is_rejected_at_decode() {
    let records: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/retained.json")).unwrap();
    let upgrade = &records[0]["plan"];
    let maintenance = records
        .iter()
        .find(|r| !r["plan"]["maintenance"].is_null())
        .unwrap()["plan"]
        .clone();
    let creation = records
        .iter()
        .find(|r| r["plan"]["target"].is_null())
        .unwrap()["plan"]
        .clone();
    for (field, value) in [
        ("target", serde_json::Value::Null),
        ("previous_module", serde_json::Value::Null),
        ("creation_cycles", serde_json::json!(1)),
        ("maintenance", maintenance["maintenance"].clone()),
        (
            "maintenance_sibling",
            maintenance["maintenance_sibling"].clone(),
        ),
    ] {
        let mut record = upgrade.clone();
        record[field] = value;
        assert!(serde_json::from_value::<Plan>(record).is_err(), "{field}");
    }
    let mut record = creation;
    record["creation_cycles"] = serde_json::json!(canic_management_protocol::CREATION_CYCLES + 1);
    assert!(serde_json::from_value::<Plan>(record).is_err());
    let mut record = maintenance;
    record["maintenance"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<Plan>(record).is_err());
}
