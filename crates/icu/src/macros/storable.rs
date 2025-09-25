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
