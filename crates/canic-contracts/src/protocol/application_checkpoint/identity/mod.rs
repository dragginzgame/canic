//! Canonical current checkpoint identities shared by Root and host admission.
//!
//! Wire authority bytes and explicit domains are independent of stable-value envelopes.

use crate::{ids::CallerReceiverAuthority, protocol::authority};
use sha2::{Digest, Sha256};

/// Commit one complete installed audience using the current Wire checkpoint domain.
pub fn authority_digest(authority: &CallerReceiverAuthority) -> Result<[u8; 32], ic_wire::Error> {
    let mut digest = Sha256::new();
    digest.update(b"canic.application-checkpoint.authority.wire.v1\0");
    digest.update(authority::encode(authority)?);
    Ok(digest.finalize().into())
}

/// Commit the complete canonical principal-ordered unit, without changing membership.
/// The caller validates uniqueness, authority and count before invoking this helper.
pub fn unit_digest(members: &[CallerReceiverAuthority]) -> Result<[u8; 32], ic_wire::Error> {
    let mut digest = Sha256::new();
    digest.update(b"canic.application-checkpoint.unit.wire.v1\0");
    for member in members {
        digest.update(authority_digest(member)?);
    }
    Ok(digest.finalize().into())
}
