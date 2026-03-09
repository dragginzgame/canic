use crate::{
    dto::auth::{AttestationKey, AttestationKeyStatus, DelegationCert, DelegationProof},
    storage::stable::auth::{
        AttestationKeyStatusRecord, AttestationPublicKeyRecord, DelegationCertRecord,
        DelegationProofRecord,
    },
};

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
