//! Module: cdk
//!
//! Responsibility: Canic's internal Candid, stable-structure, value-type, and hash substrate.
//! Does not own: IC runtime APIs, application policy, endpoint auth, or stable schema design.
//! Boundary: shared implementation support for Canic-owned runtime crates.

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

            fn to_bytes(
                &self,
            ) -> ::core::result::Result<
                ::std::borrow::Cow<'_, [u8]>,
                $crate::cdk::structures::storable::CodecError,
            > {
                $crate::cdk::serialize::serialize(self)
                    .map(::std::borrow::Cow::Owned)
                    .map_err($crate::cdk::serialize::storable_error)
            }

            fn into_bytes(
                self,
            ) -> ::core::result::Result<Vec<u8>, $crate::cdk::structures::storable::CodecError>
            {
                $crate::cdk::serialize::serialize(&self)
                    .map_err($crate::cdk::serialize::storable_error)
            }

            fn from_bytes(
                bytes: ::std::borrow::Cow<'_, [u8]>,
            ) -> ::core::result::Result<Self, $crate::cdk::structures::storable::CodecError> {
                $crate::cdk::serialize::deserialize(&bytes)
                    .map_err($crate::cdk::serialize::storable_error)
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

            fn to_bytes(
                &self,
            ) -> ::core::result::Result<
                ::std::borrow::Cow<'_, [u8]>,
                $crate::cdk::structures::storable::CodecError,
            > {
                $crate::cdk::serialize::serialize(self)
                    .map(::std::borrow::Cow::Owned)
                    .map_err($crate::cdk::serialize::storable_error)
            }

            fn into_bytes(
                self,
            ) -> ::core::result::Result<Vec<u8>, $crate::cdk::structures::storable::CodecError>
            {
                $crate::cdk::serialize::serialize(&self)
                    .map_err($crate::cdk::serialize::storable_error)
            }

            fn from_bytes(
                bytes: ::std::borrow::Cow<'_, [u8]>,
            ) -> ::core::result::Result<Self, $crate::cdk::structures::storable::CodecError> {
                $crate::cdk::serialize::deserialize(&bytes)
                    .map_err($crate::cdk::serialize::storable_error)
            }
        }
    };
}
