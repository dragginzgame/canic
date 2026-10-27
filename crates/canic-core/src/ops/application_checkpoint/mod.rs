//! Convert checkpoint hook commands against independently installed receiver authority.
//!
//! No application hooks, storage commits or remote effects occur here.

use crate::{InternalError, cdk::serialize::serialize, ops::caller_authority::CallerAuthorityOps};
use canic_contracts::{
    dto::application_checkpoint::ApplicationCheckpointCommand,
    ids::{ApplicationCheckpointBinding, ApplicationCheckpointPhase, CallerReceiverAuthority},
};
use sha2::{Digest, Sha256};

/// Commit the full installed audience using the existing canonical storage encoder.
pub fn authority_digest(authority: &CallerReceiverAuthority) -> Result<[u8; 32], InternalError> {
    let mut digest = Sha256::new();
    digest.update(b"canic.application-checkpoint.authority.v1\0");
    digest.update(serialize(authority).map_err(|_| InternalError::invariant())?);
    Ok(digest.finalize().into())
}

/// Project an original binding into the bounded application hook contract.
pub fn command(
    binding: &ApplicationCheckpointBinding,
    phase: ApplicationCheckpointPhase,
) -> Result<ApplicationCheckpointCommand, InternalError> {
    Ok(ApplicationCheckpointCommand {
        authority_digest: authority_digest(&binding.authority)?,
        operation_id: binding.operation_id,
        sequence: binding.sequence,
        unit_digest: binding.unit_digest,
        phase,
    })
}

/// Recover the exact request binding only from current protected receiver state.
pub fn binding(
    request: &ApplicationCheckpointCommand,
) -> Result<ApplicationCheckpointBinding, InternalError> {
    let current = CallerAuthorityOps::receiver().ok_or_else(InternalError::unavailable)?;
    if authority_digest(&current.authority)? != request.authority_digest {
        return Err(InternalError::conflict());
    }
    Ok(ApplicationCheckpointBinding {
        authority: current.authority,
        operation_id: request.operation_id,
        sequence: request.sequence,
        unit_digest: request.unit_digest,
    })
}
