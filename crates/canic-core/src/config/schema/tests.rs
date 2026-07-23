//! Module: config::schema::tests
//!
//! Responsibility: verify config schema validation and helper behavior.
//! Does not own: production config schemas or runtime config storage.
//! Boundary: test-only checks over schema models and validation implementations.

use super::*;
use crate::{cdk::types::Cycles, domain::auth::MAINNET_IC_ROOT_PUBLIC_KEY_RAW};
use std::{collections::BTreeMap, fmt::Write as _, fs, path::PathBuf};

fn hex(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut out, "{byte:02x}").expect("hex write should not fail");
    }
    out
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("canic-core should live under workspace crates/")
        .to_path_buf()
}

fn base_canister_config(kind: CanisterKind) -> CanisterConfig {
    CanisterConfig {
        kind,
        initial_cycles: Cycles::new(0),
        topup: None,
        icp_refill: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        binding: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

#[test]
fn root_canister_must_exist_in_default_subnet_slot() {
    let mut cfg = ConfigModel::default();
    cfg.app.name = AppId::from("test");
    cfg.roles.insert(
        CanisterRole::ROOT,
        RoleDeclaration {
            kind: RoleDeclarationKind::Root,
            package: "root".to_string(),
        },
    );
    cfg.subnets
        .insert(SubnetSlotId::DEFAULT, SubnetConfig::default());

    cfg.validate()
        .expect_err("expected missing root canister to fail validation");
}

#[test]
fn app_name_is_accepted_when_configured() {
    let mut cfg = ConfigModel::test_default();
    cfg.app.name = AppId::from("demo");

    cfg.validate().expect("App name should be valid");
}

#[test]
fn app_name_must_be_filesystem_safe() {
    let mut cfg = ConfigModel::test_default();
    cfg.app.name = AppId::from("demo fleet");

    cfg.validate().expect_err("App name should fail");
}

#[test]
fn app_name_is_required() {
    let mut cfg = ConfigModel::test_default();
    cfg.app.name = AppId::default();

    cfg.validate().expect_err("App name should be required");
}

#[test]
fn canister_role_name_admission_accepts_canonical_segments() {
    for role in ["a", "app", "app2", "user_hub", "scale_replica", "role_2"] {
        validate_canister_role_name(role)
            .unwrap_or_else(|issue| panic!("{role:?} should be admitted: {issue}"));
    }
}

#[test]
fn canister_role_name_admission_rejects_typed_invalid_segments() {
    for role in [
        "-",
        "--help",
        "-App",
        "App",
        "_",
        "_app",
        "1app",
        "app-",
        "scale-1",
        "app_",
        "app__worker",
        "../sentinel",
        "app/name",
        "app.name",
        "app name",
        "café",
    ] {
        assert_eq!(
            validate_canister_role_name(role),
            Err(CanisterRoleNameIssue::InvalidSnakeCase),
            "{role:?} should be rejected",
        );
    }
    assert_eq!(
        validate_canister_role_name(""),
        Err(CanisterRoleNameIssue::Empty),
    );
    assert_eq!(
        validate_canister_role_name(&"a".repeat(NAME_MAX_BYTES + 1)),
        Err(CanisterRoleNameIssue::TooLong {
            max_bytes: NAME_MAX_BYTES,
        }),
    );
}

#[test]
fn complete_config_validation_rejects_unadmitted_role_declarations() {
    let invalid_roles = [
        String::new(),
        "a".repeat(NAME_MAX_BYTES + 1),
        "-app".to_string(),
        "App".to_string(),
        "_app".to_string(),
        "1app".to_string(),
        "user-hub".to_string(),
        "app_".to_string(),
        "app__worker".to_string(),
        "app.name".to_string(),
        "../sentinel".to_string(),
        "app/name".to_string(),
        "café".to_string(),
        "app name".to_string(),
        "app+worker".to_string(),
    ];

    for role in invalid_roles {
        let mut cfg = ConfigModel::test_default();
        cfg.roles.insert(
            CanisterRole::owned(role.clone()),
            RoleDeclaration {
                kind: RoleDeclarationKind::Canister,
                package: "app".to_string(),
            },
        );

        let error = cfg
            .validate()
            .expect_err("unadmitted role declaration should fail");
        assert!(
            matches!(
                error,
                ConfigSchemaError::InvalidCanisterRoleName {
                    context: "role declaration",
                    role: invalid_role,
                    ..
                } if invalid_role == role
            ),
            "{role:?} should fail canonical config admission",
        );
    }
}

#[test]
fn checked_in_delegated_auth_configs_validate_with_current_chain_key_policy() {
    let root = workspace_root();
    for rel_path in [
        "fleets/test/canic.toml",
        "fleets/test/test-configs/root-capability.toml",
        "fleets/test/test-configs/root-scaling.toml",
        "fleets/test/test-configs/root-sharding.toml",
        "canisters/test/delegation_issuer_stub/canic.toml",
        "canisters/test/delegation_root_stub/canic.toml",
        "canisters/test/project_hub_stub/canic.toml",
        "canisters/test/project_instance_stub/canic.toml",
        "canisters/test/runtime_probe/canic.toml",
    ] {
        let path = root.join(rel_path);
        let source =
            fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {rel_path} failed: {err}"));
        let cfg = crate::bootstrap::parse_config_model(&source)
            .unwrap_or_else(|err| panic!("{rel_path} should parse and validate: {err}"));

        assert_eq!(
            cfg.auth.delegated_tokens.build_network,
            BuildNetwork::Local,
            "{rel_path} should use the local build-network trust policy",
        );
        assert_eq!(
            cfg.auth
                .delegated_tokens
                .chain_key_root_proof
                .key_id
                .as_deref(),
            Some("key_1"),
            "{rel_path} should use the PocketIC-exposed local chain-key id",
        );
        assert!(
            !cfg.auth
                .delegated_tokens
                .chain_key_root_proof
                .allow_test_key,
            "{rel_path} should not require the test-key exemption",
        );
        assert_eq!(
            cfg.auth
                .delegated_tokens
                .chain_key_root_proof
                .min_accepted_proof_epoch,
            Some(2),
            "{rel_path} should use the current proof-epoch floor",
        );
        assert_eq!(
            cfg.auth
                .delegated_tokens
                .chain_key_root_proof
                .min_accepted_registry_epoch,
            Some(2),
            "{rel_path} should use the current registry-epoch floor",
        );
    }
}

#[test]
fn checked_in_active_configs_parse_and_validate() {
    let root = workspace_root();
    for rel_path in [
        "canisters/audit/leaf_probe/canic.toml",
        "canisters/audit/minimal_metrics/canic.toml",
        "canisters/audit/root_probe/canic.toml",
        "canisters/audit/scaling_probe/canic.toml",
        "canisters/test/blob_storage_cashier_mock/canic.toml",
        "canisters/test/blob_storage_probe/canic.toml",
        "canisters/test/delegation_issuer_stub/canic.toml",
        "canisters/test/delegation_root_stub/canic.toml",
        "canisters/test/payload_limit_probe/canic.toml",
        "canisters/test/project_hub_stub/canic.toml",
        "canisters/test/project_instance_stub/canic.toml",
        "canisters/test/runtime_probe/canic.toml",
        "crates/canic-wasm-store/canic.toml",
        "fleets/demo/canic.toml",
        "fleets/test/canic.toml",
    ] {
        let path = root.join(rel_path);
        let source =
            fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {rel_path} failed: {err}"));
        crate::bootstrap::parse_config_model(&source)
            .unwrap_or_else(|err| panic!("{rel_path} should parse and validate: {err}"));
    }
}

#[test]
fn topology_roles_must_be_declared() {
    let mut cfg = ConfigModel::test_default();
    cfg.subnets
        .get_mut(&SubnetSlotId::DEFAULT)
        .unwrap()
        .canisters
        .insert(
            CanisterRole::from("app"),
            base_canister_config(CanisterKind::Singleton),
        );

    cfg.validate()
        .expect_err("topology role should need declaration");
}

#[test]
fn non_root_role_declaration_may_be_declared_only() {
    let mut cfg = ConfigModel::test_default();
    cfg.roles.insert(
        CanisterRole::from("store"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "crates/store".to_string(),
        },
    );

    cfg.validate()
        .expect("declared-only non-root role should be valid");

    assert!(cfg.declares_role(&CanisterRole::from("store")));
    assert!(!cfg.attached_roles().contains("store"));
}

#[test]
fn role_declarations_require_package_paths() {
    toml::from_str::<RoleDeclaration>(
        r#"
kind = "canister"
"#,
    )
    .expect_err("role declaration without package should fail deserialization");
}

#[test]
fn role_declaration_package_paths_must_not_be_empty() {
    let mut cfg = ConfigModel::test_default();
    cfg.roles.insert(
        CanisterRole::from("store"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: " ".to_string(),
        },
    );

    cfg.validate().expect_err("empty role package should fail");
}

#[test]
fn topology_less_config_may_declare_only_non_root_roles() {
    let mut cfg = ConfigModel::test_default();
    cfg.subnets.clear();
    cfg.roles.remove(&CanisterRole::ROOT);
    cfg.roles.insert(
        CanisterRole::from("store"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "store".to_string(),
        },
    );

    cfg.validate()
        .expect("topology-less non-root role declaration should be valid");

    assert!(cfg.declares_role(&CanisterRole::from("store")));
    assert!(cfg.attached_roles().is_empty());
}

#[test]
fn topology_less_config_rejects_root_and_fleet_services() {
    let mut root_cfg = ConfigModel::test_default();
    root_cfg.subnets.clear();
    root_cfg.roles.insert(
        CanisterRole::ROOT,
        RoleDeclaration {
            kind: RoleDeclarationKind::Root,
            package: "root".to_string(),
        },
    );

    let root_err = root_cfg
        .validate()
        .expect_err("topology-less root declaration should fail");
    assert!(
        root_err
            .to_string()
            .contains("topology-less configs cannot declare role 'root'"),
        "expected root error, got: {root_err}"
    );

    let mut services_cfg = ConfigModel::test_default();
    services_cfg.subnets.clear();
    services_cfg.roles.remove(&CanisterRole::ROOT);
    services_cfg
        .services
        .fleet
        .roles
        .insert(CanisterRole::from("store"));
    services_cfg.roles.insert(
        CanisterRole::from("store"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "store".to_string(),
        },
    );

    let services_err = services_cfg
        .validate()
        .expect_err("topology-less Fleet services should fail");
    assert!(
        services_err
            .to_string()
            .contains("topology-less configs cannot define services.fleet.roles entries"),
        "expected Fleet services error, got: {services_err}"
    );
}

#[test]
fn fleet_services_require_default_slot_service_roles() {
    let mut cfg = ConfigModel::test_default();
    cfg.services
        .fleet
        .roles
        .insert(CanisterRole::from("project_hub"));
    cfg.roles.insert(
        CanisterRole::from("project_hub"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "project_hub".to_string(),
        },
    );
    cfg.subnets
        .get_mut(&SubnetSlotId::DEFAULT)
        .unwrap()
        .canisters
        .insert(
            CanisterRole::from("project_hub"),
            base_canister_config(CanisterKind::Singleton),
        );

    cfg.validate()
        .expect_err("Fleet service singleton roles should be rejected");

    cfg.subnets
        .get_mut(&SubnetSlotId::DEFAULT)
        .unwrap()
        .canisters
        .insert(
            CanisterRole::from("project_hub"),
            base_canister_config(CanisterKind::Service),
        );

    cfg.validate().expect("Fleet service role should validate");
}

#[test]
fn attached_app_roles_include_role_bearing_pool_targets() {
    let mut cfg = ConfigModel::test_default();
    let mut hub = base_canister_config(CanisterKind::Service);
    let mut sharding = ShardingConfig::default();
    sharding.pools.insert(
        "users".to_string(),
        ShardPool {
            canister_role: CanisterRole::from("user_shard"),
            policy: ShardPoolPolicy::default(),
        },
    );
    hub.sharding = Some(sharding);

    let default_slot = cfg.subnets.get_mut(&SubnetSlotId::DEFAULT).unwrap();
    default_slot
        .canisters
        .insert(CanisterRole::from("user_hub"), hub);
    default_slot.canisters.insert(
        CanisterRole::from("user_shard"),
        base_canister_config(CanisterKind::Shard),
    );
    cfg.roles.insert(
        CanisterRole::from("user_hub"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "user_hub".to_string(),
        },
    );
    cfg.roles.insert(
        CanisterRole::from("user_shard"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "user_shard".to_string(),
        },
    );

    cfg.validate().expect("config should validate");
    let attached = cfg.attached_app_roles();

    assert!(
        attached
            .iter()
            .any(|role| role.to_string() == "test.user_hub")
    );
    assert!(
        attached
            .iter()
            .any(|role| role.to_string() == "test.user_shard")
    );
}

#[test]
fn root_canister_must_be_kind_root() {
    let mut cfg = ConfigModel::test_default();
    let mut canisters = BTreeMap::new();

    canisters.insert(
        CanisterRole::ROOT,
        base_canister_config(CanisterKind::Singleton),
    );

    cfg.subnets
        .get_mut(&SubnetSlotId::DEFAULT)
        .unwrap()
        .canisters = canisters;

    cfg.validate().expect_err("expected non-root kind to fail");
}

#[test]
fn multiple_root_canisters_are_rejected() {
    let mut cfg = ConfigModel::test_default();

    cfg.subnets.insert(
        SubnetSlotId::new("aux"),
        SubnetConfig {
            canisters: {
                let mut m = BTreeMap::new();
                m.insert(CanisterRole::ROOT, base_canister_config(CanisterKind::Root));
                m
            },
            ..Default::default()
        },
    );

    cfg.validate().expect_err("expected multiple roots to fail");
}

#[test]
fn delegated_tokens_max_ttl_zero_is_invalid() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.max_ttl_secs = Some(0);

    cfg.validate().expect_err("expected zero ttl to fail");
}

#[test]
fn delegated_tokens_invalid_root_canister_id_is_invalid() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.root_canister_id = Some("not a principal".to_string());

    cfg.validate()
        .expect_err("expected invalid root canister id to fail");
}

#[test]
fn delegated_tokens_invalid_ic_root_public_key_hex_is_invalid() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.ic_root_public_key_raw_hex = Some("not-hex".to_string());

    cfg.validate()
        .expect_err("expected invalid root key hex to fail");
}

#[test]
fn delegated_tokens_ic_root_public_key_hex_must_be_raw_length() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.ic_root_public_key_raw_hex = Some("00".repeat(95));

    cfg.validate()
        .expect_err("expected short raw root key to fail");
}

#[test]
fn delegated_tokens_build_network_must_be_known() {
    crate::bootstrap::parse_config_model(
        r#"
[auth.delegated_tokens]
enabled = false
build_network = "mars"
"#,
    )
    .expect_err("expected invalid build network to fail");
}

#[test]
fn delegated_tokens_chain_key_batch_requires_key_policy() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.build_network = BuildNetwork::Local;
    cfg.auth.delegated_tokens.chain_key_root_proof = ChainKeyRootProofConfig::default();

    cfg.validate()
        .expect_err("expected missing chain-key policy to fail");
}

#[test]
fn delegated_tokens_chain_key_batch_requires_derivation_path() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .derivation_path_hex = None;

    cfg.validate()
        .expect_err("expected missing derivation path to fail");
}

#[test]
fn delegated_tokens_chain_key_derivation_path_must_be_hex() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .derivation_path_hex = Some(vec!["not hex".to_string()]);

    cfg.validate()
        .expect_err("expected invalid derivation path hex to fail");
}

#[test]
fn delegated_tokens_chain_key_derivation_path_hash_must_match_path() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .derivation_path_hash_hex = Some("11".repeat(32));

    cfg.validate()
        .expect_err("expected mismatched derivation path hash to fail");
}

#[test]
fn delegated_tokens_chain_key_public_key_must_be_sec1_secp256k1() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .public_key_hex = Some("00".repeat(33));

    cfg.validate()
        .expect_err("expected invalid chain-key public key to fail");
}

#[test]
fn delegated_tokens_chain_key_ic_rejects_test_key() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.build_network = BuildNetwork::Ic;
    cfg.auth.delegated_tokens.chain_key_root_proof.key_id = Some("test_key_1".to_string());
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .derivation_path_hash_hex =
        Some("fe51a87b988d221227b134c48f36787e891a902dcb5d48ea5f94cff8bfed5a16".to_string());
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .derivation_path_hex = Some(vec![
        "63616e6963".to_string(),
        "64656c65676174696f6e".to_string(),
    ]);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .public_key_hex = Some("02".repeat(33));
    cfg.auth.delegated_tokens.chain_key_root_proof.key_version = Some(1);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .min_accepted_key_version = Some(1);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .min_accepted_proof_epoch = Some(1);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .min_accepted_registry_epoch = Some(1);
    cfg.auth.delegated_tokens.chain_key_root_proof.valid_from_ns = Some(1);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .accept_until_ns = Some(2);
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .max_revocation_latency_ns = Some(1);

    cfg.validate().expect_err("expected IC test key to fail");
}

#[test]
fn delegated_tokens_ic_requires_known_mainnet_root_key_when_key_is_configured() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.build_network = BuildNetwork::Ic;
    cfg.auth.delegated_tokens.ic_root_public_key_raw_hex = Some("07".repeat(96));

    cfg.validate()
        .expect_err("expected wrong IC root key to fail");
}

#[test]
fn delegated_tokens_local_rejects_configured_mainnet_root_key() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.build_network = BuildNetwork::Local;
    cfg.auth.delegated_tokens.ic_root_public_key_raw_hex =
        Some(hex(MAINNET_IC_ROOT_PUBLIC_KEY_RAW));

    cfg.validate()
        .expect_err("expected local config with mainnet root key to fail");
}

#[test]
fn role_attestation_max_ttl_zero_is_invalid() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.role_attestation.max_ttl_secs = 0;

    cfg.validate().expect_err("expected zero ttl to fail");
}

#[test]
fn role_attestation_empty_min_epoch_role_key_is_invalid() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth
        .role_attestation
        .min_accepted_epoch_by_role
        .insert("   ".to_string(), 1);

    cfg.validate()
        .expect_err("expected empty min epoch role key to fail");
}

#[test]
fn invalid_whitelist_principal_is_rejected() {
    let mut cfg = ConfigModel::test_default();
    cfg.app.whitelist = Some(Whitelist {
        principals: std::iter::once("not-a-principal".into()).collect(),
    });

    cfg.validate()
        .expect_err("expected invalid principal to fail");
}

#[test]
fn missing_whitelist_fails_closed() {
    let cfg = ConfigModel::test_default();
    let caller = Principal::from_slice(&[42; 29]);

    assert!(!cfg.is_whitelisted(&caller));
}
