//! Small contracts and explicitly selected actor adapters.
//! No host dependencies or management surface enters a Wasm consumer.

// Endpoint attributes use the same absolute crate path in Canic and in applications.
extern crate self as canic;

#[cfg(any(
    feature = "auth",
    feature = "blob",
    feature = "db",
    feature = "lifecycle",
    feature = "method",
    feature = "readiness",
    feature = "sharding"
))]
#[doc(hidden)]
pub mod __private {
    pub use ic_platform_runtime as runtime;
}

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

/// Fresh platform requests and an optional supplier with shared diagnostics.
#[cfg(feature = "raw-rand")]
pub mod entropy;

/// Opt-in Jobs runtime. The application owns job binding and timer initialization.
#[cfg(feature = "jobs")]
pub use ic_jobs as jobs;

/// Explicit lifecycle hooks independent of storage and Auth.
#[cfg(feature = "lifecycle")]
pub mod lifecycle;

/// Bounded operational diagnostics owned by the hosting canister.
#[cfg(feature = "logging")]
pub mod logging;

#[cfg(any(feature = "method", feature = "sharding"))]
pub mod method;

/// Passive operational reports. The optional `metrics` feature exports one guarded reader.
pub mod metrics;

/// Application-owned readiness; the optional feature enables an explicit replicated mount.
pub mod readiness;

#[cfg(feature = "recovery")]
pub mod recovery;

/// Actor composition for Auth-enrolled stateful hubs and shards.
#[cfg(feature = "stateful-role")]
pub mod role;

#[cfg(feature = "storage")]
pub mod storage;

/// Optional runtime shard assignment and lookup for application hubs.
#[cfg(feature = "sharding")]
pub mod sharding;

/// Fleet-approved shard installation context.
#[cfg(feature = "topology")]
pub mod topology;

#[cfg(feature = "method")]
pub use canic_macros::{method_descriptor, query, role_descriptor, update};

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

/// Opt-in host inventory. Normal canister builds omit the collectors.
#[cfg(all(feature = "inspection", not(target_arch = "wasm32")))]
pub mod inspection;

#[cfg(all(feature = "inspection", not(target_arch = "wasm32")))]
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_inspect {
    (@root wire, $ty:ty) => {
        |schemas| {
            use $crate::inspection::Describe as _;
            (&&$crate::inspection::Type::<$ty>(::core::marker::PhantomData)).describe(schemas)
        }
    };
    (@root auth, $ty:ty) => {
        $crate::inspection::auth_root::<$ty>
    };
    ($source:ident, $name:expr, $mode:ident, $id:expr, $input:ty, $output:ty) => {
        $crate::inspection::inventory::submit! {
            $crate::inspection::Declaration {
                module: module_path!(), name: $name, mode: stringify!($mode), id: $id,
                request: $crate::__canic_inspect!(@root $source, $input),
                reply: $crate::__canic_inspect!(@root $source, $output),
            }
        }
    };
}

#[cfg(not(all(feature = "inspection", not(target_arch = "wasm32"))))]
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_inspect {
    ($($tokens:tt)*) => {};
}

/// Mount a built-in framed endpoint and describe the same signature to an opted-in host.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_builtin {
    ($source:ident, $id:expr, $platform:ident :: $mode:ident, $name:literal, ($($options:tt)*),
     [$($async:ident)?] fn $handler:ident($arg:tt : $input:ty $(,)?) -> $output:ty $body:block) => {
        $crate::__canic_inspect!($source, $name, $mode, $id, $input, $output);
        #[$platform::$mode(name = $name, $($options)*)]
        $($async)? fn $handler($arg: $input) -> $output $body
    };
}
