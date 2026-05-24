///
/// Canic IC SDK facade
///
pub use candid;
pub use ic_cdk::{
    api, call, eprintln, export_candid, futures, init, inspect_message, post_upgrade, println,
    query, trap, update,
};
pub use ic_cdk_management_canister as mgmt;
pub use ic_cdk_timers as timers;
pub use icrc_ledger_types;

pub mod serialize;
pub mod spec;
pub mod structures;
pub mod types;
pub mod utils;

///
/// Storable helpers
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
