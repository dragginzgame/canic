use crate::{application::*, children::ChildId, custody, stored, upgrade};

#[test]
fn maximum_application_history_fits_and_incomplete_effect_custody_refuses() {
    let arguments = Arguments::new(vec![255; MAX_ARGUMENT_BYTES]).unwrap();
    let mut maintenance = Maintenance {
        operator: "x".repeat(63),
        arguments: arguments.clone(),
        attempt: 1,
        phase: MaintenancePhase::Active,
        requests: [Some([1; 32]); 4],
        before: Some(arguments.clone()),
        after: Some(arguments.clone()),
        activator: Some("x".repeat(63)),
    };
    assert!(maintenance.valid());
    let lifecycle = Lifecycle {
        creation: arguments,
        upgrades: vec![maintenance.clone(); upgrade::MAX_UPGRADES],
    };
    let value = Custody {
        root: custody::RootHistory {
            current: custody::Root {
                stored: stored::Custody {
                    children: (0..8)
                        .map(|_| custody::Custody {
                            creation: Default::default(),
                            upgrades: vec![],
                        })
                        .collect(),
                    sources: vec![None; 8],
                },
                store_operation: None,
            },
            store_history: vec![],
        },
        applications: (0..8).map(|_| Some(lifecycle.clone())).collect(),
    };
    let encoding = ic_wire::Encoding::Frame {
        method: CUSTODY,
        direction: ic_wire::Direction::Response,
    };
    let bytes = ic_wire::encode(&value, encoding, LIMITS).unwrap();
    assert_eq!(
        ic_wire::decode::<Custody>(&bytes, encoding, LIMITS).unwrap(),
        value
    );
    maintenance.requests[3] = None;
    assert!(!maintenance.valid());
    maintenance.requests[3] = Some([1; 32]);
    maintenance.after = None;
    assert!(!maintenance.valid());
    maintenance.phase = MaintenancePhase::Inspected;
    maintenance.requests[3] = None;
    maintenance.after = maintenance.before.clone();
    assert!(
        !maintenance.valid(),
        "separate activation authority cannot appear before dispatch"
    );
    maintenance.activator = None;
    assert!(maintenance.valid());
}

#[test]
fn upgrade_pin_binds_arguments_attempt_and_permanent_slot() {
    let mut value = Upgrade {
        source: Stage {
            source: stored::Stage {
                approval: upgrade::Approval {
                    manager: "manager".into(),
                    child: "child".into(),
                    previous_module: [1; 32],
                    module_hash: [2; 32],
                    execute_at_ns: 10,
                    expires_at_ns: 20,
                },
                reference: stored::Reference {
                    store: "store".into(),
                    manifest: ic_wasm_store_protocol::Manifest {
                        release: ic_wasm_store_protocol::Release {
                            name: "catalog".into(),
                            version: "v2".into(),
                        },
                        hash: [2; 32],
                        bytes: 1,
                        chunks: vec![[2; 32]],
                    },
                },
            },
            arguments: Arguments::new(vec![7]).unwrap(),
        },
        attempt: 1,
    };
    let original = value.pin(ChildId::FIRST);
    assert_ne!(original, value.pin(ChildId::SECOND));
    value.attempt = 2;
    assert_ne!(original, value.pin(ChildId::FIRST));
    value.attempt = 1;
    value.source.arguments = Arguments::new(vec![8]).unwrap();
    assert_ne!(original, value.pin(ChildId::FIRST));
}
