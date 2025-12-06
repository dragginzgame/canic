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
                let bytes = $crate::utils::serialize::serialize(self).unwrap_or_else(|e| {
                    panic!("impl_storable_bounded: serialize failed: {e}");
                });

                ::std::borrow::Cow::Owned(bytes)
            }

            fn into_bytes(self) -> Vec<u8> {
                $crate::utils::serialize::serialize(&self).unwrap_or_else(|e| {
                    panic!("impl_storable_bounded: serialize failed: {e}");
                })
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                $crate::utils::serialize::deserialize(&bytes).unwrap_or_else(|e| {
                    panic!("impl_storable_bounded: deserialize failed: {e}");
                })
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
                let bytes = $crate::utils::serialize::serialize(self).unwrap_or_else(|e| {
                    panic!("impl_storable_unbounded: serialize failed: {e}");
                });

                ::std::borrow::Cow::Owned(bytes)
            }

            fn into_bytes(self) -> Vec<u8> {
                $crate::utils::serialize::serialize(&self).unwrap_or_else(|e| {
                    panic!("impl_storable_unbounded: serialize failed: {e}");
                })
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                $crate::utils::serialize::deserialize(&bytes).unwrap_or_else(|e| {
                    panic!("impl_storable_unbounded: deserialize failed: {e}");
                })
            }
        }
    };
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use canic_cdk::structures::storable::Storable;
    use serde::{Deserialize, Serialize};
    use std::borrow::Cow;

    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    struct Sample {
        v: u32,
    }

    impl_storable_bounded!(Sample, 32, false);

    #[test]
    fn bounded_round_trip() {
        let s = Sample { v: 42 };
        let bytes = s.to_bytes();
        let decoded = Sample::from_bytes(bytes);

        assert_eq!(decoded, s);
    }

    #[test]
    #[should_panic(expected = "impl_storable_bounded: deserialize failed")]
    fn bounded_deserialize_panics_with_context() {
        let bytes = Cow::Owned(vec![0xFF]); // invalid CBOR
        let _ = Sample::from_bytes(bytes);
    }
}
