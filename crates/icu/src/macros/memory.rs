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

// impl_storable_bounded
#[macro_export]
macro_rules! impl_storable_bounded {
    ($ident:ident, $max_size:expr, $is_fixed_size:expr) => {
        impl $crate::ic::structures::storable::Storable for $ident {
            const BOUND: $crate::ic::structures::storable::Bound =
                $crate::ic::structures::storable::Bound::Bounded {
                    max_size: $max_size,
                    is_fixed_size: $is_fixed_size,
                };

            fn to_bytes(&self) -> ::std::borrow::Cow<[u8]> {
                ::std::borrow::Cow::Owned(::icu::serialize::serialize(self).unwrap())
            }

            fn from_bytes(bytes: ::std::borrow::Cow<[u8]>) -> Self {
                $crate::serialize::deserialize(&bytes).unwrap()
            }
        }
    };
}

// impl_storable_unbounded
#[macro_export]
macro_rules! impl_storable_unbounded {
    ($ident:ident) => {
        impl $crate::ic::structures::storable::Storable for $ident {
            const BOUND: $crate::ic::structures::storable::Bound =
                $crate::ic::structures::storable::Bound::Unbounded;

            fn to_bytes(&self) -> ::std::borrow::Cow<[u8]> {
                ::std::borrow::Cow::Owned($crate::serialize::serialize(self).unwrap())
            }

            fn from_bytes(bytes: ::std::borrow::Cow<[u8]>) -> Self {
                $crate::serialize::deserialize(&bytes).unwrap()
            }
        }
    };
}
