use super::AuthOps;
#[cfg(any(
    feature = "auth-root-canister-sig-create",
    feature = "auth-root-canister-sig-verify"
))]
use crate::cdk;
use crate::{
    InternalError, cdk::types::Principal, dto::auth::RootProof, ops::auth::AuthSignatureError,
};
#[cfg(feature = "auth-root-canister-sig-create")]
use crate::{dto::auth::IcCanisterSignatureProofV1, ops::auth::AuthValidationError};
#[cfg(feature = "auth-root-canister-sig-create")]
use std::{cell::RefCell, collections::BTreeMap};

#[cfg(feature = "auth-root-canister-sig-create")]
pub const ROOT_PROOF_RETRIEVAL_TTL_NS: u64 = 60_000_000_000;
pub const IC_ROOT_PUBLIC_KEY_RAW_LENGTH: usize = 96;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RootPayloadKind {
    DelegationCert,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedRootCanisterSignature {
    pub retrieval_expires_at_ns: u64,
}

#[cfg(feature = "auth-root-canister-sig-create")]
#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingRootProof {
    operation_id: [u8; 32],
    retrieval_expires_at_ns: u64,
    prepared_by: Principal,
}

#[cfg(feature = "auth-root-canister-sig-create")]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PendingRootProofKey {
    kind: RootPayloadKind,
    cert_hash: [u8; 32],
    prepared_by: Vec<u8>,
}

#[cfg(feature = "auth-root-canister-sig-create")]
impl PendingRootProofKey {
    fn new(kind: RootPayloadKind, cert_hash: [u8; 32], prepared_by: Principal) -> Self {
        Self {
            kind,
            cert_hash,
            prepared_by: prepared_by.as_slice().to_vec(),
        }
    }
}

#[cfg(any(
    feature = "auth-root-canister-sig-create",
    feature = "auth-root-canister-sig-verify",
    test
))]
#[allow(dead_code)]
pub const fn root_sig_seed(kind: RootPayloadKind) -> &'static [u8] {
    match kind {
        RootPayloadKind::DelegationCert => b"canic-root-delegation-cert",
    }
}

#[cfg(any(
    feature = "auth-root-canister-sig-create",
    feature = "auth-root-canister-sig-verify",
    test
))]
pub const fn root_sig_domain(kind: RootPayloadKind) -> &'static [u8] {
    match kind {
        RootPayloadKind::DelegationCert => b"canic-root-delegation-cert",
    }
}

#[cfg(any(feature = "auth-root-canister-sig-verify", test))]
pub fn root_canister_sig_verification_message(
    kind: RootPayloadKind,
    payload_hash: [u8; 32],
) -> Vec<u8> {
    let domain = root_sig_domain(kind);
    let domain_len =
        u8::try_from(domain.len()).expect("root canister signature domain exceeds 255 bytes");

    let mut msg = Vec::with_capacity(1 + domain.len() + payload_hash.len());
    msg.push(domain_len);
    msg.extend_from_slice(domain);
    msg.extend_from_slice(&payload_hash);
    msg
}

impl AuthOps {
    pub(crate) fn prepare_root_canister_signature(
        kind: RootPayloadKind,
        operation_id: [u8; 32],
        payload_hash: [u8; 32],
        prepared_by: Principal,
        now_ns: u64,
    ) -> Result<PreparedRootCanisterSignature, InternalError> {
        #[cfg(feature = "auth-root-canister-sig-create")]
        {
            validate_root_sig_domain_len(kind)?;
            Ok(prepare_root_canister_signature(
                kind,
                operation_id,
                payload_hash,
                prepared_by,
                now_ns,
            ))
        }
        #[cfg(not(feature = "auth-root-canister-sig-create"))]
        {
            prepare_root_canister_signature(kind, operation_id, payload_hash, prepared_by, now_ns)
        }
    }

    pub(crate) fn get_root_canister_signature_proof(
        kind: RootPayloadKind,
        payload_hash: [u8; 32],
        prepared_by: Principal,
        root_pid: Principal,
        now_ns: u64,
    ) -> Result<RootProof, InternalError> {
        get_root_canister_signature_proof(kind, payload_hash, prepared_by, root_pid, now_ns)
    }

    pub(crate) fn verify_root_canister_signature_proof(
        kind: RootPayloadKind,
        payload_hash: [u8; 32],
        proof: &RootProof,
        expected_root_pid: Principal,
        ic_root_public_key_raw: &[u8],
    ) -> Result<(), InternalError> {
        verify_root_canister_signature_proof(
            kind,
            payload_hash,
            proof,
            expected_root_pid,
            ic_root_public_key_raw,
        )
    }

    pub(crate) fn ic_root_public_key_raw() -> Result<Vec<u8>, InternalError> {
        ic_root_public_key_raw()
    }

    pub(crate) const fn root_canister_sig_verify_enabled() -> bool {
        cfg!(feature = "auth-root-canister-sig-verify")
    }

    pub(crate) const fn root_canister_sig_create_enabled() -> bool {
        cfg!(feature = "auth-root-canister-sig-create")
    }
}

#[cfg(feature = "auth-root-canister-sig-create")]
fn validate_root_sig_domain_len(kind: RootPayloadKind) -> Result<(), InternalError> {
    u8::try_from(root_sig_domain(kind).len())
        .map(|_| ())
        .map_err(|_| {
            AuthSignatureError::CertSignatureInvalid(
                "root canister signature domain exceeds 255 bytes".to_string(),
            )
            .into()
        })
}

#[cfg(feature = "auth-root-canister-sig-create")]
thread_local! {
    static ROOT_SIGNATURES: RefCell<ic_canister_sig_creation::signature_map::SignatureMap> =
        RefCell::new(ic_canister_sig_creation::signature_map::SignatureMap::default());
    static PENDING_ROOT_PROOFS: RefCell<BTreeMap<PendingRootProofKey, PendingRootProof>> =
        const { RefCell::new(BTreeMap::new()) };
}

#[cfg(feature = "auth-root-canister-sig-create")]
fn prepare_root_canister_signature(
    kind: RootPayloadKind,
    operation_id: [u8; 32],
    payload_hash: [u8; 32],
    prepared_by: Principal,
    now_ns: u64,
) -> PreparedRootCanisterSignature {
    use ic_canister_sig_creation::signature_map::CanisterSigInputs;

    crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics::record_root_proof_prepare_started();

    let inputs = CanisterSigInputs {
        domain: root_sig_domain(kind),
        seed: root_sig_seed(kind),
        message: &payload_hash,
    };
    ROOT_SIGNATURES.with(|signatures| {
        let mut signatures = signatures.borrow_mut();
        signatures.add_signature(&inputs);
        refresh_root_signature_certified_data(&signatures.root_hash());
    });

    let retrieval_expires_at_ns = now_ns.saturating_add(ROOT_PROOF_RETRIEVAL_TTL_NS);
    PENDING_ROOT_PROOFS.with(|pending| {
        pending.borrow_mut().insert(
            PendingRootProofKey::new(kind, payload_hash, prepared_by),
            PendingRootProof {
                operation_id,
                retrieval_expires_at_ns,
                prepared_by,
            },
        );
    });

    crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics::record_root_proof_prepare_completed();
    PreparedRootCanisterSignature {
        retrieval_expires_at_ns,
    }
}

#[cfg(feature = "auth-root-canister-sig-create")]
fn refresh_root_signature_certified_data(signature_root_hash: &[u8; 32]) {
    use ic_canister_sig_creation::signature_map::LABEL_SIG;
    use ic_certification::labeled_hash;

    cdk::api::certified_data_set(labeled_hash(LABEL_SIG, signature_root_hash));
}

#[cfg(not(feature = "auth-root-canister-sig-create"))]
fn prepare_root_canister_signature(
    _kind: RootPayloadKind,
    _operation_id: [u8; 32],
    _payload_hash: [u8; 32],
    _prepared_by: Principal,
    _now_ns: u64,
) -> Result<PreparedRootCanisterSignature, InternalError> {
    crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics::record_root_proof_prepare_failed();
    Err(AuthSignatureError::CertSignatureUnavailable.into())
}

#[cfg(feature = "auth-root-canister-sig-create")]
fn get_root_canister_signature_proof(
    kind: RootPayloadKind,
    payload_hash: [u8; 32],
    prepared_by: Principal,
    root_pid: Principal,
    now_ns: u64,
) -> Result<RootProof, InternalError> {
    use ic_canister_sig_creation::{CanisterSigPublicKey, signature_map::CanisterSigInputs};

    let key = PendingRootProofKey::new(kind, payload_hash, prepared_by);
    let pending = PENDING_ROOT_PROOFS.with(|pending| pending.borrow().get(&key).cloned());
    let pending = pending.ok_or_else(|| {
        AuthValidationError::Auth(
            "delegation proof was not prepared or has been pruned".to_string(),
        )
    })?;
    if pending.prepared_by != prepared_by {
        return Err(AuthValidationError::Auth(
            "delegation proof retrieval caller mismatch".to_string(),
        )
        .into());
    }
    if now_ns >= pending.retrieval_expires_at_ns {
        return Err(AuthValidationError::Auth(format!(
            "delegation proof retrieval window expired for operation {:?}",
            pending.operation_id
        ))
        .into());
    }
    let inputs = CanisterSigInputs {
        domain: root_sig_domain(kind),
        seed: root_sig_seed(kind),
        message: &payload_hash,
    };
    let signature_cbor = ROOT_SIGNATURES.with(|signatures| {
        signatures
            .borrow()
            .get_signature_as_cbor(&inputs, None)
            .map_err(|err| AuthSignatureError::CertSignatureInvalid(err.to_string()))
    })?;
    let public_key_der = CanisterSigPublicKey::new(root_pid, root_sig_seed(kind).to_vec()).to_der();

    Ok(RootProof::IcCanisterSignatureV1(
        IcCanisterSignatureProofV1 {
            signature_cbor,
            public_key_der,
        },
    ))
}

#[cfg(not(feature = "auth-root-canister-sig-create"))]
fn get_root_canister_signature_proof(
    _kind: RootPayloadKind,
    _payload_hash: [u8; 32],
    _prepared_by: Principal,
    _root_pid: Principal,
    _now_ns: u64,
) -> Result<RootProof, InternalError> {
    Err(AuthSignatureError::CertSignatureUnavailable.into())
}

#[cfg(feature = "auth-root-canister-sig-verify")]
fn verify_root_canister_signature_proof(
    kind: RootPayloadKind,
    payload_hash: [u8; 32],
    proof: &RootProof,
    expected_root_pid: Principal,
    ic_root_public_key_raw: &[u8],
) -> Result<(), InternalError> {
    let RootProof::IcCanisterSignatureV1(proof) = proof;
    let (canister_id, seed) = parse_canister_sig_public_key_der(&proof.public_key_der)
        .map_err(AuthSignatureError::CertSignatureInvalid)?;
    if canister_id != expected_root_pid {
        return Err(AuthSignatureError::CertSignatureInvalid(
            "root canister signature public key canister id mismatch".to_string(),
        )
        .into());
    }
    if seed != root_sig_seed(kind) {
        return Err(AuthSignatureError::CertSignatureInvalid(
            "root canister signature seed mismatch".to_string(),
        )
        .into());
    }

    let message = root_canister_sig_verification_message(kind, payload_hash);
    ic_signature_verification::verify_canister_sig(
        &message,
        &proof.signature_cbor,
        &proof.public_key_der,
        ic_root_public_key_raw,
    )
    .map_err(AuthSignatureError::CertSignatureInvalid)?;

    Ok(())
}

#[cfg(not(feature = "auth-root-canister-sig-verify"))]
fn verify_root_canister_signature_proof(
    _kind: RootPayloadKind,
    _payload_hash: [u8; 32],
    _proof: &RootProof,
    _expected_root_pid: Principal,
    _ic_root_public_key_raw: &[u8],
) -> Result<(), InternalError> {
    Err(AuthSignatureError::CertSignatureUnavailable.into())
}

#[cfg(feature = "auth-root-canister-sig-verify")]
fn ic_root_public_key_raw() -> Result<Vec<u8>, InternalError> {
    let root_key = cdk::api::root_key();
    extract_ic_root_public_key_raw(&root_key)
        .map_err(|err| AuthSignatureError::CertSignatureInvalid(err).into())
}

#[cfg(not(feature = "auth-root-canister-sig-verify"))]
fn ic_root_public_key_raw() -> Result<Vec<u8>, InternalError> {
    Err(AuthSignatureError::CertSignatureUnavailable.into())
}

#[cfg(any(feature = "auth-root-canister-sig-verify", test))]
const IC_ROOT_PK_DER_PREFIX: &[u8; 37] = b"\x30\x81\x82\x30\x1d\x06\x0d\x2b\x06\x01\x04\x01\x82\xdc\x7c\x05\x03\x01\x02\x01\x06\x0c\x2b\x06\x01\x04\x01\x82\xdc\x7c\x05\x03\x02\x01\x03\x61\x00";
#[cfg(feature = "auth-root-canister-sig-verify")]
const CANISTER_SIG_PK_DER_PREFIX_LENGTH: usize = 19;
#[cfg(feature = "auth-root-canister-sig-verify")]
const CANISTER_SIG_PK_DER_OID: &[u8; 14] =
    b"\x30\x0C\x06\x0A\x2B\x06\x01\x04\x01\x83\xB8\x43\x01\x02";

#[cfg(any(feature = "auth-root-canister-sig-verify", test))]
fn extract_ic_root_public_key_raw(root_key: &[u8]) -> Result<Vec<u8>, String> {
    if root_key.len() == IC_ROOT_PUBLIC_KEY_RAW_LENGTH {
        return Ok(root_key.to_vec());
    }

    let expected_length = IC_ROOT_PK_DER_PREFIX.len() + IC_ROOT_PUBLIC_KEY_RAW_LENGTH;
    if root_key.len() != expected_length {
        return Err("invalid IC root public key length".to_string());
    }
    if &root_key[..IC_ROOT_PK_DER_PREFIX.len()] != IC_ROOT_PK_DER_PREFIX {
        return Err("invalid IC root public key DER prefix".to_string());
    }
    Ok(root_key[IC_ROOT_PK_DER_PREFIX.len()..].to_vec())
}

#[cfg(feature = "auth-root-canister-sig-verify")]
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
    fn verification_message_prefixes_domain_length_and_domain() {
        let msg = root_canister_sig_verification_message(RootPayloadKind::DelegationCert, [7; 32]);
        let domain = root_sig_domain(RootPayloadKind::DelegationCert);
        let domain_len = u8::try_from(domain.len()).unwrap();
        let domain_start = 1;
        let domain_end = domain_start + domain.len();

        assert_eq!(msg[0], domain_len);
        assert_eq!(&msg[domain_start..domain_end], domain);
        assert_eq!(&msg[domain_end..], &[7; 32]);
    }

    #[test]
    fn extracts_raw_ic_root_key_from_der_or_raw() {
        let mut der = IC_ROOT_PK_DER_PREFIX.to_vec();
        der.extend_from_slice(&[9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]);

        assert_eq!(
            extract_ic_root_public_key_raw(&der).unwrap(),
            vec![9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]
        );
        assert_eq!(
            extract_ic_root_public_key_raw(&[8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]).unwrap(),
            vec![8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]
        );
    }
}
