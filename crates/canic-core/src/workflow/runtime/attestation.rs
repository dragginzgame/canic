use crate::workflow::prelude::*;

///
/// RoleAttestationKeyRefreshWorkflow
///

pub struct RoleAttestationKeyRefreshWorkflow;

impl RoleAttestationKeyRefreshWorkflow {
    // Role attestations are self-contained canister-signature proofs in 0.65.
    // The old root ECDSA public-key refresh timer is intentionally inert.
    pub fn start() {
        log!(
            Topic::Auth,
            Debug,
            "attestation key refresh skipped: role attestations use embedded canister-signature proofs"
        );
    }
}
