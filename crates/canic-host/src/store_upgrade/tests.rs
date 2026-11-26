use super::*;
use crate::{Artifact, DeploymentName, Network, PlanAction, UpgradeTarget};

fn plan() -> Plan {
    Plan {
        version: 2,
        network: Network::Mainnet,
        controller: crate::tests::input().controller,
        artifact: Artifact {
            sha256: "33".repeat(32),
            bytes: 8,
        },
        action: PlanAction::UpgradeStore {
            sequence: None,
            target: UpgradeTarget {
                canister: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
                module: "22".repeat(32),
            },
            root: UpgradeTarget {
                canister: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
                module: "11".repeat(32),
            },
        },
        created_at_ns: 1,
        expires_at_ns: 600_000_000_001,
        deployment: Some(DeploymentName {
            app: "demo".into(),
            fleet: "mainnet".into(),
            canister: "store".into(),
        }),
    }
}

#[test]
fn store_approval_binds_code_owner_operation_and_original_deadline() {
    let plan = plan();
    plan.validate().unwrap();
    let approval = approval(&plan).unwrap();
    assert_eq!(approval.attempt.sequence, 1);
    assert_eq!(
        crate::hex(&approval.attempt.operation),
        plan.digest().unwrap()
    );
    assert_eq!(approval.expires_at_ns, plan.expires_at_ns);
    assert_eq!(approval.previous_module, [0x22; 32]);
    assert_eq!(approval.module_hash, [0x33; 32]);
    let encoded = serde_json::to_value(&plan).unwrap();
    let decoded: Plan = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(decoded.digest().unwrap(), plan.digest().unwrap());
    for (field, value) in [
        ("store_root", encoded["store_upgrade_root"].clone()),
        ("creation_cycles", serde_json::json!(1)),
        ("target", serde_json::Value::Null),
        ("previous_module", serde_json::Value::Null),
        ("version", serde_json::json!(3)),
    ] {
        let mut changed = encoded.clone();
        changed[field] = value;
        assert!(serde_json::from_value::<Plan>(changed).is_err(), "{field}");
    }
    let mut changed = plan.clone();
    changed.expires_at_ns += 1;
    assert_ne!(super::approval(&changed).unwrap().attempt, approval.attempt);
    changed = plan;
    let PlanAction::UpgradeStore { root, .. } = &mut changed.action else {
        unreachable!()
    };
    root.module = "44".repeat(32);
    assert_ne!(super::approval(&changed).unwrap().attempt, approval.attempt);
}

#[test]
fn unified_commands_bind_new_plans_and_leave_signed_predecessor_frames_intact() {
    let mut old = plan();
    old.version = 9;
    let mut current = old.clone();
    current.version = crate::plan::STORE_MAINTENANCE_VERSION;
    assert_ne!(old.digest().unwrap(), current.digest().unwrap());
    let action = wire::Request::Stage(approval(&current).unwrap());
    let call = command(&current, &action).unwrap();
    assert_eq!(call.method, "maintain_store");
    let original = command(&old, &wire::Request::Stage(approval(&old).unwrap())).unwrap();
    assert_eq!(original.method, "stage_store_maintenance");
    assert_ne!(call.args, original.args);
    assert_eq!(
        observation(&current).unwrap().method,
        "observe_store_maintenance"
    );
    assert_eq!(
        recovery(&current).unwrap().method,
        "recover_store_maintenance"
    );
}

#[test]
fn explicit_sequence_binds_exact_observation_without_rewriting_old_approvals() {
    let historical = plan();
    let old_bytes = serde_json::to_vec(&historical).unwrap();
    assert!(
        serde_json::to_value(&historical)
            .unwrap()
            .get("store_upgrade_sequence")
            .is_none()
    );
    let mut next = historical.clone();
    let PlanAction::UpgradeStore { sequence, .. } = &mut next.action else {
        unreachable!()
    };
    *sequence = Some(2);
    next.validate().unwrap();
    assert_ne!(next.digest().unwrap(), historical.digest().unwrap());
    let call = observation(&next).unwrap();
    assert_eq!(call.method, "observe_store_attempt");
    assert_eq!(
        ic_wire::decode::<wire::Approval>(
            &call.args,
            ic_wire::Encoding::Frame {
                method: wire::OBSERVE_ATTEMPT,
                direction: ic_wire::Direction::Request,
            },
            wire::LIMITS
        )
        .unwrap(),
        approval(&next).unwrap()
    );
    assert_eq!(approval(&next).unwrap().attempt.sequence, 2);
    let roundtrip: Plan = serde_json::from_slice(&serde_json::to_vec(&next).unwrap()).unwrap();
    assert_eq!(roundtrip, next);
    for invalid in [0, wire::MAX_UPGRADES as u64 + 1] {
        let PlanAction::UpgradeStore { sequence, .. } = &mut next.action else {
            unreachable!()
        };
        *sequence = Some(invalid);
        assert!(next.validate().is_err());
    }
    let restored: Plan = serde_json::from_slice(&old_bytes).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), old_bytes);
    assert_eq!(
        observation(&restored).unwrap().method,
        "observe_store_maintenance"
    );
}

#[test]
fn store_upgrade_refuses_anonymous_reflected_and_unbounded_authority() {
    for case in 0..5 {
        let mut plan = plan();
        let PlanAction::UpgradeStore { root, target, .. } = &mut plan.action else {
            unreachable!()
        };
        match case {
            0 => plan.controller = "2vxsx-fae".into(),
            1 => target.canister = root.canister.clone(),
            2 => target.module = plan.artifact.sha256.clone(),
            3 => root.canister = "aaaaa-aa".into(),
            _ => plan.artifact.bytes = wire::MAX_MODULE_BYTES + 1,
        }
        assert!(plan.validate().is_err(), "case {case}");
    }
}

#[test]
fn recovery_keeps_expired_effect_authority_and_cannot_select_advance() {
    let plan = plan(); // Deliberately expired; typed observations remain available.
    let read = observation(&plan).unwrap();
    assert_eq!(
        read.canister.to_text(),
        plan.store_upgrade_root().unwrap().canister
    );
    assert_eq!(read.method, "observe_store_maintenance");
    ic_wire::decode::<()>(
        &read.args,
        ic_wire::Encoding::Frame {
            method: wire::OBSERVE,
            direction: ic_wire::Direction::Request,
        },
        wire::LIMITS,
    )
    .unwrap();
    let recover = recovery(&plan).unwrap();
    assert_eq!(recover.method, "recover_store_maintenance");
    let retained: wire::Approval = ic_wire::decode(
        &recover.args,
        ic_wire::Encoding::Frame {
            method: wire::RECOVER,
            direction: ic_wire::Direction::Request,
        },
        wire::LIMITS,
    )
    .unwrap();
    assert_eq!(retained, approval(&plan).unwrap());
    assert!(
        ic_wire::decode::<wire::Approval>(
            &recover.args,
            ic_wire::Encoding::Frame {
                method: wire::ADVANCE,
                direction: ic_wire::Direction::Request
            },
            wire::LIMITS
        )
        .is_err()
    );
    let mut status = Status {
        approval: retained,
        operator: plan.controller.clone(),
        phase: Phase::Ready,
        stop_request: None,
        failure: None,
        preservation: canic_management_protocol::store_maintenance::Scan::Legacy(None),
        installation: None,
    };
    assert!(admit(&plan, &status).is_ok());
    assert!(
        !ready(&status),
        "a phase label cannot replace preservation and effect evidence"
    );
    status.operator = status.approval.store.clone();
    assert!(admit(&plan, &status).is_err());
}
