//! Module: ops::root_draining_reservation
//!
//! Responsibility: hash exact Coordinator-owned root-draining reservations.
//! Does not own: reservation persistence, transport, or root lifecycle mutation.
//! Boundary: Coordinator production and root consumption share this canonical authority.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::fleet_registry::FleetSubnetRootDrainingReservationResponse,
};
use sha2::{Digest, Sha256};

const ROOT_DRAINING_RESERVATION_HASH_DOMAIN: &[u8] =
    b"canic/fleet-subnet-root/draining-reservation/v1";

/// Canonical hashing boundary shared by Coordinator production and root verification.
pub struct FleetSubnetRootDrainingReservationOps;

impl FleetSubnetRootDrainingReservationOps {
    /// Hash one exact reservation after excluding its self-referential hash field.
    pub fn content_hash(
        response: &FleetSubnetRootDrainingReservationResponse,
    ) -> Result<[u8; 32], InternalError> {
        let mut authority = response.clone();
        authority.reservation_hash = [0; 32];
        let payload = candid::encode_one(authority).map_err(|error| {
            InternalError::invariant(
                InternalErrorOrigin::Ops,
                format!("Fleet Subnet Root draining reservation cannot be encoded: {error}"),
            )
        })?;
        let mut hasher = Sha256::new();
        hasher.update(ROOT_DRAINING_RESERVATION_HASH_DOMAIN);
        hasher.update(payload);
        Ok(hasher.finalize().into())
    }
}

#[cfg(test)]
mod tests;
