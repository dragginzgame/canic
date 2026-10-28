//! Canonical existing checkpoint identities shared by Root and host admission.
//!
//! These retained v1 identities preserve their original storage-codec bytes.

use crate::{
    ids::CallerReceiverAuthority,
    serialization::{SerializeError, serialize},
};
use sha2::{Digest, Sha256};

/// Commit one complete installed audience using the existing checkpoint domain.
pub fn authority_digest(authority: &CallerReceiverAuthority) -> Result<[u8; 32], SerializeError> {
    let mut digest = Sha256::new();
    digest.update(b"canic.application-checkpoint.authority.v1\0");
    digest.update(serialize(authority)?);
    Ok(digest.finalize().into())
}

/// Commit the complete canonical principal-ordered unit, without changing membership.
/// The caller validates uniqueness, authority and count before invoking this helper.
pub fn unit_digest(members: &[CallerReceiverAuthority]) -> Result<[u8; 32], SerializeError> {
    let mut digest = Sha256::new();
    digest.update(b"canic.application-checkpoint.unit.v1\0");
    for member in members {
        digest.update(authority_digest(member)?);
    }
    Ok(digest.finalize().into())
}
