use super::*;
use crate::{
    CanisterConfig, FleetConfig, Network,
    tests::{directory, input},
};
use canic_management_protocol::{STAGE, STATUS};
use std::collections::BTreeMap;

fn selection() -> SelectedFleet {
    SelectedFleet {
        app: "starter".into(),
        fleet: "mainnet".into(),
        config: FleetConfig {
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

#[tokio::test]
async fn recovery_requires_retained_admission_and_never_bootstraps_work() {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (id, plan) = create_managed_plan(
        &dir,
        &wasm,
        input(),
        &selection(),
        "app",
        &"01".repeat(32),
        None,
        None,
    )
    .unwrap();
    let before = plan.clone();
    let result = crate::reconcile(&dir, &id, "must-not-run", Path::new("must-not-run")).await;
    assert!(matches!(result, Err(Error::Blocked(_))));
    assert!(!dir.join(&id).join("observe").exists());
    assert!(!dir.join(&id).join("stage").exists());
    assert_eq!(plan::load(&dir.join(&id), &id).unwrap().0, before);
    let call = recovery::call(&plan).unwrap();
    assert_eq!(call.method, "reconcile");
    assert_eq!(
        call.canister,
        principal(plan.application_controller()).unwrap()
    );
    let bound: Approval =
        ic_wire::native::decode_request(canic_management_protocol::RECONCILE, &call.args, LIMITS)
            .unwrap();
    assert_eq!(bound, approval(&plan).unwrap());
    let mut unmanaged = plan;
    unmanaged.action = crate::PlanAction::Create {
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
    let (id, plan) = create_managed_plan(
        &dir,
        &wasm,
        input(),
        &selection(),
        "app",
        &"01".repeat(32),
        None,
        None,
    )
    .unwrap();
    let operation = dir.join(&id);
    let slot = operation.join("observe/0");
    ic_host_fs::durable::prepare_private_parents(&slot.join("reconcile")).unwrap();
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
        &ic_wire::native::encode_response(canic_management_protocol::RECONCILE, &retained, LIMITS)
            .unwrap(),
    )
    .unwrap();
    assert!(recovery::require_settled(&slot).is_ok());
    assert_eq!(evidence(&operation, &plan).unwrap(), Some(retained.clone()));
    assert_eq!(
        crate::status(&dir, &id).unwrap()["effects"]["reconcile/0"],
        "reply_retained"
    );
    let earlier = Status {
        approval: retained.approval,
        phase: Phase::Scheduled,
        ..Default::default()
    };
    store::put(
        &slot.join("reply.bin"),
        &ic_wire::native::encode_response(STATUS, &earlier, LIMITS).unwrap(),
    )
    .unwrap();
    assert!(evidence(&operation, &plan).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn managed_plan_reuses_authority_and_refuses_direct_child_or_root_drift() {
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let selected = selection();
    assert!(crate::create_configured_plan(&dir, &wasm, input(), &selected, "app").is_err());
    let (id, plan) = create_managed_plan(
        &dir,
        &wasm,
        input(),
        &selected,
        "app",
        &"01".repeat(32),
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        plan.application_controller(),
        selected.config.canisters["root"].id.as_ref().unwrap()
    );
    assert_eq!(
        create_managed_plan(
            &dir,
            &wasm,
            input(),
            &selected,
            "app",
            &"01".repeat(32),
            None,
            None
        )
        .unwrap(),
        (id.clone(), plan.clone())
    );
    assert_eq!(
        create_managed_plan(
            &dir,
            &wasm,
            input(),
            &selected,
            "app",
            &"01".repeat(32),
            Some(1),
            None
        )
        .unwrap(),
        (id.clone(), plan.clone())
    );
    assert!(matches!(
        create_managed_plan(
            &dir,
            &wasm,
            input(),
            &selected,
            "app",
            &"02".repeat(32),
            None,
            None
        ),
        Err(Error::Blocked(_))
    ));
    let mut other = selected;
    other.app = "another".into();
    assert!(matches!(
        create_managed_plan(
            &dir,
            &wasm,
            input(),
            &other,
            "app",
            &"01".repeat(32),
            None,
            None
        ),
        Err(Error::Blocked(_))
    ));
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
    let status = crate::status(&dir, &id).unwrap();
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
    let (_, plan) = create_managed_plan(
        &dir,
        &wasm,
        input(),
        &selection(),
        "app",
        &"01".repeat(32),
        None,
        None,
    )
    .unwrap();
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
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let selected = selection();
    let root_hash = "01".repeat(32);
    let (created_id, created) = create_managed_plan(
        &dir,
        &wasm,
        input(),
        &selected,
        "app",
        &root_hash,
        None,
        None,
    )
    .unwrap();
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
            None
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
    ic_host_fs::durable::prepare_private_parents(&created_dir.join("stage")).unwrap();
    store::put(
        &created_dir.join("stage/reply.bin"),
        &ic_wire::native::encode_response(STAGE, &Ok::<_, Refusal>(ready), LIMITS).unwrap(),
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
            None
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
                &dir, &wasm, invalid, &selected, "app", &root_hash, None, None
            )
            .is_err()
        );
    }
    let (id, plan) = create_managed_plan(
        &dir,
        &wasm,
        upgrade_input(),
        &selected,
        "app",
        &root_hash,
        None,
        None,
    )
    .unwrap();
    assert_ne!(id, created_id);
    assert_eq!(plan.target().map(String::as_str), Some(child));
    assert_eq!(
        plan.application_controller(),
        created.application_controller()
    );
    assert_eq!(
        create_managed_plan(
            &dir,
            &wasm,
            upgrade_input(),
            &selected,
            "app",
            &root_hash,
            Some(1),
            None
        )
        .unwrap(),
        (id, plan)
    );
    assert_eq!(plan::load(&created_dir, &created_id).unwrap().0, created);
    assert_eq!(
        store::json::<Receipt>(&created_dir.join("receipt.json")).unwrap(),
        receipt
    );
    assert!(
        create_managed_plan(
            &dir,
            &wasm,
            upgrade_input(),
            &selected,
            "app",
            &"02".repeat(32),
            None,
            None
        )
        .is_err()
    );
    std::fs::write(&wasm, b"\0asm\x01\0\0\0\0\x02\x01x").unwrap();
    assert!(
        create_managed_plan(
            &dir,
            &wasm,
            upgrade_input(),
            &selected,
            "app",
            &root_hash,
            None,
            None
        )
        .is_err()
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn upgrade_observations_require_version_proof_and_monotonic_custody() {
    use canic_management_protocol::upgrade as wire;
    let dir = directory();
    let wasm = dir.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (_, mut plan) = create_managed_plan(
        &dir,
        &wasm,
        input(),
        &selection(),
        "app",
        &"01".repeat(32),
        None,
        None,
    )
    .unwrap();
    plan.action = crate::PlanAction::UpgradeChild {
        target: crate::UpgradeTarget {
            canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
            module: "02".repeat(32),
        },
        owner: plan.management().unwrap().clone(),
    };
    let evidence_dir = dir.join("evidence");
    let staged = evidence_dir.join("stage/reply.bin");
    let observed = evidence_dir.join("observe/0/reply.bin");
    std::fs::create_dir_all(staged.parent().unwrap()).unwrap();
    std::fs::create_dir_all(observed.parent().unwrap()).unwrap();
    let installed = wire::Status {
        approval: Some(upgrade::approval(&plan).unwrap()),
        phase: wire::Phase::InstallIssued,
        attempts: [1; 2],
        requests: [Some([3; 32]), Some([4; 32])],
        previous_version: Some(7),
        ..Default::default()
    };
    std::fs::write(
        staged,
        ic_wire::native::encode_response(wire::STAGE, &Ok::<_, Refusal>(&installed), LIMITS)
            .unwrap(),
    )
    .unwrap();
    let ready = wire::Status {
        phase: wire::Phase::Ready,
        observed_version: Some(8),
        ..installed.clone()
    };
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
        std::fs::write(
            &observed,
            ic_wire::native::encode_response(wire::STATUS, &invalid, LIMITS).unwrap(),
        )
        .unwrap();
        assert!(
            upgrade::evidence(&evidence_dir, &plan).is_err(),
            "case {kind}"
        );
    }
    std::fs::write(
        &observed,
        ic_wire::native::encode_response(wire::STATUS, &ready, LIMITS).unwrap(),
    )
    .unwrap();
    assert_eq!(
        upgrade::evidence(&evidence_dir, &plan).unwrap(),
        Some(ready)
    );
    let regression = evidence_dir.join("observe/1/reply.bin");
    std::fs::create_dir_all(regression.parent().unwrap()).unwrap();
    std::fs::write(
        regression,
        ic_wire::native::encode_response(wire::STATUS, &installed, LIMITS).unwrap(),
    )
    .unwrap();
    assert!(upgrade::evidence(&evidence_dir, &plan).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stored_child_binds_release_slot_and_retained_recovery_without_inline_bytes() {
    use canic_management_protocol::{children, stored as wire};
    use canic_wasm_store_protocol::{Manifest, Release};
    let dir = directory();
    let wasm = dir.join("app.wasm");
    let bytes = b"\0asm\x01\0\0\0";
    std::fs::write(&wasm, bytes).unwrap();
    let (_, mut plan) = create_managed_plan(
        &dir,
        &wasm,
        input(),
        &selection(),
        "app",
        &"01".repeat(32),
        None,
        None,
    )
    .unwrap();
    let source = crate::StoredArtifact {
        store: crate::UpgradeTarget {
            canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
            module: "02".repeat(32),
        },
        manifest: Manifest {
            release: Release {
                name: "app".into(),
                version: "v1".into(),
            },
            hash: canic_wasm_store_protocol::digest(bytes),
            bytes: bytes.len() as u64,
            chunks: vec![canic_wasm_store_protocol::digest(bytes)],
        },
    };
    let crate::PlanAction::CreateChild(owner) = &mut plan.action else {
        panic!("creation")
    };
    owner.source = Some(source.clone());
    plan.validate().unwrap();
    let digest = plan.digest().unwrap();
    let call = stage_call(&plan, bytes).unwrap();
    assert_eq!(call.method, "stage_stored_child");
    let request: children::Request<wire::Stage<Approval>> =
        ic_wire::native::decode_request(wire::STAGE, &call.args, wire::LIMITS).unwrap();
    assert_eq!(request.slot, children::Slot::First);
    assert_eq!(request.value.reference.manifest, source.manifest);
    let recovery = recovery::call(&plan).unwrap();
    let recovered: children::Request<wire::Stage<Approval>> =
        ic_wire::native::decode_request(wire::RECONCILE, &recovery.args, wire::LIMITS).unwrap();
    assert_eq!(recovered.value, request.value);
    assert_eq!(
        observation(&plan).unwrap().method,
        "observe_stored_children"
    );
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
        children: [
            canic_management_protocol::custody::Custody {
                creation: status.clone(),
                upgrades: vec![],
            },
            canic_management_protocol::custody::Custody {
                creation: Status::default(),
                upgrades: vec![],
            },
        ],
        sources: [
            Some(wire::Sources {
                creation: delivery.clone(),
                upgrades: vec![],
            }),
            None,
        ],
    };
    let encode = |value: &wire::Custody| {
        ic_wire::native::encode_response(wire::CUSTODY, value, wire::LIMITS).unwrap()
    };
    assert_eq!(stored::creation(&plan, &encode(&custody)).unwrap(), status);
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
    custody.sources[0].as_mut().unwrap().creation = delivery;
    custody.sources[0].as_mut().unwrap().creation.operation = request
        .value
        .pin(wire::STAGE, children::Slot::Second)
        .operation;
    assert!(stored::creation(&plan, &encode(&custody)).is_err());
    let crate::PlanAction::CreateChild(owner) = &mut plan.action else {
        panic!("creation")
    };
    owner.source.as_mut().unwrap().manifest.release.version = "v2".into();
    assert_ne!(plan.digest().unwrap(), digest);
    let crate::PlanAction::CreateChild(owner) = &mut plan.action else {
        panic!("creation")
    };
    owner.source.as_mut().unwrap().manifest.hash[0] ^= 1;
    assert!(plan.validate().is_err());
    let mut owner = plan.management().unwrap().clone();
    owner.source = Some(source);
    plan.action = crate::PlanAction::UpgradeChild {
        owner,
        target: crate::UpgradeTarget {
            canister: "r7inp-6aaaa-aaaaa-aaabq-cai".into(),
            module: "03".repeat(32),
        },
    };
    plan.validate().unwrap();
    let call = Method::StageUpgrade.call(&plan, &(), LIMITS).unwrap();
    let expected: children::Request<wire::Stage<canic_management_protocol::upgrade::Approval>> =
        ic_wire::native::decode_request(wire::STAGE_UPGRADE, &call.args, wire::LIMITS).unwrap();
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
    custody.sources[0].as_mut().unwrap().upgrades[0]
        .reference
        .store = "rrkah-fqaaa-aaaaa-aaaaq-cai".into();
    assert!(stored::upgraded(&plan, &encode(&custody)).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}
