use super::*;
use crate::{CanisterConfig, FleetConfig, Network, tests::directory};
use std::collections::BTreeMap;

fn fixture() -> (std::path::PathBuf, SelectedFleet) {
    let dir = directory();
    std::fs::write(dir.join("root.wasm"), b"\0asm\x01\0\0\0").unwrap();
    let now = crate::now().unwrap();
    let selected = SelectedFleet {
        app: "demo".into(),
        fleet: "local".into(),
        config: FleetConfig {
            network: Network::Local {
                url: "http://localhost:8000/".into(),
                root_key: "00".repeat(133),
                root_subnet: None,
                routing_canister: "aaaaa-aa".into(),
            },
            journal: None,
            canisters: BTreeMap::from([
                (
                    "root".into(),
                    CanisterConfig {
                        id: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".into()),
                        wasm: Some("root.wasm".into()),
                        ..Default::default()
                    },
                ),
                (
                    "receiver".into(),
                    CanisterConfig {
                        id: Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into()),
                        wasm: Some("root.wasm".into()),
                        ..Default::default()
                    },
                ),
            ]),
            auth: Some(AuthConfig {
                root: "root".into(),
                receiver: "receiver".into(),
                issuer: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
                key: "11".repeat(32),
                network_id: "22".repeat(32),
                application_id: "33".repeat(32),
                role: "user".into(),
                scopes: vec!["read".into()],
                not_before_ns: now - 1,
                expires_at_ns: now + 600_000_000_000,
            }),
        },
    };
    (dir, selected)
}
fn retained_reply(dir: &Path, plan: &Plan) -> std::path::PathBuf {
    let approval = plan.auth().unwrap();
    let root = crate::network::principal(&approval.root.canister).unwrap();
    let call = crate::signed::Call {
        canister: root,
        effective: root,
        method: "configure_auth",
        args: ic_wire::encode(
            &approval.grant,
            ic_wire::Encoding::Frame {
                method: wire::CONFIGURE,
                direction: ic_wire::Direction::Request,
            },
            wire::LIMITS,
        )
        .unwrap(),
    };
    let path = dir.join(plan.digest().unwrap()).join("auth");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(
        path.join("signed.json"),
        crate::signed::tests::signed_fixture(plan, &call, plan.expires_at_ns),
    )
    .unwrap();
    std::fs::write(
        path.join("reply.bin"),
        ic_wire::encode(
            &Ok::<(), wire::Error>(()),
            ic_wire::Encoding::Frame {
                method: wire::CONFIGURE,
                direction: ic_wire::Direction::Response,
            },
            wire::LIMITS,
        )
        .unwrap(),
    )
    .unwrap();
    path
}

#[test]
fn auth_approval_keeps_original_authority_and_rejects_conflicting_configuration() {
    let (dir, mut selected) = fixture();
    let input = dir.join("canic.json");
    let (id, plan) = create_auth_plan(&dir, &input, &selected, "2vxsx-fae").unwrap();
    assert_eq!(
        create_auth_plan(&dir, &input, &selected, "2vxsx-fae").unwrap(),
        (id.clone(), plan.clone())
    );
    let before = std::fs::read(dir.join(&id).join("plan.json")).unwrap();
    assert!(create_auth_revocation(&dir, &id, &selected).is_err());
    selected.config.auth.as_mut().unwrap().expires_at_ns += 1;
    assert!(create_auth_plan(&dir, &input, &selected, "2vxsx-fae").is_err());
    selected.config.auth.as_mut().unwrap().expires_at_ns -= 1;
    for field in ["creation_cycles", "target", "version"] {
        let mut value = serde_json::to_value(&plan).unwrap();
        value[field] = serde_json::json!(1);
        let decoded = serde_json::from_value::<Plan>(value);
        assert!(decoded.is_err() || decoded.unwrap().digest().is_err());
    }
    assert_eq!(
        before,
        std::fs::read(dir.join(&id).join("plan.json")).unwrap()
    );
    assert_eq!(plan::load(&dir.join(&id), &id).unwrap().0, plan);
    assert!(
        crate::fleet::inventory(&dir, false, crate::now().unwrap())
            .unwrap()
            .records
            .is_empty()
    );
    selected.check_operation(&dir, &id).unwrap();
    let mut upgrade = plan.clone();
    upgrade.action = PlanAction::Upgrade(plan.auth().unwrap().receiver.clone());
    assert!(permit_upgrade(&dir, &upgrade).is_err());
    assert!(
        crate::create_plan(
            &dir,
            &dir.join("root.wasm"),
            crate::PlanInput {
                network: selected.config.network.clone(),
                controller: "2vxsx-fae".into(),
                target: Some(plan.auth().unwrap().receiver.canister.clone()),
                previous_module: Some(plan.auth().unwrap().receiver.module.clone()),
                creation_cycles: 0,
            }
        )
        .is_err()
    );
}

#[test]
fn auth_receipts_require_exact_retained_call_and_revocation_does_not_consume_mutable_config() {
    let (dir, mut selected) = fixture();
    let (id, plan) = create_auth_plan(
        &dir,
        &dir.join("canic.json"),
        &selected,
        &crate::signed::tests::controller(),
    )
    .unwrap();
    let path = retained_reply(&dir, &plan);
    assert!(execution::receipt(&dir.join(&id), &plan).unwrap().is_some());
    selected.config.auth.as_mut().unwrap().issuer = "ryjl3-tyaaa-aaaaa-aaaba-cai".into();
    let (revoked, revocation) = create_auth_revocation(&dir, &id, &selected).unwrap();
    assert_eq!(revocation.auth().unwrap().grant, plan.auth().unwrap().grant);
    assert!(revocation.auth().unwrap().revoke);
    assert_eq!(
        create_auth_revocation(&dir, &id, &selected).unwrap().0,
        revoked
    );
    let mut forged = plan.clone();
    if let PlanAction::Auth(a) = &mut forged.action {
        a.revoke = true;
    }
    assert!(execution::receipt(&dir.join(&id), &forged).is_err());
    let signed = std::fs::read(path.join("signed.json")).unwrap();
    std::fs::remove_file(path.join("reply.bin")).unwrap();
    assert!(execution::receipt(&dir.join(&id), &plan).unwrap().is_none());
    assert!(create_auth_revocation(&dir, &id, &selected).is_err());
    assert_eq!(std::fs::read(path.join("signed.json")).unwrap(), signed);
}

#[test]
fn auth_config_requires_bound_distinct_slots_and_canonical_application_grants() {
    let (_, selected) = fixture();
    let config = selected.config.auth.as_ref().unwrap();
    for scopes in [
        vec![],
        vec!["read".into(), "read".into()],
        vec!["write".into(), "read".into()],
    ] {
        let mut bad = config.clone();
        bad.scopes = scopes;
        assert!(bad.grant(&selected.config).is_err());
    }
    let mut bad = config.clone();
    bad.receiver = bad.root.clone();
    assert!(bad.grant(&selected.config).is_err());
    let mut bad = config.clone();
    bad.application_id = "AA".repeat(32);
    assert!(bad.grant(&selected.config).is_err());
    let mut bad = config.clone();
    bad.issuer = "2vxsx-fae".into();
    assert!(bad.grant(&selected.config).is_err());
    let mut bad = selected.config.clone();
    bad.canisters.get_mut("receiver").unwrap().id = None;
    assert!(config.grant(&bad).is_err());
}
