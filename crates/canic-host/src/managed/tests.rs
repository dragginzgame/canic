use super::*;
use crate::{
    CanisterConfig, Network,
    tests::{directory, input},
};

use std::collections::BTreeMap;

fn selection() -> SelectedFleet {
    crate::tests::selected_fleet(
        "starter",
        "mainnet",
        Network::Mainnet,
        BTreeMap::from([
            (
                "root".into(),
                CanisterConfig {
                    id: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".into()),
                    role: Some("root".into()),
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
    )
}

// Retain a Store-backed approval independently of network setup.
fn child_record(dir: &Path, wasm: &Path) -> (String, Plan) {
    let at = crate::now().unwrap();
    plan::retain(
        dir,
        wasm,
        input(),
        Some(DeploymentName {
            app: "starter".into(),
            fleet: "mainnet".into(),
            canister: "app".into(),
        }),
        Some(ManagedOwner {
            child_evidence_only: false,
            root: selection().config.canisters["root"].id.clone().unwrap(),
            root_module: "01".repeat(32),
            execute_at_ns: at,
            slot: ChildId::FIRST,
            arguments: None,
            blob_attempt: None,
            source: crate::tests::stored_source("app", b"\0asm\x01\0\0\0"),
        }),
        at,
    )
    .unwrap()
}

#[tokio::test]
async fn recovery_requires_retained_admission_and_never_bootstraps_work() {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (id, plan) = child_record(&dir, &wasm);
    let before = plan.clone();
    let result = crate::reconcile(&dir, &id, "must-not-run", Path::new("must-not-run")).await;
    assert!(matches!(result, Err(Error::Blocked(_))));
    assert!(!dir.join(&id).join("observe").exists());
    assert!(!dir.join(&id).join("stage").exists());
    assert_eq!(plan::load(&dir.join(&id), &id).unwrap().0, before);
    assert_eq!(
        recovery::call(&plan).unwrap().method,
        "reconcile_child_creation"
    );
    let mut unmanaged = plan;
    unmanaged.action = crate::PlanAction::Create {
        bootstrap: None,
        cycles: unmanaged.creation_cycles(),
    };
    assert!(recovery::call(&unmanaged).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn recovery_evidence_survives_before_observation_and_rejects_later_regression() {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (id, plan) = child_record(&dir, &wasm);
    let operation = dir.join(&id);
    let slot = operation.join("observe/0");
    crate::output_publication::prepare(&slot.join("reconcile"), 0o700).unwrap();
    store::put_json(&slot.join("reconcile/authority.json"), &1_u64).unwrap();
    assert!(recovery::require_settled(&slot).is_err());
    let retained = Status {
        approval: Some(approval(&plan).unwrap()),
        phase: Phase::Created,
        worker: Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into()),
        attempts: [1, 0, 0],
        ..Default::default()
    };
    store::put(
        &slot.join("reconcile/reply.bin"),
        &ic_wire::encode(
            &retained,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::application::RECONCILE_CHILD_CREATION,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap(),
    )
    .unwrap();
    assert!(recovery::require_settled(&slot).is_ok());
    assert_eq!(evidence(&operation, &plan).unwrap(), Some(retained.clone()));
    assert_eq!(
        serde_json::to_value(crate::status(&dir, &id).unwrap()).unwrap()["effects"]["reconcile/0"],
        "reply_retained"
    );
    let earlier = Status {
        approval: retained.approval,
        phase: Phase::Scheduled,
        ..Default::default()
    };
    let next = operation.join("observe/1/reconcile");
    crate::output_publication::prepare(&next, 0o700).unwrap();
    store::put(
        &next.join("reply.bin"),
        &ic_wire::encode(
            &earlier,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::application::RECONCILE_CHILD_CREATION,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        evidence(&operation, &plan),
        Err(Error::Invalid(_))
    ));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn managed_plan_reuses_authority_and_refuses_direct_child_or_root_drift() {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let selected = selection();
    assert!(crate::create_configured_plan(&dir, &wasm, input(), &selected, "app").is_err());
    let refusal = create_managed_plan(
        &dir,
        &wasm,
        input(),
        &selected,
        "app",
        &"01".repeat(32),
        None,
        "",
        None,
        None,
    )
    .unwrap_err();
    assert!(refusal.to_string().contains("--release"));
    let (id, plan) = child_record(&dir, &wasm);
    assert_eq!(plan.management().unwrap().execute_at_ns, plan.created_at_ns);
    assert_eq!(plan.expires_at_ns - plan.created_at_ns, 600_000_000_000);
    assert_eq!(
        plan::admission_deadline(
            plan.created_at_ns,
            plan.management().unwrap().execute_at_ns,
            plan.expires_at_ns
        ),
        plan.expires_at_ns
    );
    assert_eq!(
        plan.application_controller(),
        selected.config.canisters["root"].id.as_ref().unwrap()
    );
    let mut invalid = serde_json::to_value(&plan).unwrap();
    invalid["creation_cycles"] = serde_json::json!(plan.creation_cycles() + 1);
    assert!(serde_json::from_value::<Plan>(invalid).is_err());
    let mut invalid = plan.clone();
    invalid.version = 1;
    invalid.deployment = None;
    assert!(invalid.digest().is_err());
    let mut invalid = plan.clone();
    let crate::PlanAction::CreateChild(owner) = &mut invalid.action else {
        panic!("child creation")
    };
    owner.root = "2vxsx-fae".into();
    assert!(invalid.digest().is_err());
    let mut invalid = plan;
    invalid.expires_at_ns += 1;
    assert!(invalid.digest().is_err());
    let status = serde_json::to_value(crate::status(&dir, &id).unwrap()).unwrap();
    assert_eq!(
        status["effects"],
        serde_json::json!({"stage":"not_submitted"})
    );
    assert!(status["management"].is_null());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn managed_reply_preserves_binding_child_and_attempt_evidence() {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (_, plan) = child_record(&dir, &wasm);
    let scheduled = Status {
        approval: Some(approval(&plan).unwrap()),
        phase: Phase::Scheduled,
        ..Default::default()
    };
    assert!(validate(&plan, None, &scheduled).is_ok());
    let ready = Status {
        phase: Phase::Ready,
        worker: Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into()),
        attempts: [1; 3],
        observed_module: Some(crate::unhex(&plan.artifact.sha256).unwrap()),
        ..scheduled.clone()
    };
    assert!(validate(&plan, Some(&scheduled), &ready).is_ok());
    assert!(validate(&plan, Some(&ready), &scheduled).is_err());
    let mut reused = ready.clone();
    reused.attempts[0] = 0;
    assert!(validate(&plan, Some(&scheduled), &reused).is_ok());
    assert!(validate(&plan, Some(&reused), &ready).is_err());
    reused.creation_refund = Some(0);
    assert!(validate(&plan, None, &reused).is_err());
    let mut changed = ready.clone();
    changed.worker = Some("r7inp-6aaaa-aaaaa-aaabq-cai".into());
    assert!(validate(&plan, Some(&ready), &changed).is_err());
    let mut changed = ready.clone();
    changed.approval.as_mut().unwrap().expires_at_ns += 1;
    assert!(validate(&plan, None, &changed).is_err());
    let mut changed = ready.clone();
    changed.attempts[0] = 2;
    assert!(validate(&plan, None, &changed).is_err());
    let mut changed = ready;
    changed.observed_module = Some(vec![0; 32]);
    assert!(validate(&plan, None, &changed).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn upgrade_requires_creation_receipt_and_keeps_both_approvals_immutable() {
    use canic_management_protocol::children::ChildId;
    for slot in [ChildId::FIRST, ChildId::SECOND] {
        check_named_upgrade_custody(slot);
    }
}

fn check_named_upgrade_custody(slot: canic_management_protocol::children::ChildId) {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let mut selected = selection();
    let root_hash = "01".repeat(32);
    let (created_id, created) = {
        let now = crate::now().unwrap();
        plan::retain(
            &dir,
            &wasm,
            input(),
            Some(DeploymentName {
                app: selected.app.clone(),
                fleet: selected.fleet.clone(),
                canister: "app".into(),
            }),
            Some(ManagedOwner {
                child_evidence_only: false,
                arguments: None,
                blob_attempt: None,
                slot,
                root: selected.config.canisters["root"].id.clone().unwrap(),
                root_module: root_hash.clone(),
                execute_at_ns: now,
                source: crate::tests::stored_source("app", b"\0asm\x01\0\0\0"),
            }),
            now,
        )
    }
    .unwrap();
    selected
        .config
        .canisters
        .insert("sibling".into(), selected.config.canisters["app"].clone());
    let child = "ryjl3-tyaaa-aaaaa-aaaba-cai";
    let upgrade_input = || PlanInput {
        target: Some(child.into()),
        previous_module: Some(created.artifact.sha256.clone()),
        creation_cycles: 0,
        ..input()
    };
    std::fs::write(&wasm, b"\0asm\x01\0\0\0\0\x01\0").unwrap();
    assert!(
        create_managed_plan(
            &dir,
            &wasm,
            upgrade_input(),
            &selected,
            "app",
            &root_hash,
            None,
            "",
            None,
            None,
        )
        .is_err()
    );
    let ready = Status {
        approval: Some(approval(&created).unwrap()),
        phase: Phase::Ready,
        worker: Some(child.into()),
        attempts: [1; 3],
        requests: [Some([7; 32]); 3],
        observed_module: Some(crate::unhex(&created.artifact.sha256).unwrap()),
        ..Default::default()
    };
    let created_dir = dir.join(&created_id);
    crate::output_publication::prepare(&created_dir.join("stage"), 0o700).unwrap();
    store::put(
        &created_dir.join("stage/reply.bin"),
        &ic_wire::encode(
            &Ok::<_, Refusal>(ready),
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::application::STAGE_CHILD_CREATION,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        create_managed_plan(
            &dir,
            &wasm,
            upgrade_input(),
            &selected,
            "app",
            &root_hash,
            None,
            "",
            None,
            None,
        )
        .is_err()
    );
    let receipt = Receipt {
        operation: created_id.clone(),
        canister: child.into(),
        module_hash: created.artifact.sha256.clone(),
        controller: created.application_controller().into(),
    };
    store::put_json(&created_dir.join("receipt.json"), &receipt).unwrap();
    for kind in 0..3 {
        let mut invalid = upgrade_input();
        match kind {
            0 => invalid.target = Some("r7inp-6aaaa-aaaaa-aaabq-cai".into()),
            1 => invalid.previous_module = Some("02".repeat(32)),
            _ => invalid.creation_cycles = 1,
        }
        assert!(
            create_managed_plan(
                &dir, &wasm, invalid, &selected, "app", &root_hash, None, "", None, None,
            )
            .is_err()
        );
    }
    let refusal = create_managed_plan(
        &dir,
        &wasm,
        upgrade_input(),
        &selected,
        "app",
        &root_hash,
        None,
        "",
        None,
        None,
    )
    .unwrap_err();
    assert!(refusal.to_string().contains("--release"));
    assert_eq!(plan::load(&created_dir, &created_id).unwrap().0, created);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn upgrade_observations_require_version_proof_and_monotonic_custody() {
    use canic_management_protocol::upgrade as wire;
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (_, mut plan) = child_record(&dir, &wasm);
    plan.action = crate::PlanAction::UpgradeChild {
        target: crate::UpgradeTarget {
            canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
            module: "02".repeat(32),
        },
        owner: plan.management().unwrap().clone(),
    };
    let installed = wire::Status {
        approval: Some(upgrade::approval(&plan).unwrap()),
        phase: wire::Phase::InstallIssued,
        attempts: [1; 2],
        requests: [Some([3; 32]), Some([4; 32])],
        previous_version: Some(7),
        ..Default::default()
    };
    let ready = wire::Status {
        phase: wire::Phase::Ready,
        observed_version: Some(8),
        ..installed.clone()
    };
    let stopped = wire::Status {
        phase: wire::Phase::Stopped,
        attempts: [0; 2],
        requests: [None; 2],
        previous_version: None,
        ..installed.clone()
    };
    let checked = wire::Status {
        phase: wire::Phase::Checked,
        previous_version: Some(7),
        ..stopped.clone()
    };
    upgrade::validate(&plan, Some(&stopped), &checked).unwrap();
    assert!(upgrade::validate(&plan, Some(&checked), &stopped).is_err());
    let impossible_stop = wire::Status {
        phase: wire::Phase::StopIssued,
        ..installed.clone()
    };
    assert!(upgrade::validate(&plan, None, &impossible_stop).is_err());
    let started = wire::Status {
        phase: wire::Phase::Started,
        ..ready.clone()
    };
    upgrade::validate(&plan, Some(&started), &ready).unwrap();
    assert!(upgrade::validate(&plan, Some(&ready), &started).is_err());
    let missing_installation = wire::Status {
        observed_version: None,
        ..started
    };
    assert!(upgrade::validate(&plan, None, &missing_installation).is_err());
    for kind in 0..6 {
        let mut invalid = ready.clone();
        match kind {
            0 => invalid.observed_version = None,
            1 => invalid.observed_version = Some(9),
            2 => {
                invalid.previous_version = Some(6);
                invalid.observed_version = Some(7);
            }
            3 => invalid.requests[1] = Some([5; 32]),
            4 => invalid.approval.as_mut().unwrap().child = "r7inp-6aaaa-aaaaa-aaabq-cai".into(),
            _ => {
                invalid.attempts[1] = 0;
                invalid.requests[1] = None;
            }
        }
        assert!(
            upgrade::validate(&plan, Some(&installed), &invalid).is_err(),
            "case {kind}"
        );
    }
    upgrade::validate(&plan, Some(&installed), &ready).unwrap();
    assert!(upgrade::validate(&plan, Some(&ready), &installed).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stored_child_binds_release_slot_and_retained_recovery_without_inline_bytes() {
    use canic_management_protocol::{children, stored as wire};
    use ic_wasm_store_protocol::{Manifest, Release};
    let dir = directory();
    let wasm = dir.join("app.wasm");
    let bytes = b"\0asm\x01\0\0\0";
    std::fs::write(&wasm, bytes).unwrap();
    let (_, mut plan) = child_record(&dir, &wasm);
    let source = crate::StoredArtifact {
        maintenance: None,
        store: crate::UpgradeTarget {
            canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
            module: "02".repeat(32),
        },
        manifest: Manifest {
            release: Release {
                name: "app".into(),
                version: "v1".into(),
            },
            hash: ic_wasm_store_protocol::digest(bytes),
            bytes: bytes.len() as u64,
            chunks: vec![ic_wasm_store_protocol::digest(bytes)],
        },
    };
    let crate::PlanAction::CreateChild(owner) = &mut plan.action else {
        panic!("creation")
    };
    owner.source = source.clone();
    plan.validate().unwrap();
    let digest = plan.digest().unwrap();
    let call = Method::Stage.call(&plan).unwrap();
    assert_eq!(call.method, "stage_child_creation");
    let request: children::Request<canic_management_protocol::application::Creation> =
        ic_wire::decode(
            &call.args,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::application::STAGE_CHILD_CREATION,
                direction: ic_wire::Direction::Request,
            },
            wire::LIMITS,
        )
        .unwrap();
    let canic_management_protocol::application::Deployment::Stored(value) = request.value else {
        panic!("expected Stored input")
    };
    let request = canic_management_protocol::children::Request {
        slot: request.slot,
        value,
    };
    assert_eq!(request.slot, children::ChildId::FIRST);
    let request_frame = |method| ic_wire::Encoding::Frame {
        method,
        direction: ic_wire::Direction::Request,
    };
    let old_frame = ic_wire::encode(&request, request_frame(wire::STAGE), wire::LIMITS).unwrap();
    assert!(
        ic_wire::decode::<children::Request<canic_management_protocol::application::Creation>>(
            &old_frame,
            request_frame(canic_management_protocol::application::STAGE_CHILD_CREATION),
            wire::LIMITS,
        )
        .is_err()
    );
    assert_eq!(request.value.reference.manifest, source.manifest);
    let recovery = recovery::call(&plan).unwrap();
    let recovered: children::Request<canic_management_protocol::application::Creation> =
        ic_wire::decode(
            &recovery.args,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::application::RECONCILE_CHILD_CREATION,
                direction: ic_wire::Direction::Request,
            },
            wire::LIMITS,
        )
        .unwrap();
    let canic_management_protocol::application::Deployment::Stored(value) = recovered.value else {
        panic!("expected Stored input")
    };
    let recovered = canic_management_protocol::children::Request {
        slot: recovered.slot,
        value,
    };
    assert_eq!(recovered.value, request.value);
    assert_eq!(observation(&plan).unwrap().method, "observe_root_custody");
    let status = Status {
        approval: Some(approval(&plan).unwrap()),
        phase: Phase::Scheduled,
        ..Default::default()
    };
    let delivery = wire::Delivery {
        reference: request.value.reference.clone(),
        operation: request.value.pin(wire::STAGE, request.slot).operation,
        state: wire::PinState::Pending,
        cache_deadline: None,
    };
    let mut custody = wire::Custody {
        children: {
            let mut slots = [
                canic_management_protocol::custody::Custody {
                    creation: status.clone(),
                    upgrades: vec![],
                },
                canic_management_protocol::custody::Custody {
                    creation: Status::default(),
                    upgrades: vec![],
                },
            ]
            .into_iter();
            (0..8)
                .map(|_| {
                    slots
                        .next()
                        .unwrap_or_else(|| canic_management_protocol::custody::Custody {
                            creation: Default::default(),
                            upgrades: vec![],
                        })
                })
                .collect()
        },
        sources: {
            let mut slots = [
                Some(wire::Sources {
                    creation: delivery.clone(),
                    upgrades: vec![],
                }),
                None,
            ]
            .into_iter();
            (0..8).map(|_| slots.next().flatten()).collect()
        },
    };
    let encode = |value: &wire::Custody| {
        crate::root_custody::frame(
            canic_management_protocol::custody::Root {
                stored: value.clone(),
                store_operation: None,
            },
            vec![],
        )
    };
    assert_eq!(stored::creation(&plan, &encode(&custody)).unwrap(), status);
    assert_application_arguments(&plan, &custody);
    assert_delivery_history(&plan, custody.clone());
    // Project the existing authenticated observation; status must not create a
    // second source record or infer delivery from the approved manifest.
    let evidence_dir = dir.join("delivery-evidence");
    std::fs::create_dir_all(&evidence_dir).unwrap();
    let projected = |plan: &Plan| {
        crate::operation::status_loaded(&evidence_dir, &plan.digest().unwrap(), plan.clone(), bytes)
    };
    assert!(matches!(
        projected(&plan).unwrap().detail,
        crate::StatusDetail::StoredChild {
            delivery: None,
            maintenance: None
        }
    ));
    let observation_dir = evidence_dir.join("observe/0");
    std::fs::create_dir_all(&observation_dir).unwrap();
    let reply = observation_dir.join("reply.bin");
    for state in [
        wire::PinState::Pending,
        wire::PinState::Issued,
        wire::PinState::Retained,
    ] {
        let delivery = &mut custody.sources[0].as_mut().unwrap().creation;
        delivery.state = state;
        delivery.cache_deadline = (state == wire::PinState::Retained).then_some(123);
        let original = encode(&custody);
        std::fs::write(&reply, &original).unwrap();
        let value = projected(&plan).unwrap();
        assert_eq!(
            serde_json::to_value(&value).unwrap()["delivery"],
            serde_json::to_value(&custody.sources[0].as_ref().unwrap().creation).unwrap()
        );
        assert_eq!(std::fs::read(&reply).unwrap(), original);
        assert!(!evidence_dir.join("stage").exists());
    }
    custody.sources[0].as_mut().unwrap().creation = delivery.clone();
    // A successor Store binding selects the complete contract explicitly. The
    // old frame, another attempt or incomplete preservation cannot stand in for it.
    {
        use canic_management_protocol::{custody as full, store_maintenance as maintenance};
        let attempt = ic_wasm_store_protocol::MaintenanceAttempt {
            sequence: 1,
            operation: [7; 32],
        };
        let mut successor = plan.clone();
        let crate::PlanAction::CreateChild(owner) = &mut successor.action else {
            unreachable!()
        };
        owner.source.maintenance = Some(attempt.clone());
        assert_ne!(successor.digest().unwrap(), digest);
        assert_eq!(
            observation(&successor).unwrap().method,
            "observe_root_custody"
        );
        assert!(stored::creation(&successor, &encode(&custody)).is_err());
        let preservation = maintenance::Preservation {
            after_removed: false,
            catalog: ic_wasm_store_protocol::preservation::Catalog {
                quota: Default::default(),
                reserved_bytes: 0,
                releases: 0,
                artifacts: 0,
                chunks: 0,
            },
            after: None,
            observed: 0,
            digest: [9; 32],
            complete: true,
        };
        let operation = maintenance::Status {
            approval: maintenance::Approval {
                store: source.store.canister.clone(),
                previous_module: [1; 32],
                module_hash: [2; 32],
                attempt,
                expires_at_ns: plan.expires_at_ns,
            },
            operator: plan.controller.clone(),
            phase: maintenance::Phase::Ready,
            stop_request: Some([3; 32]),
            failure: None,
            preservation: canic_management_protocol::store_maintenance::Scan::Legacy(Some(
                preservation.clone(),
            )),
            installation: Some(maintenance::Installation {
                upload_request: [4; 32],
                request: Some([5; 32]),
                sender_version: Some(1),
                start_request: Some([6; 32]),
                verification: canic_management_protocol::store_maintenance::Scan::Legacy(Some(
                    preservation,
                )),
                activation_request: Some([8; 32]),
            }),
        };
        let full_custody = full::Root {
            stored: custody.clone(),
            store_operation: Some(operation),
        };
        let frame = |value: &full::Root| crate::root_custody::frame(value.clone(), vec![]);
        assert_eq!(
            stored::creation(&successor, &frame(&full_custody)).unwrap(),
            status
        );
        assert!(stored::creation(&plan, &frame(&full_custody)).is_err());
        for case in 0..6 {
            let mut changed = full_custody.clone();
            let operation = changed.store_operation.as_mut().unwrap();
            match case {
                0 => operation.approval.attempt.operation[0] ^= 1,
                1 => operation.approval.module_hash[0] ^= 1,
                2 => operation.operator = operation.approval.store.clone(),
                3 => {
                    operation.preservation =
                        canic_management_protocol::store_maintenance::Scan::Legacy(None)
                }
                4 => operation.phase = maintenance::Phase::Verified,
                _ => changed.store_operation = None,
            }
            assert!(
                stored::creation(&successor, &frame(&changed)).is_err(),
                "case {case}"
            );
        }
    }
    custody.sources[0]
        .as_mut()
        .unwrap()
        .creation
        .reference
        .manifest
        .release
        .version = "different".into();
    assert!(matches!(
        stored::creation(&plan, &encode(&custody)),
        Err(Error::Invalid(_))
    ));
    std::fs::write(&reply, encode(&custody)).unwrap();
    assert!(matches!(projected(&plan), Err(Error::Invalid(_))));
    custody.sources[0].as_mut().unwrap().creation = delivery;
    custody.sources[0].as_mut().unwrap().creation.operation = request
        .value
        .pin(wire::STAGE, children::ChildId::SECOND)
        .operation;
    assert!(stored::creation(&plan, &encode(&custody)).is_err());
    let crate::PlanAction::CreateChild(owner) = &mut plan.action else {
        panic!("creation")
    };
    owner.source.manifest.release.version = "v2".into();
    assert_ne!(plan.digest().unwrap(), digest);
    let crate::PlanAction::CreateChild(owner) = &mut plan.action else {
        panic!("creation")
    };
    owner.source.manifest.hash[0] ^= 1;
    assert!(plan.validate().is_err());
    let mut owner = plan.management().unwrap().clone();
    owner.source = source;
    plan.action = crate::PlanAction::UpgradeChild {
        owner,
        target: crate::UpgradeTarget {
            canister: "r7inp-6aaaa-aaaaa-aaabq-cai".into(),
            module: "03".repeat(32),
        },
    };
    plan.validate().unwrap();
    let call = Method::StageUpgrade.call(&plan).unwrap();
    let expected: children::Request<canic_management_protocol::application::Replacement> =
        ic_wire::decode(
            &call.args,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::application::STAGE_CHILD_UPGRADE,
                direction: ic_wire::Direction::Request,
            },
            wire::LIMITS,
        )
        .unwrap();
    let canic_management_protocol::application::Deployment::Stored(value) = expected.value else {
        panic!("expected Stored input")
    };
    let expected = canic_management_protocol::children::Request {
        slot: expected.slot,
        value,
    };
    let upgraded = canic_management_protocol::upgrade::Status {
        approval: Some(expected.value.approval.clone()),
        phase: canic_management_protocol::upgrade::Phase::Scheduled,
        ..Default::default()
    };
    custody.children[0].upgrades.push(upgraded.clone());
    custody.sources[0]
        .as_mut()
        .unwrap()
        .upgrades
        .push(wire::Delivery {
            operation: expected
                .value
                .pin(wire::STAGE_UPGRADE, expected.slot)
                .operation,
            reference: expected.value.reference,
            state: wire::PinState::Pending,
            cache_deadline: None,
        });
    assert_eq!(
        stored::upgraded(&plan, &encode(&custody)).unwrap(),
        upgraded
    );
    for (child_evidence_only, observe_method) in [
        (false, "observe_root_custody"),
        (true, "observe_child_deployments"),
    ] {
        let mut blob = plan.clone();
        let crate::PlanAction::UpgradeChild { owner, .. } = &mut blob.action else {
            panic!("expected child upgrade")
        };
        owner.arguments = Some(vec![1]);
        owner.blob_attempt = Some(1);
        owner.child_evidence_only = child_evidence_only;
        blob.validate().unwrap();
        assert_eq!(
            Method::ActivateUpgrade.call(&blob).unwrap().method,
            "activate_child_upgrade"
        );
        assert_eq!(observation(&blob).unwrap().method, observe_method);
    }
    let mut mixed = plan.clone();
    let crate::PlanAction::UpgradeChild { owner, .. } = &mut mixed.action else {
        unreachable!()
    };
    owner.child_evidence_only = true;
    mixed.validate().unwrap();
    assert_eq!(
        observation(&mixed).unwrap().method,
        "observe_child_deployments"
    );
    let child_evidence = canic_management_protocol::application::Custody {
        root: canic_management_protocol::custody::RootHistory {
            current: canic_management_protocol::custody::Root {
                stored: custody.clone(),
                store_operation: None,
            },
            store_history: Vec::new(),
        },
        applications: vec![None; custody.children.len()],
    };
    let frame = ic_wire::encode(
        &child_evidence,
        ic_wire::Encoding::Frame {
            method: canic_management_protocol::application::CHILD_EVIDENCE,
            direction: ic_wire::Direction::Response,
        },
        canic_management_protocol::application::LIMITS,
    )
    .unwrap();
    assert_eq!(stored::upgraded(&mixed, &frame).unwrap(), upgraded);
    assert!(stored::upgraded(&mixed, &encode(&custody)).is_err());
    assert!(stored::upgraded(&plan, &frame).is_err());
    assert_delivery_history(&plan, custody.clone());
    std::fs::write(&reply, encode(&custody)).unwrap();
    let value = projected(&plan).unwrap();
    assert!(matches!(
        value.management,
        Some(crate::ManagementStatus::Upgrade(_))
    ));
    assert_eq!(
        serde_json::to_value(&value).unwrap()["delivery"],
        serde_json::to_value(&custody.sources[0].as_ref().unwrap().upgrades[0]).unwrap()
    );
    custody.sources[0].as_mut().unwrap().upgrades[0]
        .reference
        .store = "rrkah-fqaaa-aaaaa-aaaaq-cai".into();
    assert!(stored::upgraded(&plan, &encode(&custody)).is_err());
    std::fs::write(&reply, encode(&custody)).unwrap();
    assert!(matches!(projected(&plan), Err(Error::Invalid(_))));
    std::fs::remove_dir_all(dir).unwrap();
}

fn assert_application_arguments(
    original: &Plan,
    custody: &canic_management_protocol::stored::Custody,
) {
    use canic_management_protocol::{
        application,
        children::Request,
        custody::{Root, RootHistory},
    };
    let mut plan = original.clone();
    plan.version = 1;
    let crate::PlanAction::CreateChild(owner) = &mut plan.action else {
        panic!("creation")
    };
    let named = |key| {
        canic::topology::directory::ServiceInstall {
            funding_operator: None,
            hub_operator: None,
            name: "accounts_hub".into(),
            role: "hub".into(),
            application: vec![key; 32],
        }
        .encode()
        .unwrap()
    };
    let approved_args = named(1);
    owner.arguments = Some(approved_args.clone());
    plan.validate().unwrap();
    let digest = plan.digest().unwrap();
    assert_ne!(digest, original.digest().unwrap());
    assert_eq!(
        serde_json::from_slice::<Plan>(&serde_json::to_vec(&plan).unwrap()).unwrap(),
        plan
    );
    let call = Method::Stage.call(&plan).unwrap();
    assert_eq!(call.method, "stage_child_creation");
    let request: Request<canic_management_protocol::application::Creation> = ic_wire::decode(
        &call.args,
        ic_wire::Encoding::Frame {
            method: canic_management_protocol::application::STAGE_CHILD_CREATION,
            direction: ic_wire::Direction::Request,
        },
        application::LIMITS,
    )
    .unwrap();
    let canic_management_protocol::application::Deployment::Application(value) = request.value
    else {
        panic!("expected Application input")
    };
    let request = canic_management_protocol::children::Request {
        slot: request.slot,
        value,
    };
    assert_eq!(request.value.arguments.as_slice(), approved_args);
    let mut value = application::Custody {
        root: RootHistory {
            current: Root {
                stored: custody.clone(),
                store_operation: None,
            },
            store_history: vec![],
        },
        applications: {
            let mut slots = [
                Some(application::Lifecycle {
                    creation: request.value.arguments.clone(),
                    upgrades: vec![],
                }),
                None,
            ]
            .into_iter();
            (0..8).map(|_| slots.next().flatten()).collect()
        },
    };
    value.root.current.stored.sources[0]
        .as_mut()
        .unwrap()
        .creation
        .operation = request
        .value
        .pin(application::STAGE, request.slot)
        .operation;
    let encode = |value: &application::Custody| {
        ic_wire::encode(
            value,
            ic_wire::Encoding::Frame {
                method: application::CUSTODY,
                direction: ic_wire::Direction::Response,
            },
            application::LIMITS,
        )
        .unwrap()
    };
    assert_eq!(
        stored::creation(&plan, &encode(&value)).unwrap(),
        custody.children[0].creation
    );
    value.applications[0].as_mut().unwrap().creation =
        application::Arguments::new(named(2)).unwrap();
    assert!(stored::creation(&plan, &encode(&value)).is_err());
    value.applications[0] = None;
    assert!(stored::creation(&plan, &encode(&value)).is_err());
    let crate::PlanAction::CreateChild(owner) = &mut plan.action else {
        unreachable!()
    };
    owner.arguments = Some(named(2));
    assert_ne!(digest, plan.digest().unwrap());
    assert!(!crate::plan::same_authority(original, &plan));
    for arguments in [vec![], vec![0; application::MAX_ARGUMENT_BYTES + 1]] {
        let crate::PlanAction::CreateChild(owner) = &mut plan.action else {
            unreachable!()
        };
        owner.arguments = Some(arguments);
        assert!(plan.validate().is_err());
    }
}

// Both effect readers must retain delivery across partial recovery replies and
// refuse later custody that would erase a pin or renew its cache deadline.
fn assert_delivery_history(plan: &Plan, mut custody: canic_management_protocol::stored::Custody) {
    use canic_management_protocol::{stored as wire, upgrade as protocol_upgrade};
    let dir = directory();
    let upgrading = plan.target().is_some();
    let source = |value: &mut wire::Custody, delivery: wire::Delivery| {
        let sources = value.sources[0].as_mut().unwrap();
        if upgrading {
            sources.upgrades[0] = delivery;
        } else {
            sources.creation = delivery;
        }
    };
    let sources = custody.sources[0].as_ref().unwrap();
    let mut delivery = if upgrading {
        sources.upgrades[0].clone()
    } else {
        sources.creation.clone()
    };
    let write = |index: usize, value: &wire::Custody| {
        let path = dir.join(format!("observe/{index}/reply.bin"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bytes = crate::root_custody::frame(
            canic_management_protocol::custody::Root {
                stored: value.clone(),
                store_operation: None,
            },
            vec![],
        );
        std::fs::write(path, bytes).unwrap();
    };
    let read = || crate::operation::status_loaded(&dir, &plan.digest().unwrap(), plan.clone(), &[]);
    for (index, state) in [
        wire::PinState::Pending,
        wire::PinState::Issued,
        wire::PinState::Retained,
    ]
    .into_iter()
    .enumerate()
    {
        delivery.state = state;
        source(&mut custody, delivery.clone());
        write(index, &custody);
        let crate::StatusDetail::StoredChild {
            delivery: Some(observed),
            ..
        } = read().unwrap().detail
        else {
            panic!("retained delivery");
        };
        assert_eq!(*observed, delivery);
        if state == wire::PinState::Issued {
            let mut regressed = delivery.clone();
            regressed.state = wire::PinState::Pending;
            source(&mut custody, regressed);
            write(index + 1, &custody);
            assert!(matches!(read(), Err(Error::Invalid(_))));
        }
    }
    delivery.cache_deadline = Some(plan.expires_at_ns);
    source(&mut custody, delivery.clone());
    // The later recovery reply contains child progress, but no delivery fields.
    let partial = if upgrading {
        let status = &mut custody.children[0].upgrades[0];
        status.phase = protocol_upgrade::Phase::UploadIssued;
        status.attempts = [1, 0];
        status.requests = [Some([1; 32]), None];
        status.previous_version = Some(1);
        ic_wire::encode(
            status,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::application::RECONCILE_CHILD_UPGRADE,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap()
    } else {
        let status = &mut custody.children[0].creation;
        status.phase = Phase::UploadIssued;
        status.worker = Some("r7inp-6aaaa-aaaaa-aaabq-cai".into());
        status.attempts = [1, 1, 0];
        status.requests = [Some([1; 32]), Some([2; 32]), None];
        ic_wire::encode(
            status,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::application::RECONCILE_CHILD_CREATION,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap()
    };
    write(3, &custody);
    let partial_path = dir.join("observe/4/reconcile/reply.bin");
    std::fs::create_dir_all(partial_path.parent().unwrap()).unwrap();
    std::fs::write(&partial_path, &partial).unwrap();
    let crate::StatusDetail::StoredChild {
        delivery: Some(observed),
        ..
    } = read().unwrap().detail
    else {
        panic!("partial recovery must preserve earlier delivery");
    };
    assert_eq!(*observed, delivery);
    // A repeated full observation is admissible; changed deadlines and pin
    // regressions after the partial reply are not.
    write(5, &custody);
    assert!(read().is_ok());
    for change in 0..5 {
        let mut changed = delivery.clone();
        match change {
            0 => changed.cache_deadline = None,
            1 => changed.cache_deadline = Some(plan.expires_at_ns - 1),
            2 => changed.cache_deadline = Some(plan.expires_at_ns + 1),
            3 => changed.state = wire::PinState::Issued,
            _ => changed.state = wire::PinState::Pending,
        }
        source(&mut custody, changed);
        write(5, &custody);
        assert!(
            matches!(read(), Err(Error::Invalid(_))),
            "mutation {change}"
        );
        assert_eq!(std::fs::read(&partial_path).unwrap(), partial);
    }
    custody.sources[0] = None;
    write(5, &custody);
    assert!(matches!(read(), Err(Error::Invalid(_))));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn adding_discovery_preserves_two_earlier_named_creation_approvals() {
    let fleet = selection();
    let deployment = |name: &str| DeploymentName {
        app: fleet.app.clone(),
        fleet: fleet.fleet.clone(),
        canister: name.into(),
    };
    let user = deployment("user_hub");
    let registry = deployment("project_registry");
    let discovery = deployment("discovery_hub");
    let children = ["discovery_hub", "project_registry", "user_hub"];
    let reserved = [
        (Some(ChildId::SECOND), Some(&user)),
        (Some(ChildId::FIRST), Some(&registry)),
    ];
    for (name, expected) in [
        (&user, ChildId::SECOND),
        (&registry, ChildId::FIRST),
        (&discovery, ChildId::HISTORICAL_SERVICES[2]),
    ] {
        assert_eq!(
            permanent_slot(&children, name, reserved.into_iter()).unwrap(),
            expected
        );
    }
}
