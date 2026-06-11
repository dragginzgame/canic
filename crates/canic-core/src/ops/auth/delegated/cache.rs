use super::canonical::{CanonicalAuthError, claims_hash, issuer_proof_hash, proof_hash};
use crate::{cdk::types::Principal, dto::auth::DelegatedToken};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::BTreeMap};

const DELEGATED_TOKEN_CACHE_KEY_DOMAIN: &[u8] = b"canic-delegated-token-cache-v1";
const MAX_DELEGATED_TOKEN_PROOF_CACHE_ENTRIES: usize = 1024;

///
/// CachedDelegatedTokenProofValidity
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CachedDelegatedTokenProofValidity {
    pub valid_until_ns: u64,
    pub verified_at_ns: u64,
}

thread_local! {
    static DELEGATED_TOKEN_PROOF_CACHE: RefCell<BTreeMap<[u8; 32], CachedDelegatedTokenProofValidity>> =
        const { RefCell::new(BTreeMap::new()) };
}

pub fn delegated_token_cache_key(
    token: &DelegatedToken,
    caller: Principal,
) -> Result<[u8; 32], CanonicalAuthError> {
    let proof_hash = proof_hash(&token.proof)?;
    let claims_hash = claims_hash(&token.claims)?;
    let signature_hash = issuer_proof_hash(&token.issuer_proof);

    Ok(delegated_token_cache_key_from_hashes(
        proof_hash,
        claims_hash,
        signature_hash,
        caller,
    ))
}

fn delegated_token_cache_key_from_hashes(
    proof_hash: [u8; 32],
    claims_hash: [u8; 32],
    signature_hash: [u8; 32],
    caller: Principal,
) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(
        DELEGATED_TOKEN_CACHE_KEY_DOMAIN.len()
            + proof_hash.len()
            + claims_hash.len()
            + signature_hash.len()
            + caller.as_slice().len(),
    );
    bytes.extend_from_slice(DELEGATED_TOKEN_CACHE_KEY_DOMAIN);
    bytes.extend_from_slice(&proof_hash);
    bytes.extend_from_slice(&claims_hash);
    bytes.extend_from_slice(&signature_hash);
    bytes.extend_from_slice(caller.as_slice());
    hash_bytes(&bytes)
}

pub fn positive_cache_get(key: [u8; 32], now_ns: u64) -> Option<CachedDelegatedTokenProofValidity> {
    DELEGATED_TOKEN_PROOF_CACHE.with_borrow_mut(|cache| {
        let value = cache.get(&key).copied()?;
        if now_ns >= value.valid_until_ns {
            cache.remove(&key);
            return None;
        }
        Some(value)
    })
}

pub fn positive_cache_insert(key: [u8; 32], value: CachedDelegatedTokenProofValidity) {
    if value.verified_at_ns >= value.valid_until_ns {
        return;
    }

    DELEGATED_TOKEN_PROOF_CACHE.with_borrow_mut(|cache| {
        prune_expired(cache, value.verified_at_ns);
        if !cache.contains_key(&key) && cache.len() >= MAX_DELEGATED_TOKEN_PROOF_CACHE_ENTRIES {
            evict_oldest(cache);
        }
        cache.insert(key, value);
    });
}

pub fn positive_cache_remove(key: [u8; 32]) {
    DELEGATED_TOKEN_PROOF_CACHE.with_borrow_mut(|cache| {
        cache.remove(&key);
    });
}

fn prune_expired(cache: &mut BTreeMap<[u8; 32], CachedDelegatedTokenProofValidity>, now_ns: u64) {
    cache.retain(|_, value| now_ns < value.valid_until_ns);
}

fn evict_oldest(cache: &mut BTreeMap<[u8; 32], CachedDelegatedTokenProofValidity>) {
    let Some(oldest_key) = cache
        .iter()
        .min_by_key(|(_, value)| value.verified_at_ns)
        .map(|(key, _)| *key)
    else {
        return;
    };
    cache.remove(&oldest_key);
}

fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
pub(crate) fn positive_cache_clear_for_tests() {
    DELEGATED_TOKEN_PROOF_CACHE.with_borrow_mut(BTreeMap::clear);
}

#[cfg(test)]
pub(crate) fn positive_cache_len_for_tests() -> usize {
    DELEGATED_TOKEN_PROOF_CACHE.with_borrow(BTreeMap::len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{
            DelegatedRoleGrant, DelegatedTokenClaims, DelegationAudience, DelegationCert,
            DelegationProof, IcCanisterSignatureProofV1, IssuerProof, RootProof, ShardKeyBinding,
            ShardSignatureAlgorithm,
        },
        ids::CanisterRole,
        ops::auth::delegated::canonical::cert_hash,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn token() -> DelegatedToken {
        let cert = DelegationCert {
            root_pid: p(1),
            shard_pid: p(2),
            shard_key_id: "shard-key".to_string(),
            shard_sig_alg: ShardSignatureAlgorithm::IcThresholdEcdsaSecp256k1,
            shard_public_key_sec1: vec![8; 33],
            shard_key_hash: [9; 32],
            shard_key_binding: ShardKeyBinding::IcThresholdEcdsaSecp256k1 {
                key_name_hash: [10; 32],
                derivation_path_hash: [11; 32],
            },
            issued_at_ns: 10,
            not_before_ns: 10,
            expires_at_ns: 200,
            max_token_ttl_ns: 60,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![DelegatedRoleGrant {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["read".to_string()],
            }],
        };
        let cert_hash = cert_hash(&cert).expect("cert hash");
        DelegatedToken {
            claims: DelegatedTokenClaims {
                subject: p(9),
                issuer_shard_pid: p(2),
                cert_hash,
                issued_at_ns: 100,
                expires_at_ns: 150,
                aud: DelegationAudience::Project("test".to_string()),
                grants: vec![DelegatedRoleGrant {
                    target: CanisterRole::owned("project_instance".to_string()),
                    scopes: vec!["read".to_string()],
                }],
                nonce: [5; 16],
                ext: None,
            },
            proof: DelegationProof {
                cert,
                root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                    signature_cbor: vec![12; 64],
                    public_key_der: vec![13; 32],
                }),
            },
            issuer_proof: sample_issuer_proof(14),
        }
    }

    #[test]
    fn delegated_token_cache_key_binds_issuer_proof_claims_ext_and_caller() {
        let token = token();
        let key = delegated_token_cache_key(&token, p(9)).expect("key");

        let mut changed_proof = token.clone();
        changed_proof.issuer_proof = sample_issuer_proof(15);
        let proof_key = delegated_token_cache_key(&changed_proof, p(9)).expect("proof key");

        let mut changed_claims = token.clone();
        changed_claims.claims.ext = Some(b"different".to_vec());
        let claims_key = delegated_token_cache_key(&changed_claims, p(9)).expect("claims key");

        let caller_key = delegated_token_cache_key(&token, p(10)).expect("caller key");

        assert_ne!(key, proof_key);
        assert_ne!(key, claims_key);
        assert_ne!(key, caller_key);
    }

    #[test]
    fn positive_cache_hit_expires_at_valid_until_boundary() {
        positive_cache_clear_for_tests();
        let key = [7; 32];
        positive_cache_insert(
            key,
            CachedDelegatedTokenProofValidity {
                valid_until_ns: 20,
                verified_at_ns: 10,
            },
        );

        assert!(positive_cache_get(key, 19).is_some());
        assert_eq!(positive_cache_get(key, 20), None);
        assert_eq!(positive_cache_len_for_tests(), 0);
    }

    #[test]
    fn positive_cache_is_bounded_and_evicts_oldest_entry() {
        positive_cache_clear_for_tests();
        for idx in 0..MAX_DELEGATED_TOKEN_PROOF_CACHE_ENTRIES {
            let mut key = [0; 32];
            key[0..8].copy_from_slice(&(idx as u64).to_be_bytes());
            positive_cache_insert(
                key,
                CachedDelegatedTokenProofValidity {
                    valid_until_ns: 20_000,
                    verified_at_ns: idx as u64,
                },
            );
        }

        let mut extra_key = [0; 32];
        extra_key[0..8]
            .copy_from_slice(&(MAX_DELEGATED_TOKEN_PROOF_CACHE_ENTRIES as u64).to_be_bytes());
        positive_cache_insert(
            extra_key,
            CachedDelegatedTokenProofValidity {
                valid_until_ns: 20_000,
                verified_at_ns: 10_000,
            },
        );

        assert_eq!(
            positive_cache_len_for_tests(),
            MAX_DELEGATED_TOKEN_PROOF_CACHE_ENTRIES
        );
        assert_eq!(positive_cache_get([0; 32], 5), None);
        assert!(positive_cache_get(extra_key, 5).is_some());
    }

    fn sample_issuer_proof(byte: u8) -> IssuerProof {
        IssuerProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: vec![byte; 64],
            public_key_der: vec![byte + 1; 32],
        })
    }
}
