//! Lossless proof boundary conversions; never re-encode signed data or change deadlines.
use crate::dto::{auth as stored, issuer_proof as native};

impl From<stored::ChainKeyBatchHeaderV1> for native::ChainKeyBatchHeaderV1 {
    fn from(value: stored::ChainKeyBatchHeaderV1) -> Self {
        Self {
            schema_version: value.schema_version,
            root_canister_id: value.root_canister_id.into(),
            batch_id: value.batch_id,
            proof_epoch: value.proof_epoch,
            registry_epoch: value.registry_epoch,
            registry_hash: value.registry_hash,
            tree_root: value.tree_root,
            not_before_ns: value.not_before_ns,
            expires_at_ns: value.expires_at_ns,
            algorithm: value.algorithm,
            key_id: value.key_id,
            derivation_path_hash: value.derivation_path_hash,
            key_version: value.key_version,
        }
    }
}

impl From<native::ChainKeyBatchHeaderV1> for stored::ChainKeyBatchHeaderV1 {
    fn from(value: native::ChainKeyBatchHeaderV1) -> Self {
        Self {
            schema_version: value.schema_version,
            root_canister_id: value.root_canister_id.into(),
            batch_id: value.batch_id,
            proof_epoch: value.proof_epoch,
            registry_epoch: value.registry_epoch,
            registry_hash: value.registry_hash,
            tree_root: value.tree_root,
            not_before_ns: value.not_before_ns,
            expires_at_ns: value.expires_at_ns,
            algorithm: value.algorithm,
            key_id: value.key_id,
            derivation_path_hash: value.derivation_path_hash,
            key_version: value.key_version,
        }
    }
}

impl From<stored::IcChainKeyBatchSignatureProofV1> for native::IcChainKeyBatchSignatureProofV1 {
    fn from(value: stored::IcChainKeyBatchSignatureProofV1) -> Self {
        Self {
            header: value.header.into(),
            delegation_cert: value.delegation_cert.into(),
            issuer_witness: value.issuer_witness,
            signature: value.signature,
        }
    }
}

impl From<native::IcChainKeyBatchSignatureProofV1> for stored::IcChainKeyBatchSignatureProofV1 {
    fn from(value: native::IcChainKeyBatchSignatureProofV1) -> Self {
        Self {
            header: value.header.into(),
            delegation_cert: value.delegation_cert.into(),
            issuer_witness: value.issuer_witness,
            signature: value.signature,
        }
    }
}

impl From<stored::ChainKeyDelegationCertV1> for native::ChainKeyDelegationCertV1 {
    fn from(value: stored::ChainKeyDelegationCertV1) -> Self {
        Self {
            root_canister_id: value.root_canister_id.into(),
            issuer_canister_id: value.issuer_canister_id.into(),
            proof_epoch: value.proof_epoch,
            issuer_proof_algorithm: value.issuer_proof_algorithm,
            issuer_proof_binding_hash: value.issuer_proof_binding_hash,
            issuer_proof_binding: value.issuer_proof_binding,
            max_token_ttl_ns: value.max_token_ttl_ns,
            audience: value.audience,
            grants: value.grants,
            not_before_ns: value.not_before_ns,
            expires_at_ns: value.expires_at_ns,
            registry_epoch: value.registry_epoch,
            registry_hash: value.registry_hash,
        }
    }
}

impl From<native::ChainKeyDelegationCertV1> for stored::ChainKeyDelegationCertV1 {
    fn from(value: native::ChainKeyDelegationCertV1) -> Self {
        Self {
            root_canister_id: value.root_canister_id.into(),
            issuer_canister_id: value.issuer_canister_id.into(),
            proof_epoch: value.proof_epoch,
            issuer_proof_algorithm: value.issuer_proof_algorithm,
            issuer_proof_binding_hash: value.issuer_proof_binding_hash,
            issuer_proof_binding: value.issuer_proof_binding,
            max_token_ttl_ns: value.max_token_ttl_ns,
            audience: value.audience,
            grants: value.grants,
            not_before_ns: value.not_before_ns,
            expires_at_ns: value.expires_at_ns,
            registry_epoch: value.registry_epoch,
            registry_hash: value.registry_hash,
        }
    }
}

impl From<stored::DelegationCert> for native::DelegationCert {
    fn from(value: stored::DelegationCert) -> Self {
        Self {
            root_pid: value.root_pid.into(),
            issuer_pid: value.issuer_pid.into(),
            issuer_proof_alg: value.issuer_proof_alg,
            issuer_proof_binding_hash: value.issuer_proof_binding_hash,
            issuer_proof_binding: value.issuer_proof_binding,
            issued_at_ns: value.issued_at_ns,
            not_before_ns: value.not_before_ns,
            expires_at_ns: value.expires_at_ns,
            max_token_ttl_ns: value.max_token_ttl_ns,
            aud: value.aud,
            grants: value.grants,
        }
    }
}

impl From<native::DelegationCert> for stored::DelegationCert {
    fn from(value: native::DelegationCert) -> Self {
        Self {
            root_pid: value.root_pid.into(),
            issuer_pid: value.issuer_pid.into(),
            issuer_proof_alg: value.issuer_proof_alg,
            issuer_proof_binding_hash: value.issuer_proof_binding_hash,
            issuer_proof_binding: value.issuer_proof_binding,
            issued_at_ns: value.issued_at_ns,
            not_before_ns: value.not_before_ns,
            expires_at_ns: value.expires_at_ns,
            max_token_ttl_ns: value.max_token_ttl_ns,
            aud: value.aud,
            grants: value.grants,
        }
    }
}

impl From<stored::DelegationProof> for native::DelegationProof {
    fn from(value: stored::DelegationProof) -> Self {
        Self {
            cert: value.cert.into(),
            root_proof: value.root_proof.into(),
        }
    }
}

impl From<native::DelegationProof> for stored::DelegationProof {
    fn from(value: native::DelegationProof) -> Self {
        Self {
            cert: value.cert.into(),
            root_proof: value.root_proof.into(),
        }
    }
}

impl From<stored::ActiveDelegationProof> for native::ActiveDelegationProof {
    fn from(value: stored::ActiveDelegationProof) -> Self {
        Self {
            proof: value.proof.into(),
            cert_hash: value.cert_hash,
            not_before_ns: value.not_before_ns,
            expires_at_ns: value.expires_at_ns,
            refresh_after_ns: value.refresh_after_ns,
            installed_at_ns: value.installed_at_ns,
            installed_by: value.installed_by.into(),
        }
    }
}

impl From<native::ActiveDelegationProof> for stored::ActiveDelegationProof {
    fn from(value: native::ActiveDelegationProof) -> Self {
        Self {
            proof: value.proof.into(),
            cert_hash: value.cert_hash,
            not_before_ns: value.not_before_ns,
            expires_at_ns: value.expires_at_ns,
            refresh_after_ns: value.refresh_after_ns,
            installed_at_ns: value.installed_at_ns,
            installed_by: value.installed_by.into(),
        }
    }
}

impl From<stored::InstallActiveDelegationProofRequest>
    for native::InstallActiveDelegationProofRequest
{
    fn from(value: stored::InstallActiveDelegationProofRequest) -> Self {
        Self {
            proof: value.proof.into(),
        }
    }
}

impl From<native::InstallActiveDelegationProofRequest>
    for stored::InstallActiveDelegationProofRequest
{
    fn from(value: native::InstallActiveDelegationProofRequest) -> Self {
        Self {
            proof: value.proof.into(),
        }
    }
}

impl From<stored::InstallActiveDelegationProofResponse>
    for native::InstallActiveDelegationProofResponse
{
    fn from(value: stored::InstallActiveDelegationProofResponse) -> Self {
        Self {
            active_proof: value.active_proof.into(),
        }
    }
}

impl From<native::InstallActiveDelegationProofResponse>
    for stored::InstallActiveDelegationProofResponse
{
    fn from(value: native::InstallActiveDelegationProofResponse) -> Self {
        Self {
            active_proof: value.active_proof.into(),
        }
    }
}

impl From<stored::RootProof> for native::RootProof {
    fn from(value: stored::RootProof) -> Self {
        match value {
            stored::RootProof::IcChainKeyBatchSignatureV1(proof) => {
                Self::IcChainKeyBatchSignatureV1(proof.into())
            }
        }
    }
}

impl From<native::RootProof> for stored::RootProof {
    fn from(value: native::RootProof) -> Self {
        match value {
            native::RootProof::IcChainKeyBatchSignatureV1(proof) => {
                Self::IcChainKeyBatchSignatureV1(proof.into())
            }
        }
    }
}

impl From<stored::ActiveDelegationProofStatusResponse>
    for native::ActiveDelegationProofStatusResponse
{
    fn from(value: stored::ActiveDelegationProofStatusResponse) -> Self {
        Self {
            status: value.status,
            root_pid: value.root_pid.map(Into::into),
            issuer_pid: value.issuer_pid.map(Into::into),
            cert_hash: value.cert_hash,
            expires_at_ns: value.expires_at_ns,
            refresh_after_ns: value.refresh_after_ns,
        }
    }
}

impl From<native::ActiveDelegationProofStatusResponse>
    for stored::ActiveDelegationProofStatusResponse
{
    fn from(value: native::ActiveDelegationProofStatusResponse) -> Self {
        Self {
            status: value.status,
            root_pid: value.root_pid.map(Into::into),
            issuer_pid: value.issuer_pid.map(Into::into),
            cert_hash: value.cert_hash,
            expires_at_ns: value.expires_at_ns,
            refresh_after_ns: value.refresh_after_ns,
        }
    }
}
