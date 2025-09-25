#[macro_export]
macro_rules! thread_local_memory {
    // match: vis static NAME: TYPE = INIT;
    ($vis:vis static $name:ident : $ty:ty = $init:expr;) => {
        thread_local! {
            $vis static $name: $ty = $init;
        }

        // Each declaration registers itself into TLS_INITIALIZERS
        #[$crate::export::ctor::ctor(anonymous, crate_path = $crate::export::ctor)]
        fn __ctor() {
            $crate::memory::registry::TLS_INITIALIZERS.with(|v| {
                v.borrow_mut().push(|| {
                    $name.with(|_| {});
                });
            });
        }
    };
}

#[macro_export]
macro_rules! icu_memory {
    ($label:ident, $id:expr) => {{
        const ID: u8 = $id;

        #[cfg(not(icu_internal))]
        const _: () = assert!(
            ID < $crate::ICU_MEMORY_MIN || ID > $crate::ICU_MEMORY_MAX,
            "Non-ICU crate memory id {} within ICU_MEMORY_RANGE ({} - {})",
            $crate::ICU_MEMORY_MIN,
            $crate::ICU_MEMORY_MAX,
            ID
        );

        // Enqueue this registration for later
        $crate::memory::registry::TLS_PENDING_REGISTRATIONS.with(|q| {
            q.borrow_mut().push((
                ID,
                env!("CARGO_PKG_NAME"),
                concat!(module_path!(), "::", stringify!($label)),
            ));
        });

        // Return the stable memory handle immediately
        $crate::memory::MEMORY_MANAGER
            .with_borrow_mut(|mgr| mgr.get($crate::cdk::structures::memory::MemoryId::new(ID)))
    }};
}

///
/// MiniCBOR Versions
/// (much faster, doesn't support u128)
///

// impl_storable_bounded
#[macro_export]
macro_rules! impl_storable_bounded {
    ($ident:ident, $max_size:expr, $is_fixed_size:expr) => {
        impl $crate::cdk::structures::storable::Storable for $ident {
            const BOUND: $crate::cdk::structures::storable::Bound =
                $crate::cdk::structures::storable::Bound::Bounded {
                    max_size: $max_size,
                    is_fixed_size: $is_fixed_size,
                };

            fn to_bytes(&self) -> ::std::borrow::Cow<'_, [u8]> {
                ::std::borrow::Cow::Owned($crate::utils::cbor::serialize(self).unwrap())
            }

            fn into_bytes(self) -> Vec<u8> {
                $crate::utils::cbor::serialize(&self).unwrap()
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                $crate::utils::cbor::deserialize(&bytes).unwrap()
            }
        }
    };
}

// impl_storable_unbounded
#[macro_export]
macro_rules! impl_storable_unbounded {
    ($ident:ident) => {
        impl $crate::cdk::structures::storable::Storable for $ident {
            const BOUND: $crate::cdk::structures::storable::Bound =
                $crate::cdk::structures::storable::Bound::Unbounded;

            fn to_bytes(&self) -> ::std::borrow::Cow<'_, [u8]> {
                ::std::borrow::Cow::Owned($crate::utils::cbor::serialize(self).unwrap())
            }

            fn into_bytes(self) -> Vec<u8> {
                $crate::utils::cbor::serialize(&self).unwrap()
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                $crate::utils::cbor::deserialize(&bytes).unwrap()
            }
        }
    };
}
