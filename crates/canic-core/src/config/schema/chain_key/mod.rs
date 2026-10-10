//! Select the public master key used by host-side configuration derivation.
//!
//! This schema contains no cryptographic implementation or runtime key discovery.

use canic_contracts::ids::BuildNetwork;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Public master-key environment selected explicitly by the App configuration.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainKeyPublicKeyDerivation {
    Ic,
    Pocketic,
}

/// Host configuration failures that must stop before artifact construction.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ChainKeyDerivationError {
    #[error("offline chain-key derivation requires delegated token authentication to be enabled")]
    AuthDisabled,

    #[error(
        "offline chain-key derivation produced a different {field}; remove the stale value or correct the derivation inputs"
    )]
    DerivedValueMismatch { field: &'static str },

    #[error("offline chain-key derivation requires valid hexadecimal bytes in {field}")]
    InvalidHex { field: &'static str },

    #[error("offline chain-key derivation requires root_canister_id to identify a canister")]
    InvalidRoot,

    #[error("offline chain-key derivation requires auth.delegated_tokens.{field}")]
    MissingField { field: &'static str },

    #[error(
        "offline chain-key derivation {derivation:?} cannot be used with build_network {network:?}"
    )]
    NetworkMismatch {
        derivation: ChainKeyPublicKeyDerivation,
        network: BuildNetwork,
    },

    #[error("chain-key derivation paths may contain at most 255 components")]
    PathTooLong,

    #[error("offline chain-key derivation {derivation:?} does not support key_id {key_id}")]
    UnsupportedKey {
        derivation: ChainKeyPublicKeyDerivation,
        key_id: String,
    },
}
