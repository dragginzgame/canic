use crate::{
    InternalError,
    dto::auth::{AttestationKey, AttestationKeyStatus, DelegationCert, DelegationProof},
    ops::auth::DelegationValidationError,
    storage::stable::auth::{
        AttestationKeyStatusRecord, AttestationPublicKeyRecord, DelegationCertRecord,
        DelegationProofEntryRecord, DelegationProofKeyRecord, DelegationProofRecord,
    },
};
use candid::encode_one;
use sha2::{Digest, Sha256};

const CERT_SIGNING_DOMAIN: &[u8] = b"CANIC_DELEGATION_CERT_V1";

///
/// DelegationProofRecordMapper
///

pub struct DelegationProofRecordMapper;

impl DelegationProofRecordMapper {
    #[must_use]
    pub fn dto_to_record(proof: DelegationProof) -> DelegationProofRecord {
        // TODO: keep record schema and DTO fields in sync; mapping lives in ops.
        DelegationProofRecord {
            cert: DelegationCertRecord {
                root_pid: proof.cert.root_pid,
                shard_pid: proof.cert.shard_pid,
                issued_at: proof.cert.issued_at,
                expires_at: proof.cert.expires_at,
                scopes: proof.cert.scopes,
                aud: proof.cert.aud,
            },
            cert_sig: proof.cert_sig,
        }
    }

    #[must_use]
    pub fn record_to_view(record: DelegationProofRecord) -> DelegationProof {
        // TODO: keep record schema and DTO fields in sync; mapping lives in ops.
        DelegationProof {
            cert: DelegationCert {
                root_pid: record.cert.root_pid,
                shard_pid: record.cert.shard_pid,
                issued_at: record.cert.issued_at,
                expires_at: record.cert.expires_at,
                scopes: record.cert.scopes,
                aud: record.cert.aud,
            },
            cert_sig: record.cert_sig,
        }
    }

    #[must_use]
    pub const fn record_to_entry(
        proof: DelegationProofRecord,
        key: DelegationProofKeyRecord,
        installed_at: u64,
    ) -> DelegationProofEntryRecord {
        DelegationProofEntryRecord {
            key,
            proof,
            installed_at,
            last_verified_at: None,
        }
    }

    pub fn dto_to_entry(
        proof: DelegationProof,
        installed_at: u64,
    ) -> Result<DelegationProofEntryRecord, InternalError> {
        let key = Self::proof_key_from_dto(&proof)?;
        Ok(Self::record_to_entry(
            Self::dto_to_record(proof),
            key,
            installed_at,
        ))
    }

    pub fn proof_key_from_dto(
        proof: &DelegationProof,
    ) -> Result<DelegationProofKeyRecord, InternalError> {
        Ok(DelegationProofKeyRecord {
            shard_pid: proof.cert.shard_pid,
            cert_hash: cert_hash(&proof.cert)?,
        })
    }
}

///
/// AttestationPublicKeyRecordMapper
///

pub struct AttestationPublicKeyRecordMapper;

impl AttestationPublicKeyRecordMapper {
    #[must_use]
    pub fn dto_to_record(key: AttestationKey) -> AttestationPublicKeyRecord {
        AttestationPublicKeyRecord {
            key_id: key.key_id,
            public_key_sec1: key.public_key,
            status: match key.status {
                AttestationKeyStatus::Current => AttestationKeyStatusRecord::Current,
                AttestationKeyStatus::Previous => AttestationKeyStatusRecord::Previous,
            },
            valid_from: key.valid_from,
            valid_until: key.valid_until,
        }
    }

    #[must_use]
    pub fn record_to_dto(record: AttestationPublicKeyRecord) -> AttestationKey {
        AttestationKey {
            key_id: record.key_id,
            public_key: record.public_key_sec1,
            status: match record.status {
                AttestationKeyStatusRecord::Current => AttestationKeyStatus::Current,
                AttestationKeyStatusRecord::Previous => AttestationKeyStatus::Previous,
            },
            valid_from: record.valid_from,
            valid_until: record.valid_until,
        }
    }
}

fn cert_hash(cert: &DelegationCert) -> Result<[u8; 32], InternalError> {
    let payload = encode_one(cert).map_err(|source| DelegationValidationError::EncodeFailed {
        context: "delegation cert",
        source,
    })?;

    Ok(hash_domain_separated(CERT_SIGNING_DOMAIN, &payload))
}

fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    hasher.finalize().into()
}
