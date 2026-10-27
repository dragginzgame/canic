//! Application endpoints use native Wire; external Candid and lifecycle bounds remain distinct.
//!
//! Use `wire(id = METHOD_ID, request = REQUEST_LIMITS, response = RESPONSE_LIMITS)`
//! for Canic-owned application methods. See the native Wire section below for
//! framing, generated schemas and caller behavior. The following `ArgumentLimits`
//! examples describe the external Candid adapter only.
//!
//! The artifact selects all five limits. Transport size is checked before the
//! argument buffer is allocated; Candid then decodes under one shared budget.
//!
//! # External Candid adapter: guarded plain replies
//!
//! `on_access_denied = "reject"` sends an IC rejection on access refusal and
//! preserves the handler's declared Candid success shape. Default Fleet guards,
//! custom predicates, preflight and denial-only metrics still apply. Predicates
//! may await before dispatch; a synchronous handler has no await before reply.
//!
//! ```no_run
//! use canic::endpoint::ArgumentLimits;
//! const LIMITS: ArgumentLimits = ArgumentLimits {
//!     max_bytes: 32 * 1024, decoding_quota: 100_000, skipping_quota: 10_000,
//!     max_type_len: 128, max_header_len: 4096,
//! };
//! #[canic::canic_update(requires(caller::is_controller()),
//!     on_access_denied = "reject", decode = LIMITS)]
//! fn accept(bytes: Vec<u8>) -> u64 { bytes.len() as u64 }
//! #[canic::canic_query(public, on_access_denied = "reject", decode = LIMITS)]
//! fn size(bytes: Vec<u8>) -> u64 { bytes.len() as u64 }
//! ```
//!
//! Select a separate `ic_wire::Limits` value with `argument_limits = LIMITS,`
//! first in the owning `canic::start!` or
//! `canic::start_local!` invocation to bound the initial init/post-upgrade
//! native Wire envelope before restoration or participants. Opaque nested application bytes
//! still require their own semantic checks and bounded decoding.
//!
//! # Invalid combinations
//!
//! Plain guarded replies require an explicit rejection choice:
//!
//! ```compile_fail
//! #[canic::canic_query(public)]
//! fn plain() -> u64 { 1 }
//! ```
//!
//! Rejection mode does not remove the explicit access declaration:
//!
//! ```compile_fail
//! #[canic::canic_update(public, requires(caller::is_controller()), on_access_denied = "reject")]
//! fn conflicting_access() -> u64 { 1 }
//! ```
//!
//! The mode is explicit and supports only `"reject"`:
//!
//! ```compile_fail
//! #[canic::canic_update(public, on_access_denied = "trap")]
//! fn invalid_mode() -> u64 { 1 }
//! ```
//!
//! `decode` requires a constant `ArgumentLimits`, not a byte count:
//!
//! ```compile_fail,E0308
//! #[canic::canic_query(public, decode = 1024)]
//! fn wrong_policy() -> Result<(), canic::Error> { Ok(()) }
//! ```
//!
//! `decode` owns the byte limit, so a separate `payload` is ambiguous:
//!
//! ```compile_fail
//! const LIMITS: canic::endpoint::ArgumentLimits = canic::endpoint::ArgumentLimits {
//!     max_bytes: 1024, decoding_quota: 1000, skipping_quota: 100,
//!     max_type_len: 16, max_header_len: 128,
//! };
//! #[canic::canic_update(public, decode = LIMITS, payload(max_bytes = 1024))]
//! fn conflicting_limits() -> Result<(), canic::Error> { Ok(()) }
//! ```
//!
//! Duplicate decoder declarations do not compile:
//!
//! ```compile_fail
//! const LIMITS: canic::endpoint::ArgumentLimits = canic::endpoint::ArgumentLimits {
//!     max_bytes: 1024, decoding_quota: 1000, skipping_quota: 100,
//!     max_type_len: 16, max_header_len: 128,
//! };
//! #[canic::canic_query(public, decode = LIMITS, decode = LIMITS)]
//! fn duplicate_limits() -> Result<(), canic::Error> { Ok(()) }
//! ```

//! # Native Wire endpoints
//!
//! Select `wire(id = 7001, request = REQUEST_LIMITS, response = RESPONSE_LIMITS)`
//! to retain ordinary Result-based access denials, or additionally select
//! `on_access_denied = "reject"` for transport rejection. An ID may name its owner
//! constant (for example `protocol::READ_ID`); calls and arithmetic are rejected.
//! Rust checks the constant is a u64 in both the codec and compiled declaration.
//! Both budget constants are `ic_wire::Limits`.
//! The explicit u64 ID binds request and response frames; request bytes are bounded
//! before copying. A handler accepts one Serde request value (or no arguments for
//! unit) and returns a Serde response. Access/startup checks and instrumentation
//! use the ordinary endpoint path. Reply encoding failure traps; synchronous
//! mutations therefore roll back. Async callbacks retain ordinary IC transaction
//! boundaries. This selection emits no Candid registration or fallback codec.
//! Native builds with both Canic's and the consuming package's `wire-schema`
//! features register these exact signatures and budgets. `endpoint::schema::snapshot(&compiled)` requires declarations read
//! from the final Wasm and refuses inventory, identity, mode or budget drift.
//! Request/reply types must implement Wire metadata; deployed Wasm acquires no
//! registry or JSON dependency. Target-specific type-layout parity, release
//! artifact binding and frontend delivery remain pending.
//!
//! Raw endpoints may select the same `wire(...)` contract instead of `payload(...)`.
//! Canic emits its compiled declaration and enforces the request byte allowance
//! before raw IO. The codec owner still enforces framing, traversal/allocation
//! and reply limits; Canic does not add another decoder or encoder.
//! Native schema producers must register the owner's actual payload projections;
//! the raw zero-argument/unit signature is never inferred as a payload schema.
//! Supplier adapters contribute those projections through the same native registry
//! as typed methods; snapshot admission validates the combined final-Wasm inventory.
//!
#[cfg(all(feature = "wire-schema", not(target_arch = "wasm32")))]
pub mod schema;

mod registration;

use ic_platform::candid::contract::ArgumentDecoder;
use ic_platform::candid::decoder::DecoderConfig;
use std::fmt;

///
/// ArgumentLimits
///
/// Bounds selected by the owning artifact with `decode = LIMITS` on endpoints
/// or `argument_limits = LIMITS` in `start!` and `start_local!`.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArgumentLimits {
    /// Maximum complete encoded argument envelope, including Candid headers.
    pub max_bytes: usize,
    /// Candid decoding-work quota (not an IC instruction count).
    pub decoding_quota: usize,
    /// Candid work quota for extra fields, arguments and skipped values.
    pub skipping_quota: usize,
    /// Maximum number of type-table entries.
    pub max_type_len: usize,
    /// Maximum Candid header byte length, bounding declared header complexity.
    pub max_header_len: usize,
}

///
/// ArgumentDecodeError
///
/// Refusal before endpoint dispatch or lifecycle restoration/participants.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgumentDecodeError {
    /// The envelope is malformed, has incompatible types, or exhausts a quota.
    InvalidCandid,

    /// The wire envelope exceeds the artifact's byte limit.
    TooLarge { actual: usize, maximum: usize },
}

impl fmt::Display for ArgumentDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "argument envelope is {actual} bytes; maximum is {maximum}"
                )
            }
            Self::InvalidCandid => formatter.write_str("invalid or over-budget Candid arguments"),
        }
    }
}

impl std::error::Error for ArgumentDecodeError {}

impl ArgumentLimits {
    const fn check_size(self, actual: usize) -> Result<(), ArgumentDecodeError> {
        if actual > self.max_bytes {
            return Err(ArgumentDecodeError::TooLarge {
                actual,
                maximum: self.max_bytes,
            });
        }
        Ok(())
    }

    /// Decode an already owned envelope with the same limits as generated entrypoints.
    ///
    /// # Errors
    /// Returns a size or Candid refusal, including exhausted decoding budgets.
    pub fn decode<'a, T: ArgumentDecoder<'a>>(
        self,
        bytes: &'a [u8],
    ) -> Result<T, ArgumentDecodeError> {
        self.check_size(bytes.len())?;
        let mut config = DecoderConfig::new();
        config
            .set_decoding_quota(self.decoding_quota)
            .set_skipping_quota(self.skipping_quota)
            .set_max_type_len(self.max_type_len)
            .set_max_header_len(self.max_header_len)
            .set_full_error_message(false);
        // IC lifecycle calls may omit the empty Candid envelope, as in the CDK.
        let bytes = if bytes.is_empty() {
            b"DIDL\x00\x00"
        } else {
            bytes
        };
        ic_platform::candid::decoder::decode_args_with_config(bytes, &config)
            .map_err(|_| ArgumentDecodeError::InvalidCandid)
    }

    /// Read platform arguments only after checking their scalar byte length.
    /// Generated adapters own refusal and dispatch; application code need not
    /// call this method or implement a second decoder.
    ///
    /// # Errors
    /// Returns a size refusal without copying, or a bounded Candid refusal.
    #[doc(hidden)]
    pub fn read<T: for<'a> ArgumentDecoder<'a>>(self) -> Result<T, ArgumentDecodeError> {
        self.check_size(ic_platform::ic0::msg_arg_data_size())?;
        let bytes = ic_platform::cdk::api::msg_arg_data();
        self.decode(&bytes)
    }
}

/// Read a managed lifecycle envelope only after admitting its byte length.
/// IC Wire owns value decoding and all structural resource admission.
#[doc(hidden)]
pub fn read_initialization<T: serde::de::DeserializeOwned>(
    limits: ic_wire::Limits,
) -> Result<T, ic_wire::Error> {
    let size = ic_platform::ic0::msg_arg_data_size();
    if size > limits.bytes {
        return Err(ic_wire::Error::Limit);
    }
    let bytes = ic_platform::cdk::api::msg_arg_data();
    ic_wire::native::decode(&bytes, limits)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests;
