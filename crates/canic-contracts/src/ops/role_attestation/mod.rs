//! Lossless role-attestation boundary projections.
use crate::dto::{auth as stored, role_attestation as native};
impl From<stored::RoleAttestationRequest> for native::RoleAttestationRequest {
    fn from(value: stored::RoleAttestationRequest) -> Self {
        Self {
            subject: value.subject.into(),
            role: value.role,
            subnet_id: value.subnet_id.map(Into::into),
            audience: value.audience.into(),
            ttl_ns: value.ttl_ns,
            epoch: value.epoch,
            metadata: value.metadata,
        }
    }
}
impl From<native::RoleAttestationRequest> for stored::RoleAttestationRequest {
    fn from(value: native::RoleAttestationRequest) -> Self {
        Self {
            subject: value.subject.into(),
            role: value.role,
            subnet_id: value.subnet_id.map(Into::into),
            audience: value.audience.into(),
            ttl_ns: value.ttl_ns,
            epoch: value.epoch,
            metadata: value.metadata,
        }
    }
}
impl From<stored::RoleAttestation> for native::RoleAttestation {
    fn from(value: stored::RoleAttestation) -> Self {
        Self {
            subject: value.subject.into(),
            role: value.role,
            subnet_id: value.subnet_id.map(Into::into),
            audience: value.audience.into(),
            issued_at_ns: value.issued_at_ns,
            expires_at_ns: value.expires_at_ns,
            epoch: value.epoch,
        }
    }
}
impl From<native::RoleAttestation> for stored::RoleAttestation {
    fn from(value: native::RoleAttestation) -> Self {
        Self {
            subject: value.subject.into(),
            role: value.role,
            subnet_id: value.subnet_id.map(Into::into),
            audience: value.audience.into(),
            issued_at_ns: value.issued_at_ns,
            expires_at_ns: value.expires_at_ns,
            epoch: value.epoch,
        }
    }
}
impl From<stored::RoleAttestationPrepareResponse> for native::RoleAttestationPrepareResponse {
    fn from(value: stored::RoleAttestationPrepareResponse) -> Self {
        Self {
            payload: value.payload.into(),
            payload_hash: value.payload_hash,
            retrieval_expires_at_ns: value.retrieval_expires_at_ns,
        }
    }
}
impl From<native::RoleAttestationPrepareResponse> for stored::RoleAttestationPrepareResponse {
    fn from(value: native::RoleAttestationPrepareResponse) -> Self {
        Self {
            payload: value.payload.into(),
            payload_hash: value.payload_hash,
            retrieval_expires_at_ns: value.retrieval_expires_at_ns,
        }
    }
}
impl From<stored::SignedRoleAttestation> for native::SignedRoleAttestation {
    fn from(value: stored::SignedRoleAttestation) -> Self {
        Self {
            payload: value.payload.into(),
            root_proof: value.root_proof,
        }
    }
}
impl From<native::SignedRoleAttestation> for stored::SignedRoleAttestation {
    fn from(value: native::SignedRoleAttestation) -> Self {
        Self {
            payload: value.payload.into(),
            root_proof: value.root_proof,
        }
    }
}

#[cfg(test)]
mod tests;
