#[macro_export]
macro_rules! icu_register_memory {
    ($ty:ty, $id:expr, $init:expr) => {{
        let path = stringify!($ty).to_string();

        // check the registry with logging
        let id = $crate::memory::MEMORY_REGISTRY.with_borrow_mut(|reg| {
            let result = reg.register(
                $id,
                $crate::memory::registry::RegistryEntry { path: path.clone() },
            );

            match result {
                Ok(id) => {
                    $crate::log!(
                        $crate::Log::Info,
                        "✅ icu_register_memory registered {} @ {}",
                        path,
                        id
                    );
                    id
                },
                Err(ref err) => {
                    $crate::log!(
                        $crate::Log::Error,
                        "❌ icu_register_memory failed for {} @ {}: {}",
                        path,
                        $id,
                        err
                    );
                    panic!("Failed to register memory: {}", err);
                }
            }
        });

        // acquire memory_id
        let mem = $crate::memory::MEMORY_MANAGER
            .with_borrow_mut(|mgr| mgr.get($crate::ic::structures::memory::MemoryId::new(id)));

        // init
        $init(mem)
    }};
    
    // Version with auto-incremented ID
    ($ty:ty, $init:expr) => {{
        let path = stringify!($ty).to_string();

        // check the registry with logging and auto-assign ID
        let id = $crate::memory::MEMORY_REGISTRY.with_borrow_mut(|reg| {
            let result = reg.register_auto(
                $crate::memory::registry::RegistryEntry { path: path.clone() },
            );

            match result {
                Ok(id) => {
                    $crate::log!(
                        $crate::Log::Info,
                        "✅ icu_register_memory auto-registered {} @ {}",
                        path,
                        id
                    );
                    id
                },
                Err(ref err) => {
                    $crate::log!(
                        $crate::Log::Error,
                        "❌ icu_register_memory failed to auto-register {}: {}",
                        path,
                        err
                    );
                    panic!("Failed to auto-register memory: {}", err);
                }
            }
        });

        // acquire memory_id
        let mem = $crate::memory::MEMORY_MANAGER
            .with_borrow_mut(|mgr| mgr.get($crate::ic::structures::memory::MemoryId::new(id)));

        // init
        $init(mem)
    }};
}
