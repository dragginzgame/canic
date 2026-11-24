use super::*;

const WASM: &[u8] = b"\0asm\x01\0\0\0";

#[test]
fn numeric_child_identity_keeps_retained_json_names() {
    use canic_management_protocol::children::ChildId;
    for (identity, encoded) in [
        (ChildId::FIRST, "\"First\""),
        (ChildId::SECOND, "\"Second\""),
    ] {
        assert_eq!(serde_json::to_string(&identity).unwrap(), encoded);
        assert_eq!(serde_json::from_str::<ChildId>(encoded).unwrap(), identity);
    }
    for invalid in ["0", "1", "2", "-1", "\"Third\"", "null", "{}"] {
        assert!(serde_json::from_str::<ChildId>(invalid).is_err());
    }
}

fn named_creation() -> Plan {
    Plan {
        version: 2,
        network: Network::Mainnet,
        controller: crate::tests::input().controller,
        artifact: artifact::admit(WASM).unwrap(),
        action: PlanAction::Create {
            bootstrap: None,
            cycles: 1_000_000_000_000,
        },
        created_at_ns: 100,
        expires_at_ns: 600_000_000_100,
        deployment: Some(crate::DeploymentName {
            app: "demo".into(),
            fleet: "mainnet".into(),
            canister: "app".into(),
        }),
    }
}

fn publication() -> Plan {
    Plan {
        version: 3,
        action: PlanAction::Publish(crate::Publication {
            after_removal: None,
            root: UpgradeTarget {
                canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
                module: "01".repeat(32),
            },
            store: UpgradeTarget {
                canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
                module: "02".repeat(32),
            },
            release: ic_wasm_store_protocol::Release {
                name: "app".into(),
                version: "1".into(),
            },
        }),
        ..named_creation()
    }
}

#[test]
fn replanning_refuses_conflicting_and_duplicate_reservations_in_either_directory_order() {
    for kind in 0..5 {
        let mut first = if kind >= 3 {
            publication()
        } else {
            named_creation()
        };
        if kind == 2 {
            first.action = PlanAction::CreateChild(ManagedOwner {
                arguments: None,
                blob_attempt: None,
                root: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
                root_module: "01".repeat(32),
                execute_at_ns: 100,
                slot: None,
                source: None,
            });
        }
        let mut second = first.clone();
        second.created_at_ns += 1;
        match kind {
            0 => second.controller = "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
            1 | 4 => {} // Same apparent inputs, two distinct admitted approvals.
            2 => {
                second.deployment.as_mut().unwrap().canister = "other".into();
                let PlanAction::CreateChild(owner) = &mut second.action else {
                    unreachable!()
                };
                owner.slot = Some(canic_management_protocol::children::ChildId::FIRST);
                owner.execute_at_ns += 1;
                second.expires_at_ns += 1;
            }
            _ => {
                let PlanAction::Publish(binding) = &mut second.action else {
                    unreachable!()
                };
                binding.root.module = "03".repeat(32);
            }
        }
        let dir = crate::tests::directory();
        let journal = Store::open(&dir).unwrap();
        // Model conflicting retained custody; ordinary planning must never produce this pair.
        let ids =
            [&first, &second].map(|plan| publish_locked(&journal, plan.clone(), WASM).unwrap().0);
        let before = ids
            .each_ref()
            .map(|id| std::fs::read(dir.join(id).join("plan.json")).unwrap());
        // Whichever record the filesystem visits first, one request matches it exactly.
        for request in [first, second] {
            assert!(
                matches!(
                    retain_locked(&journal, request, WASM),
                    Err(Error::Blocked(_))
                ),
                "case {kind}"
            );
        }
        for (id, bytes) in ids.iter().zip(before) {
            assert_eq!(
                std::fs::read(dir.join(id).join("plan.json")).unwrap(),
                bytes
            );
            assert_eq!(
                std::fs::read(dir.join(id).join("module.wasm")).unwrap(),
                WASM
            );
        }
    }
}

#[test]
fn replanning_checks_later_corruption_and_inventory_bound_before_reusing_a_match() {
    let dir = crate::tests::directory();
    let journal = Store::open(&dir).unwrap();
    let first = named_creation();
    let mut other = first.clone();
    other.deployment.as_mut().unwrap().canister = "other".into();
    let ids = [first, other].map(|plan| publish_locked(&journal, plan, WASM).unwrap().0);
    let ordered: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|id| ids.contains(id))
        .collect();
    let retained = load(&dir.join(&ordered[0]), &ordered[0]).unwrap().0;
    let corrupt = dir.join(&ordered[1]).join("plan.json");
    let original = std::fs::read(&corrupt).unwrap();
    std::fs::write(&corrupt, b"invalid journal").unwrap();
    assert!(retain_locked(&journal, retained.clone(), WASM).is_err());
    std::fs::write(corrupt, original).unwrap();
    // A valid unrelated operation does not prevent exact reuse, even after expiry.
    assert_eq!(
        retain_locked(&journal, retained.clone(), WASM).unwrap(),
        (ordered[0].clone(), retained)
    );
    for index in 0..127 {
        let mut plan = named_creation();
        plan.deployment.as_mut().unwrap().canister = format!("extra-{index}");
        publish_locked(&journal, plan, WASM).unwrap();
    }
    // Select the first matching record so an early-return implementation cannot hide the bound.
    let first = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap())
        .find(|entry| entry.file_name().len() == 64)
        .unwrap();
    let request: Plan = store::json(&first.path().join("plan.json")).unwrap();
    assert!(
        retain_locked(&journal, request, WASM)
            .unwrap_err()
            .to_string()
            .contains("exceeds 128")
    );
}

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

#[test]
fn historical_scan_json_keeps_approval_fields_while_entries_are_explicit() {
    use canic_management_protocol::store_maintenance::Scan;
    let historical = serde_json::json!({
        "catalog": {"quota": {"reserved_bytes": 8, "releases": 2},
            "reserved_bytes": 8, "releases": 1, "artifacts": 1, "chunks": 1},
        "after": {"name": "app", "version": "v1"},
        "observed": 1, "digest": ([7; 32]), "complete": true
    });
    let scan: Scan = serde_json::from_value(historical.clone()).unwrap();
    assert!(matches!(scan, Scan::Legacy(_)));
    assert_eq!(serde_json::to_value(&scan).unwrap(), historical);
    assert_eq!(serde_json::to_string(&Scan::Legacy(None)).unwrap(), "null");
    let entries = Scan::Entries(scan.as_ref().cloned());
    let value = serde_json::to_value(&entries).unwrap();
    assert_eq!(value["entries"]["after_removed"], false);
    assert_eq!(serde_json::from_value::<Scan>(value).unwrap(), entries);
    for invalid in [
        serde_json::json!({"entries": null, "unknown": true}),
        serde_json::json!({"version": 3}),
    ] {
        assert!(serde_json::from_value::<Scan>(invalid).is_err());
    }
}
