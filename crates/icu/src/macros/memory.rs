#[macro_export]
macro_rules! thread_local_register {
    // match: vis static NAME: TYPE = INIT;
    ($vis:vis static $name:ident : $ty:ty = $init:expr;) => {
        thread_local! {
            $vis static $name: $ty = $init;
        }

        // Each declaration registers itself into TLS_REGISTRARS
        #[$crate::export::ctor::ctor(anonymous, crate_path = $crate::export::ctor)]
        fn __ctor() {
            $crate::memory::registry::TLS_REGISTRARS.with(|v| {
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

        // Compile-time guards
        #[cfg(icu_internal)]
        const _: () = assert!(
            ID >= $crate::ICU_MEMORY_MIN && ID <= $crate::ICU_MEMORY_MAX,
            "ICU IDs must be within ICU_MEMORY_RANGE"
        );

        #[cfg(not(icu_internal))]
        const _: () = assert!(
            ID < $crate::ICU_MEMORY_MIN || ID > $crate::ICU_MEMORY_MAX,
            "Non-ICU crates must not use ICU_MEMORY_RANGE"
        );

        // Enqueue this registration for later
        $crate::memory::registry::PENDING_REGISTRATIONS.with(|q| {
            q.borrow_mut().push((
                ID,
                env!("CARGO_PKG_NAME"),
                concat!(module_path!(), "::", line!()),
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
