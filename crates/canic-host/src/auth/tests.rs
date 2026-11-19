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
            receiver_policy: None,
            instance_pool: None,
            shard_pool: None,
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
            auth: vec![AuthConfig {
                root_wasm: None,
                receiver_controller: ReceiverController::Root,
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
            }],
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
    let input = dir.join("canic.toml");
    let (id, plan) = create_auth_plan(&dir, &input, &selected, "2vxsx-fae").unwrap();
    assert_eq!(
        create_auth_plan(&dir, &input, &selected, "2vxsx-fae").unwrap(),
        (id.clone(), plan.clone())
    );
    let before = std::fs::read(dir.join(&id).join("plan.json")).unwrap();
    assert!(create_auth_revocation(&dir, &id, &selected).is_err());
    selected.config.auth.first_mut().unwrap().expires_at_ns += 1;
    assert!(create_auth_plan(&dir, &input, &selected, "2vxsx-fae").is_err());
    selected.config.auth.first_mut().unwrap().expires_at_ns -= 1;
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
    permit_upgrade(&dir, &upgrade).unwrap();
    upgrade.action = PlanAction::Upgrade(plan.auth().unwrap().root.clone());
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
        .is_ok()
    );
}

#[test]
fn a_prior_receiver_upgrade_allows_grant_planning_but_a_root_upgrade_blocks_it() {
    for slot in ["receiver", "root"] {
        let (dir, selected) = fixture();
        let wasm = dir.join("root.wasm");
        let module =
            artifact::admit_bounded(&std::fs::read(&wasm).unwrap(), artifact::MAX_ARTIFACT_BYTES)
                .unwrap()
                .sha256;
        crate::create_plan(
            &dir,
            &wasm,
            crate::PlanInput {
                network: selected.config.network.clone(),
                controller: "2vxsx-fae".into(),
                target: selected.config.canisters[slot].id.clone(),
                previous_module: Some(module),
                creation_cycles: 0,
            },
        )
        .unwrap();
        let grant = create_auth_plan(&dir, &dir.join("canic.toml"), &selected, "2vxsx-fae");
        if slot == "receiver" {
            grant.unwrap();
        } else {
            assert!(
                matches!(grant, Err(Error::Blocked(message)) if message.contains("Root already has an admitted upgrade"))
            );
        }
    }
}

#[test]
fn auth_receipts_require_exact_retained_call_and_revocation_does_not_consume_mutable_config() {
    let (dir, mut selected) = fixture();
    let (id, plan) = create_auth_plan(
        &dir,
        &dir.join("canic.toml"),
        &selected,
        &crate::signed::tests::controller(),
    )
    .unwrap();
    let path = retained_reply(&dir, &plan);
    assert!(execution::receipt(&dir.join(&id), &plan).unwrap().is_some());
    selected.config.auth.first_mut().unwrap().issuer = "ryjl3-tyaaa-aaaaa-aaaba-cai".into();
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
    let config = selected.config.auth.first().unwrap();
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

#[test]
fn operator_controller_is_bound_into_the_plan_and_retained_for_revocation() {
    let (dir, mut selected) = fixture();
    let path = dir.join("canic.toml");
    let signer = crate::signed::tests::controller();
    let legacy = create_auth_plan(&dir.join("legacy"), &path, &selected, &signer)
        .unwrap()
        .1;
    let legacy_json = serde_json::to_value(&legacy).unwrap();
    assert!(legacy_json["auth"].get("receiver_controller").is_none());
    assert_eq!(
        serde_json::from_slice::<Plan>(&serde_json::to_vec(&legacy).unwrap())
            .unwrap()
            .digest()
            .unwrap(),
        legacy.digest().unwrap()
    );

    let config = selected.config.auth.first_mut().unwrap();
    config.receiver_controller = ReceiverController::Operator;
    config.root_wasm = Some("root.wasm".into());
    assert!(
        create_auth_plan(&dir, &path, &selected, &signer).is_err(),
        "conflicting artifact sources"
    );
    selected.config.canisters.get_mut("root").unwrap().wasm = None;
    let (id, plan) = create_auth_plan(&dir, &path, &selected, &signer).unwrap();
    assert_eq!(
        plan.auth()
            .unwrap()
            .expected_receiver_controller(&plan.controller),
        signer
    );
    let mut same_times = plan.clone();
    same_times.created_at_ns = legacy.created_at_ns;
    same_times.expires_at_ns = legacy.expires_at_ns;
    assert_ne!(legacy.digest().unwrap(), same_times.digest().unwrap());
    retained_reply(&dir, &plan);
    selected
        .config
        .auth
        .first_mut()
        .unwrap()
        .receiver_controller = ReceiverController::Root;
    assert!(create_auth_plan(&dir, &path, &selected, &signer).is_err());
    assert_eq!(
        create_auth_revocation(&dir, &id, &selected)
            .unwrap()
            .1
            .auth()
            .unwrap()
            .receiver_controller,
        ReceiverController::Operator
    );
}

#[test]
fn fleet_policy_selects_fresh_install_bytes_without_creating_a_grant() {
    let (_, mut selected) = fixture();
    let config = &mut selected.config;
    config.auth.clear();
    config.canisters.get_mut("root").unwrap().role = Some("root".into());
    config.canisters.get_mut("receiver").unwrap().role = Some("shard".into());
    let policy: ReceiverPolicy = toml::from_str(&format!(r#"
        root = "root"
        roles = ["shard"]
        issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai"
        role = "user"
        network_id = "{}"
        application_id = "{}"
        limits = {{ default_session_secs = 60, max_session_secs = 1800, max_proof_ttl_ns = 60000000000, max_sessions = 7 }}
    "#, "11".repeat(32), "22".repeat(32))).unwrap();
    config.receiver_policy = Some(policy);
    let root = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let args = config
        .receiver_install_args("receiver", root, vec![5, 4])
        .unwrap();
    let decoded = canic::auth::install::Install::decode(&args).unwrap();
    use canic::auth::install::Storage;
    assert_eq!(decoded.storage, Storage::Volatile);
    config.receiver_policy.as_mut().unwrap().storage = Storage::Stable;
    let stable_args = config
        .receiver_install_args("receiver", root, vec![5, 4])
        .unwrap();
    assert_ne!(args, stable_args);
    assert_eq!(
        canic::auth::install::Install::decode(&stable_args)
            .unwrap()
            .storage,
        Storage::Stable
    );
    assert_eq!(decoded.root.to_text(), root);
    assert_eq!(decoded.limits.max_sessions, Some(7));
    assert_eq!(decoded.application, [5, 4]);
    assert_eq!(decoded.audience, [0x22; 32]);
    assert_eq!(decoded.encode().unwrap(), args);
    assert!(config.auth.is_empty());
    assert!(config.receiver_install_args("root", root, vec![]).is_err());
    assert!(
        config
            .receiver_install_args("receiver", "ryjl3-tyaaa-aaaaa-aaaba-cai", vec![])
            .is_err()
    );
    assert!(
        config
            .receiver_install_args("receiver", root, vec![0; 4096])
            .is_err()
    );
    config
        .receiver_policy
        .as_mut()
        .unwrap()
        .limits
        .max_session_secs = 1801;
    assert!(
        config
            .receiver_install_args("receiver", root, vec![])
            .is_err()
    );
}

#[test]
fn receiver_grant_inherits_only_install_identity_and_preserves_its_own_authority() {
    let (_, mut selected) = fixture();
    let config = &mut selected.config;
    config.canisters.get_mut("receiver").unwrap().role = Some("hub".into());
    let original = config.auth[0].clone();
    config.receiver_policy = Some(ReceiverPolicy {
        storage: canic::auth::install::Storage::Volatile,
        root: original.root.clone(),
        roles: vec!["hub".into()],
        issuer: original.issuer.clone(),
        role: original.role.clone(),
        network_id: original.network_id.clone(),
        application_id: original.application_id.clone(),
        limits: canic::auth::install::LimitsPolicy {
            default_session_secs: 1,
            max_session_secs: 60,
            max_proof_ttl_ns: 10,
            max_sessions: None,
        },
    });
    let grant = &mut config.auth[0];
    grant.root.clear();
    grant.issuer.clear();
    grant.role.clear();
    grant.network_id.clear();
    grant.application_id.clear();
    config.resolve_receiver_grants().unwrap();
    assert_eq!(config.auth, [original]);
    config.auth[0].application_id = "00".repeat(32);
    assert!(config.resolve_receiver_grants().is_err());
}
