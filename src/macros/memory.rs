#[macro_export]
macro_rules! icu_register_memory {
    ($ty:ty, $id:expr, $init:expr) => {{
        let path = stringify!($ty).to_string();

        // check the registry
        $crate::memory::MEMORY_REGISTRY.with_borrow_mut(|reg| {
            reg.register($id, $crate::memory::registry::RegistryEntry { path })
                .unwrap();
        });

        // acquire memory_id
        let mem = $crate::memory::MEMORY_MANAGER
            .with_borrow_mut(|mgr| mgr.get($crate::ic::structures::memory::MemoryId::new($id)));

        // init
        $init(mem)
    }};
}
