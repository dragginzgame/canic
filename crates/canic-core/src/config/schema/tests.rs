use super::*;
use crate::cdk::types::Cycles;
use std::collections::BTreeMap;

fn base_canister_config(kind: CanisterKind) -> CanisterConfig {
    CanisterConfig {
        kind,
        initial_cycles: Cycles::new(0),
        topup: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
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
    let err = toml::from_str::<RoleDeclaration>(
        r#"
kind = "canister"
"#,
    )
    .expect_err("role declaration without package should fail deserialization");

    assert!(err.to_string().contains("missing field `package`"));
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
fn attached_fleet_roles_include_role_bearing_pool_targets() {
    let mut cfg = ConfigModel::test_default();
    let mut hub = base_canister_config(CanisterKind::Singleton);
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
