//! Execution ownership whose lifetime is one running canister module.
//!
//! Durable effect intent remains in stable storage across execution cancellation.

pub mod canister_pool;
pub mod capacity_import;
