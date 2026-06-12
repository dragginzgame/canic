#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "issuer-proof token runtime is being landed in bounded slices"
    )
)]

use super::AuthOps;
#[cfg(feature = "auth-issuer-canister-sig-create")]
use crate::cdk;
use crate::{
    InternalError, cdk::types::Principal, dto::auth::IssuerProof, ops::auth::AuthSignatureError,
};
#[cfg(feature = "auth-issuer-canister-sig-create")]
use crate::{dto::auth::IcCanisterSignatureProofV1, ops::auth::AuthValidationError};
use sha2::{Digest, Sha256};
#[cfg(feature = "auth-issuer-canister-sig-create")]
use std::{cell::RefCell, collections::BTreeMap};

#[cfg(feature = "auth-issuer-canister-sig-create")]
pub const ISSUER_PROOF_RETRIEVAL_TTL_NS: u64 = 60_000_000_000;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum IssuerPayloadKind {
    DelegatedTokenClaims,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedIssuerCanisterSignature {
    pub retrieval_expires_at_ns: u64,
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingIssuerProof {
    operation_id: [u8; 32],
    retrieval_expires_at_ns: u64,
    prepared_by: Principal,
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PendingIssuerProofKey {
    kind: IssuerPayloadKind,
    claims_hash: [u8; 32],
    prepared_by: Vec<u8>,
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
impl PendingIssuerProofKey {
    fn new(kind: IssuerPayloadKind, claims_hash: [u8; 32], prepared_by: Principal) -> Self {
        Self {
            kind,
            claims_hash,
            prepared_by: prepared_by.as_slice().to_vec(),
        }
    }
}

pub const fn issuer_sig_seed(kind: IssuerPayloadKind) -> &'static [u8] {
    match kind {
        IssuerPayloadKind::DelegatedTokenClaims => b"canic-issuer-delegated-token",
    }
}

pub const fn issuer_sig_domain(kind: IssuerPayloadKind) -> &'static [u8] {
    match kind {
        IssuerPayloadKind::DelegatedTokenClaims => b"canic-issuer-delegated-token",
    }
}

pub fn issuer_sig_seed_hash(kind: IssuerPayloadKind) -> [u8; 32] {
    Sha256::digest(issuer_sig_seed(kind)).into()
}

pub fn issuer_canister_sig_verification_message(
    kind: IssuerPayloadKind,
    payload_hash: [u8; 32],
) -> Vec<u8> {
    let domain = issuer_sig_domain(kind);
    let domain_len =
        u8::try_from(domain.len()).expect("issuer canister signature domain exceeds 255 bytes");

    let mut msg = Vec::with_capacity(1 + domain.len() + payload_hash.len());
    msg.push(domain_len);
    msg.extend_from_slice(domain);
    msg.extend_from_slice(&payload_hash);
    msg
}

impl AuthOps {
    pub(crate) fn prepare_issuer_canister_signature(
        kind: IssuerPayloadKind,
        operation_id: [u8; 32],
        payload_hash: [u8; 32],
        prepared_by: Principal,
        now_ns: u64,
    ) -> Result<PreparedIssuerCanisterSignature, InternalError> {
        #[cfg(feature = "auth-issuer-canister-sig-create")]
        {
            validate_issuer_sig_domain_len(kind)?;
            Ok(prepare_issuer_canister_signature(
                kind,
                operation_id,
                payload_hash,
                prepared_by,
                now_ns,
            ))
        }
        #[cfg(not(feature = "auth-issuer-canister-sig-create"))]
        {
            prepare_issuer_canister_signature(kind, operation_id, payload_hash, prepared_by, now_ns)
        }
    }

    pub(crate) fn get_issuer_canister_signature_proof(
        kind: IssuerPayloadKind,
        payload_hash: [u8; 32],
        prepared_by: Principal,
        issuer_pid: Principal,
        now_ns: u64,
    ) -> Result<IssuerProof, InternalError> {
        get_issuer_canister_signature_proof(kind, payload_hash, prepared_by, issuer_pid, now_ns)
    }

    pub(crate) fn verify_issuer_canister_signature_proof(
        kind: IssuerPayloadKind,
        payload_hash: [u8; 32],
        proof: &IssuerProof,
        expected_issuer_pid: Principal,
        ic_root_public_key_raw: &[u8],
    ) -> Result<(), InternalError> {
        verify_issuer_canister_signature_proof(
            kind,
            payload_hash,
            proof,
            expected_issuer_pid,
            ic_root_public_key_raw,
        )
    }

    pub(crate) const fn issuer_canister_sig_verify_enabled() -> bool {
        cfg!(feature = "auth-issuer-canister-sig-verify")
    }

    pub(crate) const fn issuer_canister_sig_create_enabled() -> bool {
        cfg!(feature = "auth-issuer-canister-sig-create")
    }
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
fn validate_issuer_sig_domain_len(kind: IssuerPayloadKind) -> Result<(), InternalError> {
    u8::try_from(issuer_sig_domain(kind).len())
        .map(|_| ())
        .map_err(|_| {
            AuthSignatureError::ProofInvalid(
                "issuer canister signature domain exceeds 255 bytes".to_string(),
            )
            .into()
        })
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
thread_local! {
    static ISSUER_SIGNATURES: RefCell<ic_canister_sig_creation::signature_map::SignatureMap> =
        RefCell::new(ic_canister_sig_creation::signature_map::SignatureMap::default());
    static PENDING_ISSUER_PROOFS: RefCell<BTreeMap<PendingIssuerProofKey, PendingIssuerProof>> =
        const { RefCell::new(BTreeMap::new()) };
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
fn prepare_issuer_canister_signature(
    kind: IssuerPayloadKind,
    operation_id: [u8; 32],
    payload_hash: [u8; 32],
    prepared_by: Principal,
    now_ns: u64,
) -> PreparedIssuerCanisterSignature {
    use ic_canister_sig_creation::signature_map::CanisterSigInputs;

    crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics::record_issuer_proof_prepare_started();

    let inputs = CanisterSigInputs {
        domain: issuer_sig_domain(kind),
        seed: issuer_sig_seed(kind),
        message: &payload_hash,
    };
    ISSUER_SIGNATURES.with(|signatures| {
        let mut signatures = signatures.borrow_mut();
        signatures.add_signature(&inputs);
        refresh_issuer_signature_certified_data(&signatures.root_hash());
    });

    let retrieval_expires_at_ns = now_ns.saturating_add(ISSUER_PROOF_RETRIEVAL_TTL_NS);
    PENDING_ISSUER_PROOFS.with(|pending| {
        pending.borrow_mut().insert(
            PendingIssuerProofKey::new(kind, payload_hash, prepared_by),
            PendingIssuerProof {
                operation_id,
                retrieval_expires_at_ns,
                prepared_by,
            },
        );
    });

    crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics::record_issuer_proof_prepare_completed();
    PreparedIssuerCanisterSignature {
        retrieval_expires_at_ns,
    }
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
fn refresh_issuer_signature_certified_data(signature_root_hash: &[u8; 32]) {
    use ic_canister_sig_creation::signature_map::LABEL_SIG;
    use ic_certification::labeled_hash;

    cdk::api::certified_data_set(labeled_hash(LABEL_SIG, signature_root_hash));
}

#[cfg(not(feature = "auth-issuer-canister-sig-create"))]
fn prepare_issuer_canister_signature(
    _kind: IssuerPayloadKind,
    _operation_id: [u8; 32],
    _payload_hash: [u8; 32],
    _prepared_by: Principal,
    _now_ns: u64,
) -> Result<PreparedIssuerCanisterSignature, InternalError> {
    crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics::record_issuer_proof_prepare_failed();
    Err(AuthSignatureError::ProofUnavailable.into())
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
fn get_issuer_canister_signature_proof(
    kind: IssuerPayloadKind,
    payload_hash: [u8; 32],
    prepared_by: Principal,
    issuer_pid: Principal,
    now_ns: u64,
) -> Result<IssuerProof, InternalError> {
    use ic_canister_sig_creation::{CanisterSigPublicKey, signature_map::CanisterSigInputs};

    let key = PendingIssuerProofKey::new(kind, payload_hash, prepared_by);
    let pending = PENDING_ISSUER_PROOFS.with(|pending| pending.borrow().get(&key).cloned());
    let pending = pending.ok_or_else(|| {
        AuthValidationError::Auth(
            "issuer token proof was not prepared or has been pruned".to_string(),
        )
    })?;
    if pending.prepared_by != prepared_by {
        return Err(AuthValidationError::Auth(
            "issuer token proof retrieval caller mismatch".to_string(),
        )
        .into());
    }
    if now_ns >= pending.retrieval_expires_at_ns {
        return Err(AuthValidationError::Auth(format!(
            "issuer token proof retrieval window expired for operation {:?}",
            pending.operation_id
        ))
        .into());
    }

    let inputs = CanisterSigInputs {
        domain: issuer_sig_domain(kind),
        seed: issuer_sig_seed(kind),
        message: &payload_hash,
    };
    let signature_cbor = ISSUER_SIGNATURES.with(|signatures| {
        signatures
            .borrow()
            .get_signature_as_cbor(&inputs, None)
            .map_err(|err| AuthSignatureError::ProofInvalid(err.to_string()))
    })?;
    let public_key_der =
        CanisterSigPublicKey::new(issuer_pid, issuer_sig_seed(kind).to_vec()).to_der();

    Ok(IssuerProof::IcCanisterSignatureV1(
        IcCanisterSignatureProofV1 {
            signature_cbor,
            public_key_der,
        },
    ))
}

#[cfg(not(feature = "auth-issuer-canister-sig-create"))]
fn get_issuer_canister_signature_proof(
    _kind: IssuerPayloadKind,
    _payload_hash: [u8; 32],
    _prepared_by: Principal,
    _issuer_pid: Principal,
    _now_ns: u64,
) -> Result<IssuerProof, InternalError> {
    Err(AuthSignatureError::ProofUnavailable.into())
}

#[cfg(feature = "auth-issuer-canister-sig-verify")]
fn verify_issuer_canister_signature_proof(
    kind: IssuerPayloadKind,
    payload_hash: [u8; 32],
    proof: &IssuerProof,
    expected_issuer_pid: Principal,
    ic_root_public_key_raw: &[u8],
) -> Result<(), InternalError> {
    let IssuerProof::IcCanisterSignatureV1(proof) = proof;
    let (canister_id, seed) = parse_canister_sig_public_key_der(&proof.public_key_der)
        .map_err(AuthSignatureError::ProofInvalid)?;
    if canister_id != expected_issuer_pid {
        return Err(AuthSignatureError::ProofInvalid(
            "issuer canister signature public key canister id mismatch".to_string(),
        )
        .into());
    }
    if seed != issuer_sig_seed(kind) {
        return Err(AuthSignatureError::ProofInvalid(
            "issuer canister signature seed mismatch".to_string(),
        )
        .into());
    }

    let message = issuer_canister_sig_verification_message(kind, payload_hash);
    ic_signature_verification::verify_canister_sig(
        &message,
        &proof.signature_cbor,
        &proof.public_key_der,
        ic_root_public_key_raw,
    )
    .map_err(AuthSignatureError::ProofInvalid)?;

    Ok(())
}

#[cfg(not(feature = "auth-issuer-canister-sig-verify"))]
fn verify_issuer_canister_signature_proof(
    _kind: IssuerPayloadKind,
    _payload_hash: [u8; 32],
    _proof: &IssuerProof,
    _expected_issuer_pid: Principal,
    _ic_root_public_key_raw: &[u8],
) -> Result<(), InternalError> {
    Err(AuthSignatureError::ProofUnavailable.into())
}

#[cfg(feature = "auth-issuer-canister-sig-verify")]
const CANISTER_SIG_PK_DER_PREFIX_LENGTH: usize = 19;
#[cfg(feature = "auth-issuer-canister-sig-verify")]
const CANISTER_SIG_PK_DER_OID: &[u8; 14] =
    b"\x30\x0C\x06\x0A\x2B\x06\x01\x04\x01\x83\xB8\x43\x01\x02";

#[cfg(feature = "auth-issuer-canister-sig-verify")]
fn parse_canister_sig_public_key_der(
    public_key_der: &[u8],
) -> Result<(Principal, Vec<u8>), String> {
    if public_key_der.len() < CANISTER_SIG_PK_DER_PREFIX_LENGTH + 1 {
        return Err("canister signature public key DER too short".to_string());
    }
    let oid_end = 2 + CANISTER_SIG_PK_DER_OID.len();
    if public_key_der.len() < oid_end || &public_key_der[2..oid_end] != CANISTER_SIG_PK_DER_OID {
        return Err("invalid canister signature public key OID".to_string());
    }

    let raw = &public_key_der[CANISTER_SIG_PK_DER_PREFIX_LENGTH..];
    let canister_id_len = usize::from(raw[0]);
    if raw.len() < 1 + canister_id_len {
        return Err("canister signature public key raw bytes too short".to_string());
    }
    let canister_id_end = 1 + canister_id_len;
    let canister_id = Principal::try_from_slice(&raw[1..canister_id_end])
        .map_err(|err| format!("invalid canister id in canister signature public key: {err}"))?;
    let seed = raw[canister_id_end..].to_vec();
    Ok((canister_id, seed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issuer_canister_sig_verification_message_prefixes_domain_len() {
        let payload_hash = [7; 32];
        let msg = issuer_canister_sig_verification_message(
            IssuerPayloadKind::DelegatedTokenClaims,
            payload_hash,
        );
        let domain = issuer_sig_domain(IssuerPayloadKind::DelegatedTokenClaims);

        assert_eq!(usize::from(msg[0]), domain.len());
        assert_eq!(&msg[1..=domain.len()], domain);
        assert_eq!(&msg[1 + domain.len()..], payload_hash);
    }

    #[test]
    fn issuer_seed_hash_matches_binding_seed_hash_input() {
        let seed_hash = issuer_sig_seed_hash(IssuerPayloadKind::DelegatedTokenClaims);
        let expected: [u8; 32] = Sha256::digest(b"canic-issuer-delegated-token").into();

        assert_eq!(seed_hash, expected);
    }

    #[test]
    fn issuer_canister_sig_feature_flags_match_compile_features() {
        assert_eq!(
            AuthOps::issuer_canister_sig_create_enabled(),
            cfg!(feature = "auth-issuer-canister-sig-create")
        );
        assert_eq!(
            AuthOps::issuer_canister_sig_verify_enabled(),
            cfg!(feature = "auth-issuer-canister-sig-verify")
        );
    }

    #[cfg(not(feature = "auth-issuer-canister-sig-create"))]
    #[test]
    fn issuer_canister_sig_create_requires_create_feature() {
        let caller = Principal::from_slice(&[1; 29]);

        assert!(
            AuthOps::prepare_issuer_canister_signature(
                IssuerPayloadKind::DelegatedTokenClaims,
                [1; 32],
                [2; 32],
                caller,
                1,
            )
            .is_err()
        );
        assert!(
            AuthOps::get_issuer_canister_signature_proof(
                IssuerPayloadKind::DelegatedTokenClaims,
                [2; 32],
                caller,
                Principal::from_slice(&[2; 29]),
                1,
            )
            .is_err()
        );
    }

    #[cfg(not(feature = "auth-issuer-canister-sig-verify"))]
    #[test]
    fn issuer_canister_sig_verify_requires_verify_feature() {
        let proof =
            IssuerProof::IcCanisterSignatureV1(crate::dto::auth::IcCanisterSignatureProofV1 {
                signature_cbor: vec![1, 2, 3],
                public_key_der: vec![4, 5, 6],
            });

        assert!(
            AuthOps::verify_issuer_canister_signature_proof(
                IssuerPayloadKind::DelegatedTokenClaims,
                [2; 32],
                &proof,
                Principal::from_slice(&[2; 29]),
                &[9; 96],
            )
            .is_err()
        );
    }
}
