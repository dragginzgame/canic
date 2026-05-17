/// Declare a stable-memory slot backed by the Canic memory registry.
///
/// The macro registers a declaration descriptor for lifecycle bootstrap
/// validation during process startup. Evaluating the macro expression returns the
/// [`VirtualMemory`](crate::cdk::structures::memory::VirtualMemory)
/// handle only after bootstrap has validated the sealed declaration snapshot.
/// Memory IDs are automatically namespaced per crate via `CARGO_PKG_NAME`.
#[macro_export]
macro_rules! ic_memory {
    ($label:path, $id:expr) => {{
        const _: () = {
            #[ $crate::__reexports::ctor::ctor(unsafe, anonymous, crate_path = $crate::__reexports::ctor) ]
            fn __canic_declare_memory_slot() {
                $crate::registry::defer_register($id, env!("CARGO_PKG_NAME"), stringify!($label))
                    .expect("memory id declaration validation failed");
            }
        };

        $crate::runtime::assert_memory_bootstrap_ready(stringify!($label), $id);

        // Force the compiler to resolve the type. This causes a compile-time error
        // if `$label` does not exist or is not a valid local type.
        let _type_check: Option<$label> = None;

        // Return the stable memory handle for further wrapping after bootstrap.
        $crate::manager::MEMORY_MANAGER
            .with_borrow_mut(|mgr| mgr.get($crate::cdk::structures::memory::MemoryId::new($id)))
    }};
}

/// Declare a stable-memory slot with an explicit ABI-stable key.
///
/// Use this for every Canic-managed memory. The key, not the crate name or Rust
/// type path, is the durable stable-memory identity.
#[macro_export]
macro_rules! ic_memory_key {
    ($stable_key:literal, $label:path, $id:expr) => {{
        const _: () = {
            #[ $crate::__reexports::ctor::ctor(unsafe, anonymous, crate_path = $crate::__reexports::ctor) ]
            fn __canic_declare_memory_slot() {
                $crate::registry::defer_register_with_key(
                    $id,
                    env!("CARGO_PKG_NAME"),
                    stringify!($label),
                    $stable_key,
                )
                .expect("memory id declaration validation failed");
            }
        };

        $crate::runtime::assert_memory_bootstrap_ready(stringify!($label), $id);

        // Force the compiler to resolve the type. This causes a compile-time error
        // if `$label` does not exist or is not a valid local type.
        let _type_check: Option<$label> = None;

        $crate::manager::MEMORY_MANAGER
            .with_borrow_mut(|mgr| mgr.get($crate::cdk::structures::memory::MemoryId::new($id)))
    }};
}

/// Reserve a contiguous block of stable-memory IDs for the current crate.
///
/// Stores the range request for validation during memory bootstrap. The
/// reservation shares the crate namespace used by [`macro@ic_memory`].
#[macro_export]
macro_rules! ic_memory_range {
    ($start:expr, $end:expr) => {{
        // Enqueue this range reservation. The actual check/insert happens in
        // `init_eager_tls()`. This guarantees the reservation is made
        // before any memory IDs from this range are registered.
        $crate::registry::defer_reserve_range(env!("CARGO_PKG_NAME"), $start, $end)
            .expect("memory range reservation validation failed");
        $crate::runtime::registry::MemoryRegistryRuntime::commit_pending_if_initialized()
            .expect("late memory range registration commit failed");
    }};
}
