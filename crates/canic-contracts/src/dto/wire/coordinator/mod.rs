//! Module: dto::wire::coordinator
//!
//! Responsibility: own the coordinator wire union declarations.
//! Does not own: endpoint dispatch, authority, state, or capability resolution.
//! Boundary: ordinary consumers use the complete types; endpoint emitters select
//! the same declarations with the build-resolved capability attributes.

/// Emit the coordinator contract with an explicit capability selection.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_coordinator_wire_types {
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
            Overview,
        }

        #[derive(
            $crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// PublicStatusResponse wire union selected from the role's compiled capabilities.
        pub enum PublicStatusResponse {
            Health($crate::dto::public_status::PublicHealth),
            Overview($crate::dto::role::RoleOverviewResponse),
        }

        pub use $crate::dto::fleet_coordinator::{
            CoordinatorObservabilityRequest, CoordinatorObservabilityResponse,
            CoordinatorOperationReadRequest, CoordinatorOperationReadResponse,
            CoordinatorRegistryRequest, CoordinatorRegistryResponse,
        };
    };
}

crate::__canic_coordinator_wire_types! {
    serde_crate = "serde",

}
