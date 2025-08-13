// icu_endpoints_root
#[macro_export]
macro_rules! icu_endpoints_root {
    () => {
        #[::icu::ic::update]
        async fn icu_canister_status(
            pid: Principal,
        ) -> Result<::icu::ic::mgmt::CanisterStatusResult, ::icu::Error> {
            ::icu::interface::ic::canister_status(pid).await
        }

        #[::icu::ic::query]
        fn icu_memory_registry() -> ::icu::memory::MemoryRegistryData {
            $crate::memory::MemoryRegistry::export()
        }

        #[::icu::ic::query]
        fn icu_canister_registry() -> ::icu::canister::CanisterRegistryData {
            $crate::canister::CanisterRegistry::export()
        }
    };
}
