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
    (authority = $authority:expr, key = $stable_key:literal $(,)?) => {{
        $crate::__reexports::ic_memory::ic_memory_declaration!(authority = $authority, key = $stable_key);
        $crate::memory::runtime::assert_memory_bootstrap_ready($stable_key);
        $crate::__reexports::ic_memory::open_default_memory_manager_memory($stable_key)
            .expect("ic-memory committed key binding must exist before opening stable memory")
    }};
}

/// Select the final host pool, including framework grants.
///
/// Additional grants
/// must name each linked component's permanent namespace. Exclusions protect
/// actual unmanaged physical memory, never component partitions.
/// Add `provider = name` to expose a checked, fallible builder in the invocation
/// module. Registration calls that same builder; calling it later does not
/// register, seal or bootstrap a runtime.
///
/// # Panics
/// Invalid authored exclusion bounds panic, as with registration-only declarations.
/// Namespace and pool validation errors are returned by the named provider.
#[macro_export]
macro_rules! memory_allocation_pool {
    (provider = $provider:ident, authorities = [$(($owner:expr, $prefix:expr)),* $(,)?], exclusions = [$(($start:expr, $end:expr)),* $(,)?] $(,)?) => {
        fn $provider() -> ::core::result::Result<
            $crate::__reexports::ic_memory::MemoryAllocationPool,
            $crate::__reexports::ic_memory::MemoryAllocationPoolError,
        > {
            $crate::memory::pool::framework_pool(
                vec![$($crate::__reexports::ic_memory::MemoryAuthority::new($owner, $prefix)?),*],
                vec![$($crate::__reexports::ic_memory::MemoryManagerIdRange::new($start, $end).expect("valid unmanaged exclusion")),*],
            )
        }
        const _: () = {
            #[$crate::__reexports::ctor::ctor(unsafe, anonymous, crate_path = $crate::__reexports::ctor)]
            fn __canic_register_memory_pool() {
                let pool = $provider().expect("valid host memory grants and unmanaged exclusions");
                $crate::memory::pool::register(pool).expect("one host memory pool before bootstrap");
            }
        };
    };
    (authorities = [$(($owner:expr, $prefix:expr)),* $(,)?], exclusions = [$(($start:expr, $end:expr)),* $(,)?] $(,)?) => {
        const _: () = {
            $crate::memory_allocation_pool!(
                provider = __canic_memory_pool,
                authorities = [$(($owner, $prefix)),*],
                exclusions = [$(($start, $end)),*],
            );
        };
    };
}

/// Register the artifact's composed memory admission before Canic bootstrap.
///
/// Supply a semantic `ic_memory::PolicyIdentity` and a synchronous callback taking
/// `&mut ic_memory::BootstrapAdmission` and returning `Result<(), E>` where `E`
/// implements `Error + Send + Sync + 'static`. Register once per artifact, not per
/// database. Select host namespace grants separately with `memory_allocation_pool!`.
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
