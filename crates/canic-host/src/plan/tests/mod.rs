use super::*;

const WASM: &[u8] = b"\0asm\x01\0\0\0";

#[test]
fn child_identity_refuses_retired_json_names() {
    use canic_management_protocol::children::ChildId;
    for (identity, encoded) in [
        (ChildId::FIRST, r#"{"service":0}"#),
        (ChildId::SECOND, r#"{"service":1}"#),
    ] {
        assert_eq!(serde_json::to_string(&identity).unwrap(), encoded);
        assert_eq!(serde_json::from_str::<ChildId>(encoded).unwrap(), identity);
    }
    for invalid in [
        "\"First\"",
        "\"Second\"",
        "0",
        "1",
        "2",
        "-1",
        "\"Third\"",
        "null",
        "{}",
    ] {
        assert!(serde_json::from_str::<ChildId>(invalid).is_err());
    }
}

fn named_creation() -> Plan {
    Plan {
        version: 1,
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
        version: 1,
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
                child_evidence_only: false,
                arguments: None,
                blob_attempt: None,
                root: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
                root_module: "01".repeat(32),
                execute_at_ns: 100,
                slot: canic_management_protocol::children::ChildId::FIRST,
                source: crate::tests::stored_source("app", b"\0asm\x01\0\0\0"),
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
                owner.slot = canic_management_protocol::children::ChildId::FIRST;
                owner.source.manifest.release.name = "other".into();
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
fn untagged_host_approvals_are_refused_even_at_version_one() {
    let fixtures: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/retained.json")).unwrap();
    for fixture in fixtures {
        let mut old = fixture["plan"].clone();
        assert!(serde_json::from_value::<Plan>(old.clone()).is_err());
        old["version"] = 1.into();
        assert!(serde_json::from_value::<Plan>(old).is_err());
    }
}

#[tokio::test]
async fn a_superseded_or_substituted_contract_cannot_start_an_attempt() {
    let directory = crate::tests::directory();
    let plan = publication();
    let id = {
        let journal = Store::open(&directory).unwrap();
        publish_locked(&journal, plan.clone(), WASM).unwrap().0
    };
    let path = directory.join(&id);
    for (version, contract) in [
        (3, Some("canic:artifact-publication.v1")),
        (1, None),
        (1, Some("canic:artifact-removal.v1")),
    ] {
        let mut record = serde_json::to_value(&plan).unwrap();
        record["version"] = version.into();
        match contract {
            Some(name) => record["action"]["contract"] = name.into(),
            None => {
                record["action"].as_object_mut().unwrap().remove("contract");
            }
        }
        let bytes = serde_json::to_vec(&record).unwrap();
        std::fs::write(path.join("plan.json"), &bytes).unwrap();
        assert!(serde_json::from_slice::<Plan>(&bytes).is_err());
        assert!(
            crate::apply(&directory, &id, "anonymous", Path::new("unused"))
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_dir(&path).unwrap().count(), 2);
    }
}

#[test]
fn flat_named_v1_approval_requires_its_original_host() {
    let mut record = serde_json::to_value(named_creation()).unwrap();
    let action = record.as_object_mut().unwrap().remove("action").unwrap();
    record["contract"] = action["contract"].clone();
    record["creation_cycles"] = action["approval"]["cycles"].clone();
    record["target"] = serde_json::Value::Null;
    record["previous_module"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<Plan>(record).is_err());
}

#[test]
fn ready_admission_ends_at_original_expiry_and_scheduled_admission_stays_earlier() {
    assert_eq!(admission_deadline(100, 100, 200), 200);
    assert_eq!(admission_deadline(100, 150, 200), 150);
}

#[test]
fn incomplete_or_conflicting_record_authority_is_rejected_at_decode() {
    let mut plan = named_creation();
    plan.action = PlanAction::CreateChild(ManagedOwner {
        slot: canic_management_protocol::children::ChildId::FIRST,
        source: crate::tests::stored_source("app", WASM),
        root: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        root_module: "01".repeat(32),
        execute_at_ns: 100,
        arguments: None,
        blob_attempt: None,
        child_evidence_only: false,
    });
    let original = serde_json::to_value(&plan).unwrap();
    assert_eq!(
        serde_json::from_value::<Plan>(original.clone()).unwrap(),
        plan
    );
    for field in ["slot", "source"] {
        let mut record = original.clone();
        record["action"]["approval"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(serde_json::from_value::<Plan>(record).is_err());
    }
    let mut record = original;
    record["action"]["approval"]["target"] = "aaaaa-aa".into();
    assert!(serde_json::from_value::<Plan>(record).is_err());
}

#[test]
fn scan_json_refuses_retired_approval_fields() {
    use canic_management_protocol::store_maintenance::Preservation;
    let historical = serde_json::json!({
        "catalog": {"quota": {"reserved_bytes": 8, "releases": 2},
            "reserved_bytes": 8, "releases": 1, "artifacts": 1, "chunks": 1},
        "after": {"name": "app", "version": "v1"},
        "observed": 1, "digest": ([7; 32]), "complete": true
    });
    assert!(serde_json::from_value::<Option<Preservation>>(historical).is_err());
    let entries = None::<Preservation>;
    let value = serde_json::to_value(&entries).unwrap();
    assert!(value.is_null());
    assert_eq!(
        serde_json::from_value::<Option<Preservation>>(value).unwrap(),
        entries
    );
    for invalid in [
        serde_json::json!({"entries": null, "unknown": true}),
        serde_json::json!({"version": 3}),
    ] {
        assert!(serde_json::from_value::<Option<Preservation>>(invalid).is_err());
    }
}

#[tokio::test]
async fn direct_approvals_cannot_start_standalone_effects() {
    for mut legacy in [
        named_creation(),
        Plan {
            version: 1,
            deployment: None,
            ..named_creation()
        },
    ] {
        // Keep the approval current so expiration cannot account for the refusal.
        legacy.created_at_ns = crate::now().unwrap();
        legacy.expires_at_ns = legacy.created_at_ns + 60_000_000_000;
        let directory = crate::tests::directory();
        let id = {
            let journal = Store::open(&directory).unwrap();
            publish_locked(&journal, legacy.clone(), WASM).unwrap().0
        };
        let path = directory.join(&id);
        assert_eq!(load(&path, &id).unwrap().0, legacy);
        assert!(
            matches!(crate::apply(&directory, &id, "anonymous", Path::new("unused")).await,
            Err(Error::Blocked(message)) if message.contains("configured Root bootstrap"))
        );
        assert!(!path.join("create").exists());
    }
}
