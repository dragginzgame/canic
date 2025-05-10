#[macro_export]
macro_rules! impl_storable_bounded {
    ($ident:ident, $max_size:expr, $is_fixed_size:expr) => {
        impl ::ic_stable_structures::storable::Storable for $ident {
            fn to_bytes(&self) -> ::std::borrow::Cow<[u8]> {
                ::std::borrow::Cow::Owned(::icu::serialize::serialize(self).unwrap())
            }

            fn from_bytes(bytes: ::std::borrow::Cow<[u8]>) -> Self {
                ::icu::serialize::deserialize(&bytes).unwrap()
            }

            const BOUND: ::ic_stable_structures::storable::Bound =
                ::ic_stable_structures::storable::Bound::Bounded {
                    max_size: $max_size,
                    is_fixed_size: $is_fixed_size,
                };
        }
    };
}

#[macro_export]
macro_rules! impl_storable_unbounded {
    ($ident:ident) => {
        impl ::ic_stable_structures::storable::Storable for $ident {
            fn to_bytes(&self) -> ::std::borrow::Cow<[u8]> {
                ::std::borrow::Cow::Owned(::icu::serialize::serialize(self).unwrap())
            }

            fn from_bytes(bytes: ::std::borrow::Cow<[u8]>) -> Self {
                ::icu::serialize::deserialize(&bytes).unwrap()
            }

            const BOUND: ::ic_stable_structures::storable::Bound =
                ::ic_stable_structures::storable::Bound::Unbounded;
        }
    };
}
