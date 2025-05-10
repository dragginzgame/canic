// memory_manager
#[macro_export]
macro_rules! memory_manager {
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

        }
    };
}

// state_init
#[macro_export]
macro_rules! state_init {
    ($name:ident, $state:ty, $memory_id:expr) => {
        const _: () = {
            if !($memory_id >= 10 && $memory_id <= 19) {
                panic!("state_init! macro error: $memory_id must be between 10 and 19");
            }
        };

        thread_local! {
            pub static $name: ::std::cell::RefCell<$state> = ::std::cell::RefCell::new(<$state>::init(
                MEMORY_MANAGER.with_borrow(|this| {
                    this.get(::icu::ic::structures::memory::MemoryId::new($memory_id))
                }),
            ));
        }
    };
}
