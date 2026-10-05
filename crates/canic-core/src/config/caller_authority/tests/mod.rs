//! Module: config::caller_authority::tests
//!
//! Tests for build admission and canonical receiver policy identity.

use super::*;
use crate::config::{
    RoleRuntimeAuthority,
    schema::{ConfigModel, ConfigSchemaError, Validate},
};

fn source(spec: &str, role: &str) -> CallerSourceSelector {
    CallerSourceSelector {
        component_spec: spec.parse().expect("Spec"),
        role: CanisterRole::owned(role.to_string()),
    }
}

#[test]
fn endpoint_permission_metadata_matches_exact_names_only() {
    for name in ["notify", "read_metrics", "lookup"] {
        assert!(permission_is_declared("lookup,notify,read_metrics", name));
    }
    for name in ["", "read", "metrics", "other", "notify,read_metrics"] {
        assert!(!permission_is_declared("lookup,notify,read_metrics", name));
    }
    assert!(!permission_is_declared("", "notify"));
}

fn configuration() -> CallerAuthorityConfig {
    CallerAuthorityConfig {
        maximum_entries: 100,
        maximum_bytes: 100_000,
        permissions: BTreeMap::from([(
            "read_metrics".into(),
            CallerPermission {
                direction: crate::config::caller_authority::CallerPermissionDirection::Caller,
                scope: CallerScope::SameRoot,
                sources: vec![source("default", "app")],
            },
        )]),
    }
}

fn compile(config: CallerAuthorityConfig) -> CompiledCallerPolicy {
    CompiledCallerPolicy::compile(CanisterRole::from("receiver"), Some(config)).expect("policy")
}

#[test]
fn canonical_identity_normalizes_order_and_binds_every_authority_field() {
    let mut config = configuration();
    config
        .permissions
        .get_mut("read_metrics")
        .unwrap()
        .sources
        .push(source("users", "shard"));
    let original = compile(config.clone());
    config
        .permissions
        .get_mut("read_metrics")
        .unwrap()
        .sources
        .reverse();
    assert_eq!(compile(config.clone()), original);
    let mut changed = config.clone();
    changed.maximum_entries += 1;
    assert_ne!(compile(changed).digest, original.digest);
    let mut changed = config.clone();
    changed.maximum_bytes += 1;
    assert_ne!(compile(changed).digest, original.digest);
    let mut changed = config.clone();
    changed.permissions.get_mut("read_metrics").unwrap().scope = CallerScope::SameComponent;
    assert_ne!(compile(changed).digest, original.digest);
    let mut changed = config.clone();
    changed
        .permissions
        .get_mut("read_metrics")
        .unwrap()
        .direction = CallerPermissionDirection::Target;
    assert_ne!(compile(changed).digest, original.digest);
    let mut changed = config.clone();
    let permission = changed.permissions.remove("read_metrics").unwrap();
    changed
        .permissions
        .insert("send_notification".into(), permission);
    assert_ne!(compile(changed).digest, original.digest);
    let mut changed = config.clone();
    changed.permissions.get_mut("read_metrics").unwrap().sources[0].role =
        CanisterRole::from("hub");
    assert_ne!(compile(changed).digest, original.digest);
    let mut changed = config.clone();
    changed.permissions.get_mut("read_metrics").unwrap().sources[0].component_spec =
        "other".parse().unwrap();
    assert_ne!(compile(changed).digest, original.digest);
    assert_ne!(
        CompiledCallerPolicy::compile(CanisterRole::from("other"), Some(config))
            .unwrap()
            .digest,
        original.digest
    );
    assert!(original.is_canonical());
    let mut tampered = original;
    tampered.digest[0] ^= 1;
    assert!(!tampered.is_canonical());
}

#[test]
fn invalid_and_unbounded_policies_are_typed_build_failures() {
    for (entries, bytes) in [
        (0, 1),
        (1, 0),
        (1, CALLER_HEADER_BYTES),
        (MAX_CALLER_ENTRIES + 1, 1),
        (1, MAX_CALLER_BYTES + 1),
    ] {
        let mut config = configuration();
        config.maximum_entries = entries;
        config.maximum_bytes = bytes;
        assert_eq!(
            CompiledCallerPolicy::compile(CanisterRole::from("receiver"), Some(config)),
            Err(CallerPolicyError::Capacity)
        );
    }
    let mut config = configuration();
    config
        .permissions
        .get_mut("read_metrics")
        .unwrap()
        .sources
        .push(source("default", "app"));
    assert_eq!(
        CompiledCallerPolicy::compile(CanisterRole::from("receiver"), Some(config)),
        Err(CallerPolicyError::DuplicateSelector)
    );
    let mut config = configuration();
    config
        .permissions
        .get_mut("read_metrics")
        .unwrap()
        .sources
        .clear();
    assert_eq!(
        CompiledCallerPolicy::compile(CanisterRole::from("receiver"), Some(config)),
        Err(CallerPolicyError::EmptySources)
    );
}

#[test]
fn only_managed_receiver_roles_can_declare_projection_permissions() {
    let mut model = ConfigModel::test_default();
    model
        .roles
        .get_mut(&CanisterRole::ROOT)
        .unwrap()
        .caller_authority = Some(configuration());
    assert!(matches!(
        model.validate(),
        Err(ConfigSchemaError::CallerAuthority(
            CallerPolicyError::UnsupportedReceiver
        ))
    ));
}

#[test]
fn build_validation_preserves_pairs_instead_of_cross_joining_specs_and_roles() {
    let mut model = ConfigModel::test_default();
    let mut second = model.component_specs.values().next().unwrap().clone();
    second.component_role = CanisterRole::from("other");
    model
        .component_specs
        .insert("other".parse().unwrap(), second);
    let declaration = model.roles.get(&CanisterRole::from("app")).unwrap().clone();
    model.roles.insert(CanisterRole::from("other"), declaration);
    let mut config = configuration();
    config.permissions.get_mut("read_metrics").unwrap().sources = vec![source("default", "other")];
    model
        .roles
        .get_mut(&CanisterRole::from("app"))
        .unwrap()
        .caller_authority = Some(config);
    assert!(matches!(
        model.validate(),
        Err(ConfigSchemaError::CallerAuthority(
            CallerPolicyError::UnknownSource
        ))
    ));
    assert!(matches!(
        RoleRuntimeAuthority::compile(&model, &CanisterRole::from("app")),
        Err(crate::config::RoleRuntimeAuthorityError::CallerPolicy(
            CallerPolicyError::UnknownSource
        ))
    ));
}

#[test]
fn omitted_policy_embeds_no_permissions_and_distinct_role_identity() {
    let model = ConfigModel::test_default();
    let authority = RoleRuntimeAuthority::compile(&model, &CanisterRole::from("app")).unwrap();
    assert_eq!(authority.caller_policy.configuration, None);
    assert!(authority.caller_policy.is_canonical());
    assert_ne!(
        authority.caller_policy.digest,
        CompiledCallerPolicy::compile(CanisterRole::ROOT, None)
            .unwrap()
            .digest
    );
}

#[test]
fn toml_policy_is_compiled_into_generated_runtime_source() {
    let mut model = ConfigModel::test_default();
    model
        .roles
        .get_mut(&CanisterRole::from("app"))
        .unwrap()
        .caller_authority = Some(configuration());
    let encoded = toml::to_string(&configuration()).unwrap();
    model
        .roles
        .get_mut(&CanisterRole::from("app"))
        .unwrap()
        .caller_authority = Some(toml::from_str(&encoded).unwrap());
    let decoded = model;
    decoded.validate().unwrap();
    let authority = RoleRuntimeAuthority::compile(&decoded, &CanisterRole::from("app")).unwrap();
    assert_eq!(authority.caller_policy.configuration, Some(configuration()));
    let source = crate::bootstrap::emit_role_runtime_authority_source(
        &decoded,
        &CanisterRole::from("app"),
        false,
    )
    .unwrap();
    assert!(source.contains("CompiledCallerPolicy"));
    assert!(source.contains("read_metrics"));
    assert!(source.contains("SameRoot"));
}
