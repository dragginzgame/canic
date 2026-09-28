use super::*;
use crate::config::{
    Config, ConfigError, RoleRuntimeAuthority,
    schema::{ConfigModel, ConfigSchemaError},
};

fn model() -> ConfigModel {
    let mut model = ConfigModel::test_default();
    let config = &mut model.auth.delegated_tokens;
    config.root_canister_id = Some("5lnwm-ziaaa-aaaae-agtqa-cai".into());
    config.build_network = BuildNetwork::Ic;
    config.chain_key_root_proof.public_key_derivation = Some(ChainKeyPublicKeyDerivation::Ic);
    config.chain_key_root_proof.public_key_hex = None;
    config.chain_key_root_proof.derivation_path_hash_hex = None;
    model
}

fn source(model: &ConfigModel) -> String {
    #[derive(serde::Serialize)]
    struct AuthSource<'a> {
        auth: &'a crate::config::schema::AuthConfig,
    }
    let auth = toml::to_string(&AuthSource { auth: &model.auth }).unwrap();
    format!(
        r#"
[app]
name = "offline"
[roles.root]
kind = "root"
[roles.app]
kind = "canister"
package = "app"
[component_specs.default]
component_role = "app"
maximum_instances = 1
{auth}
"#
    )
}

#[test]
fn offline_keys_materialize_before_role_rendering_and_round_trip() {
    let config = Config::parse_toml(&source(&model())).unwrap();
    let proof = &config.auth.delegated_tokens.chain_key_root_proof;
    let key = decode_hex(proof.public_key_hex.as_ref().unwrap()).unwrap();
    assert_eq!(key.len(), 33);
    let resolved = source(&config);
    assert_eq!(resolved, source(&Config::parse_toml(&resolved).unwrap()));
    let authority = RoleRuntimeAuthority::compile(&config, &"app".into()).unwrap();
    assert_eq!(
        authority
            .auth
            .delegated_tokens
            .chain_key_root_proof
            .public_key_hex,
        proof.public_key_hex
    );
}

#[test]
fn offline_key_binds_root_path_and_network() {
    let base = model().auth.delegated_tokens;
    let key = |mut config: DelegatedTokenConfig| {
        resolve(&mut config).unwrap();
        config.chain_key_root_proof.public_key_hex.unwrap()
    };
    let original = key(base.clone());
    let mut changed = base.clone();
    changed.root_canister_id = Some("lyzo5-yiaaa-aaaae-agxvq-cai".into());
    assert_ne!(original, key(changed));
    let mut changed = base.clone();
    changed.chain_key_root_proof.derivation_path_hex = Some(vec!["00".into()]);
    assert_ne!(original, key(changed));
    let mut changed = base;
    changed.build_network = BuildNetwork::Local;
    changed.chain_key_root_proof.public_key_derivation =
        Some(ChainKeyPublicKeyDerivation::Pocketic);
    assert_ne!(original, key(changed));
}

#[test]
fn offline_rejects_wrong_environment_unknown_key_and_missing_root() {
    let mut config = model().auth.delegated_tokens;
    config.chain_key_root_proof.public_key_derivation = Some(ChainKeyPublicKeyDerivation::Pocketic);
    assert_eq!(
        resolve(&mut config),
        Err(ChainKeyDerivationError::NetworkMismatch {
            derivation: ChainKeyPublicKeyDerivation::Pocketic,
            network: BuildNetwork::Ic,
        })
    );
    config.chain_key_root_proof.public_key_derivation = Some(ChainKeyPublicKeyDerivation::Ic);
    config.root_canister_id = None;
    assert_eq!(
        resolve(&mut config),
        Err(ChainKeyDerivationError::MissingField {
            field: "root_canister_id"
        })
    );
    config.root_canister_id = Some(Principal::anonymous().to_text());
    assert_eq!(
        resolve(&mut config),
        Err(ChainKeyDerivationError::InvalidRoot)
    );
    config.root_canister_id = model().auth.delegated_tokens.root_canister_id;
    config.chain_key_root_proof.key_id = Some("future_key".into());
    assert_eq!(
        resolve(&mut config),
        Err(ChainKeyDerivationError::UnsupportedKey {
            derivation: ChainKeyPublicKeyDerivation::Ic,
            key_id: "future_key".into(),
        })
    );
}

#[test]
fn offline_rejects_stale_material_before_build() {
    let mut model = model();
    resolve(&mut model.auth.delegated_tokens).unwrap();
    model.auth.delegated_tokens.root_canister_id = Some("lyzo5-yiaaa-aaaae-agxvq-cai".into());
    assert!(matches!(
        Config::parse_toml(&source(&model)),
        Err(ConfigError::ConfigSchema(
            ConfigSchemaError::ChainKeyDerivation(ChainKeyDerivationError::DerivedValueMismatch {
                field: "chain_key_root_proof.public_key_hex"
            })
        ))
    ));
}

#[test]
fn offline_rejects_malformed_paths_and_does_not_replace_partial_values() {
    let mut config = model().auth.delegated_tokens;
    config.chain_key_root_proof.derivation_path_hex = Some(vec!["zz".into()]);
    assert_eq!(
        resolve(&mut config),
        Err(ChainKeyDerivationError::InvalidHex {
            field: "chain_key_root_proof.derivation_path_hex"
        })
    );
    config.chain_key_root_proof.derivation_path_hex = Some(vec![String::new(); 256]);
    assert_eq!(
        resolve(&mut config),
        Err(ChainKeyDerivationError::PathTooLong)
    );
    config.chain_key_root_proof.derivation_path_hex = Some(vec![]);
    config.chain_key_root_proof.derivation_path_hash_hex = Some("00".repeat(32));
    assert_eq!(
        resolve(&mut config),
        Err(ChainKeyDerivationError::DerivedValueMismatch {
            field: "chain_key_root_proof.derivation_path_hash_hex"
        })
    );
    assert!(config.chain_key_root_proof.public_key_hex.is_none());
}
