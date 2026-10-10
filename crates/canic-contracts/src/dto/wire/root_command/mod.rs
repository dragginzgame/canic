//! Module: dto::wire::root_command
//!
//! Responsibility: own the root command wire union declarations.
//! Does not own: endpoint dispatch, authority, state, or capability resolution.
//! Boundary: ordinary consumers use the complete types; endpoint emitters select
//! the same declarations with the build-resolved capability attributes.

/// Emit the root command contract with an explicit capability selection.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_root_command_wire_types {
    (
        serde_crate = $serde_crate:literal,
        root_delegation_attributes = [$($root_delegation_attributes:tt)*],
        role_attestation_attributes = [$($role_attestation_attributes:tt)*],
    ) => {
        #[derive(
            $crate::__reexports::candid::CandidType,
            $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// RootCommand wire union selected from the role's compiled capabilities.
        pub enum RootCommand {
            AcceptFunding($crate::dto::fleet_funding::FleetRootFundingAcceptanceRequest),
            ActivateFleetAdmission(
                $crate::dto::fleet_admission::FleetAdmissionActivateRootRequest,
            ),
            ActivateFundingPolicyRotation(
                $crate::dto::fleet_funding::FleetFundingPolicyRotationRootActivateRequest,
            ),
            AdoptStore($crate::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest),
            BindComponentInitialization($crate::dto::component_registry::RootComponentInitializationRequest),
            BootstrapStore($crate::dto::root_store::RootStoreBootstrapRequest),
            $($root_delegation_attributes)*
            GetOrCreateDelegationProof,
            $($root_delegation_attributes)*
            GetChainKeyPublicKey($crate::dto::auth::RootChainKeyPublicKeyRequest),
            HandoffPoolCanister($crate::dto::pool::PoolHandoffRequest),
            ImportPoolCanister($crate::dto::pool::PoolCanisterRequest),
            ImportPoolCapacity($crate::dto::pool_import::PoolImportCommand),
            InspectCanister($crate::dto::canister::CanisterInspectionRequest),
            InspectCanisterHistory($crate::dto::canister::CanisterInspectionRequest),
            MaintainPool,
            ObserveCanister($crate::dto::observability::FleetCanisterObservabilityRequest),
            OpenFleetAdmission(
                $crate::dto::fleet_admission::FleetAdmissionOpenRootRequest,
            ),
            PrepareAuthoritySnapshot($crate::dto::authority_restore::AuthoritySnapshotRequest),
            PrepareComponentRegistry(
                $crate::dto::component_registry::RootComponentRegistryPreparationRequest,
            ),
            PrepareFleetActivation,
            PrepareFleetAdmission(
                $crate::dto::fleet_admission::FleetAdmissionPrepareRootRequest,
            ),
            PrepareFundingPolicyRotation(
                $crate::dto::fleet_funding::FleetFundingPolicyRotationRootPrepareRequest,
            ),
            $($role_attestation_attributes)*
            PrepareRoleAttestation($crate::dto::auth::RoleAttestationRequest),
            PrepareStoreFixture($crate::dto::root_store::RootStoreFixturePrepareRequest),
            PreviewCycleRefill($crate::dto::icp_refill::CycleRefillInput),
            ProvisionChild($crate::dto::component_registry::RootComponentChildAllocationRequest),
            ProvisionComponent($crate::dto::component_registry::RootComponentAllocationRequest),
            ProvisionComponents(
                $crate::dto::component_provisioning::RootComponentProvisioningAcceptanceRequest,
            ),
            ProvisionPeer($crate::dto::component_registry::RootPeerComponentAllocationRequest),
            PublishReleaseSet($crate::dto::template::WasmStoreAdminCommand),
            RefillCycles($crate::dto::icp_refill::CycleRefillInput),
            RemoveComponent($crate::dto::component_registry::RootComponentDrainingRequest),
            RemoveRoot($crate::dto::role::RootRemovalRequest),
            RemoveSubtree($crate::dto::component_registry::RootComponentSubtreeRemovalRequest),
            RespondCapability($crate::dto::capability::RootCapabilityEnvelopeV1),
            ResumeAuthoritySnapshot($crate::dto::authority_restore::AuthoritySnapshotRequest),
            ResumeFleetActivation($crate::dto::fleet_activation::FleetActivationResumeRequest),
            RetryPoolRefill,
            RetryPoolReset($crate::dto::pool::PoolCanisterRequest),
            SetCyclesFunding($crate::dto::state::SetCyclesFundingRequest),
            SetFleetStatus($crate::dto::state::SetFleetStatusRequest),
            SynchronizeComponentDirectories(
                $crate::dto::component_provisioning::RootComponentDirectorySynchronizationRequest,
            ),
            SynchronizeRegistry($crate::dto::fleet_registry::FleetSubnetRootRegistrySyncRequest),
            $($root_delegation_attributes)*
            ConfigureIssuer($crate::dto::auth::RootIssuerConfigureRequest),
        }

#[derive(
            $crate::__reexports::candid::CandidType,
            $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// RootCommandResponse wire union selected from the role's compiled capabilities.
        pub enum RootCommandResponse {
            PrepareStoreFixture(Result<$crate::dto::fixture_provisioning::FixtureSourceStatus,
                $crate::dto::fixture_provisioning::FixtureStoreError>),
            AcceptFunding($crate::dto::fleet_funding::FleetRootFundingAcceptanceReceipt),
            ActivateFleetAdmission(
                $crate::dto::fleet_admission::FleetAdmissionRootReceipt,
            ),
            ActivateFundingPolicyRotation(
                $crate::dto::fleet_funding::FleetFundingPolicyRotationRootReceipt,
            ),
            $($root_delegation_attributes)*
            GetOrCreateDelegationProof($crate::dto::auth::RootDelegationProofBatchProof),
            $($root_delegation_attributes)*
            GetChainKeyPublicKey(Vec<u8>),
            HandoffPoolCanister($crate::dto::pool::PoolHandoffResponse),
            ImportPoolCanister($crate::dto::pool::PoolImportResponse),
            ImportPoolCapacity($crate::dto::pool_import::PoolImportStatus),
            InspectCanister($crate::dto::canister::CanisterStatusResponse),
            InspectionReserveRequired($crate::dto::canister::CanisterInspectionReserveResponse),
            InspectCanisterHistory($crate::dto::canister::CanisterHistoryResponse),
            MaintainPool($crate::dto::pool::PoolMaintenanceResponse),
            ObserveCanister($crate::dto::observability::CanisterObservabilityResponse),
            OperationAccepted($crate::dto::role::OperationReceipt),
            OpenFleetAdmission($crate::dto::fleet_admission::FleetAdmissionRootReceipt),
            PrepareAuthoritySnapshot(
                $crate::dto::authority_restore::AuthorityRestoreFenceStatusResponse,
            ),
            PrepareComponentRegistry(
                $crate::dto::component_registry::RootComponentRegistryStatusResponse,
            ),
            PrepareFleetAdmission($crate::dto::fleet_admission::FleetAdmissionRootReceipt),
            PrepareFundingPolicyRotation(
                $crate::dto::fleet_funding::FleetFundingPolicyRotationRootReceipt,
            ),
            $($role_attestation_attributes)*
            PrepareRoleAttestation($crate::dto::auth::RoleAttestationPrepareResponse),
            PreviewCycleRefill($crate::dto::icp_refill::IcpRefillDryRun),
            PublishReleaseSet($crate::dto::template::WasmStoreAdminResponse),
            RespondCapability($crate::dto::capability::RootCapabilityResponseV1),
            ResumeAuthoritySnapshot(
                $crate::dto::authority_restore::AuthorityRestoreFenceStatusResponse,
            ),
            RetryPoolRefill($crate::dto::pool::PoolRefillRetryResponse),
            RetryPoolReset($crate::dto::pool::PoolResetRetryResponse),
            SetCyclesFunding($crate::dto::state::FleetStateCommandResult<bool>),
            SetFleetStatus(
                $crate::dto::state::FleetStateCommandResult<$crate::dto::state::FleetStatus>,
            ),
            SynchronizeComponentDirectories(
                $crate::dto::component_provisioning::RootComponentDirectorySynchronizationResponse,
            ),
            $($root_delegation_attributes)*
            ConfigureIssuer($crate::dto::auth::RootIssuerConfigureResponse),
        }
    };
}

crate::__canic_root_command_wire_types! {
    serde_crate = "serde",
    root_delegation_attributes = [],
    role_attestation_attributes = [],
}
