//! Checked host pool builders; no constructor or registration effects.

/// Select the final host pool, including framework grants.
///
/// Additional grants
/// must name each linked component's permanent namespace. Exclusions protect
/// actual unmanaged physical memory, never component partitions.
/// `provider = name` exposes a checked, fallible builder in the invocation
/// module. Pass its result to the host composition. It has no registration effects.
///
/// Exclusion, namespace and pool validation errors are returned by the named provider.
#[macro_export]
macro_rules! memory_allocation_pool {
    (provider = $provider:ident, authorities = [$(($owner:expr, $prefix:expr)),* $(,)?], exclusions = [$( $id:expr ),* $(,)?] $(,)?) => {
        fn $provider() -> ::core::result::Result<
            $crate::__reexports::ic_memory::MemoryAllocationPool,
            $crate::__reexports::ic_memory::MemoryAllocationPoolError,
        > {
            $crate::memory::pool::framework_pool(
                vec![$($crate::__reexports::ic_memory::MemoryAuthority::new($owner, $prefix)?),*],
                vec![$($id),*],
            )
        }
    };
}
