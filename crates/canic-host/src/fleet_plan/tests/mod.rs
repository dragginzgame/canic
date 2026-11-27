use super::*;
use crate::{
    CanisterConfig, Network,
    tests::{directory, input},
};
use std::collections::BTreeMap;

fn fixture() -> (std::path::PathBuf, SelectedFleet) {
    let dir = directory();
    for file in ["root.wasm", "app.wasm"] {
        std::fs::write(dir.join(file), b"\0asm\x01\0\0\0").unwrap();
    }
    let selected = crate::tests::selected_fleet(
        "demo",
        "local",
        Network::Mainnet,
        BTreeMap::from([
            (
                "root".into(),
                CanisterConfig {
                    role: Some("root".into()),
                    wasm: Some("root.wasm".into()),
                    ..Default::default()
                },
            ),
            (
                "store".into(),
                CanisterConfig {
                    role: Some("wasm-store".into()),
                    parent: Some("root".into()),
                    wasm: Some("root.wasm".into()),
                    ..Default::default()
                },
            ),
            (
                "app".into(),
                CanisterConfig {
                    parent: Some("root".into()),
                    wasm: Some("app.wasm".into()),
                    ..Default::default()
                },
            ),
        ]),
    );
    (dir, selected)
}

fn store_output(fleet: &FleetPlan, root: &Receipt) -> Receipt {
    let plan = fleet.store_plan(root).unwrap();
    Receipt {
        operation: plan.digest().unwrap(),
        module_hash: plan.artifact.sha256,
        controller: root.canister.clone(),
        canister: "r7inp-6aaaa-aaaaa-aaabq-cai".into(),
    }
}

fn create(dir: &Path, selected: &SelectedFleet) -> Result<(String, FleetPlan)> {
    let result = plan_fleet(
        &dir.join("journal"),
        &dir.join("canic.toml"),
        selected,
        &input().controller,
        Some(5_000_000_000_000),
        None,
    )?;
    match result {
        FleetPlanning::Create {
            operation,
            fleet_plan,
        } => Ok((operation, fleet_plan)),
        _ => panic!("expected creation"),
    }
}

#[test]
fn stored_fleet_admits_full_manifests_and_rejects_retained_inline_delivery() {
    let (dir, mut selected) = fixture();
    selected.config.canisters.insert(
        "store".into(),
        CanisterConfig {
            role: Some("wasm-store".into()),
            parent: Some("root".into()),
            wasm: Some("root.wasm".into()),
            ..Default::default()
        },
    );
    selected
        .config
        .canisters
        .insert("worker".into(), selected.config.canisters["app"].clone());
    let mut bytes = b"\0asm\x01\0\0\0".to_vec();
    bytes.resize(artifact::MAX_ARTIFACT_BYTES, 42);
    std::fs::write(dir.join("app.wasm"), &bytes).unwrap();
    let result = plan_fleet(
        &dir.join("journal"),
        &dir.join("canic.toml"),
        &selected,
        &input().controller,
        None,
        None,
    )
    .unwrap();
    let FleetPlanning::Create {
        operation,
        fleet_plan,
    } = result
    else {
        panic!("creation");
    };
    assert_eq!(
        load(&dir.join("journal").join(&operation), &operation).unwrap(),
        fleet_plan
    );
    assert!(
        fleet_plan
            .children
            .iter()
            .all(|c| c.release.chunks.len() == ic_wasm_store_protocol::MAX_CHUNKS)
    );
    let record = serde_json::to_value(&fleet_plan).unwrap();
    let mut wrong = fleet_plan.clone();
    wrong.children[0].release.hash[0] ^= 1;
    assert!(wrong.digest().is_err());
    wrong = fleet_plan;
    wrong.children[0].release.release.name = "another-app".into();
    assert!(wrong.digest().is_err());
    cancel_fleet_plan(&dir.join("journal"), &operation).unwrap();

    for key in ["store", "release"] {
        let mut invalid = record.clone();
        let object = if key == "store" {
            &mut invalid
        } else {
            &mut invalid["children"][0]
        };
        object.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<FleetPlan>(invalid).is_err());
    }
}

#[test]
fn fleet_store_is_an_independent_output_binding_and_debit() {
    let (dir, mut selected) = fixture();
    selected.config.canisters.insert(
        "store".into(),
        CanisterConfig {
            role: Some("wasm-store".into()),
            parent: Some("root".into()),
            wasm: Some("root.wasm".into()),
            ..Default::default()
        },
    );
    selected
        .config
        .canisters
        .insert("worker".into(), selected.config.canisters["app"].clone());
    let plan = || {
        plan_fleet(
            &dir.join("journal"),
            &dir.join("canic.toml"),
            &selected,
            &input().controller,
            None,
            None,
        )
    };
    let FleetPlanning::Create {
        operation: id,
        fleet_plan: value,
    } = plan().unwrap()
    else {
        panic!("creation");
    };
    assert_eq!(value.children.len(), 2);
    assert_eq!(value.creation_cycles(), 5_000_000_000_000);
    assert_eq!(value.root.creation_cycles(), 4_000_000_000_000);
    let store = &value.store;
    assert_eq!(store.parent.operation, value.root.digest().unwrap());
    assert_eq!(
        plan().unwrap(),
        FleetPlanning::Create {
            operation: id.clone(),
            fleet_plan: value.clone()
        }
    );
    let receipt = Receipt {
        operation: value.root.digest().unwrap(),
        canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        module_hash: value.root.artifact.sha256.clone(),
        controller: value.root.controller.clone(),
    };
    let resolved = value.store_plan(&receipt).unwrap();
    assert!(resolved.management().is_none());
    assert_eq!(resolved.application_controller(), receipt.canister);
    assert_eq!(resolved.creation_cycles(), 1_000_000_000_000);
    let path = dir.join("journal").join(&id);
    assert!(
        permits_upgrade(
            &Store::open(&dir.join("journal")).unwrap(),
            &id,
            &value,
            &mut value.root.clone()
        )
        .unwrap()
        .is_none()
    );
    assert!(value.child_plan(&receipt, None, ChildId::FIRST).is_err());
    let store_receipt = Receipt {
        operation: resolved.digest().unwrap(),
        canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
        module_hash: resolved.artifact.sha256,
        controller: receipt.canister.clone(),
    };
    let child = value
        .child_plan(&receipt, Some(&store_receipt), ChildId::FIRST)
        .unwrap();
    assert_eq!(
        child.management().unwrap().source.manifest.release.version,
        child.artifact.sha256
    );
    let mut wrong = store_receipt;
    wrong.operation = "00".repeat(32);
    assert!(
        value
            .child_plan(&receipt, Some(&wrong), ChildId::FIRST)
            .is_err()
    );
    assert!(path.join("store/module.wasm").exists());
    assert!(!path.join("store/plan.json").exists());
    let mut invalid = value.clone();
    invalid.store.parent.operation = "00".repeat(32);
    assert!(invalid.digest().is_err());
    // Altering/removing infrastructure cannot be treated as an application upgrade.
    let mut changed = selected.clone();
    changed.config.canisters.remove("store");
    assert!(create(&dir, &changed).is_err());
    std::fs::write(dir.join("store.wasm"), b"\0asm\x01\0\0\0\0\x01\0").unwrap();
    changed = selected.clone();
    changed.config.canisters.get_mut("store").unwrap().wasm = Some("store.wasm".into());
    assert!(matches!(
        plan_fleet(
            &dir.join("journal"),
            &dir.join("canic.toml"),
            &changed,
            &input().controller,
            None,
            None
        ),
        Err(Error::Blocked(_))
    ));
    let prepared = path.join("store/create");
    std::fs::create_dir(&prepared).unwrap();
    assert!(cancel_fleet_plan(&dir.join("journal"), &id).is_err());
    std::fs::remove_dir(prepared).unwrap();
    cancel_fleet_plan(&dir.join("journal"), &id).unwrap();
    assert!(cancelled(&path, &id).unwrap());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn fleet_upgrade_requires_completion_and_never_reopens_or_substitutes_authority() {
    use canic_management_protocol::{LIMITS, Phase, Status, children::STATUS};
    let (dir, selected) = fixture();
    let (id, fleet) = create(&dir, &selected).unwrap();
    let journal = dir.join("journal");
    let path = journal.join(&id);
    let root = Receipt {
        operation: fleet.children[0].parent.operation.clone(),
        canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        module_hash: fleet.root.artifact.sha256.clone(),
        controller: fleet.root.controller.clone(),
    };
    std::fs::create_dir(path.join("root/create")).unwrap();
    store::put(
        &path.join("root/create/reply.bin"),
        &crate::tests::wire_fixture("created"),
    )
    .unwrap();
    store::put_json(&path.join("root/receipt.json"), &root).unwrap();
    let child = fleet
        .child_plan(&root, Some(&store_output(&fleet, &root)), ChildId::FIRST)
        .unwrap();
    store::put_json(&path.join("child/plan.json"), &child).unwrap();
    let receipt = Receipt {
        operation: child.digest().unwrap(),
        canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
        module_hash: child.artifact.sha256.clone(),
        controller: root.canister,
    };
    store::put_json(&path.join("child/receipt.json"), &receipt).unwrap();
    // A matching receipt without authenticated Ready evidence cannot authorize an upgrade.
    assert!(create(&dir, &selected).is_err());
    let ready = Status {
        approval: Some(crate::managed::approval(&child).unwrap()),
        phase: Phase::Ready,
        worker: Some(receipt.canister),
        observed_module: Some(crate::unhex(&child.artifact.sha256).unwrap()),
        attempts: [1; 3],
        ..Default::default()
    };
    std::fs::create_dir_all(path.join("child/observe/0")).unwrap();
    store::put(
        &path.join("child/observe/0/reply.bin"),
        &ic_wire::encode(
            &ready,
            ic_wire::Encoding::Frame {
                method: STATUS,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap(),
    )
    .unwrap();
    let planning = |dispatch| {
        plan_fleet(
            &journal,
            &dir.join("canic.toml"),
            &selected,
            &input().controller,
            None,
            dispatch,
        )
    };
    // A fabricated inline Ready reply cannot substitute for Store and publication custody.
    assert!(planning(None).is_err());
    assert!(!path.join("store/receipt.json").exists());
}

#[test]
fn fleet_authority_is_retained_before_outputs_and_replanning_never_renews_it() {
    let (dir, selected) = fixture();
    let (id, value) = create(&dir, &selected).unwrap();
    assert_eq!(value.children[0].execute_at_ns, value.root.created_at_ns);
    assert_eq!(
        value.root.expires_at_ns - value.root.created_at_ns,
        600_000_000_000
    );
    let journal = dir.join("journal");
    let original = store::read(&journal.join(&id).join("fleet.json"), store::MAX_RECORD).unwrap();
    assert_eq!(
        create(&dir, &selected).unwrap(),
        (id.clone(), value.clone())
    );
    // A requested expired schedule must not replace original authority.
    assert_eq!(
        plan_fleet(
            &journal,
            &dir.join("canic.toml"),
            &selected,
            &input().controller,
            Some(5_000_000_000_000),
            Some(1)
        )
        .unwrap(),
        FleetPlanning::Create {
            operation: id.clone(),
            fleet_plan: value.clone()
        }
    );
    assert_eq!(
        original,
        store::read(&journal.join(&id).join("fleet.json"), store::MAX_RECORD).unwrap()
    );
    let state = serde_json::to_value(crate::status(&journal, &id).unwrap()).unwrap();
    assert!(state["children"][0]["state"].is_null());
    assert!(state["root"]["receipt"].is_null());
    assert_eq!(
        value.children[0].parent.operation,
        value.root.digest().unwrap()
    );
    assert!(!journal.join(&id).join("child/plan.json").exists());
    let inventory = crate::fleet::inventory(&journal, false, crate::now().unwrap()).unwrap();
    assert!(inventory.issues.is_empty());
    assert_eq!(inventory.records.len(), 1);
    assert_eq!(inventory.records[0].0.operation, id);
    assert!(inventory.records[0].0.canister.is_none());
    {
        let _writer = Store::open(&journal).unwrap();
        let busy = crate::fleet::inventory(&journal, false, crate::now().unwrap()).unwrap();
        assert!(busy.records.is_empty());
        assert_eq!(busy.issues.len(), 1);
        assert!(busy.issues[0].contains(&id));
    }
    let mut other = selected.clone();
    other.fleet = "other".into();
    assert!(other.check_operation(&journal, &id).is_err());
    assert!(
        crate::create_configured_plan(&journal, &dir.join("root.wasm"), input(), &selected, "root")
            .is_err()
    );
    let mut changed = selected.clone();
    changed.config.network = Network::Local {
        url: "http://localhost:8000".into(),
        root_key: "00".repeat(133),
        root_subnet: None,
        routing_canister: "aaaaa-aa".into(),
    };
    assert!(create(&dir, &changed).is_err());
    std::fs::write(dir.join("app.wasm"), b"\0asm\x01\0\0\0\0").unwrap();
    assert!(create(&dir, &selected).is_err());
    assert_eq!(
        serde_json::to_value(crate::status(&journal, &id).unwrap()).unwrap(),
        state
    );
}

#[test]
fn fleet_child_bound_is_explicit_before_artifact_reads_or_journal_creation() {
    for with_store in [false, true] {
        for count in [0, ChildId::HISTORICAL_SERVICES.len() + 1] {
            let (dir, mut selected) = fixture();
            selected.config.policy.subnet_child_limit = Some(8);
            let child = selected.config.canisters.remove("app").unwrap();
            for index in 0..count {
                selected
                    .config
                    .canisters
                    .insert(format!("app-{index}"), child.clone());
            }
            if with_store {
                selected.config.canisters.insert(
                    "store".into(),
                    CanisterConfig {
                        role: Some("wasm-store".into()),
                        parent: Some("root".into()),
                        wasm: Some("missing-store.wasm".into()),
                        ..Default::default()
                    },
                );
            }
            std::fs::remove_file(dir.join("app.wasm")).unwrap();
            std::fs::remove_file(dir.join("root.wasm")).unwrap();
            let Err(Error::Invalid(message)) = create(&dir, &selected) else {
                panic!("expected child capacity refusal");
            };
            assert_eq!(
                message,
                format!(
                    "fleet declares {count} direct application children under root; supported range is 1..={} (Wasm Store is separate)",
                    ChildId::HISTORICAL_SERVICES.len()
                )
            );
            assert!(!dir.join("journal").exists());
        }
    }
}

#[test]
fn fleet_refuses_unsupported_trees_conflicts_and_tampered_artifacts() {
    let (dir, selected) = fixture();
    let mut bad = selected.clone();
    bad.config
        .canisters
        .insert("extra".into(), CanisterConfig::default());
    assert!(create(&dir, &bad).is_err());
    assert!(!dir.join("journal").exists());
    bad = selected.clone();
    bad.config.canisters.get_mut("app").unwrap().id = Some("aaaaa-aa".into());
    assert!(create(&dir, &bad).is_err());
    let (id, value) = create(&dir, &selected).unwrap();
    let root = dir.join("journal").join(&id);
    let mut changed = value.clone();
    changed.children[0].parent.operation = "00".repeat(32);
    assert!(changed.digest().is_err());
    let receipt = Receipt {
        operation: value.children[0].parent.operation.clone(),
        canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        module_hash: value.root.artifact.sha256.clone(),
        controller: value.root.controller.clone(),
    };
    let child = value
        .child_plan(
            &receipt,
            Some(&store_output(&value, &receipt)),
            ChildId::FIRST,
        )
        .unwrap();
    store::put_json(&root.join("child/plan.json"), &child).unwrap();
    // An invented child plan without Root result custody is never admitted.
    assert!(load(&root, &id).is_err());
    std::fs::remove_file(root.join("child/plan.json")).unwrap();
    std::fs::write(root.join("child/module.wasm"), b"\0asm\x01\0\0\0\0").unwrap();
    assert!(crate::status(&dir.join("journal"), &id).is_err());
    assert!(
        !crate::fleet::inventory(&dir.join("journal"), false, crate::now().unwrap())
            .unwrap()
            .issues
            .is_empty()
    );
    assert!(create(&dir, &selected).is_err());
}

#[tokio::test]
async fn expired_unsubmitted_fleet_cannot_start_root_or_renew_its_schedule() {
    for dispatch in [1, 2] {
        let (dir, selected) = fixture();
        let (original_id, mut value) = create(&dir, &selected).unwrap();
        let journal = dir.join("journal");
        value.root.created_at_ns = 1;
        value.children[0].execute_at_ns = dispatch;
        value.root.expires_at_ns = 600_000_000_000 + dispatch;
        value.children[0].parent.operation = value.root.digest().unwrap();
        value.store.parent.operation = value.root.digest().unwrap();
        let id = value.digest().unwrap();
        let path = journal.join(&id);
        for leaf in ["root", "store", "child"] {
            std::fs::create_dir_all(path.join(leaf)).unwrap();
            std::fs::copy(
                journal.join(&original_id).join(leaf).join("module.wasm"),
                path.join(leaf).join("module.wasm"),
            )
            .unwrap();
        }
        store::put_json(&path.join("root/plan.json"), &value.root).unwrap();
        store::put_json(&path.join("fleet.json"), &value).unwrap();
        let error = crate::reconcile(&journal, &id, "unused", Path::new("must-not-run"))
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Root bootstrap has no retained receipt")
        );
        let error = crate::apply(&journal, &id, "unused", Path::new("must-not-run"))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("no Root creation was signed"));
        assert!(!path.join("root/create").exists());
        assert!(!path.join("child/plan.json").exists());
        cancel_fleet_plan(&journal, &original_id).unwrap();
        cancel_fleet_plan(&journal, &id).unwrap();
        assert_ne!(create(&dir, &selected).unwrap().0, id);
    }
}

#[tokio::test]
async fn cancellation_retains_evidence_and_permanently_revokes_only_unprepared_work() {
    let (dir, selected) = fixture();
    let (id, _) = create(&dir, &selected).unwrap();
    let journal = dir.join("journal");
    let path = journal.join(&id);
    let before = std::fs::read(path.join("fleet.json")).unwrap();
    {
        let _writer = Store::open(&journal).unwrap();
        assert!(cancel_fleet_plan(&journal, &id).is_err());
    }
    // Even an interrupted attempt without a completed signature remains reserved.
    std::fs::create_dir(path.join("root/create")).unwrap();
    assert!(cancel_fleet_plan(&journal, &id).is_err());
    assert!(!path.join("cancelled.json").exists());
    std::fs::remove_dir(path.join("root/create")).unwrap();
    let record = cancel_fleet_plan(&journal, &id).unwrap();
    assert_eq!(cancel_fleet_plan(&journal, &id).unwrap(), record);
    assert_eq!(before, std::fs::read(path.join("fleet.json")).unwrap());
    assert!(
        serde_json::to_value(crate::status(&journal, &id).unwrap()).unwrap()["cancellation"]
            .is_object()
    );
    let failure = crate::apply(&journal, &id, "unused", Path::new("must-not-run"))
        .await
        .unwrap_err();
    assert!(failure.to_string().contains("cancelled"));
    let failure = crate::reconcile(&journal, &id, "unused", Path::new("must-not-run"))
        .await
        .unwrap_err();
    assert!(failure.to_string().contains("cancelled"));
    let (replacement, _) = create(&dir, &selected).unwrap();
    assert_ne!(id, replacement);
    assert_eq!(create(&dir, &selected).unwrap().0, replacement);
    let inventory = crate::fleet::inventory(&journal, false, crate::now().unwrap()).unwrap();
    assert!(inventory.issues.is_empty());
    assert_eq!(inventory.records.len(), 1);
    assert_eq!(inventory.records[0].0.operation, replacement);
    // A forged revocation cannot hide an effect attempt from inventory or replanning.
    std::fs::create_dir(path.join("root/create")).unwrap();
    assert!(crate::status(&journal, &id).is_err());
    assert!(create(&dir, &selected).is_err());
}

#[test]
fn untagged_historical_fleet_approvals_are_refused() {
    for old in [
        include_str!("fixtures/v1.json"),
        include_str!("fixtures/v2.json"),
    ] {
        assert!(serde_json::from_str::<FleetPlan>(old).is_err());
    }
}

#[test]
fn two_child_approval_retains_slot_mapping_and_cancels_only_before_preparation() {
    let (dir, mut selected) = fixture();
    let mut sibling = selected.config.canisters["app"].clone();
    sibling.wasm = Some("app.wasm".into());
    selected.config.canisters.insert("worker".into(), sibling);
    let (id, fleet) = create(&dir, &selected).unwrap();
    assert_eq!(fleet.version, 1);
    assert_eq!(fleet.children[0].deployment.canister, "app");
    assert_eq!(fleet.children[1].deployment.canister, "worker");
    assert_eq!(create(&dir, &selected).unwrap().0, id);
    let root = Receipt {
        operation: fleet.root.digest().unwrap(),
        canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        module_hash: fleet.root.artifact.sha256.clone(),
        controller: fleet.root.controller.clone(),
    };
    let first = fleet
        .child_plan(&root, Some(&store_output(&fleet, &root)), ChildId::FIRST)
        .unwrap();
    let second = fleet
        .child_plan(&root, Some(&store_output(&fleet, &root)), ChildId::SECOND)
        .unwrap();
    assert_eq!(
        first.management().unwrap().slot,
        canic_management_protocol::children::ChildId::FIRST
    );
    assert_eq!(
        second.management().unwrap().slot,
        canic_management_protocol::children::ChildId::SECOND
    );
    assert_ne!(first.digest().unwrap(), second.digest().unwrap());
    let mut renamed = selected.clone();
    let definition = renamed.config.canisters.remove("worker").unwrap();
    renamed.config.canisters.insert("aaa".into(), definition);
    assert!(matches!(create(&dir, &renamed), Err(Error::Blocked(_))));
    let journal = dir.join("journal");
    let prepared = journal.join(&id).join("sibling/stage");
    std::fs::create_dir(&prepared).unwrap();
    assert!(matches!(
        cancel_fleet_plan(&journal, &id),
        Err(Error::Blocked(_))
    ));
    std::fs::remove_dir(prepared).unwrap();
    cancel_fleet_plan(&journal, &id).unwrap();
    assert!(cancelled(&journal.join(&id), &id).unwrap());
}

#[test]
fn two_child_approval_rejects_slot_reassignment() {
    let (dir, mut selected) = fixture();
    let sibling = selected.config.canisters["app"].clone();
    selected.config.canisters.insert("worker".into(), sibling);
    let (_, fleet) = create(&dir, &selected).unwrap();
    let encoded = serde_json::to_vec(&fleet).unwrap();
    assert_eq!(
        serde_json::from_slice::<FleetPlan>(&encoded).unwrap(),
        fleet
    );
    let mut changed = fleet.clone();
    changed.children.swap(0, 1);
    assert!(changed.digest().is_err());
    let mut changed = fleet.clone();
    changed.children[1].slot = ChildId::FIRST;
    assert!(changed.digest().is_err());
    let mut changed = fleet.clone();
    changed.children[1].deployment = changed.children[0].deployment.clone();
    assert!(changed.digest().is_err());
    let mut changed = fleet;
    changed.children.clear();
    assert!(changed.digest().is_err());
}

#[test]
fn whole_fleet_keeps_nine_service_approvals_and_journal_paths() {
    let (dir, mut selected) = fixture();
    let service = selected.config.canisters.remove("app").unwrap();
    for name in [
        "user_hub",
        "project_registry",
        "discovery_hub",
        "a",
        "b",
        "c",
        "d",
        "e",
        "f",
    ] {
        selected
            .config
            .canisters
            .insert(name.into(), service.clone());
    }
    let journal = dir.join("journal");
    let plan = || {
        plan_fleet(
            &journal,
            &dir.join("canic.toml"),
            &selected,
            &input().controller,
            None,
            None,
        )
        .unwrap()
    };
    let FleetPlanning::Create {
        operation,
        fleet_plan,
    } = plan()
    else {
        panic!("fresh fleet")
    };
    let retained = journal.join(&operation);
    assert_eq!(load(&retained, &operation).unwrap(), fleet_plan);
    assert_eq!(fleet_plan.children.len(), 9);
    let mut record = serde_json::to_value(&fleet_plan).unwrap();
    assert!(record.get("child").is_none());
    assert!(record.get("sibling").is_none());
    assert!(record.get("additional").is_none());
    assert_eq!(
        record["children"][2]["slot"],
        serde_json::json!(ChildId::HISTORICAL_SERVICES[2])
    );
    record["children"][2]["slot"] = serde_json::json!(ChildId::FIRST);
    assert!(serde_json::from_value::<FleetPlan>(record).is_err());
    for (index, child) in fleet_plan.children.iter().enumerate() {
        assert_eq!(child.slot, ChildId::service(index).unwrap());
        assert!(
            retained
                .join(record::child_leaf(child.slot))
                .join("module.wasm")
                .is_file()
        );
    }
    assert!(matches!(plan(), FleetPlanning::Create { operation: id, .. } if id == operation));
    // Evidence anywhere in the operation blocks cancellation, even when
    // this reader does not recognize the service directory.
    std::fs::create_dir_all(retained.join("child-9/stage")).unwrap();
    assert!(matches!(
        cancel_fleet_plan(&journal, &operation),
        Err(Error::Blocked(_))
    ));
}

#[test]
fn retired_catalog_record_version_is_refused() {
    let (dir, selected) = fixture();
    let (_, plan) = create(&dir, &selected).unwrap();
    let mut record = serde_json::json!({
        "version": 10,
        "root": plan.root,
        "children": plan.children,
        "store": plan.store,
    });
    assert!(serde_json::from_value::<FleetPlan>(record.clone()).is_err());
    record["children"][0]["backup_history"] = serde_json::json!({});
    assert!(serde_json::from_value::<FleetPlan>(record).is_err());
}
