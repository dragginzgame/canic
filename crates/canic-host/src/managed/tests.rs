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
