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
        store: None,
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
fn store_maintenance_preserves_exact_pins_sources_and_empty_slots() {
    use canic_management_protocol::stored as wire;
    let (dir, mut plan) = fixture();
    let crate::PlanAction::MaintainRoot {
        target,
        children,
        store,
    } = &mut plan.action
    else {
        panic!("maintenance");
    };
    target.module = "11".repeat(32);
    let reference = wire::Reference {
        store: "r7inp-6aaaa-aaaaa-aaabq-cai".into(),
        manifest: ic_wasm_store_protocol::Manifest {
            release: ic_wasm_store_protocol::Release {
                name: "app".into(),
                version: "v1".into(),
            },
            hash: [1; 32],
            bytes: 8,
            chunks: vec![[2; 32]],
        },
    };
    let stage = wire::Stage {
        approval: children[0].creation.approval.clone().unwrap(),
        reference,
    };
    *store = Some(Box::new(crate::StoreMaintenance {
        history: Vec::new(),
        operation: None,
        store: crate::UpgradeTarget {
            canister: stage.reference.store.clone(),
            module: "22".repeat(32),
        },
        sources: [
            Some(wire::Sources {
                creation: wire::Delivery {
                    operation: stage
                        .pin(
                            wire::STAGE,
                            canic_management_protocol::children::ChildId::FIRST,
                        )
                        .operation,
                    reference: stage.reference,
                    state: wire::PinState::Retained,
                    cache_deadline: Some(2),
                },
                upgrades: Vec::new(),
            }),
            None,
        ],
    }));
    plan.validate().unwrap();
    assert_eq!(
        serde_json::from_value::<Plan>(serde_json::to_value(&plan).unwrap()).unwrap(),
        plan
    );
    let observed = wire::Custody {
        children: [
            plan.maintenance().unwrap()[0].clone(),
            Custody {
                creation: Status::default(),
                upgrades: vec![],
            },
        ],
        sources: plan.maintenance_store().unwrap().sources.clone(),
    };
    let frame = |v: &wire::Custody| {
        ic_wire::encode(
            v,
            ic_wire::Encoding::Frame {
                method: wire::CUSTODY,
                direction: ic_wire::Direction::Response,
            },
            wire::LIMITS,
        )
        .unwrap()
    };
    assert!(matches_plan(&plan, "observe_stored_children", &frame(&observed)).unwrap());
    for method in [
        "observe",
        "observe_upgrade",
        "observe_children",
        "observe_custody",
    ] {
        assert!(observation_args(&plan, method).is_err());
        assert!(matches_plan(&plan, method, &frame(&observed)).is_err());
    }
    for case in 0..5 {
        let mut changed = observed.clone();
        match case {
            0 => changed.sources[0].as_mut().unwrap().creation.operation[0] ^= 1,
            1 => changed.sources[0].as_mut().unwrap().creation.cache_deadline = Some(3),
            2 => changed.sources[0] = None,
            3 => changed.children[1] = changed.children[0].clone(),
            _ => {
                changed.sources[0]
                    .as_mut()
                    .unwrap()
                    .creation
                    .reference
                    .manifest
                    .release
                    .version = "v2".into()
            }
        }
        assert!(!matches_plan(&plan, "observe_stored_children", &frame(&changed)).unwrap());
    }
    store::put(&dir.join("custody-before/reply.bin"), &frame(&observed)).unwrap();
    require_before(&dir, &plan).unwrap();
    assert!(!retained(&dir, &plan).unwrap());
    store::put(&dir.join("custody-after/reply.bin"), &frame(&observed)).unwrap();
    assert!(retained(&dir, &plan).unwrap());
    let child_only_digest = plan.digest().unwrap();
    let operation = canic_management_protocol::store_maintenance::Status {
        approval: canic_management_protocol::store_maintenance::Approval {
            store: plan.maintenance_store().unwrap().store.canister.clone(),
            previous_module: [0x22; 32],
            module_hash: [0x33; 32],
            attempt: ic_wasm_store_protocol::MaintenanceAttempt {
                sequence: 1,
                operation: [3; 32],
            },
            expires_at_ns: 2,
        },
        operator: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        phase: canic_management_protocol::store_maintenance::Phase::Reserved,
        stop_request: None,
        failure: None,
        preservation: None,
        installation: None,
    };
    let mut full_plan = plan.clone();
    let crate::PlanAction::MaintainRoot {
        store: Some(full), ..
    } = &mut full_plan.action
    else {
        unreachable!()
    };
    full.operation = Some(operation.clone());
    full_plan.validate().unwrap();
    assert_ne!(full_plan.digest().unwrap(), child_only_digest);
    assert_eq!(
        serde_json::from_slice::<Plan>(&serde_json::to_vec(&full_plan).unwrap()).unwrap(),
        full_plan
    );
    let complete = custody::Root {
        stored: observed.clone(),
        store_operation: Some(operation),
    };
    let full_frame = |value: &custody::Root| {
        ic_wire::encode(
            value,
            ic_wire::Encoding::Frame {
                method: custody::ROOT,
                direction: ic_wire::Direction::Response,
            },
            custody::ROOT_LIMITS,
        )
        .unwrap()
    };
    assert!(matches_plan(&full_plan, "observe_root_custody", &full_frame(&complete)).unwrap());
    assert!(observation_args(&full_plan, "observe_stored_children").is_err());
    assert!(matches_plan(&full_plan, "observe_stored_children", &frame(&observed)).is_err());
    assert!(observation_args(&plan, "observe_root_custody").is_err());
    assert!(!retained(&dir, &full_plan).unwrap_or(false)); // Old child-only replies cannot authorize full custody.
    for case in 0..5 {
        let mut changed = complete.clone();
        match case {
            0 => changed.store_operation = None,
            1 => {
                changed
                    .store_operation
                    .as_mut()
                    .unwrap()
                    .approval
                    .attempt
                    .operation[0] ^= 1
            }
            2 => {
                changed
                    .store_operation
                    .as_mut()
                    .unwrap()
                    .approval
                    .expires_at_ns += 1
            }
            3 => changed.store_operation.as_mut().unwrap().operator = "2vxsx-fae".into(),
            _ => changed.stored.sources[0] = None,
        }
        assert!(!matches_plan(&full_plan, "observe_root_custody", &full_frame(&changed)).unwrap());
    }
    for side in ["custody-before", "custody-after"] {
        store::put(
            &dir.join("complete").join(side).join("reply.bin"),
            &full_frame(&complete),
        )
        .unwrap();
    }
    assert!(retained(&dir.join("complete"), &full_plan).unwrap());
    complete_history_requires_every_retained_predecessor(&full_plan, &complete);
    let crate::PlanAction::MaintainRoot {
        store: Some(full), ..
    } = &mut full_plan.action
    else {
        unreachable!()
    };
    full.operation = None;
    assert_eq!(full_plan.digest().unwrap(), child_only_digest);
    for case in 0..4 {
        let mut invalid = plan.clone();
        let crate::PlanAction::MaintainRoot {
            store: Some(store), ..
        } = &mut invalid.action
        else {
            panic!("stored maintenance");
        };
        let source = &mut store.sources[0].as_mut().unwrap().creation;
        match case {
            0 => source.operation[0] ^= 1,
            1 => source.state = wire::PinState::Issued,
            2 => source.reference.store = "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
            _ => store.sources[1] = store.sources[0].clone(),
        }
        assert!(invalid.validate().is_err());
    }
}

fn complete_history_requires_every_retained_predecessor(plan: &Plan, previous: &custody::Root) {
    use canic_management_protocol::store_maintenance as wire;
    let mut first = previous.store_operation.clone().unwrap();
    first.phase = wire::Phase::Ready;
    let scan = wire::Preservation {
        catalog: ic_wasm_store_protocol::preservation::Catalog {
            quota: ic_wasm_store_protocol::Quota {
                reserved_bytes: 8,
                releases: 1,
            },
            reserved_bytes: 8,
            releases: 1,
            artifacts: 1,
            chunks: 1,
        },
        after: None,
        observed: 1,
        digest: [7; 32],
        complete: true,
    };
    first.preservation = Some(scan.clone());
    first.installation = Some(wire::Installation {
        upload_request: [1; 32],
        request: Some([2; 32]),
        sender_version: Some(1),
        start_request: Some([3; 32]),
        verification: Some(scan),
        activation_request: Some([4; 32]),
    });
    let mut next = first.clone();
    next.approval.previous_module = first.approval.module_hash;
    next.approval.module_hash = [0x44; 32];
    next.approval.attempt.sequence = 2;
    next.approval.attempt.operation = [4; 32];
    let complete = custody::RootHistory {
        current: custody::Root {
            stored: previous.stored.clone(),
            store_operation: Some(next.clone()),
        },
        store_history: vec![first.clone()],
    };
    let mut plan = plan.clone();
    let crate::PlanAction::MaintainRoot {
        store: Some(store), ..
    } = &mut plan.action
    else {
        unreachable!()
    };
    store.store.module = "44".repeat(32);
    store.operation = Some(next);
    store.history = vec![first];
    plan.validate().unwrap();
    assert_eq!(
        serde_json::from_slice::<Plan>(&serde_json::to_vec(&plan).unwrap()).unwrap(),
        plan
    );
    assert_eq!(observations(&plan, false)[0].1, "observe_root_history");
    assert!(observation_args(&plan, "observe_root_custody").is_err());
    let frame = |value: &custody::RootHistory| {
        ic_wire::encode(
            value,
            ic_wire::Encoding::Frame {
                method: custody::ROOT_HISTORY,
                direction: ic_wire::Direction::Response,
            },
            custody::ROOT_HISTORY_LIMITS,
        )
        .unwrap()
    };
    assert!(matches_plan(&plan, "observe_root_history", &frame(&complete)).unwrap());
    for case in 0..4 {
        let mut changed = complete.clone();
        match case {
            0 => changed.store_history.clear(),
            1 => changed.store_history[0].approval.attempt.operation[0] ^= 1,
            2 => changed.store_history[0].approval.expires_at_ns += 1,
            _ => {
                changed.store_history[0]
                    .installation
                    .as_mut()
                    .unwrap()
                    .request = None
            }
        }
        assert!(!matches_plan(&plan, "observe_root_history", &frame(&changed)).unwrap());
    }
    for case in 0..4 {
        let mut changed = plan.clone();
        let crate::PlanAction::MaintainRoot {
            store: Some(store), ..
        } = &mut changed.action
        else {
            unreachable!()
        };
        match case {
            0 => store.history[0].phase = wire::Phase::InstallIssued,
            1 => store.operation.as_mut().unwrap().approval.attempt.sequence = 3,
            2 => store.history[0].approval.module_hash = [9; 32],
            _ => store.history = vec![store.history[0].clone(); wire::MAX_UPGRADES],
        }
        assert!(changed.validate().is_err());
    }
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
    let status = || serde_json::to_value(operation::status_at(&dir, &id).unwrap()).unwrap();
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
        &ic_wire::encode(
            &creation,
            ic_wire::Encoding::Frame {
                method: STATUS,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap(),
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
        &ic_wire::encode(
            &empty,
            ic_wire::Encoding::Frame {
                method: upgrade::STATUS,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap(),
    )
    .unwrap();
    require_before(&dir, &plan).unwrap();
    assert!(!retained(&dir, &plan).unwrap());
    store::put(
        &dir.join("custody-after/reply.bin"),
        &ic_wire::encode(
            expected,
            ic_wire::Encoding::Frame {
                method: custody::OBSERVE,
                direction: ic_wire::Direction::Response,
            },
            custody::LIMITS,
        )
        .unwrap(),
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
    let crate::PlanAction::MaintainRoot {
        target, children, ..
    } = &mut plan.action
    else {
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
            &ic_wire::encode(
                &changed,
                ic_wire::Encoding::Frame {
                    method: STATUS,
                    direction: ic_wire::Direction::Response
                },
                LIMITS
            )
            .unwrap()
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
            &ic_wire::encode(
                &admitted,
                ic_wire::Encoding::Frame {
                    method: upgrade::STATUS,
                    direction: ic_wire::Direction::Response
                },
                LIMITS
            )
            .unwrap()
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
            &ic_wire::encode(
                &completed,
                ic_wire::Encoding::Frame {
                    method: upgrade::STATUS,
                    direction: ic_wire::Direction::Response
                },
                LIMITS
            )
            .unwrap()
        )
        .unwrap()
    );
    let mut changed = completed;
    changed.requests[1] = Some([4; 32]);
    assert!(
        !matches(
            &expected,
            "observe_upgrade",
            &ic_wire::encode(
                &changed,
                ic_wire::Encoding::Frame {
                    method: upgrade::STATUS,
                    direction: ic_wire::Direction::Response
                },
                LIMITS
            )
            .unwrap()
        )
        .unwrap()
    );
}
