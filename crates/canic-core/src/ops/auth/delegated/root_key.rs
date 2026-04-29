use crate::{
    cdk::types::Principal,
    dto::auth::{RootPublicKey, RootTrustAnchor, SignatureAlgorithm},
};
use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
pub enum RootKeyResolutionError {
    #[error("delegated auth root pid mismatch (expected {expected}, found {found})")]
    RootPidMismatch {
        expected: Principal,
        found: Principal,
    },
    #[error("delegated auth root key is unknown")]
    UnknownRootKey,
    #[error("delegated auth root key not valid yet (not_before {not_before}, now {now_secs})")]
    RootKeyNotYetValid { not_before: u64, now_secs: u64 },
    #[error("delegated auth root key expired at {not_after} (now {now_secs})")]
    RootKeyExpired { not_after: u64, now_secs: u64 },
}

pub struct RootKeyResolveRequest<'a> {
    pub root_pid: Principal,
    pub key_id: &'a str,
    pub key_hash: [u8; 32],
    pub alg: SignatureAlgorithm,
    pub now_secs: u64,
}

pub fn resolve_root_key(
    trust: &RootTrustAnchor,
    req: RootKeyResolveRequest<'_>,
) -> Result<RootPublicKey, RootKeyResolutionError> {
    if req.root_pid != trust.root_pid {
        return Err(RootKeyResolutionError::RootPidMismatch {
            expected: trust.root_pid,
            found: req.root_pid,
        });
    }

    let key = &trust.root_key;
    if key.root_pid != req.root_pid
        || key.key_id != req.key_id
        || key.key_hash != req.key_hash
        || key.alg != req.alg
    {
        return Err(RootKeyResolutionError::UnknownRootKey);
    }

    validate_key_window(key.not_before, key.not_after, req.now_secs)?;
    Ok(key.clone())
}

const fn validate_key_window(
    not_before: u64,
    not_after: Option<u64>,
    now_secs: u64,
) -> Result<(), RootKeyResolutionError> {
    if now_secs < not_before {
        return Err(RootKeyResolutionError::RootKeyNotYetValid {
            not_before,
            now_secs,
        });
    }

    if let Some(not_after) = not_after
        && now_secs >= not_after
    {
        return Err(RootKeyResolutionError::RootKeyExpired {
            not_after,
            now_secs,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::auth::delegated::canonical::public_key_hash;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn trusted_key() -> RootPublicKey {
        let public_key_sec1 = vec![2, 3, 4];
        RootPublicKey {
            root_pid: p(1),
            key_id: "root-key".to_string(),
            alg: SignatureAlgorithm::EcdsaP256Sha256,
            key_hash: public_key_hash(&public_key_sec1),
            public_key_sec1,
            not_before: 100,
            not_after: Some(300),
        }
    }

    fn trust_with_key(key: RootPublicKey) -> RootTrustAnchor {
        RootTrustAnchor {
            root_pid: p(1),
            root_key: key,
        }
    }

    fn req<'a>(
        key: &'a RootPublicKey,
        root_pid: Principal,
        now_secs: u64,
    ) -> RootKeyResolveRequest<'a> {
        RootKeyResolveRequest {
            root_pid,
            key_id: &key.key_id,
            key_hash: key.key_hash,
            alg: key.alg,
            now_secs,
        }
    }

    #[test]
    fn resolve_root_key_uses_explicit_trusted_root_key() {
        let key = trusted_key();
        let resolved =
            resolve_root_key(&trust_with_key(key.clone()), req(&key, p(1), 150)).unwrap();

        assert_eq!(resolved, key);
    }

    #[test]
    fn resolve_root_key_rejects_unknown_key() {
        let key = trusted_key();
        let mut other = key.clone();
        other.key_id = "other-root-key".to_string();

        assert_eq!(
            resolve_root_key(&trust_with_key(other), req(&key, p(1), 150)),
            Err(RootKeyResolutionError::UnknownRootKey)
        );
    }

    #[test]
    fn resolve_root_key_enforces_root_pid_binding_before_key_lookup() {
        let key = trusted_key();

        assert_eq!(
            resolve_root_key(&trust_with_key(key.clone()), req(&key, p(9), 150)),
            Err(RootKeyResolutionError::RootPidMismatch {
                expected: p(1),
                found: p(9),
            })
        );
    }

    #[test]
    fn resolve_root_key_enforces_key_validity_window() {
        let key = trusted_key();

        assert_eq!(
            resolve_root_key(&trust_with_key(key.clone()), req(&key, p(1), 99)),
            Err(RootKeyResolutionError::RootKeyNotYetValid {
                not_before: 100,
                now_secs: 99,
            })
        );

        assert_eq!(
            resolve_root_key(&trust_with_key(key.clone()), req(&key, p(1), 300)),
            Err(RootKeyResolutionError::RootKeyExpired {
                not_after: 300,
                now_secs: 300,
            })
        );
    }
}
