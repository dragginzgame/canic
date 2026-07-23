//! Module: cdk
//!
//! Responsibility: Canic's internal IC SDK substrate and stable-structure helper macros.
//! Does not own: application policy, endpoint auth, or stable schema design.
//! Boundary: centralizes SDK dependencies for Canic-owned runtime implementation.

pub use candid;
pub use ic_cdk::{
    api, call, eprintln, export_candid, futures, init, inspect_message, post_upgrade, println,
    query, trap, update,
};

pub mod serialize;
pub mod structures;
pub mod types;
pub mod utils;

///
/// impl_storable_bounded
///
/// Implement bounded stable storage encoding for one serde-backed type.
///

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
                let bytes = $crate::cdk::serialize::serialize(self).unwrap_or_else(|err| {
                    panic!("impl_storable_bounded: serialize failed: {err}");
                });

                ::std::borrow::Cow::Owned(bytes)
            }

            fn into_bytes(self) -> Vec<u8> {
                $crate::cdk::serialize::serialize(&self).unwrap_or_else(|err| {
                    panic!("impl_storable_bounded: serialize failed: {err}");
                })
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                $crate::cdk::serialize::deserialize(&bytes).unwrap_or_else(|err| {
                    panic!("impl_storable_bounded: deserialize failed: {err}");
                })
            }
        }
    };
}

///
/// impl_storable_unbounded
///
/// Implement unbounded stable storage encoding for one serde-backed type.
///

#[macro_export]
macro_rules! impl_storable_unbounded {
    ($ident:ident) => {
        impl $crate::cdk::structures::storable::Storable for $ident {
            const BOUND: $crate::cdk::structures::storable::Bound =
                $crate::cdk::structures::storable::Bound::Unbounded;

            fn to_bytes(&self) -> ::std::borrow::Cow<'_, [u8]> {
                let bytes = $crate::cdk::serialize::serialize(self).unwrap_or_else(|err| {
                    panic!("impl_storable_unbounded: serialize failed: {err}");
                });

                ::std::borrow::Cow::Owned(bytes)
            }

            fn into_bytes(self) -> Vec<u8> {
                $crate::cdk::serialize::serialize(&self).unwrap_or_else(|err| {
                    panic!("impl_storable_unbounded: serialize failed: {err}");
                })
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                $crate::cdk::serialize::deserialize(&bytes).unwrap_or_else(|err| {
                    panic!("impl_storable_unbounded: deserialize failed: {err}");
                })
            }
        }
    };
}
