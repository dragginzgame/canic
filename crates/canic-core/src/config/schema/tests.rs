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
        cycles_funding: CyclesFundingPolicyConfig::default(),
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

#[test]
fn root_canister_must_exist_in_prime_subnet() {
    let mut cfg = ConfigModel {
        fleet: Some(FleetConfig {
            name: Some("test".to_string()),
        }),
        ..Default::default()
    };
    cfg.roles.insert(
        CanisterRole::ROOT,
        RoleDeclaration {
            kind: RoleDeclarationKind::Root,
            package: "root".to_string(),
        },
    );
    cfg.subnets
        .insert(SubnetRole::PRIME, SubnetConfig::default());

    cfg.validate()
        .expect_err("expected missing root canister to fail validation");
}

#[test]
fn fleet_name_is_accepted_when_configured() {
    let mut cfg = ConfigModel::test_default();
    cfg.fleet = Some(FleetConfig {
        name: Some("demo".to_string()),
    });

    cfg.validate().expect("fleet name should be valid");
}

#[test]
fn fleet_name_must_be_filesystem_safe() {
    let mut cfg = ConfigModel::test_default();
    cfg.fleet = Some(FleetConfig {
        name: Some("demo fleet".to_string()),
    });

    cfg.validate().expect_err("fleet name should fail");
}

#[test]
fn fleet_name_is_required() {
    let mut cfg = ConfigModel::test_default();
    cfg.fleet = None;

    let err = cfg.validate().expect_err("fleet name should be required");

    assert!(
        err.to_string().contains("fleet config is required"),
        "expected fleet error, got: {err}"
    );
}

#[test]
fn test_fleet_configs_validate_with_chain_key_batch_policy() {
    let root = workspace_root();
    for rel_path in [
        "fleets/test/canic.toml",
        "fleets/test/test-configs/root-capability.toml",
        "fleets/test/test-configs/root-scaling.toml",
        "fleets/test/test-configs/root-sharding.toml",
    ] {
        let path = root.join(rel_path);
        let source =
            fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {rel_path} failed: {err}"));
        let cfg = crate::bootstrap::parse_config_model(&source)
            .unwrap_or_else(|err| panic!("{rel_path} should parse and validate: {err}"));

        assert_eq!(
            cfg.auth.delegated_tokens.root_proof_mode, "chain_key_batch",
            "{rel_path} should use the 0.76 hard-cut root proof mode",
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
    }
}

#[test]
fn topology_roles_must_be_declared() {
    let mut cfg = ConfigModel::test_default();
    cfg.subnets
        .get_mut(&SubnetRole::PRIME)
        .unwrap()
        .canisters
        .insert(
            CanisterRole::from("app"),
            base_canister_config(CanisterKind::Singleton),
        );

    let err = cfg
        .validate()
        .expect_err("topology role should need declaration");

    assert!(
        err.to_string().contains("is not declared"),
        "expected role declaration error, got: {err}"
    );
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

    let err = cfg.validate().expect_err("empty role package should fail");

    assert!(
        err.to_string()
            .contains("role declaration 'store' package must not be empty"),
        "expected empty package error, got: {err}"
    );
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
fn topology_less_config_rejects_root_and_app_index() {
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

    let mut app_index_cfg = ConfigModel::test_default();
    app_index_cfg.subnets.clear();
    app_index_cfg.roles.remove(&CanisterRole::ROOT);
    app_index_cfg.app_index.insert(CanisterRole::from("store"));
    app_index_cfg.roles.insert(
        CanisterRole::from("store"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "store".to_string(),
        },
    );

    let app_index_err = app_index_cfg
        .validate()
        .expect_err("topology-less app_index should fail");
    assert!(
        app_index_err
            .to_string()
            .contains("topology-less configs cannot define app_index entries"),
        "expected app_index error, got: {app_index_err}"
    );
}

#[test]
fn app_index_requires_prime_service_role() {
    let mut cfg = ConfigModel::test_default();
    cfg.app_index.insert(CanisterRole::from("project_hub"));
    cfg.roles.insert(
        CanisterRole::from("project_hub"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "project_hub".to_string(),
        },
    );
    cfg.subnets
        .get_mut(&SubnetRole::PRIME)
        .unwrap()
        .canisters
        .insert(
            CanisterRole::from("project_hub"),
            base_canister_config(CanisterKind::Singleton),
        );

    let err = cfg
        .validate()
        .expect_err("app_index singleton roles should be rejected");

    assert!(
        err.to_string().contains("must have kind = \"service\""),
        "expected service-kind app_index error, got: {err}"
    );

    cfg.subnets
        .get_mut(&SubnetRole::PRIME)
        .unwrap()
        .canisters
        .insert(
            CanisterRole::from("project_hub"),
            base_canister_config(CanisterKind::Service),
        );

    cfg.validate()
        .expect("app_index service role should validate");
}

#[test]
fn attached_fleet_roles_include_role_bearing_pool_targets() {
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

    let prime = cfg.subnets.get_mut(&SubnetRole::PRIME).unwrap();
    prime.canisters.insert(CanisterRole::from("user_hub"), hub);
    prime.canisters.insert(
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
    let attached = cfg.attached_fleet_roles();

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

    cfg.subnets.get_mut(&SubnetRole::PRIME).unwrap().canisters = canisters;

    cfg.validate().expect_err("expected non-root kind to fail");
}

#[test]
fn multiple_root_canisters_are_rejected() {
    let mut cfg = ConfigModel::test_default();

    cfg.subnets.insert(
        SubnetRole::new("aux"),
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
fn delegated_tokens_network_must_be_known() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.network = "mars".to_string();

    cfg.validate()
        .expect_err("expected invalid network to fail");
}

#[test]
fn delegated_tokens_root_proof_mode_must_be_chain_key_batch() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.root_proof_mode = "canister_signature".to_string();

    let err = cfg
        .validate()
        .expect_err("expected non-chain-key root proof mode to fail");

    assert!(
        err.to_string().contains("must be chain_key_batch"),
        "unexpected error: {err}"
    );
}

#[test]
fn delegated_tokens_chain_key_batch_requires_key_policy() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.root_proof_mode = "chain_key_batch".to_string();
    cfg.auth.delegated_tokens.network = "local".to_string();
    cfg.auth.delegated_tokens.chain_key_root_proof = ChainKeyRootProofConfig::default();

    let err = cfg
        .validate()
        .expect_err("expected missing chain-key policy to fail");

    assert!(
        err.to_string().contains("chain_key_root_proof.key_id"),
        "unexpected error: {err}"
    );
}

#[test]
fn delegated_tokens_chain_key_batch_requires_derivation_path() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .derivation_path_hex = None;

    let err = cfg
        .validate()
        .expect_err("expected missing derivation path to fail");

    assert!(
        err.to_string().contains("derivation_path_hex"),
        "unexpected error: {err}"
    );
}

#[test]
fn delegated_tokens_chain_key_derivation_path_must_be_hex() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .derivation_path_hex = Some(vec!["not hex".to_string()]);

    let err = cfg
        .validate()
        .expect_err("expected invalid derivation path hex to fail");

    assert!(
        err.to_string().contains("derivation_path_hex[0]"),
        "unexpected error: {err}"
    );
}

#[test]
fn delegated_tokens_chain_key_derivation_path_hash_must_match_path() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .derivation_path_hash_hex = Some("11".repeat(32));

    let err = cfg
        .validate()
        .expect_err("expected mismatched derivation path hash to fail");

    assert!(
        err.to_string()
            .contains("does not match derivation_path_hex"),
        "unexpected error: {err}"
    );
}

#[test]
fn delegated_tokens_chain_key_public_key_must_be_sec1_secp256k1() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth
        .delegated_tokens
        .chain_key_root_proof
        .public_key_hex = Some("00".repeat(33));

    let err = cfg
        .validate()
        .expect_err("expected invalid chain-key public key to fail");

    assert!(
        err.to_string()
            .contains("must be a secp256k1 SEC1 public key"),
        "unexpected error: {err}"
    );
}

#[test]
fn delegated_tokens_chain_key_mainnet_rejects_test_key() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.root_proof_mode = "chain_key_batch".to_string();
    cfg.auth.delegated_tokens.network = "mainnet".to_string();
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

    let err = cfg
        .validate()
        .expect_err("expected mainnet test key to fail");

    assert!(
        err.to_string().contains("must not be test_key_1"),
        "unexpected error: {err}"
    );
}

#[test]
fn delegated_tokens_mainnet_requires_known_mainnet_root_key_when_key_is_configured() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.network = "mainnet".to_string();
    cfg.auth.delegated_tokens.ic_root_public_key_raw_hex = Some("07".repeat(96));

    let err = cfg
        .validate()
        .expect_err("expected wrong mainnet root key to fail");

    assert!(
        err.to_string()
            .contains("requires the known mainnet raw IC root public key"),
        "unexpected error: {err}"
    );
}

#[test]
fn delegated_tokens_local_rejects_configured_mainnet_root_key() {
    let mut cfg = ConfigModel::test_default();
    cfg.auth.delegated_tokens.network = "local".to_string();
    cfg.auth.delegated_tokens.ic_root_public_key_raw_hex =
        Some(hex(MAINNET_IC_ROOT_PUBLIC_KEY_RAW));

    let err = cfg
        .validate()
        .expect_err("expected local config with mainnet root key to fail");

    assert!(
        err.to_string()
            .contains("network=\"local\" must not use the mainnet IC root public key"),
        "unexpected error: {err}"
    );
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
