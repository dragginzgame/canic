//! Explicit host namespace composition without constructors or registration effects.

/// Build checked application and framework namespace grants in the consuming host.
#[macro_export]
macro_rules! memory_allocation_grants {
    (provider = $provider:ident, authorities = [$(($owner:expr, $prefix:expr)),* $(,)?] $(,)?) => {
        fn $provider() -> ::core::result::Result<
            $crate::__reexports::ic_storage::AllocationGrants,
            $crate::__reexports::ic_storage::AdmissionError,
        > {
            $crate::memory::grants::framework_grants(
                vec![$($crate::__reexports::ic_storage::MemoryAuthority::new($owner, $prefix)?),*],
            )
        }
    };
}
