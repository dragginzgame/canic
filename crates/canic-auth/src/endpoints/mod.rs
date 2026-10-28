//! Endpoint mounting boundary for the unfinished portable Auth integration.
//!
//! No endpoint, lifecycle hook or state allocation is emitted until the owning
//! production integration is implemented and qualified.

/// Refuse mounting while portable authentication integration is incomplete.
///
/// The crate README records the required integration boundaries and links the
/// owning acceptance issue. Arguments are intentionally unspecified until the
/// protected issuer/receiver composition is implemented.
///
/// ```compile_fail
/// canic_auth::mount!();
/// ```
#[macro_export]
macro_rules! mount {
    ($($configuration:tt)*) => {
        compile_error!(
            "canic-auth mounting is unavailable: implement and qualify protected enrollment, portable endpoints, atomic storage and lifecycle composition; see crates/canic-auth/README.md and Canic #491"
        );
    };
}
