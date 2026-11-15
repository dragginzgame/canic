use super::*;
use crate::{
    CanisterConfig, FleetConfig, Network,
    tests::{directory, input},
};
use std::collections::BTreeMap;

fn fixture() -> (std::path::PathBuf, SelectedFleet) {
    let dir = directory();
    for file in ["root.wasm", "app.wasm"] {
        std::fs::write(dir.join(file), b"\0asm\x01\0\0\0").unwrap();
    }
    let selected = SelectedFleet {
        app: "demo".into(),
        fleet: "local".into(),
        config: FleetConfig {
            network: Network::Mainnet,
            journal: None,
            canisters: BTreeMap::from([
                (
                    "root".into(),
                    CanisterConfig {
                        role: Some("root".into()),
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
        },
    };
    (dir, selected)
}
fn create(dir: &Path, selected: &SelectedFleet) -> Result<(String, FleetPlan)> {
    let result = plan_fleet(
        &dir.join("journal"),
        &dir.join("canic.json"),
        selected,
        &input().controller,
        Some(3_000_000_000_000),
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
fn stored_fleet_admits_full_manifests_and_keeps_retained_inline_delivery() {
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
        &dir.join("canic.json"),
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
            .all(|c| c.release.as_ref().unwrap().chunks.len()
                == ic_wasm_store_protocol::MAX_CHUNKS)
    );
    let mut wrong = fleet_plan.clone();
    wrong.children[0].release.as_mut().unwrap().hash[0] ^= 1;
    assert!(wrong.digest().is_err());
    wrong = fleet_plan;
    wrong.children[0].release.as_mut().unwrap().release.name = "another-app".into();
    assert!(wrong.digest().is_err());
    cancel_fleet_plan(&dir.join("journal"), &operation).unwrap();

    // Model a retained 0.206 Store + inline-child approval. Replanning cannot silently migrate it.
    std::fs::write(dir.join("app.wasm"), b"\0asm\x01\0\0\0").unwrap();
    let FleetPlanning::Create {
        operation,
        mut fleet_plan,
    } = plan_fleet(
        &dir.join("journal"),
        &dir.join("canic.json"),
        &selected,
        &input().controller,
        None,
        None,
    )
    .unwrap()
    else {
        panic!("creation");
    };
    for child in &mut fleet_plan.children {
        child.release = None;
    }
    let old_id = fleet_plan.digest().unwrap();
    let path = dir.join("journal").join(&operation);
    std::fs::write(
        path.join("fleet.json"),
        serde_json::to_vec(&fleet_plan).unwrap(),
    )
    .unwrap();
    std::fs::rename(path, dir.join("journal").join(&old_id)).unwrap();
    assert_eq!(
        plan_fleet(
            &dir.join("journal"),
            &dir.join("canic.json"),
            &selected,
            &input().controller,
            None,
            None
        )
        .unwrap(),
        FleetPlanning::Create {
            operation: old_id,
            fleet_plan
        }
    );
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
            &dir.join("canic.json"),
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
    let store = value.store.as_ref().unwrap();
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
    assert!(!permits_upgrade(&path, &value, &value.root).unwrap());
    assert!(value.child_plan(&receipt, None, Slot::First).is_err());
    let store_receipt = Receipt {
        operation: resolved.digest().unwrap(),
        canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
        module_hash: resolved.artifact.sha256,
        controller: receipt.canister.clone(),
    };
    let child = value
        .child_plan(&receipt, Some(&store_receipt), Slot::First)
        .unwrap();
    assert_eq!(
        child
            .management()
            .unwrap()
            .source
            .as_ref()
            .unwrap()
            .manifest
            .release
            .version,
        child.artifact.sha256
    );
    let mut wrong = store_receipt;
    wrong.operation = "00".repeat(32);
    assert!(
        value
            .child_plan(&receipt, Some(&wrong), Slot::First)
            .is_err()
    );
    assert!(path.join("store/module.wasm").exists());
    assert!(!path.join("store/plan.json").exists());
    let mut invalid = value.clone();
    invalid.store.as_mut().unwrap().parent.operation = "00".repeat(32);
    assert!(invalid.digest().is_err());
    // Altering/removing infrastructure cannot be treated as an application upgrade.
    let mut changed = selected.clone();
    changed.config.canisters.remove("store");
    assert!(matches!(create(&dir, &changed), Err(Error::Blocked(_))));
    std::fs::write(dir.join("store.wasm"), b"\0asm\x01\0\0\0\0\x01\0").unwrap();
    changed = selected.clone();
    changed.config.canisters.get_mut("store").unwrap().wasm = Some("store.wasm".into());
    assert!(matches!(
        plan_fleet(
            &dir.join("journal"),
            &dir.join("canic.json"),
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
    use canic_management_protocol::{LIMITS, Phase, STATUS, Status};
    use ic_platform::{candid_codec, cycles_ledger_types as ledger, principal::Principal};
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
    let result: ledger::CreateCanisterResult = Ok(ledger::CreateCanisterSuccess {
        block_id: 1_u64.into(),
        canister_id: Principal::from_text(&root.canister).unwrap(),
    });
    store::put(
        &path.join("root/create/reply.bin"),
        &candid_codec::encode_one(&result, 4096).unwrap(),
    )
    .unwrap();
    store::put_json(&path.join("root/receipt.json"), &root).unwrap();
    let child = fleet.child_plan(&root, None, Slot::First).unwrap();
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
        worker: Some(receipt.canister.clone()),
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
            &dir.join("canic.json"),
            &selected,
            &input().controller,
            None,
            dispatch,
        )
    };
    assert_eq!(
        planning(None).unwrap(),
        FleetPlanning::Unchanged {
            operation: id.clone()
        }
    );
    std::fs::write(dir.join("app.wasm"), b"\0asm\x01\0\0\0\0\x02\x01x").unwrap();
    let planned = planning(None).unwrap();
    let FleetPlanning::Upgrade { operation, plan } = &planned else {
        panic!("expected upgrade")
    };
    assert_eq!(plan.target(), Some(&receipt.canister));
    assert_eq!(plan.previous_module(), Some(&receipt.module_hash));
    assert_eq!(plan.creation_cycles(), 0);
    assert_eq!(planning(Some(1)).unwrap(), planned);
    // An upgrade receipt without Ready/version evidence also cannot become no-change.
    let upgraded = Receipt {
        operation: operation.clone(),
        module_hash: plan.artifact.sha256.clone(),
        ..receipt
    };
    store::put_json(&journal.join(operation).join("receipt.json"), &upgraded).unwrap();
    assert!(planning(None).is_err());
    std::fs::remove_file(journal.join(operation).join("receipt.json")).unwrap();
    assert_eq!(planning(None).unwrap(), planned);
    // The stale creation artifact must not hide the pending upgrade.
    std::fs::write(dir.join("app.wasm"), b"\0asm\x01\0\0\0").unwrap();
    assert!(
        planning(None)
            .unwrap_err()
            .to_string()
            .contains("unfinished")
    );
    // A different child target cannot bypass the fleet reservation.
    let mut foreign = plan.clone();
    let crate::PlanAction::UpgradeChild { target, .. } = &mut foreign.action else {
        panic!("child upgrade")
    };
    target.canister = "r7inp-6aaaa-aaaaa-aaabq-cai".into();
    assert!(!permits_upgrade(&path, &fleet, &foreign).unwrap());
    assert_eq!(load(&path, &id).unwrap(), fleet);
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
            &dir.join("canic.json"),
            &selected,
            &input().controller,
            Some(3_000_000_000_000),
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
    let child = value.child_plan(&receipt, None, Slot::First).unwrap();
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
        let id = value.digest().unwrap();
        let path = journal.join(&id);
        for leaf in ["root", "child"] {
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
        assert!(error.to_string().contains("retained child admission"));
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
fn existing_fleet_approval_keeps_its_original_digest() {
    // Public approval from the retained 0.204.1 demo; contains no signed envelopes.
    let fleet: FleetPlan = serde_json::from_str(include_str!("fixtures/v1.json")).unwrap();
    assert_eq!(
        fleet.digest().unwrap(),
        "a92356c7cfbeae5a2b380666093d7eae551f09f417e4264a0319729a76e9a1ab"
    );
    assert_eq!(
        fleet.root.digest().unwrap(),
        fleet.children[0].parent.operation
    );
    assert!(
        !serde_json::to_value(&fleet)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("sibling")
    );
}

#[test]
fn two_child_approval_retains_slot_mapping_and_cancels_only_before_preparation() {
    let (dir, mut selected) = fixture();
    let mut sibling = selected.config.canisters["app"].clone();
    sibling.wasm = Some("app.wasm".into());
    selected.config.canisters.insert("worker".into(), sibling);
    let (id, fleet) = create(&dir, &selected).unwrap();
    assert_eq!(fleet.version, 2);
    assert_eq!(fleet.children[0].deployment.canister, "app");
    assert_eq!(fleet.children[1].deployment.canister, "worker");
    assert_eq!(create(&dir, &selected).unwrap().0, id);
    let root = Receipt {
        operation: fleet.root.digest().unwrap(),
        canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        module_hash: fleet.root.artifact.sha256.clone(),
        controller: fleet.root.controller.clone(),
    };
    let first = fleet.child_plan(&root, None, Slot::First).unwrap();
    let second = fleet.child_plan(&root, None, Slot::Second).unwrap();
    assert_eq!(
        first.management().unwrap().slot,
        Some(canic_management_protocol::children::Slot::First)
    );
    assert_eq!(
        second.management().unwrap().slot,
        Some(canic_management_protocol::children::Slot::Second)
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
fn retained_two_child_approval_preserves_digest_and_rejects_slot_reassignment() {
    let fleet: FleetPlan = serde_json::from_str(include_str!("fixtures/v2.json")).unwrap();
    assert_eq!(
        fleet.digest().unwrap(),
        "50b59c920abc1f3b3986e423623a38c2a4fe37ae0b77c52c77ad518e736a476f"
    );
    let encoded = serde_json::to_vec(&fleet).unwrap();
    assert_eq!(
        serde_json::from_slice::<FleetPlan>(&encoded).unwrap(),
        fleet
    );
    let mut changed = fleet.clone();
    changed.children.swap(0, 1);
    assert!(changed.digest().is_err());
    let mut changed = fleet.clone();
    changed.children[1].slot = Slot::First;
    assert!(changed.digest().is_err());
    let mut changed = fleet.clone();
    changed.children[1].deployment = changed.children[0].deployment.clone();
    assert!(changed.digest().is_err());
    let mut changed = fleet;
    changed.children.clear();
    assert!(changed.digest().is_err());
}
