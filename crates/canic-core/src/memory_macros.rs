//! Module: memory_macros
//!
//! Responsibility: adapt explicit Canic memory authorities to `ic-memory` registration.
//! Does not own: authority values, stable keys, memory IDs, or bootstrap ordering.
//! Boundary: queues declarations for ic-memory sealing and opens only committed slots.

/// Declare a stable-memory slot with an explicit authority and ABI-stable key.
///
/// Use this for every Canic-managed memory. The stable key, not crate or Rust
/// type identity, is the durable allocation identity.
#[macro_export]
macro_rules! ic_memory_key {
    (authority = CANIC_CORE_MEMORY_AUTHORITY, $($rest:tt)*) => {
        $crate::ic_memory_key!(authority = $crate::memory::CANIC_CORE_MEMORY_AUTHORITY, $($rest)*)
    };
    (authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, $($rest:tt)*) => {
        $crate::ic_memory_key!(authority = $crate::memory::CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, $($rest)*)
    };
    (authority = $authority:expr, key = $stable_key:literal, ty = $label:path, id = $id:expr $(,)?) => {{
        $crate::__reexports::ic_memory::ic_memory_declaration!(
            authority = $authority, key = $stable_key, ty = $label, id = $id,
        );
        $crate::memory::runtime::assert_memory_bootstrap_ready(stringify!($label), $id);
        $crate::__reexports::ic_memory::open_default_memory_manager_memory($stable_key, $id)
            .expect("Canic failed to open committed stable memory; bootstrap must run first and the stable key/id must match the committed declaration")
    }};
}

/// Declare a MemoryManager ID range owned by an explicit authority.
#[macro_export]
macro_rules! ic_memory_range {
    (authority = CANIC_CORE_MEMORY_AUTHORITY, $($rest:tt)*) => {
        $crate::ic_memory_range!(authority = $crate::memory::CANIC_CORE_MEMORY_AUTHORITY, $($rest)*);
    };
    (authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, $($rest:tt)*) => {
        $crate::ic_memory_range!(authority = $crate::memory::CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, $($rest)*);
    };
    (authority = $authority:expr, $($rest:tt)*) => {
        $crate::__reexports::ic_memory::ic_memory_range!(authority = $authority, $($rest)*);
    };
}

/// Register the artifact's composed memory admission before Canic bootstrap.
///
/// Supply a semantic `ic_memory::PolicyIdentity` and a synchronous callback taking
/// `&mut ic_memory::BootstrapAdmission` and returning `Result<(), E>` where `E`
/// implements `Error + Send + Sync + 'static`. Register once per artifact, not per
/// database. Grant consumer ranges separately with `ic_memory_range!`.
#[macro_export]
macro_rules! memory_bootstrap_admission {
    (identity = $identity:expr, prepare = $prepare:path $(,)?) => {
        const _: () = {
            #[ $crate::__reexports::ctor::ctor(unsafe, anonymous, crate_path = $crate::__reexports::ctor) ]
            fn __canic_register_memory_admission() {
                fn __canic_prepare_memory_admission(admission: &mut $crate::__reexports::ic_memory::BootstrapAdmission<'_>)
                    -> ::core::result::Result<(), $crate::memory::registry::MemoryRegistryError>
                {
                    $prepare(admission).map_err(|source| $crate::memory::registry::MemoryRegistryError::Admission { source: ::std::boxed::Box::new(source) })
                }
                $crate::memory::admission::register(
                    $crate::memory::admission::MemoryBootstrapAdmission::new($identity, __canic_prepare_memory_admission),
                ).expect("Canic memory admission registration failed");
            }
        };
    };
}
