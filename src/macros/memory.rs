// icu_memory_manager
#[macro_export]
macro_rules! icu_memory_manager {
    () => {
        thread_local! {

            ///
            /// Define MEMORY_MANAGER thread-locally for the entire scope
            ///
            pub static MEMORY_MANAGER: ::std::cell::RefCell<
                $crate::ic::structures::memory::MemoryManager<
                    $crate::ic::structures::DefaultMemoryImpl,
                >,
            > = ::std::cell::RefCell::new($crate::ic::structures::memory_manager::MemoryManager::init(
                $crate::ic::structures::DefaultMemoryImpl::default(),
            ));

            ///
            /// MEMORY_COUNTER
            ///
            pub static MEMORY_COUNTER: ::std::cell::RefCell<$crate::memory::MemoryCounter> =
                ::std::cell::RefCell::new(<$crate::memory::MemoryCounter>::init(
                    MEMORY_MANAGER.with_borrow(|this| {
                        this.get($crate::ic::structures::memory::MemoryId::new(0))
                    }
                ),
            ));
        }
    };
}

// // icu_memory_add
#[macro_export]
macro_rules! icu_memory_add {
    ($name:ident, $state:ty) => {
        thread_local! {
            pub static $name: ::std::cell::RefCell<$state> = ::std::cell::RefCell::new(
                ::icu::memory::allocate_state($state)
            );
        }
    };
}
