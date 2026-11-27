mod installed;

use super::*;
use crate::{
    DeploymentName, create_plan,
    tests::{directory, input},
};
use canic_management_protocol::{
    Status,
    custody::{self, Custody},
};

fn save_pages(dir: &Path, plan: &Plan, side: &str) {
    let full = stored::expected(plan).unwrap();
    for (i, key) in full.keys().enumerate() {
        let page = application::CustodyPage {
            revision: (5, 7),
            key,
            item: full.item(key),
        };
        let bytes = ic_wire::encode(
            &page,
            ic_wire::Encoding::Frame {
                method: application::CUSTODY_PAGE,
                direction: ic_wire::Direction::Response,
            },
            application::LIMITS,
        )
        .unwrap();
        store::put(&page_path(&dir.join(side), i).join("reply.bin"), &bytes).unwrap();
    }
}

#[test]
fn paged_preflight_requires_every_reply_at_one_revision_in_order() {
    let (dir, plan) = fixture();
    save_pages(&dir, &plan, BEFORE);
    assert!(retained_at(&dir, &plan, false).unwrap());
    let full = stored::expected(&plan).unwrap();
    let child_path = page_path(&dir.join(BEFORE), 1).join("reply.bin");
    let child = std::fs::read(&child_path).unwrap();
    let key = full.keys().nth(1).unwrap();
    assert!(matches!(
        check_page(&full, key, &child, &mut Some((5, 8))),
        Err(Error::Blocked(_))
    ));
    let inventory = full.keys().next().unwrap();
    assert!(matches!(
        check_page(&full, inventory, &child, &mut None),
        Err(Error::Blocked(_))
    ));
    std::fs::remove_file(child_path).unwrap();
    assert!(!retained_at(&dir, &plan, false).unwrap());
}

fn fixture() -> (std::path::PathBuf, Plan) {
    let dir = directory();
    let wasm = dir.join("root.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (_, mut plan) = create_plan(&dir.join("journal"), &wasm, input()).unwrap();
    plan.version = 1;
    plan.deployment = Some(DeploymentName {
        app: "demo".into(),
        fleet: "mainnet".into(),
        canister: "root".into(),
    });
    plan.action = action();
    plan.validate().unwrap();
    (dir, plan)
}

pub(crate) fn action() -> crate::PlanAction {
    use canic_management_protocol::stored as wire;
    let children = vec![Custody {
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
    }];
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
    let store_id = stage.reference.store.clone();
    let mut sources = vec![None; children.len()];
    sources[0] = Some(wire::Sources {
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
    });
    let store = Box::new(crate::StoreMaintenance {
        history: Vec::new(),
        operation: None,
        store: crate::UpgradeTarget {
            canister: store_id,
            module: "22".repeat(32),
        },
        sources,
    });
    crate::PlanAction::MaintainRoot {
        target: crate::UpgradeTarget {
            canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            module: "11".repeat(32),
        },
        children,
        store,
    }
}

#[test]
fn store_maintenance_preserves_exact_pins_sources_and_empty_slots() {
    use canic_management_protocol::stored as wire;
    let (dir, plan) = fixture();
    plan.validate().unwrap();
    assert_eq!(
        serde_json::from_value::<Plan>(serde_json::to_value(&plan).unwrap()).unwrap(),
        plan
    );
    let serialized = serde_json::to_value(&plan).unwrap();
    for old in ["canic:root-maintenance.v1", "canic:deployment-upgrade.v1"] {
        let mut prior = serialized.clone();
        prior["action"]["contract"] = old.into();
        assert!(serde_json::from_value::<Plan>(prior).is_err());
    }
    let mut missing = serialized;
    missing["action"]["approval"]
        .as_object_mut()
        .unwrap()
        .remove("store");
    assert!(serde_json::from_value::<Plan>(missing).is_err());
    let observed = wire::Custody {
        children: plan.maintenance().unwrap().to_vec(),
        sources: plan.maintenance_store().unwrap().sources.clone(),
    };
    let frame = |v: &wire::Custody| {
        crate::root_custody::frame(
            custody::Root {
                stored: v.clone(),
                store_operation: None,
            },
            vec![],
        )
    };
    assert!(matches_plan(&plan, "observe_root_custody", &frame(&observed)).unwrap());
    let frame_domain = |direction| ic_wire::Encoding::Frame {
        method: application::CUSTODY,
        direction,
    };
    let mut unexpected: application::Custody = ic_wire::decode(
        &frame(&observed),
        frame_domain(ic_wire::Direction::Response),
        application::LIMITS,
    )
    .unwrap();
    unexpected.children[0].lifecycle = Some(application::Lifecycle {
        creation: application::Arguments::new(vec![42]).unwrap(),
        upgrades: vec![],
    });
    let bytes = ic_wire::encode(
        &unexpected,
        frame_domain(ic_wire::Direction::Response),
        application::LIMITS,
    )
    .unwrap();
    assert!(!matches_plan(&plan, "observe_root_custody", &bytes).unwrap());
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
            3 => changed.children[0].upgrades.push(Default::default()),
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
        assert!(!matches_plan(&plan, "observe_root_custody", &frame(&changed)).unwrap());
    }
    save_pages(&dir, &plan, "custody-before");
    require_before(&dir, &plan).unwrap();
    assert!(!retained(&dir, &plan).unwrap());
    save_pages(&dir, &plan, "custody-after");
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
    let crate::PlanAction::MaintainRoot { store: full, .. } = &mut full_plan.action else {
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
    let full_frame = |value: &custody::Root| crate::root_custody::frame(value.clone(), vec![]);
    assert!(matches_plan(&full_plan, "observe_root_custody", &full_frame(&complete)).unwrap());
    assert!(observation_args(&full_plan, "observe_stored_children").is_err());
    assert!(matches_plan(&full_plan, "observe_stored_children", &frame(&observed)).is_err());
    assert!(observation_args(&plan, "observe_root_history").is_err());
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
        save_pages(&dir.join("complete"), &full_plan, side);
    }
    assert!(retained(&dir.join("complete"), &full_plan).unwrap());
    complete_history_requires_every_retained_predecessor(&full_plan, &complete);
    let crate::PlanAction::MaintainRoot { store: full, .. } = &mut full_plan.action else {
        unreachable!()
    };
    full.operation = None;
    assert_eq!(full_plan.digest().unwrap(), child_only_digest);
    for case in 0..5 {
        let mut invalid = plan.clone();
        let crate::PlanAction::MaintainRoot { store, .. } = &mut invalid.action else {
            panic!("stored maintenance");
        };
        let source = &mut store.sources[0].as_mut().unwrap().creation;
        match case {
            0 => source.operation[0] ^= 1,
            1 => source.state = wire::PinState::Issued,
            2 => source.reference.store = "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
            3 => store.sources.push(store.sources[0].clone()),
            _ => {
                store.sources.pop();
            }
        }
        assert!(invalid.validate().is_err());
    }
}

fn complete_history_requires_every_retained_predecessor(plan: &Plan, previous: &custody::Root) {
    use canic_management_protocol::store_maintenance as wire;
    let mut first = previous.store_operation.clone().unwrap();
    first.phase = wire::Phase::Ready;
    let scan = wire::Preservation {
        after_removed: false,
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
    let crate::PlanAction::MaintainRoot { store, .. } = &mut plan.action else {
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
    assert!(observation_args(&plan, "observe_root_history").is_err());
    let frame = |value: &custody::RootHistory| {
        crate::root_custody::frame(value.current.clone(), value.store_history.clone())
    };
    assert!(matches_plan(&plan, "observe_root_custody", &frame(&complete)).unwrap());
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
        assert!(!matches_plan(&plan, "observe_root_custody", &frame(&changed)).unwrap());
    }
    for case in 0..5 {
        let mut changed = plan.clone();
        let crate::PlanAction::MaintainRoot { store, .. } = &mut changed.action else {
            unreachable!()
        };
        match case {
            0 => store.history[0].phase = wire::Phase::InstallIssued,
            1 => store.operation.as_mut().unwrap().approval.attempt.sequence = 3,
            2 => store.history[0].approval.module_hash = [9; 32],
            3 => store.history.clear(),
            _ => store.history = vec![store.history[0].clone(); 8],
        }
        assert!(changed.validate().is_err());
    }
}
