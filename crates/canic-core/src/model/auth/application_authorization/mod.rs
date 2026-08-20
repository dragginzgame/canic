//! Module: model::auth::application_authorization
//!
//! Responsibility: own canonical local-application authorization values and bounds.
//! Does not own: stable encoding, state access, proof verification, or policy decisions.
//! Boundary: ops constructs these values; pure policy inspects their invariants.

mod authority;
mod scope;

pub use authority::{
    ApplicationAuthorityModelError, LocalApplicationAuthorityBinding,
    LocalApplicationAuthoritySnapshot, LocalApplicationReplay, LocalApplicationSession,
    VerifiedApplicationAuthority,
};
pub use scope::{
    ApplicationScope, ApplicationScopeError, ApplicationScopeRef, CanonicalApplicationScopes,
};

pub const MAX_ACTIVE_APPLICATION_SESSIONS: usize = 2_048;
pub const MAX_ACTIVE_APPLICATION_SESSIONS_PER_SUBJECT: usize = 128;
pub const MAX_APPLICATION_SESSION_AUDIT_PAGE_ENTRIES: usize = 128;
pub const MAX_APPLICATION_PROOF_LIFETIME_NS: u64 = 60_000_000_000;
pub const MAX_APPLICATION_REPLAY_RECORDS: usize = 4_096;
pub const MAX_APPLICATION_REPLAY_RECORDS_PER_SUBJECT: usize = 256;
pub const MAX_APPLICATION_SCOPE_BYTES: usize = 64;
pub const MAX_APPLICATION_SESSION_SCOPE_BYTES: usize = 1_024;
pub const MAX_APPLICATION_SESSION_SCOPES: usize = 16;
pub const MAX_APPLICATION_SESSION_RECORD_BYTES: usize = 2_048;
pub const MAX_APPLICATION_SESSION_STABLE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_APPLICATION_SESSION_INDEX_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_APPLICATION_SESSION_CLEANUP_REMOVALS: usize = 128;
pub const MAX_LOCAL_APPLICATION_SESSION_TTL_NS: u64 = 1_800_000_000_000;
pub const MAX_VERIFIED_APPLICATION_SCOPES: usize = 32;
