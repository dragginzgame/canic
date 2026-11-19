//! Application-owned discovery; worker storage and caller authentication stay at the worker.
mod canister;

#[cfg(feature = "named-profile-witness")]
mod named_profile;
