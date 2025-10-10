/// Declare a stable-memory slot backed by the Canic memory registry.
///
/// The macro enqueues a registration for later validation (during
/// `force_init_all_tls`) and immediately returns the
/// [`VirtualMemory`](crate::cdk::structures::memory::VirtualMemory)
/// handle so callers can wrap it in `Cell`, `BTreeMap`, and other structures.
/// Memory IDs are automatically namespaced per crate via `CARGO_PKG_NAME`.
#[macro_export]
macro_rules! ic_memory {
    ($label:ident, $id:expr) => {{
        // Force the compiler to resolve the type. This causes a compile-time error
        // if `$label` does not exist or is not a valid local type.
        let _type_check: Option<$label> = None;

        // Enqueue this memory ID registration for deferred validation.
        $crate::memory::registry::defer_register(
            $id,
            env!("CARGO_PKG_NAME"),
            concat!(module_path!(), "::", stringify!($label)),
        );

        // Return the stable memory handle immediately for further wrapping.
        $crate::memory::MEMORY_MANAGER
            .with_borrow_mut(|mgr| mgr.get($crate::cdk::structures::memory::MemoryId::new($id)))
    }};
}

/// Reserve a contiguous block of stable-memory IDs for the current crate.
///
/// Stores the range request for validation during eager TLS initialization.
/// The reservation shares the crate namespace used by [`macro@ic_memory`].
#[macro_export]
macro_rules! ic_memory_range {
    ($start:expr, $end:expr) => {{
        // Enqueue this range reservation. The actual check/insert happens in
        // `force_init_all_tls()`. This guarantees the reservation is made
        // before any memory IDs from this range are registered.
        $crate::memory::registry::defer_reserve_range(env!("CARGO_PKG_NAME"), $start, $end);
    }};
}
