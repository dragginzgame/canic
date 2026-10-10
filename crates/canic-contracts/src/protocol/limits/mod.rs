//! Shared wire-envelope byte limits; endpoint admission remains runtime-owned.

/// Default maximum encoded update command size.
pub const DEFAULT_UPDATE_INGRESS_MAX_BYTES: usize = 16_384;
