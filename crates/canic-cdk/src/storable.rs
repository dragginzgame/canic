//! Derive stable-structures storage traits using CBOR serialization.

/// Implement [`Storable`](crate::structures::storable::Storable) with a
/// bounded size guarantee.
#[macro_export]
macro_rules! impl_storable_bounded {
    ($ident:ident, $max_size:expr, $is_fixed_size:expr) => {
        impl $crate::structures::storable::Storable for $ident {
            const BOUND: $crate::structures::storable::Bound =
                $crate::structures::storable::Bound::Bounded {
                    max_size: $max_size,
                    is_fixed_size: $is_fixed_size,
                };

            fn to_bytes(&self) -> ::std::borrow::Cow<'_, [u8]> {
                let bytes = $crate::serialize::serialize(self).unwrap_or_else(|err| {
                    panic!("impl_storable_bounded: serialize failed: {err}");
                });

                ::std::borrow::Cow::Owned(bytes)
            }

            fn into_bytes(self) -> Vec<u8> {
                $crate::serialize::serialize(&self).unwrap_or_else(|err| {
                    panic!("impl_storable_bounded: serialize failed: {err}");
                })
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                $crate::serialize::deserialize(&bytes).unwrap_or_else(|err| {
                    panic!("impl_storable_bounded: deserialize failed: {err}");
                })
            }
        }
    };
}

/// Implement [`Storable`](crate::structures::storable::Storable) without a
/// size bound.
#[macro_export]
macro_rules! impl_storable_unbounded {
    ($ident:ident) => {
        impl $crate::structures::storable::Storable for $ident {
            const BOUND: $crate::structures::storable::Bound =
                $crate::structures::storable::Bound::Unbounded;

            fn to_bytes(&self) -> ::std::borrow::Cow<'_, [u8]> {
                let bytes = $crate::serialize::serialize(self).unwrap_or_else(|err| {
                    panic!("impl_storable_unbounded: serialize failed: {err}");
                });

                ::std::borrow::Cow::Owned(bytes)
            }

            fn into_bytes(self) -> Vec<u8> {
                $crate::serialize::serialize(&self).unwrap_or_else(|err| {
                    panic!("impl_storable_unbounded: serialize failed: {err}");
                })
            }

            fn from_bytes(bytes: ::std::borrow::Cow<'_, [u8]>) -> Self {
                $crate::serialize::deserialize(&bytes).unwrap_or_else(|err| {
                    panic!("impl_storable_unbounded: deserialize failed: {err}");
                })
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::structures::storable::Storable;
    use serde::{Deserialize, Serialize};
    use std::borrow::Cow;

    ///
    /// Sample
    ///
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    struct Sample {
        v: u32,
    }

    impl_storable_bounded!(Sample, 32, false);

    #[test]
    fn bounded_round_trip() {
        let sample = Sample { v: 42 };
        let bytes = sample.to_bytes();
        let decoded = Sample::from_bytes(bytes);

        assert_eq!(decoded, sample);
    }

    #[test]
    #[should_panic(expected = "impl_storable_bounded: deserialize failed")]
    fn bounded_deserialize_panics_with_context() {
        let bytes = Cow::Owned(vec![0xFF]);
        let _ = Sample::from_bytes(bytes);
    }
}
