//! Module: ops::auth::signature_store
//!
//! Responsibility: bind the shared signature store to one host certification root.
//! Boundary: callers authorize fixed signing profiles; IC Auth owns retention and witnesses.

use crate::{InternalError, ops::auth::AuthSignatureError};

use std::cell::RefCell;

use ic_auth::signature_store::{
    MAX_COMPOSITION_DEPTH, MAX_SIGNATURES, SignatureLimits, SignatureSibling, SignatureStore,
    SignatureStoreError,
};
#[cfg(feature = "auth-root-canister-sig-create")]
use ic_auth::signature_store::{MAX_PRUNE_BATCH, PreparedSignature, SignatureInputs};
#[cfg(feature = "auth-root-canister-sig-create")]
use ic_auth_protocol_types::IcCanisterSignatureProofV1;
use ic_platform::principal::ExternalPrincipal as Principal;

/// One /sig owner shared by Root and issuer, with host-owned sibling branches.
struct SignatureHost {
    store: SignatureStore,
    siblings: Vec<SignatureSibling>,
}

impl SignatureHost {
    fn new(signing_canister: Principal) -> Result<Self, SignatureStoreError> {
        Ok(Self {
            store: SignatureStore::new(
                signing_canister.into(),
                SignatureLimits {
                    max_signatures: MAX_SIGNATURES,
                    max_message_bytes: 32,
                    max_certificate_bytes: 64 * 1024,
                    max_signature_bytes: 128 * 1024,
                },
            )?,
            siblings: Vec::new(),
        })
    }

    fn root_hash(&self) -> [u8; 32] {
        self.siblings
            .iter()
            .fold(self.store.root_hash(), |root, sibling| match sibling {
                SignatureSibling::Left(hash) => ic_platform::certification::fork_hash(hash, &root),
                SignatureSibling::Right(hash) => ic_platform::certification::fork_hash(&root, hash),
            })
    }
}

thread_local! {
    static HOST: RefCell<Option<SignatureHost>> = const { RefCell::new(None) };
}

fn with_host<T>(
    f: impl FnOnce(&mut SignatureHost) -> Result<T, SignatureStoreError>,
) -> Result<T, InternalError> {
    HOST.with_borrow_mut(|host| {
        if host.is_none() {
            *host = Some(SignatureHost::new(
                ic_platform::cdk::api::canister_self().into(),
            )?);
        }
        f(host.as_mut().expect("initialized signature host"))
    })
    .map_err(|error| InternalError::from(map_error(error)))
}

fn publish(host: &SignatureHost) {
    ic_platform::cdk::api::certified_data_set(host.root_hash());
}

/// Reset volatile leaves synchronously before application lifecycle participation.
pub fn reset() -> Result<(), InternalError> {
    let host = SignatureHost::new(ic_platform::cdk::api::canister_self().into())
        .map_err(|error| InternalError::from(map_error(error)))?;
    publish(&host);
    HOST.with_borrow_mut(|state| *state = Some(host));
    Ok(())
}

/// Prepare one already-authorized profile; cleanup and publication share this message.
#[cfg(feature = "auth-root-canister-sig-create")]
pub(super) fn prepare(
    inputs: SignatureInputs<'_>,
    now_ns: u64,
    retention_ns: u64,
) -> Result<PreparedSignature, InternalError> {
    let deadline = now_ns
        .checked_add(retention_ns)
        .ok_or_else(|| InternalError::from(map_error(SignatureStoreError::DeadlineOverflow)))?;
    let clock = ic_platform::cdk::api::time();
    if clock < now_ns {
        return Err(map_error(SignatureStoreError::ClockRegressed).into());
    }
    let remaining = deadline
        .checked_sub(clock)
        .filter(|remaining| *remaining > 0)
        .ok_or_else(|| InternalError::from(map_error(SignatureStoreError::Expired)))?;
    with_host(|host| {
        host.store.prune(clock, MAX_PRUNE_BATCH)?;
        let result = host.store.prepare(inputs, clock, remaining);
        // Explicit cleanup may change the root even when preparation is refused.
        publish(host);
        result
    })
}

/// Retrieve against the actual host certificate and current composed branch roots.
#[cfg(feature = "auth-root-canister-sig-create")]
pub(super) fn retrieve(
    inputs: SignatureInputs<'_>,
    signing_canister: Principal,
    now_ns: u64,
) -> Result<IcCanisterSignatureProofV1, InternalError> {
    if signing_canister != ic_platform::cdk::api::canister_self().into() {
        return Err(
            AuthSignatureError::ProofInvalid("signing canister differs from host".into()).into(),
        );
    }
    let certificate = ic_platform::cdk::api::data_certificate()
        .ok_or(AuthSignatureError::RootDataCertificateUnavailable)?;
    with_host(|host| {
        host.store
            .retrieve(inputs, now_ns, &certificate, &host.siblings)
    })
}

/// Replace trusted application branch roots; this does not authorize a signature.
pub fn set_siblings(siblings: Vec<SignatureSibling>) -> Result<(), InternalError> {
    if siblings.len() > MAX_COMPOSITION_DEPTH {
        return Err(map_error(SignatureStoreError::CompositionTooDeep).into());
    }
    with_host(|host| {
        host.siblings = siblings;
        publish(host);
        Ok(())
    })
}

pub(super) fn map_error(error: SignatureStoreError) -> AuthSignatureError {
    match error {
        SignatureStoreError::NoCertificate => AuthSignatureError::RootDataCertificateUnavailable,
        cause => AuthSignatureError::ProofInvalid(cause.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_platform::certification::{Certificate, labeled, leaf};

    fn certificate(canister: Principal, root: [u8; 32]) -> Vec<u8> {
        let mut bytes = Vec::new();
        serde_cbor::to_writer(
            &mut bytes,
            &Certificate {
                tree: labeled(
                    b"canister",
                    labeled(
                        canister.as_slice(),
                        labeled(b"certified_data", leaf(root.to_vec())),
                    ),
                ),
                signature: Vec::new(),
                delegation: None,
            },
        )
        .unwrap();
        bytes
    }

    #[test]
    fn both_host_profiles_share_the_current_application_composition() {
        let canister = Principal::from_slice(&[1]);
        let mut host = SignatureHost::new(canister).unwrap();
        let root = SignatureInputs {
            seed: b"canic-root-role-attestation",
            domain: b"canic-root-role-attestation",
            message: &[1; 32],
        };
        let issuer = SignatureInputs {
            seed: b"canic-issuer-delegated-token",
            domain: b"canic-issuer-delegated-token",
            message: &[2; 32],
        };
        host.store.prepare(root, 1, 60).unwrap();
        host.siblings = vec![SignatureSibling::Left([3; 32])];
        let old_certificate = certificate(canister, host.root_hash());
        host.store
            .retrieve(root, 1, &old_certificate, &host.siblings)
            .unwrap();
        host.store.prepare(issuer, 2, 60).unwrap();
        assert_eq!(host.store.len(), 2);
        assert!(matches!(
            host.store
                .retrieve(root, 2, &old_certificate, &host.siblings),
            Err(SignatureStoreError::CertificateRootMismatch)
        ));
        let current = certificate(canister, host.root_hash());
        let root_proof = host
            .store
            .retrieve(root, 2, &current, &host.siblings)
            .unwrap();
        let issuer_proof = host
            .store
            .retrieve(issuer, 2, &current, &host.siblings)
            .unwrap();
        assert_ne!(root_proof.public_key_der, issuer_proof.public_key_der);
        host.siblings = vec![SignatureSibling::Left([4; 32])];
        assert!(matches!(
            host.store.retrieve(root, 3, &current, &host.siblings),
            Err(SignatureStoreError::CertificateRootMismatch)
        ));
        let current = certificate(canister, host.root_hash());
        host.store
            .retrieve(root, 3, &current, &host.siblings)
            .unwrap();
        host.store
            .retrieve(issuer, 3, &current, &host.siblings)
            .unwrap();
    }
}
