//! Module: dto::wire::store
//!
//! Responsibility: own the store wire union declarations.
//! Does not own: endpoint dispatch, authority, state, or capability resolution.
//! Boundary: ordinary consumers use the complete types; endpoint emitters select
//! the same declarations with the build-resolved capability attributes.

/// Emit the store contract with an explicit capability selection.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_store_wire_types {
    (
        serde_crate = $serde_crate:literal,

    ) => {
        #[derive(
            $crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// PublicStatusRequest wire union selected from the role's compiled capabilities.
        pub enum PublicStatusRequest {
            Health,
            Metrics($crate::dto::public_status::PublicMetricsRequest),
            History($crate::dto::public_status::PublicHistoryRequest),
            Overview,
        }

        #[derive(
            $crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// PublicStatusResponse wire union selected from the role's compiled capabilities.
        pub enum PublicStatusResponse {
            Health($crate::dto::public_status::PublicHealth),
            Metrics($crate::dto::public_status::PublicMetricsSnapshot),
            History($crate::dto::public_status::PublicHistorySnapshot),
            Overview($crate::dto::role::RoleOverviewResponse),
        }

        pub use $crate::dto::template::{
            StoreCatalogRequest, StoreCatalogResponse, StoreObservabilityRequest,
            StoreObservabilityResponse, StoreStatusRequest, StoreStatusResponse,
        };
    };
}

crate::__canic_store_wire_types! {
    serde_crate = "serde",

}
