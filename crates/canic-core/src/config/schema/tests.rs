use super::*;
use crate::cdk::types::Cycles;
use std::collections::BTreeMap;

fn base_canister_config(kind: CanisterKind) -> CanisterConfig {
    CanisterConfig {
        kind,
        initial_cycles: Cycles::new(0),
        topup_policy: None,
        randomness: RandomnessConfig::default(),
        scaling: None,
        sharding: None,
        directory: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
    }
}

#[test]
fn root_canister_must_exist_in_prime_subnet() {
    let mut cfg = ConfigModel::default();
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
