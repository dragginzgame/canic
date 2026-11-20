//! Compose application roles from their explicitly selected actor adapters.
mod mount;

#[cfg(feature = "stateful-role")]
mod actor;
#[cfg(feature = "stateful-role")]
pub use actor::{
    HubConfig, ShardConfig, decode_hub, decode_shard, hub_readiness, open_hub, open_shard, root,
    shard_readiness,
};

// Select the directory inside the role mount so linking Canic alone adds no endpoint.
#[cfg(feature = "role-attestation")]
#[doc(hidden)]
pub use crate::mount_service_directory as mount_directory;
#[cfg(not(feature = "role-attestation"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_no_role_directory {
    () => {};
}
#[cfg(not(feature = "role-attestation"))]
#[doc(hidden)]
pub use crate::__canic_no_role_directory as mount_directory;
