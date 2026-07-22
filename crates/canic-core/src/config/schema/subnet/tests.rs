//! Module: config::schema::subnet::tests
//!
//! Responsibility: verify subnet schema defaults, parsing, and validation behavior.
//! Does not own: production subnet schemas or placement workflows.
//! Boundary: test-only checks over subnet config data shapes.

use super::*;
use crate::cdk::types::TC;
use crate::config::schema::{NAME_MAX_BYTES, Validate};
use std::collections::BTreeMap;
use std::str::FromStr;

fn base_canister_config(kind: CanisterKind) -> CanisterConfig {
    CanisterConfig {
        kind,
        initial_cycles: defaults::initial_cycles(),
        topup: None,
        icp_refill: None,
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
fn empty_topup_table_enables_default_topup() {
    let cfg: CanisterConfig = toml::from_str(
        r#"
kind = "singleton"

[topup]
"#,
    )
    .expect("empty topup table should parse");

    let topup = cfg.topup.expect("topup policy should be present");
    assert_eq!(topup.threshold.to_u128(), 10 * TC);
    assert_eq!(topup.amount.to_u128(), 5 * TC);
}

#[test]
fn inline_empty_topup_table_enables_default_topup() {
    let cfg: CanisterConfig =
        toml::from_str("kind = \"singleton\"\ntopup = {}\n").expect("inline topup should parse");

    let topup = cfg.topup.expect("topup policy should be present");
    assert_eq!(topup.threshold.to_u128(), 10 * TC);
    assert_eq!(topup.amount.to_u128(), 5 * TC);
}

#[test]
fn cycles_funding_defaults_are_finite() {
    let cfg: CanisterConfig =
        toml::from_str("kind = \"singleton\"\n").expect("canister config should parse");

    assert_eq!(cfg.cycles_funding.max_per_request.to_u128(), 5 * TC);
    assert_eq!(cfg.cycles_funding.max_per_child.to_u128(), 100 * TC);
    assert_eq!(cfg.cycles_funding.cooldown_secs, 60);
}

#[test]
fn cycles_funding_custom_values_parse() {
    let cfg: CanisterConfig = toml::from_str(
        r#"
kind = "singleton"

[cycles_funding]
max_per_request = "2T"
max_per_child = "11T"
cooldown_secs = 17
"#,
    )
    .expect("cycles funding policy should parse");

    assert_eq!(cfg.cycles_funding.max_per_request.to_u128(), 2 * TC);
    assert_eq!(cfg.cycles_funding.max_per_child.to_u128(), 11 * TC);
    assert_eq!(cfg.cycles_funding.cooldown_secs, 17);
}

#[test]
fn service_kind_parses_and_displays() {
    let cfg: CanisterConfig =
        toml::from_str("kind = \"service\"\n").expect("service canister kind should parse");

    assert_eq!(cfg.kind, CanisterKind::Service);
    assert_eq!(cfg.kind.to_string(), "service");
}

#[test]
fn root_icp_refill_parses_manual_policy() {
    let cfg: CanisterConfig = toml::from_str(
        r#"
kind = "root"

[topup]
threshold = "10T"
amount = "5T"

[icp_refill]
max_refill_e8s_per_call = 100000000
min_xdr_permyriad_per_icp = 40000
"#,
    )
    .expect("icp refill mvp config should parse");

    let icp_refill = cfg.icp_refill.expect("icp refill policy should be present");

    assert_eq!(icp_refill.max_refill_e8s_per_call, 100_000_000);
    assert_eq!(icp_refill.min_xdr_permyriad_per_icp, Some(40_000));
    assert_eq!(icp_refill.ledger_canister_id, None);
    assert_eq!(icp_refill.cmc_canister_id, None);
    assert!(!icp_refill.allow_ic_system_canister_overrides);
}

#[test]
fn root_icp_refill_parses_system_canister_overrides() {
    let cfg: CanisterConfig = toml::from_str(
        r#"
kind = "root"

[topup]
threshold = "10T"
amount = "5T"

[icp_refill]
max_refill_e8s_per_call = 100000000
ledger_canister_id = "ryjl3-tyaaa-aaaaa-aaaba-cai"
cmc_canister_id = "rkp4c-7iaaa-aaaaa-aaaca-cai"
allow_ic_system_canister_overrides = true
"#,
    )
    .expect("icp refill canister ID overrides should parse");

    let icp_refill = cfg.icp_refill.expect("icp refill policy should be present");

    assert_eq!(
        icp_refill.ledger_canister_id,
        Some(Principal::from_str("ryjl3-tyaaa-aaaaa-aaaba-cai").expect("valid ledger principal"))
    );
    assert_eq!(
        icp_refill.cmc_canister_id,
        Some(Principal::from_str("rkp4c-7iaaa-aaaaa-aaaca-cai").expect("valid CMC principal"))
    );
    assert!(icp_refill.allow_ic_system_canister_overrides);
}

#[test]
fn root_icp_refill_rejects_followup_knobs() {
    toml::from_str::<CanisterConfig>(
        r#"
kind = "root"

[topup]
threshold = "10T"
amount = "5T"

[icp_refill]
max_refill_e8s_per_call = 100000000
max_refill_e8s_per_day = 1000000000
"#,
    )
    .expect_err("follow-up treasury knobs should not parse in the current subnet config");
}

#[test]
fn root_icp_refill_rejects_redundant_enabled_key() {
    toml::from_str::<CanisterConfig>(
        r#"
kind = "root"

[icp_refill]
enabled = true
max_refill_e8s_per_call = 100000000
"#,
    )
    .expect_err("refill policy presence is the only enablement authority");
}

#[test]
fn nonroot_icp_refill_policy_is_rejected() {
    let mut canisters = BTreeMap::new();
    let cfg = CanisterConfig {
        icp_refill: Some(IcpRefillPolicy {
            max_refill_e8s_per_call: 100_000_000,
            min_xdr_permyriad_per_icp: None,
            ledger_canister_id: None,
            cmc_canister_id: None,
            allow_ic_system_canister_overrides: false,
        }),
        ..base_canister_config(CanisterKind::Service)
    };
    canisters.insert(CanisterRole::from("app"), cfg);

    SubnetConfig {
        canisters,
        ..Default::default()
    }
    .validate()
    .expect_err("nonroot refill policy must fail validation");
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
fn diagnostics_memory_ledger_defaults_off() {
    let cfg: CanisterConfig = toml::from_str(
        r#"
kind = "singleton"
"#,
    )
    .expect("minimal canister config should parse");

    assert!(!cfg.diagnostics.memory_ledger);
}

#[test]
fn diagnostics_memory_ledger_parses_explicit_opt_in() {
    let cfg: CanisterConfig = toml::from_str(
        r#"
kind = "singleton"

[diagnostics]
memory_ledger = true
"#,
    )
    .expect("diagnostics memory ledger config should parse");

    assert!(cfg.diagnostics.memory_ledger);
}

#[test]
fn root_canister_rejects_configured_auth_roles() {
    let mut cfg = base_canister_config(CanisterKind::Root);
    cfg.auth = CanisterAuthConfig {
        delegated_token_issuer: true,
        delegated_token_verifier: true,
        role_attestation_cache: true,
    };

    let mut subnet = SubnetConfig::default();
    subnet.canisters.insert(CanisterRole::ROOT, cfg);

    subnet.validate().expect_err(
        "root delegated auth verifier/issuer/cache roles must be implicit services, not config toggles",
    );
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
fn service_roles_are_derived_for_auto_create_and_subnet_index() {
    let mut subnet = SubnetConfig::default();
    subnet.canisters.insert(
        CanisterRole::from("app"),
        base_canister_config(CanisterKind::Service),
    );
    subnet.canisters.insert(
        CanisterRole::from("ledger"),
        base_canister_config(CanisterKind::Singleton),
    );
    subnet.canisters.insert(
        CanisterRole::from("worker"),
        base_canister_config(CanisterKind::Replica),
    );

    let auto_create = subnet.auto_create_roles();
    let subnet_index = subnet.subnet_index_roles();

    assert!(auto_create.contains("app"));
    assert!(!auto_create.contains("ledger"));
    assert!(!auto_create.contains("worker"));
    assert_eq!(auto_create, subnet_index);
}

#[test]
fn sharding_pool_policy_requires_positive_capacity_and_shards() {
    let managing_role: CanisterRole = "shard_hub".into();
    let worker_role: CanisterRole = "shard_worker".into();
    let mut canisters = BTreeMap::new();

    let mut sharding = ShardingConfig::default();
    sharding.pools.insert(
        "primary".into(),
        ShardPool {
            canister_role: worker_role.clone(),
            policy: ShardPoolPolicy {
                capacity: 0,
                initial_shards: 1,
                max_shards: 0,
            },
        },
    );

    canisters.insert(worker_role, base_canister_config(CanisterKind::Shard));
    canisters.insert(
        managing_role,
        CanisterConfig {
            sharding: Some(sharding),
            ..base_canister_config(CanisterKind::Service)
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
            ..base_canister_config(CanisterKind::Service)
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
            ..base_canister_config(CanisterKind::Service)
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
        ..base_canister_config(CanisterKind::Service)
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
        ..base_canister_config(CanisterKind::Service)
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
        ..base_canister_config(CanisterKind::Service)
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
        ..base_canister_config(CanisterKind::Service)
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
            ..base_canister_config(CanisterKind::Service)
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
            ..base_canister_config(CanisterKind::Service)
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
fn service_kind_can_own_directory_pool() {
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
        base_canister_config(CanisterKind::Instance),
    );
    canisters.insert(
        managing_role,
        CanisterConfig {
            directory: Some(directory),
            ..base_canister_config(CanisterKind::Service)
        },
    );

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect("service manager should accept directory pools");
}

#[test]
fn singleton_kind_cannot_own_manager_pools() {
    let role: CanisterRole = "project_ledger".into();
    let mut directory = DirectoryConfig::default();
    directory.pools.insert(
        "projects".into(),
        DirectoryPool {
            canister_role: CanisterRole::from("project_instance"),
            key_name: "project".into(),
        },
    );

    let mut canisters = BTreeMap::new();
    canisters.insert(
        CanisterRole::from("project_instance"),
        base_canister_config(CanisterKind::Instance),
    );
    canisters.insert(
        role,
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
        .expect_err("singleton manager pools should be rejected");
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
fn topup_amount_above_half_threshold_fails() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        topup: Some(TopupPolicy {
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
        .expect_err("expected topup amount above half threshold to fail");
}

#[test]
fn topup_amount_equal_half_threshold_is_valid() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        topup: Some(TopupPolicy {
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
        .expect("expected topup amount equal to half threshold to validate");
}

#[test]
fn topup_amount_below_half_threshold_is_valid() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        topup: Some(TopupPolicy {
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
        .expect("expected topup amount below half threshold to validate");
}

#[test]
fn default_topup_satisfies_half_threshold_invariant() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        topup: Some(TopupPolicy::default()),
        ..base_canister_config(CanisterKind::Singleton)
    };

    canisters.insert(CanisterRole::from("app"), cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect("expected default topup to satisfy half-threshold invariant");
}

#[test]
fn cycles_funding_zero_max_per_request_fails() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        cycles_funding: CyclesFundingPolicyConfig {
            max_per_request: Cycles::new(0),
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
        .expect_err("expected zero max_per_request to fail");
}

#[test]
fn cycles_funding_zero_max_per_child_fails() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        cycles_funding: CyclesFundingPolicyConfig {
            max_per_child: Cycles::new(0),
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
        .expect_err("expected zero max_per_child to fail");
}

#[test]
fn cycles_funding_zero_cooldown_fails() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        cycles_funding: CyclesFundingPolicyConfig {
            cooldown_secs: 0,
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
        .expect_err("expected zero cooldown to fail");
}

#[test]
fn cycles_funding_request_limit_cannot_exceed_child_budget() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        cycles_funding: CyclesFundingPolicyConfig {
            max_per_request: Cycles::new(11 * TC),
            max_per_child: Cycles::new(10 * TC),
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
        .expect_err("expected request limit above child budget to fail");
}

#[test]
fn root_icp_refill_zero_max_refill_fails() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        icp_refill: Some(IcpRefillPolicy {
            max_refill_e8s_per_call: 0,
            min_xdr_permyriad_per_icp: None,
            ledger_canister_id: None,
            cmc_canister_id: None,
            allow_ic_system_canister_overrides: false,
        }),
        ..base_canister_config(CanisterKind::Root)
    };

    canisters.insert(CanisterRole::ROOT, cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected zero icp refill max refill to fail");
}

#[test]
fn root_icp_refill_zero_rate_gate_fails() {
    let mut canisters = BTreeMap::new();

    let cfg = CanisterConfig {
        icp_refill: Some(IcpRefillPolicy {
            max_refill_e8s_per_call: 100_000_000,
            min_xdr_permyriad_per_icp: Some(0),
            ledger_canister_id: None,
            cmc_canister_id: None,
            allow_ic_system_canister_overrides: false,
        }),
        ..base_canister_config(CanisterKind::Root)
    };

    canisters.insert(CanisterRole::ROOT, cfg);

    let subnet = SubnetConfig {
        canisters,
        ..Default::default()
    };

    subnet
        .validate()
        .expect_err("expected zero icp refill rate gate to fail");
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
