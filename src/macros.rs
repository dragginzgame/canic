#[macro_export]
macro_rules! impl_storable_bounded {
    ($ident:ident, $max_size:expr, $is_fixed_size:expr) => {
        impl ::ic_stable_structures::storable::Storable for $ident {
            fn to_bytes(&self) -> ::std::borrow::Cow<[u8]> {
                ::std::borrow::Cow::Owned(::icu::serialize::serialize(self).unwrap())
            }

            fn from_bytes(bytes: ::std::borrow::Cow<[u8]>) -> Self {
                $crate::serialize::deserialize(&bytes).unwrap()
            }

            const BOUND: ::ic_stable_structures::storable::Bound =
                ::ic_stable_structures::storable::Bound::Bounded {
                    max_size: $max_size,
                    is_fixed_size: $is_fixed_size,
                };
        }
    };
}

#[macro_export]
macro_rules! impl_storable_unbounded {
    ($ident:ident) => {
        impl ::ic_stable_structures::storable::Storable for $ident {
            fn to_bytes(&self) -> ::std::borrow::Cow<[u8]> {
                ::std::borrow::Cow::Owned($crate::serialize::serialize(self).unwrap())
            }

            fn from_bytes(bytes: ::std::borrow::Cow<[u8]>) -> Self {
                $crate::serialize::deserialize(&bytes).unwrap()
            }

            const BOUND: ::ic_stable_structures::storable::Bound =
                ::ic_stable_structures::storable::Bound::Unbounded;
        }
    };
}

// memory_manager
#[macro_export]
macro_rules! memory_manager {
    () => {
        thread_local! {

            ///
            /// Define MEMORY_MANAGER thread-locally for the entire scope
            ///
            pub static MEMORY_MANAGER: ::std::cell::RefCell<
                ::ic_stable_structures::memory_manager::MemoryManager<
                    ::ic_stable_structures::DefaultMemoryImpl,
                >,
            > = ::std::cell::RefCell::new(::ic_stable_structures::memory_manager::MemoryManager::init(
                ::ic_stable_structures::DefaultMemoryImpl::default(),
            ));

        }
    };
}

// perf
#[macro_export]
macro_rules! perf {
    () => {
        ::defer::defer!(::mimic::log!(
            Log::Perf,
            "api call used {} instructions ({})",
            $crate::ic::api::performance_counter(1),
            module_path!()
        ));
    };
}
