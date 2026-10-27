//! Lossless Root issuer administration projections; cryptographic contracts stay unchanged.
use crate::dto::{auth as stored, issuer_control as native};
impl From<stored::RootIssuerConfigureRequest> for native::RootIssuerConfigureRequest {
    fn from(value: stored::RootIssuerConfigureRequest) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
            enabled: value.enabled,
            aud: value.aud,
            grants: value.grants,
            cert_ttl_ns: value.cert_ttl_ns,
            refresh_after_ratio_bps: value.refresh_after_ratio_bps,
        }
    }
}
impl From<native::RootIssuerConfigureRequest> for stored::RootIssuerConfigureRequest {
    fn from(value: native::RootIssuerConfigureRequest) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
            enabled: value.enabled,
            aud: value.aud,
            grants: value.grants,
            cert_ttl_ns: value.cert_ttl_ns,
            refresh_after_ratio_bps: value.refresh_after_ratio_bps,
        }
    }
}
impl From<stored::RootIssuerPolicyView> for native::RootIssuerPolicyView {
    fn from(value: stored::RootIssuerPolicyView) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
            enabled: value.enabled,
            allowed_audiences: value.allowed_audiences,
            allowed_grants: value.allowed_grants,
            max_cert_ttl_ns: value.max_cert_ttl_ns,
            refresh_after_ratio_bps: value.refresh_after_ratio_bps,
        }
    }
}
impl From<native::RootIssuerPolicyView> for stored::RootIssuerPolicyView {
    fn from(value: native::RootIssuerPolicyView) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
            enabled: value.enabled,
            allowed_audiences: value.allowed_audiences,
            allowed_grants: value.allowed_grants,
            max_cert_ttl_ns: value.max_cert_ttl_ns,
            refresh_after_ratio_bps: value.refresh_after_ratio_bps,
        }
    }
}
impl From<stored::RootIssuerConfigureResponse> for native::RootIssuerConfigureResponse {
    fn from(value: stored::RootIssuerConfigureResponse) -> Self {
        Self {
            issuer: value.issuer.into(),
            template: value.template.into(),
        }
    }
}
impl From<native::RootIssuerConfigureResponse> for stored::RootIssuerConfigureResponse {
    fn from(value: native::RootIssuerConfigureResponse) -> Self {
        Self {
            issuer: value.issuer.into(),
            template: value.template.into(),
        }
    }
}
impl From<stored::RootDelegationProofBatchProof> for native::RootDelegationProofBatchProof {
    fn from(value: stored::RootDelegationProofBatchProof) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
            cert_hash: value.cert_hash,
            proof: value.proof.into(),
        }
    }
}
impl From<native::RootDelegationProofBatchProof> for stored::RootDelegationProofBatchProof {
    fn from(value: native::RootDelegationProofBatchProof) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
            cert_hash: value.cert_hash,
            proof: value.proof.into(),
        }
    }
}

#[cfg(test)]
mod tests;
