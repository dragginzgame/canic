//! Module: config::schema::tests
//!
//! Responsibility: verify config schema validation and helper behavior.
//! Does not own: production config schemas or runtime config storage.
//! Boundary: test-only checks over schema models and validation implementations.

use super::*;
use crate::{cdk::types::Cycles, domain::auth::MAINNET_IC_ROOT_PUBLIC_KEY_RAW};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

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

fn collect_canic_configs(root: &Path, configs: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(root).unwrap_or_else(|err| panic!("read {} failed: {err}", root.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("read entry in {} failed: {err}", root.display()))
            .path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.is_dir() {
            if !matches!(name, ".git" | ".tmp" | "target") {
                collect_canic_configs(&path, configs);
            }
        } else if name == "canic.toml" {
            configs.push(path);
        }
    }
}

fn component_spec_id(value: &str) -> ComponentSpecId {
    value.parse().expect("valid Component Spec ID")
}

fn default_component_spec_id() -> ComponentSpecId {
    component_spec_id("default")
}

fn component_spec_config(role: &str, maximum_instances: u32) -> ComponentSpecConfig {
    ComponentSpecConfig {
        component_role: CanisterRole::owned(role.to_string()),
        maximum_instances,
        limits: ComponentLimitsConfig::default(),
        initial_cycles: Cycles::new(0),
        topup: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        index: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
        provisions: BTreeMap::default(),
        children: BTreeMap::default(),
        spawn_grants: BTreeMap::default(),
    }
}

fn provisioning_grant(
    maximum_instances_per_requester_per_root: u32,
) -> ComponentProvisioningGrantConfig {
    ComponentProvisioningGrantConfig {
        maximum_instances_per_requester_per_root,
    }
}

#[test]
fn component_role_must_be_admitted() {
    let mut cfg = ConfigModel::test_default();
    cfg.component_specs
        .get_mut(&default_component_spec_id())
        .expect("default Component Spec")
        .component_role = CanisterRole::from("Invalid");

    cfg.validate()
        .expect_err("invalid Component role should fail validation");
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
fn app_name_must_not_exceed_the_canonical_name_limit() {
    let mut cfg = ConfigModel::test_default();
    cfg.app.name = AppId::from("a".repeat(NAME_MAX_BYTES + 1));

    cfg.validate()
        .expect_err("App name over the canonical limit should fail");
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
        "apps/test/canic.toml",
        "apps/test/test-configs/root-capability.toml",
        "apps/test/test-configs/root-scaling.toml",
        "apps/test/test-configs/root-sharding.toml",
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
fn every_checked_in_canic_config_parses_and_validates() {
    let root = workspace_root();
    let mut configs = Vec::new();
    collect_canic_configs(&root, &mut configs);
    configs.sort();
    assert_eq!(configs.len(), 22, "checked-in canic.toml inventory changed");

    for path in configs {
        let rel_path = path.strip_prefix(&root).unwrap_or(&path).display();
        let source =
            fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {rel_path} failed: {err}"));
        crate::bootstrap::parse_config_model(&source)
            .unwrap_or_else(|err| panic!("{rel_path} should parse and validate: {err}"));
    }
}

#[test]
fn topology_roles_must_be_declared() {
    let mut cfg = ConfigModel::test_default();
    cfg.component_specs
        .get_mut(&default_component_spec_id())
        .unwrap()
        .component_role = CanisterRole::from("missing");

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
    cfg.component_specs.clear();
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
fn topology_less_config_may_declare_root_infrastructure() {
    let mut root_cfg = ConfigModel::test_default();
    root_cfg.component_specs.clear();
    root_cfg.roles.insert(
        CanisterRole::ROOT,
        RoleDeclaration {
            kind: RoleDeclarationKind::Root,
            package: "root".to_string(),
        },
    );

    root_cfg
        .validate()
        .expect("Fleet Subnet Root infrastructure sits outside Component Specs");
}

#[test]
fn component_spec_instance_ceilings_are_fleet_bounded() {
    let mut cfg = ConfigModel::test_default();
    cfg.component_specs
        .get_mut(&default_component_spec_id())
        .expect("default Component Spec")
        .maximum_instances = MAX_FLEET_COMPONENT_INSTANCES;
    cfg.validate()
        .expect("exact Fleet maximum Component-instance bound must validate");

    cfg.roles.insert(
        CanisterRole::from("aux"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "aux".to_string(),
        },
    );
    cfg.component_specs
        .insert(component_spec_id("aux"), component_spec_config("aux", 1));
    assert!(matches!(
        cfg.validate(),
        Err(ConfigSchemaError::ValidationError(_))
    ));
}

#[test]
fn component_roles_cannot_occur_in_multiple_component_specs() {
    let mut cfg = ConfigModel::test_default();
    cfg.component_specs
        .insert(component_spec_id("other"), component_spec_config("app", 1));

    cfg.validate()
        .expect_err("one Component role cannot belong to multiple Component Specs");
}

#[test]
fn provisioning_grant_graph_requires_existing_distinct_acyclic_specs() {
    let mut self_target = ConfigModel::test_default();
    self_target
        .component_specs
        .get_mut(&default_component_spec_id())
        .expect("default Component Spec")
        .provisions
        .insert(default_component_spec_id(), provisioning_grant(1));
    self_target.validate().expect_err("self grant must reject");

    let mut missing = ConfigModel::test_default();
    missing
        .component_specs
        .get_mut(&default_component_spec_id())
        .expect("default Component Spec")
        .provisions
        .insert(component_spec_id("missing"), provisioning_grant(1));
    missing
        .validate()
        .expect_err("missing grant target must reject");

    let mut cyclic = ConfigModel::test_default();
    cyclic.roles.insert(
        CanisterRole::from("aux"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "aux".to_string(),
        },
    );
    let mut aux = component_spec_config("aux", 1);
    aux.provisions
        .insert(default_component_spec_id(), provisioning_grant(1));
    cyclic
        .component_specs
        .get_mut(&default_component_spec_id())
        .expect("default Component Spec")
        .provisions
        .insert(component_spec_id("aux"), provisioning_grant(1));
    cyclic.component_specs.insert(component_spec_id("aux"), aux);
    cyclic.validate().expect_err("grant cycle must reject");
}

#[test]
fn potential_descendant_roles_may_be_reused_across_component_specs() {
    let mut cfg = ConfigModel::test_default();
    let shared_role = CanisterRole::from("shared_worker");
    let shared_child = ComponentChildConfig {
        kind: ComponentChildKind::Replica,
        initial_cycles: Cycles::new(0),
        topup: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        index: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    };
    let default = cfg
        .component_specs
        .get_mut(&default_component_spec_id())
        .expect("default Component Spec");
    default
        .children
        .insert(shared_role.clone(), shared_child.clone());
    default.spawn_grants.insert(
        CanisterRole::from("app"),
        BTreeMap::from([(
            shared_role.clone(),
            ComponentSpawnGrantConfig {
                maximum_instances_per_parent: 4,
            },
        )]),
    );
    let mut other = component_spec_config("aux", 1);
    other.children.insert(shared_role.clone(), shared_child);
    other.spawn_grants.insert(
        CanisterRole::from("aux"),
        BTreeMap::from([(
            shared_role.clone(),
            ComponentSpawnGrantConfig {
                maximum_instances_per_parent: 4,
            },
        )]),
    );
    cfg.component_specs
        .insert(component_spec_id("other"), other);
    for role in ["aux", "shared_worker"] {
        cfg.roles.insert(
            CanisterRole::from(role),
            RoleDeclaration {
                kind: RoleDeclarationKind::Canister,
                package: role.to_string(),
            },
        );
    }

    cfg.validate()
        .expect("one declared child artifact may belong to several Specs");
    assert!(
        cfg.component_spec_for_role(&shared_role).is_none(),
        "role-only lookup must not choose between owning Specs"
    );
}

#[test]
fn a_component_role_may_also_be_a_potential_child_role() {
    let mut cfg = ConfigModel::test_default();
    let mut other = component_spec_config("aux", 1);
    other.children.insert(
        CanisterRole::from("app"),
        ComponentChildConfig {
            kind: ComponentChildKind::Singleton,
            initial_cycles: Cycles::new(0),
            topup: None,
            cycles_funding: CyclesFundingPolicyConfig::default(),
            scaling: None,
            sharding: None,
            index: None,
            auth: CanisterAuthConfig::default(),
            standards: StandardsCanisterConfig::default(),
            diagnostics: DiagnosticsCanisterConfig::default(),
            metrics: MetricsCanisterConfig::default(),
        },
    );
    other.spawn_grants.insert(
        CanisterRole::from("aux"),
        BTreeMap::from([(
            CanisterRole::from("app"),
            ComponentSpawnGrantConfig {
                maximum_instances_per_parent: 1,
            },
        )]),
    );
    cfg.roles.insert(
        CanisterRole::from("aux"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "aux".to_string(),
        },
    );
    cfg.component_specs
        .insert(component_spec_id("other"), other);

    cfg.validate()
        .expect("flat declarations may reuse a Component role in a runtime child catalog");
}

#[test]
fn attached_and_deployable_roles_follow_structural_ownership() {
    let mut cfg = ConfigModel::test_default();
    let default_component_spec = cfg
        .component_specs
        .get_mut(&default_component_spec_id())
        .unwrap();
    default_component_spec.component_role = CanisterRole::from("user_hub");
    default_component_spec.children.insert(
        CanisterRole::from("user_shard"),
        ComponentChildConfig {
            kind: ComponentChildKind::Shard,
            initial_cycles: Cycles::new(0),
            topup: None,
            cycles_funding: CyclesFundingPolicyConfig::default(),
            scaling: None,
            sharding: None,
            index: None,
            auth: CanisterAuthConfig::default(),
            standards: StandardsCanisterConfig::default(),
            diagnostics: DiagnosticsCanisterConfig::default(),
            metrics: MetricsCanisterConfig::default(),
        },
    );
    default_component_spec.spawn_grants.insert(
        CanisterRole::from("user_hub"),
        BTreeMap::from([(
            CanisterRole::from("user_shard"),
            ComponentSpawnGrantConfig {
                maximum_instances_per_parent: 4,
            },
        )]),
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
    let attached = cfg.attached_roles();

    assert!(!attached.contains(&CanisterRole::ROOT));
    assert!(cfg.deployable_roles().contains(&CanisterRole::ROOT));
    assert!(attached.contains(&CanisterRole::from("user_hub")));
    assert!(attached.contains(&CanisterRole::from("user_shard")));
}

#[test]
fn component_role_cannot_be_root() {
    let mut cfg = ConfigModel::test_default();
    cfg.component_specs
        .get_mut(&default_component_spec_id())
        .unwrap()
        .component_role = CanisterRole::ROOT;

    cfg.validate()
        .expect_err("Fleet Subnet Root cannot be a Component");
}

#[test]
fn app_cannot_declare_the_built_in_fleet_coordinator_role() {
    let mut cfg = ConfigModel::test_default();
    cfg.roles.insert(
        CanisterRole::FLEET_COORDINATOR,
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "coordinator".to_string(),
        },
    );

    cfg.validate()
        .expect_err("Fleet Coordinator is reserved infrastructure");
}

#[test]
fn several_component_specs_may_define_distinct_components() {
    let mut cfg = ConfigModel::test_default();
    cfg.roles.insert(
        CanisterRole::from("aux"),
        RoleDeclaration {
            kind: RoleDeclarationKind::Canister,
            package: "aux".to_string(),
        },
    );

    cfg.component_specs
        .insert(component_spec_id("aux"), component_spec_config("aux", 2));

    cfg.validate()
        .expect("distinct flat Component Specs should validate");
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
