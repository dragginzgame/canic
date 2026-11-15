use super::*;
use crate::{
    DeploymentName, create_plan,
    tests::{directory, input},
};

fn fixture() -> (std::path::PathBuf, Plan) {
    let dir = directory();
    let wasm = dir.join("root.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (_, mut plan) = create_plan(&dir.join("journal"), &wasm, input()).unwrap();
    plan.version = 2;
    plan.deployment = Some(DeploymentName {
        app: "demo".into(),
        fleet: "mainnet".into(),
        canister: "root".into(),
    });
    plan.action = crate::PlanAction::MaintainRoot {
        target: crate::UpgradeTarget {
            canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            module: SINGLE_UPGRADE_ROOT.into(),
        },
        children: vec![Custody {
            creation: Status {
                approval: Some(canic_management_protocol::Approval {
                    manager: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
                    module_hash: [1; 32],
                    creation_cycles: canic_management_protocol::CREATION_CYCLES,
                    execute_at_ns: 1,
                    expires_at_ns: 2,
                }),
                worker: Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into()),
                phase: Phase::Ready,
                ..Default::default()
            },
            upgrades: vec![],
        }],
    };
    plan.validate().unwrap();
    (dir, plan)
}

#[test]
fn predecessor_requires_both_observations_and_current_postflight() {
    let (dir, plan) = fixture();
    store::put_json(&dir.join("plan.json"), &plan).unwrap();
    store::put(
        &dir.join("module.wasm"),
        &std::fs::read(dir.join("root.wasm")).unwrap(),
    )
    .unwrap();
    let id = plan.digest().unwrap();
    let status = || operation::status_at(&dir, &id).unwrap();
    assert_eq!(
        status()["effects"],
        serde_json::json!({
            "custody-before-creation": "not_submitted",
            "custody-before-upgrade": "not_submitted",
            "install": "not_submitted",
            "custody-after": "not_submitted",
        })
    );
    let expected = &plan.maintenance().unwrap()[0];
    assert!(!retained(&dir, &plan).unwrap());
    let mut creation = expected.creation.clone();
    creation.stable_pages = 18;
    store::put(
        &dir.join("custody-before-creation/reply.bin"),
        &ic_wire::native::encode_response(STATUS, &creation, LIMITS).unwrap(),
    )
    .unwrap();
    assert!(require_before(&dir, &plan).is_err());
    assert_eq!(
        status()["effects"]["custody-before-creation"],
        "reply_retained"
    );
    assert_eq!(
        status()["effects"]["custody-before-upgrade"],
        "not_submitted"
    );
    let empty = upgrade::Status::default();
    store::put(
        &dir.join("custody-before-upgrade/reply.bin"),
        &ic_wire::native::encode_response(upgrade::STATUS, &empty, LIMITS).unwrap(),
    )
    .unwrap();
    require_before(&dir, &plan).unwrap();
    assert!(!retained(&dir, &plan).unwrap());
    store::put(
        &dir.join("custody-after/reply.bin"),
        &ic_wire::native::encode_response(custody::OBSERVE, expected, custody::LIMITS).unwrap(),
    )
    .unwrap();
    assert!(retained(&dir, &plan).unwrap());
    assert_eq!(
        status()["effects"],
        serde_json::json!({
            "custody-before-creation": "reply_retained",
            "custody-before-upgrade": "reply_retained",
            "install": "not_submitted",
            "custody-after": "reply_retained",
        })
    );
    assert!(status()["receipt"].is_null());
    std::fs::remove_file(dir.join("custody-before-creation/reply.bin")).unwrap();
    assert!(!retained(&dir, &plan).unwrap());
}

#[test]
fn predecessor_refuses_unknown_code_extra_history_and_foreign_reads() {
    let (_, mut plan) = fixture();
    for method in ["observe", "observe_upgrade", "observe_custody"] {
        observation_args(&plan, method).unwrap();
    }
    assert!(observation_args(&plan, "stage_upgrade").is_err());
    let crate::PlanAction::MaintainRoot { target, .. } = &mut plan.action else {
        panic!("maintenance")
    };
    target.module = "ff".repeat(32);
    for method in ["observe", "observe_upgrade"] {
        assert!(observation_args(&plan, method).is_err());
    }
    assert_eq!(
        observations(&plan, false),
        &[("custody-before", "observe_custody")]
    );
    let crate::PlanAction::MaintainRoot { target, children } = &mut plan.action else {
        panic!("maintenance")
    };
    target.module = SINGLE_UPGRADE_ROOT.into();
    children[0].upgrades = vec![upgrade::Status::default(); 2];
    assert!(plan.validate().is_err());
}

#[test]
fn predecessor_compares_complete_creation_and_single_upgrade_custody() {
    let (_, plan) = fixture();
    let mut expected = plan.maintenance().unwrap()[0].clone();
    let mut changed = expected.creation.clone();
    changed.requests[0] = Some([9; 32]);
    assert!(
        !matches(
            &expected,
            "observe",
            &ic_wire::native::encode_response(STATUS, &changed, LIMITS).unwrap()
        )
        .unwrap()
    );
    let admitted = upgrade::Status {
        phase: upgrade::Phase::Scheduled,
        ..Default::default()
    };
    assert!(
        !matches(
            &expected,
            "observe_upgrade",
            &ic_wire::native::encode_response(upgrade::STATUS, &admitted, LIMITS).unwrap()
        )
        .unwrap()
    );
    let completed = upgrade::Status {
        approval: Some(upgrade::Approval {
            manager: expected.creation.approval.as_ref().unwrap().manager.clone(),
            child: expected.creation.worker.clone().unwrap(),
            previous_module: [1; 32],
            module_hash: [2; 32],
            execute_at_ns: 3,
            expires_at_ns: 4,
        }),
        phase: upgrade::Phase::Ready,
        attempts: [1; 2],
        requests: [Some([3; 32]); 2],
        previous_version: Some(1),
        observed_version: Some(2),
        failure: None,
    };
    expected.upgrades.push(completed.clone());
    assert!(
        matches(
            &expected,
            "observe_upgrade",
            &ic_wire::native::encode_response(upgrade::STATUS, &completed, LIMITS).unwrap()
        )
        .unwrap()
    );
    let mut changed = completed;
    changed.requests[1] = Some([4; 32]);
    assert!(
        !matches(
            &expected,
            "observe_upgrade",
            &ic_wire::native::encode_response(upgrade::STATUS, &changed, LIMITS).unwrap()
        )
        .unwrap()
    );
}
