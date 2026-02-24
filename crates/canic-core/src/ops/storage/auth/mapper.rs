use crate::{
    dto::auth::{DelegationCert, DelegationProof},
    storage::stable::auth::{DelegationCertRecord, DelegationProofRecord},
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
