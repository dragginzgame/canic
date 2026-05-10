use super::*;
use crate::config::schema::{NAME_MAX_BYTES, Validate};
use std::collections::{BTreeMap, BTreeSet};

fn base_canister_config(kind: CanisterKind) -> CanisterConfig {
    CanisterConfig {
        kind,
        initial_cycles: defaults::initial_cycles(),
        topup_policy: None,
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
fn randomness_defaults_to_ic() {
    let cfg = RandomnessConfig::default();

    assert!(cfg.enabled);
    assert_eq!(cfg.reseed_interval_secs, 3600);
    assert_eq!(cfg.source, RandomnessSource::Ic);
}

#[test]
fn randomness_source_parses_ic_and_time() {
    let cfg: RandomnessConfig = toml::from_str("source = \"ic\"").unwrap();
    assert_eq!(cfg.source, RandomnessSource::Ic);

    let cfg: RandomnessConfig = toml::from_str("source = \"time\"").unwrap();
    assert_eq!(cfg.source, RandomnessSource::Time);
}

#[test]
fn metrics_profile_defaults_follow_canister_role() {
    let root = base_canister_config(CanisterKind::Root);
    assert_eq!(
        root.resolved_metrics_profile(&CanisterRole::ROOT),
        MetricsProfile::Root
    );

    let wasm_store = base_canister_config(CanisterKind::Singleton);
    assert_eq!(
        wasm_store.resolved_metrics_profile(&CanisterRole::WASM_STORE),
        MetricsProfile::Storage
    );

    let mut hub = base_canister_config(CanisterKind::Singleton);
    hub.directory = Some(DirectoryConfig::default());
    assert_eq!(
        hub.resolved_metrics_profile(&CanisterRole::from("user_hub")),
        MetricsProfile::Hub
    );

    let leaf = base_canister_config(CanisterKind::Shard);
    assert_eq!(
        leaf.resolved_metrics_profile(&CanisterRole::from("user_shard")),
        MetricsProfile::Leaf
    );
}

#[test]
fn metrics_profile_override_wins_over_default() {
    let mut cfg = base_canister_config(CanisterKind::Singleton);
    cfg.metrics.profile = Some(MetricsProfile::Full);

    assert_eq!(
        cfg.resolved_metrics_profile(&CanisterRole::from("app")),
        MetricsProfile::Full
    );
}

#[test]
fn root_canister_rejects_configured_auth_roles() {
    let mut cfg = base_canister_config(CanisterKind::Root);
    cfg.auth = CanisterAuthConfig {
        delegated_token_signer: true,
        role_attestation_cache: true,
    };

    let mut subnet = SubnetConfig::default();
    subnet.canisters.insert(CanisterRole::ROOT, cfg);

    let err = subnet.validate().expect_err(
        "root delegated auth signer/cache roles must be implicit services, not config toggles",
    );

    assert!(
        err.to_string().contains("auth signer/cache roles"),
        "expected root auth role validation error, got: {err}"
    );
}

#[test]
fn auto_create_entries_must_exist_in_subnet() {
    let mut auto_create = BTreeSet::new();
    auto_create.insert(CanisterRole::from("missing_auto_canister"));

    let subnet = SubnetConfig {
        auto_create,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected missing auto-create role to fail");
}

#[test]
fn sharding_pool_references_must_exist_in_subnet() {
    let managing_role: CanisterRole = "shard_hub".into();
    let mut canisters = BTreeMap::new();

    let mut sharding = ShardingConfig::default();
    sharding.pools.insert(
        "primary".into(),
        ShardPool {
            canister_role: CanisterRole::from("missing_shard_worker"),
            policy: ShardPoolPolicy::default(),
        },
    );

    let manager_cfg = CanisterConfig {
        sharding: Some(sharding),
        ..base_canister_config(CanisterKind::Shard)
    };

    canisters.insert(managing_role, manager_cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected missing replica role to fail");
}

#[test]
fn sharding_pool_policy_requires_positive_capacity_and_shards() {
    let managing_role: CanisterRole = "shard_hub".into();
    let mut canisters = BTreeMap::new();

    let mut sharding = ShardingConfig::default();
    sharding.pools.insert(
        "primary".into(),
        ShardPool {
            canister_role: managing_role.clone(),
            policy: ShardPoolPolicy {
                capacity: 0,
                initial_shards: 1,
                max_shards: 0,
            },
        },
    );

    canisters.insert(
        managing_role,
        CanisterConfig {
            sharding: Some(sharding),
            ..base_canister_config(CanisterKind::Shard)
        },
    );

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected invalid sharding policy to fail");
}

#[test]
fn sharding_pool_policy_defaults_to_one_initial_shard() {
    let policy: ShardPoolPolicy =
        toml::from_str("capacity = 100\nmax_shards = 4").expect("policy should parse");

    assert_eq!(policy.initial_shards, 1);
}

#[test]
fn sharding_pool_policy_rejects_initial_shards_above_max() {
    let managing_role: CanisterRole = "shard_hub".into();
    let worker_role: CanisterRole = "shard_worker".into();
    let mut canisters = BTreeMap::new();

    let mut sharding = ShardingConfig::default();
    sharding.pools.insert(
        "primary".into(),
        ShardPool {
            canister_role: worker_role.clone(),
            policy: ShardPoolPolicy {
                capacity: 10,
                initial_shards: 3,
                max_shards: 2,
            },
        },
    );

    canisters.insert(worker_role, base_canister_config(CanisterKind::Shard));
    canisters.insert(
        managing_role,
        CanisterConfig {
            sharding: Some(sharding),
            ..base_canister_config(CanisterKind::Singleton)
        },
    );

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected oversized initial_shards to fail");
}

#[test]
fn canister_role_name_must_fit_bound() {
    let long_role = "a".repeat(NAME_MAX_BYTES + 1);
    let mut canisters = BTreeMap::new();
    canisters.insert(
        CanisterRole::from(long_role),
        base_canister_config(CanisterKind::Singleton),
    );

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected canister role length to fail");
}

#[test]
fn sharding_pool_name_must_fit_bound() {
    let managing_role: CanisterRole = "shard_hub".into();
    let mut canisters = BTreeMap::new();

    let mut sharding = ShardingConfig::default();
    sharding.pools.insert(
        "a".repeat(NAME_MAX_BYTES + 1),
        ShardPool {
            canister_role: managing_role.clone(),
            policy: ShardPoolPolicy::default(),
        },
    );

    canisters.insert(
        managing_role,
        CanisterConfig {
            sharding: Some(sharding),
            ..base_canister_config(CanisterKind::Shard)
        },
    );

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected sharding pool name length to fail");
}

#[test]
fn scaling_pool_policy_requires_max_ge_min_when_bounded() {
    let mut canisters = BTreeMap::new();
    let mut pools = BTreeMap::new();
    pools.insert(
        "replica".into(),
        ScalePool {
            canister_role: CanisterRole::from("replica"),
            policy: ScalePoolPolicy {
                initial_workers: 1,
                min_workers: 5,
                max_workers: 3,
            },
        },
    );

    canisters.insert(
        CanisterRole::from("replica"),
        base_canister_config(CanisterKind::Replica),
    );

    let manager_cfg = CanisterConfig {
        scaling: Some(ScalingConfig { pools }),
        ..base_canister_config(CanisterKind::Singleton)
    };

    canisters.insert(CanisterRole::from("manager"), manager_cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected invalid scaling policy to fail");
}

#[test]
fn scaling_pool_policy_defaults_to_one_initial_worker() {
    let policy: ScalePoolPolicy =
        toml::from_str("min_workers = 2\nmax_workers = 4").expect("policy should parse");

    assert_eq!(policy.initial_workers, 1);
}

#[test]
fn scaling_pool_policy_rejects_initial_workers_above_bounded_max() {
    let mut canisters = BTreeMap::new();
    let mut pools = BTreeMap::new();
    pools.insert(
        "replica".into(),
        ScalePool {
            canister_role: CanisterRole::from("replica"),
            policy: ScalePoolPolicy {
                initial_workers: 4,
                min_workers: 1,
                max_workers: 3,
            },
        },
    );

    canisters.insert(
        CanisterRole::from("replica"),
        base_canister_config(CanisterKind::Replica),
    );

    let manager_cfg = CanisterConfig {
        scaling: Some(ScalingConfig { pools }),
        ..base_canister_config(CanisterKind::Singleton)
    };

    canisters.insert(CanisterRole::from("manager"), manager_cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected oversized initial_workers to fail");
}

#[test]
fn scaling_pool_name_must_fit_bound() {
    let mut canisters = BTreeMap::new();
    let mut pools = BTreeMap::new();
    pools.insert(
        "a".repeat(NAME_MAX_BYTES + 1),
        ScalePool {
            canister_role: CanisterRole::from("replica"),
            policy: ScalePoolPolicy::default(),
        },
    );

    canisters.insert(
        CanisterRole::from("replica"),
        base_canister_config(CanisterKind::Replica),
    );

    let manager_cfg = CanisterConfig {
        scaling: Some(ScalingConfig { pools }),
        ..base_canister_config(CanisterKind::Singleton)
    };

    canisters.insert(CanisterRole::from("manager"), manager_cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected scaling pool name length to fail");
}

#[test]
fn directory_pool_references_must_exist_in_subnet() {
    let managing_role: CanisterRole = "project_hub".into();
    let mut canisters = BTreeMap::new();

    let mut directory = DirectoryConfig::default();
    directory.pools.insert(
        "projects".into(),
        DirectoryPool {
            canister_role: CanisterRole::from("missing_project_instance"),
            key_name: "project".into(),
        },
    );

    let manager_cfg = CanisterConfig {
        directory: Some(directory),
        ..base_canister_config(CanisterKind::Singleton)
    };

    canisters.insert(managing_role, manager_cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected missing directory target role to fail");
}

#[test]
fn directory_pool_target_must_be_instance_kind() {
    let managing_role: CanisterRole = "project_hub".into();
    let mut canisters = BTreeMap::new();

    let mut directory = DirectoryConfig::default();
    directory.pools.insert(
        "projects".into(),
        DirectoryPool {
            canister_role: CanisterRole::from("project_instance"),
            key_name: "project".into(),
        },
    );

    canisters.insert(
        CanisterRole::from("project_instance"),
        base_canister_config(CanisterKind::Singleton),
    );
    canisters.insert(
        managing_role,
        CanisterConfig {
            directory: Some(directory),
            ..base_canister_config(CanisterKind::Singleton)
        },
    );

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected non-instance directory target role to fail");
}

#[test]
fn directory_pool_requires_non_empty_key_name() {
    let managing_role: CanisterRole = "project_hub".into();
    let mut canisters = BTreeMap::new();

    let mut directory = DirectoryConfig::default();
    directory.pools.insert(
        "projects".into(),
        DirectoryPool {
            canister_role: CanisterRole::from("project_instance"),
            key_name: String::new(),
        },
    );

    canisters.insert(
        CanisterRole::from("project_instance"),
        base_canister_config(CanisterKind::Instance),
    );
    canisters.insert(
        managing_role,
        CanisterConfig {
            directory: Some(directory),
            ..base_canister_config(CanisterKind::Singleton)
        },
    );

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected empty directory key name to fail");
}

#[test]
fn randomness_interval_requires_positive_value() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        randomness: RandomnessConfig {
            enabled: true,
            reseed_interval_secs: 0,
            ..Default::default()
        },
        ..base_canister_config(CanisterKind::Singleton)
    };

    canisters.insert(CanisterRole::from("app"), cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected invalid randomness interval to fail");
}

#[test]
fn wasm_store_canister_config_is_implicit() {
    let subnet = SubnetConfig::default();
    let cfg = subnet
        .get_canister(&CanisterRole::WASM_STORE)
        .expect("expected implicit wasm_store canister");

    assert_eq!(cfg.kind, CanisterKind::Singleton);
    assert_eq!(cfg.initial_cycles, defaults::initial_cycles());
}

#[test]
fn explicit_wasm_store_canister_config_is_rejected() {
    let mut canisters = BTreeMap::new();
    canisters.insert(
        CanisterRole::WASM_STORE,
        base_canister_config(CanisterKind::Singleton),
    );

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected explicit wasm_store config to fail");
}

#[test]
fn topup_policy_amount_above_half_threshold_fails() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        topup_policy: Some(TopupPolicy {
            threshold: Cycles::new(10 * TC),
            amount: Cycles::new(6 * TC),
        }),
        ..base_canister_config(CanisterKind::Singleton)
    };

    canisters.insert(CanisterRole::from("app"), cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected topup_policy amount above half threshold to fail");
}

#[test]
fn topup_policy_amount_equal_half_threshold_is_valid() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        topup_policy: Some(TopupPolicy {
            threshold: Cycles::new(50 * TC),
            amount: Cycles::new(25 * TC),
        }),
        ..base_canister_config(CanisterKind::Singleton)
    };

    canisters.insert(CanisterRole::from("app"), cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect("expected topup_policy amount equal to half threshold to validate");
}

#[test]
fn topup_policy_amount_below_half_threshold_is_valid() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        topup_policy: Some(TopupPolicy {
            threshold: Cycles::new(10 * TC),
            amount: Cycles::new(4 * TC),
        }),
        ..base_canister_config(CanisterKind::Singleton)
    };

    canisters.insert(CanisterRole::from("app"), cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect("expected topup_policy amount below half threshold to validate");
}

#[test]
fn default_topup_policy_is_below_half_threshold() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        topup_policy: Some(TopupPolicy::default()),
        ..base_canister_config(CanisterKind::Singleton)
    };

    canisters.insert(CanisterRole::from("app"), cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect("expected default topup_policy to satisfy half-threshold invariant");
}

#[test]
fn shard_kind_allows_missing_sharding_config() {
    let mut canisters = BTreeMap::new();
    canisters.insert(
        CanisterRole::from("shard"),
        base_canister_config(CanisterKind::Shard),
    );

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect("expected shard config without sharding to validate");
}

#[test]
fn explicit_canister_role_is_rejected() {
    toml::from_str::<SubnetConfig>(
        r#"
[canisters.app]
role = "app"
kind = "singleton"
"#,
    )
    .expect_err("expected explicit role to fail validation");
}

#[test]
fn explicit_canister_type_is_rejected() {
    toml::from_str::<SubnetConfig>(
        r#"
[canisters.app]
kind = "singleton"
type = "singleton"
"#,
    )
    .expect_err("expected explicit type to fail validation");
}

#[test]
fn explicit_sharding_role_is_rejected() {
    toml::from_str::<SubnetConfig>(
        r#"
[canisters.manager]
kind = "singleton"

[canisters.manager.sharding]
role = "shard"
"#,
    )
    .expect_err("expected explicit sharding role to fail validation");
}

#[test]
fn instance_kind_parses() {
    let subnet = toml::from_str::<SubnetConfig>(
        r#"
[canisters.instance_role]
kind = "instance"
"#,
    )
    .expect("expected instance kind to parse");

    let cfg = subnet
        .canisters
        .get(&CanisterRole::from("instance_role"))
        .expect("instance role config should exist");
    assert_eq!(cfg.kind, CanisterKind::Instance);
}

#[test]
fn removed_node_kind_is_rejected() {
    toml::from_str::<SubnetConfig>(
        r#"
[canisters.app]
kind = "node"
"#,
    )
    .expect_err("expected removed node kind to fail parsing");
}

#[test]
fn removed_worker_kind_is_rejected() {
    toml::from_str::<SubnetConfig>(
        r#"
[canisters.app]
kind = "worker"
"#,
    )
    .expect_err("expected removed worker kind to fail parsing");
}
