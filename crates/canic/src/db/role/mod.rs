//! Hub and shard installation configuration and local state attachment.

mod actor;

use crate::{auth::install::Install, topology};
pub use actor::{
    HubConfig, decode_hub, hub_readiness, open_hub, open_shard, root, shard_readiness,
};

fn nonce(bytes: &[u8]) -> Result<[u8; 16], &'static str> {
    let nonce = bytes
        .try_into()
        .map_err(|_| "expected a 16-byte install nonce")?;
    if nonce == [0; 16] {
        return Err("zero installation nonce");
    }
    Ok(nonce)
}

#[doc(hidden)]
pub struct ShardConfig {
    binding: topology::Install,
    nonce: [u8; 16],
}

impl ShardConfig {
    /// Consume the validated installation context and its application nonce.
    #[must_use]
    pub fn into_parts(self) -> (topology::Install, [u8; 16]) {
        (self.binding, self.nonce)
    }
}

#[doc(hidden)]
/// Decode the shard's nested topology and check the outer Auth Root.
/// # Errors
/// Rejects a missing frame, conflicting Root or invalid nonce.
pub fn decode_shard(input: &Install) -> Result<ShardConfig, &'static str> {
    let binding = topology::Install::decode(&input.application)?;
    binding.check_root(input.root)?;
    Ok(ShardConfig {
        nonce: nonce(&binding.application)?,
        binding,
    })
}
