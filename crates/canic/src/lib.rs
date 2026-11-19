//! Small contracts and explicitly selected actor adapters.
//! No host dependencies or management surface enters a Wasm consumer.

/// Optional caller-bound Auth endpoints; lifecycle remains explicit.
#[cfg(feature = "auth")]
pub mod auth;

#[cfg(any(feature = "blob", feature = "db"))]
mod frame;

/// Optional Blob endpoint composition; no implicit lifecycle or service activation.
#[cfg(feature = "blob")]
pub mod blob;

/// Optional application-owned typed database endpoints.
#[cfg(any(feature = "db", feature = "db-store"))]
pub mod db;

/// One bounded management entropy request, with no local generator or retry.
#[cfg(feature = "entropy")]
pub mod entropy;

/// Explicit lifecycle hooks independent of storage and Auth.
#[cfg(feature = "lifecycle")]
pub mod lifecycle;

#[cfg(feature = "method")]
pub mod method;

/// Passive operational reports; the optional `metrics` feature enables endpoint mounting.
pub mod metrics;

/// Application-owned readiness; the optional feature enables an explicit replicated mount.
pub mod readiness;

#[cfg(feature = "recovery")]
pub mod recovery;

#[cfg(feature = "storage")]
pub mod storage;

/// Optional runtime shard assignment and lookup for application hubs.
#[cfg(feature = "sharding")]
pub mod sharding;

/// Fleet-approved shard installation context.
#[cfg(feature = "topology")]
pub mod topology;

#[cfg(feature = "method")]
pub use canic_macros::{query, update};

/// Emit a local build-report marker without adding an IC export.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_report_mount {
    ($section:literal) => {
        #[cfg(target_arch = "wasm32")]
        const _: () = {
            #[used]
            #[allow(unsafe_code, reason = "Wasm custom section for local build reporting")]
            #[unsafe(link_section = $section)]
            static MODULE: u8 = 1;
        };
    };
}
