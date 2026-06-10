//! Protected Canic-to-Canic internal call API.
//!
//! This module owns protected endpoint descriptors for retained protected
//! internal proof verification. Fresh root ECDSA proof issuance and the
//! outbound protected-internal client surface are removed in the 0.65 normal
//! auth hard cut.

mod endpoint;

pub use endpoint::ProtectedInternalEndpoint;

#[cfg(test)]
mod tests;
