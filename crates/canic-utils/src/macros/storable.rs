//! Derive `ic-stable-structures` storage traits using Mini CBOR serialization.
//!
//! The helper macros below wire types into the Canister Development Kit's
//! `Storable` trait by delegating to Canic's MiniCBOR helpers. The bounded
//! variant requires callers to specify a maximum serialized size and whether
//! the size is fixed; the unbounded variant is suitable for archival data
//! that can grow, at the cost of larger metadata cells.

/// Implement [`Storable`](ic_stable_structures::storable::Storable) with a
/// bounded size guarantee.
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
                ::std::borrow::Cow::Owned($crate::serialize::serialize(self).unwrap())
            }

            fn into_bytes(self) -> Vec<u8> {
                $crate::serialize::serialize(&self).unwrap()
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                $crate::serialize::deserialize(&bytes).unwrap()
            }
        }
    };
}

/// Implement [`Storable`](ic_stable_structures::storable::Storable) without a
/// size bound.
#[macro_export]
macro_rules! impl_storable_unbounded {
    ($ident:ident) => {
        impl $crate::cdk::structures::storable::Storable for $ident {
            const BOUND: $crate::cdk::structures::storable::Bound =
                $crate::cdk::structures::storable::Bound::Unbounded;

            fn to_bytes(&self) -> ::std::borrow::Cow<'_, [u8]> {
                ::std::borrow::Cow::Owned($crate::serialize::serialize(self).unwrap())
            }

            fn into_bytes(self) -> Vec<u8> {
                $crate::serialize::serialize(&self).unwrap()
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                $crate::serialize::deserialize(&bytes).unwrap()
            }
        }
    };
}
