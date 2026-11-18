use super::*;
use crate::{
    CanisterConfig, FleetConfig, Network,
    tests::{directory, input},
};
use std::collections::BTreeMap;

pub(super) fn selection() -> SelectedFleet {
    SelectedFleet {
        app: "starter".into(),
        fleet: "mainnet".into(),
        config: FleetConfig {
            shard_pool: None,
            auth: None,
            network: Network::Mainnet,
            journal: None,
            canisters: BTreeMap::from([
                (
                    "root".into(),
                    CanisterConfig {
                        id: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".into()),
                        role: Some("root".into()),
                        ..Default::default()
                    },
                ),
                (
                    "store".into(),
                    CanisterConfig {
                        id: Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into()),
                        role: Some("wasm-store".into()),
                        parent: Some("root".into()),
                        ..Default::default()
                    },
                ),
                (
                    "app".into(),
                    CanisterConfig {
                        parent: Some("root".into()),
                        ..Default::default()
                    },
                ),
            ]),
        },
    }
}
pub(super) fn publication(version: &str) -> PublicationInput {
    PublicationInput {
        canister: "app".into(),
        store: "store".into(),
        version: version.into(),
        root_module: "11".repeat(32),
        store_module: "22".repeat(32),
    }
}

#[test]
fn artifact_listing_preserves_unknown_outcomes_scope_and_corrupt_evidence() {
    let dir = directory();
    let selected = selection();
    let absent = dir.join("absent");
    assert!(
        crate::fleet_artifacts(&absent, &selected)
            .unwrap()
            .publications
            .is_empty()
    );
    assert!(!absent.exists());
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (second, _) = create_publication_plan(
        &dir,
        &wasm,
        &selected,
        &input().controller,
        publication("v2"),
    )
    .unwrap();
    let (first, plan) = create_publication_plan(
        &dir,
        &wasm,
        &selected,
        &input().controller,
        publication("v1"),
    )
    .unwrap();
    let private = dir.join(&first).join("publication/prepare");
    std::fs::create_dir_all(&private).unwrap();
    std::fs::write(private.join("signed.json"), "PRIVATE_ATTEMPT_SENTINEL").unwrap();
    let report = crate::fleet_artifacts(&dir, &selected).unwrap();
    assert!(report.issues.is_empty());
    assert_eq!(
        report
            .publications
            .iter()
            .map(|p| p.publication.release.version.as_str())
            .collect::<Vec<_>>(),
        ["v1", "v2"]
    );
    assert!(report.publications.iter().all(|p| !p.publication_retained));
    assert_eq!(report.publications[0].operation, first);
    assert_eq!(report.publications[0].publication_operation, first);
    assert_eq!(report.publications[0].artifact, plan.artifact);
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("PRIVATE_ATTEMPT_SENTINEL")
    );
    for selector in 0..3 {
        let mut other = selected.clone();
        match selector {
            0 => other.app = "other".into(),
            1 => other.fleet = "other".into(),
            _ => {
                other.config.network = Network::Local {
                    url: "http://127.0.0.1:1/".into(),
                    root_key: "aa".repeat(133),
                    root_subnet: None,
                    routing_canister: "aaaaa-aa".into(),
                }
            }
        }
        assert!(
            crate::fleet_artifacts(&dir, &other)
                .unwrap()
                .publications
                .is_empty()
        );
    }
    // A completion-shaped record cannot establish publication without its signed observation.
    store::put_json(
        &dir.join(&first).join("publication.json"),
        &progress(
            &plan,
            &first,
            0,
            Some(protocol::Status {
                manifest: manifest(&plan, b"\0asm\x01\0\0\0").unwrap(),
                published: true,
                missing: vec![],
            }),
        )
        .unwrap(),
    )
    .unwrap();
    let report = crate::fleet_artifacts(&dir, &selected).unwrap();
    assert_eq!(report.publications.len(), 1);
    assert_eq!(report.publications[0].operation, second);
    assert!(!report.publications[0].publication_retained);
    assert_eq!(report.issues.len(), 1);
    assert!(report.issues[0].starts_with(&first));
    assert_eq!(
        std::fs::read_to_string(private.join("signed.json")).unwrap(),
        "PRIVATE_ATTEMPT_SENTINEL"
    );
    std::fs::write(dir.join(&second).join("module.wasm"), b"tampered").unwrap();
    let report = crate::fleet_artifacts(&dir, &selected).unwrap();
    assert!(report.publications.is_empty());
    assert_eq!(report.issues.len(), 2);
}
#[test]
fn immutable_release_plans_retain_deadlines_without_reserving_deployment_slots() {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    let mut bytes = vec![42; 2 * 1024 * 1024];
    bytes[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    std::fs::write(&wasm, &bytes).unwrap();
    assert!(artifact::admit(&bytes).is_err()); // Direct install limit is not widened.
    let controller = input().controller;
    let (id, original) =
        create_publication_plan(&dir, &wasm, &selection(), &controller, publication("v1")).unwrap();
    let (same, retained) =
        create_publication_plan(&dir, &wasm, &selection(), &controller, publication("v1")).unwrap();
    assert_eq!(same, id);
    assert_eq!(retained, original);
    assert_eq!(plan::load(&dir.join(&id), &id).unwrap().1, bytes);
    let encoded = serde_json::to_vec(&original).unwrap();
    assert_eq!(serde_json::from_slice::<Plan>(&encoded).unwrap(), original);
    let inventory = crate::fleet::inventory(&dir, false, crate::now().unwrap()).unwrap();
    assert!(inventory.records.is_empty() && inventory.issues.is_empty());
    let mut changed = publication("v1");
    changed.store_module = "33".repeat(32);
    assert!(create_publication_plan(&dir, &wasm, &selection(), &controller, changed).is_err());
    bytes[8] ^= 1;
    std::fs::write(&wasm, &bytes).unwrap();
    assert!(
        create_publication_plan(&dir, &wasm, &selection(), &controller, publication("v1")).is_err()
    );
    assert!(
        create_publication_plan(&dir, &wasm, &selection(), &controller, publication("v2")).is_ok()
    );
    // Publishing a canister's artifact does not claim to have deployed that canister.
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    plan::retain(
        &dir,
        &wasm,
        input(),
        original.deployment,
        None,
        crate::now().unwrap(),
    )
    .unwrap();
    assert!(!dir.join(&id).join("receipt.json").exists());
}
#[test]
fn publication_rejects_conflicting_authority_and_retains_an_exact_read_contract() {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (id, plan) = create_publication_plan(
        &dir,
        &wasm,
        &selection(),
        &input().controller,
        publication("v1"),
    )
    .unwrap();
    let before = std::fs::read(dir.join(&id).join("plan.json")).unwrap();
    for (field, value) in [
        ("creation_cycles", serde_json::json!(1)),
        ("target", serde_json::json!("rrkah-fqaaa-aaaaa-aaaaq-cai")),
    ] {
        let mut record = serde_json::to_value(&plan).unwrap();
        record[field] = value;
        assert!(serde_json::from_value::<Plan>(record).is_err());
    }
    let mut old = plan.clone();
    old.version = 2;
    assert!(old.digest().is_err());
    let deadline = crate::now().unwrap() + wire::MAX_WINDOW_NS;
    let read = observation(&plan, deadline).unwrap();
    let request: wire::Request = ic_wire::decode(
        &read.args,
        ic_wire::Encoding::Frame {
            method: wire::EXECUTE,
            direction: ic_wire::Direction::Request,
        },
        wire::REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(
        read.canister.to_text(),
        plan.publication().unwrap().root.canister
    );
    assert_eq!(request.store, plan.publication().unwrap().store.canister);
    assert!(
        matches!(request.command, Command::Observe(release) if release == plan.publication().unwrap().release)
    );
    let step = dir.join(&id).join("publication/prepare");
    let first = step_deadline(&step, &plan).unwrap();
    assert_eq!(step_deadline(&step, &plan).unwrap(), first);
    assert!(first <= plan.expires_at_ns);
    assert_eq!(
        std::fs::read(dir.join(&id).join("plan.json")).unwrap(),
        before
    );
}
#[test]
fn publication_evidence_cannot_become_an_installation_receipt_or_change_the_manifest() {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    let bytes = b"\0asm\x01\0\0\0";
    std::fs::write(&wasm, bytes).unwrap();
    let (id, plan) = create_publication_plan(
        &dir,
        &wasm,
        &selection(),
        &input().controller,
        publication("v1"),
    )
    .unwrap();
    let expected = manifest(&plan, bytes).unwrap();
    let status = protocol::Status {
        manifest: expected.clone(),
        published: true,
        missing: vec![],
    };
    let frame = ic_wire::encode(
        &Ok::<_, wire::Error>(status.clone()),
        ic_wire::Encoding::Frame {
            method: wire::EXECUTE,
            direction: ic_wire::Direction::Response,
        },
        wire::RESPONSE_LIMITS,
    )
    .unwrap();
    assert_eq!(decode(&frame, &expected).unwrap(), Some(status.clone()));
    let mut wrong = expected;
    wrong.hash = [3; 32];
    assert!(decode(&frame, &wrong).is_err());
    let outcome = progress(&plan, &id, 0, Some(status)).unwrap();
    store::put_json(&dir.join(&id).join("publication.json"), &outcome).unwrap();
    // A summary without its exact retained signed observation cannot prove completion.
    assert!(operation::status(&dir, &id).is_err());
    assert_eq!(plan.application_controller(), plan.controller);
}

#[test]
fn store_declarations_cannot_use_the_greeting_child_contract() {
    let dir = directory();
    let mut selected = selection();
    for slot in selected.config.canisters.values_mut() {
        slot.id = None;
    }
    selected.config.canisters.remove("app");
    let failure = crate::create_managed_plan(
        &dir.join("journal"),
        &dir.join("unused.wasm"),
        input(),
        &selected,
        "store",
        &"11".repeat(32),
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(matches!(failure, Error::Invalid(_)));
    assert!(!dir.join("journal").exists());
}

#[test]
fn deployment_refuses_an_unpublished_release_before_reserving_the_child() {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let selected = selection();
    let (id, _) = create_publication_plan(
        &dir,
        &wasm,
        &selected,
        &input().controller,
        publication("v1"),
    )
    .unwrap();
    assert!(matches!(
        crate::create_managed_plan(
            &dir,
            &wasm,
            input(),
            &selected,
            "app",
            &"11".repeat(32),
            None,
            Some(&id),
            None,
            None,
        ),
        Err(Error::Blocked(_))
    ));
    // Publication did not reserve the child: an ordinary independent approval still fits.
    crate::create_managed_plan(
        &dir,
        &wasm,
        input(),
        &selected,
        "app",
        &"11".repeat(32),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}
