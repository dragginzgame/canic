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
    create_fleet_plan(
        &dir.join("journal"),
        &dir.join("canic.json"),
        selected,
        &input().controller,
        3_000_000_000_000,
        None,
    )
}

#[test]
fn fleet_authority_is_retained_before_outputs_and_replanning_never_renews_it() {
    let (dir, selected) = fixture();
    let (id, value) = create(&dir, &selected).unwrap();
    let journal = dir.join("journal");
    let original = store::read(&journal.join(&id).join("fleet.json"), store::MAX_RECORD).unwrap();
    assert_eq!(
        create(&dir, &selected).unwrap(),
        (id.clone(), value.clone())
    );
    // A requested expired schedule must not replace original authority.
    assert_eq!(
        create_fleet_plan(
            &journal,
            &dir.join("canic.json"),
            &selected,
            &input().controller,
            3_000_000_000_000,
            Some(1)
        )
        .unwrap()
        .0,
        id
    );
    assert_eq!(
        original,
        store::read(&journal.join(&id).join("fleet.json"), store::MAX_RECORD).unwrap()
    );
    let state = crate::status(&journal, &id).unwrap();
    assert!(state["child"].is_null());
    assert!(state["root"]["receipt"].is_null());
    assert_eq!(value.child.parent.operation, value.root.digest().unwrap());
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
    assert_eq!(crate::status(&journal, &id).unwrap(), state);
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
    changed.child.parent.operation = "00".repeat(32);
    assert!(changed.digest().is_err());
    let receipt = Receipt {
        operation: value.child.parent.operation.clone(),
        canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        module_hash: value.root.artifact.sha256.clone(),
        controller: value.root.controller.clone(),
    };
    let child = value.child_plan(&receipt).unwrap();
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
    let (dir, selected) = fixture();
    let (original_id, mut value) = create(&dir, &selected).unwrap();
    let journal = dir.join("journal");
    value.root.created_at_ns = 1;
    value.child.execute_at_ns = 2;
    value.root.expires_at_ns = 600_000_000_002;
    value.child.parent.operation = value.root.digest().unwrap();
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
    assert!(crate::status(&journal, &id).unwrap()["cancellation"].is_object());
    let failure = crate::apply(&journal, &id, "unused", Path::new("must-not-run"))
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
