///
/// Declare a stable memory handle for a specific ID, tied to the current crate.
/// - Enqueues the registration (`defer_register`) so it will be validated later.
/// - Returns the `VirtualMemory` handle immediately so you can use it in
///   `Cell`, `BTreeMap`, etc.
///
#[macro_export]
macro_rules! icu_memory {
    ($label:ident, $id:expr) => {{
        // Enqueue this memory ID registration for later processing during
        // `force_init_all_tls()`. The crate key is always derived from
        // `CARGO_PKG_NAME`, so all memory for a crate lives under one namespace.
        $crate::memory::registry::defer_register(
            $id,
            env!("CARGO_PKG_NAME"),
            concat!(module_path!(), "::", stringify!($label)),
        );

        // Return the stable memory handle immediately so it can be wrapped in
        // higher-level data structures (BTreeMap, Cell, Vec, etc.).
        $crate::memory::MEMORY_MANAGER
            .with_borrow_mut(|mgr| mgr.get($crate::cdk::structures::memory::MemoryId::new($id)))
    }};
}

///
/// Reserve a contiguous block of memory IDs for the current crate.
/// - Enqueues the reservation (`defer_reserve_range`) so it will be validated later.
/// - Uses the crate name (`CARGO_PKG_NAME`) as the registry key, matching `icu_memory!`.
///
#[macro_export]
macro_rules! icu_memory_range {
    ($start:expr, $end:expr) => {{
        // Enqueue this range reservation. The actual check/insert happens in
        // `force_init_all_tls()`. This guarantees the reservation is made
        // before any memory IDs from this range are registered.
        $crate::memory::registry::defer_reserve_range(env!("CARGO_PKG_NAME"), $start, $end);
    }};
}
