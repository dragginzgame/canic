use super::canonical::{public_key_hash, root_key_certificate_payload_hash};
use crate::{
    cdk::types::Principal,
    dto::auth::{RootKeyCertificateV2, RootPublicKeyV2, RootTrustAnchorV2, SignatureAlgorithmV2},
};
use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
pub enum RootKeyResolutionV2Error {
    #[error("delegated auth v2 root pid mismatch (expected {expected}, found {found})")]
    RootPidMismatch {
        expected: Principal,
        found: Principal,
    },
    #[error("delegated auth v2 root key is unknown")]
    UnknownRootKey,
    #[error("delegated auth v2 root key hash mismatch")]
    RootKeyHashMismatch,
    #[error("delegated auth v2 root key certificate mismatch")]
    RootKeyCertificateMismatch,
    #[error("delegated auth v2 root key authority hash mismatch")]
    RootKeyAuthorityHashMismatch,
    #[error("delegated auth v2 root key signature is invalid: {0}")]
    RootKeyCertificateSignatureInvalid(String),
    #[error("delegated auth v2 root key not valid yet (not_before {not_before}, now {now_secs})")]
    RootKeyNotYetValid { not_before: u64, now_secs: u64 },
    #[error("delegated auth v2 root key expired at {not_after} (now {now_secs})")]
    RootKeyExpired { not_after: u64, now_secs: u64 },
}

pub struct RootKeyResolveRequestV2<'a> {
    pub root_pid: Principal,
    pub key_id: &'a str,
    pub key_hash: [u8; 32],
    pub alg: SignatureAlgorithmV2,
    pub embedded_key: Option<&'a [u8]>,
    pub embedded_key_cert: Option<&'a RootKeyCertificateV2>,
    pub now_secs: u64,
}

pub fn resolve_root_key<F>(
    trust: &RootTrustAnchorV2,
    req: RootKeyResolveRequestV2<'_>,
    verify_authority_sig: F,
) -> Result<RootPublicKeyV2, RootKeyResolutionV2Error>
where
    F: FnOnce(&[u8], [u8; 32], &[u8], SignatureAlgorithmV2) -> Result<(), String>,
{
    if req.root_pid != trust.root_pid {
        return Err(RootKeyResolutionV2Error::RootPidMismatch {
            expected: trust.root_pid,
            found: req.root_pid,
        });
    }

    if let Some(key) = find_trusted_key(trust, &req) {
        validate_key_window(key.not_before, key.not_after, req.now_secs)?;
        return Ok(key.clone());
    }

    let embedded_key = req
        .embedded_key
        .ok_or(RootKeyResolutionV2Error::UnknownRootKey)?;
    if public_key_hash(embedded_key) != req.key_hash {
        return Err(RootKeyResolutionV2Error::RootKeyHashMismatch);
    }

    let key_cert = req
        .embedded_key_cert
        .ok_or(RootKeyResolutionV2Error::UnknownRootKey)?;
    let authority = trust
        .key_authority
        .as_ref()
        .ok_or(RootKeyResolutionV2Error::UnknownRootKey)?;

    if key_cert.root_pid != req.root_pid
        || key_cert.key_id != req.key_id
        || key_cert.alg != req.alg
        || key_cert.public_key_sec1.as_slice() != embedded_key
        || key_cert.key_hash != req.key_hash
    {
        return Err(RootKeyResolutionV2Error::RootKeyCertificateMismatch);
    }

    if public_key_hash(&authority.authority_public_key_sec1) != authority.authority_key_hash {
        return Err(RootKeyResolutionV2Error::RootKeyAuthorityHashMismatch);
    }

    validate_key_window(key_cert.not_before, key_cert.not_after, req.now_secs)?;

    verify_authority_sig(
        &authority.authority_public_key_sec1,
        root_key_certificate_payload_hash(key_cert),
        &key_cert.authority_sig,
        authority.authority_alg,
    )
    .map_err(RootKeyResolutionV2Error::RootKeyCertificateSignatureInvalid)?;

    Ok(RootPublicKeyV2 {
        root_pid: req.root_pid,
        key_id: req.key_id.to_string(),
        alg: req.alg,
        public_key_sec1: embedded_key.to_vec(),
        key_hash: req.key_hash,
        not_before: key_cert.not_before,
        not_after: key_cert.not_after,
    })
}

fn find_trusted_key<'a>(
    trust: &'a RootTrustAnchorV2,
    req: &RootKeyResolveRequestV2<'_>,
) -> Option<&'a RootPublicKeyV2> {
    trust.trusted_root_keys.keys.iter().find(|key| {
        key.root_pid == req.root_pid
            && key.key_id == req.key_id
            && key.key_hash == req.key_hash
            && key.alg == req.alg
    })
}

const fn validate_key_window(
    not_before: u64,
    not_after: Option<u64>,
    now_secs: u64,
) -> Result<(), RootKeyResolutionV2Error> {
    if now_secs < not_before {
        return Err(RootKeyResolutionV2Error::RootKeyNotYetValid {
            not_before,
            now_secs,
        });
    }

    if let Some(not_after) = not_after
        && now_secs >= not_after
    {
        return Err(RootKeyResolutionV2Error::RootKeyExpired {
            not_after,
            now_secs,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::auth::{RootKeyAuthorityV2, RootKeySetV2};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn trusted_key() -> RootPublicKeyV2 {
        let public_key_sec1 = vec![2, 3, 4];
        RootPublicKeyV2 {
            root_pid: p(1),
            key_id: "root-key".to_string(),
            alg: SignatureAlgorithmV2::EcdsaP256Sha256,
            key_hash: public_key_hash(&public_key_sec1),
            public_key_sec1,
            not_before: 100,
            not_after: Some(300),
        }
    }

    fn authority() -> RootKeyAuthorityV2 {
        let authority_public_key_sec1 = vec![9, 8, 7];
        RootKeyAuthorityV2 {
            authority_key_id: "authority-key".to_string(),
            authority_alg: SignatureAlgorithmV2::EcdsaP256Sha256,
            authority_key_hash: public_key_hash(&authority_public_key_sec1),
            authority_public_key_sec1,
        }
    }

    fn trust_with_key(key: RootPublicKeyV2) -> RootTrustAnchorV2 {
        RootTrustAnchorV2 {
            root_pid: p(1),
            trusted_root_keys: RootKeySetV2 { keys: vec![key] },
            key_authority: Some(authority()),
        }
    }

    fn trust_without_key() -> RootTrustAnchorV2 {
        RootTrustAnchorV2 {
            root_pid: p(1),
            trusted_root_keys: RootKeySetV2 { keys: vec![] },
            key_authority: Some(authority()),
        }
    }

    fn key_cert(key: &RootPublicKeyV2) -> RootKeyCertificateV2 {
        RootKeyCertificateV2 {
            root_pid: key.root_pid,
            key_id: key.key_id.clone(),
            alg: key.alg,
            public_key_sec1: key.public_key_sec1.clone(),
            key_hash: key.key_hash,
            not_before: key.not_before,
            not_after: key.not_after,
            authority_sig: vec![1, 2, 3],
        }
    }

    fn req<'a>(
        key: &'a RootPublicKeyV2,
        root_pid: Principal,
        embedded_key: Option<&'a [u8]>,
        embedded_key_cert: Option<&'a RootKeyCertificateV2>,
        now_secs: u64,
    ) -> RootKeyResolveRequestV2<'a> {
        RootKeyResolveRequestV2 {
            root_pid,
            key_id: &key.key_id,
            key_hash: key.key_hash,
            alg: key.alg,
            embedded_key,
            embedded_key_cert,
            now_secs,
        }
    }

    #[test]
    fn resolve_root_key_uses_trusted_local_key_without_fallback() {
        let key = trusted_key();
        let resolved = resolve_root_key(
            &trust_with_key(key.clone()),
            req(&key, p(1), None, None, 150),
            |_, _, _, _| Err("must not verify fallback".to_string()),
        )
        .unwrap();

        assert_eq!(resolved, key);
    }

    #[test]
    fn resolve_root_key_requires_cert_for_unknown_embedded_key() {
        let key = trusted_key();

        assert_eq!(
            resolve_root_key(
                &trust_without_key(),
                req(&key, p(1), Some(&key.public_key_sec1), None, 150),
                |_, _, _, _| Ok(()),
            ),
            Err(RootKeyResolutionV2Error::UnknownRootKey)
        );
    }

    #[test]
    fn resolve_root_key_accepts_unknown_key_with_authority_cert() {
        let key = trusted_key();
        let cert = key_cert(&key);
        let expected_hash = root_key_certificate_payload_hash(&cert);

        let resolved = resolve_root_key(
            &trust_without_key(),
            req(&key, p(1), Some(&key.public_key_sec1), Some(&cert), 150),
            |authority_key, hash, sig, alg| {
                assert_eq!(
                    authority_key,
                    authority().authority_public_key_sec1.as_slice()
                );
                assert_eq!(hash, expected_hash);
                assert_eq!(sig, [1, 2, 3]);
                assert_eq!(alg, SignatureAlgorithmV2::EcdsaP256Sha256);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(resolved, key);
    }

    #[test]
    fn resolve_root_key_rejects_self_signed_unknown_key() {
        let key = trusted_key();
        let mut cert = key_cert(&key);
        cert.authority_sig = vec![];

        assert_eq!(
            resolve_root_key(
                &trust_without_key(),
                req(&key, p(1), Some(&key.public_key_sec1), Some(&cert), 150),
                |_, _, _, _| Err("bad signature".to_string()),
            ),
            Err(
                RootKeyResolutionV2Error::RootKeyCertificateSignatureInvalid(
                    "bad signature".to_string(),
                ),
            )
        );
    }

    #[test]
    fn resolve_root_key_enforces_root_pid_binding_before_key_lookup() {
        let key = trusted_key();

        assert_eq!(
            resolve_root_key(
                &trust_with_key(key.clone()),
                req(&key, p(9), None, None, 150),
                |_, _, _, _| Ok(()),
            ),
            Err(RootKeyResolutionV2Error::RootPidMismatch {
                expected: p(1),
                found: p(9),
            })
        );
    }

    #[test]
    fn resolve_root_key_enforces_key_validity_window() {
        let key = trusted_key();

        assert_eq!(
            resolve_root_key(
                &trust_with_key(key.clone()),
                req(&key, p(1), None, None, 99),
                |_, _, _, _| Ok(()),
            ),
            Err(RootKeyResolutionV2Error::RootKeyNotYetValid {
                not_before: 100,
                now_secs: 99,
            })
        );

        assert_eq!(
            resolve_root_key(
                &trust_with_key(key.clone()),
                req(&key, p(1), None, None, 300),
                |_, _, _, _| Ok(()),
            ),
            Err(RootKeyResolutionV2Error::RootKeyExpired {
                not_after: 300,
                now_secs: 300,
            })
        );
    }

    #[test]
    fn resolve_root_key_rejects_embedded_key_hash_mismatch() {
        let key = trusted_key();

        assert_eq!(
            resolve_root_key(
                &trust_without_key(),
                req(&key, p(1), Some(&[0, 1, 2]), Some(&key_cert(&key)), 150),
                |_, _, _, _| Ok(()),
            ),
            Err(RootKeyResolutionV2Error::RootKeyHashMismatch)
        );
    }
}
