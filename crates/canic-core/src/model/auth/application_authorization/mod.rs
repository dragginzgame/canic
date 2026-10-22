//! Module: model::auth::application_authorization
//!
//! Responsibility: own canonical local-application authorization values and bounds.
//! Does not own: stable encoding, state access, proof verification, or policy decisions.
//! Boundary: ops constructs these values; pure policy inspects their invariants.

#[cfg(any(test, feature = "auth-local-application-authorization"))]
mod authority;
mod scope;

#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub use authority::LocalApplicationAuthorityBinding;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub use authority::LocalApplicationAuthoritySnapshot;
pub use scope::{
    ApplicationScope, ApplicationScopeError, ApplicationScopeRef, CanonicalApplicationScopes,
};

#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub const MAX_ACTIVE_APPLICATION_SESSIONS: usize = 2_048;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub const MAX_APPLICATION_REPLAY_RECORDS: usize = 4_096;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub const MAX_APPLICATION_REPLAY_RECORDS_PER_CALLER: usize = 256;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub const MAX_APPLICATION_ADMITTED_LOGINS: usize = 4_096;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub const MAX_APPLICATION_UNKNOWN_LOGOUTS: usize = 256;
pub const MAX_APPLICATION_SESSION_SCOPE_BYTES: usize = 1_024;
pub const MAX_APPLICATION_SESSION_SCOPES: usize = 16;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub const MAX_APPLICATION_SESSION_RECORD_BYTES: usize = 4_096;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub const MAX_APPLICATION_SESSION_STABLE_BYTES: usize = 8 * 1024 * 1024;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub const MAX_APPLICATION_SESSION_INDEX_BYTES: usize = 4 * 1024 * 1024;
#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub const MAX_APPLICATION_SESSION_CLEANUP_REMOVALS: usize = 128;
#[cfg(any(
    not(target_arch = "wasm32"),
    test,
    feature = "auth-local-application-authorization"
))]
pub const MAX_LOCAL_APPLICATION_SESSION_TTL_NS: u64 = 1_800_000_000_000;
pub const MAX_VERIFIED_APPLICATION_SCOPES: usize = 32;
