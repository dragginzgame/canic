//! Bind an effect-free release review to exact physical inventory and current observations.
//!
//! Sealing grants no effect authority. Physical observations separately consume caller-reserved reads.

pub mod accounts;
pub mod funding;
pub mod inventory;
pub mod observation;
pub mod reservation;

use crate::fleet_ensure::{
    model::release::FleetReleaseReviewRecord,
    policy::release::{FleetReleaseError, validate_review},
    view::release::FleetReleaseObservation,
};
use sha2_host::{Digest, Sha256};

/// Seal reviewed evidence for inspection, without granting reset or spending authority.
pub fn prepare_review(
    mut review: FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
) -> Result<FleetReleaseReviewRecord, FleetReleaseError> {
    validate_review(&review, observed)?;
    for account in &mut review.accounts {
        account.subaccount = account
            .subaccount
            .filter(|subaccount| *subaccount != [0; 32]);
    }
    review.review_sha256 = [0; 32];
    review.review_sha256 = digest(&review)?;
    Ok(review)
}

/// Reject altered retained review material before comparing fresh authority.
pub fn verify_review(
    review: &FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
) -> Result<(), FleetReleaseError> {
    verify_digest(review)?;
    validate_review(review, observed)
}

/// Check retained review integrity and child custody before any owner is reset.
pub fn verify_reset_custody(
    review: &FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
) -> Result<(), FleetReleaseError> {
    verify_digest(review)?;
    crate::fleet_ensure::policy::release::validate_reset_custody(review, observed)
}

/// Check retained review integrity and empty held capacity without predecessor endpoints.
pub fn verify_held_capacity(
    review: &FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
) -> Result<(), FleetReleaseError> {
    verify_digest(review)?;
    crate::fleet_ensure::policy::release::validate_held_capacity(review, observed)
}

pub(super) fn verify_digest(review: &FleetReleaseReviewRecord) -> Result<(), FleetReleaseError> {
    let mut unsigned = review.clone();
    unsigned.review_sha256 = [0; 32];
    if review.review_sha256 != digest(&unsigned)? {
        return Err(FleetReleaseError::Authority);
    }
    Ok(())
}

fn digest(review: &FleetReleaseReviewRecord) -> Result<[u8; 32], FleetReleaseError> {
    let mut hash = Sha256::new();
    hash.update(b"canic.fleet-release.review.v1\0");
    hash.update(serde_json::to_vec(review).map_err(|_| FleetReleaseError::Authority)?);
    Ok(hash.finalize().into())
}
