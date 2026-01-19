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
                v: proof.cert.v,
                signer_pid: proof.cert.signer_pid,
                audiences: proof.cert.audiences,
                scopes: proof.cert.scopes,
                issued_at: proof.cert.issued_at,
                expires_at: proof.cert.expires_at,
            },
            cert_sig: proof.cert_sig,
        }
    }

    #[must_use]
    pub fn record_to_view(record: DelegationProofRecord) -> DelegationProof {
        // TODO: keep record schema and DTO fields in sync; mapping lives in ops.
        DelegationProof {
            cert: DelegationCert {
                v: record.cert.v,
                signer_pid: record.cert.signer_pid,
                audiences: record.cert.audiences,
                scopes: record.cert.scopes,
                issued_at: record.cert.issued_at,
                expires_at: record.cert.expires_at,
            },
            cert_sig: record.cert_sig,
        }
    }
}
