#[macro_export]
macro_rules! icu_register_memory {
    ($id:expr) => {{
        let path = concat!(module_path!(), "::", line!()).to_string();

        // check the registry with logging
        let result = $crate::memory::MemoryRegistry::register(
            $id,
            $crate::memory::memory_registry::MemoryRegistryEntry { path: path.clone() },
        );

        if let Err(ref err) = result {
            $crate::log!(
                $crate::Log::Error,
                "❌ icu_register_memory failed for {} @ {}: {}",
                path,
                $id,
                err
            );
        }

        result.unwrap();

        // acquire memory_id → explicitly return VirtualMemory<DefaultMemoryImpl>
        $crate::memory::MEMORY_MANAGER
            .with_borrow_mut(|mgr| mgr.get($crate::cdk::structures::memory::MemoryId::new($id)))
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

///
/// CANDID VERSIONS
///

#[macro_export]
macro_rules! impl_storable_candid_bounded {
    ($ident:ident, $max_size:expr, $is_fixed_size:expr) => {
        impl $crate::cdk::structures::storable::Storable for $ident {
            const BOUND: $crate::cdk::structures::storable::Bound =
                $crate::cdk::structures::storable::Bound::Bounded {
                    max_size: $max_size,
                    is_fixed_size: $is_fixed_size,
                };

            fn to_bytes(&self) -> ::std::borrow::Cow<'_, [u8]> {
                use $crate::cdk::candid::Encode;
                let bytes = Encode!(self).expect("Candid encode failed");
                ::std::borrow::Cow::Owned(bytes)
            }

            fn into_bytes(self) -> Vec<u8> {
                use $crate::cdk::candid::Encode;
                Encode!(&self).expect("Candid encode failed")
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                use $crate::cdk::candid::Decode;
                Decode!(&bytes, $ident).expect("Candid decode failed")
            }
        }
    };
}

#[macro_export]
macro_rules! impl_storable_candid_unbounded {
    ($ident:ident) => {
        impl $crate::cdk::structures::storable::Storable for $ident {
            const BOUND: $crate::cdk::structures::storable::Bound =
                $crate::cdk::structures::storable::Bound::Unbounded;

            fn to_bytes(&self) -> ::std::borrow::Cow<'_, [u8]> {
                use $crate::cdk::candid::Encode;

                let bytes = Encode!(self).expect("Candid encode failed");
                ::std::borrow::Cow::Owned(bytes)
            }

            fn into_bytes(self) -> Vec<u8> {
                use $crate::cdk::candid::Encode;

                Encode!(&self).expect("Candid encode failed")
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                use $crate::cdk::candid::Decode;

                Decode!(&bytes, $ident).expect("Candid decode failed")
            }
        }
    };
}
