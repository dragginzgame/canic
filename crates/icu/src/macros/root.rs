// icu_endpoints_root
#[macro_export]
macro_rules! icu_endpoints_root {
    () => {
        // icu_memory_registry
        #[::icu::ic::query]
        fn icu_memory_registry() -> ::icu::memory::MemoryRegistryData {
            $crate::memory::MemoryRegistry::export()
        }

        // icu_canister_registry
        #[::icu::ic::query]
        fn icu_canister_registry() -> ::icu::canister::CanisterRegistryData {
            $crate::canister::CanisterRegistry::export()
        }
    };
}
