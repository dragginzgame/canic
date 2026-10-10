//! Resolve configured Root public keys before compiling role artifacts.
//!
//! This host-only operation uses IC public derivation; it performs no network or
//! canister calls and never generates or receives a private key.

#[cfg(test)]
mod tests;

use crate::{
    cdk::{
        types::Principal,
        utils::hash::{decode_hex, hex_bytes},
    },
    config::schema::{ChainKeyDerivationError, ChainKeyPublicKeyDerivation, DelegatedTokenConfig},
};
use canic_contracts::ids::BuildNetwork;
use ic_auth::canonical::chain_key_derivation_path_hash;
use ic_secp256k1::{MasterPublicKeyId, PocketIcMasterPublicKeyId, PublicKey};

/// Materialize public verification data without changing operator policy fields.
pub fn resolve(config: &mut DelegatedTokenConfig) -> Result<(), ChainKeyDerivationError> {
    let Some(derivation) = config.chain_key_root_proof.public_key_derivation else {
        return Ok(());
    };
    if !config.enabled {
        return Err(ChainKeyDerivationError::AuthDisabled);
    }
    if !matches!(
        (derivation, config.build_network),
        (ChainKeyPublicKeyDerivation::Ic, BuildNetwork::Ic)
            | (ChainKeyPublicKeyDerivation::Pocketic, BuildNetwork::Local)
    ) {
        return Err(ChainKeyDerivationError::NetworkMismatch {
            derivation,
            network: config.build_network,
        });
    }
    let root_text =
        config
            .root_canister_id
            .as_deref()
            .ok_or(ChainKeyDerivationError::MissingField {
                field: "root_canister_id",
            })?;
    let root = Principal::from_text(root_text).map_err(|_| ChainKeyDerivationError::InvalidRoot)?;
    if root == Principal::anonymous()
        || root == Principal::management_canister()
        || root.as_slice().last() != Some(&1)
    {
        return Err(ChainKeyDerivationError::InvalidRoot);
    }
    let proof = &mut config.chain_key_root_proof;
    let key_id = proof
        .key_id
        .as_deref()
        .ok_or(ChainKeyDerivationError::MissingField {
            field: "chain_key_root_proof.key_id",
        })?;
    let path = proof
        .derivation_path_hex
        .as_ref()
        .ok_or(ChainKeyDerivationError::MissingField {
            field: "chain_key_root_proof.derivation_path_hex",
        })?;
    if path.len() > 255 {
        return Err(ChainKeyDerivationError::PathTooLong);
    }
    let path = path
        .iter()
        .map(|part| {
            decode_hex(part).map_err(|_| ChainKeyDerivationError::InvalidHex {
                field: "chain_key_root_proof.derivation_path_hex",
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let public_key = derive_public_key(derivation, key_id, root, &path)?;
    let path_hash =
        chain_key_derivation_path_hash(&path).map_err(|_| ChainKeyDerivationError::PathTooLong)?;
    check_existing(
        proof.public_key_hex.as_deref(),
        &public_key,
        "chain_key_root_proof.public_key_hex",
    )?;
    check_existing(
        proof.derivation_path_hash_hex.as_deref(),
        &path_hash,
        "chain_key_root_proof.derivation_path_hash_hex",
    )?;
    proof.public_key_hex = Some(hex_bytes(&public_key));
    proof.derivation_path_hash_hex = Some(hex_bytes(path_hash));
    Ok(())
}

fn derive_public_key(
    derivation: ChainKeyPublicKeyDerivation,
    key_id: &str,
    root: Principal,
    path: &[Vec<u8>],
) -> Result<Vec<u8>, ChainKeyDerivationError> {
    let key = match (derivation, key_id) {
        (ChainKeyPublicKeyDerivation::Ic, "key_1") => {
            PublicKey::derive_mainnet_key(MasterPublicKeyId::EcdsaKey1, &root, path).0
        }
        (ChainKeyPublicKeyDerivation::Pocketic, "key_1") => {
            PublicKey::derive_pocketic_key(PocketIcMasterPublicKeyId::EcdsaKey1, &root, path).0
        }
        (ChainKeyPublicKeyDerivation::Pocketic, "test_key_1") => {
            PublicKey::derive_pocketic_key(PocketIcMasterPublicKeyId::EcdsaTestKey1, &root, path).0
        }
        (ChainKeyPublicKeyDerivation::Pocketic, "dfx_test_key") => {
            PublicKey::derive_pocketic_key(PocketIcMasterPublicKeyId::EcdsaDfxTestKey, &root, path)
                .0
        }
        _ => {
            return Err(ChainKeyDerivationError::UnsupportedKey {
                derivation,
                key_id: key_id.into(),
            });
        }
    };
    Ok(key.serialize_sec1(true))
}

fn check_existing(
    value: Option<&str>,
    expected: &[u8],
    field: &'static str,
) -> Result<(), ChainKeyDerivationError> {
    if let Some(value) = value {
        let bytes = decode_hex(value).map_err(|_| ChainKeyDerivationError::InvalidHex { field })?;
        if bytes != expected {
            return Err(ChainKeyDerivationError::DerivedValueMismatch { field });
        }
    }
    Ok(())
}
