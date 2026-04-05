use crate::{
    dto::auth::{AttestationKey, AttestationKeyStatus, DelegationCert, DelegationProof},
    ops::auth::delegation_cert_hash,
    ops::storage::auth::{StoredDelegationCert, StoredDelegationProof},
    storage::stable::auth::{
        AttestationKeyStatusRecord, AttestationPublicKeyRecord, DelegationCertRecord,
        DelegationProofEntryRecord, DelegationProofKeyRecord, DelegationProofRecord,
    },
};

///
/// DelegationProofRecordMapper
///

pub struct DelegationProofRecordMapper;

impl DelegationProofRecordMapper {
    #[must_use]
    pub(super) fn dto_ref_to_record(proof: &DelegationProof) -> DelegationProofRecord {
        DelegationProofRecord {
            cert: DelegationCertRecord {
                root_pid: proof.cert.root_pid,
                shard_pid: proof.cert.shard_pid,
                issued_at: proof.cert.issued_at,
                expires_at: proof.cert.expires_at,
                scopes: proof.cert.scopes.clone(),
                aud: proof.cert.aud.clone(),
            },
            cert_sig: proof.cert_sig.clone(),
        }
    }

    #[must_use]
    pub(super) fn stored_proof_to_dto(proof: StoredDelegationProof) -> DelegationProof {
        DelegationProof {
            cert: DelegationCert {
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
    pub(super) fn record_to_stored_proof(record: DelegationProofRecord) -> StoredDelegationProof {
        StoredDelegationProof {
            cert: StoredDelegationCert {
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
    pub(super) const fn record_to_entry(
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

    pub(super) fn proof_key_from_dto(proof: &DelegationProof) -> DelegationProofKeyRecord {
        DelegationProofKeyRecord {
            shard_pid: proof.cert.shard_pid,
            cert_hash: cert_hash(&proof.cert),
        }
    }

    pub(super) fn dto_ref_to_entry(
        proof: &DelegationProof,
        installed_at: u64,
    ) -> DelegationProofEntryRecord {
        let key = Self::proof_key_from_dto(proof);
        Self::record_to_entry(Self::dto_ref_to_record(proof), key, installed_at)
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

fn cert_hash(cert: &DelegationCert) -> [u8; 32] {
    delegation_cert_hash(cert)
}
