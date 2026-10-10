#![expect(
    clippy::large_enum_variant,
    reason = "complete canonical role unions retain the existing inline wire payloads"
)]

//! Module: dto::wire::managed_command
//!
//! Responsibility: own the managed command wire union declarations.
//! Does not own: endpoint dispatch, authority, state, or capability resolution.
//! Boundary: ordinary consumers use the complete types; endpoint emitters select
//! the same declarations with the build-resolved capability attributes.

use crate::dto::wire::relay::RelayedObservabilityResponse;

/// Emit the managed command contract with an explicit capability selection.

#[doc(hidden)]
#[macro_export]
macro_rules! __canic_managed_command_wire_types {
    (
        serde_crate = $serde_crate:literal,
        fleet_admission_attributes = [$($fleet_admission_attributes:tt)*],
        application_authorization_attributes = [$($application_authorization_attributes:tt)*],
        caller_authority_attributes = [$($caller_authority_attributes:tt)*],
        token_issuer_attributes = [$($token_issuer_attributes:tt)*],
        child_provisioning_attributes = [$($child_provisioning_attributes:tt)*],
    ) => {
        #[derive(
            $crate::__reexports::candid::CandidType,
            $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// CanisterCommand wire union selected from the role's compiled capabilities.
        pub enum CanisterCommand {
            $($fleet_admission_attributes)*
            ActivateFleetAdmission(
                $crate::dto::fleet_admission::FleetAdmissionActivateTargetRequest,
            ),
            $($application_authorization_attributes)*
            ApplicationSession($crate::dto::auth::ApplicationSessionCommand),
            $($caller_authority_attributes)*
            CallerAuthority($crate::dto::caller_authority::CallerAuthorityCommand),
            $($caller_authority_attributes)*
            ReleaseApplicationStartup($crate::dto::caller_authority::CallerAuthorityPublication),
            ConfigureRuntime(
                $crate::dto::component_registry::ComponentRuntimeDirectoryPreparationRequest,
            ),
            $($token_issuer_attributes)*
            InstallDelegationProof(
                $crate::dto::auth::InstallActiveDelegationProofRequest,
            ),
            Observe($crate::dto::observability::CanisterObservabilityRequest),
            $($fleet_admission_attributes)*
            OpenFleetAdmission(
                $crate::dto::fleet_admission::FleetAdmissionOpenTargetRequest,
            ),
            $($token_issuer_attributes)*
            PrepareDelegatedToken($crate::dto::auth::DelegatedTokenPrepareRequest),
            $($fleet_admission_attributes)*
            PrepareFleetAdmission(
                $crate::dto::fleet_admission::FleetAdmissionPrepareTargetRequest,
            ),
            $($child_provisioning_attributes)*
            RespondCapability($crate::dto::capability::NonrootCyclesCapabilityEnvelopeV1),
            SynchronizeState($crate::dto::cascade::StateSnapshotInput),
        }

#[derive(
            $crate::__reexports::candid::CandidType,
            $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// CanisterCommandResponse wire union selected from the role's compiled capabilities.
        pub enum CanisterCommandResponse {
            $($fleet_admission_attributes)*
            ActivateFleetAdmission(
                $crate::dto::fleet_admission::FleetAdmissionTargetReceipt,
            ),
            $($application_authorization_attributes)*
            ApplicationSession($crate::dto::auth::ApplicationSessionCommandResponse),
            $($caller_authority_attributes)*
            CallerAuthority($crate::dto::caller_authority::CallerAuthorityReceipt),
            $($token_issuer_attributes)*
            InstallDelegationProof(
                $crate::dto::auth::InstallActiveDelegationProofResponse,
            ),
            Observe(RelayedObservabilityResponse),
            $($fleet_admission_attributes)*
            OpenFleetAdmission(
                $crate::dto::fleet_admission::FleetAdmissionTargetReceipt,
            ),
            OperationAccepted($crate::dto::role::OperationReceipt),
            $($token_issuer_attributes)*
            PrepareDelegatedToken($crate::dto::auth::DelegatedTokenPrepareResponse),
            $($fleet_admission_attributes)*
            PrepareFleetAdmission(
                $crate::dto::fleet_admission::FleetAdmissionTargetReceipt,
            ),
            $($child_provisioning_attributes)*
            RespondCapability($crate::dto::capability::NonrootCyclesCapabilityResponseV1),
            SynchronizeState($crate::dto::cascade::StateCascadeReport),
        }
    };
}

crate::__canic_managed_command_wire_types! {
    serde_crate = "serde",
    fleet_admission_attributes = [],
    application_authorization_attributes = [],
    caller_authority_attributes = [],
    token_issuer_attributes = [],
    child_provisioning_attributes = [],
}
