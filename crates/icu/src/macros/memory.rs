#[macro_export]
macro_rules! icu_register_memory {
    ($ty:ty, $id:expr, $init:expr) => {{
        let path = stringify!($ty).to_string();

        // check the registry with logging
        $crate::memory::MEMORY_REGISTRY.with_borrow_mut(|reg| {
            let result = reg.register(
                $id,
                $crate::memory::registry::RegistryEntry { path: path.clone() },
            );

            if let Err(ref err) = result {
                $crate::log!(
                    $crate::Log::Error,
                    "❌ icu_register_memory failed for {} @ {}: {}",
                    path,
                    $id,
                    err
                );
            } else {
                $crate::log!(
                    $crate::Log::Info,
                    "✅ icu_register_memory registered {} @ {}",
                    path,
                    $id
                );
            }

            result.unwrap()
        });

        // acquire memory_id
        let mem = $crate::memory::MEMORY_MANAGER
            .with_borrow_mut(|mgr| mgr.get($crate::ic::structures::memory::MemoryId::new($id)));

        // init
        $init(mem)
    }};
}
