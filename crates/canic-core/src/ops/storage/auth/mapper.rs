use crate::{
    domain::policy::auth::{
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerPolicy,
    },
    dto::auth::{
        ActiveDelegationProof, DelegatedRoleGrant, DelegationAudience, DelegationCert,
        DelegationProof, IcCanisterSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding,
        RootProof,
    },
    storage::stable::auth::{
        ActiveDelegationProofRecord, DelegatedRoleGrantRecord, DelegationAudienceRecord,
        DelegationCertRecord, DelegationProofRecord, IcCanisterSignatureProofRecord,
        IssuerProofAlgorithmRecord, IssuerProofBindingRecord, RootIssuerRecord, RootProofRecord,
    },
};

///
/// ActiveDelegationProofRecordMapper
///

pub struct ActiveDelegationProofRecordMapper;

impl ActiveDelegationProofRecordMapper {
    #[must_use]
    pub fn dto_to_record(proof: ActiveDelegationProof) -> ActiveDelegationProofRecord {
        ActiveDelegationProofRecord {
            proof: delegation_proof_to_record(proof.proof),
            cert_hash: proof.cert_hash,
            not_before_ns: proof.not_before_ns,
            expires_at_ns: proof.expires_at_ns,
            refresh_after_ns: proof.refresh_after_ns,
            installed_at_ns: proof.installed_at_ns,
            installed_by: proof.installed_by,
        }
    }

    #[must_use]
    pub fn record_to_dto(record: ActiveDelegationProofRecord) -> ActiveDelegationProof {
        ActiveDelegationProof {
            proof: delegation_proof_record_to_dto(record.proof),
            cert_hash: record.cert_hash,
            not_before_ns: record.not_before_ns,
            expires_at_ns: record.expires_at_ns,
            refresh_after_ns: record.refresh_after_ns,
            installed_at_ns: record.installed_at_ns,
            installed_by: record.installed_by,
        }
    }
}

///
/// RootIssuerPolicyRecordMapper
///

pub struct RootIssuerPolicyRecordMapper;

impl RootIssuerPolicyRecordMapper {
    #[must_use]
    pub fn record_to_policy(record: RootIssuerRecord) -> RootIssuerPolicy {
        RootIssuerPolicy {
            issuer_pid: record.issuer_pid,
            enabled: record.enabled,
            allowed_audiences: record
                .allowed_audiences
                .into_iter()
                .map(audience_record_to_policy)
                .collect(),
            allowed_grants: record
                .allowed_grants
                .into_iter()
                .map(grant_record_to_policy)
                .collect(),
            max_cert_ttl_ns: record.max_cert_ttl_ns,
            refresh_after_ratio_bps: record.refresh_after_ratio_bps,
        }
    }

    #[cfg(any(test, canic_test_delegation_material))]
    #[must_use]
    pub fn policy_to_record(policy: RootIssuerPolicy) -> RootIssuerRecord {
        RootIssuerRecord {
            issuer_pid: policy.issuer_pid,
            enabled: policy.enabled,
            allowed_audiences: policy
                .allowed_audiences
                .into_iter()
                .map(audience_policy_to_record)
                .collect(),
            allowed_grants: policy
                .allowed_grants
                .into_iter()
                .map(grant_policy_to_record)
                .collect(),
            max_cert_ttl_ns: policy.max_cert_ttl_ns,
            refresh_after_ratio_bps: policy.refresh_after_ratio_bps,
        }
    }
}

fn delegation_proof_to_record(proof: DelegationProof) -> DelegationProofRecord {
    DelegationProofRecord {
        cert: delegation_cert_to_record(proof.cert),
        root_proof: root_proof_to_record(proof.root_proof),
    }
}

fn delegation_proof_record_to_dto(record: DelegationProofRecord) -> DelegationProof {
    DelegationProof {
        cert: delegation_cert_record_to_dto(record.cert),
        root_proof: root_proof_record_to_dto(record.root_proof),
    }
}

fn delegation_cert_to_record(cert: DelegationCert) -> DelegationCertRecord {
    DelegationCertRecord {
        root_pid: cert.root_pid,
        issuer_pid: cert.issuer_pid,
        issuer_proof_alg: issuer_proof_alg_to_record(cert.issuer_proof_alg),
        issuer_proof_binding_hash: cert.issuer_proof_binding_hash,
        issuer_proof_binding: issuer_proof_binding_to_record(cert.issuer_proof_binding),
        issued_at_ns: cert.issued_at_ns,
        not_before_ns: cert.not_before_ns,
        expires_at_ns: cert.expires_at_ns,
        max_token_ttl_ns: cert.max_token_ttl_ns,
        aud: audience_to_record(cert.aud),
        grants: cert.grants.into_iter().map(grant_to_record).collect(),
    }
}

fn delegation_cert_record_to_dto(record: DelegationCertRecord) -> DelegationCert {
    DelegationCert {
        root_pid: record.root_pid,
        issuer_pid: record.issuer_pid,
        issuer_proof_alg: issuer_proof_alg_record_to_dto(record.issuer_proof_alg),
        issuer_proof_binding_hash: record.issuer_proof_binding_hash,
        issuer_proof_binding: issuer_proof_binding_record_to_dto(record.issuer_proof_binding),
        issued_at_ns: record.issued_at_ns,
        not_before_ns: record.not_before_ns,
        expires_at_ns: record.expires_at_ns,
        max_token_ttl_ns: record.max_token_ttl_ns,
        aud: audience_record_to_dto(record.aud),
        grants: record.grants.into_iter().map(grant_record_to_dto).collect(),
    }
}

fn root_proof_to_record(proof: RootProof) -> RootProofRecord {
    match proof {
        RootProof::IcCanisterSignatureV1(proof) => {
            RootProofRecord::IcCanisterSignatureV1(IcCanisterSignatureProofRecord {
                signature_cbor: proof.signature_cbor,
                public_key_der: proof.public_key_der,
            })
        }
    }
}

fn root_proof_record_to_dto(record: RootProofRecord) -> RootProof {
    match record {
        RootProofRecord::IcCanisterSignatureV1(proof) => {
            RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: proof.signature_cbor,
                public_key_der: proof.public_key_der,
            })
        }
    }
}

fn audience_to_record(audience: DelegationAudience) -> DelegationAudienceRecord {
    match audience {
        DelegationAudience::Canister(canister) => DelegationAudienceRecord::Canister(canister),
        DelegationAudience::CanicSubnet(subnet) => DelegationAudienceRecord::CanicSubnet(subnet),
        DelegationAudience::Project(project) => DelegationAudienceRecord::Project(project),
    }
}

fn audience_record_to_dto(record: DelegationAudienceRecord) -> DelegationAudience {
    match record {
        DelegationAudienceRecord::Canister(canister) => DelegationAudience::Canister(canister),
        DelegationAudienceRecord::CanicSubnet(subnet) => DelegationAudience::CanicSubnet(subnet),
        DelegationAudienceRecord::Project(project) => DelegationAudience::Project(project),
    }
}

fn grant_to_record(grant: DelegatedRoleGrant) -> DelegatedRoleGrantRecord {
    DelegatedRoleGrantRecord {
        target: grant.target,
        scopes: grant.scopes,
    }
}

fn grant_record_to_dto(record: DelegatedRoleGrantRecord) -> DelegatedRoleGrant {
    DelegatedRoleGrant {
        target: record.target,
        scopes: record.scopes,
    }
}

fn audience_record_to_policy(record: DelegationAudienceRecord) -> RootDelegationAudiencePolicy {
    match record {
        DelegationAudienceRecord::Canister(canister) => {
            RootDelegationAudiencePolicy::Canister(canister)
        }
        DelegationAudienceRecord::CanicSubnet(subnet) => {
            RootDelegationAudiencePolicy::CanicSubnet(subnet)
        }
        DelegationAudienceRecord::Project(project) => {
            RootDelegationAudiencePolicy::Project(project)
        }
    }
}

#[cfg(any(test, canic_test_delegation_material))]
fn audience_policy_to_record(policy: RootDelegationAudiencePolicy) -> DelegationAudienceRecord {
    match policy {
        RootDelegationAudiencePolicy::Canister(canister) => {
            DelegationAudienceRecord::Canister(canister)
        }
        RootDelegationAudiencePolicy::CanicSubnet(subnet) => {
            DelegationAudienceRecord::CanicSubnet(subnet)
        }
        RootDelegationAudiencePolicy::Project(project) => {
            DelegationAudienceRecord::Project(project)
        }
    }
}

fn grant_record_to_policy(record: DelegatedRoleGrantRecord) -> RootDelegatedRoleGrantPolicy {
    RootDelegatedRoleGrantPolicy {
        target: record.target,
        scopes: record.scopes,
    }
}

#[cfg(any(test, canic_test_delegation_material))]
fn grant_policy_to_record(policy: RootDelegatedRoleGrantPolicy) -> DelegatedRoleGrantRecord {
    DelegatedRoleGrantRecord {
        target: policy.target,
        scopes: policy.scopes,
    }
}

const fn issuer_proof_alg_to_record(alg: IssuerProofAlgorithm) -> IssuerProofAlgorithmRecord {
    match alg {
        IssuerProofAlgorithm::IcCanisterSignatureV1 => {
            IssuerProofAlgorithmRecord::IcCanisterSignatureV1
        }
    }
}

const fn issuer_proof_alg_record_to_dto(
    record: IssuerProofAlgorithmRecord,
) -> IssuerProofAlgorithm {
    match record {
        IssuerProofAlgorithmRecord::IcCanisterSignatureV1 => {
            IssuerProofAlgorithm::IcCanisterSignatureV1
        }
    }
}

const fn issuer_proof_binding_to_record(binding: IssuerProofBinding) -> IssuerProofBindingRecord {
    match binding {
        IssuerProofBinding::IcCanisterSignatureV1 { seed_hash } => {
            IssuerProofBindingRecord::IcCanisterSignatureV1 { seed_hash }
        }
    }
}

const fn issuer_proof_binding_record_to_dto(
    record: IssuerProofBindingRecord,
) -> IssuerProofBinding {
    match record {
        IssuerProofBindingRecord::IcCanisterSignatureV1 { seed_hash } => {
            IssuerProofBinding::IcCanisterSignatureV1 { seed_hash }
        }
    }
}
