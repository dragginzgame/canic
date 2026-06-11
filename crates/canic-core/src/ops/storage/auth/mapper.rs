use crate::{
    dto::auth::{
        ActiveDelegationProof, AttestationKey, AttestationKeyStatus, DelegatedRoleGrant,
        DelegationAudience, DelegationCert, DelegationProof, IcCanisterSignatureProofV1, RootProof,
        ShardKeyBinding, ShardSignatureAlgorithm,
    },
    storage::stable::auth::{
        ActiveDelegationProofRecord, AttestationKeyStatusRecord, AttestationPublicKeyRecord,
        DelegatedRoleGrantRecord, DelegationAudienceRecord, DelegationCertRecord,
        DelegationProofRecord, IcCanisterSignatureProofRecord, RootProofRecord,
        ShardKeyBindingRecord, ShardSignatureAlgorithmRecord,
    },
};

///
/// AttestationPublicKeyRecordMapper
///

pub struct AttestationPublicKeyRecordMapper;

impl AttestationPublicKeyRecordMapper {
    #[must_use]
    pub fn dto_to_record(key: AttestationKey) -> AttestationPublicKeyRecord {
        AttestationPublicKeyRecord {
            key_id: key.key_id,
            key_hash: key.key_hash,
            key_name: key.key_name,
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
            key_name: record.key_name,
            key_hash: record.key_hash,
            status: match record.status {
                AttestationKeyStatusRecord::Current => AttestationKeyStatus::Current,
                AttestationKeyStatusRecord::Previous => AttestationKeyStatus::Previous,
            },
            valid_from: record.valid_from,
            valid_until: record.valid_until,
        }
    }
}

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
        shard_pid: cert.shard_pid,
        shard_key_id: cert.shard_key_id,
        shard_sig_alg: shard_sig_alg_to_record(cert.shard_sig_alg),
        shard_public_key_sec1: cert.shard_public_key_sec1,
        shard_key_hash: cert.shard_key_hash,
        shard_key_binding: shard_key_binding_to_record(cert.shard_key_binding),
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
        shard_pid: record.shard_pid,
        shard_key_id: record.shard_key_id,
        shard_sig_alg: shard_sig_alg_record_to_dto(record.shard_sig_alg),
        shard_public_key_sec1: record.shard_public_key_sec1,
        shard_key_hash: record.shard_key_hash,
        shard_key_binding: shard_key_binding_record_to_dto(record.shard_key_binding),
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

const fn shard_sig_alg_to_record(alg: ShardSignatureAlgorithm) -> ShardSignatureAlgorithmRecord {
    match alg {
        ShardSignatureAlgorithm::IcThresholdEcdsaSecp256k1 => {
            ShardSignatureAlgorithmRecord::IcThresholdEcdsaSecp256k1
        }
    }
}

const fn shard_sig_alg_record_to_dto(
    record: ShardSignatureAlgorithmRecord,
) -> ShardSignatureAlgorithm {
    match record {
        ShardSignatureAlgorithmRecord::IcThresholdEcdsaSecp256k1 => {
            ShardSignatureAlgorithm::IcThresholdEcdsaSecp256k1
        }
    }
}

const fn shard_key_binding_to_record(binding: ShardKeyBinding) -> ShardKeyBindingRecord {
    match binding {
        ShardKeyBinding::IcThresholdEcdsaSecp256k1 {
            key_name_hash,
            derivation_path_hash,
        } => ShardKeyBindingRecord::IcThresholdEcdsaSecp256k1 {
            key_name_hash,
            derivation_path_hash,
        },
    }
}

const fn shard_key_binding_record_to_dto(record: ShardKeyBindingRecord) -> ShardKeyBinding {
    match record {
        ShardKeyBindingRecord::IcThresholdEcdsaSecp256k1 {
            key_name_hash,
            derivation_path_hash,
        } => ShardKeyBinding::IcThresholdEcdsaSecp256k1 {
            key_name_hash,
            derivation_path_hash,
        },
    }
}
