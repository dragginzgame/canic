use super::*;
use crate::{
    CanisterConfig, FleetConfig, Network,
    tests::{directory, input},
};
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
            Some(1)
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
            None
        ),
        Err(Error::Blocked(_))
    ));
    let mut other = selected;
    other.app = "another".into();
    assert!(matches!(
        create_managed_plan(&dir, &wasm, input(), &other, "app", &"01".repeat(32), None),
        Err(Error::Blocked(_))
    ));
    let mut invalid = plan.clone();
    invalid.creation_cycles += 1;
    assert!(invalid.digest().is_err());
    let mut invalid = plan.clone();
    invalid.version = 1;
    invalid.deployment = None;
    assert!(invalid.digest().is_err());
    let mut invalid = plan.clone();
    invalid.management.as_mut().unwrap().root = "2vxsx-fae".into();
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
    let (created_id, created) =
        create_managed_plan(&dir, &wasm, input(), &selected, "app", &root_hash, None).unwrap();
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
            create_managed_plan(&dir, &wasm, invalid, &selected, "app", &root_hash, None).is_err()
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
    )
    .unwrap();
    assert_ne!(id, created_id);
    assert_eq!(plan.target.as_deref(), Some(child));
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
            Some(1)
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
    )
    .unwrap();
    plan.target = Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into());
    plan.previous_module = Some("02".repeat(32));
    plan.creation_cycles = 0;
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
