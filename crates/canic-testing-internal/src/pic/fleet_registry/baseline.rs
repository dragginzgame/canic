//! Prepared-root Fleet Registry and Component Registry PocketIC journey.

#[cfg(test)]
use super::build::{
    build_five_component_root_wasm, build_five_trillion_component_root_wasm, build_icp_refill_pic,
    build_icp_refill_stub_wasm, build_initial_shard_root_wasm, build_journey_cycles_ledger_wasm,
    build_mainnet_five_component_refill_wasms, build_mainnet_refill_wasms, build_management_pic,
    build_two_root_pic, five_component_root_canister_config_path,
    five_trillion_component_root_canister_config_path, initial_shard_root_canister_config_path,
};
use super::build::{
    build_pic, build_test_root_wasm, build_test_wasm_store_wasm, root_canister_config_path,
};
use candid::Principal;
use ic_testkit::pic::{CandidCallExt, PocketIc};
use std::path::Path;

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;
const PREPAID_POOL_ASSET_COUNT: usize = 10;
const PREPAID_POOL_ASSET_CYCLES: u128 = 6_000_000_000_000;

#[cfg(test)]
pub(in crate::pic) use tests::{governed_fleet_journey_cases, governed_pocketic_cases};

mod tests {
    #[cfg(test)]
    mod activation_reset;
    #[cfg(test)]
    mod child_reserve;
    #[cfg(test)]
    mod funding_deadline;
    #[cfg(test)]
    mod native_funding;
    #[cfg(test)]
    mod operator_shortfall;
    #[cfg(test)]
    mod sibling_funding;
    #[cfg(test)]
    mod state_cascade;

    use super::*;
    use crate::pic::{report_canister_diagnostics, report_canister_diagnostics_batch};
    #[cfg(test)]
    use candid::Nat;
    #[cfg(test)]
    use candid::decode_args;
    use candid::{CandidType, Deserialize, decode_one, encode_one};
    #[cfg(test)]
    use canic::dto::authority_restore::{
        AuthorityRestoreFencePhase, AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest,
    };
    #[cfg(test)]
    use canic::dto::canister::{CanisterInspectionRequest, CanisterStatusResponse};
    #[cfg(test)]
    use canic::dto::component_registry::{
        ComponentLifecycleStatus, ComponentRegistryActivePartitionRequest,
        ComponentRegistryActivePartitionResponse,
    };
    #[cfg(test)]
    use canic::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest;
    #[cfg(test)]
    use canic::dto::pool::{
        CanisterPoolAssetOrigin, CanisterPoolAssetStatus, PoolCanisterRequest, PoolImportResponse,
    };
    use canic::dto::pool::{
        CanisterPoolResponse, CanisterPoolStatusRequest, PoolMaintenanceResponse,
    };
    #[cfg(test)]
    use canic::dto::runtime::{CanicRuntimeStatus, TimerRegistrationStatus};
    #[cfg(test)]
    use canic::dto::{
        cycles::CycleTrackerEntry,
        fleet_admission::FleetAdmissionProjectionStatusResponse,
        metrics::{MetricEntry, MetricsKind},
        observability::{
            CanisterObservabilityRequest, CanisterObservabilityResponse,
            FleetCanisterObservabilityRequest,
        },
        page::{Page, PageRequest},
        role::{CycleBalanceStatusResponse, MetricsStatusRequest},
    };
    use canic::ids::ManagedCanisterBinding;
    use canic::{
        CANIC_WASM_CHUNK_BYTES,
        dto::{
            component_registry::{
                ComponentRuntimePhase, RootComponentAllocationPhase,
                RootComponentAllocationRequest, RootComponentAllocationResponse,
                RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
            },
            fixture_provisioning::{FixtureChunkUpload, FixtureSourceStatus, FixtureStoreError},
            fleet_registry::{
                FleetRegistryActivationRequest, FleetSubnetRootEntry, FleetSubnetRootJoinRequest,
                FleetSubnetRootRegistrySyncRequest, FleetSubnetRootStatus,
            },
            fleet_subnet_root::{
                FleetSubnetRootAuthority, FleetSubnetRootCanisterSummary, FleetSubnetRootInitArgs,
                FleetSubnetWasmStoreInitArgs,
            },
            role::{
                ComponentRuntimeOperationStatus, OperationReceipt, OperationStatusRequest,
                RoleOverviewResponse,
            },
            root_store::{
                ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX, ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX,
                RootStoreArtifact, RootStoreBootstrapRequest, RootStoreBootstrapResponse,
                RootStoreFixturePrepareRequest, RootStoreReleaseSetEntry,
                RootStoreReleaseSetEntryKind, RootStoreReleaseSetManifest,
            },
        },
        ids::{CanisterRole, ComponentBinding, FleetId, ReleaseSetDigest, SubnetId},
    };
    use canic::{
        Error,
        dto::fleet_activation::{FleetActivationPhase, FleetActivationResumeRequest},
    };
    #[cfg(test)]
    use canic_control_plane::dto::fleet_coordinator::{
        CoordinatorOperationReadRequest, CoordinatorOperationReadResponse,
    };
    #[cfg(test)]
    use canic_control_plane::dto::root::RootFundingStatusResponse;
    #[cfg(test)]
    use canic_control_plane::dto::template::{
        StoreCatalogRequest, StoreCatalogResponse, StoreObservabilityRequest,
        StoreObservabilityResponse, TemplateLookupRequest, TemplateManifestResponse,
        TemplateStagingStatusResponse,
    };
    use canic_control_plane::{
        dto::template::{
            StoreCommand, StoreCommandResponse, TemplateChunkInput, TemplateChunkSetInfoResponse,
            TemplateChunkSetPrepareInput, TemplateManifestInput,
        },
        dto::{
            fleet_coordinator::{
                CoordinatorCommand, CoordinatorCommandResponse, CoordinatorObservabilityRequest,
                CoordinatorObservabilityResponse, CoordinatorRegistryRequest,
                CoordinatorRegistryResponse, FleetCoordinatorInitArgs,
            },
            root::RootOperationStatusResponse,
        },
        ids::{
            TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion,
            WasmStoreBinding,
        },
    };
    #[cfg(test)]
    use canic_core::{
        cdk::types::Cycles,
        dto::{
            component_provisioning::{
                FleetComponentProvisioningRetryStage, ProvisioningFailureOrigin,
                ProvisioningFailureStage, ProvisioningRetryCategory,
                RootComponentProvisioningPhase,
            },
            fleet_admission::{
                FleetAdmissionMutationAction, FleetAdmissionMutationOutcome,
                FleetAdmissionMutationRequest, FleetAdmissionMutationResponse,
                FleetAdmissionOperationPhase, FleetAdmissionProjectionPhase,
                FleetAdmissionRootStatusResponse, FleetAdmissionRootTransitionPhase,
            },
            fleet_funding::{
                FleetFundingPolicyRotationApplyRequest, FleetFundingPolicyRotationBeginRequest,
                FleetFundingPolicyRotationFundingSource,
                FleetFundingPolicyRotationPlacementEvidence, FleetFundingPolicyRotationPlan,
                FleetFundingPolicyRotationPlanHeader, FleetFundingPolicyRotationReceipt,
                FleetFundingPolicyRotationRootPlan, FleetFundingPolicyRotationStageRootRequest,
                FleetFundingPolicyUsage, FleetRootFundingNoGrantReason, FleetRootFundingResponse,
            },
            icp_refill::{IcpRefillStatus, IcpRefillTrigger},
        },
        ids::{
            CyclesFundingBudget, FleetAdmissionPolicy, FleetAdmissionSelector, FleetFundingProfile,
            FleetSubnetRootAutomaticIcpRefillPolicy, FleetSubnetRootFundingPolicy,
            FleetSubnetRootIcpRefillPolicy,
        },
        shared_support::fleet_admission_policy::{
            compile_installed_fleet_admission_policy, effective_fleet_admission_principals,
            fleet_admission_participant_catalog_digest,
            fleet_admission_root_participant_catalog_digest, fleet_admission_target_for_binding,
            materialize_fleet_admission_projection,
        },
        shared_support::fleet_funding_policy::{
            coordinator_root_funding_policy_hash, fleet_funding_policy_rotation_operation_id,
            fleet_funding_policy_rotation_plan_digest, fleet_funding_policy_rotation_roots_digest,
            validate_fleet_funding_policy_rotation_plan,
        },
    };
    use canic_core::{
        cdk::utils::hash::{hex_bytes, wasm_hash},
        ids::{FleetCoordinatorRootFundingPolicy, FleetSubnetRootFundingAuthority, ReleaseBuildId},
    };
    #[cfg(test)]
    use canic_core::{
        ids::{BuildNetwork, ReleaseBuildNonce},
        shared_support::fleet_admission_policy::compile_fleet_admission_policy_template,
    };
    #[cfg(test)]
    use canic_host::fleet_ensure::model::{
        CurrentFleetProtocolAction, DesiredCanisterKind, DesiredComponentGroupPlacement,
        DesiredFleet, DesiredFleetBootstrap, DesiredFleetBootstrapRoot, DesiredFleetProtocol,
        EffectRecord, EffectState, EnsureAction, FLEET_ENSURE_SCHEMA_VERSION, FleetEnsurePlan,
        FleetEnsureStateRecord,
    };
    #[cfg(test)]
    use canic_host::fleet_ensure::ops::{EnsurePlatform, action_sha256};
    #[cfg(test)]
    use canic_host::fleet_ensure::{
        CompiledCurrentComponentProvisioning, CompiledCurrentProtocolStep,
        CurrentComponentGroupPlacement, CurrentRegistryStage, EnsureWorkflowError,
        IcpEnsurePlatform, IcpEnsurePlatformError, compile_current_component_provisioning,
        compile_current_protocol_sequence, compile_current_registry_sequence,
        compile_current_registry_sequence_with_status, compile_current_store_sequence_from_union,
        workflow as fleet_ensure_workflow,
    };
    #[cfg(test)]
    use canic_host::icp::LocalReplicaTarget;
    use canic_host::release_set::AppConfigSnapshot;
    #[cfg(test)]
    use canic_host::release_set::{
        ApplicationArtifactBuildTarget, ApplicationArtifactEntry,
        ApplicationArtifactFileBuildOutput, ApplicationArtifactUnion,
        CanicInfrastructureArtifactBuildOutput, CanicInfrastructureRole,
        compile_and_persist_application_artifact_union,
        compile_and_persist_canic_infrastructure_artifact_manifest,
        compile_and_persist_current_release_set_manifest,
    };
    #[cfg(test)]
    use canic_host::{
        canister_build::{
            CanisterArtifactBuildOutput, CanisterArtifactBuilder, CanisterBuildProfile,
            WorkspaceBuildContext,
        },
        release_build::finalize_release_build_from_manifest,
        role_contract::{
            PackageValidationMode, RolePackageValidation, validate_declared_role_package,
        },
    };
    #[cfg(test)]
    use ciborium::Value;
    use flate2::{Compression, write::GzEncoder};
    use std::{
        collections::BTreeMap,
        error::Error as StdError,
        fmt,
        io::Write,
        num::NonZeroUsize,
        sync::OnceLock,
        time::{Duration, Instant},
    };
    #[cfg(test)]
    use std::{collections::BTreeSet, path::PathBuf, process::Command, time::SystemTime};

    #[cfg(test)]
    use crate::pic::artifacts::{
        INTERNAL_TEST_RELEASE_BUILD_ID, INTERNAL_TEST_RELEASE_BUILD_NONCE,
        internal_test_artifact_maintenance_interval, internal_test_artifact_prune_policy,
        report_artifact_cache_maintenance, with_canonical_root_cargo_inputs,
    };
    use crate::pic::fleet_registry::fixture::progress_elapsed;
    #[cfg(test)]
    use crate::pic::timing::Span;
    use crate::pic::{
        CanicWasmBuildProfile,
        artifacts::{
            build_canonical_fleet_coordinator_wasm, build_internal_test_wasm_canisters_with_env,
        },
        canic::{
            ManagedRootInstallInput, adopt_sibling_wasm_store,
            install_root_args_with_release_set_digest_and_coordinator, managed_test_init_identity,
            prepare_sibling_wasm_store_controllers,
        },
    };
    #[cfg(test)]
    use ic_testkit::artifacts::{
        ArtifactCacheOutcome, ArtifactCachePreparation, ArtifactCacheSpec, WasmBuildSpec,
        prepare_artifact_cache, resolve_cargo_build_inputs,
    };
    use ic_testkit::artifacts::{test_target_dir, workspace_root_for};
    #[cfg(test)]
    use ic_testkit::pic::PocketIcSnapshotExt;
    use ic_testkit::pic::{
        BaselinePoolContractError, BaselinePreparationStage, CachedPocketIcBaseline,
        CachedPocketIcBaselinePool, CachedPocketIcBaselinePoolGuard, CandidCallError,
        CanisterRestoreReceipt, CanisterSnapshotTarget, ControllerSnapshotError, CycleResetPolicy,
        FailureDisposition, FixtureRecipeId, PocketIcBaselineRecipe, PreparedBaseline,
        ReadinessReceipt, RebuildReason, ResetAchievement, ResetReceipt, ResetRequirement,
        ResetRequirements, SnapshotRestoreFunding, TimeResetPolicy, ValidationReceipt,
        is_dead_pocket_ic_transport_error,
    };
    #[cfg(test)]
    use ic_testkit::pocket_ic::{
        CanisterSettings, CreateCanisterParams, CreateCanisterPlacement,
        common::rest::RawEffectivePrincipal,
    };

    #[cfg(test)]
    use canic::dto::component_provisioning::FleetComponentProvisioningPhase;
    #[cfg(test)]
    use canic::dto::fleet_registry::FleetSubnetRootDrainingReservationRequest;
    #[cfg(test)]
    #[cfg(test)]
    use canic_control_plane::dto::fleet_coordinator::{
        CoordinatorFundingStatusResponse, CoordinatorOperationStatusResponse,
        FleetFundingPolicyRotationStatusPhase,
    };

    const ISSUER_PACKAGE: &str = "delegation_issuer_stub";
    const COORDINATOR_INSTALL_CYCLES: u128 = 500_000_000_000_000;
    #[cfg(test)]
    const LITERAL_ZERO_OBSERVATION_DELAY: Duration = Duration::from_millis(25);
    #[cfg(test)]
    const REINSTALL_RELEASE_BUILD_NONCE: [u8; 32] = [0x12; 32];
    #[cfg(test)]
    const ROOT_REMOVAL_MAX_SIMULATED_SECONDS: usize = 512;
    #[cfg(test)]
    const ROOT_REMOVAL_TICKS_PER_SECOND: usize = 4;
    #[cfg(test)]
    const QUALIFICATION_ASSET_CYCLES: u128 = 5_000_000_000_000;
    #[cfg(test)]
    const QUALIFICATION_FEE_CYCLES: u128 = 100_000_000;
    #[cfg(test)]
    const QUALIFICATION_RESERVE_CYCLES: u128 = 10_000_000_000_000;
    #[cfg(test)]
    const QUALIFICATION_WORKLOAD_PACKAGE: &str = "payload_limit_probe";

    #[cfg(test)]
    struct TestDirectoryCleanup(PathBuf);

    #[cfg(test)]
    impl Drop for TestDirectoryCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[derive(CandidType)]
    enum RootCommandFragment {
        #[cfg(test)]
        AdoptStore(FleetSubnetWasmStoreAdoptionRequest),
        BootstrapStore(RootStoreBootstrapRequest),
        PrepareStoreFixture(canic::dto::root_store::RootStoreFixturePrepareRequest),
        #[cfg(test)]
        InspectCanister(CanisterInspectionRequest),
        #[cfg(test)]
        #[expect(
            dead_code,
            reason = "the production adapter uses the replicated history Candid sidecar"
        )]
        InspectCanisterHistory(CanisterInspectionRequest),
        #[cfg(test)]
        ImportPoolCanister(PoolCanisterRequest),
        MaintainPool,
        #[cfg(test)]
        ObserveCanister(FleetCanisterObservabilityRequest),
        #[cfg(test)]
        PrepareAuthoritySnapshot(AuthoritySnapshotRequest),
        PrepareComponentRegistry(RootComponentRegistryPreparationRequest),
        PrepareFleetActivation,
        ProvisionComponent(RootComponentAllocationRequest),
        #[cfg(test)]
        RemoveSubtree(canic::dto::component_registry::RootComponentSubtreeRemovalRequest),
        #[cfg(test)]
        RespondCapability(canic::dto::capability::RootCapabilityEnvelopeV1),
        #[cfg(test)]
        ResumeAuthoritySnapshot(AuthoritySnapshotRequest),
        ResumeFleetActivation(FleetActivationResumeRequest),
        SynchronizeRegistry(FleetSubnetRootRegistrySyncRequest),
    }

    /// Exact host command type table used to inject loss at a binary request boundary.
    #[cfg(test)]
    #[derive(CandidType)]
    #[expect(
        dead_code,
        reason = "Candid includes every host command variant in its type table"
    )]
    enum HostRootCommandFragment {
        MaintainPool,
        ImportPoolCanister(PoolCanisterRequest),
        AdoptStore(FleetSubnetWasmStoreAdoptionRequest),
        BootstrapStore(RootStoreBootstrapRequest),
        PrepareStoreFixture(RootStoreFixturePrepareRequest),
        PrepareComponentRegistry(RootComponentRegistryPreparationRequest),
        SynchronizeRegistry(FleetSubnetRootRegistrySyncRequest),
    }

    #[derive(CandidType, Deserialize)]
    #[expect(
        clippy::large_enum_variant,
        reason = "the direct Root wire decoder changes size across test-only variants"
    )]
    enum RootCommandResponseFragment {
        PrepareStoreFixture(
            Result<
                canic::dto::fixture_provisioning::FixtureSourceStatus,
                canic::dto::fixture_provisioning::FixtureStoreError,
            >,
        ),
        #[cfg(test)]
        ImportPoolCanister(PoolImportResponse),
        #[cfg(test)]
        InspectCanister(CanisterStatusResponse),
        #[cfg(test)]
        InspectCanisterHistory(canic::dto::canister::CanisterHistoryResponse),
        MaintainPool(PoolMaintenanceResponse),
        #[cfg(test)]
        ObserveCanister(CanisterObservabilityResponse),
        OperationAccepted(OperationReceipt),
        #[cfg(test)]
        PrepareAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
        PrepareComponentRegistry(RootComponentRegistryStatusResponse),
        #[cfg(test)]
        ResumeAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
        #[cfg(test)]
        RespondCapability(canic::dto::capability::RootCapabilityResponseV1),
    }

    #[derive(CandidType)]
    enum RootStatusRequestFragment {
        #[cfg(test)]
        Admission(PageRequest),
        #[cfg(test)]
        AuthorityRestore,
        #[cfg(test)]
        ComponentRegistry(RootComponentRegistryPreparationRequest),
        #[cfg(test)]
        ComponentRegistryActivePartition(ComponentRegistryActivePartitionRequest),
        #[cfg(test)]
        ComponentRegistryPartition(
            canic::dto::component_registry::ComponentRegistryPartitionRequest,
        ),
        #[cfg(test)]
        CycleBalance,
        #[cfg(test)]
        CycleHistory(PageRequest),
        FleetAuthority,
        #[cfg(test)]
        Funding,
        Inventory,
        Operation(OperationStatusRequest),
        Pool(CanisterPoolStatusRequest),
        #[cfg(test)]
        Metrics(MetricsStatusRequest),
        #[cfg(test)]
        Runtime,
    }

    #[derive(CandidType, Deserialize)]
    #[expect(
        clippy::large_enum_variant,
        reason = "the PocketIC decoder mirrors the direct Root status wire"
    )]
    enum RootStatusResponseFragment {
        #[cfg(test)]
        Admission(FleetAdmissionRootStatusResponse),
        #[cfg(test)]
        AuthorityRestore(AuthorityRestoreFenceStatusResponse),
        #[cfg(test)]
        ComponentRegistry(RootComponentRegistryStatusResponse),
        #[cfg(test)]
        ComponentRegistryActivePartition(ComponentRegistryActivePartitionResponse),
        #[cfg(test)]
        ComponentRegistryPartition(
            canic::dto::component_registry::ComponentRegistryPartitionResponse,
        ),
        #[cfg(test)]
        CycleBalance(CycleBalanceStatusResponse),
        #[cfg(test)]
        CycleHistory(Page<CycleTrackerEntry>),
        FleetAuthority(FleetSubnetRootAuthority),
        #[cfg(test)]
        Funding(RootFundingStatusResponse),
        Inventory(FleetSubnetRootCanisterSummary),
        Operation(RootOperationStatusResponse),
        Pool(CanisterPoolResponse),
        #[cfg(test)]
        Metrics(Page<MetricEntry>),
        #[cfg(test)]
        Runtime(Box<CanicRuntimeStatus>),
    }

    #[derive(CandidType)]
    enum ManagedStatusRequestFragment {
        #[cfg_attr(
            not(test),
            expect(
                dead_code,
                reason = "binding status is exercised by the governed test build"
            )
        )]
        Binding,
        #[cfg(test)]
        CycleHistory(PageRequest),
        Operation(OperationStatusRequest),
    }

    #[derive(CandidType, Deserialize)]
    enum ManagedStatusResponseFragment {
        Binding(Box<ManagedCanisterBinding>),
        #[cfg(test)]
        CycleHistory(Page<CycleTrackerEntry>),
        Operation(Box<ManagedOperationStatusResponseFragment>),
    }

    #[cfg(test)]
    #[derive(CandidType)]
    enum ManagedAdmissionStatusRequestFragment {
        Admission(PageRequest),
    }

    #[cfg(test)]
    #[derive(CandidType, Debug, Deserialize)]
    enum ManagedAdmissionStatusResponseFragment {
        Admission(FleetAdmissionProjectionStatusResponse),
    }

    #[derive(CandidType, Debug, Deserialize)]
    enum ManagedOperationStatusResponseFragment {
        ConfigureRuntime(ComponentRuntimeOperationStatus),
    }

    #[derive(CandidType)]
    enum RoleOverviewStatusRequestFragment {
        Overview,
    }

    #[derive(CandidType, Deserialize)]
    enum RoleOverviewStatusResponseFragment {
        Overview(RoleOverviewResponse),
    }

    #[derive(Debug)]
    enum RoleOverviewReadinessObservation {
        Pending {
            phase: String,
            last_error: Option<String>,
        },
        Ready,
        Rejected(Error),
    }

    impl RoleOverviewReadinessObservation {
        const fn is_ready(&self) -> bool {
            matches!(self, Self::Ready)
        }
    }

    impl fmt::Display for RoleOverviewReadinessObservation {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Pending { phase, last_error } => {
                    write!(
                        formatter,
                        "pending phase={phase:?} last_error={last_error:?}"
                    )
                }
                Self::Ready => formatter.write_str("ready"),
                Self::Rejected(error) => write!(formatter, "rejected error={error:?}"),
            }
        }
    }

    fn root_command(
        pic: &PocketIc,
        root: Principal,
        command: RootCommandFragment,
    ) -> Result<RootCommandResponseFragment, Error> {
        root_command_as(pic, root, Principal::anonymous(), command)
    }

    fn root_command_as(
        pic: &PocketIc,
        root: Principal,
        caller: Principal,
        command: RootCommandFragment,
    ) -> Result<RootCommandResponseFragment, Error> {
        pic.update_candid_as(
            root,
            caller,
            canic::protocol::CANIC_ROOT_COMMAND,
            (command,),
        )
        .expect("Root command transport")
    }

    #[cfg(test)]
    fn request_descendant_funding(
        pic: &PocketIc,
        root: Principal,
        descendant: Principal,
        request: canic::dto::capability::RootCapabilityEnvelopeV1,
    ) -> u128 {
        let canic::dto::rpc::CyclesResponse::Transferred { cycles_transferred } =
            descendant_funding_response(pic, root, descendant, request)
        else {
            panic!("Root returned a funding preflight rejection");
        };
        cycles_transferred
    }

    #[cfg(test)]
    fn descendant_funding_response(
        pic: &PocketIc,
        root: Principal,
        descendant: Principal,
        request: canic::dto::capability::RootCapabilityEnvelopeV1,
    ) -> canic::dto::rpc::CyclesResponse {
        let response = root_command_as(
            pic,
            root,
            descendant,
            RootCommandFragment::RespondCapability(request),
        )
        .expect("Root accepts registered descendant funding request");
        let RootCommandResponseFragment::RespondCapability(response) = response else {
            panic!("Root returned a differently correlated capability response");
        };
        let canic::dto::rpc::Response::Cycles(response) = response.response else {
            panic!("Root returned a differently correlated cycles response");
        };
        response
    }

    fn root_status(
        pic: &PocketIc,
        root: Principal,
        request: RootStatusRequestFragment,
    ) -> Result<RootStatusResponseFragment, Error> {
        pic.query_candid(
            root,
            match &request {
                RootStatusRequestFragment::Operation(_) => {
                    canic::protocol::CANIC_ROOT_OPERATION_STATUS
                }
                #[cfg(test)]
                RootStatusRequestFragment::CycleBalance
                | RootStatusRequestFragment::CycleHistory(_)
                | RootStatusRequestFragment::Metrics(_)
                | RootStatusRequestFragment::Runtime => canic::protocol::CANIC_OBSERVABILITY,
                _ => canic::protocol::CANIC_ROOT_STATUS,
            },
            (request,),
        )
        .expect("Root status transport")
    }

    #[cfg(test)]
    fn controller_authority_unavailable<T>(result: &Result<T, Error>) -> bool {
        matches!(
            result,
            Err(error)
                if error.code()
                    == canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        )
    }

    #[cfg(test)]
    fn application_rejection<T>(result: Result<T, Error>, context: &str) -> Error {
        match result {
            Err(error) => error,
            Ok(_) => panic!("{context}"),
        }
    }

    #[cfg(test)]
    fn validate_root_sensitive_observation<F>(
        pic: &PocketIc,
        root: Principal,
        outsider: Principal,
        request: F,
        accepted: fn(&RootStatusResponseFragment) -> bool,
    ) -> Result<(), ActiveComponentRegistryBaselineError>
    where
        F: Fn() -> RootStatusRequestFragment,
    {
        let denied: Result<RootStatusResponseFragment, Error> = pic.query_candid_as(
            root,
            outsider,
            canic::protocol::CANIC_OBSERVABILITY,
            (request(),),
        )?;
        if !controller_authority_unavailable(&denied) {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Root sensitive observability accepted a non-controller".to_string(),
            ));
        }
        let response = baseline_application_result(
            root_status(pic, root, request()),
            "query controller-authenticated Root observability",
        )?;
        if !accepted(&response) {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Root sensitive observability returned the wrong response variant".to_string(),
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    fn validate_store_sensitive_observation<F>(
        pic: &PocketIc,
        store: Principal,
        controller: Principal,
        outsider: Principal,
        request: F,
        accepted: fn(&StoreObservabilityResponse) -> bool,
    ) -> Result<(), ActiveComponentRegistryBaselineError>
    where
        F: Fn() -> StoreObservabilityRequest,
    {
        let denied: Result<StoreObservabilityResponse, Error> = pic.query_candid_as(
            store,
            outsider,
            canic::protocol::CANIC_OBSERVABILITY,
            (request(),),
        )?;
        if !controller_authority_unavailable(&denied) {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Store sensitive observability accepted a non-controller".to_string(),
            ));
        }
        let response: Result<StoreObservabilityResponse, Error> = pic.query_candid_as(
            store,
            controller,
            canic::protocol::CANIC_OBSERVABILITY,
            (request(),),
        )?;
        let response = baseline_application_result(
            response,
            "query controller-authenticated Store observability",
        )?;
        if !accepted(&response) {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Store sensitive observability returned the wrong response variant".to_string(),
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    fn validate_sensitive_observability_authority(
        pic: &PocketIc,
        metadata: &ActiveComponentRegistryBaselineMetadata,
    ) -> Result<(), ActiveComponentRegistryBaselineError> {
        let outsider = Principal::from_slice(&[0x7f; 29]);
        let page = || PageRequest {
            limit: 1,
            offset: 0,
        };

        validate_root_sensitive_observation(
            pic,
            metadata.root,
            outsider,
            || RootStatusRequestFragment::CycleBalance,
            matches_root_cycle_balance,
        )?;
        validate_root_sensitive_observation(
            pic,
            metadata.root,
            outsider,
            || RootStatusRequestFragment::CycleHistory(page()),
            matches_root_cycle_history,
        )?;
        validate_root_sensitive_observation(
            pic,
            metadata.root,
            outsider,
            || {
                RootStatusRequestFragment::Metrics(MetricsStatusRequest {
                    kind: MetricsKind::Runtime,
                    page: page(),
                })
            },
            matches_root_metrics,
        )?;
        validate_store_sensitive_observation(
            pic,
            metadata.wasm_store,
            metadata.root,
            outsider,
            || StoreObservabilityRequest::CycleBalance,
            matches_store_cycle_balance,
        )?;
        validate_store_sensitive_observation(
            pic,
            metadata.wasm_store,
            metadata.root,
            outsider,
            || StoreObservabilityRequest::CycleHistory(page()),
            matches_store_cycle_history,
        )?;

        let direct_denied: Result<ManagedStatusResponseFragment, Error> = pic.query_candid_as(
            metadata.issuer.canister_id,
            outsider,
            canic::protocol::CANIC_OBSERVABILITY,
            (ManagedStatusRequestFragment::CycleHistory(page()),),
        )?;
        if !controller_authority_unavailable(&direct_denied) {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "managed cycle history accepted a non-controller".to_string(),
            ));
        }

        let relay_request = || {
            RootCommandFragment::ObserveCanister(FleetCanisterObservabilityRequest {
                canister_id: metadata.issuer.canister_id,
                request: CanisterObservabilityRequest::CycleHistory(page()),
            })
        };
        let relay_denied: Result<RootCommandResponseFragment, Error> = pic.update_candid_as(
            metadata.root,
            outsider,
            canic::protocol::CANIC_ROOT_COMMAND,
            (relay_request(),),
        )?;
        if !controller_authority_unavailable(&relay_denied) {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Root observability relay accepted a non-controller".to_string(),
            ));
        }
        let relayed: Result<RootCommandResponseFragment, Error> = pic.update_candid(
            metadata.root,
            canic::protocol::CANIC_ROOT_COMMAND,
            (relay_request(),),
        )?;
        if !matches!(
            baseline_application_result(relayed, "relay managed cycle history through Root")?,
            RootCommandResponseFragment::ObserveCanister(
                CanisterObservabilityResponse::CycleHistory(_)
            )
        ) {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Root observability relay returned the wrong response variant".to_string(),
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    const fn matches_root_cycle_balance(response: &RootStatusResponseFragment) -> bool {
        matches!(response, RootStatusResponseFragment::CycleBalance(_))
    }

    #[cfg(test)]
    const fn matches_root_cycle_history(response: &RootStatusResponseFragment) -> bool {
        matches!(response, RootStatusResponseFragment::CycleHistory(_))
    }

    #[cfg(test)]
    const fn matches_root_metrics(response: &RootStatusResponseFragment) -> bool {
        matches!(response, RootStatusResponseFragment::Metrics(_))
    }

    #[cfg(test)]
    const fn matches_store_cycle_balance(response: &StoreObservabilityResponse) -> bool {
        matches!(response, StoreObservabilityResponse::CycleBalance(_))
    }

    #[cfg(test)]
    const fn matches_store_cycle_history(response: &StoreObservabilityResponse) -> bool {
        matches!(response, StoreObservabilityResponse::CycleHistory(_))
    }

    #[cfg(test)]
    fn managed_binding_status(
        pic: &PocketIc,
        root: Principal,
        canister: Principal,
    ) -> ManagedCanisterBinding {
        let response: Result<ManagedStatusResponseFragment, Error> = pic
            .query_candid_as(
                canister,
                root,
                canic::protocol::CANIC_OBSERVABILITY,
                (ManagedStatusRequestFragment::Binding,),
            )
            .expect("managed binding status transport");
        let ManagedStatusResponseFragment::Binding(binding) =
            response.expect("Root reads exact managed binding")
        else {
            panic!("managed canister returned a differently correlated status");
        };
        *binding
    }

    #[cfg(test)]
    fn root_admission_catalog_authority(
        pic: &PocketIc,
        root: Principal,
        successor: &FleetAdmissionPolicy,
    ) -> canic_core::shared_support::fleet_admission_authority::FleetAdmissionRootCatalogAuthorityModel{
        let RootStatusResponseFragment::Admission(status) = root_status(
            pic,
            root,
            RootStatusRequestFragment::Admission(PageRequest {
                limit: 32,
                offset: 0,
            }),
        )
        .expect("query Root admission catalog") else {
            panic!("Root returned a differently correlated admission status");
        };
        assert!(status.operation_id.is_none());
        assert!(status.phase.is_none());
        assert_eq!(
            usize::try_from(status.participants.total).expect("participant total fits usize"),
            status.participants.entries.len()
        );
        let projections = status
            .participants
            .entries
            .iter()
            .map(|participant| {
                let selector = fleet_admission_target_for_binding(&participant.target);
                let principals = effective_fleet_admission_principals(successor, &selector);
                materialize_fleet_admission_projection(
                    successor,
                    participant.target.clone(),
                    principals,
                )
                .expect("compile successor participant projection")
            })
            .collect::<Vec<_>>();
        canic_core::shared_support::fleet_admission_authority::FleetAdmissionRootCatalogAuthorityModel {
            fleet_subnet_root: root,
            participant_catalog_digest: fleet_admission_root_participant_catalog_digest(
                &projections,
            ),
            participant_count: u32::try_from(status.participants.total)
                .expect("Root admission participant count fits u32"),
        }
    }

    #[cfg(test)]
    fn root_admission_catalog_authorities(
        pic: &PocketIc,
        roots: &[Principal],
        successor: &FleetAdmissionPolicy,
    ) -> Vec<canic_core::shared_support::fleet_admission_authority::FleetAdmissionRootCatalogAuthorityModel>{
        let mut catalogs = roots
            .iter()
            .map(|root| root_admission_catalog_authority(pic, *root, successor))
            .collect::<Vec<_>>();
        catalogs.sort_unstable_by(|left, right| {
            left.fleet_subnet_root
                .as_slice()
                .cmp(right.fleet_subnet_root.as_slice())
        });
        catalogs
    }

    #[cfg(test)]
    fn admission_participant_catalog_authority(
        pic: &PocketIc,
        roots: &[Principal],
        successor: &FleetAdmissionPolicy,
    ) -> ([u8; 32], u32) {
        let catalogs = root_admission_catalog_authorities(pic, roots, successor);
        let participant_count = catalogs
            .iter()
            .try_fold(0_u32, |total, catalog| {
                total.checked_add(catalog.participant_count)
            })
            .expect("Fleet admission participant count fits u32");
        (
            fleet_admission_participant_catalog_digest(&catalogs),
            participant_count,
        )
    }

    #[cfg(test)]
    fn root_pool_status(pic: &PocketIc, root: Principal) -> CanisterPoolResponse {
        root_pool_status_as(pic, root, Principal::anonymous())
    }

    fn root_pool_status_as(
        pic: &PocketIc,
        root: Principal,
        caller: Principal,
    ) -> CanisterPoolResponse {
        let response: Result<RootStatusResponseFragment, Error> = pic
            .query_candid_as(
                root,
                caller,
                canic::protocol::CANIC_ROOT_STATUS,
                (RootStatusRequestFragment::Pool(CanisterPoolStatusRequest {
                    start_after: None,
                    limit: 256,
                }),),
            )
            .expect("query Canister pool transport");
        let RootStatusResponseFragment::Pool(status) = response.expect("query Canister pool")
        else {
            panic!("Root returned a differently correlated pool status");
        };
        status
    }

    fn wait_for_role_overviews_ready<I, L>(
        pic: &PocketIc,
        targets: I,
        tick_limit: usize,
        context: &str,
    ) -> Result<(), ActiveComponentRegistryBaselineError>
    where
        I: IntoIterator<Item = (L, Principal, Principal)>,
        L: Into<String>,
    {
        let targets = targets
            .into_iter()
            .map(|(label, canister_id, diagnostic_sender)| {
                (label.into(), canister_id, diagnostic_sender)
            })
            .collect::<Vec<_>>();
        let mut observations = Vec::with_capacity(targets.len());
        for _ in 0..tick_limit {
            observations.clear();
            let mut query_failures = Vec::new();
            for (label, canister_id, _) in &targets {
                match fetch_role_overview_readiness(pic, *canister_id) {
                    Ok(observation) => observations.push((label, *canister_id, observation)),
                    Err(error) => query_failures.push(RoleOverviewQueryFailure {
                        label: label.clone(),
                        canister_id: *canister_id,
                        error,
                    }),
                }
            }
            if !query_failures.is_empty() {
                report_canister_diagnostics_batch(
                    pic,
                    targets
                        .iter()
                        .map(|(label, canister_id, diagnostic_sender)| {
                            (label.clone(), *canister_id, *diagnostic_sender)
                        }),
                    context,
                );
                return Err(ActiveComponentRegistryBaselineError::Calls {
                    context: context.to_string(),
                    failures: query_failures,
                });
            }
            if observations
                .iter()
                .all(|(_, _, observation)| observation.is_ready())
            {
                return Ok(());
            }
            pic.tick();
        }

        report_canister_diagnostics_batch(
            pic,
            targets
                .iter()
                .zip(&observations)
                .filter(|(_, (_, _, observation))| !observation.is_ready())
                .map(|((label, canister_id, diagnostic_sender), _)| {
                    (label.clone(), *canister_id, *diagnostic_sender)
                }),
            context,
        );
        let detail = observations
            .iter()
            .filter(|(_, _, observation)| !observation.is_ready())
            .map(|(label, canister_id, observation)| {
                format!("{label}({canister_id})={observation}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        Err(ActiveComponentRegistryBaselineError::Invariant(format!(
            "{context}: role overviews did not become ready after {tick_limit} ticks: {detail}"
        )))
    }

    fn fetch_role_overview_readiness(
        pic: &PocketIc,
        canister_id: Principal,
    ) -> Result<RoleOverviewReadinessObservation, CandidCallError> {
        match pic.query_candid::<Result<RoleOverviewStatusResponseFragment, Error>, _>(
            canister_id,
            canic::protocol::CANIC_PUBLIC_STATUS,
            (RoleOverviewStatusRequestFragment::Overview,),
        ) {
            Ok(Ok(RoleOverviewStatusResponseFragment::Overview(overview))) => {
                if overview.bootstrap.ready {
                    Ok(RoleOverviewReadinessObservation::Ready)
                } else {
                    Ok(RoleOverviewReadinessObservation::Pending {
                        phase: overview.bootstrap.phase,
                        last_error: overview.bootstrap.last_error,
                    })
                }
            }
            Ok(Err(error)) => Ok(RoleOverviewReadinessObservation::Rejected(error)),
            Err(error) => Err(error),
        }
    }

    fn coordinator_command(
        pic: &PocketIc,
        coordinator: Principal,
        command: CoordinatorCommand,
    ) -> Result<CoordinatorCommandResponse, Error> {
        pic.update_candid(
            coordinator,
            canic::protocol::CANIC_COORDINATOR_COMMAND,
            (command,),
        )
        .expect("Coordinator command transport")
    }

    fn coordinator_status<R: crate::pic::canic::CoordinatorRead>(
        pic: &PocketIc,
        coordinator: Principal,
        request: R,
    ) -> Result<R::Response, Error> {
        pic.query_candid(coordinator, R::METHOD, (request,))
            .expect("Coordinator status transport")
    }

    #[cfg(test)]
    fn await_fleet_admission_convergence(
        pic: &PocketIc,
        coordinator: Principal,
        operation_id: [u8; 32],
    ) -> FleetAdmissionMutationResponse {
        await_fleet_admission_convergence_as(pic, coordinator, operation_id, Principal::anonymous())
    }

    #[cfg(test)]
    fn await_fleet_admission_convergence_as(
        pic: &PocketIc,
        coordinator: Principal,
        operation_id: [u8; 32],
        caller: Principal,
    ) -> FleetAdmissionMutationResponse {
        let mut last_phase = String::new();
        for _ in 0..128 {
            let status: Result<CoordinatorOperationReadResponse, Error> = pic
                .query_candid_as(
                    coordinator,
                    caller,
                    canic::protocol::CANIC_COORDINATOR_OPERATION_STATUS,
                    (CoordinatorOperationReadRequest::Operation(
                        OperationStatusRequest { operation_id },
                    ),),
                )
                .expect("query Fleet admission transport");
            let status = status.expect("query Fleet admission operation");
            let CoordinatorOperationReadResponse::Operation(
                CoordinatorOperationStatusResponse::Admission(operation),
            ) = status
            else {
                panic!("Coordinator returned a differently correlated admission operation")
            };
            let observed_phase = format!("{:?}", operation.phase);
            if observed_phase != last_phase {
                eprintln!("[pic_fleet_admission] Coordinator phase={observed_phase}");
                last_phase = observed_phase;
            }
            if let FleetAdmissionOperationPhase::Completed(response) = operation.phase {
                return response;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        report_canister_diagnostics_batch(
            pic,
            [("coordinator", coordinator, Principal::anonymous())],
            "Fleet admission convergence timeout",
        );
        panic!("Fleet admission operation did not converge; last phase={last_phase}")
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one proof restarts every retained Coordinator and Root transition boundary"
    )]
    fn await_fleet_admission_convergence_across_coordinator_restarts(
        pic: &PocketIc,
        coordinator: Principal,
        roots: &[Principal],
        targets: &[Principal],
        operation_id: [u8; 32],
    ) -> FleetAdmissionMutationResponse {
        assert_eq!(roots.len(), targets.len());
        let mut restarted = std::collections::BTreeSet::new();
        let mut restarted_roots = std::collections::BTreeSet::new();
        let mut stopped_targets = std::collections::BTreeSet::new();
        for _ in 0..192 {
            let status = coordinator_status(
                pic,
                coordinator,
                CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .expect("query interrupted Fleet admission operation");
            let CoordinatorOperationReadResponse::Operation(
                CoordinatorOperationStatusResponse::Admission(operation),
            ) = status
            else {
                panic!("Coordinator returned a differently correlated admission operation")
            };
            let boundary = match &operation.phase {
                FleetAdmissionOperationPhase::Preparing { .. } => Some("preparing"),
                FleetAdmissionOperationPhase::Releasing { .. } => Some("releasing"),
                FleetAdmissionOperationPhase::PerimeterFenced { .. } => Some("perimeter_fenced"),
                FleetAdmissionOperationPhase::Activating { .. } => Some("activating"),
                FleetAdmissionOperationPhase::Opening { .. } => Some("opening"),
                FleetAdmissionOperationPhase::Planned { .. } => None,
                FleetAdmissionOperationPhase::Completed(response) => {
                    assert_eq!(
                        restarted,
                        std::collections::BTreeSet::from([
                            "activating",
                            "opening",
                            "perimeter_fenced",
                            "preparing",
                        ])
                    );
                    for root in roots {
                        for boundary in ["perimeter_fenced", "activating", "opening"] {
                            assert!(
                                restarted_roots.contains(&(*root, boundary)),
                                "Root {root} was not restarted at {boundary}"
                            );
                        }
                    }
                    for boundary in ["preparing", "perimeter_fenced", "activating", "opening"] {
                        assert!(
                            restarted_roots
                                .iter()
                                .any(|(_, observed)| *observed == boundary),
                            "no Root was restarted at {boundary}"
                        );
                    }
                    return response.clone();
                }
            };
            if let Some(boundary) = boundary
                && restarted.insert(boundary)
            {
                if boundary == "perimeter_fenced" {
                    for (root, target) in roots.iter().zip(targets) {
                        pic.stop_canister(*target, Some(*root))
                            .expect("hold target before Root activation begins");
                        stopped_targets.insert(*target);
                    }
                }
                pic.stop_canister(coordinator, None)
                    .expect("stop Coordinator at retained admission boundary");
                pic.advance_time(Duration::from_secs(1));
                pic.tick();
                pic.start_canister(coordinator, None)
                    .expect("restart Coordinator at retained admission boundary");
            }
            for (root, target) in roots.iter().zip(targets) {
                let response = root_status(
                    pic,
                    *root,
                    RootStatusRequestFragment::Admission(PageRequest {
                        limit: 1,
                        offset: 0,
                    }),
                );
                let Ok(RootStatusResponseFragment::Admission(status)) = response else {
                    continue;
                };
                if status.operation_id != Some(operation_id) {
                    continue;
                }
                let boundary = match status.phase {
                    Some(FleetAdmissionRootTransitionPhase::Preparing) => Some("preparing"),
                    Some(FleetAdmissionRootTransitionPhase::PerimeterFenced) => {
                        Some("perimeter_fenced")
                    }
                    Some(FleetAdmissionRootTransitionPhase::Activating) => Some("activating"),
                    Some(FleetAdmissionRootTransitionPhase::Opening) => Some("opening"),
                    Some(
                        FleetAdmissionRootTransitionPhase::Converged
                        | FleetAdmissionRootTransitionPhase::Released,
                    )
                    | None => None,
                };
                if let Some(boundary) = boundary
                    && restarted_roots.insert((*root, boundary))
                {
                    pic.stop_canister(*root, None)
                        .expect("stop Root at retained admission boundary");
                    pic.start_canister(*root, None)
                        .expect("restart Root at retained admission boundary");
                    if boundary == "activating" && stopped_targets.remove(target) {
                        pic.start_canister(*target, Some(*root))
                            .expect("release target after retained Root activation boundary");
                    }
                }
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        panic!("interrupted Fleet admission operation did not converge")
    }

    #[cfg(test)]
    fn managed_admission_status(
        pic: &PocketIc,
        canister_id: Principal,
        root: Principal,
    ) -> FleetAdmissionProjectionStatusResponse {
        let response: Result<ManagedAdmissionStatusResponseFragment, Error> = pic
            .query_candid_as(
                canister_id,
                root,
                canic::protocol::CANIC_ADMISSION_STATUS,
                (ManagedAdmissionStatusRequestFragment::Admission(
                    PageRequest {
                        limit: 128,
                        offset: 0,
                    },
                ),),
            )
            .expect("managed admission status transport");
        let ManagedAdmissionStatusResponseFragment::Admission(status) =
            response.expect("managed admission status");
        status
    }

    fn store_command_as(
        pic: &PocketIc,
        store: Principal,
        caller: Principal,
        command: StoreCommand,
    ) -> Result<StoreCommandResponse, Error> {
        pic.update_candid_as(
            store,
            caller,
            canic::protocol::CANIC_WASM_STORE_COMMAND,
            (command,),
        )
        .expect("Store command transport")
    }

    fn store_prepare_as(
        pic: &PocketIc,
        store: Principal,
        caller: Principal,
        request: TemplateChunkSetPrepareInput,
    ) -> Result<TemplateChunkSetInfoResponse, Error> {
        let response =
            store_command_as(pic, store, caller, StoreCommand::PrepareChunkSet(request))?;
        let StoreCommandResponse::PrepareChunkSet(prepared) = response else {
            panic!("Store returned a differently correlated prepare response");
        };
        Ok(prepared)
    }

    fn store_stage_manifest_as(
        pic: &PocketIc,
        store: Principal,
        caller: Principal,
        request: TemplateManifestInput,
    ) -> Result<(), Error> {
        let response = store_command_as(pic, store, caller, StoreCommand::StageManifest(request))?;
        let StoreCommandResponse::StageManifest = response else {
            panic!("Store returned a differently correlated manifest response");
        };
        Ok(())
    }

    ///
    /// ActiveComponentRegistryFixture
    ///
    /// Coordinator-anchored Fleet fixture whose root and two Components are
    /// active under current Component Registry authority.
    ///
    pub struct ActiveComponentRegistryFixture {
        runtime: ActiveComponentRegistryRuntime,
        pub coordinator: Principal,
        pub root: Principal,
        pub issuer: ComponentBinding,
        pub verifier: ComponentBinding,
        issuer_runtime_operation_id: [u8; 32],
        verifier_runtime_operation_id: [u8; 32],
        store_bootstrap: RootStoreBootstrapRequest,
        wasm_store: Principal,
        pool_assets: Vec<Principal>,
    }

    impl ActiveComponentRegistryFixture {
        /// Borrow the live PocketIC instance.
        #[must_use]
        pub fn pic(&self) -> &PocketIc {
            match &self.runtime {
                ActiveComponentRegistryRuntime::Fresh(pic) => pic,
                ActiveComponentRegistryRuntime::Pooled(baseline) => baseline.pocket_ic(),
            }
        }

        /// Start the HTTP gateway for a fresh, exclusively owned fixture.
        ///
        /// # Panics
        ///
        /// Panics when called for an immutable pooled fixture.
        #[must_use]
        pub fn start_http_gateway(&mut self) -> String {
            let ActiveComponentRegistryRuntime::Fresh(pic) = &mut self.runtime else {
                panic!("the HTTP gateway requires a fresh Component Registry fixture")
            };
            pic.make_live(None).to_string()
        }

        /// Return the exact issuer Wasm installed by this fixture.
        ///
        /// # Panics
        /// Panics if the admitted issuer has no built artifact.
        #[must_use]
        pub fn issuer_wasm(&self) -> Vec<u8> {
            build_test_component_wasms()
                .get(&self.issuer.role)
                .expect("fixture issuer artifact")
                .clone()
        }

        /// Return the exact configured verifier Wasm installed by this fixture.
        ///
        /// # Panics
        ///
        /// Panics when the fixture's admitted verifier role has no built artifact.
        #[must_use]
        pub fn verifier_wasm(&self) -> Vec<u8> {
            build_test_component_wasms()
                .get(&self.verifier.role)
                .expect("fixture verifier artifact")
                .clone()
        }
    }

    enum ActiveComponentRegistryRuntime {
        Fresh(Box<PocketIc>),
        Pooled(CachedPocketIcBaselinePoolGuard<'static, ActiveComponentRegistryBaselineRecipe>),
    }

    #[derive(Clone)]
    struct ActiveComponentRegistryBaselineMetadata {
        coordinator: Principal,
        root: Principal,
        issuer: ComponentBinding,
        verifier: ComponentBinding,
        issuer_runtime_operation_id: [u8; 32],
        verifier_runtime_operation_id: [u8; 32],
        store_bootstrap: RootStoreBootstrapRequest,
        wasm_store: Principal,
        pool_assets: Vec<Principal>,
    }

    struct ActiveComponentRegistryBaselineRecipe {
        id: FixtureRecipeId,
        reset_requirements: ResetRequirements,
    }

    #[derive(Debug)]
    enum ActiveComponentRegistryBaselineError {
        Call(CandidCallError),
        Calls {
            context: String,
            failures: Vec<RoleOverviewQueryFailure>,
        },
        Contract(BaselinePoolContractError),
        Invariant(String),
        Snapshot(ControllerSnapshotError),
    }

    #[derive(Debug)]
    struct RoleOverviewQueryFailure {
        label: String,
        canister_id: Principal,
        error: CandidCallError,
    }

    impl ActiveComponentRegistryBaselineRecipe {
        fn new() -> Result<Self, BaselinePoolContractError> {
            Ok(Self {
                id: FixtureRecipeId::try_new("canic/active-component-registry/v1")?,
                reset_requirements: ResetRequirements::try_new([
                    ResetRequirement::CanisterSnapshots,
                    ResetRequirement::CanisterCycles(CycleResetPolicy::TopUpTo(
                        crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
                    )),
                    ResetRequirement::PocketIcTime(TimeResetPolicy::PreserveCurrent),
                ])?,
            })
        }
    }

    impl PocketIcBaselineRecipe for ActiveComponentRegistryBaselineRecipe {
        type Metadata = ActiveComponentRegistryBaselineMetadata;
        type Error = ActiveComponentRegistryBaselineError;

        fn id(&self) -> &FixtureRecipeId {
            &self.id
        }

        fn reset_requirements(&self) -> &ResetRequirements {
            &self.reset_requirements
        }

        fn build(&self) -> Result<CachedPocketIcBaseline<Self::Metadata>, Self::Error> {
            let ActiveComponentRegistryFixture {
                runtime,
                coordinator,
                root,
                issuer,
                verifier,
                issuer_runtime_operation_id,
                verifier_runtime_operation_id,
                store_bootstrap,
                wasm_store,
                pool_assets,
            } = setup_active_component_registry_fresh();
            let ActiveComponentRegistryRuntime::Fresh(pic) = runtime else {
                unreachable!("fresh baseline builder must own its PocketIC instance")
            };
            let metadata = ActiveComponentRegistryBaselineMetadata {
                coordinator,
                root,
                issuer,
                verifier,
                issuer_runtime_operation_id,
                verifier_runtime_operation_id,
                store_bootstrap,
                wasm_store,
                pool_assets,
            };
            let snapshot_targets = [
                CanisterSnapshotTarget::new(metadata.coordinator, None),
                CanisterSnapshotTarget::new(metadata.root, None),
                CanisterSnapshotTarget::new(metadata.wasm_store, Some(metadata.root)),
                CanisterSnapshotTarget::new(metadata.issuer.canister_id, Some(metadata.root)),
                CanisterSnapshotTarget::new(metadata.verifier.canister_id, Some(metadata.root)),
            ];
            CachedPocketIcBaseline::capture_with_senders(*pic, snapshot_targets, metadata)
                .map_err(Into::into)
        }

        fn restore_canisters(
            &self,
            baseline: &CachedPocketIcBaseline<Self::Metadata>,
        ) -> Result<CanisterRestoreReceipt, Self::Error> {
            baseline.restore_with_captured_senders_and_funding(
                SnapshotRestoreFunding::TopUpTo {
                    minimum_cycles: crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
                },
            )?;
            CanisterRestoreReceipt::try_from_baseline(
                baseline,
                CycleResetPolicy::TopUpTo(crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES),
            )
            .map_err(Into::into)
        }

        fn reset_non_snapshot_state(
            &self,
            baseline: &CachedPocketIcBaseline<Self::Metadata>,
        ) -> Result<ResetReceipt, Self::Error> {
            reset_unclaimed_pool_assets(baseline)?;
            ResetReceipt::try_new([ResetAchievement::PocketIcTime(
                TimeResetPolicy::PreserveCurrent,
            )])
            .map_err(Into::into)
        }

        fn drive_to_readiness(
            &self,
            baseline: &CachedPocketIcBaseline<Self::Metadata>,
        ) -> Result<ReadinessReceipt, Self::Error> {
            let metadata = baseline.metadata();
            wait_for_role_overviews_ready(
                baseline.pocket_ic(),
                [
                    ("coordinator", metadata.coordinator, Principal::anonymous()),
                    ("root", metadata.root, Principal::anonymous()),
                    ("wasm_store", metadata.wasm_store, metadata.root),
                    ("issuer", metadata.issuer.canister_id, metadata.root),
                    ("verifier", metadata.verifier.canister_id, metadata.root),
                ],
                60,
                "restored active Component Registry baseline",
            )?;
            ReadinessReceipt::try_new("active-fleet-ready").map_err(Into::into)
        }

        fn validate(
            &self,
            baseline: &CachedPocketIcBaseline<Self::Metadata>,
            _preparation: &PreparedBaseline,
        ) -> Result<ValidationReceipt, Self::Error> {
            validate_active_component_registry_baseline(baseline)?;
            ValidationReceipt::try_new(self.id.clone(), "active-fleet-authority-exact")
                .map_err(Into::into)
        }

        fn classify_failure(
            &self,
            stage: BaselinePreparationStage,
            error: &Self::Error,
        ) -> FailureDisposition {
            if is_dead_pocket_ic_transport_error(error) {
                FailureDisposition::Rebuild(RebuildReason::DeadPocketIcTransport)
            } else {
                FailureDisposition::Rebuild(stage.default_rebuild_reason())
            }
        }
    }

    impl From<BaselinePoolContractError> for ActiveComponentRegistryBaselineError {
        fn from(error: BaselinePoolContractError) -> Self {
            Self::Contract(error)
        }
    }

    impl From<CandidCallError> for ActiveComponentRegistryBaselineError {
        fn from(error: CandidCallError) -> Self {
            Self::Call(error)
        }
    }

    impl From<ControllerSnapshotError> for ActiveComponentRegistryBaselineError {
        fn from(error: ControllerSnapshotError) -> Self {
            Self::Snapshot(error)
        }
    }

    impl fmt::Display for ActiveComponentRegistryBaselineError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Call(error) => error.fmt(formatter),
                Self::Calls { context, failures } => {
                    write!(formatter, "{context}: role overview queries failed")?;
                    for failure in failures {
                        write!(
                            formatter,
                            "\n{}({})={:?}",
                            failure.label, failure.canister_id, failure.error
                        )?;
                    }
                    Ok(())
                }
                Self::Contract(error) => error.fmt(formatter),
                Self::Invariant(message) => formatter.write_str(message),
                Self::Snapshot(error) => error.fmt(formatter),
            }
        }
    }

    impl StdError for ActiveComponentRegistryBaselineError {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            match self {
                Self::Call(error) => Some(error),
                Self::Calls { failures, .. } => failures
                    .first()
                    .map(|failure| &failure.error as &(dyn StdError + 'static)),
                Self::Contract(error) => Some(error),
                Self::Invariant(_) => None,
                Self::Snapshot(error) => Some(error),
            }
        }
    }

    struct ActiveComponentBindings {
        issuer: ComponentBinding,
        verifier: ComponentBinding,
        issuer_runtime_operation_id: [u8; 32],
        verifier_runtime_operation_id: [u8; 32],
    }

    impl ActiveComponentBindings {
        const fn new(
            issuer: ComponentBinding,
            verifier: ComponentBinding,
            issuer_runtime_operation_id: [u8; 32],
            verifier_runtime_operation_id: [u8; 32],
        ) -> Self {
            Self {
                issuer,
                verifier,
                issuer_runtime_operation_id,
                verifier_runtime_operation_id,
            }
        }
    }

    struct BootstrappedRootFixture {
        root_id: Principal,
        init_args: FleetSubnetRootInitArgs,
        coordinator_root_funding: FleetCoordinatorRootFundingPolicy,
        request: RootStoreBootstrapRequest,
        response: RootStoreBootstrapResponse,
    }

    struct InstalledRootFixture {
        root_id: Principal,
        init_args: FleetSubnetRootInitArgs,
        coordinator_root_funding: FleetCoordinatorRootFundingPolicy,
        manifest: RootStoreReleaseSetManifest,
        artifacts: BTreeMap<CanisterRole, Vec<u8>>,
        manifest_bytes: Vec<u8>,
        digest: ReleaseSetDigest,
    }

    struct RootStoreFixture {
        wasm: Option<Vec<u8>>,
        manifest: RootStoreReleaseSetManifest,
        artifacts: BTreeMap<CanisterRole, Vec<u8>>,
    }

    #[cfg(test)]
    #[derive(CandidType)]
    struct CyclesLedgerStubInitArgs {
        canister_ids: Vec<Principal>,
        expected_controllers_by_index: Option<Vec<Vec<Principal>>>,
        expected_root: Principal,
        expected_subnet: Principal,
        initial_balances: Option<Vec<CyclesLedgerStubAccountBalance>>,
        pending_first_index: Option<u64>,
        withdrawal_fee: Option<Nat>,
    }

    #[cfg(test)]
    #[derive(CandidType)]
    struct CyclesLedgerStubAccountBalance {
        balance: Nat,
        owner: Principal,
    }

    #[cfg(test)]
    #[derive(CandidType)]
    enum IcpRefillStubInit {
        Ledger {
            balance_e8s: u64,
        },
        Cmc {
            xdr_permyriad_per_icp: u64,
            cycles_per_notify: u128,
        },
    }

    #[cfg(test)]
    #[derive(CandidType, Clone)]
    struct QualificationCreateCanisterArgs {
        from_subaccount: Option<[u8; 32]>,
        created_at_time: Option<u64>,
        amount: Nat,
        creation_args: Option<QualificationCmcCreateCanisterArgs>,
    }

    #[cfg(test)]
    #[derive(CandidType, Clone)]
    struct QualificationCmcCreateCanisterArgs {
        settings: Option<QualificationCanisterSettings>,
        subnet_selection: Option<QualificationSubnetSelection>,
    }

    #[cfg(test)]
    #[derive(CandidType, Clone)]
    struct QualificationCanisterSettings {
        controllers: Option<Vec<Principal>>,
        compute_allocation: Option<Nat>,
        memory_allocation: Option<Nat>,
        freezing_threshold: Option<Nat>,
        reserved_cycles_limit: Option<Nat>,
    }

    #[cfg(test)]
    #[derive(CandidType, Clone)]
    enum QualificationSubnetSelection {
        Subnet { subnet: Principal },
    }

    #[cfg(test)]
    #[derive(CandidType, Debug, Deserialize)]
    struct QualificationCreateCanisterSuccess {
        block_id: Nat,
        canister_id: Principal,
    }

    #[cfg(test)]
    #[derive(CandidType, Debug, Deserialize)]
    enum QualificationCreateCanisterError {
        Duplicate {
            duplicate_of: Nat,
            canister_id: Option<Principal>,
        },
        GenericError {
            message: String,
            error_code: Nat,
        },
    }

    #[cfg(test)]
    #[derive(CandidType)]
    struct QualificationCanisterIdRecord {
        canister_id: Principal,
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "the isolated ICP transport script keeps its lost-response injection and mutation log together"
    )]
    fn prepare_isolated_icp(root: &Path) -> (PathBuf, Principal, PathBuf) {
        let wrapper = root.join("icp-wrapper");
        let mutation_log = root.join("controller-mutations.log");
        std::fs::create_dir_all(root).expect("create isolated ICP root");
        std::fs::write(&mutation_log, []).expect("initialize exact controller mutation log");
        std::fs::write(
            &wrapper,
            r#"#!/bin/sh
set -eu
wrapper_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export XDG_CONFIG_HOME="$wrapper_root/xdg-config"
export XDG_DATA_HOME="$wrapper_root/xdg-data"
export DO_NOT_TRACK=1
case " $* " in
  *" canister call "*" canic_wasm_store_publish_fixture "*)
    icp "$@"
    printf '%s\n' fixture >> "$wrapper_root/fixture-publications.log"
    if [ -f "$wrapper_root/lose-fixture-response" ] && [ ! -e "$wrapper_root/lost-fixture-response" ]; then
      : > "$wrapper_root/lost-fixture-response"
      exit 78
    fi
    exit 0
    ;;
esac
case " $* " in
  *" canister call "*" icrc1_transfer "*)
    if [ -f "$wrapper_root/lose-estate-funding-response" ]; then
      icp "$@"
      if [ ! -e "$wrapper_root/lost-estate-funding-response" ]; then
        : > "$wrapper_root/lost-estate-funding-response"
        exit 74
      fi
      exit 0
    fi
    ;;
esac
case " $* " in
  *" canister call "*" withdraw "*)
    if [ -f "$wrapper_root/fail-before-funding" ] && [ ! -e "$wrapper_root/failed-before-funding" ]; then
      : > "$wrapper_root/failed-before-funding"
      exit 80
    fi
    if [ -f "$wrapper_root/lose-funding-response" ]; then
      icp "$@"
      printf '%s\n' withdrawal >> "$wrapper_root/funding-requests.log"
      if [ ! -e "$wrapper_root/lost-funding-response" ]; then
        : > "$wrapper_root/lost-funding-response"
        exit 73
      fi
      if [ -f "$wrapper_root/lose-funded-observation" ]; then
        : > "$wrapper_root/funded-observation-pending"
      fi
      exit 0
    fi
    ;;
esac
case " $* " in
  *" canister call "*" icrc1_balance_of "*)
    if [ -f "$wrapper_root/funded-observation-pending" ] &&
       [ ! -e "$wrapper_root/lost-funded-observation" ]; then
      : > "$wrapper_root/lost-funded-observation"
      exit 79
    fi
    ;;
esac
case " $* " in
  *" canister call "*" canic_root_command "*)
    previous=""
    for argument in "$@"; do
      if [ "$previous" = "--args-file" ] &&
         [ -f "$wrapper_root/lost-reset-args.bin" ] &&
         cmp -s "$argument" "$wrapper_root/lost-reset-args.bin"; then
        icp "$@"
        printf '%s\n' reset >> "$wrapper_root/reset-mutations.log"
        if [ ! -e "$wrapper_root/lost-reset-response" ]; then
          : > "$wrapper_root/lost-reset-response"
          exit 72
        fi
        exit 0
      fi
      previous="$argument"
    done
    ;;
esac
case " $* " in
  *" canister install "*)
    if [ -f "$wrapper_root/fail-before-root-install" ] && [ ! -e "$wrapper_root/failed-before-root-install" ]; then
      reinstall_root_target=$(cat "$wrapper_root/fail-before-root-install")
      for argument in "$@"; do
        if [ "$argument" = "$reinstall_root_target" ]; then
          : > "$wrapper_root/failed-before-root-install"
          exit 77
        fi
      done
    fi
    if [ -f "$wrapper_root/fail-before-install" ] && [ ! -e "$wrapper_root/failed-before-install" ]; then
      : > "$wrapper_root/failed-before-install"
      exit 76
    fi
    if [ -f "$wrapper_root/lose-install-response" ]; then
      icp "$@"
      printf '%s\n' "$*" >> "$wrapper_root/reinstall-mutations.log"
      if [ ! -e "$wrapper_root/lost-install-response" ]; then
        : > "$wrapper_root/lost-install-response"
        exit 75
      fi
      exit 0
    fi
    ;;
esac
case " $* " in
  *" canister settings update "*)
    icp "$@"
    result=$?
    if [ "$result" -ne 0 ]; then
      exit "$result"
    fi
    printf '%s\n' "$*" >> "$wrapper_root/controller-mutations.log"
    if [ ! -e "$wrapper_root/lost-controller-response" ]; then
      : > "$wrapper_root/lost-controller-response"
      printf '%s\n' 'simulated lost controller-update response' >&2
      exit 71
    fi
    exit 0
    ;;
esac
exec icp "$@"
"#,
        )
        .expect("write isolated ICP wrapper");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mut permissions = std::fs::metadata(&wrapper)
                .expect("read isolated ICP wrapper metadata")
                .permissions();
            permissions.set_mode(0o700);
            std::fs::set_permissions(&wrapper, permissions)
                .expect("make isolated ICP wrapper executable");
        }

        let seed = root.join("identity-seed.txt");
        let created = Command::new(&wrapper)
            .args([
                "identity",
                "new",
                "canic-121",
                "--storage",
                "plaintext",
                "--output-seed",
            ])
            .arg(&seed)
            .output()
            .expect("create isolated ICP identity");
        assert!(
            created.status.success(),
            "isolated ICP identity creation failed: {}",
            String::from_utf8_lossy(&created.stderr)
        );
        let selected = Command::new(&wrapper)
            .args(["identity", "default", "canic-121"])
            .output()
            .expect("select isolated ICP identity");
        assert!(
            selected.status.success(),
            "isolated ICP identity selection failed: {}",
            String::from_utf8_lossy(&selected.stderr)
        );
        let principal = Command::new(&wrapper)
            .args(["identity", "principal"])
            .output()
            .expect("resolve isolated ICP Principal");
        assert!(
            principal.status.success(),
            "isolated ICP Principal failed: {}",
            String::from_utf8_lossy(&principal.stderr)
        );
        let principal = Principal::from_text(
            String::from_utf8(principal.stdout)
                .expect("ICP Principal output UTF-8")
                .trim(),
        )
        .expect("isolated ICP Principal");
        (wrapper, principal, mutation_log)
    }

    #[cfg(test)]
    struct LiteralZeroReleaseArtifacts {
        component_wasms: BTreeMap<CanisterRole, Vec<u8>>,
        coordinator_candid: String,
        coordinator_wasm: String,
        release_build_id: ReleaseBuildId,
        root_candid: String,
        root_wasm: String,
        root_wasm_bytes: Vec<u8>,
        store_candid: String,
        store_wasm: String,
        store_wasm_bytes: Vec<u8>,
    }

    #[cfg(all(test, feature = "governed-pocketic-tests"))]
    #[test]
    #[ignore = "public persistent multi-subnet local Fleet qualification"]
    #[expect(
        clippy::too_many_lines,
        reason = "one public lifecycle journey shares its release artifacts across convergence and restart"
    )]
    fn persistent_local_fleet_converges_two_roots_through_public_host() {
        use canic_host::local_fleet::{model::LocalFleetConfig, workflow::LocalFleetSession};
        let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = workspace.join("canisters/audit/root_probe/local_fleet.toml");
        let config = AppConfigSnapshot::load(&config_path).unwrap();
        let deployment = config
            .model()
            .compile_component_deployment_configuration()
            .unwrap();
        let roles = config
            .model()
            .roles
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let directory = literal_zero_adapter_root(&workspace).join("persistent-local");
        std::fs::create_dir_all(&directory).unwrap();
        let _cleanup = TestDirectoryCleanup(directory.clone());
        let app_directory = directory.join("apps/qualification");
        std::fs::create_dir_all(&app_directory).unwrap();
        let mut app: toml::Value =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        app["roles"]["app"]["package"] = toml::Value::String(
            workspace
                .join("canisters/audit/leaf_probe")
                .to_str()
                .unwrap()
                .into(),
        );
        std::fs::write(
            app_directory.join("canic.toml"),
            toml::to_string(&app).unwrap(),
        )
        .unwrap();
        std::fs::write(directory.join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::write(directory.join("icp.yaml"), "canisters: []\n").unwrap();
        assert_eq!(
            canic_host::config_discovery::discover_canic_workspace_root_from(&directory).unwrap(),
            Some(directory.canonicalize().unwrap())
        );
        let artifacts = build_literal_zero_release_artifacts(
            &workspace,
            &directory,
            &config_path,
            &deployment,
            &roles,
            BuildNetwork::Local,
            INTERNAL_TEST_RELEASE_BUILD_NONCE,
        );
        let (icp, operator, _) = prepare_isolated_icp(&directory);
        let browser = prepare_frontend_identity(&directory, "browser-identity.json");
        let port_guard = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = port_guard.local_addr().unwrap().port();
        drop(port_guard);
        let binary = PathBuf::from(std::env::var_os("POCKET_IC_BIN").expect("pinned PocketIC"));
        let local = LocalFleetConfig {
            schema_version: 1,
            name: "qualification".into(),
            server_binary_sha256: canic_host::local_fleet::ops::binary_sha256(&binary).unwrap(),
            server_binary: binary,
            gateway_port: port,
            application_subnets: 2,
            maximum_canisters: 12,
            canister_memory_bytes: 256 * 1024 * 1024,
            allocation_debit_cycles: 500_000_000_000_000,
            request_timeout_secs: 60,
            server_lifetime_secs: 1200,
        };
        let mut session = LocalFleetSession::open(&directory, &local).unwrap();
        let initial = session.status().unwrap();
        std::fs::write(directory.join("icp.yaml"), format!(
            "canisters: []\nnetworks:\n  - name: local\n    mode: managed\nenvironments:\n  - name: {}\n    network: local\n    canisters: []\n", initial.environment)).unwrap();
        let retained_config = retain_generated_journey_source(&directory, &config_path);
        let policy = local_fleet_two_root_policy(
            operator,
            browser,
            &initial.application_subnets,
            &retained_config,
        );
        let source = directory.join("fleet-policy.toml");
        let seed = directory.join("fleet-seed.toml");
        std::fs::write(&source, policy).unwrap();
        canic_host::fleet_ensure::initialize_fresh_estate_seed(
            &canic_host::fleet_ensure::FreshEstateSeedRequest {
                cycles_ledger: "um5iw-rqaaa-aaaaq-qaaba-cai",
                management_creation_fee_cycles: 1_300_000_000_000,
                seed: &seed,
                source: &source,
            },
        )
        .unwrap();
        let generated = session
            .generate_fleet(&canic_host::fleet_ensure::FleetGenerateRequest {
                catalog_progress: None,
                app_config: &retained_config,
                environment: &initial.environment,
                fleet: "development",
                icp_executable: icp.to_str().unwrap(),
                release_build_id: artifacts.release_build_id,
                root: &directory,
                seed: &seed,
                source: &source,
            })
            .unwrap();
        let report = session
            .converge_fleet(&directory, &generated.desired, icp.to_str().unwrap())
            .unwrap();
        assert!(report.terminal);
        assert!(
            report
                .plan
                .canisters
                .iter()
                .flat_map(|canister| &canister.actions)
                .all(|action| !matches!(action, EnsureAction::Create { .. }))
        );
        let before = session.status().unwrap();
        let discovery = session
            .discover(&directory, &generated.desired.fleet)
            .unwrap();
        let app_subnets = discovery
            .roles
            .iter()
            .filter(|entry| entry.role.as_deref() == Some("app"))
            .map(|entry| entry.subnet_id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(app_subnets.len(), 2);
        assert_local_fleet_browser(&directory, &before, &generated.desired.fleet);
        assert_eq!(
            before
                .canisters
                .iter()
                .filter(|canister| canister.allocation_role == "root")
                .count(),
            2
        );
        assert_eq!(
            before
                .canisters
                .iter()
                .filter(|canister| canister.allocation_role == "wasm_store")
                .count(),
            2
        );
        session.restart(&initial.session_id).unwrap();
        let replay = session
            .converge_fleet(&directory, &generated.desired, icp.to_str().unwrap())
            .unwrap();
        assert!(replay.terminal);
        assert_eq!(session.status().unwrap().gateway, before.gateway);
        assert_local_fleet_browser(
            &directory,
            &session.status().unwrap(),
            &generated.desired.fleet,
        );
        assert_local_fleet_component_growth(&mut session, &directory, &generated.desired, &icp);
        session.shutdown(&initial.session_id).unwrap();
    }

    #[cfg(all(test, feature = "governed-pocketic-tests"))]
    fn local_fleet_two_root_policy(
        operator: Principal,
        browser: Principal,
        subnets: &[Principal],
        config: &Path,
    ) -> String {
        let source = generated_journey_policy(operator, subnets[0], 1, 1, config);
        let mut policy: toml::Value = toml::from_str(&source).unwrap();
        policy["admission"].as_table_mut().unwrap().insert(
            "identity_origin".into(),
            toml::Value::String("http://localhost:5173".into()),
        );
        policy["admission"]["principals"]
            .as_array_mut()
            .unwrap()
            .push(toml::Value::String(browser.to_text()));
        let roots = policy["fleet_subnet_roots"].as_array_mut().unwrap();
        let mut second = roots[0].clone();
        second["placement_subnet"] = toml::Value::String(subnets[1].to_text());
        second["component_group_placements"]["qualification"] =
            toml::Value::Array(vec![toml::Value::Integer(1)]);
        roots.push(second);
        for root in roots {
            root["component_admissions"]["default"] = toml::Value::Integer(2);
            root["limits"]["maximum_component_instances"] = toml::Value::Integer(2);
            root["canister_pool"]["maximum_size"] = toml::Value::Integer(3);
        }
        toml::to_string_pretty(&policy).unwrap()
    }

    #[cfg(all(test, feature = "governed-pocketic-tests"))]
    #[expect(
        clippy::too_many_lines,
        reason = "one real Ensure sequence qualifies Component retry, export and imported local capacity"
    )]
    fn assert_local_fleet_component_growth(
        session: &mut canic_host::local_fleet::workflow::LocalFleetSession,
        directory: &Path,
        source: &DesiredFleet,
        icp: &Path,
    ) {
        use canic_host::component_operation::{ops, workflow};
        let local = session.status().unwrap();
        let funding = Command::new(icp)
            .current_dir(directory)
            .env_remove("ICP_ENVIRONMENT")
            .args([
                "cycles",
                "transfer",
                "20T",
                &source.operator,
                "--identity",
                "anonymous",
                "--network",
                &local.gateway,
                "--root-key",
                &local.root_key_der_hex,
            ])
            .output()
            .unwrap();
        assert!(
            funding.status.success(),
            "fund only the disposable operator from the simulated Ledger: {}",
            String::from_utf8_lossy(&funding.stderr)
        );
        let root_name = source
            .canisters
            .iter()
            .find(|entry| entry.kind == DesiredCanisterKind::Root)
            .unwrap()
            .name
            .clone();
        let spec = "default".parse().unwrap();
        let mut transport = ops::transport::IcpComponentTransport::new(
            directory,
            canic_host::icp::IcpCli::new(icp.to_str().unwrap(), Some(local.environment.clone()))
                .with_local_replica(Some(session.replica_target())),
        );
        let authority = transport
            .authority(&local.environment, &source.fleet, &root_name, &spec)
            .unwrap();
        let plan = workflow::plan(directory, "extra", authority, &mut transport).unwrap();
        let noop = session
            .converge_fleet(directory, source, icp.to_str().unwrap())
            .unwrap();
        assert!(planned_actions(&noop.plan).is_empty());
        assert_ne!(
            noop.plan.plan_sha256,
            plan.plan.authority.source_plan_sha256
        );

        std::fs::write(
            directory.join("root-key.der"),
            canic_core::cdk::utils::hash::decode_hex(&local.root_key_der_hex).unwrap(),
        )
        .unwrap();
        let wrapper = operator_cli_gateway_wrapper(directory, &local.gateway, icp);
        std::fs::write(directory.join("root-key.der.lose-reply"), []).unwrap();
        let interrupted = run_environment_cli(
            directory,
            &wrapper,
            &local.environment,
            &[
                "component",
                "apply",
                &source.fleet,
                "extra",
                "--review",
                &plan.plan.review_sha256,
                "--wait-secs",
                "0",
            ],
        )
        .expect_err("the accepted request loses its reply");
        assert!(
            !directory.join("root-key.der.lose-reply").exists(),
            "Root must accept the request before losing its reply: {interrupted:?}"
        );
        let path = ops::record_path(directory, &local.environment, &source.fleet, "extra").unwrap();
        assert_eq!(ops::read(&path).unwrap().unwrap().submission_attempts, 1);

        // Local builds require explicit supply: create outside the harness allocation journal,
        // then let the selected Root verify and import that capacity through its public command.
        let ledger_icp =
            canic_host::icp::IcpCli::new(icp.to_str().unwrap(), Some(local.environment.clone()))
                .with_cwd(directory)
                .with_local_replica(Some(session.replica_target()));
        let root = plan.plan.authority.binding.fleet_subnet_root;
        let mut creation = qualification_creation_request(
            root,
            *plan.plan.authority.binding.placement_subnet.as_principal(),
            session.status().unwrap().simulated_time_ns.parse().unwrap(),
        );
        creation.amount = Nat::from(10_000_000_000_000_u64);
        let created: Result<QualificationCreateCanisterSuccess, QualificationCreateCanisterError> =
            ledger_icp
                .canister_call_candid(&source.cycles_ledger, "create_canister", &creation, None)
                .unwrap();
        let imported = created.unwrap().canister_id;
        let response: Result<RootCommandResponseFragment, canic_core::dto::error::Error> =
            ledger_icp
                .canister_call_candid(
                    &root.to_text(),
                    canic_core::protocol::CANIC_ROOT_COMMAND,
                    &HostRootCommandFragment::ImportPoolCanister(PoolCanisterRequest {
                        canister_id: imported,
                    }),
                    Some(&directory.join(&source.protocol.as_ref().unwrap().root_candid)),
                )
                .unwrap();
        assert!(
            matches!(response.unwrap(), RootCommandResponseFragment::ImportPoolCanister(PoolImportResponse::Imported { canister_id }) if canister_id == imported)
        );

        // Refresh Fleet authority while the local Component record still holds only its issued intent.
        let mut settled = false;
        for _ in 0..8 {
            match session.converge_fleet(directory, source, icp.to_str().unwrap()) {
                Ok(report) => {
                    assert!(report.terminal);
                    settled = true;
                    break;
                }
                Err(canic_host::local_fleet::LocalFleetError::Ensure(error))
                    if matches!(*error, EnsureWorkflowError::SuccessorReviewRequired { .. }) => {}
                Err(error) => panic!("same-Fleet reconciliation failed: {error:?}"),
            }
        }
        assert!(
            settled,
            "bounded Fleet reviews converge after Component allocation"
        );
        run_environment_cli(
            directory,
            &wrapper,
            &local.environment,
            &[
                "component",
                "apply",
                &source.fleet,
                "extra",
                "--review",
                &plan.plan.review_sha256,
                "--wait-secs",
                "120",
            ],
        )
        .unwrap();
        let completed = ops::read(&path).unwrap().unwrap();
        assert_eq!(completed.plan, plan.plan);
        assert_eq!(
            completed.submission_attempts, 1,
            "observe the accepted allocation without submitting twice"
        );
        let binding = completed
            .progress
            .as_ref()
            .filter(|progress| progress.complete)
            .unwrap()
            .binding
            .as_ref()
            .unwrap();
        let output = directory.join("grown-component.json");
        run_environment_cli(
            directory,
            &wrapper,
            &local.environment,
            &[
                "info",
                "env",
                &source.fleet,
                "--component-operation",
                "extra",
                "--json",
                "--out",
                output.to_str().unwrap(),
            ],
        )
        .unwrap();
        let exported: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
        assert!(
            exported["bindings"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |entry| entry["canister_id"] == binding.canister_id.to_text()
                        && entry["role"] == binding.role.to_string()
                )
        );
        let discovery = session.discover(directory, &source.fleet).unwrap();
        let allocated = local
            .canisters
            .iter()
            .filter_map(|entry| entry.canister_id)
            .collect::<BTreeSet<_>>();
        let added = discovery
            .roles
            .iter()
            .filter(|entry| !allocated.contains(&entry.canister_id))
            .collect::<Vec<_>>();
        assert!(
            added.iter().any(|entry| entry.canister_id == imported),
            "imported Root-owned capacity is outside the named allocations"
        );
        assert!(
            added
                .iter()
                .all(|entry| local.application_subnets.contains(&entry.subnet_id))
        );
        assert_eq!(
            session.status().unwrap().canisters.len(),
            local.canisters.len()
        );
        session.restart(&local.session_id).unwrap();
        let reopened = session.discover(directory, &source.fleet).unwrap();
        assert_eq!(
            reopened
                .roles
                .iter()
                .map(|entry| (entry.canister_id, entry.subnet_id))
                .collect::<BTreeSet<_>>(),
            discovery
                .roles
                .iter()
                .map(|entry| (entry.canister_id, entry.subnet_id))
                .collect::<BTreeSet<_>>()
        );
    }

    #[cfg(all(test, feature = "governed-pocketic-tests"))]
    fn assert_local_fleet_browser(
        directory: &Path,
        local: &canic_host::local_fleet::view::LocalFleetView,
        fleet: &str,
    ) {
        use canic_host::frontend::{model::*, ops, workflow};
        let authority = ops::authority(directory, &local.environment, fleet).unwrap();
        let applications = authority
            .entries
            .iter()
            .filter(|entry| entry.role.as_deref() == Some("app"))
            .collect::<Vec<_>>();
        assert_eq!(applications.len(), 2);
        assert_ne!(applications[0].parent_pid, applications[1].parent_pid);
        let input = FrontendEnvironmentInput {
            schema_version: 1,
            environment: local.environment.clone(),
            canonical_network_id: authority.network,
            api_origin: local.gateway.trim_end_matches('/').into(),
            identity: FrontendIdentityInput {
                canister_id: Principal::from_text("rdmx6-jaaaa-aaaaa-aaadq-cai").unwrap(),
                provider_origin: format!(
                    "http://rdmx6-jaaaa-aaaaa-aaadq-cai.localhost:{}",
                    local
                        .gateway
                        .trim_end_matches('/')
                        .rsplit_once(':')
                        .unwrap()
                        .1
                ),
                derivation_origin: "http://localhost:5173".into(),
                alternative_origins: Vec::new(),
            },
            asset: None,
            roles: applications
                .iter()
                .map(|entry| FrontendRoleInput {
                    role: "app".into(),
                    canister_id: Principal::from_text(&entry.pid).unwrap(),
                })
                .collect(),
        };
        let browser = directory.join("browser");
        let output = if browser.exists() {
            directory.join("browser-restarted")
        } else {
            browser.clone()
        };
        let manifest = workflow::prepare_handoff(directory, fleet, &input, &output).unwrap();
        assert_eq!(
            manifest.local_root_key_der_hex.as_deref(),
            Some(local.root_key_der_hex.as_str())
        );
        ops::verify_bundle(&output, &manifest.manifest_sha256).unwrap();
        let original: FrontendManifestRecord =
            serde_json::from_slice(&std::fs::read(browser.join("canic-frontend.json")).unwrap())
                .unwrap();
        ops::verify_bundle(&browser, &original.manifest_sha256).unwrap();
        assert_eq!(original.roles, manifest.roles);
        assert_eq!(
            original.local_root_key_der_hex,
            manifest.local_root_key_der_hex
        );
        assert_frontend_sdk_call(directory, &original, true);
    }

    #[cfg(test)]
    fn build_literal_zero_release_artifacts(
        workspace_root: &Path,
        adapter_root: &Path,
        config_path: &Path,
        configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
        configured_roles: &[String],
        build_network: BuildNetwork,
        release_nonce: [u8; 32],
    ) -> LiteralZeroReleaseArtifacts {
        let span = Span::start("release_artifact_resolution");
        let mut phase = Span::start("artifact_recipe");
        let release_build_id =
            persist_internal_test_release_build_plan(adapter_root, build_network, release_nonce);
        let fixtures = prepare_generated_fixture_artifacts(
            adapter_root,
            &configuration.component_topology,
            release_build_id,
            configured_roles,
        );
        let mut outputs =
            literal_zero_release_artifact_outputs(adapter_root, release_build_id, configured_roles);
        append_generated_fixture_outputs(adapter_root, &fixtures, &mut outputs);
        let cache = literal_zero_release_artifact_cache_spec(
            workspace_root,
            &workspace_root.join("target/test-artifacts/external-artifact-cache"),
            config_path,
            configured_roles,
            &outputs,
            build_network,
            release_build_id,
        );
        let cache = bind_generated_fixture_cache_inputs(cache, adapter_root, &fixtures);
        phase = phase.next("artifact_cache_lookup");
        let started_at = Instant::now();
        let outcome = match prepare_artifact_cache(&cache)
            .expect("prepare literal-zero release artifact cache")
        {
            ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
            ArtifactCachePreparation::Build(transaction) => {
                phase = phase.next("artifact_build_and_seal");
                build_and_seal_literal_zero_release_artifacts(
                    workspace_root,
                    adapter_root,
                    config_path,
                    configuration,
                    configured_roles,
                    release_build_id,
                    build_network,
                );
                phase = phase.next("artifact_cache_commit");
                for (name, path) in &outputs {
                    transaction
                        .import_output(name, path)
                        .unwrap_or_else(|error| {
                            panic!("import literal-zero release artifact `{name}`: {error}")
                        });
                }
                transaction
                    .commit()
                    .expect("commit literal-zero release artifact cache")
            }
        };
        crate::pic::progress::timed(
            "FLEET",
            if outcome.is_reused() {
                crate::pic::progress::ProgressStatus::Cache
            } else {
                crate::pic::progress::ProgressStatus::Done
            },
            &format!(
                "literal-zero release artifacts ({})",
                outcome.record().artifacts().len()
            ),
            started_at.elapsed(),
        );
        crate::pic::progress::detail(
            "FLEET",
            &format!("literal-zero release artifact cache: {outcome}"),
        );
        report_artifact_cache_maintenance(
            "literal-zero-release-artifacts",
            outcome.record().maintenance(),
        );

        stage_retained_release_artifacts(outcome.record(), &outputs);

        phase = phase.next("artifact_load");
        let artifacts =
            load_literal_zero_release_artifacts(adapter_root, release_build_id, configured_roles);
        phase.finish();
        span.finish();
        artifacts
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one cache recipe binds prepared packages, exact Cargo inputs and complete release outputs"
    )]
    fn literal_zero_release_artifact_cache_spec(
        workspace_root: &Path,
        cache_root: &Path,
        config_path: &Path,
        configured_roles: &[String],
        outputs: &BTreeMap<String, PathBuf>,
        build_network: BuildNetwork,
        release_build_id: ReleaseBuildId,
    ) -> ArtifactCacheSpec {
        let snapshot = AppConfigSnapshot::load(config_path)
            .expect("load literal-zero release build config for Cargo inputs");
        let mut packages = BTreeSet::from(["canic".to_string(), "canic-host".to_string()]);
        if native_funding::uses_audit_root(config_path) {
            packages.insert("root_probe".to_string());
        }
        for role in configured_roles {
            let role = CanisterRole::from(role.clone());
            if role.is_root() {
                continue;
            }
            let RolePackageValidation::Supported(evidence) = validate_declared_role_package(
                config_path,
                snapshot.model(),
                &role,
                PackageValidationMode::Passive,
                &canic_host::role_contract::CargoFeatureSelection::default(),
            ) else {
                panic!("literal-zero role `{role}` must resolve to one supported package");
            };
            packages.insert(evidence.role_package_name);
        }
        let packages = packages.iter().map(String::as_str).collect::<Vec<_>>();
        let network = build_network.to_string();
        let release_identity = release_build_id.to_string();
        let environment = [
            ("CARGO_INCREMENTAL", "0"),
            ("ICP_ENVIRONMENT", network.as_str()),
            (INTERNAL_TEST_RELEASE_BUILD_ID.0, release_identity.as_str()),
        ];
        let cargo_build = WasmBuildSpec::new(
            workspace_root,
            &canic_host::canister_build::canister_build_target_root(workspace_root),
            &packages,
            CanisterBuildProfile::Fast.target_dir_name(),
        )
        .with_cargo_profile_args(["--profile", "fast", "--locked"])
        .with_extra_env(environment);
        let config_relative = config_path
            .strip_prefix(workspace_root)
            .expect("literal-zero config must be workspace-confined")
            .to_str()
            .expect("literal-zero config path UTF-8");
        let mut cache = ArtifactCacheSpec::new(
            cache_root,
            "literal-zero-release-artifacts",
            "canic/literal-zero-release-artifacts/v1",
        )
        .with_coordination_scope("canic-external-artifact-builds")
        .with_arguments([
            "literal-zero-release-build",
            "fast",
            network.as_str(),
            config_relative,
        ])
        .with_environment(environment)
        .with_input(
            "release-build-helper",
            &workspace_root
                .join("crates/canic-testing-internal/src/pic/fleet_registry/baseline.rs"),
        )
        .with_input("build-config", config_path)
        .with_input("icp-config", &workspace_root.join("icp.yaml"))
        .with_prune_policy_at_most_every(
            internal_test_artifact_prune_policy(),
            internal_test_artifact_maintenance_interval(),
        );
        cache = with_canonical_root_cargo_inputs(
            cache,
            config_path,
            &canic_host::canister_build::canister_build_target_root(workspace_root),
            CanicWasmBuildProfile::Fast,
            &environment,
        );
        // All three generated packages live beneath the selected audit package.
        // Materialize their manifests and locks before freezing that package's inputs.
        let context = literal_zero_build_context(
            workspace_root,
            workspace_root,
            config_path,
            build_network,
            release_build_id,
        );
        canic_host::canister_build::prepare_workspace_infrastructure_packages(&context)
            .expect("prepare infrastructure packages before freezing fixture inputs");
        let cargo_inputs = resolve_cargo_build_inputs(&cargo_build)
            .expect("resolve literal-zero release Cargo build inputs");
        cache = cache.with_cargo_build_inputs(
            "literal-zero-release-cargo",
            &cargo_build,
            &cargo_inputs,
        );
        if native_funding::uses_audit_root(config_path) {
            cache = cache.with_input(
                "audit-root-build-helper",
                &workspace_root.join("crates/canic-testing-internal/src/pic/artifacts.rs"),
            );
            cache = cache.with_input("audit-root-builder", &workspace_root.join(
                "crates/canic-testing-internal/src/pic/fleet_registry/baseline/tests/native_funding/artifact/mod.rs"
            ));
        }
        for (name, path) in outputs {
            cache = cache.with_output(name, path);
        }
        cache
    }

    #[cfg(test)]
    fn literal_zero_adapter_root(workspace_root: &Path) -> PathBuf {
        std::env::var_os("CANIC_TEST_SCRATCH").map_or_else(
            || {
                test_target_dir(workspace_root, "canic-121-production-adapter")
                    .join(std::process::id().to_string())
            },
            |scratch| PathBuf::from(scratch).join("canic-121-production-adapter"),
        )
    }

    #[cfg(test)]
    fn prepare_generated_fixture_artifacts(
        root: &Path,
        topology: &canic_core::bootstrap::compiled::ComponentTopology,
        release: ReleaseBuildId,
        roles: &[String],
    ) -> canic_host::release_set::fixture::PersistedFixtureArtifactManifest {
        let inputs = if roles.iter().any(|role| role == "user_shard") {
            let path = root.join("fixture-source/user_shard.bin");
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, b"reviewed fixture source").unwrap();
            vec![canic_host::release_set::fixture::FixtureSourceInput {
                role: "user_shard".into(),
                format_hash: [2; 32],
                completion_summary: [3; 32],
                chunk_paths: vec!["fixture-source/user_shard.bin".to_string()],
            }]
        } else {
            Vec::new()
        };
        canic_host::release_set::fixture::compile_and_persist_fixture_artifact_manifest(
            root, topology, release, &inputs,
        )
        .expect("retain neutral generated fixture authority before artifact caching")
    }

    #[cfg(test)]
    fn bind_generated_fixture_cache_inputs(
        mut cache: ArtifactCacheSpec,
        root: &Path,
        fixtures: &canic_host::release_set::fixture::PersistedFixtureArtifactManifest,
    ) -> ArtifactCacheSpec {
        let selection = root.join("fixture-source/selection.json");
        std::fs::create_dir_all(selection.parent().unwrap()).unwrap();
        std::fs::write(&selection, serde_json::to_vec(&fixtures.manifest).unwrap()).unwrap();
        cache = cache.with_input("fixture-selection", &selection);
        if !fixtures.manifest.entries.is_empty() {
            cache = cache.with_input(
                "fixture-source",
                &root.join("fixture-source/user_shard.bin"),
            );
        }
        cache
    }

    #[cfg(test)]
    fn append_generated_fixture_outputs(
        root: &Path,
        fixtures: &canic_host::release_set::fixture::PersistedFixtureArtifactManifest,
        outputs: &mut BTreeMap<String, PathBuf>,
    ) {
        let release_root = literal_zero_release_root(root, fixtures.manifest.release_build_id);
        for entry in &fixtures.manifest.entries {
            let content = hex_bytes(entry.content_id);
            for index in 0..entry.descriptor.chunks.len() {
                outputs.insert(
                    format!("fixture-{content}-{index}"),
                    release_root.join(format!("fixture-content/{content}/{index}.bin")),
                );
            }
        }
    }

    #[cfg(test)]
    fn literal_zero_release_artifact_outputs(
        adapter_root: &Path,
        release_build_id: ReleaseBuildId,
        configured_roles: &[String],
    ) -> BTreeMap<String, PathBuf> {
        let release_root = literal_zero_release_root(adapter_root, release_build_id);
        let mut outputs = BTreeMap::from([
            (
                "application-manifest".to_string(),
                release_root.join("application-artifact-union.json"),
            ),
            (
                "current-manifest".to_string(),
                release_root.join("current-release-set-manifest.json"),
            ),
            (
                "fixture-manifest".to_string(),
                release_root.join("fixture-artifact-manifest.json"),
            ),
            (
                "infrastructure-manifest".to_string(),
                release_root.join("infrastructure-artifact-manifest.json"),
            ),
            ("release-plan".to_string(), release_root.join("plan.cbor")),
        ]);
        let mut roles = configured_roles.iter().cloned().collect::<BTreeSet<_>>();
        roles.extend([
            CanicInfrastructureRole::FleetCoordinator
                .as_str()
                .to_string(),
            CanicInfrastructureRole::WasmStore.as_str().to_string(),
        ]);
        for role in roles {
            for extension in ["did", "wasm", "wasm.gz"] {
                outputs.insert(
                    format!("{role}-{extension}"),
                    literal_zero_role_artifact_path(
                        adapter_root,
                        release_build_id,
                        &role,
                        extension,
                    ),
                );
            }
        }
        outputs
    }

    #[cfg(test)]
    fn load_literal_zero_release_artifacts(
        adapter_root: &Path,
        release_build_id: ReleaseBuildId,
        configured_roles: &[String],
    ) -> LiteralZeroReleaseArtifacts {
        let artifact_path = |role: &str, extension: &str| {
            literal_zero_role_artifact_path(adapter_root, release_build_id, role, extension)
        };
        let coordinator_role = CanicInfrastructureRole::FleetCoordinator.as_str();
        let root_role = CanisterRole::ROOT.as_str();
        let store_role = CanicInfrastructureRole::WasmStore.as_str();
        let component_wasms = configured_roles
            .iter()
            .filter(|role| role.as_str() != root_role)
            .map(|role| {
                (
                    CanisterRole::from(role.clone()),
                    std::fs::read(artifact_path(role, "wasm"))
                        .expect("read cached literal-zero Component Wasm"),
                )
            })
            .collect();
        let coordinator_wasm_path = artifact_path(coordinator_role, "wasm");
        let root_wasm_path = artifact_path(root_role, "wasm");
        let store_wasm_path = artifact_path(store_role, "wasm");

        LiteralZeroReleaseArtifacts {
            component_wasms,
            coordinator_candid: relative_artifact_path(
                adapter_root,
                &artifact_path(coordinator_role, "did"),
            ),
            coordinator_wasm: relative_artifact_path(adapter_root, &coordinator_wasm_path),
            release_build_id,
            root_candid: relative_artifact_path(adapter_root, &artifact_path(root_role, "did")),
            root_wasm: relative_artifact_path(adapter_root, &root_wasm_path),
            root_wasm_bytes: std::fs::read(root_wasm_path)
                .expect("read cached literal-zero Root Wasm"),
            store_candid: relative_artifact_path(adapter_root, &artifact_path(store_role, "did")),
            store_wasm: relative_artifact_path(adapter_root, &store_wasm_path),
            store_wasm_bytes: std::fs::read(store_wasm_path)
                .expect("read cached literal-zero Store Wasm"),
        }
    }

    #[cfg(test)]
    fn literal_zero_release_root(adapter_root: &Path, release_build_id: ReleaseBuildId) -> PathBuf {
        adapter_root
            .join(".canic/release-builds")
            .join(release_build_id.to_string())
    }

    #[cfg(test)]
    fn literal_zero_role_artifact_path(
        adapter_root: &Path,
        release_build_id: ReleaseBuildId,
        role: &str,
        extension: &str,
    ) -> PathBuf {
        literal_zero_release_root(adapter_root, release_build_id)
            .join("artifacts")
            .join(role)
            .join(format!("{role}.{extension}"))
    }

    #[cfg(test)]
    fn literal_zero_build_context(
        workspace_root: &Path,
        adapter_root: &Path,
        config_path: &Path,
        build_network: BuildNetwork,
        release_build_id: ReleaseBuildId,
    ) -> WorkspaceBuildContext {
        WorkspaceBuildContext {
            role: "root".into(),
            profile: CanisterBuildProfile::Fast,
            environment: build_network.to_string(),
            build_network,
            workspace_root: workspace_root.to_path_buf(),
            icp_root: adapter_root.to_path_buf(),
            config_path: config_path.to_path_buf(),
            local_replica: None,
            refresh_canonical_infrastructure_did: false,
            release_build_id: Some(release_build_id),
        }
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one helper builds and seals the complete literal-zero release authority"
    )]
    fn build_and_seal_literal_zero_release_artifacts(
        workspace_root: &Path,
        adapter_root: &Path,
        config_path: &Path,
        configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
        configured_roles: &[String],
        release_build_id: ReleaseBuildId,
        build_network: BuildNetwork,
    ) {
        let context = literal_zero_build_context(
            workspace_root,
            adapter_root,
            config_path,
            build_network,
            release_build_id,
        );
        let builder = CanisterArtifactBuilder::for_profile(context.profile)
            .expect("preflight literal-zero artifact toolchain");
        // Match the production App path: Cargo stays serial while captured
        // infrastructure outputs finalize alongside later compilation.
        let mut phase = Span::start("app_artifacts_build");
        let audit_root = native_funding::uses_audit_root(config_path);
        let compiled_roles = configured_roles
            .iter()
            .filter(|role| !audit_root || role.as_str() != "root")
            .cloned()
            .collect::<Vec<_>>();
        let app = builder
            .build_workspace_app_artifacts(&context, &compiled_roles)
            .expect("build literal-zero App artifacts");
        let coordinator = app.coordinator.output;
        let store = app.store.output;
        let configured = app.configured;
        let root = if audit_root {
            // This fixture installs the audit Root; do not compile a canonical
            // Root only to discard its declaration, runtime and finalization.
            native_funding::build_audit_root(&context)
        } else {
            configured
                .iter()
                .find(|output| output.role == "root")
                .expect("literal-zero Root artifact")
                .output
                .clone()
        };
        let components = configured
            .iter()
            .filter(|output| output.role != "root")
            .map(|output| {
                (
                    CanisterRole::from(output.role.clone()),
                    output.output.clone(),
                )
            })
            .collect::<Vec<_>>();
        assert!(!components.is_empty(), "literal-zero Component artifacts");

        phase = phase.next("seal_release_manifests");
        let infrastructure = compile_and_persist_canic_infrastructure_artifact_manifest(
            adapter_root,
            release_build_id,
            &[
                infrastructure_build_output(
                    CanicInfrastructureRole::FleetCoordinator,
                    release_build_id,
                    &coordinator,
                ),
                infrastructure_build_output(
                    CanicInfrastructureRole::FleetSubnetRoot,
                    release_build_id,
                    &root,
                ),
                infrastructure_build_output(
                    CanicInfrastructureRole::WasmStore,
                    release_build_id,
                    &store,
                ),
            ],
        )
        .expect("persist literal-zero infrastructure authority");
        let application_targets = components
            .iter()
            .map(|(role, component)| ApplicationArtifactBuildTarget {
                role: role.clone(),
                package: component.package_name.clone(),
                wasm_relative_path: relative_artifact_path(adapter_root, &component.wasm_path),
                wasm_gz_relative_path: relative_artifact_path(
                    adapter_root,
                    &component.wasm_gz_path,
                ),
            })
            .collect::<Vec<_>>();
        let application_outputs = components
            .iter()
            .map(|(role, component)| ApplicationArtifactFileBuildOutput {
                role: role.clone(),
                package: component.package_name.clone(),
                release_build_id,
                wasm_path: component.wasm_path.clone(),
                wasm_gz_path: component.wasm_gz_path.clone(),
                candid_sha256: component.candid_sha256,
                protocol_profile_digest: component.protocol_profile_digest,
            })
            .collect::<Vec<_>>();
        let application = compile_and_persist_application_artifact_union(
            adapter_root,
            &configuration.component_topology,
            release_build_id,
            &application_targets,
            &application_outputs,
        )
        .expect("persist literal-zero application authority");
        let fixtures = prepare_generated_fixture_artifacts(
            adapter_root,
            &configuration.component_topology,
            release_build_id,
            configured_roles,
        );
        let current = compile_and_persist_current_release_set_manifest(
            adapter_root,
            &configuration.component_topology,
            release_build_id,
            &application,
            &infrastructure,
            &fixtures,
        )
        .expect("persist literal-zero current release authority");
        finalize_release_build_from_manifest(adapter_root, release_build_id, &current.path)
            .expect("finalize literal-zero current release authority");
        phase.finish();
    }

    #[cfg(test)]
    fn persist_internal_test_release_build_plan(
        root: &Path,
        build_network: BuildNetwork,
        release_nonce: [u8; 32],
    ) -> ReleaseBuildId {
        let nonce = ReleaseBuildNonce::from_random_bytes(release_nonce);
        let release_build_id = ReleaseBuildId::from_nonce(nonce);
        let value = Value::Array(vec![
            Value::Bytes(nonce.as_bytes().to_vec()),
            Value::Bytes(release_build_id.as_bytes().to_vec()),
            Value::Text(env!("CARGO_PKG_VERSION").to_string()),
            Value::Text("fast".to_string()),
            Value::Text(build_network.to_string()),
            Value::Array(vec![Value::Integer(0.into())]),
        ]);
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&value, &mut bytes)
            .expect("encode deterministic internal release-build plan");
        let path = canic_host::release_build::release_build_plan_path(root, release_build_id);
        std::fs::create_dir_all(path.parent().expect("release-build plan parent"))
            .expect("create release-build plan parent");
        std::fs::write(path, bytes).expect("write deterministic internal release-build plan");
        let planned = canic_host::release_build::load_release_build_plan(root, release_build_id)
            .expect("validate fixture release-build authority before building artifacts");
        assert_eq!(planned.build_network, build_network);
        release_build_id
    }

    #[test]
    #[ignore = "focused build qualification requires installed Wasm and artifact tools"]
    fn pipelined_release_artifacts_match_serial_builds() {
        let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let adapter_root = literal_zero_adapter_root(&workspace);
        std::fs::create_dir_all(&adapter_root).unwrap();
        let _cleanup = TestDirectoryCleanup(adapter_root.clone());
        let release = persist_internal_test_release_build_plan(
            &adapter_root,
            BuildNetwork::Local,
            INTERNAL_TEST_RELEASE_BUILD_NONCE,
        );
        let context = WorkspaceBuildContext {
            role: "root".to_string(),
            profile: CanisterBuildProfile::Fast,
            environment: "local".to_string(),
            build_network: BuildNetwork::Local,
            workspace_root: workspace.clone(),
            icp_root: adapter_root,
            config_path: workspace.join("apps/test/test-configs/literal-zero-initial-shard.toml"),
            local_replica: None,
            refresh_canonical_infrastructure_did: false,
            release_build_id: Some(release),
        };
        let roles = AppConfigSnapshot::load(&context.config_path)
            .unwrap()
            .model()
            .roles
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let builder = CanisterArtifactBuilder::for_profile(context.profile).unwrap();
        let mut expected = BTreeMap::new();
        for role in ["fleet_coordinator", "wasm_store"] {
            let output = builder
                .build_workspace_canister_artifact(&context.with_role(role))
                .unwrap();
            expected.insert(role.to_string(), artifact_parity_snapshot(&output));
        }
        for output in builder
            .build_workspace_configured_canister_artifacts(&context, &roles)
            .unwrap()
        {
            expected.insert(output.role, artifact_parity_snapshot(&output.output));
        }
        let app = builder
            .build_workspace_app_artifacts(&context, &roles)
            .unwrap();
        let mut actual = BTreeMap::from([
            (
                "fleet_coordinator".to_string(),
                artifact_parity_snapshot(&app.coordinator.output),
            ),
            (
                "wasm_store".to_string(),
                artifact_parity_snapshot(&app.store.output),
            ),
        ]);
        for output in app.configured {
            actual.insert(output.role, artifact_parity_snapshot(&output.output));
        }
        // Avoid dumping binary artifacts if this assertion fails.
        assert!(
            expected == actual,
            "pipelined artifact bytes or identities differ"
        );
    }

    #[cfg(test)]
    fn artifact_parity_snapshot(output: &CanisterArtifactBuildOutput) -> (String, [Vec<u8>; 3]) {
        let bytes = [&output.wasm_path, &output.wasm_gz_path, &output.did_path]
            .map(|path| std::fs::read(path).unwrap());
        let identity = format!(
            "{}:{}:{}:{}:{:?}:{:?}:{:?}",
            output.package_name,
            output.package_version,
            output.protocol_release_identity,
            output.protocol_role,
            output.protocol_capabilities,
            output.candid_sha256,
            output.protocol_profile_digest,
        );
        eprintln!(
            "App artifact parity: identity={identity} wasm_bytes={} wasm_sha256={}",
            bytes[0].len(),
            canic_core::cdk::utils::hash::sha256_hex(&bytes[0]),
        );
        (identity, bytes)
    }

    #[test]
    fn reinstall_fixture_release_cache_binds_distinct_repeatable_identities() {
        let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let root = std::env::temp_dir().join(format!(
            "canic-reinstall-cache-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        let _cleanup = TestDirectoryCleanup(root.clone());
        let first = persist_internal_test_release_build_plan(
            &root,
            BuildNetwork::Local,
            INTERNAL_TEST_RELEASE_BUILD_NONCE,
        );
        let second = persist_internal_test_release_build_plan(
            &root,
            BuildNetwork::Local,
            REINSTALL_RELEASE_BUILD_NONCE,
        );
        assert_eq!(first.to_string(), INTERNAL_TEST_RELEASE_BUILD_ID.1);
        assert_ne!(first, second);
        let path = canic_host::release_build::release_build_plan_path(&root, second);
        let retained = std::fs::read(&path).unwrap();
        assert_eq!(
            persist_internal_test_release_build_plan(
                &root,
                BuildNetwork::Local,
                REINSTALL_RELEASE_BUILD_NONCE,
            ),
            second
        );
        assert_eq!(std::fs::read(path).unwrap(), retained);
        let config = workspace.join("canisters/audit/root_probe/activation.toml");
        let roles = vec!["app".to_string(), "root".to_string()];
        // Keep output paths identical here: only the declared release identity
        // may distinguish these cache specifications.
        let outputs = literal_zero_release_artifact_outputs(&root, first, &roles);
        let acquire = |id| {
            let cache = literal_zero_release_artifact_cache_spec(
                &workspace,
                &root.join("cache"),
                &config,
                &roles,
                &outputs,
                BuildNetwork::Local,
                id,
            );
            match prepare_artifact_cache(&cache).unwrap() {
                ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
                ArtifactCachePreparation::Build(transaction) => {
                    // Exercise the real content cache with inert files in this private
                    // directory; this test does not build or install canisters.
                    for name in outputs.keys() {
                        std::fs::write(
                            transaction.output_path(name).unwrap(),
                            format!("{id}:{name}"),
                        )
                        .unwrap();
                    }
                    transaction.commit().unwrap()
                }
            }
        };
        let initial = acquire(first);
        assert!(!initial.is_reused());
        let replacement = acquire(second);
        assert!(!replacement.is_reused());
        assert_ne!(initial.record().key(), replacement.record().key());
        let replay = acquire(first);
        assert!(replay.is_reused());
        assert_eq!(initial.record().key(), replay.record().key());
        // A later build and destination replacement cannot change a retained input.
        for path in outputs.values() {
            std::fs::write(path, b"replaced mutable output").unwrap();
        }
        stage_retained_release_artifacts(initial.record(), &outputs);
        for (name, path) in &outputs {
            assert_eq!(
                std::fs::read_to_string(path).unwrap(),
                format!("{first}:{name}")
            );
            let replacement_path =
                crate::pic::artifacts::retained_artifact_path(replacement.record(), name);
            assert_eq!(
                std::fs::read_to_string(replacement_path).unwrap(),
                format!("{second}:{name}")
            );
        }
    }

    // Stage from immutable retained inputs into this invocation's private release root.
    #[cfg(test)]
    fn stage_retained_release_artifacts(
        record: &ic_testkit::artifacts::ArtifactCacheRecord,
        outputs: &BTreeMap<String, PathBuf>,
    ) {
        assert_eq!(record.artifacts().len(), outputs.len());
        for (name, destination) in outputs {
            let source = crate::pic::artifacts::retained_artifact_path(record, name);
            std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
            std::fs::copy(source, destination).expect("stage retained release artifact");
        }
    }

    #[test]
    fn generated_release_cache_restores_fixture_authority_without_source_rebuild() {
        let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let root = std::env::temp_dir().join(format!(
            "canic-fixture-authority-cache-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        let _cleanup = TestDirectoryCleanup(root.clone());
        let release = persist_internal_test_release_build_plan(
            &root,
            BuildNetwork::Local,
            INTERNAL_TEST_RELEASE_BUILD_NONCE,
        );
        let configuration = AppConfigSnapshot::load(
            &workspace.join("apps/test/test-configs/generated-mixed-topology.toml"),
        )
        .unwrap()
        .model()
        .compile_component_deployment_configuration()
        .unwrap();
        let topology = &configuration.component_topology;
        let roles = vec!["user_shard".to_string(), "root".to_string()];
        let fixtures = prepare_generated_fixture_artifacts(&root, topology, release, &roles);
        assert!(!fixtures.manifest.entries.is_empty());
        let mut outputs = literal_zero_release_artifact_outputs(&root, release, &roles);
        append_generated_fixture_outputs(&root, &fixtures, &mut outputs);
        let mut cache = ArtifactCacheSpec::new(
            &root.join("cache"),
            "generated-fixture-authority",
            "canic/generated-fixture-authority/v1",
        );
        for (name, path) in &outputs {
            cache = cache.with_output(name, path);
        }
        let cache = bind_generated_fixture_cache_inputs(cache, &root, &fixtures);
        let ArtifactCachePreparation::Build(transaction) = prepare_artifact_cache(&cache).unwrap()
        else {
            panic!("private cache must start empty");
        };
        for (name, path) in &outputs {
            if path.is_file() {
                transaction.import_output(name, path).unwrap();
            } else {
                // Only the release plan and fixture authority are real in this native proof.
                std::fs::write(transaction.output_path(name).unwrap(), name).unwrap();
            }
        }
        transaction.commit().unwrap();
        std::fs::remove_dir_all(literal_zero_release_root(&root, release)).unwrap();
        let ArtifactCachePreparation::Reused(record) = prepare_artifact_cache(&cache).unwrap()
        else {
            panic!("expected cached fixture authority");
        };
        stage_retained_release_artifacts(&record, &outputs);
        let restored = canic_host::release_set::fixture::load_fixture_artifact_manifest(
            &root,
            topology,
            release,
            fixtures.digest,
        )
        .expect("cache restores exact fixture authority after the release directory is removed");
        assert_eq!(restored, fixtures);
        std::fs::write(
            root.join("fixture-source/user_shard.bin"),
            b"changed source",
        )
        .unwrap();
        assert!(matches!(
            prepare_artifact_cache(&cache).unwrap(),
            ArtifactCachePreparation::Build(_)
        ));
        std::fs::remove_dir_all(root.join("fixture-source")).unwrap();
        canic_host::release_set::fixture::verify_fixture_artifacts(
            &root,
            topology,
            release,
            fixtures.digest,
        )
        .expect("retained payload verifies without authored source files");
    }

    #[cfg(test)]
    fn infrastructure_build_output(
        role: CanicInfrastructureRole,
        release_build_id: ReleaseBuildId,
        output: &CanisterArtifactBuildOutput,
    ) -> CanicInfrastructureArtifactBuildOutput {
        CanicInfrastructureArtifactBuildOutput {
            role,
            package: output.package_name.clone(),
            protocol_release_identity: output.protocol_release_identity.clone(),
            protocol_role: output.protocol_role.clone(),
            protocol_capabilities: output.protocol_capabilities.clone(),
            release_build_id,
            wasm_path: output.wasm_path.clone(),
            wasm_gz_path: output.wasm_gz_path.clone(),
            candid_sha256: output.candid_sha256,
            protocol_profile_digest: output.protocol_profile_digest,
        }
    }

    #[cfg(test)]
    fn relative_artifact_path(root: &Path, path: &Path) -> String {
        path.strip_prefix(root)
            .expect("literal-zero artifact stays inside its operator root")
            .to_str()
            .expect("literal-zero artifact path UTF-8")
            .to_string()
    }

    #[cfg(test)]
    struct LiteralZeroFleetInput<'a> {
        bootstrap: DesiredFleetBootstrap,
        coordinator_wasm: &'a Path,
        cycles_ledger: Principal,
        operator: Principal,
        pool_count: usize,
        pool_creation_funding: u128,
        pool_minimum_cycles: u128,
        protocol: DesiredFleetProtocol,
        root_wasm: &'a Path,
        store_wasm: &'a Path,
        subnet: Principal,
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one constructor keeps the complete literal-zero desired estate visible"
    )]
    fn literal_zero_fleet_desired(input: LiteralZeroFleetInput<'_>) -> DesiredFleet {
        let LiteralZeroFleetInput {
            bootstrap,
            coordinator_wasm,
            cycles_ledger,
            operator,
            pool_count,
            pool_creation_funding,
            pool_minimum_cycles,
            protocol,
            root_wasm,
            store_wasm,
            subnet,
        } = input;
        let mut canisters = vec![
            serde_json::json!({
                "canic_init": { "role": "coordinator" },
                "controller_canisters": [],
                "controllers": [operator.to_text()],
                "drain": null,
                "initial_cycles": COORDINATOR_INSTALL_CYCLES.to_string(),
                "init_arg": null,
                "init_candid": null,
                "kind": "coordinator",
                "minimum_cycles": "0",
                "name": "coordinator",
                "parent": null,
                "presence": "present",
                "principal": null,
                "replace": false,
                "subnet": subnet.to_text(),
                "wasm": coordinator_wasm.display().to_string(),
            }),
            serde_json::json!({
                "canic_init": { "role": "root", "root": "root" },
                "controller_canisters": [],
                "controllers": [operator.to_text()],
                "drain": null,
                "initial_cycles": ROOT_INSTALL_CYCLES.to_string(),
                "init_arg": null,
                "init_candid": null,
                "kind": "root",
                "minimum_cycles": "0",
                "name": "root",
                "parent": "coordinator",
                "presence": "present",
                "principal": null,
                "replace": false,
                "subnet": subnet.to_text(),
                "wasm": root_wasm.display().to_string(),
            }),
            serde_json::json!({
                "canic_init": { "role": "store", "root": "root" },
                "controller_canisters": ["root"],
                "controllers": [operator.to_text()],
                "drain": null,
                "initial_cycles": ROOT_INSTALL_CYCLES.to_string(),
                "init_arg": null,
                "init_candid": null,
                "kind": "store",
                "minimum_cycles": "0",
                "name": "store",
                "parent": "root",
                "presence": "present",
                "principal": null,
                "replace": false,
                "subnet": subnet.to_text(),
                "wasm": store_wasm.display().to_string(),
            }),
        ];
        canisters.extend((0..pool_count).map(|index| {
            serde_json::json!({
                "canic_init": null,
                "controller_canisters": ["root"],
                "controllers": [],
                "drain": null,
                "initial_cycles": pool_creation_funding.to_string(),
                "init_arg": null,
                "init_candid": null,
                "kind": "pool",
                "minimum_cycles": pool_minimum_cycles.to_string(),
                "name": format!("pool-{index}"),
                "parent": "root",
                "presence": "present",
                "principal": null,
                "replace": false,
                "subnet": subnet.to_text(),
                "wasm": null,
            })
        }));
        serde_json::from_value(serde_json::json!({
            "bootstrap": bootstrap,
            "canisters": canisters,
            "cycles_ledger": cycles_ledger.to_text(),
            "environment": "local",
            "fleet": "canic-121-literal-zero-estate",
            "ledger_fee_cycles": "0",
            "management_creation_fee_cycles": "0",
            "material_cycle_threshold": "0",
            "maximum_observation_burn_cycles": "1000000000000",
            "maximum_stalled_observations": 64,
            "maximum_update_burn_cycles": "1000000000000",
            "operator": operator.to_text(),
            "protocol": protocol,
            "schema_version": FLEET_ENSURE_SCHEMA_VERSION,
            "treasury": "coordinator",
        }))
        .expect("decode literal-zero desired Fleet")
    }

    #[cfg(test)]
    fn desired_sha256(desired: &DesiredFleet) -> String {
        hex_bytes(wasm_hash(
            toml::to_string_pretty(desired)
                .expect("encode desired Fleet identity")
                .as_bytes(),
        ))
    }

    #[cfg(test)]
    fn planned_actions(plan: &FleetEnsurePlan) -> Vec<&EnsureAction> {
        plan.canisters
            .iter()
            .flat_map(|canister| canister.actions.iter())
            .chain(plan.protocol_actions.iter())
            .collect()
    }

    #[test]
    fn prepared_mainnet_root_automatically_refills_one_exact_pool_asset() {
        assert_mainnet_refill(false, 1, 1);
    }

    #[test]
    fn autonomous_refill_margin_survives_burn_and_replays_without_another_debit() {
        assert_mainnet_refill(true, 4, 5);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one recovery journey proves upgrade, refresh, retained identity and exact allocation"
    )]
    fn historical_pool_assets_upgrade_refresh_and_claim_without_losing_cycles() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = five_trillion_component_root_canister_config_path(&workspace_root);
        let root_wasm = build_five_trillion_component_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let store_fixture = build_root_store_fixture_with_config(
            &config_path,
            build_five_trillion_component_wasms(),
        );
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let imported = std::cell::RefCell::new(Vec::new());
        let fixture = install_bootstrapped_root_with_config_and_pool_setup(
            &pic,
            root_wasm.clone(),
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: None,
                canister_pool_minimum_size: Some(2),
                canister_pool_cycles: Some(Cycles::new(2_000_000_000_000)),
                coordinator_subnet: None,
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: None,
                component_admission_limits: None,
                fleet_id: None,
                funding: None,
                coordinator_root_funding: None,
            },
            &config_path,
            |pic, root| {
                let root_subnet = pic.get_subnet(root).expect("root placement Subnet");
                let assets = [2_000_000_000_000, 4_500_000_000_000]
                    .into_iter()
                    .map(|cycles| {
                        let asset = pic
                            .create_canister_with_params(
                                None,
                                CreateCanisterParams {
                                    cycles: Some(cycles + 10_000_000_000),
                                    settings: None,
                                    placement: Some(CreateCanisterPlacement::SubnetId(root_subnet)),
                                },
                            )
                            .expect("create bounded retained pool asset");
                        pic.set_controllers(asset, None, vec![root])
                            .expect("prepare retained Root-controlled import");
                        asset
                    })
                    .collect::<Vec<_>>();
                imported.replace(assets.clone());
                assets
            },
        );
        reset_prepaid_pool_assets_for_count(&pic, fixture.root_id, 2);
        let assets = imported.borrow();
        let small = assets[0];
        let large = assets[1];
        let retained = root_pool_status(&pic, fixture.root_id);
        assert_eq!(retained.ready, 2);
        assert_eq!(retained.failed, 0);
        let small_retained_balance = pic.cycle_balance(small);
        let large_retained_balance = pic.cycle_balance(large);
        assert!((2_000_000_000_000..2_010_000_000_000).contains(&small_retained_balance));
        assert!((4_500_000_000_000..4_510_000_000_000).contains(&large_retained_balance));

        pic.upgrade_canister(fixture.root_id, root_wasm, crate::pic::upgrade_args(), None)
            .expect("upgrade the Root without rebuilding retained pool state");
        let upgraded = root_pool_status(&pic, fixture.root_id);
        assert_eq!(upgraded.ready, 2);
        assert_eq!(
            upgraded
                .entries
                .iter()
                .filter(|entry| entry.origin == CanisterPoolAssetOrigin::Imported)
                .count(),
            2
        );

        pic.add_cycles(large, 5_000_000_000_000 - pic.cycle_balance(large));
        let funded_balance = pic.cycle_balance(large);
        assert_eq!(funded_balance, 5_000_000_000_000);
        for attempt in 0..2 {
            let RootCommandResponseFragment::ImportPoolCanister(PoolImportResponse::Imported {
                canister_id,
            }) = root_command(
                &pic,
                fixture.root_id,
                RootCommandFragment::ImportPoolCanister(PoolCanisterRequest { canister_id: large }),
            )
            .expect("refresh topped-up import")
            else {
                panic!("Root did not publish the refreshed imported asset");
            };
            assert_eq!(canister_id, large);
            assert_eq!(
                pic.cycle_balance(large),
                funded_balance,
                "attempt {attempt} must not debit the imported asset"
            );
        }

        let refreshed = root_pool_status(&pic, fixture.root_id);
        assert_eq!(refreshed.failed, 0);
        assert_eq!(refreshed.ready, 2);
        let refreshed_entry = refreshed
            .entries
            .iter()
            .find(|entry| entry.canister_id == large)
            .expect("refreshed imported row");
        assert_eq!(refreshed_entry.origin, CanisterPoolAssetOrigin::Imported);
        assert_eq!(refreshed_entry.status, CanisterPoolAssetStatus::Ready);
        assert_eq!(refreshed_entry.cycles, Cycles::new(funded_balance));

        let operation_id = [0x71; 32];
        let component_registry_request = begin_fixture_fresh_component_provisioning_with_config(
            &pic,
            coordinator,
            coordinator_wasm,
            &fixture,
            operation_id,
            &config_path,
        );
        let mut last_status = None;
        let mut terminal = None;
        for _ in 0..120 {
            let CoordinatorOperationReadResponse::Operation(
                CoordinatorOperationStatusResponse::ComponentProvisioning(status),
            ) = coordinator_status(
                &pic,
                coordinator,
                CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .expect("query retained-pool provisioning")
            else {
                panic!("Coordinator returned a differently correlated operation status");
            };
            if status.phase == FleetComponentProvisioningPhase::RuntimesActivated {
                terminal = Some(status);
                break;
            }
            last_status = Some(status);
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        let terminal = terminal.unwrap_or_else(|| {
            panic!("retained-pool provisioning did not converge: {last_status:?}")
        });
        assert_eq!(terminal.component_count, 1);
        assert_eq!(terminal.runtime_activated_root_count, 1);
        assert!(terminal.pending_root_failure.is_none());

        let RootCommandResponseFragment::PrepareComponentRegistry(first_proof) = root_command(
            &pic,
            fixture.root_id,
            RootCommandFragment::PrepareComponentRegistry(component_registry_request.clone()),
        )
        .expect("replay advanced retained Component Registry without mutation") else {
            panic!("Root returned a differently correlated Component Registry proof");
        };
        let RootCommandResponseFragment::PrepareComponentRegistry(replayed_proof) = root_command(
            &pic,
            fixture.root_id,
            RootCommandFragment::PrepareComponentRegistry(component_registry_request),
        )
        .expect("replay advanced retained Component Registry proof") else {
            panic!("Root returned a differently correlated Component Registry proof");
        };
        assert_eq!(replayed_proof, first_proof);
        assert_eq!(first_proof.fleet_subnet_root, fixture.root_id);
        assert_eq!(first_proof.reserved_component_instances, 0);
        assert_eq!(first_proof.committed_component_instances, 1);
        assert_eq!(first_proof.next_allocation_sequence, 2);

        let claimed = root_pool_status(&pic, fixture.root_id);
        assert_eq!(claimed.ready, 1);
        assert_eq!(claimed.workload, 1);
        let small_entry = claimed
            .entries
            .iter()
            .find(|entry| entry.canister_id == small)
            .expect("smaller retained asset");
        assert_eq!(small_entry.status, CanisterPoolAssetStatus::Ready);
        let large_entry = claimed
            .entries
            .iter()
            .find(|entry| entry.canister_id == large)
            .expect("refreshed retained asset");
        assert!(matches!(
            large_entry.status,
            CanisterPoolAssetStatus::Workload { .. }
        ));
        assert_eq!(pic.cycle_balance(small), small_retained_balance);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one production-shaped journey binds pool creation, typed retry and provisioning evidence"
    )]
    fn fresh_five_component_acceptance_seeds_the_root_owned_pool_before_effects() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = five_component_root_canister_config_path(&workspace_root);
        let (root_wasm, cycles_ledger_wasm) = build_mainnet_five_component_refill_wasms();
        let coordinator_wasm = build_test_coordinator_wasm();
        let store_fixture =
            build_root_store_fixture_with_config(&config_path, build_five_component_wasms());
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let created_assets = std::cell::RefCell::new(Vec::new());
        let fixture = install_bootstrapped_root_with_config_and_pool_setup(
            &pic,
            root_wasm,
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: None,
                canister_pool_minimum_size: Some(5),
                canister_pool_cycles: None,
                coordinator_subnet: None,
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: None,
                component_admission_limits: None,
                fleet_id: None,
                funding: None,
                coordinator_root_funding: None,
            },
            &config_path,
            |pic, root| {
                let root_subnet = pic.get_subnet(root).expect("root placement Subnet");
                let assets = (0..5)
                    .map(|_| {
                        let asset = pic
                            .create_canister_with_params(
                                None,
                                CreateCanisterParams {
                                    cycles: Some(
                                        QUALIFICATION_ASSET_CYCLES
                                            + MAINNET_REFILL_EXECUTION_MARGIN,
                                    ),
                                    settings: None,
                                    placement: Some(CreateCanisterPlacement::SubnetId(root_subnet)),
                                },
                            )
                            .expect("prepare funded Cycles Ledger creation result");
                        pic.set_controllers(asset, None, vec![root])
                            .expect("prepare Cycles Ledger creation result");
                        asset
                    })
                    .collect::<Vec<_>>();
                let cycles_ledger = Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai")
                    .expect("canonical Cycles Ledger principal");
                pic.create_canister_with_id(None, None, cycles_ledger)
                    .expect("create canonical Cycles Ledger stub principal");
                pic.install_canister(
                    cycles_ledger,
                    cycles_ledger_wasm,
                    encode_one(CyclesLedgerStubInitArgs {
                        canister_ids: assets.clone(),
                        expected_controllers_by_index: None,
                        expected_root: root,
                        expected_subnet: root_subnet,
                        initial_balances: Some(vec![CyclesLedgerStubAccountBalance {
                            balance: Nat::from(
                                5_u128
                                    * (QUALIFICATION_ASSET_CYCLES
                                        + MAINNET_REFILL_EXECUTION_MARGIN
                                        + MAINNET_REFILL_MANAGEMENT_CREATION_FEE
                                        + MAINNET_REFILL_LEDGER_FEE),
                            ),
                            owner: root,
                        }]),
                        pending_first_index: None,
                        withdrawal_fee: Some(Nat::from(MAINNET_REFILL_LEDGER_FEE)),
                    })
                    .expect("encode Cycles Ledger stub init"),
                    None,
                );
                created_assets.replace(assets);
                Vec::new()
            },
        );
        let operation_id = [0x6d; 32];
        begin_fixture_fresh_component_provisioning_with_config(
            &pic,
            coordinator,
            coordinator_wasm,
            &fixture,
            operation_id,
            &config_path,
        );

        let mut provisioned = None;
        let mut last_status = None;
        for _ in 0..240 {
            let CoordinatorOperationReadResponse::Operation(
                CoordinatorOperationStatusResponse::ComponentProvisioning(status),
            ) = coordinator_status(
                &pic,
                coordinator,
                CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .expect("query fresh Component provisioning")
            else {
                panic!("Coordinator returned a differently correlated operation status");
            };
            if status.provisioned_root_count == status.root_batch_count
                && status.components_provisioned_at_ns.is_some()
            {
                provisioned = Some(status);
                break;
            }
            last_status = Some(status);
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        let provisioned = provisioned.unwrap_or_else(|| {
            let root_progress = match root_status(
                &pic,
                fixture.root_id,
                RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
            ) {
                Ok(RootStatusResponseFragment::Operation(
                    RootOperationStatusResponse::ProvisionComponents(status),
                )) => format!(
                    "phase={:?} reserved={} claimed={} installed={} committed={}",
                    status.phase,
                    status.reserved_component_count,
                    status.claimed_component_count,
                    status.installed_component_count,
                    status.registry_committed_component_count,
                ),
                Ok(_) => "differently correlated operation status".to_string(),
                Err(error) => format!("error={error}"),
            };
            report_canister_diagnostics_batch(
                &pic,
                [
                    ("Coordinator", coordinator, Principal::anonymous()),
                    ("Root", fixture.root_id, Principal::anonymous()),
                ],
                "fresh provisioning automatic pool readiness",
            );
            panic!(
                "fresh Component provisioning did not complete: coordinator={last_status:?}; root={root_progress}"
            )
        });
        assert_eq!(provisioned.component_count, 5);
        assert!(provisioned.pending_root_failure.is_none());
        assert!(provisioned.estate_funding_required.is_none());

        let pool = root_pool_status(&pic, fixture.root_id);
        assert_eq!(pool.workload, 5);
        for asset in created_assets.borrow().iter().copied() {
            let entry = pool
                .entries
                .iter()
                .find(|entry| entry.canister_id == asset)
                .expect("created asset remains in the Root inventory");
            assert!(matches!(
                entry.status,
                CanisterPoolAssetStatus::Workload { .. }
            ));
        }
        let cycles_ledger = Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai")
            .expect("canonical Cycles Ledger principal");
        let request_count: u64 = pic
            .query_candid(cycles_ledger, "request_count", ())
            .expect("query pool creation request count");
        assert_eq!(request_count, 5);
    }

    #[cfg(test)]
    #[derive(CandidType)]
    enum FixtureReadinessRequest {
        Readiness,
    }

    #[cfg(test)]
    #[derive(CandidType, Deserialize)]
    enum FixtureReadinessResponse {
        Readiness(canic::dto::runtime::CanicReadinessStatus),
    }

    #[cfg(test)]
    fn fixture_readiness(
        pic: &PocketIc,
        root: Principal,
        target: Principal,
    ) -> canic::dto::runtime::CanicReadinessStatus {
        let response: Result<FixtureReadinessResponse, Error> = pic
            .query_candid_as(
                target,
                root,
                canic::protocol::CANIC_OBSERVABILITY,
                (FixtureReadinessRequest::Readiness,),
            )
            .expect("protected readiness remains observable");
        let FixtureReadinessResponse::Readiness(response) = response.unwrap();
        response
    }

    #[cfg(test)]
    fn selected_fixture_targets(pic: &PocketIc, root: Principal) -> Vec<(Principal, CanisterRole)> {
        selected_fixture_targets_from_pool(pic, root, &root_pool_status(pic, root))
    }

    #[cfg(test)]
    fn selected_fixture_targets_from_pool(
        pic: &PocketIc,
        root: Principal,
        pool: &CanisterPoolResponse,
    ) -> Vec<(Principal, CanisterRole)> {
        pool.entries
            .iter()
            .filter(|entry| matches!(entry.status, CanisterPoolAssetStatus::Workload { .. }))
            .filter_map(|entry| {
                let response = pic
                    .query_candid_as::<Result<ManagedStatusResponseFragment, Error>, _>(
                        entry.canister_id,
                        root,
                        canic::protocol::CANIC_OBSERVABILITY,
                        (ManagedStatusRequestFragment::Binding,),
                    )
                    .ok()?
                    .ok()?;
                let ManagedStatusResponseFragment::Binding(binding) = response else {
                    return None;
                };
                let role = match binding.as_ref() {
                    ManagedCanisterBinding::Component(binding) => &binding.role,
                    ManagedCanisterBinding::ComponentChild(binding) => &binding.role,
                };
                ["user_hub", "user_shard"]
                    .contains(&role.as_str())
                    .then_some((entry.canister_id, role.clone()))
            })
            .collect()
    }

    #[cfg(test)]
    fn assert_root_waits_for_fixture_membership(
        pic: &PocketIc,
        root: Principal,
        operation_id: [u8; 32],
    ) {
        let response = root_status(
            pic,
            root,
            RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
        )
        .expect("observe Root provisioning while fixtures are held");
        let RootStatusResponseFragment::Operation(
            RootOperationStatusResponse::ProvisionComponents(status),
        ) = response
        else {
            panic!("expected exact Component provisioning status");
        };
        assert!(
            !status.root_runtime_active,
            "initial child must progress while Root remains Prepared"
        );
        assert!(status.runtimes_activated_at_ns.is_none());
    }

    /// The initial child completes autonomously while the parent's data remains held.
    #[cfg(test)]
    fn qualify_pending_fixture_bootstrap(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
        funding: &canic_core::bootstrap::compiled::CyclesFundingPolicyConfig,
    ) {
        use canic::dto::fixture_provisioning::{FixtureImportError, FixtureProvisioningStatus};
        let mut selected = Vec::new();
        for _ in 0..240 {
            selected = selected_fixture_targets(pic, fixture.root_id);
            if selected.len() == 2 {
                break;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        assert_eq!(
            selected.len(),
            2,
            "initial child must be allocated before parent fixture readiness"
        );
        let hub = selected
            .iter()
            .find(|(_, role)| role.as_str() == "user_hub")
            .unwrap()
            .0;
        let shard = selected
            .iter()
            .find(|(_, role)| role.as_str() == "user_shard")
            .unwrap()
            .0;
        for _ in 0..40 {
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        assert_root_waits_for_fixture_membership(pic, fixture.root_id, operation_id);
        let pending_targets = selected_fixture_targets(pic, fixture.root_id);
        let readiness = fixture_readiness(pic, fixture.root_id, hub);
        assert_eq!(
            readiness.status,
            canic::dto::runtime::ReadinessStatus::NotReady
        );
        assert_eq!(readiness.fixture, Err(FixtureImportError::NotReady));
        assert!(
            pic.query_candid::<Result<String, Error>, _>(hub, "test_recovery_generation", ())
                .is_err()
        );
        assert!(
            pic.update_candid::<Result<Principal, Error>, _>(
                hub,
                "create_account",
                (Principal::anonymous(),)
            )
            .is_err()
        );
        for _ in 0..160 {
            if fixture_readiness(pic, fixture.root_id, shard).status
                == canic::dto::runtime::ReadinessStatus::Ready
            {
                break;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        let child = fixture_readiness(pic, fixture.root_id, shard);
        assert!(matches!(
            child.fixture,
            Ok(FixtureProvisioningStatus::Complete(_))
        ));
        assert_eq!(child.status, canic::dto::runtime::ReadinessStatus::Ready);
        assert_eq!(
            fixture_readiness(pic, fixture.root_id, hub).fixture,
            Err(FixtureImportError::NotReady)
        );
        assert_eq!(
            pending_targets,
            selected_fixture_targets(pic, fixture.root_id),
            "observing pending receipts must not allocate another selected target"
        );
        qualify_pending_fixture_funding(pic, fixture.root_id, hub, funding);
        assert_eq!(
            fixture_readiness(pic, fixture.root_id, hub).fixture,
            Err(FixtureImportError::NotReady)
        );
        assert_root_waits_for_fixture_membership(pic, fixture.root_id, operation_id);
        pic.update_candid_as::<(), _>(hub, fixture.root_id, "test_release_fixture", ())
            .unwrap();
    }

    /// Pending imports share the configured allowance and exact funding replay owner.
    #[cfg(test)]
    fn qualify_pending_fixture_funding(
        pic: &PocketIc,
        root: Principal,
        hub: Principal,
        funding: &canic_core::bootstrap::compiled::CyclesFundingPolicyConfig,
    ) {
        use canic::dto::rpc::{CyclesFundingPreflightResponse, CyclesResponse};
        let per_request = funding.max_per_request.to_u128();
        let total = funding.max_per_child.to_u128();
        assert!(per_request > 0 && total > 0 && funding.cooldown_secs > 0);
        // Stop only the recipient so exact balance assertions exclude its timer execution.
        // Root remains live and executes the actual management transfer and durable receipt.
        pic.stop_canister(hub, Some(root)).unwrap();
        let initial_balance = pic.cycle_balance(hub);
        let error = root_command_as(
            pic,
            root,
            Principal::from_slice(&[0xb4; 29]),
            RootCommandFragment::RespondCapability(descendant_funding_request(pic, 0xb4)),
        )
        .err()
        .expect("unregistered caller cannot acquire a fixture funding allowance");
        assert_eq!(
            error.code(),
            canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );
        let mut forbidden = descendant_funding_request(pic, 0xb5);
        forbidden.capability =
            canic::dto::rpc::Request::RecycleCanister(canic::dto::rpc::RecycleCanisterRequest {
                canister_pid: hub,
                metadata: None,
            });
        let error = root_command_as(
            pic,
            root,
            hub,
            RootCommandFragment::RespondCapability(forbidden),
        )
        .err()
        .expect("Prepared funding authority must not admit recycling");
        assert_eq!(
            error.code(),
            canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );
        assert_eq!(pic.cycle_balance(hub), initial_balance);
        let mut granted = 0;
        let grants = total.div_ceil(per_request);
        for index in 0..grants {
            pic.advance_time(Duration::from_secs(funding.cooldown_secs));
            // Measure each effect separately from earlier elapsed-time charges.
            pic.tick();
            let before_transfer = pic.cycle_balance(hub);
            let mut request = descendant_funding_request(pic, 0xb1);
            request.metadata.request_id[..16].copy_from_slice(&index.to_be_bytes());
            let canic::dto::rpc::Request::Cycles(ref mut cycles) = request.capability else {
                unreachable!();
            };
            cycles.cycles = total.checked_add(1).unwrap();
            let amount = request_descendant_funding(pic, root, hub, request.clone());
            assert_eq!(amount, per_request.min(total - granted));
            granted += amount;
            assert_eq!(pic.cycle_balance(hub), before_transfer + amount);
            // Discard the first receipt and replay its exact identity during cooldown.
            assert_eq!(
                request_descendant_funding(pic, root, hub, request.clone()),
                amount
            );
            assert_eq!(pic.cycle_balance(hub), before_transfer + amount);
            request.metadata.request_id[31] = 0xb2;
            assert!(
                matches!(descendant_funding_response(pic, root, hub, request),
                CyclesResponse::PreflightRejected(
                    CyclesFundingPreflightResponse::CooldownActive { retry_after_secs }
                ) if retry_after_secs > 0 && retry_after_secs <= funding.cooldown_secs)
            );
            assert_eq!(pic.cycle_balance(hub), before_transfer + amount);
        }
        assert_eq!(granted, total);
        pic.advance_time(Duration::from_secs(funding.cooldown_secs));
        pic.tick();
        let exhausted_balance = pic.cycle_balance(hub);
        assert_eq!(
            descendant_funding_response(pic, root, hub, descendant_funding_request(pic, 0xb3)),
            CyclesResponse::PreflightRejected(
                CyclesFundingPreflightResponse::ChildBudgetExhausted {
                    remaining_child_budget: 0,
                    max_per_child: total,
                }
            )
        );
        assert_eq!(pic.cycle_balance(hub), exhausted_balance);
        pic.start_canister(hub, Some(root)).unwrap();
    }

    /// A later application allocation uses retained sources after publication authority leaves.
    #[cfg(test)]
    fn qualify_later_fixture_shard(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        hub: Principal,
        initial_shard: Principal,
        capacity: u32,
    ) {
        // The initial journey already assigned one distinct account.
        for index in 1..capacity {
            let user = Principal::from_slice(&index.to_be_bytes());
            let assigned: Result<Principal, Error> = pic
                .update_candid(hub, "create_account", (user,))
                .expect("fill the configured initial Shard capacity");
            assert_eq!(assigned.unwrap(), initial_shard);
        }
        let before = root_pool_status(pic, fixture.root_id);
        let store = fixture.response.wasm_store;
        let controller = fixture
            .init_args
            .authority
            .wasm_store_authority
            .installation_controller;
        pic.set_controllers(store, Some(controller), vec![fixture.root_id])
            .expect("remove publication controller after the initial Fleet completes");
        pic.stop_canister(store, Some(fixture.root_id))
            .expect("interrupt retained source access");
        let user = Principal::from_slice(&[0xe4; 29]);
        let interrupted: Result<Principal, Error> = pic
            .update_candid(hub, "create_account", (user,))
            .expect("source outage returns an application result");
        assert!(
            interrupted.is_err(),
            "an unavailable source cannot yield a ready assignment"
        );
        pic.start_canister(store, Some(fixture.root_id))
            .expect("restore the retained Store");
        let mut last = None;
        let mut assigned = None;
        for _ in 0..120 {
            let result: Result<Principal, Error> = pic
                .update_candid(hub, "create_account", (user,))
                .expect("retry the same account allocation");
            match result {
                Ok(shard) => {
                    assigned = Some(shard);
                    break;
                }
                Err(error) => last = Some(error),
            }
            pic.advance_time(Duration::from_secs(30));
            pic.tick();
        }
        let shard = assigned.unwrap_or_else(|| panic!("later Shard did not recover: {last:?}"));
        assert_ne!(shard, initial_shard);
        let after = root_pool_status(pic, fixture.root_id);
        assert_eq!(after.workload, before.workload + 1);
        assert_eq!(after.failed, before.failed);
        let response: Result<StoreCatalogResponse, Error> = pic
            .query_candid_as(
                store,
                fixture.root_id,
                canic::protocol::CANIC_WASM_STORE_CATALOG,
                (StoreCatalogRequest::FixtureGrant(shard),),
            )
            .unwrap();
        let StoreCatalogResponse::FixtureGrant(Some(grant)) = response.unwrap() else {
            panic!("the later Shard must receive an autonomous exact grant");
        };
        assert_eq!(grant.revision, 1);
        assert!(grant.enabled);
        assert_eq!(
            grant.binding.target,
            managed_binding_status(pic, fixture.root_id, shard)
        );
        assert_eq!(
            grant.binding.release_build_id,
            fixture.response.release_set.release_build_id
        );
        let source = fixture
            .response
            .fixtures
            .iter()
            .find(|source| source.role.as_str() == "user_shard")
            .unwrap();
        assert_eq!(grant.binding.content_id, source.content_id);
        let readiness = fixture_readiness(pic, fixture.root_id, shard);
        assert_eq!(
            readiness.status,
            canic::dto::runtime::ReadinessStatus::Ready
        );
        let Ok(canic::dto::fixture_provisioning::FixtureProvisioningStatus::Complete(receipt)) =
            readiness.fixture
        else {
            panic!("later placement requires a completed application receipt");
        };
        assert_eq!(receipt.binding, grant.binding);
        assert_eq!(
            receipt.completion_summary,
            source.descriptor.completion_summary
        );
        // Discard the first successful assignment response and reconcile by the same key.
        let replay: Result<Principal, Error> =
            pic.update_candid(hub, "create_account", (user,)).unwrap();
        assert_eq!(replay.unwrap(), shard);
        assert_eq!(root_pool_status(pic, fixture.root_id), after);
        qualify_fixture_revocation_before_recycling(pic, fixture, shard, &grant);
        qualify_fixture_replacement(pic, fixture, hub, shard, &grant);
    }

    /// Unavailable Store authority prevents removal; successful removal revokes before reset.
    #[cfg(test)]
    fn qualify_fixture_revocation_before_recycling(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        shard: Principal,
        grant: &canic::dto::fixture_provisioning::FixtureGrant,
    ) {
        let ManagedCanisterBinding::ComponentChild(binding) = &grant.binding.target else {
            unreachable!()
        };
        let component = binding.component.component;
        let root = fixture.root_id;
        let store = fixture.response.wasm_store;
        let RootStatusResponseFragment::ComponentRegistryPartition(partition) = root_status(
            pic,
            root,
            RootStatusRequestFragment::ComponentRegistryPartition(
                canic::dto::component_registry::ComponentRegistryPartitionRequest { component },
            ),
        )
        .unwrap() else {
            panic!("expected current Component partition");
        };
        let request = canic::dto::component_registry::RootComponentSubtreeRemovalRequest {
            operation_id: [0xe5; 32],
            component,
            target_canister_id: shard,
            expected_registry: partition.head,
        };
        let before = root_pool_status(pic, root)
            .entries
            .into_iter()
            .find(|entry| entry.canister_id == shard)
            .unwrap();
        pic.stop_canister(store, Some(root)).unwrap();
        let unavailable = root_command(
            pic,
            root,
            RootCommandFragment::RemoveSubtree(request.clone()),
        )
        .err()
        .unwrap();
        assert_eq!(
            unavailable,
            Error::from_registered(canic_core::diagnostics::codes::PLATFORM_UNAVAILABLE)
        );
        assert_eq!(
            root_pool_status(pic, root)
                .entries
                .iter()
                .find(|entry| entry.canister_id == shard),
            Some(&before)
        );
        assert!(
            pic.canister_status(shard, Some(root))
                .unwrap()
                .module_hash
                .is_some()
        );
        pic.start_canister(store, Some(root)).unwrap();
        root_command(
            pic,
            root,
            RootCommandFragment::RemoveSubtree(request.clone()),
        )
        .unwrap();
        wait_for_fixture_subtree_removal(pic, root, request.operation_id);
        assert!(
            pic.canister_status(shard, Some(root))
                .unwrap()
                .module_hash
                .is_none()
        );
        assert_fixture_revocation_and_replay(pic, root, store, shard, grant, request);
    }

    #[cfg(test)]
    fn wait_for_fixture_subtree_removal(pic: &PocketIc, root: Principal, operation_id: [u8; 32]) {
        let mut latest = None;
        for _ in 0..180 {
            let response = root_status(
                pic,
                root,
                RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
            )
            .unwrap();
            if let RootStatusResponseFragment::Operation(
                RootOperationStatusResponse::RemoveSubtree(status),
            ) = response
            {
                let completed = matches!(
                    status.phase,
                    canic::dto::component_registry::RootComponentSubtreeRemovalPhase::Completed(_)
                );
                latest = Some(status);
                if completed {
                    break;
                }
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        assert!(
            latest.as_ref().is_some_and(|status| matches!(
                status.phase,
                canic::dto::component_registry::RootComponentSubtreeRemovalPhase::Completed(_)
            )),
            "subtree removal must recover after Store returns: {latest:?}"
        );
    }

    #[cfg(test)]
    fn assert_fixture_revocation_and_replay(
        pic: &PocketIc,
        root: Principal,
        store: Principal,
        shard: Principal,
        grant: &canic::dto::fixture_provisioning::FixtureGrant,
        request: canic::dto::component_registry::RootComponentSubtreeRemovalRequest,
    ) {
        let read_grant = || {
            let response: Result<StoreCatalogResponse, Error> = pic
                .query_candid_as(
                    store,
                    root,
                    canic::protocol::CANIC_WASM_STORE_CATALOG,
                    (StoreCatalogRequest::FixtureGrant(shard),),
                )
                .unwrap();
            let StoreCatalogResponse::FixtureGrant(Some(revoked)) = response.unwrap() else {
                panic!("revoked grant remains retained");
            };
            revoked
        };
        let revoked = read_grant();
        assert_eq!(revoked.binding, grant.binding);
        assert_eq!(revoked.revision, grant.revision + 1);
        assert!(!revoked.enabled);
        let before = root_pool_status(pic, root)
            .entries
            .into_iter()
            .find(|entry| entry.canister_id == shard)
            .unwrap();
        assert_eq!(before.status, CanisterPoolAssetStatus::Ready);
        root_command(pic, root, RootCommandFragment::RemoveSubtree(request)).unwrap();
        assert_eq!(
            root_pool_status(pic, root)
                .entries
                .iter()
                .find(|entry| entry.canister_id == shard),
            Some(&before)
        );
        assert_eq!(read_grant(), revoked);
        let StoreCommandResponse::FixtureGrant(stale) = store_command_as(
            pic,
            store,
            root,
            StoreCommand::SetFixtureGrant(Box::new(
                canic::dto::fixture_provisioning::FixtureGrantRequest {
                    expected_revision: 0,
                    binding: grant.binding.clone(),
                    enabled: true,
                },
            )),
        )
        .unwrap() else {
            panic!("exact grant response");
        };
        assert_eq!(*stale, Err(FixtureStoreError::Conflict));
    }

    /// A new operation reuses the physical target with a retained successor grant.
    #[cfg(test)]
    fn qualify_fixture_replacement(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        hub: Principal,
        shard: Principal,
        previous: &canic::dto::fixture_provisioning::FixtureGrant,
    ) {
        let root = fixture.root_id;
        pic.stop_canister(shard, Some(root)).unwrap();
        let operation_id: [u8; 32] = [0xe6; 32];
        let mut last = None;
        let mut replacement = None;
        for _ in 0..120 {
            let result: Result<Principal, Error> = pic
                .update_candid_as(hub, root, "test_create_fixture_child", (operation_id,))
                .unwrap();
            match result {
                Ok(canister) => {
                    replacement = Some(canister);
                    break;
                }
                Err(error) => last = Some(error),
            }
            pic.advance_time(Duration::from_secs(30));
            pic.tick();
        }
        if replacement.is_none() {
            fixture_replacement_diagnostics(pic, root, shard);
        }
        assert_eq!(
            replacement,
            Some(shard),
            "replacement must reuse the recycled asset: {last:?}"
        );
        let store = fixture.response.wasm_store;
        let response: Result<StoreCatalogResponse, Error> = pic
            .query_candid_as(
                store,
                root,
                canic::protocol::CANIC_WASM_STORE_CATALOG,
                (StoreCatalogRequest::FixtureGrant(shard),),
            )
            .unwrap();
        let StoreCatalogResponse::FixtureGrant(Some(grant)) = response.unwrap() else {
            panic!("replacement grant must be retained");
        };
        assert_eq!(grant.revision, previous.revision + 2);
        assert!(grant.enabled);
        assert_ne!(grant.binding.installation, previous.binding.installation);
        assert_eq!(grant.binding.content_id, previous.binding.content_id);
        assert_eq!(
            grant.binding.target,
            managed_binding_status(pic, root, shard)
        );
        let readiness = fixture_readiness(pic, root, shard);
        assert_eq!(
            readiness.status,
            canic::dto::runtime::ReadinessStatus::Ready
        );
        let Ok(canic::dto::fixture_provisioning::FixtureProvisioningStatus::Complete(receipt)) =
            readiness.fixture
        else {
            panic!("replacement requires its own completed import");
        };
        assert_eq!(receipt.binding, grant.binding);
        let replay: Result<Principal, Error> = pic
            .update_candid_as(hub, root, "test_create_fixture_child", (operation_id,))
            .unwrap();
        assert_eq!(replay.unwrap(), shard);
        let StoreCommandResponse::FixtureGrant(stale) = store_command_as(
            pic,
            store,
            root,
            StoreCommand::SetFixtureGrant(Box::new(
                canic::dto::fixture_provisioning::FixtureGrantRequest {
                    expected_revision: previous.revision,
                    binding: previous.binding.clone(),
                    enabled: false,
                },
            )),
        )
        .unwrap() else {
            panic!("exact grant response");
        };
        assert_eq!(*stale, Err(FixtureStoreError::Conflict));
        let response: Result<StoreCatalogResponse, Error> = pic
            .query_candid_as(
                store,
                root,
                canic::protocol::CANIC_WASM_STORE_CATALOG,
                (StoreCatalogRequest::FixtureGrant(shard),),
            )
            .unwrap();
        let StoreCatalogResponse::FixtureGrant(retained) = response.unwrap() else {
            panic!("exact grant response");
        };
        assert_eq!(retained, Some(grant));
    }

    #[cfg(test)]
    fn fixture_replacement_diagnostics(pic: &PocketIc, root: Principal, shard: Principal) {
        for entry in root_pool_status(pic, root).entries {
            if entry.canister_id == shard {
                eprintln!("replacement target pool entry: {entry:?}");
            }
            if let CanisterPoolAssetStatus::Workload { claim } = entry.status {
                let response = root_status(
                    pic,
                    root,
                    RootStatusRequestFragment::Operation(OperationStatusRequest {
                        operation_id: claim.operation_id,
                    }),
                );
                if let Ok(RootStatusResponseFragment::Operation(
                    RootOperationStatusResponse::ProvisionChild(operation),
                )) = response
                {
                    eprintln!(
                        "retained child phase: {:?}, creation: {:?}",
                        operation.allocation.phase, operation.allocation.creation
                    );
                }
            }
        }
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one production-boundary journey keeps Prepared-Root child bootstrap, terminal activation and replay together"
    )]
    fn prepared_root_initial_shard_bootstrap_reaches_terminal_component_membership() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = initial_shard_root_canister_config_path(&workspace_root);
        let config = AppConfigSnapshot::load(&config_path).expect("load initial-Shard config");
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let fixture = install_initial_shard_fixture(
            &pic,
            coordinator,
            &config_path,
            &["user_hub", "user_shard"],
        );
        let (joining_version, sync_request) =
            join_and_synchronize_root(&pic, coordinator, &fixture);
        let component_registry_request = activate_registry_and_prepare_component_registry(
            &pic,
            coordinator,
            &fixture,
            joining_version,
            sync_request,
        );
        let CoordinatorRegistryResponse::Registry(registry) =
            coordinator_status(&pic, coordinator, CoordinatorRegistryRequest::Registry)
                .expect("query active Registry");
        let operation_id = [0x71; 32];
        let compiled = fixture_fresh_component_plan(config.model(), &registry, operation_id);
        let request = compiled.request;
        let CoordinatorCommandResponse::OperationAccepted(first_receipt) = coordinator_command(
            &pic,
            coordinator,
            CoordinatorCommand::ProvisionComponents(request.clone()),
        )
        .expect("begin one-Hub Component provisioning") else {
            panic!("Coordinator returned a differently correlated provisioning response");
        };
        assert_eq!(first_receipt.operation_id, operation_id);
        let CoordinatorCommandResponse::OperationAccepted(replayed_receipt) = coordinator_command(
            &pic,
            coordinator,
            CoordinatorCommand::ProvisionComponents(request.clone()),
        )
        .expect("replay the response-lost provisioning command") else {
            panic!("Coordinator returned a differently correlated replay response");
        };
        assert_eq!(replayed_receipt, first_receipt);
        qualify_pending_fixture_bootstrap(
            &pic,
            &fixture,
            operation_id,
            &config.model().component_specs["users"].cycles_funding,
        );

        let mut last_status = None;
        let terminal = (0..240).find_map(|_| {
            let status = coordinator_status(
                &pic,
                coordinator,
                CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .expect("query initial-Shard Component provisioning");
            let CoordinatorOperationReadResponse::Operation(
                CoordinatorOperationStatusResponse::ComponentProvisioning(status),
            ) = status
            else {
                panic!("Coordinator returned a differently correlated operation status");
            };
            if status.components_provisioned_at_ns.is_some()
                && status.runtimes_activated_at_ns.is_some()
                && status.runtime_activated_root_count == status.root_batch_count
            {
                Some(status)
            } else {
                last_status = Some(status);
                pic.advance_time(Duration::from_secs(1));
                pic.tick();
                None
            }
        });
        let terminal = terminal.unwrap_or_else(|| {
            let pool = root_pool_status(&pic, fixture.root_id);
            let workload_callers = pool
                .entries
                .iter()
                .filter(|entry| {
                    matches!(entry.status, CanisterPoolAssetStatus::Workload { .. })
                })
                .map(|entry| entry.canister_id)
                .collect::<Vec<_>>();
            let child_operations = pool
                .entries
                .iter()
                .filter_map(|entry| match &entry.status {
                    CanisterPoolAssetStatus::Workload { claim } => Some(claim.operation_id),
                    _ => None,
                })
                .filter_map(|operation_id| {
                    workload_callers.iter().find_map(|caller| {
                        let response: Result<RootStatusResponseFragment, Error> = pic
                            .query_candid_as(
                                fixture.root_id,
                                *caller,
                                canic::protocol::CANIC_ROOT_OPERATION_STATUS,
                                (RootStatusRequestFragment::Operation(OperationStatusRequest {
                                    operation_id,
                                }),),
                            )
                            .expect("Root child-operation status transport");
                        match response {
                            Ok(RootStatusResponseFragment::Operation(
                                RootOperationStatusResponse::ProvisionChild(status),
                            )) => Some(format!(
                                "caller={caller} operation={operation_id:?} phase={:?}",
                                status.allocation.phase
                            )),
                            _ => None,
                        }
                    })
                })
                .collect::<Vec<_>>();
            let workload_readiness = pool
                .entries
                .iter()
                .filter(|entry| {
                    matches!(entry.status, CanisterPoolAssetStatus::Workload { .. })
                })
                .map(|entry| {
                    let readiness = fetch_role_overview_readiness(
                        &pic,
                        entry.canister_id,
                    )
                        .map_or_else(|error| format!("query-error={error}"), |status| status.to_string());
                    format!("canister={} {readiness}", entry.canister_id)
                })
                .collect::<Vec<_>>();
            let component_registry_summary = match root_status(
                &pic,
                fixture.root_id,
                RootStatusRequestFragment::ComponentRegistry(component_registry_request.clone()),
            ) {
                Ok(RootStatusResponseFragment::ComponentRegistry(status)) => format!(
                    "committed={} descendants={} created={}",
                    status.committed_component_instances,
                    status.managed_descendants,
                    status.known_created_component_canisters
                ),
                Ok(_) => "differently correlated Root status".to_string(),
                Err(error) => format!("error={error}"),
            };
            let root_provisioning_summary = match root_status(
                &pic,
                fixture.root_id,
                RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
            ) {
                Ok(RootStatusResponseFragment::Operation(
                    RootOperationStatusResponse::ProvisionComponents(status),
                )) => format!(
                    "phase={:?} activated={}/{} root_active={} accepted={} published={:?} activation_started={:?} completed={:?} receipt={:?}",
                    status.phase,
                    status.activated_component_count,
                    status.component_count,
                    status.root_runtime_active,
                    status.accepted_at_ns,
                    status.published_at_ns,
                    status.activation_started_at_ns,
                    status.runtimes_activated_at_ns,
                    status.receipt_content_hash
                ),
                Ok(_) => "differently correlated Root operation status".to_string(),
                Err(error) => format!("error={error}"),
            };
            report_canister_diagnostics_batch(
                &pic,
                [("Root".to_string(), fixture.root_id, Principal::anonymous())],
                "Prepared-Root initial-Shard bootstrap",
            );
            let coordinator_summary = last_status.as_ref().map(|status| {
                format!(
                    "phase={:?} activated_roots={}/{} components={} failure={:?}",
                    status.phase,
                    status.runtime_activated_root_count,
                    status.root_batch_count,
                    status.component_count,
                    status.pending_root_failure
                )
            });
            panic!(
                "initial-Shard Component provisioning did not become terminal: coordinator={coordinator_summary:?} root_provisioning={root_provisioning_summary:?} child_operations={child_operations:?} workload_readiness={workload_readiness:?} component_registry={component_registry_summary:?} pool_ready={} pool_workload={} pool_pending_reset={} pool_failed={}",
                pool.ready,
                pool.workload,
                pool.pending_reset,
                pool.failed
            );
        });
        assert_eq!(terminal.component_count, 3);
        assert!(terminal.pending_root_failure.is_none());

        let RootStatusResponseFragment::ComponentRegistry(component_registry) = root_status(
            &pic,
            fixture.root_id,
            RootStatusRequestFragment::ComponentRegistry(component_registry_request),
        )
        .expect("query terminal initial-Shard Component Registry") else {
            panic!("Root returned a differently correlated Component Registry status");
        };
        assert_eq!(component_registry.committed_component_instances, 3);
        assert_eq!(component_registry.managed_descendants, 2);
        assert_eq!(component_registry.known_created_component_canisters, 5);

        let terminal_pool = root_pool_status(&pic, fixture.root_id);
        assert_eq!(terminal_pool.workload, 5);
        let workload_principals = terminal_pool
            .entries
            .iter()
            .filter(|entry| matches!(entry.status, CanisterPoolAssetStatus::Workload { .. }))
            .map(|entry| entry.canister_id)
            .collect::<Vec<_>>();
        assert_eq!(workload_principals.len(), 5);
        let bindings = workload_principals
            .iter()
            .copied()
            .map(|canister| {
                (
                    canister,
                    managed_binding_status(&pic, fixture.root_id, canister),
                )
            })
            .collect::<Vec<_>>();
        let (hub, hub_binding) = bindings
            .iter()
            .find(|(_, binding)| {
                matches!(
                    binding,
                    ManagedCanisterBinding::Component(binding)
                        if binding.role.as_str() == "user_hub"
                )
            })
            .expect("one top-level user Hub binding");
        let (_, child_binding) = bindings
            .iter()
            .find(|(_, binding)| {
                matches!(
                    binding,
                    ManagedCanisterBinding::ComponentChild(binding)
                        if binding.role.as_str() == "user_shard"
                )
            })
            .expect("one initial user Shard binding");
        let ManagedCanisterBinding::Component(hub_binding) = hub_binding else {
            unreachable!("selected top-level binding")
        };
        let ManagedCanisterBinding::ComponentChild(child_binding) = child_binding else {
            unreachable!("selected child binding")
        };
        assert_eq!(hub_binding.role.as_str(), "user_hub");
        assert_eq!(child_binding.role.as_str(), "user_shard");
        assert_eq!(child_binding.parent_canister_id, *hub);
        assert_eq!(child_binding.component, *hub_binding);
        let store = fixture.response.wasm_store;
        let controller = fixture
            .init_args
            .authority
            .wasm_store_authority
            .installation_controller;
        let grants = bindings
            .iter()
            .filter_map(|(canister, binding)| {
                let response: Result<StoreCatalogResponse, Error> = pic
                    .query_candid_as(
                        store,
                        controller,
                        canic::protocol::CANIC_WASM_STORE_CATALOG,
                        (StoreCatalogRequest::FixtureGrant(*canister),),
                    )
                    .unwrap();
                let StoreCatalogResponse::FixtureGrant(grant) = response.unwrap() else {
                    panic!("Store returned a different status variant");
                };
                let role = match binding {
                    ManagedCanisterBinding::Component(binding) => &binding.role,
                    ManagedCanisterBinding::ComponentChild(binding) => &binding.role,
                };
                let Some(source) = fixture
                    .response
                    .fixtures
                    .iter()
                    .find(|source| &source.role == role)
                else {
                    assert!(
                        grant.is_none(),
                        "unselected roles must not receive fixture access"
                    );
                    return None;
                };
                let grant =
                    grant.expect("selected target must have an autonomous Root-derived grant");
                assert_eq!(&grant.binding.target, binding);
                assert!(grant.enabled);
                assert_eq!(grant.revision, 1);
                assert_eq!(
                    grant.binding.release_build_id,
                    fixture.response.release_set.release_build_id
                );
                assert_eq!(grant.binding.content_id, source.content_id);
                let runtime: Result<ManagedStatusResponseFragment, Error> = pic
                    .query_candid_as(
                        *canister,
                        fixture.root_id,
                        canic::protocol::CANIC_CONTROL_STATUS,
                        (ManagedStatusRequestFragment::Operation(
                            OperationStatusRequest {
                                operation_id: grant.binding.installation,
                            },
                        ),),
                    )
                    .unwrap();
                let ManagedStatusResponseFragment::Operation(operation) = runtime.unwrap() else {
                    panic!("expected exact installed runtime operation");
                };
                let ManagedOperationStatusResponseFragment::ConfigureRuntime(runtime) = *operation;
                assert_eq!(runtime.runtime.operation_id, grant.binding.installation);
                assert_eq!(&runtime.runtime.binding, binding);
                let assignment = runtime
                    .runtime
                    .fixture
                    .expect("installed fixture assignment");
                assert_eq!(assignment.store, store);
                assert_eq!(assignment.grant, *grant);
                assert_eq!(assignment.descriptor, source.descriptor);
                let ready = fixture_readiness(&pic, fixture.root_id, *canister);
                let Ok(canic::dto::fixture_provisioning::FixtureProvisioningStatus::Complete(
                    receipt,
                )) = ready.fixture
                else {
                    panic!("terminal membership requires an exact durable fixture receipt");
                };
                assert_eq!(receipt.binding, grant.binding);
                assert_eq!(
                    receipt.completion_summary,
                    source.descriptor.completion_summary
                );
                Some((*canister, *grant))
            })
            .collect::<Vec<_>>();
        for canister in workload_principals {
            assert!(matches!(
                fetch_role_overview_readiness(&pic, canister)
                    .expect("query managed initial-Shard readiness"),
                RoleOverviewReadinessObservation::Ready
            ));
        }

        let user = Principal::from_slice(&[0xe3; 29]);
        for _ in 0..2 {
            let assigned: Result<Principal, Error> = pic
                .update_candid(*hub, "create_account", (user,))
                .expect("ready Hub application dispatch");
            assert_eq!(assigned.unwrap(), child_binding.canister_id);
        }
        assert_eq!(
            root_pool_status(&pic, fixture.root_id).workload,
            terminal_pool.workload
        );

        let RootStatusResponseFragment::Operation(
            RootOperationStatusResponse::ProvisionComponents(root_terminal),
        ) = root_status(
            &pic,
            fixture.root_id,
            RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
        )
        .expect("query terminal Root provisioning result")
        else {
            panic!("Root returned a differently correlated terminal operation");
        };
        let result = root_terminal.result.expect("terminal Root result");
        let placement = result
            .placements
            .iter()
            .find(|placement| {
                placement
                    .members
                    .iter()
                    .any(|member| member.binding.canister_id == *hub)
            })
            .expect("placement containing the initial-Shard Hub");
        let member = placement
            .members
            .iter()
            .find(|member| member.binding.canister_id == *hub)
            .expect("initial-Shard Hub terminal member");
        let RootStatusResponseFragment::ComponentRegistryActivePartition(coverage) = root_status(
            &pic,
            fixture.root_id,
            RootStatusRequestFragment::ComponentRegistryActivePartition(
                ComponentRegistryActivePartitionRequest {
                    component: member.binding.component,
                    provisioning_operation_id: operation_id,
                    plan_hash: root_terminal.plan_hash,
                    group_placement: placement.group_placement.clone(),
                    member_path: member.member_path.clone(),
                },
            ),
        )
        .expect("query Root-qualified active partition coverage") else {
            panic!("Root returned a differently correlated partition coverage");
        };
        assert_eq!(coverage.allocation_operation_id, member.member_operation_id);
        assert_eq!(
            coverage.prepared.head.revision,
            member.component_registry_revision
        );
        assert_eq!(
            coverage.prepared.head.content_hash,
            member.component_registry_content_hash
        );
        assert_eq!(coverage.prepared.status, ComponentLifecycleStatus::Prepared);
        assert_eq!(coverage.activation.status, ComponentLifecycleStatus::Active);
        assert_eq!(coverage.current.status, ComponentLifecycleStatus::Active);
        assert!(coverage.activation.head.revision > coverage.prepared.head.revision);
        assert!(coverage.current.head.revision >= coverage.activation.head.revision);
        assert_eq!(coverage.current.committed_descendants, 1);

        let replay_pool = root_pool_status(&pic, fixture.root_id);
        let CoordinatorCommandResponse::OperationAccepted(terminal_receipt) = coordinator_command(
            &pic,
            coordinator,
            CoordinatorCommand::ProvisionComponents(request),
        )
        .expect("replay terminal initial-Shard provisioning") else {
            panic!("Coordinator returned a differently correlated terminal replay");
        };
        assert_eq!(terminal_receipt, first_receipt);
        assert_eq!(root_pool_status(&pic, fixture.root_id), replay_pool);
        for (canister, grant) in grants {
            let response: Result<StoreCatalogResponse, Error> = pic
                .query_candid_as(
                    store,
                    controller,
                    canic::protocol::CANIC_WASM_STORE_CATALOG,
                    (StoreCatalogRequest::FixtureGrant(canister),),
                )
                .unwrap();
            let StoreCatalogResponse::FixtureGrant(Some(replayed)) = response.unwrap() else {
                panic!("terminal replay must retain the exact grant");
            };
            assert_eq!(*replayed, grant);
        }
        qualify_later_fixture_shard(
            &pic,
            &fixture,
            *hub,
            child_binding.canister_id,
            config.model().component_specs["users"]
                .sharding
                .as_ref()
                .unwrap()
                .pools["user_shards"]
                .policy
                .capacity,
        );
        assert_retained_fixtures(&pic, &fixture);
        let CoordinatorRegistryResponse::Registry(before) =
            coordinator_status(&pic, coordinator, CoordinatorRegistryRequest::Registry).unwrap();
        let CoordinatorObservabilityResponse::RegistryVersion(version) = coordinator_status(
            &pic,
            coordinator,
            CoordinatorObservabilityRequest::RegistryVersion,
        )
        .unwrap() else {
            panic!("Registry version correlation");
        };
        let Err(rejected) = coordinator_command(
            &pic,
            coordinator,
            CoordinatorCommand::RemoveRoot(FleetSubnetRootDrainingReservationRequest {
                asset_recipient: Principal::from_slice(&[0xe7; 29]),
                operation_id: [0xd1; 32],
                expected_registry: version,
                expected_root: before
                    .fleet_subnet_roots
                    .iter()
                    .find(|entry| entry.fleet_subnet_root == fixture.root_id)
                    .unwrap()
                    .clone(),
            }),
        ) else {
            panic!("grouped service references must fence standalone Root retirement");
        };
        assert_eq!(
            rejected.code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );
        let CoordinatorRegistryResponse::Registry(after) =
            coordinator_status(&pic, coordinator, CoordinatorRegistryRequest::Registry).unwrap();
        assert_eq!(after, before);
        assert_retained_fixtures(&pic, &fixture);
    }

    #[cfg(test)]
    fn assert_retained_fixtures(pic: &PocketIc, fixture: &BootstrappedRootFixture) {
        let store = fixture.response.wasm_store;
        for source in &fixture.response.fixtures {
            let retained: Result<StoreCatalogResponse, Error> = pic
                .query_candid_as(
                    store,
                    fixture.root_id,
                    canic::protocol::CANIC_WASM_STORE_CATALOG,
                    (StoreCatalogRequest::Fixture(source.content_id),),
                )
                .expect("retained fixture catalog before whole-Root retirement");
            let Ok(StoreCatalogResponse::Fixture(Ok(retained))) = retained else {
                panic!("completed imports must retain their exact source");
            };
            assert_eq!(retained.content_id, source.content_id);
            assert_eq!(retained.received_bytes, source.descriptor.encoded_length);
            assert_eq!(retained.next_chunk, retained.chunk_count);
            assert!(retained.complete);
        }
    }

    #[cfg(test)]
    fn install_initial_shard_fixture(
        pic: &PocketIc,
        coordinator: Principal,
        config_path: &Path,
        fixture_roles: &[&str],
    ) -> BootstrappedRootFixture {
        install_fixture_root(
            pic,
            coordinator,
            config_path,
            fixture_roles,
            build_initial_shard_root_wasm(),
            build_initial_shard_component_wasms(),
        )
    }

    #[cfg(test)]
    fn install_fixture_root(
        pic: &PocketIc,
        coordinator: Principal,
        config_path: &Path,
        fixture_roles: &[&str],
        root_wasm: Vec<u8>,
        component_wasms: &BTreeMap<CanisterRole, Vec<u8>>,
    ) -> BootstrappedRootFixture {
        install_fixture_root_with_store(
            pic,
            coordinator,
            config_path,
            fixture_roles,
            root_wasm,
            component_wasms,
            None,
        )
    }

    #[cfg(test)]
    fn install_fixture_root_with_store(
        pic: &PocketIc,
        coordinator: Principal,
        config_path: &Path,
        fixture_roles: &[&str],
        root_wasm: Vec<u8>,
        component_wasms: &BTreeMap<CanisterRole, Vec<u8>>,
        store_wasm: Option<Vec<u8>>,
    ) -> BootstrappedRootFixture {
        let coordinator_wasm = build_test_coordinator_wasm();
        let mut store_fixture = build_root_store_fixture_with_config(config_path, component_wasms);
        store_fixture.wasm = store_wasm;
        let payload = b"reviewed fixture source";
        for (index, role) in ["user_hub", "user_shard"].into_iter().enumerate() {
            if !fixture_roles.contains(&role) {
                continue;
            }
            let descriptor = canic::dto::fixture_provisioning::FixtureDescriptor {
                schema_version: 1,
                format_hash: [u8::try_from(index + 1).unwrap(); 32],
                encoded_length: payload.len() as u64,
                chunks: vec![canic::dto::fixture_provisioning::FixtureChunkDescriptor {
                    digest: wasm_hash(payload).try_into().unwrap(),
                    length: u32::try_from(payload.len()).unwrap(),
                }],
                completion_summary: [3; 32],
            };
            store_fixture
                .manifest
                .fixtures
                .push(canic::dto::root_store::RootStoreFixture {
                    role: CanisterRole::new(role),
                    content_id:
                        canic_control_plane::api::fixture_content::FixtureContentApi::content_id(
                            &descriptor,
                        )
                        .unwrap(),
                    descriptor,
                });
        }
        let fixture = install_bootstrapped_root_with_config_and_pool_setup(
            pic,
            root_wasm,
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: None,
                canister_pool_minimum_size: None,
                canister_pool_cycles: None,
                coordinator_subnet: None,
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: None,
                component_admission_limits: None,
                fleet_id: None,
                funding: None,
                coordinator_root_funding: None,
            },
            config_path,
            create_prepaid_pool_assets,
        );
        install_fixture_coordinator_with_config(
            pic,
            coordinator,
            coordinator_wasm,
            &fixture,
            config_path,
        );
        fixture
    }

    #[cfg(test)]
    #[derive(CandidType)]
    enum FixtureHeldReply {
        Grant,
        Revoke,
    }

    #[cfg(test)]
    fn arm_fixture_reply(
        pic: &PocketIc,
        store: Principal,
        root: Principal,
        kind: FixtureHeldReply,
    ) {
        assert!(
            pic.update_candid_as::<bool, _>(store, root, "canic_test_fixture_reply_arm", (kind,))
                .unwrap()
        );
    }

    #[cfg(test)]
    fn fixture_replies_waiting(pic: &PocketIc, store: Principal, root: Principal) -> u32 {
        pic.query_candid_as(store, root, "canic_test_fixture_reply_waiting", ())
            .unwrap()
    }

    #[cfg(test)]
    #[track_caller]
    fn wait_for_held_fixture_reply(pic: &PocketIc, store: Principal, root: Principal) {
        let mut waiting = 0;
        for _ in 0..180 {
            waiting = fixture_replies_waiting(pic, store, root);
            if waiting == 1 {
                return;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        panic!("Root must reach one held real Store reply; observed {waiting}");
    }

    #[cfg(test)]
    fn held_fixture_grant(
        pic: &PocketIc,
        store: Principal,
        root: Principal,
        target: Principal,
    ) -> canic::dto::fixture_provisioning::FixtureGrant {
        let response: Result<StoreCatalogResponse, Error> = pic
            .query_candid_as(
                store,
                root,
                canic::protocol::CANIC_WASM_STORE_CATALOG,
                (StoreCatalogRequest::FixtureGrant(target),),
            )
            .unwrap();
        let StoreCatalogResponse::FixtureGrant(Some(grant)) = response.unwrap() else {
            panic!("actual Store grant");
        };
        *grant
    }

    #[test]
    fn root_restart_reconciles_held_store_grant_and_revocation_replies() {
        let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = initial_shard_root_canister_config_path(&workspace);
        let config = AppConfigSnapshot::load(&config_path).unwrap();
        let root_wasm = build_initial_shard_root_wasm();
        let store_wasm = crate::pic::held_fixture_store_wasm();
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let fixture = install_fixture_root_with_store(
            &pic,
            coordinator,
            &config_path,
            &["user_shard"],
            root_wasm.clone(),
            build_initial_shard_component_wasms(),
            Some(store_wasm),
        );
        let root = fixture.root_id;
        let store = fixture.response.wasm_store;
        let (version, sync) = join_and_synchronize_root(&pic, coordinator, &fixture);
        activate_registry_and_prepare_component_registry(
            &pic,
            coordinator,
            &fixture,
            version,
            sync,
        );
        let CoordinatorRegistryResponse::Registry(registry) =
            coordinator_status(&pic, coordinator, CoordinatorRegistryRequest::Registry).unwrap();
        let request = fixture_fresh_component_plan(config.model(), &registry, [0xc7; 32]).request;
        arm_fixture_reply(&pic, store, root, FixtureHeldReply::Grant);
        coordinator_command(
            &pic,
            coordinator,
            CoordinatorCommand::ProvisionComponents(request),
        )
        .unwrap();
        wait_for_held_fixture_reply(&pic, store, root);
        let selected = selected_fixture_targets(&pic, root);
        let shard = selected
            .iter()
            .find(|(_, role)| role.as_str() == "user_shard")
            .unwrap()
            .0;
        let grant = held_fixture_grant(&pic, store, root, shard);
        assert!(grant.enabled);
        assert_eq!(grant.revision, 1);
        assert_ne!(
            fixture_readiness(&pic, root, shard).status,
            canic::dto::runtime::ReadinessStatus::Ready
        );
        restart_root_with_held_fixture_reply(&pic, root, store, root_wasm.clone());
        assert_eq!(fixture_replies_waiting(&pic, store, root), 1);
        release_fixture_reply(&pic, store, root);
        for _ in 0..240 {
            if fixture_readiness(&pic, root, shard).status
                == canic::dto::runtime::ReadinessStatus::Ready
            {
                break;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        assert_eq!(
            fixture_readiness(&pic, root, shard).status,
            canic::dto::runtime::ReadinessStatus::Ready
        );
        assert_eq!(selected_fixture_targets(&pic, root), selected);
        assert_eq!(held_fixture_grant(&pic, store, root, shard), grant);
        // A child receipt precedes Fleet activation. Removal must wait for the
        // existing Coordinator owner to finish that transition before admission.
        wait_for_fixture_fleet_activation(&pic, coordinator, [0xc7; 32]);
        qualify_held_fixture_revocation(&pic, &fixture, shard, &grant, root_wasm);
    }

    #[cfg(test)]
    fn release_fixture_reply(pic: &PocketIc, store: Principal, root: Principal) {
        pic.update_candid_as::<(), _>(store, root, "canic_test_fixture_reply_release", ())
            .unwrap();
        for _ in 0..40 {
            if fixture_replies_waiting(pic, store, root) == 0 {
                return;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        panic!("released Store response must drain before another barrier is armed");
    }

    /// Stop during the held call, drain its bounded response, then replace the heap.
    #[cfg(test)]
    fn restart_root_with_held_fixture_reply(
        pic: &PocketIc,
        root: Principal,
        store: Principal,
        wasm: Vec<u8>,
    ) {
        let stopping = pic
            .submit_call_with_effective_principal(
                Principal::management_canister(),
                RawEffectivePrincipal::CanisterId(root.as_slice().to_vec()),
                Principal::anonymous(),
                "stop_canister",
                encode_one(QualificationCanisterIdRecord { canister_id: root }).unwrap(),
            )
            .unwrap();
        // Admit stop before advancing beyond the pinned CDK's 300-second
        // bounded-call timeout. Store still retains its successful effect/reply.
        for _ in 0..5 {
            pic.tick();
        }
        pic.advance_time(Duration::from_secs(301));
        pic.await_call(stopping)
            .expect("bounded outstanding calls must drain before Root restart");
        assert_eq!(fixture_replies_waiting(pic, store, root), 1);
        pic.upgrade_canister(root, wasm, crate::pic::upgrade_args(), None)
            .unwrap();
        pic.start_canister(root, None).unwrap();
    }

    #[cfg(test)]
    fn wait_for_fixture_fleet_activation(
        pic: &PocketIc,
        coordinator: Principal,
        operation_id: [u8; 32],
    ) {
        let mut last = None;
        for _ in 0..240 {
            let CoordinatorOperationReadResponse::Operation(
                CoordinatorOperationStatusResponse::ComponentProvisioning(status),
            ) = coordinator_status(
                pic,
                coordinator,
                CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .unwrap()
            else {
                panic!("exact fixture provisioning operation");
            };
            if status.components_provisioned_at_ns.is_some()
                && status.runtimes_activated_at_ns.is_some()
                && status.runtime_activated_root_count == status.root_batch_count
            {
                assert!(status.pending_root_failure.is_none());
                return;
            }
            last = Some(status);
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        panic!("fixture Fleet must reach terminal activation: {last:?}");
    }

    #[cfg(test)]
    fn qualify_held_fixture_revocation(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        shard: Principal,
        grant: &canic::dto::fixture_provisioning::FixtureGrant,
        root_wasm: Vec<u8>,
    ) {
        let root = fixture.root_id;
        let store = fixture.response.wasm_store;
        let ManagedCanisterBinding::ComponentChild(binding) = &grant.binding.target else {
            unreachable!()
        };
        let component = binding.component.component;
        let RootStatusResponseFragment::ComponentRegistryPartition(partition) = root_status(
            pic,
            root,
            RootStatusRequestFragment::ComponentRegistryPartition(
                canic::dto::component_registry::ComponentRegistryPartitionRequest { component },
            ),
        )
        .unwrap() else {
            panic!("Component partition");
        };
        let request = canic::dto::component_registry::RootComponentSubtreeRemovalRequest {
            operation_id: [0xc9; 32],
            component,
            target_canister_id: shard,
            expected_registry: partition.head,
        };
        arm_fixture_reply(pic, store, root, FixtureHeldReply::Revoke);
        let _message = pic
            .submit_call(
                root,
                Principal::anonymous(),
                canic::protocol::CANIC_ROOT_COMMAND,
                encode_one(RootCommandFragment::RemoveSubtree(request.clone())).unwrap(),
            )
            .unwrap();
        wait_for_held_fixture_reply(pic, store, root);
        let revoked = held_fixture_grant(pic, store, root, shard);
        assert!(!revoked.enabled);
        assert_eq!(revoked.binding, grant.binding);
        assert_eq!(revoked.revision, grant.revision + 1);
        assert!(
            pic.canister_status(shard, Some(root))
                .unwrap()
                .module_hash
                .is_some()
        );
        restart_root_with_held_fixture_reply(pic, root, store, root_wasm);
        assert_eq!(fixture_replies_waiting(pic, store, root), 1);
        release_fixture_reply(pic, store, root);
        root_command(
            pic,
            root,
            RootCommandFragment::RemoveSubtree(request.clone()),
        )
        .unwrap();
        wait_for_fixture_subtree_removal(pic, root, request.operation_id);
        assert!(
            pic.canister_status(shard, Some(root))
                .unwrap()
                .module_hash
                .is_none()
        );
        assert_fixture_revocation_and_replay(pic, root, store, shard, grant, request);
    }

    #[test]
    fn pending_fixture_automatically_funds_within_configured_allowance() {
        let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = workspace.join("apps/test/test-configs/fixture-automatic-funding.toml");
        let config = AppConfigSnapshot::load(&config_path).unwrap();
        let root_wasm = crate::pic::artifacts::build_generated_fleet_wasm(
            &workspace,
            &config_path,
            "root",
            CanicWasmBuildProfile::Fast,
        );
        let components = build_component_fixture_wasms(
            &workspace,
            &config_path,
            "fleet-fixture-automatic-funding",
            &[
                ("user_hub", "canister_user_hub"),
                ("user_shard", "canister_user_shard"),
            ],
        );
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let fixture = install_fixture_root(
            &pic,
            coordinator,
            &config_path,
            &["user_hub", "user_shard"],
            root_wasm,
            &components,
        );
        let (version, sync) = join_and_synchronize_root(&pic, coordinator, &fixture);
        activate_registry_and_prepare_component_registry(
            &pic,
            coordinator,
            &fixture,
            version,
            sync,
        );
        let CoordinatorRegistryResponse::Registry(registry) =
            coordinator_status(&pic, coordinator, CoordinatorRegistryRequest::Registry).unwrap();
        let operation_id = [0xb6; 32];
        let plan = fixture_fresh_component_plan(config.model(), &registry, operation_id);
        coordinator_command(
            &pic,
            coordinator,
            CoordinatorCommand::ProvisionComponents(plan.request),
        )
        .unwrap();
        let hub = await_initial_fixture_hub(&pic, coordinator, fixture.root_id, operation_id);
        qualify_automatic_fixture_funding(
            &pic,
            fixture.root_id,
            hub,
            &config.model().component_specs["users"],
        );
        assert_root_waits_for_fixture_membership(&pic, fixture.root_id, operation_id);
        assert_eq!(
            fixture_readiness(&pic, fixture.root_id, hub).fixture,
            Err(canic::dto::fixture_provisioning::FixtureImportError::NotReady)
        );
        let selected = selected_fixture_targets(&pic, fixture.root_id);
        pic.update_candid_as::<(), _>(hub, fixture.root_id, "test_release_fixture", ())
            .unwrap();
        for _ in 0..240 {
            if fixture_readiness(&pic, fixture.root_id, hub).status
                == canic::dto::runtime::ReadinessStatus::Ready
            {
                break;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        assert_eq!(
            fixture_readiness(&pic, fixture.root_id, hub).status,
            canic::dto::runtime::ReadinessStatus::Ready
        );
        assert_eq!(selected_fixture_targets(&pic, fixture.root_id), selected);
        assert_retained_fixtures(&pic, &fixture);
    }

    #[cfg(test)]
    fn await_initial_fixture_hub(
        pic: &PocketIc,
        coordinator: Principal,
        root: Principal,
        operation_id: [u8; 32],
    ) -> Principal {
        (0..240)
            .find_map(|_| {
                let hub = selected_fixture_targets(pic, root)
                    .into_iter()
                    .find(|(_, role)| role.as_str() == "user_hub")
                    .map(|(canister, _)| canister);
                if hub.is_none() {
                    pic.advance_time(Duration::from_secs(1));
                    pic.tick();
                }
                hub
            })
            .unwrap_or_else(|| {
                let status = coordinator_status(
                    pic,
                    coordinator,
                    CoordinatorOperationReadRequest::Operation(OperationStatusRequest {
                        operation_id,
                    }),
                )
                .unwrap();
                let CoordinatorOperationReadResponse::Operation(
                    CoordinatorOperationStatusResponse::ComponentProvisioning(status),
                ) = status
                else {
                    panic!("Component provisioning status correlation");
                };
                panic!(
                    "initial Hub unavailable: {status:?}; pool={:?}",
                    root_pool_status(pic, root)
                );
            })
    }

    #[cfg(test)]
    fn fixture_topup_events(
        pic: &PocketIc,
        root: Principal,
        target: Principal,
    ) -> Vec<canic::dto::cycles::CycleTopupEvent> {
        let response: Result<CanisterObservabilityResponse, Error> = pic
            .query_candid_as(
                target,
                root,
                canic::protocol::CANIC_OBSERVABILITY,
                (CanisterObservabilityRequest::CycleTopups(
                    canic::dto::page::PageRequest {
                        offset: 0,
                        limit: 100,
                    },
                ),),
            )
            .unwrap();
        let CanisterObservabilityResponse::CycleTopups(page) = response.unwrap() else {
            panic!("cycle top-up history response");
        };
        assert_eq!(
            page.entries.len() as u64,
            page.total,
            "complete bounded funding history"
        );
        page.entries
    }

    #[cfg(test)]
    fn qualify_automatic_fixture_funding(
        pic: &PocketIc,
        root: Principal,
        hub: Principal,
        spec: &canic_core::bootstrap::compiled::ComponentSpecConfig,
    ) {
        use canic::dto::cycles::CycleTopupEventStatus;
        let expected = spec.cycles_funding.max_per_child.to_u128();
        let mut events = Vec::new();
        for _ in 0..240 {
            events = fixture_topup_events(pic, root, hub);
            if events
                .iter()
                .filter_map(|event| event.transferred_cycles.as_ref())
                .map(Cycles::to_u128)
                .sum::<u128>()
                == expected
                && events
                    .iter()
                    .any(|event| event.status == CycleTopupEventStatus::RequestErr)
            {
                break;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        let transfers = events
            .iter()
            .filter(|event| event.status == CycleTopupEventStatus::RequestOk)
            .map(|event| {
                assert_eq!(event.requested_cycles, spec.topup.as_ref().unwrap().amount);
                event.transferred_cycles.as_ref().unwrap().to_u128()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            transfers.iter().sum::<u128>(),
            expected,
            "automatic funding must reach its configured allowance; transfers={transfers:?}"
        );
        assert!(
            transfers
                .iter()
                .all(|amount| *amount <= spec.cycles_funding.max_per_request.to_u128())
        );
        assert!(
            transfers
                .iter()
                .any(|amount| *amount < spec.cycles_funding.max_per_request.to_u128())
        );
        assert!(
            events
                .iter()
                .any(|event| event.status == CycleTopupEventStatus::RequestErr)
        );
        assert_eq!(
            descendant_funding_response(pic, root, hub, descendant_funding_request(pic, 0xb7)),
            canic::dto::rpc::CyclesResponse::PreflightRejected(
                canic::dto::rpc::CyclesFundingPreflightResponse::ChildBudgetExhausted {
                    remaining_child_budget: 0,
                    max_per_child: expected,
                }
            )
        );
        let history = candid::encode_one(&events).unwrap();
        for _ in 0..5 {
            pic.advance_time(Duration::from_secs(60));
            pic.tick();
        }
        assert_eq!(
            candid::encode_one(fixture_topup_events(pic, root, hub)).unwrap(),
            history,
            "exhausted automatic funding stops issuing further requests"
        );
    }

    #[test]
    fn fixture_bearing_root_retirement_conserves_assets_and_cycles() {
        let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = initial_shard_root_canister_config_path(&workspace);
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let fixture =
            install_initial_shard_fixture(&pic, coordinator, &config_path, &["user_shard"]);
        let (version, sync) = join_and_synchronize_root(&pic, coordinator, &fixture);
        activate_registry_and_prepare_component_registry(
            &pic,
            coordinator,
            &fixture,
            version,
            sync,
        );
        reset_prepaid_pool_assets(&pic, fixture.root_id);
        // Direct Component provisioning has no Coordinator group/service references.
        let hub = provision_component_request(
            &pic,
            fixture.root_id,
            RootComponentAllocationRequest {
                operation_id: [0xa3; 32],
                component_spec: "users".parse().unwrap(),
            },
        );
        activate_root(&pic, fixture.root_id);
        let assigned: Result<Principal, Error> = pic
            .update_candid(
                installed_component_binding(&hub).canister_id,
                "create_account",
                (Principal::from_slice(&[0xe3; 29]),),
            )
            .unwrap();
        let shard = assigned.expect("initial Shard assignment after its import completes");
        let readiness = fixture_readiness(&pic, fixture.root_id, shard);
        let Ok(canic::dto::fixture_provisioning::FixtureProvisioningStatus::Complete(receipt)) =
            readiness.fixture
        else {
            panic!("retirement must follow a real completed application import");
        };
        let source = fixture
            .response
            .fixtures
            .iter()
            .find(|source| source.role.as_str() == "user_shard")
            .unwrap();
        assert_eq!(
            receipt.binding.target,
            managed_binding_status(&pic, fixture.root_id, shard)
        );
        assert_eq!(receipt.binding.content_id, source.content_id);
        assert_eq!(
            receipt.binding.release_build_id,
            fixture.response.release_set.release_build_id
        );
        assert_eq!(
            receipt.completion_summary,
            source.descriptor.completion_summary
        );
        assert_retained_fixtures(&pic, &fixture);
        qualify_root_retirement(&RootRetirementFixture {
            pic: &pic,
            coordinator,
            root: fixture.root_id,
            wasm_store: fixture.response.wasm_store,
            pool_assets: &fixture.init_args.canister_pool_imports,
        });
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one real Root/Store sequence binds publication, interrupted progress and terminal replay"
    )]
    fn current_store_bootstraps_application_catalog_and_replays_zero_effects() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = five_component_root_canister_config_path(&workspace_root);
        let config = AppConfigSnapshot::load(&config_path).expect("load five-Component config");
        let configuration = config
            .model()
            .compile_component_deployment_configuration()
            .expect("compile five-Component deployment configuration");
        let root_wasm = build_five_component_root_wasm();
        let mut store_fixture =
            build_root_store_fixture_with_config(&config_path, build_five_component_wasms());
        let payloads = [b"first fixture rows".as_slice(), b"last rows".as_slice()];
        let descriptor = canic::dto::fixture_provisioning::FixtureDescriptor {
            schema_version: 1,
            format_hash: [1; 32],
            completion_summary: [3; 32],
            encoded_length: payloads.iter().map(|bytes| bytes.len() as u64).sum(),
            chunks: payloads
                .iter()
                .map(
                    |bytes| canic::dto::fixture_provisioning::FixtureChunkDescriptor {
                        digest: wasm_hash(bytes).try_into().unwrap(),
                        length: u32::try_from(bytes.len()).unwrap(),
                    },
                )
                .collect(),
        };
        let source = canic::dto::root_store::RootStoreFixture {
            role: configuration.component_topology.component_specs[0]
                .component_role
                .clone(),
            content_id: canic_control_plane::api::fixture_content::FixtureContentApi::content_id(
                &descriptor,
            )
            .unwrap(),
            descriptor,
        };
        store_fixture.manifest.fixtures.push(source.clone());
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let installed = install_current_root_with_config_and_pool_setup(
            &pic,
            root_wasm,
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: None,
                canister_pool_minimum_size: None,
                canister_pool_cycles: None,
                coordinator_subnet: None,
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: None,
                component_admission_limits: None,
                fleet_id: None,
                funding: None,
                coordinator_root_funding: None,
            },
            &config_path,
            None,
            create_prepaid_pool_assets,
        );
        let operation_id = [0x67; 32];
        let artifact_root = test_target_dir(&workspace_root, "current-store-helper-protocol")
            .join(format!("artifact-union-{}", std::process::id()));
        if artifact_root.exists() {
            std::fs::remove_dir_all(&artifact_root).expect("clear prior artifact-union fixture");
        }
        std::fs::create_dir_all(&artifact_root).expect("create artifact-union fixture");
        let union = fixture_application_artifact_union(&artifact_root, &installed);
        let retained = artifact_root.join(format!(
            ".canic/release-builds/{}/fixture-content/{}",
            union.release_build_id,
            canic_core::cdk::utils::hash::hex_bytes(source.content_id),
        ));
        std::fs::create_dir_all(&retained).unwrap();
        for (index, payload) in payloads.iter().enumerate() {
            std::fs::write(retained.join(format!("{index}.bin")), payload).unwrap();
        }
        let store_sequence = compile_current_store_sequence_from_union(
            &artifact_root,
            &configuration.component_topology,
            &installed.init_args.authority,
            operation_id,
            &union,
            &canic_host::release_set::fixture::FixtureArtifactManifest {
                schema_version: 1,
                release_build_id: union.release_build_id,
                component_topology_digest: union.fleet_component_topology_digest,
                entries: vec![canic_host::release_set::fixture::FixtureArtifactEntry {
                    role: source.role.clone(),
                    content_id: source.content_id,
                    descriptor: source.descriptor.clone(),
                }],
            },
        )
        .expect("compile current Store sequence");
        let wasm_store = installed
            .init_args
            .authority
            .wasm_store_authority
            .wasm_store;
        let installation_controller = installed
            .init_args
            .authority
            .wasm_store_authority
            .installation_controller;
        let actions = store_sequence
            .actions
            .iter()
            .enumerate()
            .map(|(index, action)| CompiledCurrentProtocolStep {
                action: action.clone(),
                name: format!("store-bootstrap-{index}"),
                target: match action {
                    CurrentFleetProtocolAction::AdoptStore { .. }
                    | CurrentFleetProtocolAction::BootstrapStore { .. }
                    | CurrentFleetProtocolAction::PrepareStoreFixture { .. } => installed.root_id,
                    CurrentFleetProtocolAction::PrepareStoreChunkSet { .. }
                    | CurrentFleetProtocolAction::PublishStoreFixtureChunk { .. }
                    | CurrentFleetProtocolAction::PublishStoreChunk { .. }
                    | CurrentFleetProtocolAction::StageStoreManifest { .. } => wasm_store,
                    _ => panic!("Store sequence emitted a non-Store/Root action"),
                },
            })
            .collect::<Vec<_>>();
        let mut fixture_updates = 0;
        for step in &actions {
            if let CurrentFleetProtocolAction::PublishStoreFixtureChunk {
                request, expected, ..
            } = &step.action
            {
                assert!(!current_protocol_step_is_terminal(
                    &pic,
                    step,
                    installation_controller
                ));
                // Commit the update and discard its response, leaving only retained Store progress.
                pic.update_call(
                    wasm_store,
                    installation_controller,
                    canic::protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
                    encode_one(request).unwrap(),
                )
                .expect("publish fixture with lost response");
                fixture_updates += 1;
                let reopened = CompiledCurrentProtocolStep {
                    action: step.action.clone(),
                    name: step.name.clone(),
                    target: step.target,
                };
                assert!(current_protocol_step_is_terminal(
                    &pic,
                    &reopened,
                    installation_controller
                ));
                let status: Result<StoreCatalogResponse, Error> = pic
                    .query_candid_as(
                        wasm_store,
                        installation_controller,
                        canic::protocol::CANIC_WASM_STORE_CATALOG,
                        (StoreCatalogRequest::Fixture(source.content_id),),
                    )
                    .unwrap();
                let Ok(StoreCatalogResponse::Fixture(Ok(observed))) = status else {
                    panic!("exact fixture source status");
                };
                assert_eq!(&observed, expected);
            } else {
                issue_current_protocol_step(&pic, step, installation_controller);
            }
            await_current_protocol_step(&pic, step, installation_controller);
        }
        assert_eq!(fixture_updates, payloads.len());
        let nonterminal = actions
            .iter()
            .filter(|step| !current_protocol_step_is_terminal(&pic, step, installation_controller))
            .map(|step| step.name.clone())
            .collect::<Vec<_>>();
        assert!(
            nonterminal.is_empty(),
            "an immediate Store/Root replay must issue no update; nonterminal={nonterminal:?}"
        );
        std::fs::remove_dir_all(artifact_root).expect("remove artifact-union fixture");
    }

    #[test]
    fn fresh_five_component_provisioning_reaches_runtime_active_and_publishes_catalog() {
        assert_five_component_activation(ActivationFailureFixture::Transient);
    }

    #[test]
    fn inactive_root_retains_permanent_store_failure_and_suspends_retries() {
        assert_five_component_activation(ActivationFailureFixture::StoreIdentity);
    }

    #[cfg(test)]
    fn observed_root_provisioning(
        pic: &PocketIc,
        root: Principal,
        operation_id: [u8; 32],
    ) -> Option<canic_core::dto::component_provisioning::RootComponentProvisioningStatusResponse>
    {
        match root_status(
            pic,
            root,
            RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
        ) {
            Ok(RootStatusResponseFragment::Operation(
                RootOperationStatusResponse::ProvisionComponents(status),
            )) => Some(status),
            _ => None,
        }
    }

    #[cfg(test)]
    fn await_root_provisioning(
        pic: &PocketIc,
        root: Principal,
        operation_id: [u8; 32],
        ready: impl Fn(
            &canic_core::dto::component_provisioning::RootComponentProvisioningStatusResponse,
        ) -> bool,
    ) -> canic_core::dto::component_provisioning::RootComponentProvisioningStatusResponse {
        for _ in 0..240 {
            if let Some(status) = observed_root_provisioning(pic, root, operation_id)
                && ready(&status)
            {
                return status;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        panic!(
            "Root did not reach the exact activation boundary: {:?}",
            observed_root_provisioning(pic, root, operation_id).map(|status| (
                status.phase,
                status.activated_component_count,
                status.last_failure
            ))
        );
    }

    #[cfg(test)]
    fn assert_accepted_provisioning_origin(
        pic: &PocketIc,
        root: Principal,
        coordinator: Principal,
        operation_id: [u8; 32],
    ) {
        let mut last_observation = None;
        for _ in 0..240 {
            let root_status = observed_root_provisioning(pic, root, operation_id)
                .expect("accepted Root operation stays readable");
            let CoordinatorOperationReadResponse::Operation(
                CoordinatorOperationStatusResponse::ComponentProvisioning(coordinator_status),
            ) = coordinator_status(
                pic,
                coordinator,
                CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .expect("Coordinator operation stays readable during Store outage")
            else {
                panic!("exact Coordinator provisioning operation");
            };
            if let Some(failure) = root_status.last_failure
                && failure.stage == ProvisioningFailureStage::Provisioning
                && let Some(observed) = coordinator_status.pending_root_failure
                && observed.origin.is_some_and(|origin| {
                    origin.failed_at_ns == failure.failed_at_ns && origin.stage == failure.stage
                })
            {
                assert_eq!(root_status.phase, RootComponentProvisioningPhase::Accepted);
                assert_eq!(failure.target, root);
                assert_eq!(failure.operation_id, operation_id);
                assert_eq!(
                    failure.diagnostic_code,
                    canic_core::diagnostics::codes::PLATFORM_UNAVAILABLE
                        .raw_code()
                        .raw()
                );
                assert_eq!(failure.retry_category, ProvisioningRetryCategory::Backoff);
                assert!(failure.retry_at_ns.is_some());
                assert_eq!(coordinator_status.operation_id, operation_id);
                assert_eq!(observed.fleet_subnet_root, root);
                assert_eq!(
                    observed.stage,
                    FleetComponentProvisioningRetryStage::RootProvisioning
                );
                assert_eq!(
                    observed.origin,
                    Some(ProvisioningFailureOrigin {
                        failed_at_ns: failure.failed_at_ns,
                        stage: failure.stage,
                        target: failure.target,
                        operation_id: failure.operation_id,
                        diagnostic_code: failure.diagnostic_code,
                        retry_category: failure.retry_category,
                    })
                );
                println!("CANIC-159 Accepted Root failure retained by Coordinator: {observed:?}");
                return;
            }
            last_observation = Some((root_status, coordinator_status));
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        panic!("Coordinator did not retain the Accepted Root origin: {last_observation:?}");
    }

    #[cfg(test)]
    /// Faults injected at the existing Root activation boundary.
    #[derive(Clone, Copy, Eq, PartialEq)]
    enum ActivationFailureFixture {
        Transient,
        StoreIdentity,
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one governed journey proves activation failure, recovery and replay"
    )]
    fn assert_five_component_activation(fault: ActivationFailureFixture) {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = five_component_root_canister_config_path(&workspace_root);
        let config = AppConfigSnapshot::load(&config_path).expect("load five-Component config");
        let configuration = config
            .model()
            .compile_component_deployment_configuration()
            .expect("compile five-Component deployment configuration");
        let root_wasm = build_five_component_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let store_fixture =
            build_root_store_fixture_with_config(&config_path, build_five_component_wasms());
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let installed = install_current_root_with_config_and_pool_setup(
            &pic,
            root_wasm,
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: None,
                canister_pool_minimum_size: None,
                canister_pool_cycles: None,
                coordinator_subnet: None,
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: None,
                component_admission_limits: None,
                fleet_id: None,
                funding: None,
                coordinator_root_funding: None,
            },
            &config_path,
            (fault == ActivationFailureFixture::StoreIdentity).then_some([0x91; 32]),
            create_prepaid_pool_assets,
        );
        let operation_id = [0x6e; 32];
        let artifact_root = test_target_dir(&workspace_root, "current-five-component-protocol")
            .join(format!("artifact-union-{}", std::process::id()));
        if artifact_root.exists() {
            std::fs::remove_dir_all(&artifact_root).expect("clear prior artifact-union fixture");
        }
        std::fs::create_dir_all(&artifact_root).expect("create artifact-union fixture");
        let union = fixture_application_artifact_union(&artifact_root, &installed);
        let store_sequence = compile_current_store_sequence_from_union(
            &artifact_root,
            &configuration.component_topology,
            &installed.init_args.authority,
            operation_id,
            &union,
            &canic_host::release_set::fixture::FixtureArtifactManifest {
                schema_version: 1,
                release_build_id: union.release_build_id,
                component_topology_digest: union.fleet_component_topology_digest,
                entries: Vec::new(),
            },
        )
        .expect("compile current Store sequence");
        let fixture = BootstrappedRootFixture {
            root_id: installed.root_id,
            init_args: installed.init_args.clone(),
            coordinator_root_funding: installed.coordinator_root_funding.clone(),
            request: store_sequence.bootstrap_request.clone(),
            response: store_sequence.expected_bootstrap.clone(),
        };
        install_fixture_coordinator_with_config(
            &pic,
            coordinator,
            coordinator_wasm,
            &fixture,
            &config_path,
        );
        assert_ne!(
            installed.init_args.install_id, installed.init_args.wasm_store_activation.operation_id,
            "Root and Store must retain distinct installation identities"
        );
        assert_eq!(
            installed.init_args.wasm_store_activation.wasm_store,
            installed
                .init_args
                .authority
                .wasm_store_authority
                .wasm_store
        );
        let wasm_store = installed
            .init_args
            .authority
            .wasm_store_authority
            .wasm_store;
        let installation_controller = installed
            .init_args
            .authority
            .wasm_store_authority
            .installation_controller;
        let coordinator_controller = Principal::anonymous();
        let coordinator_cycles_before_start = pic.cycle_balance(coordinator);
        pic.stop_canister(coordinator, Some(coordinator_controller))
            .expect("stop retained Coordinator");
        assert_eq!(
            format!(
                "{:?}",
                pic.canister_status(coordinator, Some(coordinator_controller))
                    .expect("stopped retained Coordinator status")
                    .status
            ),
            "Stopped"
        );
        pic.start_canister(coordinator, Some(coordinator_controller))
            .expect("start retained Coordinator with the same identity");
        assert_eq!(
            format!(
                "{:?}",
                pic.canister_status(coordinator, Some(coordinator_controller))
                    .expect("running retained Coordinator status")
                    .status
            ),
            "Running"
        );
        assert!(pic.cycle_balance(coordinator) <= coordinator_cycles_before_start);
        let CoordinatorRegistryResponse::Registry(genesis) =
            coordinator_status(&pic, coordinator, CoordinatorRegistryRequest::Registry)
                .expect("query current genesis Registry");
        let desired = current_protocol_desired(&configuration, coordinator, &installed.init_args);
        let state = FleetEnsureStateRecord {
            active_registry: None,
            completed_reinstall_action_sha256: BTreeMap::new(),
            completed_reinstall_operation_id: None,
            completed_reinstalls: BTreeMap::new(),
            fleet: desired.fleet.clone(),
            pending_principals: BTreeMap::new(),
            principals: BTreeMap::new(),
            retained_cycles_by_principal: BTreeMap::new(),
            schema_version: FLEET_ENSURE_SCHEMA_VERSION,
            topology: BTreeMap::new(),
        };
        let authorities = vec![installed.init_args.authority.clone()];
        let registry_sequence = compile_current_registry_sequence(
            &desired,
            &state,
            &configuration.component_topology,
            &genesis,
            &authorities,
        )
        .expect("compile current initial Registry sequence");
        assert_eq!(
            registry_sequence.current_stage,
            CurrentRegistryStage::Genesis
        );
        let stores = BTreeMap::from([(installed.root_id, store_sequence)]);
        let actions = compile_current_protocol_sequence(
            &desired,
            &state,
            &configuration,
            &registry_sequence,
            &authorities,
            &stores,
            operation_id,
        )
        .expect("compile complete current Fleet protocol");
        assert!(
            actions.is_sorted_by_key(|step| current_protocol_test_stage(&step.action)),
            "current protocol actions must preserve Store -> join -> sync -> activate -> mirror -> Component order"
        );
        let mut replayed_component_command = false;
        for step in &actions {
            if let CurrentFleetProtocolAction::ProvisionComponents { request, .. } = &step.action {
                reset_prepaid_pool_assets(&pic, fixture.root_id);
                let store_cycles_before_retry = pic.cycle_balance(wasm_store);
                pic.stop_canister(wasm_store, Some(installation_controller))
                    .expect("stop Store before Root activation retry boundary");
                issue_current_protocol_step(&pic, step, installation_controller);
                let failure = (0..160)
                    .find_map(|_| {
                        let status = coordinator_status(
                            &pic,
                            step.target,
                            CoordinatorOperationReadRequest::Operation(OperationStatusRequest {
                                operation_id: request.operation_id,
                            }),
                        );
                        let failure = match status {
                            Ok(CoordinatorOperationReadResponse::Operation(
                                CoordinatorOperationStatusResponse::ComponentProvisioning(status),
                            )) => status.pending_root_failure,
                            _ => None,
                        };
                        if failure.is_none() {
                            pic.advance_time(Duration::from_secs(1));
                            pic.tick();
                        }
                        failure
                    })
                    .expect("typed pending Root retry failure while Store is stopped");
                assert_eq!(
                    failure.stage,
                    canic_core::dto::component_provisioning::FleetComponentProvisioningRetryStage::RootAcceptance
                );
                pic.start_canister(wasm_store, Some(installation_controller))
                    .expect("restart the same retained Store");
                assert!(pic.cycle_balance(wasm_store) <= store_cycles_before_retry);
                issue_current_protocol_step(&pic, step, installation_controller);
                replayed_component_command = true;
                if fault == ActivationFailureFixture::Transient {
                    await_root_provisioning(
                        &pic,
                        installed.root_id,
                        request.operation_id,
                        |status| status.phase == RootComponentProvisioningPhase::Accepted,
                    );
                    pic.stop_canister(wasm_store, Some(installation_controller))
                        .expect("stop Store while Root provisioning is Accepted");
                    assert_accepted_provisioning_origin(
                        &pic,
                        installed.root_id,
                        coordinator,
                        request.operation_id,
                    );
                    pic.start_canister(wasm_store, Some(installation_controller))
                        .expect("resume the same operation after Accepted-phase Store outage");
                    await_root_provisioning(
                        &pic,
                        installed.root_id,
                        request.operation_id,
                        |status| {
                            status.phase == canic_core::dto::component_provisioning::RootComponentProvisioningPhase::Published
                                && status.activated_component_count == status.component_count
                        },
                    );
                    pic.stop_canister(wasm_store, Some(installation_controller))
                        .expect("stop Store before Root runtime preparation");
                }
                let (expected_stage, expected_operation) = match fault {
                    ActivationFailureFixture::Transient => (
                        canic_core::dto::component_provisioning::ProvisioningFailureStage::StoreCatalog,
                        installed.init_args.install_id,
                    ),
                    ActivationFailureFixture::StoreIdentity => (
                        canic_core::dto::component_provisioning::ProvisioningFailureStage::StoreStatus,
                        installed.init_args.wasm_store_activation.operation_id,
                    ),
                };
                let failed = await_root_provisioning(
                    &pic,
                    installed.root_id,
                    request.operation_id,
                    |status| {
                        status
                            .last_failure
                            .is_some_and(|failure| failure.stage == expected_stage)
                    },
                );
                let first = failed.last_failure.unwrap();
                assert_eq!(first.target, wasm_store);
                assert_eq!(first.operation_id, expected_operation);
                assert!(!failed.root_runtime_active);
                let forbidden: Result<RootStatusResponseFragment, Error> = pic
                    .query_candid_as(
                        installed.root_id,
                        Principal::from_slice(&[0x92; 29]),
                        canic::protocol::CANIC_ROOT_OPERATION_STATUS,
                        (RootStatusRequestFragment::Operation(
                            OperationStatusRequest {
                                operation_id: request.operation_id,
                            },
                        ),),
                    )
                    .expect("unauthorized protected status transport");
                assert!(forbidden.is_err_and(|error| error.code()
                    == canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()));
                if fault == ActivationFailureFixture::StoreIdentity {
                    assert_eq!(first.retry_category, canic_core::dto::component_provisioning::ProvisioningRetryCategory::ReviewRequired);
                    assert_eq!(first.retry_at_ns, None);
                    issue_current_protocol_step(&pic, step, installation_controller);
                    for _ in 0..20 {
                        pic.advance_time(Duration::from_mins(1));
                        pic.tick();
                    }
                    let suspended =
                        observed_root_provisioning(&pic, installed.root_id, request.operation_id)
                            .unwrap();
                    assert_eq!(suspended.last_failure, Some(first));
                    assert_eq!(suspended.receipt_content_hash, failed.receipt_content_hash);
                    assert!(!suspended.root_runtime_active);
                    let CoordinatorOperationReadResponse::Operation(
                        CoordinatorOperationStatusResponse::ComponentProvisioning(status),
                    ) = coordinator_status(
                        &pic,
                        coordinator,
                        CoordinatorOperationReadRequest::Operation(OperationStatusRequest {
                            operation_id: request.operation_id,
                        }),
                    )
                    .unwrap()
                    else {
                        panic!("exact Coordinator provisioning operation");
                    };
                    let origin = status.pending_root_failure.unwrap().origin.unwrap();
                    assert_eq!(origin.target, first.target);
                    assert_eq!(origin.stage, first.stage);
                    assert_eq!(origin.operation_id, first.operation_id);
                    assert_eq!(origin.retry_category, first.retry_category);
                    assert_eq!(origin.failed_at_ns, first.failed_at_ns);
                    std::fs::remove_dir_all(artifact_root).unwrap();
                    return;
                }
                assert_eq!(
                    first.retry_category,
                    canic_core::dto::component_provisioning::ProvisioningRetryCategory::Backoff
                );
                let backed_off = await_root_provisioning(
                    &pic,
                    installed.root_id,
                    request.operation_id,
                    |status| {
                        status.last_failure.is_some_and(|failure| {
                            failure.consecutive_failures >= first.consecutive_failures + 3
                        })
                    },
                );
                let later = backed_off.last_failure.unwrap();
                assert!(later.failed_at_ns >= first.failed_at_ns + 7_000_000_000);
                assert_eq!(backed_off.receipt_content_hash, failed.receipt_content_hash);
                pic.start_canister(wasm_store, Some(installation_controller))
                    .expect(
                        "resume exact retained activation after the transient dependency recovers",
                    );
            } else {
                issue_current_protocol_step(&pic, step, installation_controller);
            }
            await_current_protocol_step(&pic, step, installation_controller);
        }
        assert!(replayed_component_command);

        let CoordinatorRegistryResponse::Registry(terminal_registry) =
            coordinator_status(&pic, coordinator, CoordinatorRegistryRequest::Registry)
                .expect("query terminal Fleet Registry");
        assert_eq!(terminal_registry.revision, 4);
        let CoordinatorOperationReadResponse::Operation(
            CoordinatorOperationStatusResponse::ComponentProvisioning(terminal_status),
        ) = coordinator_status(
            &pic,
            coordinator,
            CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
        )
        .expect("query terminal Component operation")
        else {
            panic!("Coordinator returned a differently correlated Component operation");
        };
        let terminal_sequence = compile_current_registry_sequence_with_status(
            &desired,
            &state,
            &configuration.component_topology,
            &terminal_registry,
            &authorities,
            Some(&terminal_status),
        )
        .expect("recognize terminal current Registry");
        assert_eq!(
            terminal_sequence.current_stage,
            CurrentRegistryStage::Provisioned
        );
        assert_eq!(
            terminal_status.runtime_activated_root_count,
            terminal_status.root_batch_count
        );
        assert!(terminal_status.runtimes_activated_at_ns.is_some());
        assert_eq!(terminal_status.pending_root_failure, None);
        let terminal_pool = root_pool_status(&pic, fixture.root_id);
        assert_eq!(terminal_pool.workload, 5);
        assert!(!terminal_pool.entries.is_empty());
        let replay = compile_current_protocol_sequence(
            &desired,
            &state,
            &configuration,
            &terminal_sequence,
            &authorities,
            &stores,
            operation_id,
        )
        .expect("compile immediate current replay");
        let nonterminal = replay
            .iter()
            .filter(|step| !current_protocol_step_is_terminal(&pic, step, installation_controller))
            .map(|step| step.name.clone())
            .collect::<Vec<_>>();
        assert!(
            nonterminal.is_empty(),
            "an immediate second ensure must issue no update; nonterminal={nonterminal:?}"
        );
        for _ in 0..2 {
            pic.advance_time(Duration::from_mins(1));
            pic.tick();
        }
        let settled = observed_root_provisioning(&pic, installed.root_id, operation_id).unwrap();
        assert!(settled.root_runtime_active);
        assert_eq!(settled.last_failure, None);
        std::fs::remove_dir_all(artifact_root).expect("remove artifact-union fixture");
    }

    #[test]
    fn four_initial_shards_preserve_sealed_root_activation() {
        assert_initial_root_activation(
            "apps/test/test-configs/literal-zero-initial-shard.toml",
            1,
            4,
            5,
        );
    }

    #[test]
    fn auth_free_root_preserves_ordinary_fleet_activation() {
        assert_initial_root_activation("canisters/audit/root_probe/activation.toml", 1, 0, 1);
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the focused real-canister regression keeps exact initial-child activation and replay together"
    )]
    #[cfg(test)]
    fn assert_initial_root_activation(
        config_relative: &str,
        components: u32,
        descendants: u32,
        ready: u32,
    ) {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = workspace_root.join(config_relative);
        let workload = components + descendants;
        let pool_count = workload + ready;
        let config = AppConfigSnapshot::load(&config_path).expect("load activation fixture config");
        let configuration = config
            .model()
            .compile_component_deployment_configuration()
            .expect("compile activation fixture deployment");
        let roles = config
            .model()
            .roles
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let adapter_root = literal_zero_adapter_root(&workspace_root);
        std::fs::create_dir_all(&adapter_root).expect("create artifact fixture root");
        let _cleanup = TestDirectoryCleanup(adapter_root.clone());
        let artifacts = build_literal_zero_release_artifacts(
            &workspace_root,
            &adapter_root,
            &config_path,
            &configuration,
            &roles,
            BuildNetwork::Local,
            INTERNAL_TEST_RELEASE_BUILD_NONCE,
        );
        let coordinator_wasm = std::fs::read(adapter_root.join(&artifacts.coordinator_wasm))
            .expect("read exact Coordinator Wasm");
        let store_fixture = build_root_store_fixture_with_config_for_release(
            &config_path,
            &artifacts.component_wasms,
            artifacts.release_build_id,
        );
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let fixture = install_bootstrapped_root_with_config_and_pool_setup(
            &pic,
            artifacts.root_wasm_bytes,
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: Some(pool_count),
                canister_pool_minimum_size: Some(ready),
                canister_pool_cycles: None,
                coordinator_subnet: None,
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: None,
                component_admission_limits: None,
                fleet_id: None,
                funding: None,
                coordinator_root_funding: None,
            },
            &config_path,
            |pic, root| {
                let subnet = pic.get_subnet(root).expect("Root Subnet");
                (0..pool_count)
                    .map(|_| {
                        let asset = pic.create_canister_on_subnet(None, None, subnet);
                        pic.add_cycles(asset, PREPAID_POOL_ASSET_CYCLES);
                        pic.set_controllers(asset, None, vec![root])
                            .expect("Root-owned asset");
                        asset
                    })
                    .collect()
            },
        );
        reset_prepaid_pool_assets_for_count(
            &pic,
            fixture.root_id,
            usize::try_from(pool_count).expect("bounded pool count"),
        );
        install_fixture_coordinator_with_config(
            &pic,
            coordinator,
            coordinator_wasm,
            &fixture,
            &config_path,
        );
        let (joining_version, sync_request) =
            join_and_synchronize_root(&pic, coordinator, &fixture);
        let registry_request = activate_registry_and_prepare_component_registry(
            &pic,
            coordinator,
            &fixture,
            joining_version,
            sync_request,
        );
        let CoordinatorRegistryResponse::Registry(registry) =
            coordinator_status(&pic, coordinator, CoordinatorRegistryRequest::Registry)
                .expect("query active Registry");
        let operation_id = [0x72; 32];
        let request = fixture_fresh_component_plan(config.model(), &registry, operation_id).request;
        let CoordinatorCommandResponse::OperationAccepted(first) = coordinator_command(
            &pic,
            coordinator,
            CoordinatorCommand::ProvisionComponents(request.clone()),
        )
        .expect("provision the exact configured Component tree") else {
            panic!("expected provisioning receipt");
        };
        let mut last = None;
        let terminal = (0..240_u32.saturating_add(descendants.saturating_mul(32)))
            .find_map(|_| {
                let CoordinatorOperationReadResponse::Operation(
                    CoordinatorOperationStatusResponse::ComponentProvisioning(status),
                ) = coordinator_status(
                    &pic,
                    coordinator,
                    CoordinatorOperationReadRequest::Operation(OperationStatusRequest {
                        operation_id,
                    }),
                )
                .expect("query initial Root activation")
                else {
                    panic!("expected provisioning status");
                };
                if status.runtimes_activated_at_ns.is_some()
                    && status.runtime_activated_root_count == status.root_batch_count
                {
                    Some(status)
                } else {
                    last = Some(status);
                    pic.advance_time(Duration::from_secs(1));
                    pic.tick();
                    None
                }
            })
            .unwrap_or_else(|| {
                report_canister_diagnostics_batch(
                    &pic,
                    [("Root".to_string(), fixture.root_id, Principal::anonymous())],
                    "initial Root activation",
                );
                panic!("initial Root activation did not converge: {last:?}");
            });
        assert_eq!(terminal.component_count, components);
        assert!(terminal.pending_root_failure.is_none());
        let pool = root_pool_status(&pic, fixture.root_id);
        assert_eq!(
            (pool.workload, pool.ready, pool.pending_reset, pool.failed),
            (workload, ready, 0, 0)
        );
        let RootStatusResponseFragment::ComponentRegistry(registry) = root_status(
            &pic,
            fixture.root_id,
            RootStatusRequestFragment::ComponentRegistry(registry_request),
        )
        .expect("query terminal Registry") else {
            panic!("expected Component Registry status");
        };
        assert_eq!(registry.managed_descendants, descendants);
        let CoordinatorCommandResponse::OperationAccepted(replay) = coordinator_command(
            &pic,
            coordinator,
            CoordinatorCommand::ProvisionComponents(request),
        )
        .expect("replay exact provisioning operation") else {
            panic!("expected replayed provisioning receipt");
        };
        assert_eq!(replay, first);
        assert_eq!(root_pool_status(&pic, fixture.root_id), pool);
    }

    #[test]
    fn live_management_gateway_preserves_fresh_creation_headroom() {
        let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let mut pic = build_management_pic();
        let initial_time = pic.get_time();
        let wall_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        assert!(
            wall_time
                .as_nanos()
                .abs_diff(u128::from(initial_time.as_nanos_since_unix_epoch()))
                < Duration::from_mins(1).as_nanos(),
            "gateway fixtures must create assets near wall time: initial={initial_time:?}, wall={wall_time:?}"
        );
        let canister = pic
            .create_canister_with_params(
                None,
                CreateCanisterParams {
                    cycles: Some(PREPAID_POOL_ASSET_CYCLES),
                    placement: Some(CreateCanisterPlacement::SubnetId(
                        pic.topology().get_app_subnets()[0],
                    )),
                    ..CreateCanisterParams::default()
                },
            )
            .unwrap();
        assert_eq!(pic.cycle_balance(canister), PREPAID_POOL_ASSET_CYCLES);
        pic.make_live(None);
        pic.canister_status(canister, None).unwrap();
        let after = pic.cycle_balance(canister);
        pic.stop_live();
        assert!(
            after >= PREPAID_POOL_ASSET_CYCLES - MAINNET_REFILL_EXECUTION_MARGIN,
            "fresh creation lost its margin when the gateway started: before={PREPAID_POOL_ASSET_CYCLES}, after={after}, initial_time={initial_time:?}, live_time={:?}",
            pic.get_time()
        );
    }

    #[test]
    fn funded_failed_imports_reconcile_with_lost_withdrawal_and_reset_responses() {
        assert_literal_zero_host_journey(FundingJourney::FailedImports, 1);
    }

    #[cfg(test)]
    #[derive(Clone, Copy)]
    enum FundingJourney {
        ActivationReset,
        Fresh,
        Reinstall,
        FailedImports,
        Estate,
        FailedReserve,
        FundingPause,
        NativeFunding,
        NativeChildFunding,
    }

    #[test]
    #[ignore = "requires Node.js and the pinned frontend-consumer npm dependencies; run explicitly for OP2 qualification"]
    fn frontend_handoff_public_cli_and_sdk_preserve_admission_and_local_trust() {
        assert_literal_zero_host_journey(FundingJourney::Fresh, 1);
    }

    #[cfg(test)]
    fn frontend_consumer_script() -> PathBuf {
        workspace_root_for(env!("CARGO_MANIFEST_DIR"))
            .join("crates/canic-host/examples/frontend-consumer/qualify.mjs")
    }

    #[cfg(test)]
    fn prepare_frontend_identity(root: &Path, name: &str) -> Principal {
        let result = std::process::Command::new("node")
            .arg(frontend_consumer_script())
            .arg("identity")
            .arg(root.join(name))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        Principal::from_text(String::from_utf8(result.stdout).unwrap().trim()).unwrap()
    }

    #[cfg(test)]
    fn assert_frontend_sdk_call(
        root: &Path,
        manifest: &canic_host::frontend::model::FrontendManifestRecord,
        admitted: bool,
    ) {
        let identity = if admitted {
            "browser-identity.json"
        } else {
            "unadmitted-identity.json"
        };
        let result = std::process::Command::new("node")
            .arg(frontend_consumer_script())
            .arg(if admitted { "call" } else { "denied" })
            .arg(root.join("browser"))
            .arg(&manifest.manifest_sha256)
            .arg(manifest.roles[0].canister_id.to_text())
            .arg("audit_frontend_admission_probe")
            .env("CANIC_FRONTEND_IDENTITY", root.join(identity))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        println!("{}", String::from_utf8_lossy(&result.stdout));
    }

    #[cfg(test)]
    fn assert_frontend_handoff(
        root: &Path,
        isolated_icp: &Path,
        pic: &mut PocketIc,
        desired: &DesiredFleet,
        caller: Principal,
    ) {
        use canic_host::frontend::{model::*, ops};
        let authority = ops::authority(root, &desired.environment, &desired.fleet).unwrap();
        let application = authority
            .entries
            .iter()
            .find(|entry| entry.role.as_deref() == Some("app"))
            .unwrap();
        let canister_id = Principal::from_text(&application.pid).unwrap();
        let release = desired.bootstrap.as_ref().unwrap().release_build_id;
        let candid = literal_zero_role_artifact_path(root, release, "app", "did");
        canic_host::canister_build::validate_wasm_candid_endpoints(
            &literal_zero_role_artifact_path(root, release, "app", "wasm"),
            &candid,
        )
        .unwrap();
        let sidecar = canic_host::icp::local_canister_candid_path(root, "local", "app");
        std::fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        std::fs::copy(candid, &sidecar).unwrap();
        let gateway = pic.make_live(None);
        let input = FrontendEnvironmentInput {
            schema_version: 1,
            environment: desired.environment.clone(),
            canonical_network_id: authority.network,
            api_origin: gateway.as_str().trim_end_matches('/').to_string(),
            identity: FrontendIdentityInput {
                canister_id: Principal::from_text("rdmx6-jaaaa-aaaaa-aaadq-cai").unwrap(),
                provider_origin: "http://identity.localhost:5173".to_string(),
                derivation_origin: "http://localhost:5173".to_string(),
                alternative_origins: Vec::new(),
            },
            asset: None,
            roles: vec![FrontendRoleInput {
                role: "app".into(),
                canister_id,
            }],
        };
        assert_ne!(caller, Principal::from_text(&desired.operator).unwrap());
        std::fs::write(
            root.join("frontend.json"),
            serde_json::to_vec(&input).unwrap(),
        )
        .unwrap();
        std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        std::fs::write(root.join("icp.yaml"), "canisters: []\n").unwrap();
        std::fs::create_dir_all(root.join("apps/frontend")).unwrap();
        std::fs::copy(
            workspace_root_for(env!("CARGO_MANIFEST_DIR"))
                .join("canisters/audit/root_probe/frontend.toml"),
            root.join("apps/frontend/canic.toml"),
        )
        .unwrap();
        std::fs::write(root.join("root-key.der"), pic.root_key().unwrap()).unwrap();
        let wrapper = operator_cli_gateway_wrapper(root, gateway.as_str(), isolated_icp);
        run_environment_cli(
            root,
            &wrapper,
            "local",
            &[
                "frontend",
                "export",
                &desired.fleet,
                "--input",
                "frontend.json",
                "--out",
                "browser",
                "--json",
            ],
        )
        .unwrap();
        let manifest: FrontendManifestRecord = serde_json::from_slice(
            &std::fs::read(root.join("browser/canic-frontend.json")).unwrap(),
        )
        .unwrap();
        run_environment_cli(
            root,
            &wrapper,
            "local",
            &[
                "frontend",
                "verify",
                "browser",
                "--sha256",
                &manifest.manifest_sha256,
            ],
        )
        .unwrap();
        assert_eq!(
            manifest.local_root_key_der_hex,
            Some(hex_bytes(pic.root_key().unwrap()))
        );
        assert_frontend_sdk_call(root, &manifest, true);
        prepare_frontend_identity(root, "unadmitted-identity.json");
        assert_frontend_sdk_call(root, &manifest, false);
        assert_frontend_invalid_inputs(root, desired, &input, &sidecar);
        assert_frontend_asset_capacity(root, &wrapper, pic, desired);
        assert_observatory_handoff(root, &wrapper, desired);
    }

    #[cfg(test)]
    fn assert_observatory_handoff(root: &Path, wrapper: &Path, desired: &DesiredFleet) {
        use canic_host::observatory::view::ObservatorySnapshotView;
        let release = desired.bootstrap.as_ref().unwrap().release_build_id;
        for role in ["app", "fleet_coordinator", "root", "wasm_store"] {
            let source = literal_zero_role_artifact_path(root, release, role, "did");
            let target = canic_host::icp::local_canister_candid_path(root, "local", role);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(source, target).unwrap();
        }
        let started = Instant::now();
        run_environment_cli(
            root,
            wrapper,
            "local",
            &[
                "observatory",
                "snapshot",
                &desired.fleet,
                "--out",
                "observatory-private.json",
            ],
        )
        .unwrap();
        let snapshot: ObservatorySnapshotView =
            serde_json::from_slice(&std::fs::read(root.join("observatory-private.json")).unwrap())
                .unwrap();
        assert_observatory_live_roles(&snapshot);
        assert_observatory_public_and_partial(root, wrapper, desired, &snapshot);
        eprintln!(
            "observatory qualification: {} role instances, {} queries, {} private bytes, {:?} for three collections",
            snapshot.roles.len(),
            snapshot.remote_call_attempts,
            std::fs::metadata(root.join("observatory-private.json"))
                .unwrap()
                .len(),
            started.elapsed()
        );
    }

    #[cfg(test)]
    fn assert_observatory_live_roles(
        snapshot: &canic_host::observatory::view::ObservatorySnapshotView,
    ) {
        use canic_host::observatory::view::Observation;
        assert!(matches!(snapshot.authority, Observation::Observed { .. }));
        assert!(matches!(snapshot.operation, Observation::Observed { .. }));
        for role in ["fleet_coordinator", "root", "wasm_store", "app"] {
            let observed = snapshot
                .roles
                .iter()
                .find(|entry| entry.role == role)
                .unwrap();
            assert!(
                matches!(observed.overview, Observation::Observed { .. }),
                "{role}: {:?}",
                observed.overview
            );
        }
        let store = snapshot
            .roles
            .iter()
            .find(|entry| entry.role == "wasm_store")
            .unwrap();
        let Observation::Observed {
            value: inventory, ..
        } = &store.store
        else {
            panic!("Store inventory: {:?}", store.store);
        };
        assert!(inventory.approved_catalog_entries > 0);
        assert!(inventory.occupied_bytes > 0);
        assert_eq!(
            inventory.expected_template_chunks,
            inventory.stored_template_chunks
        );
        for role in ["fleet_coordinator", "root"] {
            let entry = snapshot
                .roles
                .iter()
                .find(|entry| entry.role == role)
                .unwrap();
            let Observation::Observed { value, .. } = &entry.funding else {
                panic!("{role} funding: {:?}", entry.funding);
            };
            assert!(value.native_cycles.parse::<u128>().unwrap() > 0);
        }
    }

    #[cfg(test)]
    fn assert_observatory_public_and_partial(
        root: &Path,
        wrapper: &Path,
        desired: &DesiredFleet,
        snapshot: &canic_host::observatory::view::ObservatorySnapshotView,
    ) {
        use canic_host::observatory::{
            model::ObservatoryProfile,
            view::{Observation, ObservatorySnapshotView, PublicObservatoryView},
        };
        let profile = ObservatoryProfile {
            schema_version: 1,
            title: "Disposable Fleet".into(),
            role_labels: BTreeMap::new(),
        };
        std::fs::write(
            root.join("observatory-profile.json"),
            serde_json::to_vec(&profile).unwrap(),
        )
        .unwrap();
        run_environment_cli(
            root,
            wrapper,
            "local",
            &[
                "observatory",
                "snapshot",
                &desired.fleet,
                "--public",
                "--profile",
                "observatory-profile.json",
                "--out",
                "observatory-public.json",
            ],
        )
        .unwrap();
        let bytes = std::fs::read(root.join("observatory-public.json")).unwrap();
        let public: PublicObservatoryView = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(public.roles.len(), snapshot.roles.len());
        let text = String::from_utf8(bytes).unwrap();
        for role in &snapshot.roles {
            assert!(!text.contains(&role.canister_id));
        }
        // An independently broken Store binding leaves other roles observable.
        std::fs::write(
            canic_host::icp::local_canister_candid_path(root, "local", "wasm_store"),
            "service : {};\n",
        )
        .unwrap();
        run_environment_cli(
            root,
            wrapper,
            "local",
            &[
                "observatory",
                "snapshot",
                &desired.fleet,
                "--out",
                "observatory-partial.json",
            ],
        )
        .unwrap();
        let partial: ObservatorySnapshotView =
            serde_json::from_slice(&std::fs::read(root.join("observatory-partial.json")).unwrap())
                .unwrap();
        assert!(matches!(
            partial
                .roles
                .iter()
                .find(|entry| entry.role == "wasm_store")
                .unwrap()
                .store,
            Observation::Unavailable { .. }
        ));
        assert!(matches!(
            partial
                .roles
                .iter()
                .find(|entry| entry.role == "app")
                .unwrap()
                .overview,
            Observation::Observed { .. }
        ));
    }

    #[cfg(test)]
    fn assert_frontend_invalid_inputs(
        root: &Path,
        desired: &DesiredFleet,
        input: &canic_host::frontend::model::FrontendEnvironmentInput,
        sidecar: &Path,
    ) {
        use canic_host::frontend::FrontendError;
        let paths = canic_host::fleet_ensure::ops::EnsurePaths::under(
            root,
            &desired.environment,
            &desired.fleet,
        );
        let held = paths.journal.with_extension("held");
        std::fs::rename(&paths.journal, &held).unwrap();
        let missing = canic_host::frontend::workflow::prepare_handoff(
            root,
            &desired.fleet,
            input,
            &root.join("incomplete"),
        );
        std::fs::rename(held, &paths.journal).unwrap();
        assert!(matches!(
            missing,
            Err(FrontendError::Inventory(
                canic_host::fleet_ensure::CurrentFleetInventoryError::NotConverged { .. }
            ))
        ));
        assert!(!root.join("incomplete").exists());
        let mut wrong = input.clone();
        wrong.identity.derivation_origin = "http://other.localhost:5173".to_string();
        assert!(matches!(
            canic_host::frontend::workflow::prepare_handoff(
                root,
                &desired.fleet,
                &wrong,
                &root.join("wrong")
            ),
            Err(FrontendError::AdmissionOrigin)
        ));
        std::fs::write(sidecar, b"service : {};").unwrap();
        assert!(matches!(
            canic_host::frontend::workflow::prepare_handoff(
                root,
                &desired.fleet,
                input,
                &root.join("stale")
            ),
            Err(FrontendError::ProtocolBinding(_))
        ));
    }

    #[cfg(test)]
    fn assert_frontend_asset_capacity(
        root: &Path,
        wrapper: &Path,
        pic: &PocketIc,
        desired: &DesiredFleet,
    ) {
        use canic_host::frontend::{FrontendError, model::FrontendAssetCapacityInput, ops};
        let asset = pic.create_canister_with_settings(
            Some(Principal::from_text(&desired.operator).unwrap()),
            None,
        );
        pic.add_cycles(asset, 10_000_000_000);
        let input = FrontendAssetCapacityInput {
            environment: "local".to_string(),
            canister_id: asset,
            payload_directory: root.join("browser"),
            minimum_native_cycles: 1,
            maximum_payload_bytes: 32 * 1024 * 1024,
            maximum_files: 100,
        };
        let icp =
            canic_host::icp::IcpCli::new(wrapper.to_str().unwrap(), Some("local".to_string()))
                .with_cwd(root);
        let report = ops::asset_capacity(&icp, &input).unwrap();
        assert!(report.sufficient);
        assert_eq!(report.payload.files, 5);
        let result = run_environment_cli(
            root,
            wrapper,
            "local",
            &[
                "frontend",
                "capacity",
                &asset.to_text(),
                "--payload",
                "browser",
                "--minimum-native-cycles",
                &u128::MAX.to_string(),
                "--maximum-payload-bytes",
                "33554432",
                "--maximum-files",
                "100",
                "--json",
            ],
        );
        assert!(
            matches!(result, Err(canic_cli::CliError::Frontend(error)) if matches!(*error, canic_cli::FrontendCommandError::Host(FrontendError::NativeCapacity { .. })))
        );
    }

    #[cfg(test)]
    fn operator_cli_root_candid(workspace: &Path) -> Vec<u8> {
        use crate::pic::artifacts::{
            CanicWasmBuildProfile, build_internal_test_wasm_canisters_with_env,
        };
        let target = workspace.join("target/pic-wasm/operator-candid");
        let config = root_canister_config_path(workspace);
        let wasms = build_internal_test_wasm_canisters_with_env(
            workspace,
            &target,
            &["delegation_root_stub"],
            CanicWasmBuildProfile::Fast,
            &[
                (
                    canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
                    config.to_str().unwrap(),
                ),
                (canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV, "1"),
            ],
        );
        let output = Command::new("candid-extractor")
            .arg(wasms.path("delegation_root_stub"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "extract Root companion Candid: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    #[cfg(test)]
    fn operator_cli_protocol(
        directory: &Path,
        role: &str,
        wasm: &[u8],
    ) -> canic_host::protocol_binding::RegistryProtocolBinding {
        let wasm_path = directory.join(format!("{role}.wasm"));
        std::fs::write(&wasm_path, wasm).unwrap();
        let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let candid = if role == "root" {
            operator_cli_root_candid(&workspace)
        } else {
            std::fs::read(workspace.join("crates/canic/candid/fleet_coordinator.did")).unwrap()
        };
        let path = canic_host::icp::local_canister_candid_path(directory, "ic", role);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &candid).unwrap();
        canic_host::canister_build::validate_wasm_candid_endpoints(&wasm_path, &path).unwrap();
        let role = canic_core::ids::CanisterRole::from(role.to_string());
        let capabilities = std::collections::BTreeSet::new();
        let release_identity = "operator-cli-runtime-fixture".to_string();
        let hashes = canic_core::role_contract::derive_protocol_profile_hashes(
            &release_identity,
            &role,
            &capabilities,
            &candid,
        );
        canic_host::protocol_binding::RegistryProtocolBinding {
            release_identity,
            role,
            capabilities,
            candid_sha256: hashes.candid_sha256,
            protocol_profile_digest: hashes.protocol_profile_digest,
        }
    }

    // An explicit terminal starting fixture, not evidence of Ensure convergence or
    // publication. Real management/Registry observations bind its operator inputs.
    #[cfg(test)]
    fn operator_cli_terminal_fixture(
        directory: &Path,
        fixture: &ActiveComponentRegistryFixture,
        operator: Principal,
    ) {
        use canic_host::fleet_ensure::{model, ops, policy};
        let CoordinatorRegistryResponse::Registry(registry) = coordinator_status(
            fixture.pic(),
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .unwrap();
        let mut canisters = Vec::new();
        let mut principals = std::collections::BTreeMap::new();
        let mut topology = std::collections::BTreeMap::new();
        for (name, kind, principal, role, wasm) in [
            (
                "coordinator",
                "coordinator",
                fixture.coordinator,
                "fleet_coordinator",
                build_test_coordinator_wasm(),
            ),
            ("root", "root", fixture.root, "root", build_test_root_wasm()),
        ] {
            let status = fixture.pic().canister_status(principal, None).unwrap();
            let controllers = status
                .settings
                .controllers
                .iter()
                .map(Principal::to_text)
                .collect::<Vec<_>>();
            let parent = (name == "root").then_some("coordinator");
            canisters.push(serde_json::json!({
                "controllers": controllers, "drain": null, "initial_cycles": "0",
                "init_arg": null, "init_candid": null, "kind": kind,
                "minimum_cycles": "0", "name": name, "parent": parent,
                "presence": "present", "principal": principal.to_text(), "replace": false,
                "subnet": fixture.pic().get_subnet(principal).unwrap().to_text(), "wasm": null
            }));
            principals.insert(name.to_string(), principal.to_text());
            topology.insert(
                name.to_string(),
                serde_json::json!({
                    "kind": kind, "module_hash": hex_bytes(status.module_hash.unwrap()),
                    "parent": parent, "role": role,
                    "protocol_binding": operator_cli_protocol(directory, role, &wasm)
                }),
            );
        }
        let desired = serde_json::json!({
            "canisters": canisters, "cycles_ledger": Principal::management_canister().to_text(),
            "environment": "ic", "fleet": "fixture", "ledger_fee_cycles": "0",
            "management_creation_fee_cycles": "0", "material_cycle_threshold": "0",
            "maximum_observation_burn_cycles": "0", "maximum_stalled_observations": 10,
            "maximum_update_burn_cycles": "0", "operator": operator.to_text(),
            "schema_version": 1, "treasury": "coordinator"
        });
        let mut plan: model::FleetEnsurePlan = serde_json::from_value(serde_json::json!({
            "recovery_review": null, "reinstall": null, "continuation": null, "canisters": [],
            "conservation": {"estate_funding_domains": [], "expected_post_operation_cycles": "0",
                "maximum_execution_burn_cycles": "0", "maximum_new_funding_cycles": "0",
                "maximum_operator_debit_cycles": "0", "maximum_unavoidable_fee_cycles": "0",
                "observed_controlled_cycles": "0", "retained_in_reused_canisters_cycles": "0",
                "scheduled_transfer_cycles": "0"},
            "desired_sha256": "operator-cli-starting-fixture", "environment": "ic", "fleet": "fixture",
            "operation_id": "operator-cli-starting-fixture", "plan_sha256": "", "planned_at_time": 0,
            "protocol_actions": [], "root_start_authority": null, "root_reinstall_bindings": [],
            "reviewed_desired": {"desired": desired, "protocol_steps": []}, "schema_version": 1,
            "scope": "full", "terminal_inventory_operation_id": null
        })).unwrap();
        plan.plan_sha256 = policy::expected_plan_sha256(&plan);
        let journal = serde_json::from_value(serde_json::json!({
            "funding_reviews": [], "successor_phases": [], "completion": "converged",
            "estate_funding_required": null, "effects": [], "fleet": "fixture",
            "initial_controlled_cycles": "0", "initial_estate_funding_cycles_by_root": {},
            "initial_operator_cycles": "0", "operation_id": plan.operation_id,
            "plan_sha256": plan.plan_sha256, "schema_version": 1, "stalled_observations": 0
        }))
        .unwrap();
        let state = serde_json::from_value(serde_json::json!({
            "active_registry": registry, "completed_reinstall_action_sha256": {},
            "completed_reinstall_operation_id": null, "completed_reinstalls": {}, "fleet": "fixture",
            "pending_principals": {}, "principals": principals, "retained_cycles_by_principal": {},
            "schema_version": 1, "topology": topology
        })).unwrap();
        let paths = ops::EnsurePaths::under(directory, "ic", "fixture");
        ops::write_plan(&paths, &plan).unwrap();
        ops::write_journal(&paths, &journal).unwrap();
        ops::write_state(&paths, &state).unwrap();
    }

    #[cfg(test)]
    fn operator_cli_gateway_wrapper(directory: &Path, gateway: &str, isolated: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt as _;
        let wrapper = directory.join("operator-cli-icp");
        let isolated = isolated.to_str().unwrap();
        let key = directory.join("root-key.der");
        let key_hex = hex_bytes(std::fs::read(&key).unwrap());
        let key = key.to_str().unwrap();
        std::fs::write(
            &wrapper,
            format!(
                r#"#!/bin/bash
set -euo pipefail
case " $* " in
  *" canister "*|*" cycles "*)
    unset ICP_ENVIRONMENT
    args=()
    while (( $# )); do
      case "$1" in
        -e|--environment) shift 2 ;;
        *) args+=("$1"); shift ;;
      esac
    done
    '{isolated}' "${{args[@]}}" -n '{gateway}' -k '{key_hex}'
    case " $* ${{args[*]}} " in
      *" canic_root_command "*)
        if [ -f '{key}.lose-reply' ]; then
          rm '{key}.lose-reply'
          exit 79
        fi ;;
    esac ;;
  *) exec '{isolated}' "$@" ;;
esac
"#
            ),
        )
        .unwrap();
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
        wrapper
    }

    #[cfg(test)]
    fn run_operator_cli(
        directory: &Path,
        wrapper: &Path,
        args: &[&str],
    ) -> Result<(), canic_cli::CliError> {
        run_environment_cli(directory, wrapper, "ic", args)
    }

    #[cfg(test)]
    fn assert_admission_cli_selected_release(
        directory: &Path,
        isolated_icp: &Path,
        pic: &PocketIc,
        desired: &DesiredFleet,
        replica: &LocalReplicaTarget,
    ) {
        std::fs::write(directory.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        std::fs::write(directory.join("icp.yaml"), "canisters: []\n").unwrap();
        let app = directory.join("apps/admission");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::copy(
            directory.join(&desired.protocol.as_ref().unwrap().app_config),
            app.join("canic.toml"),
        )
        .unwrap();
        std::fs::write(directory.join("root-key.der"), pic.root_key().unwrap()).unwrap();
        let wrapper = operator_cli_gateway_wrapper(directory, &replica.url, isolated_icp);
        let added = Principal::self_authenticating([0xa8; 32]);
        let added_text = added.to_text();
        let plan_path = directory.join("admission-add.json");
        run_environment_cli(
            directory,
            &wrapper,
            &desired.environment,
            &[
                "admission",
                "plan",
                &desired.fleet,
                "--add",
                &added_text,
                "--fleet",
                "--out",
                plan_path.to_str().unwrap(),
            ],
        )
        .expect("plan admission using only immutable release sidecars");
        let plan: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&plan_path).unwrap()).unwrap();
        let request: FleetAdmissionMutationRequest =
            serde_json::from_value(plan["request"].clone()).unwrap();
        run_environment_cli(
            directory,
            &wrapper,
            &desired.environment,
            &[
                "admission",
                "apply",
                &desired.fleet,
                plan_path.to_str().unwrap(),
            ],
        )
        .expect("apply reviewed admission through ordinary CLI");
        let operator = Principal::from_text(&desired.operator).unwrap();
        let coordinator = request.authority.coordinator;
        let completed =
            await_fleet_admission_convergence_as(pic, coordinator, request.operation_id, operator);
        assert_eq!(completed.outcome, FleetAdmissionMutationOutcome::Converged);
        run_environment_cli(
            directory,
            &wrapper,
            &desired.environment,
            &["admission", "status", &desired.fleet, "--json"],
        )
        .expect("inspect converged admission through ordinary CLI");
        run_environment_cli(
            directory,
            &wrapper,
            &desired.environment,
            &[
                "admission",
                "apply",
                &desired.fleet,
                plan_path.to_str().unwrap(),
            ],
        )
        .expect("replay exact completed admission");
        let registry: Result<CoordinatorRegistryResponse, Error> = pic
            .query_candid_as(
                coordinator,
                operator,
                canic::protocol::CANIC_COORDINATOR_REGISTRY,
                (CoordinatorRegistryRequest::Registry,),
            )
            .unwrap();
        let CoordinatorRegistryResponse::Registry(registry) = registry.unwrap();
        assert!(registry.admission.fleet_principals.contains(&added));
        assert_eq!(
            registry.admission.generation,
            request.expected_generation + 1
        );
    }

    #[cfg(test)]
    fn run_environment_cli(
        directory: &Path,
        wrapper: &Path,
        environment: &str,
        args: &[&str],
    ) -> Result<(), canic_cli::CliError> {
        use std::ffi::OsString;
        struct RestoreDirectory(PathBuf);
        impl Drop for RestoreDirectory {
            fn drop(&mut self) {
                std::env::set_current_dir(&self.0).unwrap();
            }
        }
        let _restore = RestoreDirectory(std::env::current_dir().unwrap());
        std::env::set_current_dir(directory).unwrap();
        let mut invocation = vec![
            OsString::from("--environment"),
            OsString::from(environment),
            OsString::from("--icp"),
            wrapper.as_os_str().to_owned(),
        ];
        invocation.extend(args.iter().map(OsString::from));
        canic_cli::run(invocation)
    }

    #[cfg(test)]
    fn resume_operator_cli(
        directory: &Path,
        wrapper: &Path,
    ) -> canic_host::component_operation::model::ComponentOperationRecord {
        use canic_host::component_operation::ops;
        let path = ops::record_path(directory, "ic", "fixture", "extra").unwrap();
        let planned = ops::read(&path).unwrap().unwrap();
        assert_eq!(planned.submission_attempts, 0);
        let review = &planned.plan.review_sha256;
        assert!(
            run_operator_cli(
                directory,
                wrapper,
                &[
                    "component",
                    "apply",
                    "fixture",
                    "extra",
                    "--review",
                    "wrong"
                ]
            )
            .is_err()
        );
        assert_eq!(ops::read(&path).unwrap().unwrap(), planned);
        std::fs::write(directory.join("root-key.der.lose-reply"), []).unwrap();
        assert!(
            run_operator_cli(
                directory,
                wrapper,
                &[
                    "component",
                    "apply",
                    "fixture",
                    "extra",
                    "--review",
                    review,
                    "--wait-secs",
                    "0"
                ]
            )
            .is_err()
        );
        assert_eq!(ops::read(&path).unwrap().unwrap().submission_attempts, 1);
        run_operator_cli(
            directory,
            wrapper,
            &[
                "component",
                "apply",
                "fixture",
                "extra",
                "--review",
                review,
                "--wait-secs",
                "120",
            ],
        )
        .unwrap();
        let terminal = ops::read(&path).unwrap().unwrap();
        assert!(terminal.progress.as_ref().unwrap().complete);
        terminal
    }

    #[cfg(test)]
    fn assert_operator_cli_export(directory: &Path, wrapper: &Path, binding: &ComponentBinding) {
        let destination = directory.join("frontend-environment.json");
        run_operator_cli(
            directory,
            wrapper,
            &[
                "info",
                "env",
                "fixture",
                "--component-operation",
                "extra",
                "--json",
                "--out",
                destination.to_str().unwrap(),
            ],
        )
        .unwrap();
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(destination).unwrap()).unwrap();
        assert!(report["bindings"].as_array().unwrap().iter().any(|entry| {
            entry["canister_id"] == binding.canister_id.to_text()
                && entry["role"] == binding.role.to_string()
        }));
    }

    #[test]
    fn operator_component_public_cli_uses_real_icp_and_exports_terminal_binding() {
        use canic_host::component_operation::{ComponentOperationError, ops, workflow};
        let mut fixture = setup_active_component_registry_with_pic(build_management_pic);
        let directory =
            std::env::temp_dir().join(format!("canic-operator-cli-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let (isolated, operator, _) = prepare_isolated_icp(&directory);
        for principal in [fixture.root, fixture.coordinator] {
            fixture
                .pic()
                .update_canister_settings(
                    principal,
                    None,
                    ic_testkit::pocket_ic::CanisterSettings {
                        controllers: Some(vec![Principal::anonymous(), operator]),
                        ..Default::default()
                    },
                )
                .unwrap();
        }
        operator_cli_terminal_fixture(&directory, &fixture, operator);
        std::fs::write(directory.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        std::fs::write(directory.join("icp.yaml"), "canisters: []\n").unwrap();
        std::fs::create_dir_all(directory.join("apps/test")).unwrap();
        std::fs::copy(
            root_canister_config_path(&workspace_root_for(env!("CARGO_MANIFEST_DIR"))),
            directory.join("apps/test/canic.toml"),
        )
        .unwrap();
        std::fs::write(
            directory.join("root-key.der"),
            fixture.pic().root_key().unwrap(),
        )
        .unwrap();
        let gateway = fixture.start_http_gateway();
        let wrapper = operator_cli_gateway_wrapper(&directory, &gateway, &isolated);
        let spec = fixture.issuer.component_spec.to_string();
        run_operator_cli(
            &directory,
            &wrapper,
            &[
                "component",
                "plan",
                "fixture",
                "extra",
                "--root",
                "root",
                "--spec",
                &spec,
            ],
        )
        .unwrap();
        let terminal = resume_operator_cli(&directory, &wrapper);
        let review = &terminal.plan.review_sha256;
        let binding = terminal
            .progress
            .as_ref()
            .unwrap()
            .binding
            .as_ref()
            .unwrap();
        assert_eq!(binding.role, fixture.issuer.role);
        assert_ne!(binding.canister_id, fixture.issuer.canister_id);
        assert_operator_cli_export(&directory, &wrapper, binding);

        let mut broken = ops::transport::IcpComponentTransport::new(
            &directory,
            canic_host::icp::IcpCli::new("/does-not-exist", Some("ic".to_string())),
        );
        assert_eq!(
            workflow::apply(&directory, "ic", "fixture", "extra", review, &mut broken).unwrap(),
            terminal
        );
        // A modified sidecar must fail before another observation or command is accepted.
        let candid = canic_host::icp::local_canister_candid_path(&directory, "ic", "root");
        std::fs::write(candid, "service : {};").unwrap();
        assert!(matches!(
            broken.authority("ic", "fixture", "root", &fixture.issuer.component_spec),
            Err(ComponentOperationError::ProtocolBinding(_))
        ));
        drop(fixture);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(test)]
    struct OperatorComponentTransport<'a> {
        fixture: &'a ActiveComponentRegistryFixture,
        authority: canic_host::component_operation::model::ComponentAuthorityRecord,
        caller: Principal,
        submissions: usize,
        lose_reply: bool,
    }

    #[cfg(test)]
    impl OperatorComponentTransport<'_> {
        fn query(
            &self,
            request: RootStatusRequestFragment,
        ) -> Result<
            RootStatusResponseFragment,
            canic_host::component_operation::ComponentOperationError,
        > {
            let method = match request {
                RootStatusRequestFragment::Operation(_) => {
                    canic::protocol::CANIC_ROOT_OPERATION_STATUS
                }
                _ => canic::protocol::CANIC_ROOT_STATUS,
            };
            let response: Result<RootStatusResponseFragment, Error> = self
                .fixture
                .pic()
                .query_candid_as(self.fixture.root, self.caller, method, (request,))
                .expect("operator Root query transport");
            response.map_err(|error| {
                canic_host::CanisterProtocolError::Response {
                    canister: self.fixture.root,
                    method,
                    source: canic_host::icp::IcpJsonResponseError::Rejected(error),
                }
                .into()
            })
        }
    }

    #[cfg(test)]
    impl canic_host::component_operation::ops::ComponentTransport for OperatorComponentTransport<'_> {
        fn observe(
            &mut self,
            plan: &canic_host::component_operation::model::ComponentPlanRecord,
        ) -> Result<
            canic_host::component_operation::view::ComponentObservation,
            canic_host::component_operation::ComponentOperationError,
        > {
            let RootStatusResponseFragment::FleetAuthority(authority) =
                self.query(RootStatusRequestFragment::FleetAuthority)?
            else {
                panic!("Root authority response");
            };
            assert_eq!(authority.binding, plan.authority.binding);
            assert_eq!(authority.initial_release_set, plan.authority.release_set);
            Ok(
                canic_host::component_operation::view::ComponentObservation {
                    authority: self.authority.clone(),
                    ready_assets: root_pool_status(self.fixture.pic(), self.fixture.root).ready,
                },
            )
        }

        fn progress(
            &mut self,
            plan: &canic_host::component_operation::model::ComponentPlanRecord,
        ) -> Result<
            canic_host::component_operation::view::ComponentProgressObservation,
            canic_host::component_operation::ComponentOperationError,
        > {
            use canic_host::component_operation::{
                ComponentOperationError, view::ComponentProgressObservation,
            };
            let response = self.query(RootStatusRequestFragment::Operation(
                OperationStatusRequest {
                    operation_id: plan.operation_id,
                },
            ));
            let response = match response {
                Err(ComponentOperationError::Protocol(
                    canic_host::CanisterProtocolError::Response {
                        source: canic_host::icp::IcpJsonResponseError::Rejected(error),
                        ..
                    },
                )) if error.code()
                    == canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code() =>
                {
                    return Ok(ComponentProgressObservation { progress: None });
                }
                other => other?,
            };
            let RootStatusResponseFragment::Operation(
                RootOperationStatusResponse::ProvisionComponent(status),
            ) = response
            else {
                panic!("Root Component status");
            };
            canic_host::component_operation::ops::project_progress(plan, status)
        }

        fn submit(
            &mut self,
            plan: &canic_host::component_operation::model::ComponentPlanRecord,
        ) -> Result<(), canic_host::component_operation::ComponentOperationError> {
            let response = root_command_as(
                self.fixture.pic(),
                self.fixture.root,
                self.caller,
                RootCommandFragment::ProvisionComponent(RootComponentAllocationRequest {
                    operation_id: plan.operation_id,
                    component_spec: plan.authority.component_spec.clone(),
                }),
            )
            .map_err(|error| canic_host::CanisterProtocolError::Response {
                canister: self.fixture.root,
                method: canic::protocol::CANIC_ROOT_COMMAND,
                source: canic_host::icp::IcpJsonResponseError::Rejected(error),
            })?;
            let RootCommandResponseFragment::OperationAccepted(receipt) = response else {
                panic!("Root Component receipt");
            };
            assert_eq!(receipt.operation_id, plan.operation_id);
            self.submissions += 1;
            if self.lose_reply {
                return Err(std::io::Error::from(std::io::ErrorKind::ConnectionReset).into());
            }
            Ok(())
        }
    }

    #[cfg(test)]
    fn operator_component_authority(
        fixture: &ActiveComponentRegistryFixture,
    ) -> canic_host::component_operation::model::ComponentAuthorityRecord {
        let RootStatusResponseFragment::FleetAuthority(root_authority) = root_status(
            fixture.pic(),
            fixture.root,
            RootStatusRequestFragment::FleetAuthority,
        )
        .unwrap() else {
            panic!("Root authority");
        };
        canic_host::component_operation::model::ComponentAuthorityRecord {
            environment: "local".to_string(),
            fleet: "fixture".to_string(),
            root_name: "root".to_string(),
            source_plan_sha256: canic_core::cdk::utils::hash::sha256_hex(
                b"operator runtime qualification source",
            ),
            binding: root_authority.binding,
            release_set: root_authority.initial_release_set,
            root_module_sha256: hex_bytes(root_authority.expected_module_hash),
            root_candid_sha256: [1; 32],
            root_controllers: vec![Principal::anonymous().to_text()],
            registry_sha256: [2; 32],
            operator: Principal::anonymous(),
            component_spec: fixture.issuer.component_spec.clone(),
            spec_hash: fixture.issuer.spec_hash,
            role: fixture.issuer.role.clone(),
        }
    }

    #[cfg(test)]
    fn advance_operator_component(
        directory: &Path,
        review: &str,
        transport: &mut OperatorComponentTransport<'_>,
    ) -> canic_host::component_operation::model::ComponentOperationRecord {
        // Duplicate exact commands must coalesce into one active Root driver.
        canic_host::component_operation::workflow::apply(
            directory, "local", "fixture", "core", review, transport,
        )
        .unwrap();
        canic_host::component_operation::workflow::apply(
            directory, "local", "fixture", "core", review, transport,
        )
        .unwrap();
        for _ in 0..80 {
            transport.fixture.pic().advance_time(Duration::from_secs(1));
            transport.fixture.pic().tick();
            let observed = canic_host::component_operation::workflow::status(
                directory, "local", "fixture", "core", transport,
            )
            .unwrap();
            if observed
                .progress
                .as_ref()
                .is_some_and(|progress| progress.complete)
            {
                return observed;
            }
        }
        panic!("operator Component did not reach terminal activation");
    }

    #[test]
    fn operator_component_recovers_lost_response_and_replays_terminal_binding() {
        use canic_host::component_operation::{ComponentOperationError, workflow};
        let fixture = setup_active_component_registry();
        let directory = std::env::temp_dir().join(format!(
            "canic-operator-component-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _cleanup = TestDirectoryCleanup(directory.clone());
        let authority = operator_component_authority(&fixture);
        // This case qualifies the production journal and Root lifecycle. Exact local
        // ICP sidecar/terminal-inventory resolution is a separate transport boundary.
        let mut transport = OperatorComponentTransport {
            fixture: &fixture,
            authority: authority.clone(),
            caller: Principal::anonymous(),
            submissions: 0,
            lose_reply: true,
        };
        let reviewed = workflow::plan(&directory, "core", authority, &mut transport).unwrap();
        let review = &reviewed.plan.review_sha256;
        let before = root_pool_status(fixture.pic(), fixture.root).workload;
        transport.caller = Principal::from_slice(&[99]);
        let denied = workflow::apply(
            &directory,
            "local",
            "fixture",
            "core",
            review,
            &mut transport,
        );
        assert!(
            matches!(denied, Err(ComponentOperationError::Protocol(canic_host::CanisterProtocolError::Response { source: canic_host::icp::IcpJsonResponseError::Rejected(error), .. })) if error.code() == canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code())
        );
        transport.caller = Principal::anonymous();
        assert!(matches!(
            workflow::apply(
                &directory,
                "local",
                "fixture",
                "core",
                review,
                &mut transport
            ),
            Err(ComponentOperationError::Io(_))
        ));
        fixture
            .pic()
            .upgrade_canister(
                fixture.root,
                build_test_root_wasm(),
                crate::pic::upgrade_args(),
                None,
            )
            .expect("restore same-release Root after lost submission reply");
        transport.lose_reply = false;
        let terminal = advance_operator_component(&directory, review, &mut transport);
        let submissions = transport.submissions;
        assert!((2..=3).contains(&submissions));
        assert_eq!(
            root_pool_status(fixture.pic(), fixture.root).workload,
            before + 1
        );
        let progress = terminal.progress.as_ref().unwrap();
        assert_eq!(
            progress.phase,
            canic_host::component_operation::model::ComponentPhase::Committed
        );
        let binding = progress.binding.as_ref().unwrap();
        assert_ne!(binding.canister_id, fixture.issuer.canister_id);
        assert_eq!(binding.role, fixture.issuer.role);
        let balance = fixture.pic().cycle_balance(fixture.root);
        transport.caller = Principal::from_slice(&[99]);
        assert_eq!(
            workflow::apply(
                &directory,
                "local",
                "fixture",
                "core",
                review,
                &mut transport
            )
            .unwrap(),
            terminal
        );
        assert_eq!(fixture.pic().cycle_balance(fixture.root), balance);
        assert_eq!(transport.submissions, submissions);
    }

    #[test]
    fn funded_estate_recovers_transfer_and_autonomous_creation_responses() {
        assert_literal_zero_host_journey(FundingJourney::Estate, 1);
    }

    #[test]
    fn issued_creation_funding_pause_resumes_reviewed_transfer() {
        assert_literal_zero_host_journey(FundingJourney::FundingPause, 1);
    }

    #[test]
    fn four_workloads_refill_four_ready_with_lost_funding_and_creation_responses() {
        assert_literal_zero_host_journey(FundingJourney::Estate, 4);
    }

    #[test]
    fn four_workloads_and_four_failed_assets_repair_without_new_creation() {
        assert_literal_zero_host_journey(FundingJourney::FailedReserve, 4);
    }

    #[test]
    fn generated_reinstall_recovers_lost_install_and_reaches_working_fleet() {
        assert_literal_zero_host_journey(FundingJourney::Reinstall, 19);
    }

    #[test]
    fn generated_mixed_topology_and_ready_reserve_recover_one_reviewed_operation() {
        assert_literal_zero_host_journey(FundingJourney::Fresh, 5);
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one governed control-plane journey keeps Prepared Root inspection, controller finalization, Component convergence, conservation and terminal replay together"
    )]
    fn assert_literal_zero_host_journey(funding: FundingJourney, initial_workload_count: usize) {
        let journey_span = Span::start("production_adapter_journey");
        let mut phase = Span::start("source_and_fixture_recipe");
        let funded_import_repair = matches!(funding, FundingJourney::FailedImports);
        let failed_reserve = matches!(funding, FundingJourney::FailedReserve);
        let fund_estate = matches!(
            funding,
            FundingJourney::Estate
                | FundingJourney::FailedReserve
                | FundingJourney::FundingPause
                | FundingJourney::NativeFunding
        );
        let build_network = if fund_estate {
            BuildNetwork::Ic
        } else {
            BuildNetwork::Local
        };
        let journey_started_at = Instant::now();
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = workspace_root.join(match initial_workload_count {
            1 if matches!(funding, FundingJourney::Fresh) => {
                "canisters/audit/root_probe/frontend.toml"
            }
            1 => "canisters/audit/root_probe/activation.toml",
            2 => "canisters/audit/root_probe/native-child-recovery.toml",
            4 => "canisters/audit/root_probe/four-workloads.toml",
            5 => "apps/test/test-configs/generated-mixed-topology.toml",
            19 => "canisters/audit/root_probe/retained-estate.toml",
            _ => panic!("unsupported journey Workload count: {initial_workload_count}"),
        });
        let config =
            AppConfigSnapshot::load(&config_path).expect("load initial-child Component config");
        let readiness_floor = config
            .model()
            .component_specs
            .values()
            .map(|component| component.initial_cycles.to_u128())
            .max()
            .expect("at least one initial-child Component Spec");
        let configuration = config
            .model()
            .compile_component_deployment_configuration()
            .expect("compile initial-child Component deployment configuration");
        let configured_roles = config
            .model()
            .roles
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let adapter_root = literal_zero_adapter_root(&workspace_root);
        if adapter_root.exists() {
            std::fs::remove_dir_all(&adapter_root)
                .expect("clear prior CANIC-121 production-adapter fixture");
        }
        std::fs::create_dir_all(&adapter_root)
            .expect("create CANIC-121 production-adapter fixture");
        let _adapter_root_cleanup = TestDirectoryCleanup(adapter_root.clone());
        let frontend_identity = (initial_workload_count == 1
            && matches!(funding, FundingJourney::Fresh))
        .then(|| prepare_frontend_identity(&adapter_root, "browser-identity.json"));
        phase = phase.next("initial_artifacts");
        let release_artifacts = build_literal_zero_release_artifacts(
            &workspace_root,
            &adapter_root,
            &config_path,
            &configuration,
            &configured_roles,
            build_network,
            INTERNAL_TEST_RELEASE_BUILD_NONCE,
        );
        let generated_fixtures =
            canic_host::release_set::load_persisted_current_release_set_manifest(
                &adapter_root,
                release_artifacts.release_build_id,
            )
            .unwrap()
            .manifest
            .verify_fixtures(&adapter_root, &configuration.component_topology)
            .unwrap();
        let root_wasm = release_artifacts.root_wasm_bytes.clone();
        let cycles_ledger_wasm = build_journey_cycles_ledger_wasm();
        let store_fixture = build_root_store_fixture_with_config_for_release(
            &config_path,
            &release_artifacts.component_wasms,
            release_artifacts.release_build_id,
        );
        phase = phase.next("replica_and_transport_setup");
        let (icp_wrapper, operator, controller_mutation_log) = prepare_isolated_icp(&adapter_root);
        let mut pic = build_management_pic();
        let subnet = *pic
            .topology()
            .get_app_subnets()
            .first()
            .expect("one application Subnet");

        let ready_count = match initial_workload_count {
            1 | 4 => initial_workload_count,
            19 => 5,
            _ => 1,
        };
        let pool_maximum_size = ready_count + initial_workload_count;
        let pool_count = if fund_estate {
            initial_workload_count
        } else {
            pool_maximum_size
        };
        let ledger_fee = if fund_estate {
            MAINNET_REFILL_LEDGER_FEE
        } else {
            0
        };
        let management_fee = if fund_estate {
            MAINNET_REFILL_MANAGEMENT_CREATION_FEE
        } else {
            0
        };
        let pool_creation_funding = if funded_import_repair {
            // The repair starts from an intentionally underfunded import, with its
            // original creation budget still sufficient for the reviewed controller work.
            readiness_floor
                .checked_sub(500_000_000_000)
                .expect("fixture readiness floor")
        } else if matches!(funding, FundingJourney::Fresh | FundingJourney::Reinstall) {
            canic_host::fleet_ensure::fresh_pool_creation_funding(readiness_floor)
                .expect("generator-owned initial pool funding")
        } else {
            readiness_floor
                .checked_add(3_000_000_000_000)
                .expect("pool funding fits u128")
        };
        let mut requested = vec![
            ("coordinator", COORDINATOR_INSTALL_CYCLES),
            ("root", ROOT_INSTALL_CYCLES),
            ("store", ROOT_INSTALL_CYCLES),
        ];
        requested.extend((0..pool_count).map(|_| ("pool", pool_creation_funding)));
        let create_result = |cycles, controllers: Vec<Principal>| {
            let canister = pic
                .create_canister_with_params(
                    None,
                    CreateCanisterParams {
                        cycles: Some(cycles),
                        placement: Some(CreateCanisterPlacement::SubnetId(subnet)),
                        settings: Some(CanisterSettings {
                            controllers: Some(controllers),
                            ..CanisterSettings::default()
                        }),
                    },
                )
                .expect("prepare one deterministic Cycles Ledger result");
            if matches!(funding, FundingJourney::Fresh) {
                assert_eq!(
                    pic.cycle_balance(canister),
                    cycles,
                    "the Ledger result starts with its exact requested net balance"
                );
            }
            canister
        };
        let coordinator = create_result(COORDINATOR_INSTALL_CYCLES, vec![operator]);
        let root = create_result(ROOT_INSTALL_CYCLES, vec![operator]);
        let mut root_and_operator = vec![root, operator];
        root_and_operator.sort();
        let mut root_and_operator_wire = vec![root.to_text(), operator.to_text()];
        root_and_operator_wire.sort();
        let root_and_operator_wire = root_and_operator_wire
            .into_iter()
            .map(|principal| {
                Principal::from_text(principal).expect("canonical controller Principal")
            })
            .collect::<Vec<_>>();
        let store = create_result(ROOT_INSTALL_CYCLES, root_and_operator.clone());
        let pools = (0..pool_count)
            .map(|_| create_result(pool_creation_funding, root_and_operator.clone()))
            .collect::<Vec<_>>();
        let autonomous_assets = (0..if fund_estate {
            pool_maximum_size - pool_count
        } else {
            0
        })
            .map(|_| {
                create_result(
                    if failed_reserve {
                        readiness_floor - 500_000_000_000
                    } else {
                        readiness_floor + MAINNET_REFILL_EXECUTION_MARGIN
                    },
                    vec![root],
                )
            })
            .collect::<Vec<_>>();
        let placement = BootstrappedRootPlacement {
            canister_pool_maximum_size: Some(
                u32::try_from(pool_maximum_size).expect("literal-zero pool maximum fits u32"),
            ),
            canister_pool_minimum_size: Some(
                u32::try_from(ready_count).expect("literal-zero Ready reserve fits u32"),
            ),
            canister_pool_cycles: Some(Cycles::new(readiness_floor)),
            coordinator_subnet: Some(subnet),
            existing_root: Some(root),
            existing_wasm_store: Some(store),
            root_subnet: Some(subnet),
            component_admission_limits: Some(RootComponentAdmissionLimits::Uniform(
                if initial_workload_count == 4 { 4 } else { 1 },
            )),
            fleet_id: Some(FleetId::from_generated_bytes([0x79; 32])),
            funding: None,
            coordinator_root_funding: None,
        };
        let mut installed = prepare_current_root_fixture(
            &pic,
            &root_wasm,
            &release_artifacts.store_wasm_bytes,
            coordinator,
            root,
            store,
            operator,
            store_fixture,
            &placement,
            &config_path,
            pools.clone(),
        );
        rebind_fixture_release_build_id(
            &mut installed.init_args,
            release_artifacts.release_build_id,
        );
        let admission = compile_fleet_admission_policy_template(
            vec![Principal::from_slice(&[1; 29])],
            Vec::new(),
        )
        .expect("compile literal-zero Fleet admission template");
        let authority = &installed.init_args.authority;
        let bootstrap = DesiredFleetBootstrap {
            admission_identity_origin: None,
            admission,
            app: authority.binding.authority.binding.fleet.app.clone(),
            canonical_network_id: authority
                .binding
                .authority
                .binding
                .fleet
                .fleet
                .canonical_network_id,
            component_deployment_configuration: configuration.clone(),
            coordinator: "coordinator".to_string(),
            coordinator_subnet: authority.binding.authority.binding.coordinator_subnet,
            fleet_id: authority.binding.authority.binding.fleet.fleet.fleet_id,
            fresh_estate: true,
            release_build_id: authority.initial_release_set.release_build_id,
            root_funding: Some(installed.coordinator_root_funding.clone()),
            roots: vec![DesiredFleetBootstrapRoot {
                canister_pool_imports: (0..pool_count)
                    .map(|index| format!("pool-{index}"))
                    .collect(),
                component_admissions: authority.binding.component_admissions.clone(),
                component_topology_digest: authority.binding.component_topology_digest,
                funding: authority.binding.funding.clone(),
                limits: authority.binding.limits.clone(),
                placement_subnet: authority.binding.placement_subnet,
                root: "root".to_string(),
                store: "store".to_string(),
            }],
        };
        let component_group_placements = configuration
            .deployment_topology
            .component_group_deployments
            .iter()
            .flat_map(|deployment| {
                (0..deployment.initial_placements).map(move |ordinal| {
                    DesiredComponentGroupPlacement {
                        deployment: deployment.deployment.to_string(),
                        ordinal,
                        root: "root".to_string(),
                    }
                })
            })
            .collect();
        let protocol = DesiredFleetProtocol {
            app_config: config_path.to_string_lossy().into_owned(),
            component_group_placements,
            coordinator_candid: release_artifacts.coordinator_candid.clone(),
            root_candid: release_artifacts.root_candid.clone(),
            store_candid: release_artifacts.store_candid.clone(),
        };
        let mut desired = literal_zero_fleet_desired(LiteralZeroFleetInput {
            bootstrap,
            coordinator_wasm: Path::new(&release_artifacts.coordinator_wasm),
            cycles_ledger: Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai")
                .expect("canonical Cycles Ledger Principal"),
            operator,
            pool_count,
            pool_creation_funding,
            pool_minimum_cycles: if funded_import_repair {
                0
            } else {
                readiness_floor
            },
            protocol,
            root_wasm: Path::new(&release_artifacts.root_wasm),
            store_wasm: Path::new(&release_artifacts.store_wasm),
            subnet,
        });
        if fund_estate {
            // The host transport remains explicitly local. The sealed Wasms and
            // canonical Fleet identity select the real mainnet runtime contract.
            assert_eq!(desired.environment, "local");
            desired.ledger_fee_cycles = ledger_fee.to_string();
            desired.management_creation_fee_cycles = management_fee.to_string();
            assert_eq!(
                desired.bootstrap.as_ref().unwrap().canonical_network_id,
                canic_core::ids::CanonicalNetworkId::ic_mainnet()
            );
        }
        let cycles_ledger = Principal::from_text(&desired.cycles_ledger)
            .expect("literal-zero Cycles Ledger Principal");
        pic.create_canister_with_id(None, None, cycles_ledger)
            .expect("create canonical Cycles Ledger stub principal");
        let mut total_requested = requested.iter().map(|(_, cycles)| cycles).sum::<u128>();
        let operator_balance = 2_000_000_000_000_000_u128;
        if funded_import_repair
            || failed_reserve
            || matches!(
                funding,
                FundingJourney::NativeFunding
                    | FundingJourney::NativeChildFunding
                    | FundingJourney::Reinstall
            )
        {
            pic.add_cycles(cycles_ledger, operator_balance);
        }
        assert!(operator_balance > total_requested);
        if fund_estate {
            install_journey_registry(
                &pic,
                &cycles_ledger_wasm,
                root,
                subnet,
                [coordinator, root, store]
                    .into_iter()
                    .chain(pools.iter().copied())
                    .chain(autonomous_assets.iter().copied())
                    .collect(),
            );
        }
        pic.install_canister(
            cycles_ledger,
            cycles_ledger_wasm,
            encode_one(CyclesLedgerStubInitArgs {
                canister_ids: [coordinator, root, store]
                    .into_iter()
                    .chain(pools.iter().copied())
                    .chain(autonomous_assets.iter().copied())
                    .collect(),
                expected_controllers_by_index: Some(
                    [
                        vec![operator],
                        vec![operator],
                        root_and_operator_wire.clone(),
                    ]
                    .into_iter()
                    .chain((0..pool_count).map(|_| root_and_operator_wire.clone()))
                    .chain(autonomous_assets.iter().map(|_| vec![root]))
                    .collect(),
                ),
                expected_root: root,
                expected_subnet: subnet,
                initial_balances: Some(vec![
                    CyclesLedgerStubAccountBalance {
                        balance: Nat::from(operator_balance),
                        owner: operator,
                    },
                    CyclesLedgerStubAccountBalance {
                        balance: Nat::from(0_u8),
                        owner: root,
                    },
                ]),
                pending_first_index: fund_estate.then_some(u64::try_from(3 + pool_count).unwrap()),
                withdrawal_fee: Some(Nat::from(ledger_fee)),
            })
            .expect("encode literal-zero Cycles Ledger authority"),
            None,
        );

        phase = phase.next("generation_and_initial_review");
        let live_url = pic.make_live(None);
        let local_replica = LocalReplicaTarget {
            environment: "local".into(),
            root_key: hex_bytes(pic.root_key().expect("PocketIC local root key")),
            url: live_url.to_string(),
        };
        let desired = if matches!(funding, FundingJourney::Fresh | FundingJourney::Reinstall) {
            generate_journey_desired(GeneratedJourneyInput {
                root: &adapter_root,
                config: &config_path,
                icp_wrapper: &icp_wrapper,
                local_replica: &local_replica,
                operator,
                subnet,
                release_build_id: release_artifacts.release_build_id,
                root_key: &pic.root_key().expect("PocketIC trust anchor"),
                workload_count: initial_workload_count,
                ready_count,
                frontend_identity,
            })
        } else {
            desired
        };
        let desired_identity = desired_sha256(&desired);
        let mut platform = literal_zero_journey_platform(
            &desired,
            &icp_wrapper,
            &adapter_root,
            local_replica.clone(),
            !matches!(funding, FundingJourney::Fresh),
        );
        let infrastructure_started_at = Instant::now();
        super::super::fixture::progress("planning literal-zero infrastructure");
        let planned = fleet_ensure_workflow::plan(
            &adapter_root,
            &desired,
            &desired_identity,
            &desired.fleet,
            1_800_000_000_000_000_000,
            &mut platform,
        )
        .expect("plan literal-zero estate through production adapter");
        let initial_actions = planned_actions(&planned.plan);
        // The Ledger stub returns precreated canisters. Match the initial reviewed
        // Root balance, including startup prepayment, before any host effect occurs.
        let root_name = &desired
            .canisters
            .iter()
            .find(|canister| {
                canister.kind == canic_host::fleet_ensure::model::DesiredCanisterKind::Root
            })
            .unwrap()
            .name;
        let root_creation_cycles = initial_actions
            .iter()
            .find_map(|action| match action {
                EnsureAction::Create {
                    name,
                    requested_initial_cycles,
                    ..
                } if name == root_name => Some(*requested_initial_cycles),
                _ => None,
            })
            .unwrap();
        let startup_credit = root_creation_cycles
            .checked_sub(ROOT_INSTALL_CYCLES)
            .unwrap();
        if startup_credit > 0 {
            pic.add_cycles(root, startup_credit);
            total_requested += startup_credit;
        }
        assert_eq!(
            initial_actions
                .iter()
                .filter(|action| matches!(action, EnsureAction::Create { .. }))
                .count(),
            3 + pool_count
        );
        assert_eq!(
            initial_actions
                .iter()
                .filter(|action| matches!(action, EnsureAction::Install { .. }))
                .count(),
            3
        );
        assert_eq!(
            initial_actions
                .iter()
                .filter(|action| matches!(action, EnsureAction::SetControllers { .. }))
                .count(),
            pool_count
        );
        assert!(
            initial_actions
                .iter()
                .all(|action| !matches!(action, EnsureAction::FleetProtocol { .. })),
            "literal-zero planning must finish infrastructure before protocol effects"
        );
        let desired = if fund_estate
            || funded_import_repair
            || matches!(
                funding,
                FundingJourney::Reinstall
                    | FundingJourney::NativeChildFunding
                    | FundingJourney::ActivationReset
            ) {
            prepare_journey_infrastructure(
                &pic,
                &adapter_root,
                &desired,
                &planned.plan,
                &mut platform,
            )
        } else {
            desired
        };
        let desired_identity = desired_sha256(&desired);
        if !autonomous_assets.is_empty() || matches!(funding, FundingJourney::NativeChildFunding) {
            prepare_ready_imports(&pic, root, operator, &pools);
            let desired = if initial_workload_count == 1 {
                let config = retain_generated_journey_source(&adapter_root, &config_path);
                super::super::growth::generate(
                    &adapter_root,
                    &config,
                    &icp_wrapper,
                    &local_replica,
                    &desired,
                )
            } else {
                desired
            };
            let funding_icp = if initial_workload_count == 1 {
                adapter_root.join("growth-generator-icp")
            } else {
                icp_wrapper
            };
            progress_elapsed("funding infrastructure prepared", infrastructure_started_at);
            assert_funded_autonomous_journey(AutonomousFundingJourney {
                adapter_root: &adapter_root,
                icp_wrapper: &funding_icp,
                local_replica: &local_replica,
                pic: &pic,
                desired: &desired,
                root,
                operator,
                cycles_ledger,
                assets: &autonomous_assets,
                imported: &pools,
                repair_failed_reserve: failed_reserve,
                funding_pause: matches!(funding, FundingJourney::FundingPause),
                native_pause: match funding {
                    FundingJourney::NativeFunding => {
                        Some(native_funding::Scenario::SyntheticMinimum)
                    }
                    FundingJourney::NativeChildFunding => {
                        Some(native_funding::Scenario::ChildClaim)
                    }
                    _ => None,
                },
                readiness_floor,
                operator_after_initial_creation: operator_balance
                    - total_requested
                    - u128::try_from(3 + pool_count).unwrap() * (ledger_fee + management_fee),
            });
            if failed_reserve {
                phase = phase.next("admission_cli_selected_release");
                assert_admission_cli_selected_release(
                    &adapter_root,
                    &funding_icp,
                    &pic,
                    &desired,
                    &local_replica,
                );
            }
            pic.stop_live();
            progress_elapsed(
                "funded autonomous production-adapter journey complete",
                journey_started_at,
            );
            phase.finish();
            journey_span.finish();
            return;
        }

        if matches!(funding, FundingJourney::ActivationReset) {
            prepare_ready_imports(&pic, root, operator, &pools);
            activation_reset::assert_journey(ReinstallJourney {
                adapter_root: &adapter_root,
                config: &config_path,
                icp_wrapper: &icp_wrapper,
                local_replica: &local_replica,
                pic: &pic,
                desired: &desired,
                coordinator,
                root,
                store,
                pools: &pools,
            });
            pic.stop_live();
            phase.finish();
            journey_span.finish();
            return;
        }
        if matches!(funding, FundingJourney::Reinstall) {
            phase = phase.next("initial_working_fleet");
            prepare_ready_imports(&pic, root, operator, &pools);
            let mut platform = literal_zero_journey_platform(
                &desired,
                &icp_wrapper,
                &adapter_root,
                local_replica.clone(),
                true,
            );
            let initial = fleet_ensure_workflow::plan(
                &adapter_root,
                &desired,
                &desired_identity,
                &desired.fleet,
                1_800_000_000_000_000_001,
                &mut platform,
            )
            .expect("review the working Fleet used by reinstall");
            let working = fleet_ensure_workflow::apply(
                &adapter_root,
                &desired,
                &desired_identity,
                &desired.fleet,
                &initial.plan.plan_sha256,
                &mut platform,
            )
            .expect("establish a working Fleet before generated reinstall");
            assert!(working.terminal);
            assert!(working.actual_conservation.is_some());
            let pool = root_pool_status_as(&pic, root, operator);
            assert_eq!(
                (pool.workload, pool.ready, pool.pending_reset),
                (
                    u32::try_from(initial_workload_count).unwrap(),
                    u32::try_from(ready_count).unwrap(),
                    0
                )
            );
            progress_elapsed(
                "working reinstall fixture prepared",
                infrastructure_started_at,
            );
            phase = phase.next("retained_estate_reinstall");
            assert_generated_reinstall_journey(ReinstallJourney {
                adapter_root: &adapter_root,
                config: &config_path,
                icp_wrapper: &icp_wrapper,
                local_replica: &local_replica,
                pic: &pic,
                desired: &desired,
                coordinator,
                root,
                store,
                pools: &pools,
            });
            phase = phase.next("cleanup");
            pic.stop_live();
            std::fs::remove_dir_all(&adapter_root).expect("remove completed reinstall fixture");
            phase.finish();
            journey_span.finish();
            progress_elapsed("generated reinstall journey complete", journey_started_at);
            return;
        }
        phase = phase.next("initial_recovery_and_convergence");
        let mut resumed_platform = literal_zero_journey_platform(
            &desired,
            &icp_wrapper,
            &adapter_root,
            local_replica.clone(),
            !matches!(funding, FundingJourney::Fresh),
        );
        if !funded_import_repair {
            std::fs::write(
                adapter_root.join("lost-reset-args.bin"),
                encode_one(HostRootCommandFragment::ImportPoolCanister(
                    PoolCanisterRequest {
                        canister_id: pools[0],
                    },
                ))
                .expect("encode exact automatic import"),
            )
            .expect("enable automatic reset response loss");
            let first = fleet_ensure_workflow::apply(
                &adapter_root,
                &desired,
                &desired_identity,
                &desired.fleet,
                &planned.plan.plan_sha256,
                &mut platform,
            );
            assert!(
                matches!(first, Err(EnsureWorkflowError::Platform(_))),
                "the production wrapper must lose one controller-update response: {first:?}"
            );
            let resumed = fleet_ensure_workflow::apply(
                &adapter_root,
                &desired,
                &desired_identity,
                &desired.fleet,
                &planned.plan.plan_sha256,
                &mut resumed_platform,
            );
            assert!(
                matches!(resumed, Err(EnsureWorkflowError::Platform(_))),
                "automatic continuation must reach the lost reset response: {resumed:?}"
            );
            assert!(adapter_root.join("lost-reset-response").is_file());
            assert_eq!(
                std::fs::read_to_string(&controller_mutation_log)
                    .expect("read exact controller mutation log")
                    .lines()
                    .count(),
                pool_count,
                "lost response and replay must not repeat a controller effect"
            );
            let request_count: u64 = pic
                .query_candid(cycles_ledger, "request_count", ())
                .expect("query literal-zero creation count");
            assert_eq!(
                request_count,
                u64::try_from(3 + pool_count).expect("literal-zero create count fits u64"),
                "Create replay must not debit twice"
            );
            progress_elapsed(
                "literal-zero infrastructure replay complete",
                infrastructure_started_at,
            );
        }

        let (protocol_terminal, repair_funding, withdrawals) = if funded_import_repair {
            for asset in &pools {
                pic.install_canister(*asset, b"\0asm\x01\0\0\0".to_vec(), Vec::new(), Some(root));
            }
            let failed = root_command_as(
                &pic,
                root,
                operator,
                RootCommandFragment::ImportPoolCanister(PoolCanisterRequest {
                    canister_id: pools[0],
                }),
            )
            .expect("observe the real underfunded import failure before repair review");
            assert!(
                matches!(failed, RootCommandResponseFragment::ImportPoolCanister(
                PoolImportResponse::ResetFailed { canister_id, .. }
            ) if canister_id == pools[0])
            );
            let pool = root_pool_status_as(&pic, root, operator);
            assert_eq!((pool.failed, pool.pending_reset, pool.ready), (1, 1, 0));
            let repair_started_at = Instant::now();
            let repair_plan = fleet_ensure_workflow::plan(
                &adapter_root,
                &desired,
                &desired_identity,
                &desired.fleet,
                1_800_000_000_000_000_001,
                &mut resumed_platform,
            )
            .expect("plan every exact fresh import reset through the production host");
            assert_eq!(repair_plan.plan.protocol_actions.iter().filter(|action| matches!(
            action, EnsureAction::FleetProtocol { action, .. }
                if matches!(action.as_ref(), CurrentFleetProtocolAction::ReconcilePoolAsset { .. })
        )).count(), pool_count);
            let funded_assets = planned_actions(&repair_plan.plan)
                .into_iter()
                .filter_map(|action| {
                    if let EnsureAction::Fund {
                        amount,
                        principal,
                        pool_funding: Some(pool_funding),
                        ..
                    } = action
                    {
                        assert_eq!(pool_funding.root, root.to_text());
                        Some((principal.clone(), *amount))
                    } else {
                        None
                    }
                })
                .collect::<BTreeMap<_, _>>();
            let repair_funding = funded_assets.values().sum::<u128>();
            if funded_import_repair {
                assert_eq!(
                    funded_assets.keys().cloned().collect::<BTreeSet<_>>(),
                    pools.iter().map(Principal::to_text).collect()
                );
                assert!(repair_funding > 0);
                assert!(
                    planned_actions(&repair_plan.plan)
                        .iter()
                        .all(|action| matches!(
                            action,
                            EnsureAction::Fund { .. } | EnsureAction::FleetProtocol { .. }
                        ))
                );
                let installed = pic.canister_status(pools[1], Some(root)).unwrap();
                assert!(installed.module_hash.is_some());
                assert!(installed.cycles < readiness_floor);
                pic.set_controllers(pools[1], Some(root), vec![root, operator])
                    .unwrap();
                let rejected = fleet_ensure_workflow::apply(
                    &adapter_root,
                    &desired,
                    &desired_identity,
                    &desired.fleet,
                    &repair_plan.plan.plan_sha256,
                    &mut resumed_platform,
                );
                assert!(matches!(rejected, Err(EnsureWorkflowError::Platform(
                    canic_host::fleet_ensure::ops::IcpEnsurePlatformError::FundingInspectionAuthorityConflict { .. }
                ))));
                let before: u64 = pic
                    .query_candid(cycles_ledger, "withdrawal_count", ())
                    .unwrap();
                assert_eq!(before, 0);
                pic.set_controllers(pools[1], Some(operator), vec![root])
                    .unwrap();
                std::fs::write(adapter_root.join("lose-funding-response"), [])
                    .expect("enable exact withdrawal response loss");
                let native_before_funding = pic.cycle_balance(pools[0]);
                let lost_funding = fleet_ensure_workflow::apply(
                    &adapter_root,
                    &desired,
                    &desired_identity,
                    &desired.fleet,
                    &repair_plan.plan.plan_sha256,
                    &mut resumed_platform,
                );
                assert!(
                    matches!(lost_funding, Err(EnsureWorkflowError::Platform(_))),
                    "lose one completed withdrawal reply: {lost_funding:?}"
                );
                let withdrawals: u64 = pic
                    .query_candid(cycles_ledger, "withdrawal_count", ())
                    .expect("query withdrawal receipts");
                assert_eq!(withdrawals, 1);
                let funded_ceiling = native_before_funding + funded_assets[&pools[0].to_text()];
                let observed_after_funding = pic.cycle_balance(pools[0]);
                let observation_burn = desired
                    .maximum_observation_burn_cycles
                    .parse::<Cycles>()
                    .expect("reviewed observation burn")
                    .to_u128();
                assert!(observed_after_funding <= funded_ceiling);
                assert!(funded_ceiling - observed_after_funding <= observation_burn);
                assert!(observed_after_funding >= readiness_floor);
                assert_eq!(
                    ledger_account_balance(&pic, cycles_ledger, operator),
                    Nat::from(
                        operator_balance - total_requested - funded_assets[&pools[0].to_text()]
                    )
                );
                resumed_platform = literal_zero_journey_platform(
                    &desired,
                    &icp_wrapper,
                    &adapter_root,
                    local_replica.clone(),
                    !matches!(funding, FundingJourney::Fresh),
                );
            } else {
                assert_eq!(repair_funding, 0);
            }
            std::fs::write(
                adapter_root.join("lost-reset-args.bin"),
                encode_one(HostRootCommandFragment::ImportPoolCanister(
                    PoolCanisterRequest {
                        canister_id: pools[0],
                    },
                ))
                .expect("encode exact reset fault boundary"),
            )
            .expect("retain the exact reset request for lost-response injection");
            let lost_reset = fleet_ensure_workflow::apply(
                &adapter_root,
                &desired,
                &desired_identity,
                &desired.fleet,
                &repair_plan.plan.plan_sha256,
                &mut resumed_platform,
            );
            assert!(
                matches!(lost_reset, Err(EnsureWorkflowError::Platform(_))),
                "lose exactly the first import reset reply: {lost_reset:?}"
            );
            let mut resumed_platform = literal_zero_journey_platform(
                &desired,
                &icp_wrapper,
                &adapter_root,
                local_replica.clone(),
                !matches!(funding, FundingJourney::Fresh),
            );
            let repaired = fleet_ensure_workflow::apply(
                &adapter_root,
                &desired,
                &desired_identity,
                &desired.fleet,
                &repair_plan.plan.plan_sha256,
                &mut resumed_platform,
            );
            assert!(
                matches!(
                    repaired,
                    Err(EnsureWorkflowError::SuccessorReviewRequired {
                        review: Some(_),
                        ..
                    })
                ),
                "reconciled imports must expose the provisioning successor: {repaired:?}"
            );
            assert_eq!(
                root_pool_status_as(&pic, root, operator).ready as usize,
                pool_count
            );
            assert_eq!(
                std::fs::read_to_string(adapter_root.join("reset-mutations.log"))
                    .expect("read reset mutation log")
                    .lines()
                    .count(),
                1,
                "protected Ready status must prevent replaying a completed reset"
            );
            let withdrawals: u64 = pic
                .query_candid(cycles_ledger, "withdrawal_count", ())
                .expect("query repaired withdrawal receipts");
            assert_eq!(
                withdrawals,
                if funded_import_repair {
                    u64::try_from(pool_count).expect("pool count")
                } else {
                    0
                }
            );
            assert_eq!(
                ledger_account_balance(&pic, cycles_ledger, operator),
                Nat::from(operator_balance - total_requested - repair_funding)
            );
            progress_elapsed("literal-zero imports reconciled by host", repair_started_at);
            let mut protocol_platform = literal_zero_journey_platform(
                &desired,
                &icp_wrapper,
                &adapter_root,
                local_replica.clone(),
                !matches!(funding, FundingJourney::Fresh),
            );
            let protocol_started_at = Instant::now();
            super::super::fixture::progress("converging literal-zero control plane");
            let protocol_plan = fleet_ensure_workflow::plan(
                &adapter_root,
                &desired,
                &desired_identity,
                &desired.fleet,
                1_800_000_000_000_000_002,
                &mut protocol_platform,
            )
            .expect("plan current control-plane protocol through production adapter");
            let protocol_actions = planned_actions(&protocol_plan.plan);
            assert!(
            protocol_actions
                .iter()
                .any(|action| matches!(
                    action,
                    EnsureAction::FleetProtocol { action, .. }
                        if matches!(action.as_ref(), CurrentFleetProtocolAction::ProvisionComponents { .. })
                )),
            "the governed production plan must include Component provisioning: {protocol_actions:#?}"
        );
            let protocol_terminal = fleet_ensure_workflow::apply(
                &adapter_root,
                &desired,
                &desired_identity,
                &desired.fleet,
                &protocol_plan.plan.plan_sha256,
                &mut protocol_platform,
            )
            .unwrap_or_else(|error| {
                eprintln!(
                    "terminal pool: {:?}",
                    root_pool_status_as(&pic, root, operator)
                );
                report_canister_diagnostics_batch(
                    &pic,
                    [
                        ("Coordinator".to_string(), coordinator, operator),
                        ("Root".to_string(), root, operator),
                    ]
                    .into_iter()
                    .chain(
                        pools
                            .iter()
                            .enumerate()
                            .map(|(index, pool)| (format!("pool-{index}"), *pool, root)),
                    ),
                    "literal-zero production-host activation",
                );
                panic!("apply complete current protocol through production adapter: {error:?}");
            });
            assert!(protocol_terminal.terminal);
            progress_elapsed("literal-zero control plane converged", protocol_started_at);
            (protocol_terminal, repair_funding, withdrawals)
        } else {
            let mut platform = literal_zero_journey_platform(
                &desired,
                &icp_wrapper,
                &adapter_root,
                local_replica.clone(),
                !matches!(funding, FundingJourney::Fresh),
            );
            if !generated_fixtures.manifest.entries.is_empty() {
                std::fs::write(adapter_root.join("lose-fixture-response"), []).unwrap();
                let interrupted = fleet_ensure_workflow::apply(
                    &adapter_root,
                    &desired,
                    &desired_identity,
                    &desired.fleet,
                    &planned.plan.plan_sha256,
                    &mut platform,
                );
                assert!(
                    matches!(interrupted, Err(EnsureWorkflowError::Platform(_))),
                    "actual fixture upload must succeed with its response lost: {interrupted:?}"
                );
                assert!(adapter_root.join("lost-fixture-response").is_file());
                platform = literal_zero_journey_platform(
                    &desired,
                    &icp_wrapper,
                    &adapter_root,
                    local_replica.clone(),
                    false,
                );
            }
            let completed = fleet_ensure_workflow::apply(
                &adapter_root,
                &desired,
                &desired_identity,
                &desired.fleet,
                &planned.plan.plan_sha256,
                &mut platform,
            )
            .expect("resume the original reviewed plan through every automatic successor");
            assert!(completed.terminal);
            assert_eq!(completed.plan.plan_sha256, planned.plan.plan_sha256);
            let paths = canic_host::fleet_ensure::ops::EnsurePaths::under(
                &adapter_root,
                &desired.environment,
                &desired.fleet,
            );
            let journal = canic_host::fleet_ensure::ops::read_journal(&paths)
                .expect("read automatic phase journal")
                .expect("retained journal");
            assert!(!journal.successor_phases.is_empty());
            if !generated_fixtures.manifest.entries.is_empty() {
                assert_generated_fixture_review(&planned.plan, &journal, &desired);
            }
            assert_eq!(journal.plan_sha256, planned.plan.plan_sha256);
            assert!(
                journal
                    .successor_phases
                    .iter()
                    .all(|phase| phase.plan.as_ref().is_some_and(|plan| plan
                        .canisters
                        .iter()
                        .all(|canister| canister.actions.is_empty())))
            );
            (completed, 0_u128, 0_u64)
        };
        assert!(protocol_terminal.actual_conservation.is_some());
        assert_eq!(
            std::fs::read_to_string(adapter_root.join("reset-mutations.log"))
                .expect("completed reset log")
                .lines()
                .count(),
            1,
            "lost reset response must not repeat the completed reset"
        );
        let terminal_pool = root_pool_status_as(&pic, root, operator);
        assert_eq!(terminal_pool.workload as usize, initial_workload_count);
        assert_eq!(terminal_pool.ready as usize, ready_count);
        assert!(terminal_pool.pending_creation.is_none());
        let terminal_pool_statuses = pools
            .iter()
            .map(|pool| {
                pic.canister_status(*pool, Some(root))
                    .expect("observe terminal pool identity")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            terminal_pool_statuses
                .iter()
                .filter(|status| status.module_hash.is_some())
                .count(),
            initial_workload_count,
            "exact imported pool identities must become the top-level and initial-child Component Workloads"
        );
        assert_eq!(
            terminal_pool_statuses
                .iter()
                .filter(|status| status.module_hash.is_none())
                .count(),
            ready_count,
            "the configured Ready reserve must remain after allocating every initial Workload"
        );
        for status in &terminal_pool_statuses {
            assert_eq!(status.settings.controllers, vec![root]);
            assert!(status.cycles >= readiness_floor);
        }

        assert_generated_fixture_receipts(&pic, root, store, operator, &pools, &generated_fixtures);

        let final_controlled_cycles = [coordinator, root, store]
            .into_iter()
            .chain(pools.iter().copied())
            .map(|canister| pic.cycle_balance(canister))
            .sum::<u128>();
        // Include the reviewed Root startup prepayment in creation accounting,
        // just as the Ledger debit and precreated result do above.
        let requested_controlled_cycles = total_requested + repair_funding;
        let observed_net_cycle_debit_cycles = requested_controlled_cycles
            .checked_sub(final_controlled_cycles)
            .expect("fresh estate cannot gain unreviewed controlled cycles");
        assert_eq!(
            final_controlled_cycles + observed_net_cycle_debit_cycles,
            requested_controlled_cycles
        );

        let mut replay_platform = literal_zero_journey_platform(
            &desired,
            &icp_wrapper,
            &adapter_root,
            local_replica.clone(),
            !matches!(funding, FundingJourney::Fresh),
        );
        phase = phase.next("initial_terminal_replay");
        let replay_started_at = Instant::now();
        super::super::fixture::progress("proving literal-zero terminal replay");
        let same_plan = fleet_ensure_workflow::apply(
            &adapter_root,
            &desired,
            &desired_identity,
            &desired.fleet,
            &protocol_terminal.plan.plan_sha256,
            &mut replay_platform,
        )
        .expect("effect-free replay of the completed plan");
        assert!(same_plan.terminal);
        assert_eq!(same_plan.effects_applied, 0);
        let replay_plan = fleet_ensure_workflow::plan(
            &adapter_root,
            &desired,
            &desired_identity,
            &desired.fleet,
            1_800_000_000_000_000_003,
            &mut replay_platform,
        )
        .expect("plan terminal current-protocol replay");
        assert!(planned_actions(&replay_plan.plan).is_empty());
        let replay = fleet_ensure_workflow::apply(
            &adapter_root,
            &desired,
            &desired_identity,
            &desired.fleet,
            &replay_plan.plan.plan_sha256,
            &mut replay_platform,
        )
        .expect("apply effect-free terminal replay");
        assert!(replay.terminal);
        assert_eq!(replay.effects_applied, 0);
        assert_eq!(
            std::fs::read_to_string(&controller_mutation_log)
                .expect("reread exact controller mutation log")
                .lines()
                .count(),
            if funded_import_repair { 0 } else { pool_count },
            "protocol convergence and terminal replay must not repeat controller effects"
        );
        let replay_withdrawals: u64 = pic
            .query_candid(cycles_ledger, "withdrawal_count", ())
            .expect("query replayed withdrawal receipts");
        assert_eq!(replay_withdrawals, withdrawals);
        assert_eq!(
            ledger_account_balance(&pic, cycles_ledger, operator),
            Nat::from(operator_balance - requested_controlled_cycles)
        );
        assert_eq!(
            ledger_account_balance(&pic, cycles_ledger, root),
            Nat::from(0_u8)
        );
        if !generated_fixtures.manifest.entries.is_empty() {
            let unique_chunks = generated_fixtures
                .manifest
                .entries
                .iter()
                .map(|entry| entry.descriptor.chunks.len())
                .sum::<usize>();
            assert_eq!(
                std::fs::read_to_string(adapter_root.join("fixture-publications.log"))
                    .unwrap()
                    .lines()
                    .count(),
                unique_chunks,
                "lost upload response, fresh adapter and terminal replay must not publish twice"
            );
        }
        progress_elapsed("literal-zero terminal replay complete", replay_started_at);
        if initial_workload_count == 5 && matches!(funding, FundingJourney::Fresh) {
            phase = phase.next("selected_build_reinstall");
            assert_selected_build_reinstall_journey(ReinstallJourney {
                adapter_root: &adapter_root,
                config: &config_path,
                icp_wrapper: &icp_wrapper,
                local_replica: &local_replica,
                pic: &pic,
                desired: &desired,
                coordinator,
                root,
                store,
                pools: &pools,
            });
            phase = phase.next("public_memory_allocation_qualification");
            assert_public_memory_allocation_samples(&pic, root, store, operator, &pools);
        }

        if let Some(caller) = frontend_identity {
            phase = phase.next("frontend_cli_and_sdk");
            assert_frontend_handoff(&adapter_root, &icp_wrapper, &mut pic, &desired, caller);
        }
        phase = phase.next("cleanup");
        pic.stop_live();
        std::fs::remove_dir_all(adapter_root)
            .expect("remove literal-zero production-adapter fixture");
        progress_elapsed(
            "literal-zero production-adapter journey complete",
            journey_started_at,
        );
        phase.finish();
        journey_span.finish();
    }

    #[cfg(test)]
    fn assert_generated_fixture_review(
        original: &FleetEnsurePlan,
        journal: &canic_host::fleet_ensure::model::FleetEnsureJournalRecord,
        desired: &DesiredFleet,
    ) {
        let mut publications = BTreeMap::new();
        for phase in &journal.successor_phases {
            for action in &phase.plan.as_ref().unwrap().protocol_actions {
                let EnsureAction::FleetProtocol {
                    action: publication,
                    maximum_execution_burn_cycles,
                    ..
                } = action
                else {
                    continue;
                };
                if let CurrentFleetProtocolAction::PrepareStoreFixture {
                    maximum_attempts: attempts,
                    ..
                }
                | CurrentFleetProtocolAction::PublishStoreFixtureChunk {
                    maximum_attempts: attempts,
                    ..
                } = publication.as_ref()
                {
                    assert_eq!(*attempts, desired.maximum_stalled_observations);
                    assert!(*maximum_execution_burn_cycles > 0);
                    publications.insert(
                        canic_host::fleet_ensure::ops::action_sha256(action),
                        *attempts,
                    );
                }
            }
        }
        assert!(
            !publications.is_empty(),
            "fixture publication must be a reviewed effect"
        );
        let retries: u32 = publications.values().map(|attempts| attempts - 1).sum();
        let authority = original.continuation.as_ref().unwrap();
        assert_eq!(authority.fixture_publication_retry_attempts, retries);
        assert!(authority.maximum_successor_actions as usize >= publications.len());
        let review = original.recovery_review.as_ref().unwrap();
        assert!(review.continuation_reserve_cycles > 0);
        assert!(review.whole_continuation_ceiling_cycles >= review.continuation_reserve_cycles);
    }

    #[cfg(test)]
    fn assert_generated_fixture_receipts(
        pic: &PocketIc,
        root: Principal,
        store: Principal,
        operator: Principal,
        pools: &[Principal],
        fixtures: &canic_host::release_set::fixture::PersistedFixtureArtifactManifest,
    ) {
        for source in &fixtures.manifest.entries {
            let status: Result<StoreCatalogResponse, Error> = pic
                .query_candid_as(
                    store,
                    operator,
                    canic::protocol::CANIC_WASM_STORE_CATALOG,
                    (StoreCatalogRequest::Fixture(source.content_id),),
                )
                .unwrap();
            let StoreCatalogResponse::Fixture(Ok(status)) = status.unwrap() else {
                panic!("generated apply must retain the selected source");
            };
            assert!(status.complete);
            assert_eq!(status.received_bytes, source.descriptor.encoded_length);
            let mut completed = Vec::new();
            for target in pools {
                if pic
                    .canister_status(*target, Some(root))
                    .unwrap()
                    .module_hash
                    .is_none()
                {
                    continue;
                }
                let readiness = fixture_readiness(pic, root, *target);
                if let Ok(canic::dto::fixture_provisioning::FixtureProvisioningStatus::Complete(
                    receipt,
                )) = readiness.fixture
                    && receipt.binding.content_id == source.content_id
                {
                    assert_eq!(
                        receipt.binding.release_build_id,
                        fixtures.manifest.release_build_id
                    );
                    assert_eq!(
                        receipt.binding.target,
                        managed_binding_status(pic, root, *target)
                    );
                    assert_eq!(
                        receipt.completion_summary,
                        source.descriptor.completion_summary
                    );
                    completed.push(*target);
                }
            }
            assert!(
                !completed.is_empty(),
                "generated fixture must reach its actual target"
            );
        }
    }

    #[cfg(test)]
    #[derive(CandidType)]
    enum PublicMemoryRequest {
        Health,
        Metrics(canic::dto::public_status::PublicMetricsRequest),
    }

    #[cfg(test)]
    #[derive(CandidType, Deserialize)]
    enum PublicMemoryResponse {
        Health(canic::dto::public_status::PublicHealth),
        Metrics(canic::dto::public_status::PublicMetricsSnapshot),
    }

    #[cfg(test)]
    fn public_memory_snapshot(
        pic: &PocketIc,
        target: Principal,
    ) -> canic::dto::public_status::PublicMetricsSnapshot {
        let response: Result<PublicMemoryResponse, Error> = pic
            .query_candid_as(
                target,
                Principal::anonymous(),
                canic::protocol::CANIC_PUBLIC_STATUS,
                (PublicMemoryRequest::Metrics(
                    canic::dto::public_status::PublicMetricsRequest {
                        family: canic::dto::public_status::PublicMetricFamily::Performance,
                        page: canic::dto::page::PageRequest {
                            offset: 0,
                            limit: 256,
                        },
                    },
                ),),
            )
            .expect("anonymous public memory transport");
        let PublicMemoryResponse::Metrics(snapshot) = response.expect("public memory cache") else {
            panic!("metrics response");
        };
        snapshot
    }

    #[cfg(test)]
    fn assert_public_memory_allocation_samples(
        pic: &PocketIc,
        root: Principal,
        store: Principal,
        operator: Principal,
        pools: &[Principal],
    ) {
        let targets = [root, store]
            .into_iter()
            .chain(pools.iter().copied().filter(|target| {
                pic.canister_status(*target, Some(root))
                    .unwrap()
                    .module_hash
                    .is_some()
            }))
            .collect::<Vec<_>>();
        pic.advance_time(std::time::Duration::from_secs(301));
        for _ in 0..120 {
            pic.tick();
            if targets.iter().all(|target| {
                public_memory_snapshot(pic, *target)
                    .metrics
                    .entries
                    .iter()
                    .any(|row| row.name == "memory.allocations.state")
            }) {
                break;
            }
        }
        let mut roles = BTreeSet::new();
        for target in targets {
            roles.insert(assert_public_memory_target(
                pic, target, root, store, operator,
            ));
        }
        for required in ["root", "user_hub", "user_shard"] {
            assert!(
                roles.contains(required),
                "required public-memory role {required}"
            );
        }
    }

    #[cfg(test)]
    fn assert_public_memory_target(
        pic: &PocketIc,
        target: Principal,
        root: Principal,
        store: Principal,
        operator: Principal,
    ) -> String {
        let health: Result<PublicMemoryResponse, Error> = pic
            .query_candid_as(
                target,
                Principal::anonymous(),
                canic::protocol::CANIC_PUBLIC_STATUS,
                (PublicMemoryRequest::Health,),
            )
            .unwrap();
        let PublicMemoryResponse::Health(health) = health.unwrap() else {
            panic!("public health");
        };
        let role = health.role.unwrap();
        let before = wasm_hash(&pic.get_stable_memory(target));
        let snapshot = public_memory_snapshot(pic, target);
        assert_eq!(wasm_hash(&pic.get_stable_memory(target)), before);
        assert_eq!(
            snapshot.state,
            canic::dto::public_status::PublicSnapshotState::Fresh
        );
        let value = |suffix: &str| {
            snapshot
                .metrics
                .entries
                .iter()
                .find(|row| row.name == format!("memory.allocations.{suffix}"))
                .expect("allocation gauge")
                .value
        };
        if target == store {
            assert_eq!(value("state"), 2);
            assert!(
                !snapshot
                    .metrics
                    .entries
                    .iter()
                    .any(|row| row.name == "memory.allocations.physical_extent")
            );
            return role;
        }
        assert_eq!(value("state"), 1);
        assert_eq!(value("ids_measured"), value("ids_total"));
        assert_eq!(value("payload_available"), 0);
        assert_eq!(
            value("physical_extent"),
            value("manager_metadata") + value("allocated_bucket_bytes") + value("unmanaged")
        );
        assert_eq!(
            value("allocated_bucket_bytes"),
            value("virtual_extent") + value("bucket_slack")
        );
        assert_eq!(
            value("allocated_bucket_bytes"),
            value("known_binding") + value("unknown_binding")
        );
        assert!(
            snapshot
                .metrics
                .entries
                .iter()
                .filter(|row| row.name.starts_with("memory.allocations."))
                .all(|row| row.canister_id.is_none() && row.observed_at_ns > 0)
        );
        let denied: Result<CanisterObservabilityResponse, Error> = pic
            .query_candid_as(
                target,
                Principal::anonymous(),
                canic::protocol::CANIC_OBSERVABILITY,
                (CanisterObservabilityRequest::MemoryAllocations,),
            )
            .unwrap();
        assert!(controller_authority_unavailable(&denied));
        let controller = if target == root { operator } else { root };
        let diagnostic: Result<CanisterObservabilityResponse, Error> = pic
            .query_candid_as(
                target,
                controller,
                canic::protocol::CANIC_OBSERVABILITY,
                (CanisterObservabilityRequest::MemoryAllocations,),
            )
            .unwrap();
        let CanisterObservabilityResponse::MemoryAllocations(report) = diagnostic.unwrap() else {
            panic!("allocation diagnostic");
        };
        assert_eq!(value("bucket_size"), u128::from(report.bucket_size_bytes));
        assert_eq!(
            value("physical_extent"),
            u128::from(report.physical_extent.bytes)
        );
        println!(
            "CANIC164 role={} physical_bytes={} measured_ids={} sample_ns={}",
            role,
            value("physical_extent"),
            value("ids_measured"),
            snapshot.sampled_at_ns.unwrap()
        );
        role
    }

    #[cfg(test)]
    struct ReinstallJourney<'a> {
        adapter_root: &'a Path,
        config: &'a Path,
        icp_wrapper: &'a Path,
        local_replica: &'a LocalReplicaTarget,
        pic: &'a PocketIc,
        desired: &'a DesiredFleet,
        coordinator: Principal,
        root: Principal,
        store: Principal,
        pools: &'a [Principal],
    }

    /// Passive application row returned by both fixture roles.
    #[cfg(test)]
    #[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
    struct ReinstallUserRow {
        id: u64,
        value: u64,
    }

    #[cfg(test)]
    fn plan_reinstall_through_cli(
        input: &ReinstallJourney<'_>,
    ) -> canic_host::fleet_ensure::FleetEnsureReport {
        use std::ffi::OsString;
        use std::os::unix::fs::PermissionsExt as _;
        let root = input.adapter_root;
        std::fs::create_dir_all(root.join("apps/test")).unwrap();
        std::fs::copy(input.config, root.join("apps/test/canic.toml")).unwrap();
        std::fs::write(root.join("icp.yaml"), "canisters: []\n").unwrap();
        let desired_path = root.join("reinstall-desired.toml");
        std::fs::write(
            &desired_path,
            toml::to_string_pretty(input.desired).unwrap(),
        )
        .unwrap();
        let wrapper = root.join("reinstall-cli-icp");
        let network = serde_json::json!({"api_url": input.local_replica.url, "root_key": input.local_replica.root_key}).to_string();
        std::fs::write(
            &wrapper,
            format!(
                r#"#!/bin/bash
set -euo pipefail
case " $* " in
  *" network status "*) printf '%s\n' '{network}'; exit 0 ;;
  *" canister "*|*" cycles "*)
    unset ICP_ENVIRONMENT
    args=()
    while (( $# )); do
      case "$1" in
        -e|--environment) shift 2 ;;
        *) args+=("$1"); shift ;;
      esac
    done
    exec '{}' "${{args[@]}}" -n '{}' -k '{}' ;;
  *) exec '{}' "$@" ;;
esac
"#,
                input.icp_wrapper.display(),
                input.local_replica.url,
                input.local_replica.root_key,
                input.icp_wrapper.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(root).unwrap();
        let result = canic_cli::run([
            OsString::from("--environment"),
            OsString::from("local"),
            OsString::from("--icp"),
            wrapper.into_os_string(),
            OsString::from("fleet"),
            OsString::from("ensure"),
            OsString::from(&input.desired.fleet),
            OsString::from("--desired"),
            desired_path.into_os_string(),
            OsString::from("--reinstall"),
        ]);
        std::env::set_current_dir(previous).unwrap();
        result.expect("the production CLI plans the deliberate wipe through real ICP observations");
        let paths = canic_host::fleet_ensure::ops::EnsurePaths::under(
            root,
            &input.desired.environment,
            &input.desired.fleet,
        );
        let plan = canic_host::fleet_ensure::ops::read_plan(&paths)
            .unwrap()
            .unwrap();
        canic_host::fleet_ensure::FleetEnsureReport {
            actual_conservation: None,
            funding_review: None,
            effects_applied: 0,
            plan,
            terminal: false,
        }
    }

    #[cfg(test)]
    fn selected_reinstall_artifacts(input: &ReinstallJourney<'_>) -> LiteralZeroReleaseArtifacts {
        let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config = AppConfigSnapshot::load(input.config).unwrap();
        let configuration = config
            .model()
            .compile_component_deployment_configuration()
            .unwrap();
        let roles = config
            .model()
            .roles
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        build_literal_zero_release_artifacts(
            &workspace,
            input.adapter_root,
            input.config,
            &configuration,
            &roles,
            BuildNetwork::Local,
            REINSTALL_RELEASE_BUILD_NONCE,
        )
    }

    #[cfg(test)]
    fn select_reinstall_build(
        source: &DesiredFleet,
        artifacts: &LiteralZeroReleaseArtifacts,
    ) -> DesiredFleet {
        let mut desired = source.clone();
        desired.bootstrap.as_mut().unwrap().release_build_id = artifacts.release_build_id;
        let protocol = desired.protocol.as_mut().unwrap();
        protocol
            .coordinator_candid
            .clone_from(&artifacts.coordinator_candid);
        protocol.root_candid.clone_from(&artifacts.root_candid);
        protocol.store_candid.clone_from(&artifacts.store_candid);
        for canister in &mut desired.canisters {
            let wasm = match canister.kind {
                DesiredCanisterKind::Coordinator => &artifacts.coordinator_wasm,
                DesiredCanisterKind::Root => &artifacts.root_wasm,
                DesiredCanisterKind::Store => &artifacts.store_wasm,
                _ => continue,
            };
            canister.wasm = Some(wasm.clone());
        }
        desired
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one pre-reset funding proof keeps durable intent, seal removal and lost payment response together"
    )]
    fn assert_sealed_reinstall_funding_retry(
        input: &ReinstallJourney<'_>,
        reset: &canic_host::fleet_ensure::model::FleetEnsurePlan,
    ) -> u64 {
        let root = input.adapter_root;
        let desired = input.desired;
        let operator = Principal::from_text(&desired.operator).unwrap();
        let ledger = Principal::from_text(&desired.cycles_ledger).unwrap();
        let credits = planned_actions(reset)
            .into_iter()
            .filter(|action| {
                matches!(
                    action,
                    EnsureAction::Fund { .. } | EnsureAction::FundEstate { .. }
                )
            })
            .collect::<Vec<_>>();
        let [
            EnsureAction::Fund {
                principal, amount, ..
            },
        ] = credits.as_slice()
        else {
            panic!("exactly one reviewed native Root credit is required");
        };
        assert_eq!(principal, &input.root.to_text());
        let credited = *amount;
        let withdrawals = || {
            input
                .pic
                .query_candid::<u64, _>(ledger, "withdrawal_count", ())
                .unwrap()
        };
        let withdrawals_before = withdrawals();
        let operator_before = ledger_account_balance(input.pic, ledger, operator);
        #[expect(
            clippy::result_large_err,
            reason = "qualification asserts the public workflow error"
        )]
        let apply = || {
            fleet_ensure_workflow::apply(
                root,
                desired,
                &desired_sha256(desired),
                &desired.fleet,
                &reset.plan_sha256,
                &mut literal_zero_journey_platform(
                    desired,
                    input.icp_wrapper,
                    root,
                    input.local_replica.clone(),
                    true,
                ),
            )
        };
        std::fs::write(root.join("fail-before-funding"), []).unwrap();
        assert!(matches!(apply(), Err(EnsureWorkflowError::Platform(_))));
        assert!(root.join("failed-before-funding").is_file());
        let paths = canic_host::fleet_ensure::ops::EnsurePaths::under(
            root,
            &desired.environment,
            &desired.fleet,
        );
        let retained = canic_host::fleet_ensure::ops::read_journal(&paths)
            .unwrap()
            .unwrap();
        assert_eq!(retained.effects.len(), 1);
        assert_eq!(
            retained.effects[0].state,
            canic_host::fleet_ensure::model::EffectState::Intent
        );
        assert_eq!(
            retained.effects[0].action_sha256,
            canic_host::fleet_ensure::ops::action_sha256(credits[0])
        );
        let request = AuthoritySnapshotRequest {
            operation_id: canic_core::cdk::utils::hash::decode_hex(&reset.operation_id)
                .unwrap()
                .try_into()
                .unwrap(),
        };
        let RootCommandResponseFragment::ResumeAuthoritySnapshot(open) = root_command_as(
            input.pic,
            input.root,
            operator,
            RootCommandFragment::ResumeAuthoritySnapshot(request),
        )
        .unwrap() else {
            panic!("exact resume response")
        };
        assert_eq!(open.phase, AuthorityRestoreFencePhase::Open);
        assert!(matches!(
            apply(),
            Err(EnsureWorkflowError::DriftedBeforeApply)
        ));
        assert_eq!(withdrawals(), withdrawals_before);
        assert_eq!(
            ledger_account_balance(input.pic, ledger, operator),
            operator_before
        );
        assert!(!root.join("reinstall-mutations.log").exists());
        let RootCommandResponseFragment::PrepareAuthoritySnapshot(sealed) = root_command_as(
            input.pic,
            input.root,
            operator,
            RootCommandFragment::PrepareAuthoritySnapshot(request),
        )
        .unwrap() else {
            panic!("exact seal response")
        };
        assert_eq!(sealed.phase, AuthorityRestoreFencePhase::Sealed);
        assert_eq!(sealed.operation_id, Some(request.operation_id));
        std::fs::write(root.join("lose-funding-response"), []).unwrap();
        let lost = root.join("lost-funding-response");
        if lost.exists() {
            std::fs::remove_file(&lost).unwrap();
        }
        let root_before = input.pic.cycle_balance(input.root);
        assert!(matches!(apply(), Err(EnsureWorkflowError::Platform(_))));
        assert!(lost.is_file());
        assert_eq!(withdrawals(), withdrawals_before + 1);
        assert_eq!(
            ledger_account_balance(input.pic, ledger, operator),
            operator_before - Nat::from(reset.conservation.maximum_operator_debit_cycles)
        );
        assert!(
            input.pic.cycle_balance(input.root) + reset.conservation.maximum_execution_burn_cycles
                >= root_before + credited
        );
        let child = selected_fixture_targets_from_pool(
            input.pic,
            input.root,
            &root_pool_status_as(input.pic, input.root, operator),
        )
        .into_iter()
        .find(|(_, role)| role.as_str() == "user_hub")
        .unwrap()
        .0;
        let child_before = input.pic.cycle_balance(child);
        assert_eq!(
            application_rejection(
                root_command_as(
                    input.pic,
                    input.root,
                    child,
                    RootCommandFragment::RespondCapability(descendant_funding_request(
                        input.pic, 0xba
                    ))
                ),
                "funded sealed Root rejects child grants"
            )
            .code(),
            canic_core::diagnostics::codes::AUTHORITY_INACTIVE.raw_code()
        );
        assert!(input.pic.cycle_balance(child) <= child_before);
        withdrawals_before + 1
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one recovery proof preserves applied installs across funding review, lost reply and terminal replay"
    )]
    fn complete_selected_reinstall(
        input: &ReinstallJourney<'_>,
        reset: &canic_host::fleet_ensure::model::FleetEnsurePlan,
        underfunded: bool,
    ) -> (
        canic_host::fleet_ensure::model::FleetEnsureReport,
        u128,
        u128,
    ) {
        use canic_host::fleet_ensure::model::{EffectState, FleetEnsureSuccessorReviewReason};
        let root = input.adapter_root;
        let desired = input.desired;
        let digest = desired_sha256(desired);
        let platform = || {
            literal_zero_journey_platform(
                desired,
                input.icp_wrapper,
                root,
                input.local_replica.clone(),
                true,
            )
        };
        #[expect(
            clippy::result_large_err,
            reason = "qualification asserts the existing public workflow error variants"
        )]
        let apply = |plan: &canic_host::fleet_ensure::model::FleetEnsurePlan| {
            // Child initialization can outlive one host observation window.
            // Resume the same reviewed plan; other failures still fail immediately.
            for attempt in 0..=64 {
                let result = fleet_ensure_workflow::apply(
                    root,
                    desired,
                    &digest,
                    &desired.fleet,
                    &plan.plan_sha256,
                    &mut platform(),
                );
                if attempt == 64
                    || !matches!(
                        result,
                        Err(EnsureWorkflowError::ProvisioningRetryPending { .. })
                    )
                {
                    return result;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
            unreachable!("the last attempt returns its exact result")
        };
        let first = apply(reset);
        if !underfunded {
            return (
                first.expect("complete the funded repeat wipe"),
                reset.conservation.maximum_operator_debit_cycles,
                0,
            );
        }
        assert!(
            matches!(first, Err(EnsureWorkflowError::SuccessorReviewRequired {
            reason: FleetEnsureSuccessorReviewReason::AdditionalEffect, review: Some(ref details),
        }) if details.maximum_additional_debit_cycles > 0),
            "exact additional funding review: {first:?}"
        );
        let paths = canic_host::fleet_ensure::ops::EnsurePaths::under(
            root,
            &desired.environment,
            &desired.fleet,
        );
        let retained = canic_host::fleet_ensure::ops::read_journal(&paths)
            .unwrap()
            .unwrap();
        assert_eq!(retained.operation_id, reset.operation_id);
        assert_eq!(retained.effects.len(), planned_actions(reset).len());
        assert!(
            retained
                .effects
                .iter()
                .all(|effect| effect.state == EffectState::Applied)
        );
        assert!(retained.successor_phases.is_empty());
        let original_plan = std::fs::read(&paths.plan).unwrap();
        let installs = std::fs::read_to_string(root.join("reinstall-mutations.log")).unwrap();
        assert_eq!(installs.lines().count(), 3);
        let ledger = Principal::from_text(&desired.cycles_ledger).unwrap();
        let operator = Principal::from_text(&desired.operator).unwrap();
        let balance = ledger_account_balance(input.pic, ledger, operator);
        let withdrawals_before: u64 = input
            .pic
            .query_candid(ledger, "withdrawal_count", ())
            .unwrap();
        let retry = apply(reset);
        assert!(
            matches!(
                retry,
                Err(EnsureWorkflowError::SuccessorReviewRequired { .. })
            ),
            "paused reset requires a new review without repeating effects: {retry:?}"
        );
        let replayed = canic_host::fleet_ensure::ops::read_journal(&paths)
            .unwrap()
            .unwrap();
        assert_eq!(
            replayed.effects, retained.effects,
            "all three exact install receipts survive retry"
        );
        assert_eq!(std::fs::read(&paths.plan).unwrap(), original_plan);
        assert_eq!(ledger_account_balance(input.pic, ledger, operator), balance);
        let mut debit = 0;
        let mut burn = 0;
        let mut expected_withdrawals = withdrawals_before;
        for _ in 0..8 {
            let reviewed = fleet_ensure_workflow::plan(
                root,
                desired,
                &digest,
                &desired.fleet,
                1_800_000_000_000_000_135,
                &mut platform(),
            )
            .expect("review recovery under the same reinstall operation");
            assert_eq!(reviewed.plan.operation_id, reset.operation_id);
            let actions = planned_actions(&reviewed.plan);
            assert!(actions.iter().all(|action| !matches!(
                action,
                EnsureAction::Install { .. } | EnsureAction::Create { .. }
            )));
            let funds = actions
                .iter()
                .filter_map(|action| match action {
                    EnsureAction::Fund {
                        principal,
                        pool_funding,
                        ..
                    } => {
                        let authority = pool_funding.as_ref().expect("reconciliation owns funding");
                        assert_eq!(authority.root, input.root.to_text());
                        assert_eq!(
                            authority.lifecycle,
                            canic_host::fleet_ensure::model::EstatePoolAssetLifecycle::PendingReset
                        );
                        Some(principal)
                    }
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            assert!(
                funds.len() <= 1,
                "only the intentionally depleted Hub needs funding"
            );
            debit += reviewed.plan.conservation.maximum_operator_debit_cycles;
            burn += reviewed.plan.conservation.maximum_execution_burn_cycles;
            expected_withdrawals += u64::try_from(funds.len()).unwrap();
            if !funds.is_empty() {
                let lost = root.join("lost-funding-response");
                if lost.exists() {
                    std::fs::remove_file(lost).unwrap();
                }
                std::fs::write(root.join("lose-funding-response"), []).unwrap();
            }
            let mut result = apply(&reviewed.plan);
            if !funds.is_empty() {
                assert!(
                    matches!(result, Err(EnsureWorkflowError::Platform(_))),
                    "lose the completed withdrawal response: {result:?}"
                );
                assert!(root.join("lost-funding-response").exists());
                result = apply(&reviewed.plan);
            }
            assert_eq!(
                std::fs::read_to_string(root.join("reinstall-mutations.log")).unwrap(),
                installs
            );
            let withdrawals: u64 = input
                .pic
                .query_candid(ledger, "withdrawal_count", ())
                .unwrap();
            assert_eq!(
                withdrawals, expected_withdrawals,
                "retry cannot repeat a paid withdrawal"
            );
            match result {
                Ok(complete) => {
                    assert!(debit > 0);
                    assert_eq!(
                        ledger_account_balance(input.pic, ledger, operator),
                        balance - Nat::from(debit)
                    );
                    return (
                        complete,
                        reset.conservation.maximum_operator_debit_cycles + debit,
                        burn,
                    );
                }
                Err(EnsureWorkflowError::SuccessorReviewRequired {
                    review: Some(details),
                    ..
                }) => assert!(!details.actions.is_empty()),
                Err(error) => panic!("reviewed recovery must converge: {error:?}"),
            }
        }
        panic!("bounded same-operation recovery reviews exhausted");
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one production-adapter journey proves stable-row wiping, interruption recovery, conservation and deliberate repeat identity"
    )]
    fn assert_selected_build_reinstall_journey(input: ReinstallJourney<'_>) {
        use canic_host::fleet_ensure::model::FleetEnsurePlanScope;
        let root = input.adapter_root;
        let source_desired = input.desired.clone();
        let replacement = selected_reinstall_artifacts(&input);
        let mut selected = select_reinstall_build(input.desired, &replacement);
        let root_policy = selected
            .canisters
            .iter_mut()
            .find(|canister| canister.kind == DesiredCanisterKind::Root)
            .unwrap();
        root_policy.minimum_cycles =
            (input.pic.cycle_balance(input.root) + 1_000_000_000_000).to_string();
        root_policy
            .initial_cycles
            .clone_from(&root_policy.minimum_cycles);
        assert_ne!(
            selected.bootstrap.as_ref().unwrap().release_build_id,
            input.desired.bootstrap.as_ref().unwrap().release_build_id
        );
        let input = ReinstallJourney {
            desired: &selected,
            ..input
        };
        let selected_hashes =
            canic_host::fleet_ensure::ops::resolve_desired_artifacts(root, &selected)
                .expect("selected artifact identities")
                .wasm_sha256_by_canister;
        let source_hashes =
            canic_host::fleet_ensure::ops::resolve_desired_artifacts(root, &source_desired)
                .expect("source artifact identities")
                .wasm_sha256_by_canister;
        for (name, hash) in &selected_hashes {
            assert_ne!(
                Some(hash),
                source_hashes.get(name),
                "distinct infrastructure build"
            );
        }
        let bootstrap = selected.bootstrap.as_ref().unwrap();
        let application_hashes =
            canic_host::release_set::load_persisted_application_artifact_union(
                root,
                &bootstrap
                    .component_deployment_configuration
                    .component_topology,
                bootstrap.release_build_id,
            )
            .expect("selected application release manifest")
            .union
            .entries
            .into_iter()
            .map(|artifact| artifact.wasm_gz_sha256_hex)
            .collect::<BTreeSet<_>>();
        let desired = input.desired;
        let digest = desired_sha256(desired);
        let operator = Principal::from_text(&desired.operator).unwrap();
        let ledger = Principal::from_text(&desired.cycles_ledger).unwrap();
        let platform = || {
            literal_zero_journey_platform(
                desired,
                input.icp_wrapper,
                root,
                input.local_replica.clone(),
                true,
            )
        };
        let all = [input.coordinator, input.root, input.store]
            .into_iter()
            .chain(input.pools.iter().copied())
            .collect::<Vec<_>>();
        let rows = |id| {
            input
                .pic
                .query_candid::<Result<Vec<ReinstallUserRow>, Error>, _>(id, "test_user_rows", ())
                .ok()
                .and_then(Result::ok)
        };
        let before_operator = ledger_account_balance(input.pic, ledger, operator);
        let before_root = ledger_account_balance(input.pic, ledger, input.root);
        let mut previous_operation = None;
        let mut cumulative_funding = 0_u128;
        for wipe in 0..2_u64 {
            let wipe_span = Span::start(if wipe == 0 {
                "first_deliberate_wipe"
            } else {
                "second_deliberate_wipe"
            });
            let mut phase = Span::start("seed_and_preparation_review");
            let applications = input
                .pools
                .iter()
                .copied()
                .filter(|id| rows(*id).is_some())
                .collect::<Vec<_>>();
            assert_eq!(
                applications.len(),
                2,
                "real application rows in both Hub and Shard"
            );
            for id in applications {
                for (key, value) in [(0_u64, 99_u64), (42 + wipe, 1001)] {
                    let result: Result<(), Error> = input
                        .pic
                        .update_candid(id, "test_set_user_row", (key, value))
                        .unwrap();
                    result.unwrap();
                }
                assert_eq!(
                    rows(id).unwrap(),
                    vec![
                        ReinstallUserRow { id: 0, value: 99 },
                        ReinstallUserRow {
                            id: 42 + wipe,
                            value: 1001
                        }
                    ]
                );
            }
            if wipe == 0 {
                let hub = selected_fixture_targets_from_pool(
                    input.pic,
                    input.root,
                    &root_pool_status_as(input.pic, input.root, operator),
                )
                .into_iter()
                .find(|(_, role)| role.as_str() == "user_hub")
                .unwrap()
                .0;
                let burned: Result<u128, Error> = input
                    .pic
                    .update_candid_as(
                        hub,
                        input.root,
                        "test_recovery_balance",
                        (323_400_000_000_u128,),
                    )
                    .expect("spend fixture cycles through its local controller endpoint");
                assert!(burned.unwrap() > 0);
                assert!(
                    input.pic.cycle_balance(hub)
                        < bootstrap.roots[0]
                            .limits
                            .canister_pool
                            .canister_cycles
                            .to_u128()
                );
            }
            let native_before = all
                .iter()
                .map(|id| input.pic.cycle_balance(*id))
                .sum::<u128>();
            let mut first = platform();
            let preparation = if wipe == 0 {
                plan_reinstall_through_cli(&input)
            } else {
                fleet_ensure_workflow::plan_reinstall(
                    root,
                    desired,
                    &digest,
                    &desired.fleet,
                    1_800_000_000_000_000_100 + wipe,
                    &mut first,
                )
                .expect("review identical selected-build wipe preparation")
            };
            assert_eq!(
                preparation.plan.scope,
                FleetEnsurePlanScope::ReinstallPreparation
            );
            assert_eq!(preparation.plan.desired_sha256, digest);
            assert_eq!(
                preparation
                    .plan
                    .reviewed_desired
                    .as_ref()
                    .unwrap()
                    .desired()
                    .bootstrap
                    .as_ref()
                    .unwrap()
                    .release_build_id,
                desired.bootstrap.as_ref().unwrap().release_build_id
            );
            assert_ne!(
                previous_operation.as_ref(),
                Some(&preparation.plan.operation_id)
            );
            previous_operation = Some(preparation.plan.operation_id.clone());
            assert!(matches!(
                fleet_ensure_workflow::plan_reinstall(
                    root,
                    desired,
                    &digest,
                    &desired.fleet,
                    1_800_000_000_000_000_110 + wipe,
                    &mut first
                ),
                Err(EnsureWorkflowError::ReinstallConflict)
            ));
            phase = phase.next("preparation_seal_and_replay");
            let sealed = fleet_ensure_workflow::apply(
                root,
                desired,
                &digest,
                &desired.fleet,
                &preparation.plan.plan_sha256,
                &mut first,
            )
            .expect("seal the current authorities");
            assert!(sealed.terminal);
            assert!(matches!(
                canic_host::fleet_ensure::resolve_current_fleet(
                    root,
                    &desired.environment,
                    &desired.fleet
                ),
                Err(canic_host::fleet_ensure::CurrentFleetInventoryError::NotConverged { .. })
            ));
            let replay = fleet_ensure_workflow::apply(
                root,
                desired,
                &digest,
                &desired.fleet,
                &preparation.plan.plan_sha256,
                &mut first,
            )
            .expect("replay seal without mutation");
            assert_eq!(replay.effects_applied, 0);
            phase = phase.next("reset_review");
            let reset = fleet_ensure_workflow::plan(
                root,
                desired,
                &digest,
                &desired.fleet,
                1_800_000_000_000_000_120 + wipe,
                &mut first,
            )
            .expect("review complete sealed reset inventory");
            assert_eq!(reset.plan.scope, FleetEnsurePlanScope::Full);
            assert_eq!(reset.plan.operation_id, preparation.plan.operation_id);
            assert_eq!(
                reset.plan.reinstall.as_ref().unwrap().assets.len(),
                input.pools.len()
            );
            let installs = planned_actions(&reset.plan)
                .into_iter()
                .filter(|action| {
                    matches!(
                        action,
                        EnsureAction::Install {
                            mode: canic_host::fleet_ensure::model::InstallMode::Reinstall,
                            ..
                        }
                    )
                })
                .count();
            assert_eq!(installs, 3, "reset exactly Coordinator, Store and Root");
            phase = phase.next("reset_interruptions_and_recovery");
            if wipe == 0 {
                operator_shortfall::assert_fresh_reinstall_rejection(&input, &reset.plan);
                let protected_withdrawals =
                    assert_sealed_reinstall_funding_retry(&input, &reset.plan);
                std::fs::write(root.join("fail-before-install"), []).unwrap();
                std::fs::write(root.join("fail-before-root-install"), input.root.to_text())
                    .unwrap();
                let interrupted = fleet_ensure_workflow::apply(
                    root,
                    desired,
                    &digest,
                    &desired.fleet,
                    &reset.plan.plan_sha256,
                    &mut first,
                );
                assert!(
                    matches!(interrupted, Err(EnsureWorkflowError::Platform(_))),
                    "interrupt before install: {interrupted:?}"
                );
                assert!(!root.join("reinstall-mutations.log").exists());
                assert_eq!(
                    input
                        .pic
                        .query_candid::<u64, _>(ledger, "withdrawal_count", ())
                        .unwrap(),
                    protected_withdrawals,
                    "lost pre-reset credit response must not withdraw again"
                );
                std::fs::write(root.join("lose-install-response"), []).unwrap();
                let interrupted = fleet_ensure_workflow::apply(
                    root,
                    desired,
                    &digest,
                    &desired.fleet,
                    &reset.plan.plan_sha256,
                    &mut platform(),
                );
                assert!(
                    matches!(interrupted, Err(EnsureWorkflowError::Platform(_))),
                    "interrupt after install: {interrupted:?}"
                );
                assert!(root.join("lost-install-response").is_file());
                let interrupted = fleet_ensure_workflow::apply(
                    root,
                    desired,
                    &digest,
                    &desired.fleet,
                    &reset.plan.plan_sha256,
                    &mut platform(),
                );
                assert!(
                    matches!(interrupted, Err(EnsureWorkflowError::Platform(_))),
                    "interrupt before Root reinstall: {interrupted:?}"
                );
                assert!(root.join("failed-before-root-install").is_file());
                assert_eq!(
                    std::fs::read_to_string(root.join("reinstall-mutations.log"))
                        .unwrap()
                        .lines()
                        .count(),
                    2
                );
                // Root inspections advance canister_version without deploying code.
                let inspection = encode_one(RootCommandFragment::InspectCanister(
                    CanisterInspectionRequest {
                        canister_id: input.root,
                    },
                ))
                .unwrap();
                let inspected = input
                    .pic
                    .update_call(
                        input.root,
                        operator,
                        canic::protocol::CANIC_ROOT_COMMAND,
                        inspection,
                    )
                    .unwrap();
                let inspected =
                    decode_one::<Result<RootCommandResponseFragment, Error>>(&inspected)
                        .unwrap()
                        .unwrap();
                assert!(matches!(
                    inspected,
                    RootCommandResponseFragment::InspectCanister(_)
                ));
                let replacement = fleet_ensure_workflow::plan_reinstall(
                    root,
                    &source_desired,
                    &desired_sha256(&source_desired),
                    &desired.fleet,
                    1_800_000_000_000_000_130,
                    &mut platform(),
                );
                assert!(
                    matches!(
                        &replacement,
                        Err(EnsureWorkflowError::RetainedOperationRecoveryRequired {
                            operation_id, plan_sha256,
                        }) if operation_id == &reset.plan.operation_id
                            && plan_sha256 == &reset.plan.plan_sha256
                    ),
                    "replacement must identify the interrupted reset: {replacement:?}"
                );
                std::fs::remove_file(root.join("lost-install-response")).unwrap();
                let interrupted = fleet_ensure_workflow::apply(
                    root,
                    &source_desired,
                    &desired_sha256(&source_desired),
                    &desired.fleet,
                    &reset.plan.plan_sha256,
                    &mut platform(),
                );
                assert!(
                    matches!(interrupted, Err(EnsureWorkflowError::Platform(_))),
                    "lose changed Root response while workspace selects another build: {interrupted:?}"
                );
                assert!(root.join("lost-install-response").is_file());
                assert_eq!(
                    std::fs::read_to_string(root.join("reinstall-mutations.log"))
                        .unwrap()
                        .lines()
                        .count(),
                    3
                );
            }
            let (complete, funding, additional_burn) =
                complete_selected_reinstall(&input, &reset.plan, wipe == 0);
            cumulative_funding += funding;
            assert!(complete.terminal);
            assert!(complete.actual_conservation.is_some());
            phase = phase.next("reset_state_and_conservation");
            for binding in &reset.plan.reinstall.as_ref().unwrap().authorities {
                let id = Principal::from_text(&binding.principal).unwrap();
                let status = input.pic.canister_status(id, Some(operator)).unwrap();
                assert_eq!(
                    status
                        .module_hash
                        .map(|hash| canic_core::cdk::utils::hash::hex_bytes(&hash))
                        .as_ref(),
                    selected_hashes.get(&binding.name)
                );
            }
            for id in input.pools {
                let status = input.pic.canister_status(*id, Some(input.root)).unwrap();
                if let Some(hash) = status.module_hash {
                    assert!(
                        application_hashes
                            .contains(&canic_core::cdk::utils::hash::hex_bytes(&hash)),
                        "selected application Wasm installed"
                    );
                }
            }
            let final_rows = input
                .pools
                .iter()
                .filter_map(|id| rows(*id))
                .collect::<Vec<_>>();
            assert_eq!(
                final_rows,
                vec![
                    vec![ReinstallUserRow { id: 0, value: 7 }],
                    vec![ReinstallUserRow { id: 0, value: 7 }]
                ],
                "all user rows disappear and authored system rows return"
            );
            let pool = root_pool_status_as(input.pic, input.root, operator);
            assert_eq!((pool.workload, pool.ready, pool.pending_reset), (5, 1, 0));
            assert!(pool.pending_creation.is_none());
            let native_after = all
                .iter()
                .map(|id| input.pic.cycle_balance(*id))
                .sum::<u128>();
            assert!(native_after <= native_before + funding);
            assert!(
                native_before + funding - native_after
                    <= preparation.plan.conservation.maximum_execution_burn_cycles
                        + reset.plan.conservation.maximum_execution_burn_cycles
                        + additional_burn
            );
            assert_eq!(
                ledger_account_balance(input.pic, ledger, operator),
                before_operator.clone() - Nat::from(cumulative_funding)
            );
            assert_eq!(
                ledger_account_balance(input.pic, ledger, input.root),
                before_root
            );
            let mutations = std::fs::read_to_string(root.join("reinstall-mutations.log")).unwrap();
            assert_eq!(
                mutations.lines().count(),
                3 * usize::try_from(wipe + 1).unwrap(),
                "lost changed or identical Wasm response never repeats an install"
            );
            phase = phase.next("reset_terminal_replay");
            // A newly reviewed funding plan uses its own selected input. A reset replay
            // still recovers its retained selection despite a changed workspace build.
            let replay_desired = if complete.plan.reinstall.is_some() {
                &source_desired
            } else {
                desired
            };
            let replay = fleet_ensure_workflow::apply(
                root,
                replay_desired,
                &desired_sha256(replay_desired),
                &desired.fleet,
                &complete.plan.plan_sha256,
                &mut platform(),
            )
            .expect("effect-free wipe replay");
            assert_eq!(replay.effects_applied, 0);
            assert_eq!(
                std::fs::read_to_string(root.join("reinstall-mutations.log")).unwrap(),
                mutations
            );
            let ordinary = fleet_ensure_workflow::plan(
                root,
                desired,
                &digest,
                &desired.fleet,
                1_800_000_000_000_000_140 + wipe,
                &mut platform(),
            )
            .expect("ordinary ensure after wipe");
            assert!(planned_actions(&ordinary.plan).is_empty());
            fleet_ensure_workflow::apply(
                root,
                desired,
                &digest,
                &desired.fleet,
                &ordinary.plan.plan_sha256,
                &mut platform(),
            )
            .expect("retain ordinary no-op convergence");
            phase.finish();
            wipe_span.finish();
        }
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one real release transition retains reset, recovery, conservation and both replay boundaries"
    )]
    fn assert_generated_reinstall_journey(input: ReinstallJourney<'_>) {
        let mut phase = Span::start("replacement_artifacts_and_generation");
        let root = input.adapter_root;
        let pic = input.pic;
        let operator = Principal::from_text(&input.desired.operator).unwrap();
        let ledger = Principal::from_text(&input.desired.cycles_ledger).unwrap();
        let transfer: Result<Nat, QualificationIcrc1TransferError> = pic
            .update_candid_as(
                ledger,
                operator,
                "icrc1_transfer",
                (QualificationIcrc1TransferArg {
                    from_subaccount: None,
                    to: QualificationIcrc1Account {
                        owner: input.root,
                        subaccount: None,
                    },
                    fee: Some(Nat::from(0_u8)),
                    created_at_time: None,
                    memo: None,
                    amount: Nat::from(1_000_000_000_u128),
                },),
            )
            .expect("fund the retained Root Ledger account");
        transfer.expect("fixture Ledger funding");
        let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config = AppConfigSnapshot::load(input.config).unwrap();
        let configuration = config
            .model()
            .compile_component_deployment_configuration()
            .unwrap();
        let roles = config
            .model()
            .roles
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        // This fixture has two fixed, distinct releases. Cache each complete sealed
        // artifact set by its own identity and exact build inputs across test runs.
        let replacement = build_literal_zero_release_artifacts(
            &workspace,
            root,
            input.config,
            &configuration,
            &roles,
            BuildNetwork::Local,
            REINSTALL_RELEASE_BUILD_NONCE,
        )
        .release_build_id;
        assert_ne!(
            replacement,
            input.desired.bootstrap.as_ref().unwrap().release_build_id
        );
        let config_path = retain_generated_journey_source(root, input.config);
        let trust = root.join("reinstall-root-key.der");
        let root_key = pic.root_key().unwrap();
        std::fs::write(&trust, &root_key).unwrap();
        canic_host::network::enroll_network(canic_host::network::NetworkEnrollmentOptions {
            workspace_root: root,
            environment: "local",
            root_key: &trust,
            fingerprint: &canic_core::cdk::utils::hash::sha256_hex(&root_key),
        })
        .expect("enroll the exact reinstall network");
        let source = root.join("fleet-policy.toml");
        let seed = root.join("fleet-seed.toml");
        let generator = root.join("reinstall-generator-icp");
        std::fs::write(
            &generator,
            format!(
                r#"#!/bin/bash
set -euo pipefail
args=()
canister=false
for value in "$@"; do
  if [[ "$value" == canister ]]; then canister=true; fi
done
if "$canister"; then
  unset ICP_ENVIRONMENT
  while (( $# )); do
    case "$1" in
      -e|--environment) shift 2 ;;
      *) args+=("$1"); shift ;;
    esac
  done
  exec '{}' "${{args[@]}}" -n '{}' -k '{}'
fi
exec '{}' "$@"
"#,
                input.icp_wrapper.display(),
                input.local_replica.url,
                input.local_replica.root_key,
                input.icp_wrapper.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&generator, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let old_paths =
            canic_host::fleet_ensure::ops::EnsurePaths::under(root, "local", &input.desired.fleet);
        assert_eq!(
            canic_host::fleet_ensure::ops::read_journal(&old_paths)
                .unwrap()
                .unwrap()
                .completion,
            canic_host::fleet_ensure::model::FleetEnsureCompletion::Converged
        );
        let retained_state =
            canic_host::fleet_ensure::ops::read_state(&old_paths, &input.desired.fleet).unwrap();
        assert_eq!(retained_state.principals.len(), 27);
        let old_bootstrap = input.desired.bootstrap.as_ref().unwrap();
        let imports = input
            .pools
            .iter()
            .map(Principal::to_text)
            .collect::<Vec<_>>();
        let terminal_imports = imports
            .iter()
            .filter(|principal| {
                retained_state
                    .principals
                    .values()
                    .any(|known| known == *principal)
            })
            .count();
        assert_eq!(terminal_imports, imports.len());
        // Review the terminal estate through the existing seed and policy authority.
        let mut seed_document: toml::Value =
            toml::from_str(&std::fs::read_to_string(&seed).unwrap()).unwrap();
        seed_document["fresh_estate"] = toml::Value::Boolean(false);
        seed_document["coordinator"] = toml::Value::String(input.coordinator.to_text());
        seed_document["roots"][0]["root"] = toml::Value::String(input.root.to_text());
        seed_document["roots"][0]["store"] = toml::Value::String(input.store.to_text());
        let policy_document: toml::Value =
            toml::from_str(&std::fs::read_to_string(&source).unwrap()).unwrap();
        let write_imports = |selected: &[String]| {
            let mut seed = seed_document.clone();
            let mut policy = policy_document.clone();
            let values =
                toml::Value::Array(selected.iter().cloned().map(toml::Value::String).collect());
            seed["roots"][0]["pool_imports"] = values.clone();
            policy["fleet_subnet_roots"][0]["canister_pool"]
                .as_table_mut()
                .unwrap()
                .insert("imports".into(), values);
            std::fs::write(
                root.join("fleet-seed.toml"),
                toml::to_string(&seed).unwrap(),
            )
            .unwrap();
            std::fs::write(
                root.join("fleet-policy.toml"),
                toml::to_string(&policy).unwrap(),
            )
            .unwrap();
        };
        write_imports(&imports);
        let request = canic_host::fleet_ensure::FleetGenerateRequest {
            catalog_progress: None,
            app_config: &config_path,
            environment: "local",
            fleet: &input.desired.fleet,
            icp_executable: generator.to_str().unwrap(),
            release_build_id: replacement,
            root,
            seed: &seed,
            source: &source,
        };
        let current_request = canic_host::fleet_ensure::FleetGenerateRequest {
            catalog_progress: None,
            release_build_id: old_bootstrap.release_build_id,
            ..request
        };
        let current = canic_host::fleet_ensure::generate_desired_fleet(&current_request)
            .expect("validate all terminal assets against the still-current Root");
        assert_eq!(current.observed_canisters, 27);
        let request = canic_host::fleet_ensure::FleetGenerateRequest {
            catalog_progress: None,
            release_build_id: replacement,
            ..current_request
        };
        write_imports(&imports[..8]);
        let incomplete = canic_host::fleet_ensure::generate_desired_fleet(&request)
            .expect("read incomplete reviewed imports before Root reset");
        let mut stale_platform = literal_zero_journey_platform(
            &incomplete.desired,
            input.icp_wrapper,
            root,
            input.local_replica.clone(),
            true,
        );
        let rejected = fleet_ensure_workflow::plan(
            root,
            &incomplete.desired,
            &desired_sha256(&incomplete.desired),
            &incomplete.desired.fleet,
            1_800_000_000_000_000_009,
            &mut stale_platform,
        );
        assert!(
            matches!(rejected, Err(EnsureWorkflowError::Policy(canic_host::fleet_ensure::policy::EnsurePolicyError::IncompleteRootEstate { missing_principals, .. })) if missing_principals.len() == 16)
        );
        assert!(!root.join("reinstall-mutations.log").exists());
        write_imports(&imports);
        let generated = canic_host::fleet_ensure::generate_desired_fleet(&request)
            .expect("generate from exact management authority before any reset");
        // Coordinator and Store remain observable; the changed Root is deferred.
        assert_eq!(generated.observed_canisters, 2);
        assert!(
            generated
                .desired
                .canisters
                .iter()
                .all(|canister| canister.principal.is_some())
        );
        let mut desired = generated.desired;
        desired.maximum_observation_burn_cycles = "2T".into();
        desired.maximum_update_burn_cycles = "2T".into();
        // The existing local audit endpoint changes only this disposable balance.
        // Ordinary production recovery bounds remain unchanged throughout the journey.
        let burned: u128 = pic.update_candid_as_or_panic(
            input.root,
            operator,
            "audit_recovery_balance",
            (8_000_000_000_000_u128,),
        );
        assert!(burned > 0);
        let digest = desired_sha256(&desired);
        let platform = || {
            literal_zero_journey_platform(
                &desired,
                input.icp_wrapper,
                root,
                input.local_replica.clone(),
                true,
            )
        };
        phase = phase.next("root_reinstall_review_and_authority_rejection");
        let mut first_platform = platform();
        let reviewed = fleet_ensure_workflow::plan(
            root,
            &desired,
            &digest,
            &desired.fleet,
            1_800_000_000_000_000_010,
            &mut first_platform,
        )
        .expect("review the exact Root reinstall");
        assert_eq!(
            reviewed.plan.scope,
            canic_host::fleet_ensure::model::FleetEnsurePlanScope::RootReinstallPrerequisite
        );
        assert_eq!(reviewed.plan.root_reinstall_bindings.len(), 1);
        let funding = reviewed.plan.recovery_review.as_ref().unwrap();
        let reset_funding = reviewed.plan.conservation.maximum_operator_debit_cycles;
        assert!(reset_funding > 0);
        assert!(matches!(
            reviewed.plan.canisters[0].actions.as_slice(),
            [
                EnsureAction::Stop { .. },
                EnsureAction::Fund { .. },
                EnsureAction::Install { .. },
                EnsureAction::Start { .. }
            ]
        ));
        assert_eq!(funding.maximum_successor_actions, 0);
        assert_eq!(funding.whole_continuation_ceiling_cycles, 0);
        assert_eq!(funding.per_step_burn_cycles, 8_000_000_000_000);
        let forecast = funding
            .startup_funding
            .iter()
            .find(|forecast| forecast.root == reviewed.plan.root_reinstall_bindings[0].name)
            .expect("startup forecast is visible before the destructive prerequisite");
        assert!(forecast.maximum_continuation_steps > 0);
        assert!(forecast.required_native_cycles >= forecast.startup_minimum_cycles);
        assert!(forecast.unfunded_role.is_none());
        assert!(!root.join("reinstall-mutations.log").exists());
        pic.set_controllers(
            input.root,
            Some(operator),
            vec![operator, Principal::anonymous()],
        )
        .expect("introduce controller drift after review");
        let refused = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            &mut first_platform,
        );
        assert!(matches!(refused, Err(EnsureWorkflowError::Policy(
            canic_host::fleet_ensure::policy::EnsurePolicyError::RootManagementAuthorityMismatch { .. }
        ))), "foreign controller must reject before reset: {refused:?}");
        assert!(!root.join("reinstall-mutations.log").exists());
        assert!(
            canic_host::fleet_ensure::ops::read_journal(&old_paths)
                .unwrap()
                .is_some_and(|journal| journal.completion
                    == canic_host::fleet_ensure::model::FleetEnsureCompletion::Converged)
        );
        pic.set_controllers(input.root, Some(operator), vec![operator])
            .expect("restore the exact reviewed controller authority");
        let old_pool = root_pool_status_as(pic, input.root, operator);
        let workload_ids = old_pool
            .entries
            .iter()
            .filter(|asset| matches!(asset.status, CanisterPoolAssetStatus::Workload { .. }))
            .take(6)
            .map(|asset| asset.canister_id)
            .collect::<Vec<_>>();
        assert_eq!(workload_ids.len(), 6);
        for asset in workload_ids {
            let spent: Result<u128, Error> = pic
                .update_candid_as(
                    asset,
                    input.root,
                    "audit_recovery_balance",
                    (1_900_000_000_000_u128,),
                )
                .unwrap();
            assert!(spent.unwrap() > 0);
        }
        let operator_before = ledger_account_balance(pic, ledger, operator);
        let native_before = [input.coordinator, input.root, input.store]
            .into_iter()
            .chain(input.pools.iter().copied())
            .map(|id| pic.cycle_balance(id))
            .sum::<u128>();
        let root_version = pic
            .canister_status(input.root, Some(operator))
            .unwrap()
            .version;
        let withdrawals_before: u64 = pic.query_candid(ledger, "withdrawal_count", ()).unwrap();
        std::fs::write(root.join("fail-before-funding"), b"once").unwrap();
        let stopped = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            &mut first_platform,
        );
        assert!(
            matches!(stopped, Err(EnsureWorkflowError::Platform(_))),
            "stop before payment: {stopped:?}"
        );
        assert!(root.join("failed-before-funding").is_file());
        assert_eq!(
            serde_json::to_value(
                pic.canister_status(input.root, Some(operator))
                    .unwrap()
                    .status
            )
            .unwrap(),
            serde_json::json!("stopped")
        );
        assert_eq!(
            pic.query_candid::<u64, _>(ledger, "withdrawal_count", ())
                .unwrap(),
            withdrawals_before
        );
        let journal = canic_host::fleet_ensure::ops::read_journal(&old_paths)
            .unwrap()
            .unwrap();
        assert_eq!(
            journal.effects[0].state,
            canic_host::fleet_ensure::model::EffectState::Applied
        );
        assert_eq!(
            journal.effects[1].state,
            canic_host::fleet_ensure::model::EffectState::Intent
        );
        assert!(journal.effects[1].receipt.is_none());
        pic.start_canister(input.root, Some(operator)).unwrap();
        let restarted = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            &mut platform(),
        );
        assert!(
            matches!(restarted, Err(EnsureWorkflowError::DriftedBeforeApply)),
            "restarted source rejects credit: {restarted:?}"
        );
        assert_eq!(
            pic.query_candid::<u64, _>(ledger, "withdrawal_count", ())
                .unwrap(),
            withdrawals_before
        );
        pic.stop_canister(input.root, Some(operator)).unwrap();
        std::fs::write(root.join("lose-funding-response"), b"once").unwrap();
        let lost_funding = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            &mut platform(),
        );
        assert!(
            matches!(lost_funding, Err(EnsureWorkflowError::Platform(_))),
            "lose stopped Root credit reply: {lost_funding:?}"
        );
        assert!(root.join("lost-funding-response").is_file());
        assert_eq!(
            serde_json::to_value(
                pic.canister_status(input.root, Some(operator))
                    .unwrap()
                    .status
            )
            .unwrap(),
            serde_json::json!("stopped")
        );
        assert_eq!(
            pic.query_candid::<u64, _>(ledger, "withdrawal_count", ())
                .unwrap(),
            withdrawals_before + 1
        );
        assert_eq!(
            ledger_account_balance(pic, ledger, operator),
            operator_before.clone() - Nat::from(reset_funding)
        );
        std::fs::write(root.join("lose-install-response"), b"once").unwrap();
        phase = phase.next("root_reinstall_lost_response_and_recovery");
        let lost = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            &mut first_platform,
        );
        assert!(
            matches!(lost, Err(EnsureWorkflowError::Platform(_))),
            "lost install response: {lost:?}"
        );
        assert!(root.join("lost-install-response").is_file());
        let installed_version = pic
            .canister_status(input.root, Some(operator))
            .unwrap()
            .version;
        assert!(installed_version > root_version);
        let mut recovered = platform();
        let reset = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            &mut recovered,
        )
        .expect("reconcile the issued reinstall");
        assert!(reset.terminal);
        assert!(
            pic.canister_status(input.root, Some(operator))
                .unwrap()
                .version
                >= installed_version
        );
        assert_eq!(
            std::fs::read_to_string(root.join("reinstall-mutations.log"))
                .unwrap()
                .lines()
                .count(),
            1
        );
        let replay = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            &mut platform(),
        )
        .expect("effect-free reset replay");
        assert_eq!(replay.effects_applied, 0);
        assert_eq!(
            pic.query_candid::<u64, _>(ledger, "withdrawal_count", ())
                .unwrap(),
            withdrawals_before + 1
        );
        assert_eq!(
            ledger_account_balance(pic, ledger, input.root),
            Nat::from(1_000_000_000_u128)
        );
        phase = phase.next("successor_reviews_and_convergence");
        let full = fleet_ensure_workflow::plan(
            root,
            &desired,
            &digest,
            &desired.fleet,
            1_800_000_000_000_000_011,
            &mut recovered,
        )
        .expect("review remaining current Fleet convergence");
        assert_eq!(
            full.plan.scope,
            canic_host::fleet_ensure::model::FleetEnsurePlanScope::Full
        );
        assert!(full.plan.continuation.is_some());
        let recovery = full.plan.recovery_review.as_ref().unwrap();
        assert!(recovery.continuation_reserve_cycles > 0);
        assert!(recovery.continuation_reserve_cycles <= recovery.whole_continuation_ceiling_cycles);
        assert_eq!(recovery.known_pool_funding.len(), 6);
        let reviewed_startup_funding = full.plan.conservation.maximum_operator_debit_cycles;
        let paused = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &full.plan.plan_sha256,
            &mut recovered,
        )
        .expect_err("new funding remains outside the sealed protocol-only continuation");
        let EnsureWorkflowError::SuccessorReviewRequired {
            reason:
                canic_host::fleet_ensure::model::FleetEnsureSuccessorReviewReason::AdditionalEffect,
            review: Some(details),
        } = paused
        else {
            panic!("expected informative recovery pause: {paused:?}");
        };
        assert!(details.maximum_additional_debit_cycles > 0);
        assert!(details.actions.iter().any(|action| action.kind == "fund"));
        assert_eq!(
            ledger_account_balance(pic, ledger, operator),
            operator_before.clone() - Nat::from(reset_funding + reviewed_startup_funding)
        );
        let mut total_funding = reset_funding + reviewed_startup_funding;
        let mut reviews_remaining = 8;
        let (ready, latest) = loop {
            assert!(reviews_remaining > 0, "bounded recovery reviews");
            reviews_remaining -= 1;
            let latest = fleet_ensure_workflow::plan(
                root,
                &desired,
                &digest,
                &desired.fleet,
                1_800_000_000_000_000_011,
                &mut recovered,
            )
            .expect("review newly observed recovery work");
            assert_eq!(latest.plan.operation_id, full.plan.operation_id);
            assert!(planned_actions(&latest.plan).iter().all(|action| !matches!(
                action,
                EnsureAction::Install { .. } | EnsureAction::Create { .. }
            )));
            total_funding += latest.plan.conservation.maximum_operator_debit_cycles;
            if latest.plan.conservation.maximum_operator_debit_cycles > 0 {
                std::fs::write(root.join("lose-funding-response"), []).unwrap();
            }
            let mut result = fleet_ensure_workflow::apply(
                root,
                &desired,
                &digest,
                &desired.fleet,
                &latest.plan.plan_sha256,
                &mut recovered,
            );
            if matches!(result, Err(EnsureWorkflowError::Platform(_)))
                && root.join("lost-funding-response").exists()
            {
                recovered = platform();
                result = fleet_ensure_workflow::apply(
                    root,
                    &desired,
                    &digest,
                    &desired.fleet,
                    &latest.plan.plan_sha256,
                    &mut recovered,
                );
            }
            match result {
                Ok(ready) => break (ready, latest),
                Err(EnsureWorkflowError::SuccessorReviewRequired {
                    review: Some(details),
                    ..
                }) => assert!(!details.actions.is_empty()),
                Err(error) => panic!("reviewed recovery must converge: {error:?}"),
            }
        };
        phase = phase.next("retained_estate_state_and_replay");
        assert!(total_funding > 0);
        assert_eq!(
            ledger_account_balance(pic, ledger, operator),
            operator_before - Nat::from(total_funding)
        );
        assert!(ready.terminal);
        assert!(ready.actual_conservation.is_some());
        let pool = root_pool_status_as(pic, input.root, operator);
        assert_eq!(pool.workload, 19);
        assert_eq!(pool.ready, 5);
        assert_eq!(pool.pending_reset, 0);
        let state = canic_host::fleet_ensure::ops::read_state(&old_paths, &desired.fleet).unwrap();
        assert!(state.active_registry.is_some());
        for asset in input.pools {
            assert_eq!(
                pic.canister_status(*asset, Some(input.root))
                    .unwrap()
                    .settings
                    .controllers,
                vec![input.root]
            );
        }
        let mutations = std::fs::read_to_string(root.join("reinstall-mutations.log")).unwrap();
        assert_eq!(mutations.lines().count(), 3);
        let native_after = [input.coordinator, input.root, input.store]
            .into_iter()
            .chain(input.pools.iter().copied())
            .map(|id| pic.cycle_balance(id))
            .sum::<u128>();
        assert!(native_before + total_funding >= native_after);
        assert!(native_before + total_funding - native_after < 10_000_000_000_000);
        assert_eq!(
            ledger_account_balance(pic, ledger, input.root),
            Nat::from(1_000_000_000_u128)
        );
        let same_plan = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &latest.plan.plan_sha256,
            &mut platform(),
        )
        .expect("original full plan replay");
        assert_eq!(same_plan.effects_applied, 0);
        let again = fleet_ensure_workflow::plan(
            root,
            &desired,
            &digest,
            &desired.fleet,
            1_800_000_000_000_000_012,
            &mut platform(),
        )
        .expect("plan the completed replacement");
        assert!(planned_actions(&again.plan).is_empty());
        let replay = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &again.plan.plan_sha256,
            &mut platform(),
        )
        .expect("newly planned effect-free replay");
        assert_eq!(replay.effects_applied, 0);
        assert_eq!(
            std::fs::read_to_string(root.join("reinstall-mutations.log")).unwrap(),
            mutations
        );
        super::super::fixture::progress(
            "generated changed-release reinstall, recovery and replay complete",
        );
        phase.finish();
    }

    #[cfg(test)]
    #[derive(CandidType)]
    struct JourneySubnetRequest {
        principal: Principal,
    }

    #[cfg(test)]
    #[derive(CandidType, Deserialize)]
    struct JourneySubnetPayload {
        subnet_id: Option<Principal>,
    }

    #[cfg(test)]
    fn install_journey_registry(
        pic: &PocketIc,
        wasm: &[u8],
        root: Principal,
        subnet: Principal,
        canisters: Vec<Principal>,
    ) {
        for canister in &canisters {
            assert_eq!(pic.get_subnet(*canister), Some(subnet));
        }
        let registry = Principal::from_text("rwlgt-iiaaa-aaaaa-aaaaa-cai").unwrap();
        pic.create_canister_with_id(None, None, registry)
            .expect("create canonical NNS routing boundary");
        pic.install_canister(
            registry,
            wasm.to_vec(),
            encode_one(CyclesLedgerStubInitArgs {
                canister_ids: canisters,
                expected_controllers_by_index: None,
                expected_root: root,
                expected_subnet: subnet,
                initial_balances: None,
                pending_first_index: None,
                withdrawal_fee: None,
            })
            .unwrap(),
            None,
        );
        let unknown: Result<JourneySubnetPayload, String> = pic
            .query_candid(
                registry,
                "get_subnet_for_canister",
                (JourneySubnetRequest {
                    principal: Principal::from_slice(&[0x99; 29]),
                },),
            )
            .expect("query unknown Principal routing");
        assert!(unknown.unwrap().subnet_id.is_none());
    }

    #[cfg(test)]
    fn literal_zero_journey_platform(
        desired: &DesiredFleet,
        wrapper: &Path,
        root: &Path,
        replica: LocalReplicaTarget,
        accelerate_observations: bool,
    ) -> IcpEnsurePlatform {
        let platform = IcpEnsurePlatform::new(desired.clone(), wrapper.to_str().unwrap(), root)
            .with_local_replica(replica)
            .with_observation_handler(crate::pic::timing::observation)
            .with_progress_handler(|progress| {
                let unix_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("measurement clock follows Unix epoch")
                    .as_millis();
                eprintln!(
                    "[FLEET-MEASURE] {}",
                    serde_json::json!({"unix_ms": unix_ms, "progress": progress}),
                );
            });
        if accelerate_observations {
            platform.with_observation_delay_bounds(
                LITERAL_ZERO_OBSERVATION_DELAY,
                LITERAL_ZERO_OBSERVATION_DELAY,
            )
        } else {
            platform
        }
    }

    #[cfg(test)]
    struct AutonomousFundingJourney<'a> {
        adapter_root: &'a Path,
        icp_wrapper: &'a Path,
        local_replica: &'a LocalReplicaTarget,
        pic: &'a PocketIc,
        desired: &'a DesiredFleet,
        root: Principal,
        operator: Principal,
        cycles_ledger: Principal,
        assets: &'a [Principal],
        imported: &'a [Principal],
        repair_failed_reserve: bool,
        funding_pause: bool,
        native_pause: Option<native_funding::Scenario>,
        readiness_floor: u128,
        operator_after_initial_creation: u128,
    }

    /// Prepare real infrastructure while the dedicated fresh journey owns startup recovery.
    #[cfg(test)]
    fn prepare_journey_infrastructure(
        pic: &PocketIc,
        adapter_root: &Path,
        desired: &DesiredFleet,
        plan: &FleetEnsurePlan,
        platform: &mut IcpEnsurePlatform,
    ) -> DesiredFleet {
        let paths = canic_host::fleet_ensure::ops::EnsurePaths::under(
            adapter_root,
            &desired.environment,
            &desired.fleet,
        );
        let mut state = canic_host::fleet_ensure::ops::read_state(&paths, &desired.fleet).unwrap();
        let actions = planned_actions(plan);
        // Use the production Ledger adapter and init compiler, preserving exact debits and authority.
        for action in actions
            .iter()
            .copied()
            .filter(|action| matches!(action, EnsureAction::Create { .. }))
        {
            let outcome = platform
                .apply(
                    &plan.operation_id,
                    action,
                    &fixture_effect_intent(action),
                    &state,
                )
                .expect("create prepared funding infrastructure through the Ledger");
            state
                .pending_principals
                .insert(action.name().to_owned(), outcome.created_principal.unwrap());
        }
        for action in actions
            .iter()
            .copied()
            .filter(|action| matches!(action, EnsureAction::Install { .. }))
        {
            platform
                .apply(
                    &plan.operation_id,
                    action,
                    &fixture_effect_intent(action),
                    &state,
                )
                .expect("install exact production funding infrastructure");
        }
        for action in actions {
            match action {
                EnsureAction::Create { .. } | EnsureAction::Install { .. } => {}
                EnsureAction::SetControllers {
                    name,
                    controllers,
                    controller_canisters,
                    ..
                } => {
                    let controllers = controllers
                        .iter()
                        .chain(
                            controller_canisters
                                .iter()
                                .map(|name| &state.pending_principals[name]),
                        )
                        .map(|principal| Principal::from_text(principal).unwrap())
                        .collect();
                    pic.set_controllers(
                        Principal::from_text(&state.pending_principals[name]).unwrap(),
                        Some(Principal::from_text(&desired.operator).unwrap()),
                        controllers,
                    )
                    .expect("set prepared pool controllers");
                }
                _ => panic!("funding fixture setup accepts only fresh infrastructure actions"),
            }
        }
        // No completed journal is manufactured: the funding proof plans from live observations.
        let mut prepared = desired.clone();
        prepared.bootstrap.as_mut().unwrap().fresh_estate = false;
        for canister in &mut prepared.canisters {
            canister.principal = Some(state.pending_principals[&canister.name].clone());
        }
        prepared
    }

    /// Reset prepared imports through the public Root protocol before funding or reinstall review.
    #[cfg(test)]
    fn prepare_ready_imports(
        pic: &PocketIc,
        root: Principal,
        operator: Principal,
        pools: &[Principal],
    ) {
        for asset in pools {
            let response = root_command_as(
                pic,
                root,
                operator,
                RootCommandFragment::ImportPoolCanister(PoolCanisterRequest {
                    canister_id: *asset,
                }),
            )
            .expect("prepare Ready import through the public Root protocol");
            assert!(
                matches!(response, RootCommandResponseFragment::ImportPoolCanister(
                PoolImportResponse::Imported { canister_id, .. }
            ) if canister_id == *asset)
            );
        }
    }

    /// Start one real platform effect used to prepare a PocketIC fixture.
    #[cfg(test)]
    fn fixture_effect_intent(action: &EnsureAction) -> EffectRecord {
        EffectRecord {
            publication_attempts: 0,
            maintenance_attempts: 0,
            action_sha256: action_sha256(action),
            created_principal: None,
            destination_post_cycles: None,
            destination_pre_cycles: None,
            post_cycles: None,
            pre_cycles: None,
            pre_canister_version: None,
            progress_identity: None,
            receipt: None,
            state: EffectState::Intent,
        }
    }

    /// Inject a retained underforecast without inventing paid receipts or completed effects.
    /// Every protocol effect below reaches the real adapter; the funding deficit is fixture input.
    #[cfg(test)]
    fn retain_issued_underfunded_fixture(
        input: &AutonomousFundingJourney<'_>,
        plan: &mut FleetEnsurePlan,
        state: &canic_host::fleet_ensure::model::FleetEnsureStateRecord,
        platform: &mut IcpEnsurePlatform,
    ) {
        let paths = canic_host::fleet_ensure::ops::EnsurePaths::under(
            input.adapter_root,
            &plan.environment,
            &plan.fleet,
        );
        if input.native_pause.is_some() {
            native_funding::omit_forecast_native_funding(plan, input.root);
        } else {
            for canister in &mut plan.canisters {
                canister
                    .actions
                    .retain(|action| !matches!(action, EnsureAction::FundEstate { .. }));
            }
            plan.conservation.maximum_operator_debit_cycles = 0;
            plan.conservation.maximum_new_funding_cycles = 0;
            plan.conservation.maximum_unavoidable_fee_cycles = 0;
            for domain in &mut plan.conservation.estate_funding_domains {
                domain.maximum_funding_cycles = 0;
            }
        }
        plan.plan_sha256 = canic_host::fleet_ensure::policy::expected_plan_sha256(plan);
        canic_host::fleet_ensure::ops::write_plan(&paths, plan).unwrap();
        let mut journal = canic_host::fleet_ensure::model::FleetEnsureJournalRecord {
            funding_reviews: Vec::new(),
            successor_phases: Vec::new(),
            completion: canic_host::fleet_ensure::model::FleetEnsureCompletion::InProgress,
            estate_funding_required: None,
            effects: Vec::new(),
            fleet: plan.fleet.clone(),
            initial_controlled_cycles: plan.conservation.observed_controlled_cycles,
            initial_estate_funding_cycles_by_root: plan
                .conservation
                .estate_funding_domains
                .iter()
                .map(|domain| (domain.root.clone(), domain.available_cycles.unwrap()))
                .collect(),
            initial_operator_cycles: input.operator_after_initial_creation,
            operation_id: plan.operation_id.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            schema_version: 1,
            stalled_observations: 0,
        };
        for action in planned_actions(plan) {
            let mut effect = fixture_effect_intent(action);
            effect.pre_cycles = platform.action_cycles(action, state).unwrap();
            effect.destination_pre_cycles =
                platform.action_destination_cycles(action, state).unwrap();
            journal.effects.push(effect.clone());
            canic_host::fleet_ensure::ops::write_journal(&paths, &journal).unwrap();
            let outcome = platform
                .apply(&plan.operation_id, action, &effect, state)
                .unwrap();
            effect.receipt = outcome.receipt;
            effect.post_cycles = outcome.post_cycles;
            effect.state = EffectState::Issued;
            let is_provisioning = matches!(action, EnsureAction::FleetProtocol { action, .. }
                if matches!(action.as_ref(), CurrentFleetProtocolAction::ProvisionComponents { .. }));
            if !is_provisioning {
                for attempt in 0..64 {
                    let observed = platform
                        .observe_effect(&plan.operation_id, action, &effect, state)
                        .unwrap();
                    if observed.applied {
                        effect.post_cycles = match observed.post_cycles {
                            Some(cycles) => Some(cycles),
                            None => platform.action_cycles(action, state).unwrap(),
                        };
                        effect.destination_post_cycles =
                            platform.action_destination_cycles(action, state).unwrap();
                        effect.state = EffectState::Applied;
                        break;
                    }
                    platform.pace_effect_observation(action, attempt);
                }
                assert_eq!(effect.state, EffectState::Applied);
            }
            *journal.effects.last_mut().unwrap() = effect;
            canic_host::fleet_ensure::ops::write_journal(&paths, &journal).unwrap();
            if is_provisioning {
                return;
            }
        }
        panic!("fixture must retain a real issued provisioning effect");
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one composed production journey binds lost transfer, autonomous creation, terminal conservation and replay"
    )]
    fn assert_funded_autonomous_journey(input: AutonomousFundingJourney<'_>) {
        if input.native_pause.is_some() {
            native_funding::assert_issued_native_funding(&input);
            return;
        }
        let new_platform = |desired: &DesiredFleet| {
            IcpEnsurePlatform::new(
                desired.clone(),
                input.icp_wrapper.to_str().unwrap(),
                input.adapter_root,
            )
            .with_local_replica(input.local_replica.clone())
            .with_observation_handler(crate::pic::timing::observation)
        };
        assert_eq!(
            ledger_account_balance(input.pic, input.cycles_ledger, input.operator),
            Nat::from(input.operator_after_initial_creation)
        );
        assert_eq!(
            ledger_account_balance(input.pic, input.cycles_ledger, input.root),
            Nat::from(0_u8)
        );
        let paths = canic_host::fleet_ensure::ops::EnsurePaths::under(
            input.adapter_root,
            &input.desired.environment,
            &input.desired.fleet,
        );
        let state =
            canic_host::fleet_ensure::ops::read_state(&paths, &input.desired.fleet).unwrap();
        let desired = input.desired.clone();
        let source = desired_sha256(&desired);
        let mut platform = new_platform(&desired);
        let mut planned = fleet_ensure_workflow::plan(
            input.adapter_root,
            &desired,
            &source,
            &desired.fleet,
            1_800_000_000_000_000_001,
            &mut platform,
        )
        .expect("review exact estate funding and current protocol");
        if input.imported.len() == 4 {
            if input.repair_failed_reserve {
                register_failed_reserve(&input);
            }
            establish_workloads_before_refill(&input, &planned.plan, &state, &mut platform);
            if input.repair_failed_reserve {
                assert_failed_reserve_journey(&input, &desired, &source, &mut platform);
                return;
            }
            planned = fleet_ensure_workflow::plan(
                input.adapter_root,
                &desired,
                &source,
                &desired.fleet,
                1_800_000_000_000_000_002,
                &mut platform,
            )
            .expect("review refill from four real Workloads and no Ready reserve");
        }
        let creation_count = u128::try_from(input.assets.len()).unwrap();
        let per_creation_funding = input.readiness_floor
            + MAINNET_REFILL_EXECUTION_MARGIN
            + MAINNET_REFILL_MANAGEMENT_CREATION_FEE
            + MAINNET_REFILL_LEDGER_FEE;
        let funding = creation_count * per_creation_funding;
        let fund_actions = planned_actions(&planned.plan)
            .into_iter()
            .filter(|action| matches!(action, EnsureAction::FundEstate { .. }))
            .count();
        assert_eq!(fund_actions, 1);
        assert_eq!(
            planned.plan.conservation.maximum_operator_debit_cycles,
            funding + MAINNET_REFILL_LEDGER_FEE
        );
        let domain = &planned.plan.conservation.estate_funding_domains[0];
        assert_eq!(
            domain.root_principal.as_deref(),
            Some(input.root.to_text().as_str())
        );
        assert_eq!(
            usize::try_from(domain.required_creation_count).unwrap(),
            input.assets.len()
        );
        assert_eq!(domain.maximum_funding_cycles, funding);
        let reviewed_digest = if input.funding_pause {
            retain_issued_underfunded_fixture(&input, &mut planned.plan, &state, &mut platform);
            let paused = fleet_ensure_workflow::apply(
                input.adapter_root,
                &desired,
                &source,
                &desired.fleet,
                &planned.plan.plan_sha256,
                &mut platform,
            );
            assert!(
                matches!(paused, Err(EnsureWorkflowError::EstateFundingRequired(_))),
                "issued Root creation must pause: {paused:?}"
            );
            let journal = canic_host::fleet_ensure::ops::read_journal(&paths)
                .unwrap()
                .unwrap();
            assert!(
                journal
                    .estate_funding_required
                    .as_ref()
                    .unwrap()
                    .pending_creation_operation_id
                    .is_some()
            );
            let report = fleet_ensure_workflow::plan(
                input.adapter_root,
                &desired,
                &source,
                &desired.fleet,
                1_800_000_000_000_000_099,
                &mut platform,
            )
            .expect("ordinary planning exposes exact funding review");
            assert_eq!(report.plan.plan_sha256, planned.plan.plan_sha256);
            let review = report.funding_review.unwrap();
            assert_eq!(review.pause.shortfall_cycles(), funding);
            assert_eq!(
                canic_host::fleet_ensure::ops::read_journal(&paths)
                    .unwrap()
                    .unwrap()
                    .effects,
                journal.effects
            );
            review.review_sha256
        } else {
            planned.plan.plan_sha256.clone()
        };
        let _: () = input
            .pic
            .update_candid(
                input.cycles_ledger,
                "set_transfer_fee_override",
                (Some(Nat::from(MAINNET_REFILL_LEDGER_FEE * 2)),),
            )
            .unwrap();
        let fee_rejection = fleet_ensure_workflow::apply(
            input.adapter_root,
            &desired,
            &source,
            &desired.fleet,
            &reviewed_digest,
            &mut platform,
        );
        assert!(
            matches!(fee_rejection, Err(EnsureWorkflowError::Platform(
            IcpEnsurePlatformError::LedgerTransferFeeChanged {
                reviewed_fee_cycles: MAINNET_REFILL_LEDGER_FEE, expected_fee_cycles,
            }
        )) if expected_fee_cycles == MAINNET_REFILL_LEDGER_FEE * 2),
            "a changed fee must reject before debit: {fee_rejection:?}"
        );
        let transfers: u64 = input
            .pic
            .query_candid(input.cycles_ledger, "transfer_count", ())
            .unwrap();
        assert_eq!(transfers, 0);
        assert_eq!(
            ledger_account_balance(input.pic, input.cycles_ledger, input.operator),
            Nat::from(input.operator_after_initial_creation)
        );
        assert_eq!(
            ledger_account_balance(input.pic, input.cycles_ledger, input.root),
            Nat::from(0_u8)
        );
        let _: () = input
            .pic
            .update_candid(
                input.cycles_ledger,
                "set_transfer_fee_override",
                (None::<Nat>,),
            )
            .unwrap();
        std::fs::write(input.adapter_root.join("lose-estate-funding-response"), []).unwrap();
        let lost = fleet_ensure_workflow::apply(
            input.adapter_root,
            &desired,
            &source,
            &desired.fleet,
            &reviewed_digest,
            &mut platform,
        );
        assert!(
            matches!(lost, Err(EnsureWorkflowError::Platform(_))),
            "lose exact completed transfer response: {lost:?}"
        );
        assert!(
            input
                .adapter_root
                .join("lost-estate-funding-response")
                .is_file(),
            "the transfer must reach response-loss injection: {lost:?}"
        );
        let transfers: u64 = input
            .pic
            .query_candid(input.cycles_ledger, "transfer_count", ())
            .unwrap();
        assert_eq!(transfers, 1);
        assert_eq!(
            ledger_account_balance(input.pic, input.cycles_ledger, input.operator),
            Nat::from(input.operator_after_initial_creation - funding - MAINNET_REFILL_LEDGER_FEE)
        );
        if !input.funding_pause {
            assert_eq!(
                ledger_account_balance(input.pic, input.cycles_ledger, input.root),
                Nat::from(funding)
            );
        }
        let mut recovered = new_platform(&desired);
        let completed = fleet_ensure_workflow::apply(
            input.adapter_root,
            &desired,
            &source,
            &desired.fleet,
            &reviewed_digest,
            &mut recovered,
        )
        .unwrap_or_else(|error| {
            panic!(
                "recover funding receipt and autonomous creation: {error:?}; pool: {:?}",
                root_pool_status_as(input.pic, input.root, input.operator)
            )
        });
        assert!(completed.terminal);
        let actual = completed.actual_conservation.as_ref().unwrap();
        assert_eq!(actual.estate_funding_cycles, funding);
        assert_eq!(
            actual.operator_debit_cycles,
            funding + MAINNET_REFILL_LEDGER_FEE
        );
        assert_eq!(
            actual.exact_estate_creation_fee_cycles,
            creation_count * (MAINNET_REFILL_MANAGEMENT_CREATION_FEE + MAINNET_REFILL_LEDGER_FEE)
        );
        assert_eq!(
            actual.observed_starting_cycles
                + actual.operator_debit_cycles
                + actual.observed_net_cycle_credit_cycles,
            actual.final_controlled_cycles
                + actual.observed_net_cycle_debit_cycles
                + actual.exact_unavoidable_fee_cycles
                + actual.exact_estate_creation_fee_cycles
        );
        let pool = root_pool_status_as(input.pic, input.root, input.operator);
        assert_eq!(
            (pool.workload, pool.ready, pool.failed, pool.pending_reset),
            (
                u32::try_from(input.imported.len()).unwrap(),
                u32::try_from(input.assets.len()).unwrap(),
                0,
                0
            )
        );
        assert!(pool.pending_creation.is_none());
        if input.funding_pause {
            let journal = canic_host::fleet_ensure::ops::read_journal(&paths)
                .unwrap()
                .unwrap();
            let canic_host::fleet_ensure::model::FundingPauseRecord::Estate(pause) =
                &journal.funding_reviews[0].pause
            else {
                panic!("estate funding review");
            };
            let pending = pause.pending_creation_operation_id.as_ref().unwrap();
            assert!(
                pool.entries
                    .iter()
                    .filter_map(|entry| entry.creation_receipt.as_ref())
                    .any(
                        |receipt| canic_core::cdk::utils::hash::hex_bytes(receipt.operation_id)
                            == *pending
                    )
            );
        }
        for (index, asset) in input.assets.iter().enumerate() {
            let created = pool
                .entries
                .iter()
                .find(|entry| entry.canister_id == *asset)
                .unwrap();
            assert_eq!(created.origin, CanisterPoolAssetOrigin::Created);
            let receipt = created.creation_receipt.as_ref().unwrap();
            assert_eq!(
                receipt.block_index,
                u64::try_from(4 + input.imported.len() + index).unwrap()
            );
            assert_ne!(receipt.operation_id, [0; 32]);
            assert_eq!(receipt.cycles_ledger, input.cycles_ledger);
            assert_eq!(
                receipt.ledger_amount.to_u128(),
                per_creation_funding - MAINNET_REFILL_LEDGER_FEE
            );
            assert_eq!(receipt.ledger_fee.to_u128(), MAINNET_REFILL_LEDGER_FEE);
            assert_eq!(
                receipt.management_creation_fee.to_u128(),
                MAINNET_REFILL_MANAGEMENT_CREATION_FEE
            );
            assert_eq!(receipt.readiness_floor.to_u128(), input.readiness_floor);
            assert_eq!(
                receipt.creation_execution_margin.to_u128(),
                MAINNET_REFILL_EXECUTION_MARGIN
            );
            let first_balance = receipt.first_observed_cycles.as_ref().unwrap().to_u128();
            assert!(
                (input.readiness_floor..=input.readiness_floor + MAINNET_REFILL_EXECUTION_MARGIN)
                    .contains(&first_balance)
            );
            assert!(created.cycles.to_u128() >= input.readiness_floor);
        }
        for principal in input.assets.iter().chain(input.imported).copied() {
            let status = input
                .pic
                .canister_status(principal, Some(input.root))
                .unwrap();
            assert_eq!(status.settings.controllers, vec![input.root]);
            assert!(status.cycles >= input.readiness_floor);
        }
        let requests: u64 = input
            .pic
            .query_candid(input.cycles_ledger, "request_count", ())
            .unwrap();
        assert_eq!(
            requests,
            u64::try_from(3 + input.imported.len() + input.assets.len() + 1).unwrap(),
            "every host and Root creation once, with one exact Root creation attempted twice"
        );
        assert_eq!(
            ledger_account_balance(input.pic, input.cycles_ledger, input.root),
            Nat::from(0_u8)
        );
        let mut replay_platform = new_platform(&desired);
        let replay = fleet_ensure_workflow::apply(
            input.adapter_root,
            &desired,
            &source,
            &desired.fleet,
            &planned.plan.plan_sha256,
            &mut replay_platform,
        )
        .expect("effect-free funded estate replay");
        assert!(replay.terminal);
        assert_eq!(replay.effects_applied, 0);
        let transfers: u64 = input
            .pic
            .query_candid(input.cycles_ledger, "transfer_count", ())
            .unwrap();
        let replay_requests: u64 = input
            .pic
            .query_candid(input.cycles_ledger, "request_count", ())
            .unwrap();
        assert_eq!((transfers, replay_requests), (1, requests));
        let replay_pool = root_pool_status_as(input.pic, input.root, input.operator);
        for asset in input.assets {
            let before = pool
                .entries
                .iter()
                .find(|entry| entry.canister_id == *asset)
                .unwrap();
            let after = replay_pool
                .entries
                .iter()
                .find(|entry| entry.canister_id == *asset)
                .unwrap();
            assert_eq!(after.creation_receipt, before.creation_receipt);
        }
        assert_eq!(
            ledger_account_balance(input.pic, input.cycles_ledger, input.operator),
            Nat::from(input.operator_after_initial_creation - funding - MAINNET_REFILL_LEDGER_FEE)
        );
    }

    /// Establish the refill fixture through current public protocol calls before its host review.
    #[cfg(test)]
    fn establish_workloads_before_refill(
        input: &AutonomousFundingJourney<'_>,
        setup_plan: &FleetEnsurePlan,
        state: &FleetEnsureStateRecord,
        platform: &mut IcpEnsurePlatform,
    ) {
        let started_at = Instant::now();
        super::super::fixture::progress("establishing four Workloads before reserve funding");
        for action in &setup_plan.protocol_actions {
            let EnsureAction::FleetProtocol {
                action: protocol, ..
            } = action
            else {
                panic!("workload setup accepts only current protocol calls");
            };
            if matches!(
                protocol.as_ref(),
                CurrentFleetProtocolAction::MaintainPoolReadiness { .. }
                    | CurrentFleetProtocolAction::ObservePoolReadiness { .. }
            ) {
                continue;
            }
            let mut record = fixture_effect_intent(action);
            let outcome = platform
                .apply(&setup_plan.operation_id, action, &record, state)
                .expect("execute current protocol fixture setup");
            record.receipt = outcome.receipt;
            record.state = EffectState::Issued;
            let completed = (0..120).any(|attempt| {
                let observation = platform
                    .observe_effect(&setup_plan.operation_id, action, &record, state)
                    .expect("observe current protocol fixture setup");
                if observation.applied {
                    return true;
                }
                assert!(
                    observation.estate_funding_required.is_none(),
                    "prepaid Workloads need no new creation"
                );
                platform.pace_effect_observation(action, attempt);
                false
            });
            assert!(
                completed,
                "current protocol workload setup must complete: {protocol:?}"
            );
        }
        let pool = root_pool_status_as(input.pic, input.root, input.operator);
        assert_eq!(
            (pool.workload, pool.ready, pool.failed),
            (4, 0, if input.repair_failed_reserve { 4 } else { 0 })
        );
        assert_eq!(
            ledger_account_balance(input.pic, input.cycles_ledger, input.root),
            Nat::from(0_u8)
        );
        let transfers: u64 = input
            .pic
            .query_candid(input.cycles_ledger, "transfer_count", ())
            .unwrap();
        assert_eq!(transfers, 0);
        progress_elapsed("four Workloads active with no Ready reserve", started_at);
    }

    #[cfg(test)]
    fn register_failed_reserve(input: &AutonomousFundingJourney<'_>) {
        for asset in input.assets {
            let response = root_command_as(
                input.pic,
                input.root,
                input.operator,
                RootCommandFragment::ImportPoolCanister(PoolCanisterRequest {
                    canister_id: *asset,
                }),
            )
            .expect("import the existing underfunded reserve identity");
            assert!(
                matches!(response, RootCommandResponseFragment::ImportPoolCanister(
                PoolImportResponse::ResetFailed { canister_id, .. }
            ) if canister_id == *asset)
            );
        }
        let pool = root_pool_status_as(input.pic, input.root, input.operator);
        assert_eq!((pool.ready, pool.failed, pool.pending_reset), (4, 4, 0));
        assert!(pool.pending_creation.is_none());
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one composed full-pool repair binds both lost replies, native conservation and effect-free replay"
    )]
    fn assert_failed_reserve_journey(
        input: &AutonomousFundingJourney<'_>,
        desired: &DesiredFleet,
        source: &str,
        platform: &mut IcpEnsurePlatform,
    ) {
        let counts = || {
            let withdrawals: u64 = input
                .pic
                .query_candid(input.cycles_ledger, "withdrawal_count", ())
                .unwrap();
            let transfers: u64 = input
                .pic
                .query_candid(input.cycles_ledger, "transfer_count", ())
                .unwrap();
            let creates: u64 = input
                .pic
                .query_candid(input.cycles_ledger, "request_count", ())
                .unwrap();
            (withdrawals, transfers, creates)
        };
        let before = counts();
        assert_eq!(before, (0, 0, 7));
        let reviewed = fleet_ensure_workflow::plan(
            input.adapter_root,
            desired,
            source,
            &desired.fleet,
            1_800_000_000_000_000_002,
            platform,
        )
        .expect("review bounded repair of the complete eight-asset inventory");
        let domain = &reviewed.plan.conservation.estate_funding_domains[0];
        assert_eq!(
            (
                domain.allocated_workloads,
                domain.occupied_pool_assets,
                domain.eligible_ready_pool_assets,
                domain.available_pool_slots
            ),
            (4, 8, 0, 0)
        );
        assert_eq!(domain.required_creation_count, 0);
        assert_eq!(domain.maximum_funding_cycles, 0);
        assert_eq!(domain.initial_pool_assets.len(), 8);
        let mut funded = BTreeMap::new();
        let mut resets = BTreeSet::new();
        for action in planned_actions(&reviewed.plan) {
            match action {
                EnsureAction::Fund {
                    principal,
                    amount,
                    pool_funding: Some(root),
                    ..
                } => {
                    assert_eq!(root.root, input.root.to_text());
                    assert!(funded.insert(principal.clone(), *amount).is_none());
                }
                EnsureAction::FleetProtocol { action, .. } => {
                    let CurrentFleetProtocolAction::ReconcilePoolAsset { request, .. } =
                        action.as_ref()
                    else {
                        panic!("full-pool repair must not provision or create assets");
                    };
                    assert!(resets.insert(request.canister_id.to_text()));
                }
                _ => panic!("only exact native funding and reset actions belong to this review"),
            }
        }
        let expected: BTreeSet<_> = input.assets.iter().map(Principal::to_text).collect();
        assert_eq!(funded.keys().cloned().collect::<BTreeSet<_>>(), expected);
        assert_eq!(resets, expected);
        let funding = funded.values().sum::<u128>();
        let debit = funding + 4 * MAINNET_REFILL_LEDGER_FEE;
        assert_eq!(
            reviewed.plan.conservation.maximum_operator_debit_cycles,
            debit
        );
        assert_eq!(
            counts(),
            before,
            "planning cannot debit or create at full capacity"
        );
        std::fs::write(input.adapter_root.join("lose-funding-response"), []).unwrap();
        let lost = fleet_ensure_workflow::apply(
            input.adapter_root,
            desired,
            source,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            platform,
        );
        assert!(
            matches!(lost, Err(EnsureWorkflowError::Platform(_))),
            "lose completed native withdrawal: {lost:?}"
        );
        assert_eq!(counts(), (1, 0, 7));
        std::fs::write(
            input.adapter_root.join("lost-reset-args.bin"),
            encode_one(HostRootCommandFragment::ImportPoolCanister(
                PoolCanisterRequest {
                    canister_id: input.assets[0],
                },
            ))
            .unwrap(),
        )
        .unwrap();
        let mut resumed = literal_zero_journey_platform(
            desired,
            input.icp_wrapper,
            input.adapter_root,
            input.local_replica.clone(),
            true,
        );
        let lost = fleet_ensure_workflow::apply(
            input.adapter_root,
            desired,
            source,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            &mut resumed,
        );
        assert!(
            matches!(lost, Err(EnsureWorkflowError::Platform(_))),
            "lose completed reserve reset: {lost:?}"
        );
        assert!(input.adapter_root.join("lost-reset-response").is_file());
        let mut recovered = literal_zero_journey_platform(
            desired,
            input.icp_wrapper,
            input.adapter_root,
            input.local_replica.clone(),
            true,
        );
        let terminal = fleet_ensure_workflow::apply(
            input.adapter_root,
            desired,
            source,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            &mut recovered,
        )
        .expect("complete the exact four-Failed repair");
        assert!(terminal.terminal);
        let actual = terminal.actual_conservation.as_ref().unwrap();
        assert_eq!(actual.operator_debit_cycles, debit);
        assert_eq!(actual.estate_funding_cycles, 0);
        assert_eq!(actual.exact_estate_creation_fee_cycles, 0);
        assert_eq!(
            actual.observed_starting_cycles
                + actual.operator_debit_cycles
                + actual.observed_net_cycle_credit_cycles,
            actual.final_controlled_cycles
                + actual.observed_net_cycle_debit_cycles
                + actual.exact_unavoidable_fee_cycles
                + actual.exact_estate_creation_fee_cycles
        );
        let pool = root_pool_status_as(input.pic, input.root, input.operator);
        assert_eq!(
            (pool.workload, pool.ready, pool.failed, pool.pending_reset),
            (4, 4, 0, 0)
        );
        assert!(pool.pending_creation.is_none());
        for asset in input.assets {
            let entry = pool
                .entries
                .iter()
                .find(|entry| entry.canister_id == *asset)
                .unwrap();
            assert_eq!(entry.origin, CanisterPoolAssetOrigin::Imported);
            assert!(entry.creation_receipt.is_none());
            let status = input.pic.canister_status(*asset, Some(input.root)).unwrap();
            assert_eq!(status.settings.controllers, vec![input.root]);
            assert!(status.cycles >= input.readiness_floor);
        }
        assert_eq!(counts(), (4, 0, 7));
        assert_eq!(
            ledger_account_balance(input.pic, input.cycles_ledger, input.operator),
            Nat::from(input.operator_after_initial_creation - debit)
        );
        let mut replay_platform = literal_zero_journey_platform(
            desired,
            input.icp_wrapper,
            input.adapter_root,
            input.local_replica.clone(),
            true,
        );
        let replay = fleet_ensure_workflow::apply(
            input.adapter_root,
            desired,
            source,
            &desired.fleet,
            &reviewed.plan.plan_sha256,
            &mut replay_platform,
        )
        .expect("effect-free full-pool repair replay");
        assert!(replay.terminal);
        assert_eq!(replay.effects_applied, 0);
        assert_eq!(counts(), (4, 0, 7));
        assert_eq!(
            ledger_account_balance(input.pic, input.cycles_ledger, input.root),
            Nat::from(0_u8)
        );
        assert_eq!(
            std::fs::read_to_string(input.adapter_root.join("reset-mutations.log"))
                .unwrap()
                .lines()
                .count(),
            1
        );
        let ready_plan = fleet_ensure_workflow::plan(
            input.adapter_root,
            desired,
            source,
            &desired.fleet,
            1_800_000_000_000_000_003,
            &mut replay_platform,
        )
        .expect("forecast the repaired Ready reserve from protected inventory");
        assert!(planned_actions(&ready_plan.plan).is_empty());
        let ready_domain = &ready_plan.plan.conservation.estate_funding_domains[0];
        assert_eq!(ready_domain.eligible_ready_pool_assets, 4);
        assert_eq!(ready_domain.required_creation_count, 0);
        assert_eq!(ready_domain.shortfall_cycles, 0);
        let ready = fleet_ensure_workflow::apply(
            input.adapter_root,
            desired,
            source,
            &desired.fleet,
            &ready_plan.plan.plan_sha256,
            &mut replay_platform,
        )
        .expect("complete effect-free Ready review before operator commands");
        assert!(ready.terminal);
        assert_eq!(ready.effects_applied, 0);
        assert_eq!(counts(), (4, 0, 7));
    }

    #[cfg(test)]
    struct GeneratedJourneyInput<'a> {
        root: &'a Path,
        config: &'a Path,
        icp_wrapper: &'a Path,
        local_replica: &'a LocalReplicaTarget,
        operator: Principal,
        subnet: Principal,
        release_build_id: ReleaseBuildId,
        root_key: &'a [u8],
        workload_count: usize,
        ready_count: usize,
        frontend_identity: Option<Principal>,
    }

    #[cfg(test)]
    fn retain_generated_journey_source(root: &Path, source_config: &Path) -> PathBuf {
        // Keep package-relative paths bound to the same source tree used by the
        // sealed artifacts. Copying only the TOML loses those paths at terminal
        // protocol validation, after the paid deployment has already completed.
        let source_directory = root.join("app-source");
        let source_workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        #[cfg(unix)]
        if source_directory.symlink_metadata().is_ok() {
            assert_eq!(
                std::fs::read_link(&source_directory).unwrap(),
                source_workspace
            );
        } else {
            std::os::unix::fs::symlink(&source_workspace, &source_directory)
                .expect("retain the unchanged App source layout in the isolated workspace");
        }
        let config = source_directory.join(
            source_config
                .strip_prefix(source_workspace)
                .expect("the fixture App config belongs to the Canic workspace"),
        );
        let snapshot = AppConfigSnapshot::load(&config).expect("load the retained App source");
        for role in snapshot.model().roles.keys() {
            let package = canic_host::role_contract::validate_declared_role_package(
                &config,
                snapshot.model(),
                role,
                canic_host::role_contract::PackageValidationMode::Passive,
                &canic_host::role_contract::CargoFeatureSelection::default(),
            );
            let canic_host::role_contract::RolePackageValidation::Supported(evidence) = package
            else {
                panic!("the generated fixture must retain declared package authority: {package:?}");
            };
            let contract = canic_host::role_contract::resolve_declared_role_package_contract(
                snapshot.model(),
                &evidence,
            );
            assert!(
                matches!(
                    contract,
                    canic_core::role_contract::RoleContractResolution::Resolved { .. }
                ),
                "the generated fixture must retain role {} protocol authority: {contract:?}",
                evidence.role
            );
        }
        config
    }

    #[cfg(test)]
    fn generate_journey_desired(input: GeneratedJourneyInput<'_>) -> DesiredFleet {
        let root = input.root;
        let config = retain_generated_journey_source(root, input.config);
        let trust = root.join("pocket-ic-root-key.der");
        std::fs::write(&trust, input.root_key).expect("retain PocketIC trust anchor");
        canic_host::network::enroll_network(canic_host::network::NetworkEnrollmentOptions {
            workspace_root: root,
            environment: "local",
            root_key: &trust,
            fingerprint: &canic_core::cdk::utils::hash::sha256_hex(input.root_key),
        })
        .expect("enroll the exact generator network");
        let source = root.join("fleet-policy.toml");
        let seed = root.join("fleet-seed.toml");
        let mut source_text = generated_journey_policy(
            input.operator,
            input.subnet,
            input.workload_count,
            input.ready_count,
            &config,
        );
        if let Some(caller) = input.frontend_identity {
            let mut policy: toml::Value = toml::from_str(&source_text).unwrap();
            policy["admission"].as_table_mut().unwrap().insert(
                "identity_origin".to_string(),
                toml::Value::String("http://localhost:5173".to_string()),
            );
            policy["admission"]["principals"]
                .as_array_mut()
                .unwrap()
                .push(toml::Value::String(caller.to_text()));
            source_text = toml::to_string_pretty(&policy).unwrap();
        }
        std::fs::write(&source, source_text).expect("write reviewed generator policy");
        canic_host::fleet_ensure::initialize_fresh_estate_seed(
            &canic_host::fleet_ensure::FreshEstateSeedRequest {
                cycles_ledger: "um5iw-rqaaa-aaaaq-qaaba-cai",
                management_creation_fee_cycles: 0,
                seed: &seed,
                source: &source,
            },
        )
        .expect("initialize the owned empty-estate seed");
        // Only the transport route is adapted: generation still queries the real Ledger
        // through ICP CLI and validates the isolated operator and enrolled network.
        let generator_icp = root.join("generator-icp-wrapper");
        let script = format!(
            r#"#!/bin/bash
set -euo pipefail
case " $* " in
  *" canister call "*)
    unset ICP_ENVIRONMENT
    args=()
    while (( $# )); do
      case "$1" in
        -e|--environment) shift 2 ;;
        *) args+=("$1"); shift ;;
      esac
    done
    exec '{}' "${{args[@]}}" -n '{}' -k '{}' ;;
  *) exec '{}' "$@" ;;
esac
"#,
            input.icp_wrapper.display(),
            input.local_replica.url,
            input.local_replica.root_key,
            input.icp_wrapper.display()
        );
        std::fs::write(&generator_icp, script).expect("write generator transport route");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&generator_icp, std::fs::Permissions::from_mode(0o700))
                .expect("enable generator transport route");
        }
        let request = canic_host::fleet_ensure::FleetGenerateRequest {
            catalog_progress: None,
            app_config: &config,
            environment: "local",
            fleet: "canic-121-literal-zero-estate",
            icp_executable: generator_icp.to_str().expect("generator wrapper path"),
            release_build_id: input.release_build_id,
            root,
            seed: &seed,
            source: &source,
        };
        assert_generated_capacity_rejected(&request, input.workload_count, input.ready_count);
        let generated = canic_host::fleet_ensure::generate_desired_fleet(&request)
            .expect("generate the exact desired Fleet through the production boundary");
        assert_eq!(generated.observed_canisters, 0);
        assert_eq!(generated.observed_controlled_cycles, 0);
        assert_eq!(
            generated
                .desired
                .canisters
                .iter()
                .filter(|canister| canister.kind
                    == canic_host::fleet_ensure::model::DesiredCanisterKind::Pool)
                .count(),
            input.workload_count + input.ready_count
        );
        generated.desired
    }

    #[cfg(test)]
    fn assert_generated_capacity_rejected(
        request: &canic_host::fleet_ensure::FleetGenerateRequest<'_>,
        workloads: usize,
        ready: usize,
    ) {
        let original = std::fs::read_to_string(request.source).expect("read reviewed policy");
        let insufficient = original.replace(
            &format!("maximum_size = {}", workloads + ready),
            &format!("maximum_size = {workloads}"),
        );
        assert_ne!(insufficient, original);
        std::fs::write(request.source, insufficient).expect("write capacity-negative policy");
        let Err(error) = canic_host::fleet_ensure::generate_desired_fleet(request) else {
            panic!("Workloads alone cannot satisfy the independent Ready reserve");
        };
        assert!(matches!(
            error,
            canic_host::fleet_ensure::FleetGenerateError::Policy(
                canic_host::fleet_ensure::policy::EnsurePolicyError::TerminalPoolCapacity { .. }
            )
        ));
        std::fs::write(request.source, original).expect("restore exact reviewed fixture policy");
    }

    #[cfg(test)]
    fn generated_journey_policy(
        operator: Principal,
        subnet: Principal,
        workloads: usize,
        ready: usize,
        config_path: &Path,
    ) -> String {
        let configuration = AppConfigSnapshot::load(config_path).unwrap();
        let readiness = configuration
            .model()
            .component_specs
            .values()
            .map(|spec| &spec.initial_cycles)
            .max()
            .unwrap()
            .to_config_string();
        let admissions = configuration
            .model()
            .component_specs
            .keys()
            .map(|spec| format!("{spec} = {}", if workloads == 19 { 19 } else { 1 }))
            .collect::<Vec<_>>()
            .join(", ");
        let capacity = workloads + ready;
        format!(
            r#"
schema_version = 1
funding_profile = "preview_multi_subnet"
operator = "{operator}"

[admission]
principals = ["{operator}"]

[coordinator.subnet]
kind = "explicit"
subnet = "{subnet}"
acknowledge_fiduciary_cost = false

[coordinator.creation_funding]
kind = "cycles"
cycles = "500T"

[coordinator.root_funding]
minimum_reserve_cycles = "210T"
window_secs = 7776000
maximum_cycles = "30T"
maximum_automatic_grants = 2
maximum_automatic_cycles = "60T"

[[fleet_subnet_roots]]
placement_subnet = "{subnet}"
acknowledge_fiduciary_cost = false
component_admissions = {{ {admissions} }}

[fleet_subnet_roots.component_group_placements]
qualification = [0]

[fleet_subnet_roots.canister_pool]
minimum_size = {ready}
maximum_size = {capacity}
canister_cycles = "{readiness}"

[fleet_subnet_roots.root_funding]
request_threshold = "10T"
target_balance = "30T"
cooldown_secs = 2592000
window_secs = 7776000
maximum_cycles = "30T"
maximum_automatic_grants = 2
maximum_automatic_cycles = "60T"

[fleet_subnet_roots.limits]
maximum_component_instances = {workloads}
maximum_registry_bytes = 16777216
maximum_wasm_store_bytes = 40000000
maximum_group_placements = 1

[fleet_subnet_roots.limits.cycles_funding]
window_secs = 3600
maximum_cycles = "100T"

[fleet_subnet_roots.root_creation_funding]
kind = "cycles"
cycles = "80T"

[fleet_subnet_roots.wasm_store_creation_funding]
kind = "cycles"
cycles = "80T"
"#
        )
    }

    #[cfg(test)]
    fn fixture_application_artifact_union(
        root: &Path,
        installed: &InstalledRootFixture,
    ) -> ApplicationArtifactUnion {
        let mut entries = BTreeMap::new();
        for entry in &installed.manifest.entries {
            let artifact = &entry.artifact;
            let compressed = installed
                .artifacts
                .get(&artifact.role)
                .expect("fixture Store artifact");
            let path = root.join(&artifact.wasm_gz_relative_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create fixture artifact parent");
            }
            std::fs::write(path, compressed).expect("write fixture Store artifact");
            let projected = ApplicationArtifactEntry {
                role: artifact.role.clone(),
                package: artifact.package.clone(),
                release_build_id: artifact.release_build_id,
                wasm_relative_path: artifact.wasm_relative_path.clone(),
                wasm_size_bytes: artifact.wasm_size_bytes,
                wasm_sha256_hex: artifact.wasm_sha256_hex.clone(),
                wasm_gz_relative_path: artifact.wasm_gz_relative_path.clone(),
                wasm_gz_size_bytes: artifact.wasm_gz_size_bytes,
                wasm_gz_sha256_hex: artifact.wasm_gz_sha256_hex.clone(),
                candid_sha256: artifact.candid_sha256,
                protocol_profile_digest: artifact.protocol_profile_digest,
            };
            let previous = entries.insert(projected.role.clone(), projected.clone());
            assert!(
                previous
                    .as_ref()
                    .is_none_or(|existing| existing == &projected),
                "one current role must retain one exact artifact"
            );
        }
        ApplicationArtifactUnion {
            release_build_id: installed.manifest.release_build_id,
            fleet_component_topology_digest: installed.manifest.component_topology_digest,
            entries: entries.into_values().collect(),
        }
    }

    #[cfg(test)]
    fn current_protocol_desired(
        configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
        coordinator: Principal,
        root: &FleetSubnetRootInitArgs,
    ) -> DesiredFleet {
        let authority = &root.authority;
        let root_principal = authority.binding.fleet_subnet_root;
        let store = authority.wasm_store_authority.wasm_store;
        let operator = authority.wasm_store_authority.installation_controller;
        let placements = configuration
            .deployment_topology
            .component_group_deployments
            .iter()
            .flat_map(|deployment| {
                (0..deployment.initial_placements).map(move |ordinal| {
                    serde_json::json!({
                        "deployment": deployment.deployment.to_string(),
                        "ordinal": ordinal,
                        "root": "root",
                    })
                })
            })
            .collect::<Vec<_>>();
        let desired = serde_json::json!({
            "canisters": [
                {
                    "controllers": [operator.to_string()],
                    "drain": null,
                    "initial_cycles": "0",
                    "init_arg": null,
                    "init_candid": null,
                    "kind": "coordinator",
                    "minimum_cycles": "0",
                    "name": "coordinator",
                    "parent": null,
                    "presence": "present",
                    "principal": coordinator.to_string(),
                    "replace": false,
                    "subnet": authority.binding.authority.binding.coordinator_subnet.to_string(),
                    "wasm": null,
                },
                {
                    "controllers": [operator.to_string()],
                    "drain": null,
                    "initial_cycles": "0",
                    "init_arg": null,
                    "init_candid": null,
                    "kind": "root",
                    "minimum_cycles": "0",
                    "name": "root",
                    "parent": "coordinator",
                    "presence": "present",
                    "principal": root_principal.to_string(),
                    "replace": false,
                    "subnet": authority.binding.placement_subnet.to_string(),
                    "wasm": null,
                },
                {
                    "controllers": [operator.to_string(), root_principal.to_string()],
                    "drain": null,
                    "initial_cycles": "0",
                    "init_arg": null,
                    "init_candid": null,
                    "kind": "store",
                    "minimum_cycles": "0",
                    "name": "store",
                    "parent": "root",
                    "presence": "present",
                    "principal": store.to_string(),
                    "replace": false,
                    "subnet": authority.binding.placement_subnet.to_string(),
                    "wasm": null,
                }
            ],
            "cycles_ledger": operator.to_string(),
            "environment": "local",
            "fleet": "current-five-component-protocol",
            "ledger_fee_cycles": "0",
            "management_creation_fee_cycles": "0",
            "material_cycle_threshold": "0",
            "maximum_observation_burn_cycles": "0",
            "maximum_stalled_observations": 8,
            "maximum_update_burn_cycles": "0",
            "operator": operator.to_string(),
            "protocol": {
                "app_config": "canic.toml",
                "component_group_placements": placements,
                "coordinator_candid": "coordinator.did",
                "root_candid": "root.did",
                "store_candid": "store.did",
            },
            "schema_version": FLEET_ENSURE_SCHEMA_VERSION,
            "treasury": coordinator.to_string(),
        });
        serde_json::from_value(desired).expect("decode current desired Fleet fixture")
    }

    #[cfg(test)]
    const fn current_protocol_test_stage(action: &CurrentFleetProtocolAction) -> u8 {
        match action {
            CurrentFleetProtocolAction::ReconcilePoolAsset { .. }
            | CurrentFleetProtocolAction::PrepareStoreFixture { .. }
            | CurrentFleetProtocolAction::PublishStoreFixtureChunk { .. }
            | CurrentFleetProtocolAction::PrepareStoreChunkSet { .. }
            | CurrentFleetProtocolAction::PublishStoreChunk { .. }
            | CurrentFleetProtocolAction::StageStoreManifest { .. }
            | CurrentFleetProtocolAction::AdoptStore { .. }
            | CurrentFleetProtocolAction::BootstrapStore { .. } => 0,
            CurrentFleetProtocolAction::JoinRoot { .. } => 1,
            CurrentFleetProtocolAction::SynchronizeRegistry { .. } => 2,
            CurrentFleetProtocolAction::ActivateRegistry { .. } => 3,
            CurrentFleetProtocolAction::ActivateRegistryMirror { .. } => 4,
            CurrentFleetProtocolAction::PrepareComponentRegistry { .. } => 5,
            CurrentFleetProtocolAction::ProvisionComponents { .. } => 6,
            CurrentFleetProtocolAction::MaintainPoolReadiness { .. }
            | CurrentFleetProtocolAction::ObservePoolReadiness { .. } => 7,
        }
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one exhaustive fixture issuer mirrors every closed current protocol action"
    )]
    fn issue_current_protocol_step(
        pic: &PocketIc,
        step: &CompiledCurrentProtocolStep,
        store_controller: Principal,
    ) {
        // These fixtures use the default PocketIC controller for Root and Coordinator;
        // Store calls require their separately installed controller.
        match &step.action {
            CurrentFleetProtocolAction::ObservePoolReadiness { .. } => {}
            CurrentFleetProtocolAction::MaintainPoolReadiness { .. } => {
                root_command(pic, step.target, RootCommandFragment::MaintainPool)
                    .expect("drive the current Root Ready reserve");
            }
            CurrentFleetProtocolAction::ReconcilePoolAsset { request, .. } => {
                root_command(
                    pic,
                    step.target,
                    RootCommandFragment::ImportPoolCanister(*request),
                )
                .expect("reconcile the exact retained pool asset");
            }
            CurrentFleetProtocolAction::ActivateRegistry { request, .. } => {
                let response = coordinator_command(
                    pic,
                    step.target,
                    CoordinatorCommand::ActivateRegistry(request.clone()),
                )
                .expect("activate current Registry");
                assert!(matches!(
                    response,
                    CoordinatorCommandResponse::ActivateRegistry(_)
                ));
            }
            CurrentFleetProtocolAction::ActivateRegistryMirror { request, .. }
            | CurrentFleetProtocolAction::SynchronizeRegistry { request, .. } => {
                let response = root_command(
                    pic,
                    step.target,
                    RootCommandFragment::SynchronizeRegistry(request.clone()),
                )
                .expect("synchronize current Root Registry");
                assert!(matches!(
                    response,
                    RootCommandResponseFragment::OperationAccepted(_)
                ));
            }
            CurrentFleetProtocolAction::AdoptStore { request } => {
                let response = root_command(
                    pic,
                    step.target,
                    RootCommandFragment::AdoptStore(request.clone()),
                )
                .expect("adopt current Store");
                assert!(matches!(
                    response,
                    RootCommandResponseFragment::OperationAccepted(_)
                ));
            }
            CurrentFleetProtocolAction::BootstrapStore { request, .. } => {
                let response = root_command(
                    pic,
                    step.target,
                    RootCommandFragment::BootstrapStore(request.clone()),
                )
                .expect("bootstrap current Store");
                assert!(matches!(
                    response,
                    RootCommandResponseFragment::OperationAccepted(_)
                ));
            }
            CurrentFleetProtocolAction::JoinRoot { request, .. } => {
                let response = coordinator_command(
                    pic,
                    step.target,
                    CoordinatorCommand::JoinRoot(request.clone()),
                )
                .expect("join current Root");
                assert!(matches!(response, CoordinatorCommandResponse::JoinRoot(_)));
            }
            CurrentFleetProtocolAction::PrepareStoreFixture {
                request, source, ..
            } => {
                let response = root_command(
                    pic,
                    step.target,
                    RootCommandFragment::PrepareStoreFixture(request.clone()),
                )
                .unwrap();
                assert!(
                    matches!(response, RootCommandResponseFragment::PrepareStoreFixture(Ok(status)) if status.content_id == source.content_id)
                );
            }
            CurrentFleetProtocolAction::PublishStoreFixtureChunk {
                request, expected, ..
            } => {
                let response: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> = pic
                    .update_candid_as(
                        step.target,
                        store_controller,
                        canic::protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
                        (request.clone(),),
                    )
                    .unwrap();
                let status = response.unwrap().unwrap();
                assert_eq!(status.content_id, expected.content_id);
                assert!(status.next_chunk >= expected.next_chunk);
            }
            CurrentFleetProtocolAction::PrepareStoreChunkSet { request } => {
                store_prepare_as(pic, step.target, store_controller, request.clone())
                    .expect("prepare current Store chunk set");
            }
            CurrentFleetProtocolAction::PrepareComponentRegistry { expected, request } => {
                let response = root_command(
                    pic,
                    step.target,
                    RootCommandFragment::PrepareComponentRegistry(request.clone()),
                )
                .expect("prepare current Component Registry");
                let RootCommandResponseFragment::PrepareComponentRegistry(observed) = response
                else {
                    panic!("Root returned a differently correlated Component Registry response");
                };
                assert!(current_component_registry_progresses(expected, &observed));
            }
            CurrentFleetProtocolAction::ProvisionComponents { request, .. } => {
                let response = coordinator_command(
                    pic,
                    step.target,
                    CoordinatorCommand::ProvisionComponents(request.clone()),
                )
                .expect("provision current Components");
                assert!(matches!(
                    response,
                    CoordinatorCommandResponse::OperationAccepted(_)
                ));
            }
            CurrentFleetProtocolAction::PublishStoreChunk { request } => {
                let response: Result<(), Error> = pic
                    .update_candid_as(
                        step.target,
                        store_controller,
                        canic::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
                        (request.clone(),),
                    )
                    .expect("publish current Store chunk transport");
                response.expect("publish current Store chunk");
            }
            CurrentFleetProtocolAction::StageStoreManifest { request } => {
                store_stage_manifest_as(pic, step.target, store_controller, request.clone())
                    .expect("stage current Store manifest");
            }
        }
    }

    #[cfg(test)]
    fn await_current_protocol_step(
        pic: &PocketIc,
        step: &CompiledCurrentProtocolStep,
        store_controller: Principal,
    ) {
        for _ in 0..160 {
            if current_protocol_step_is_terminal(pic, step, store_controller) {
                return;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        if let CurrentFleetProtocolAction::ProvisionComponents { request, .. } = &step.action {
            let status = coordinator_status(
                pic,
                step.target,
                CoordinatorOperationReadRequest::Operation(OperationStatusRequest {
                    operation_id: request.operation_id,
                }),
            );
            let detail = match status {
                Ok(CoordinatorOperationReadResponse::Operation(
                    CoordinatorOperationStatusResponse::ComponentProvisioning(status),
                )) => format!("{status:?}"),
                Ok(_) => "differently correlated status".to_string(),
                Err(error) => format!("{error:?}"),
            };
            panic!(
                "current protocol Component step did not converge: {}: {detail}",
                step.name,
            );
        }
        panic!("current protocol step did not converge: {}", step.name);
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the governed fixture mirrors every closed current protocol terminal predicate"
    )]
    #[cfg(test)]
    fn current_protocol_step_is_terminal(
        pic: &PocketIc,
        step: &CompiledCurrentProtocolStep,
        store_controller: Principal,
    ) -> bool {
        match &step.action {
            CurrentFleetProtocolAction::MaintainPoolReadiness {
                minimum_ready,
                readiness_floor,
                ..
            }
            | CurrentFleetProtocolAction::ObservePoolReadiness {
                minimum_ready,
                readiness_floor,
            } => {
                let pool = root_pool_status(pic, step.target);
                pool.pending_creation.is_none()
                    && pool
                        .entries
                        .iter()
                        .filter(|asset| {
                            asset.status == CanisterPoolAssetStatus::Ready
                                && asset.cycles >= *readiness_floor
                        })
                        .count()
                        >= *minimum_ready as usize
            }
            CurrentFleetProtocolAction::ReconcilePoolAsset {
                request,
                minimum_cycles,
            } => root_pool_status(pic, step.target)
                .entries
                .iter()
                .any(|asset| {
                    asset.canister_id == request.canister_id
                        && asset.status == CanisterPoolAssetStatus::Ready
                        && asset.cycles >= *minimum_cycles
                }),
            CurrentFleetProtocolAction::ActivateRegistry {
                expected_registry, ..
            }
            | CurrentFleetProtocolAction::JoinRoot {
                expected_registry, ..
            } => matches!(
                coordinator_status(pic, step.target, CoordinatorRegistryRequest::Registry),
                Ok(CoordinatorRegistryResponse::Registry(observed)) if observed == *expected_registry
            ),
            CurrentFleetProtocolAction::ActivateRegistryMirror { expected, request } => {
                matches!(
                    root_status(
                        pic,
                        step.target,
                        RootStatusRequestFragment::Operation(OperationStatusRequest {
                            operation_id: request.operation_id,
                        }),
                    ),
                    Ok(RootStatusResponseFragment::Operation(
                        RootOperationStatusResponse::SynchronizeRegistry(observed)
                    )) if observed.activation.as_ref() == Some(expected)
                )
            }
            CurrentFleetProtocolAction::AdoptStore { request } => matches!(
                root_status(
                    pic,
                    step.target,
                    RootStatusRequestFragment::Operation(OperationStatusRequest {
                        operation_id: request.operation_id,
                    }),
                ),
                Ok(RootStatusResponseFragment::Operation(
                    RootOperationStatusResponse::AdoptStore(observed)
                )) if observed.operation_id == request.operation_id
                    && observed.authority == request.authority
                    && observed.controllers == current_store_controllers(&request.authority)
            ),
            CurrentFleetProtocolAction::BootstrapStore { expected, request } => matches!(
                root_status(
                    pic,
                    step.target,
                    RootStatusRequestFragment::Operation(OperationStatusRequest {
                        operation_id: request.operation_id,
                    }),
                ),
                Ok(RootStatusResponseFragment::Operation(
                    RootOperationStatusResponse::BootstrapStore(observed)
                )) if observed == *expected
            ),
            CurrentFleetProtocolAction::PrepareStoreFixture { source, .. } => {
                let status: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> = pic
                    .query_candid(
                        step.target,
                        canic::protocol::CANIC_ROOT_FIXTURE_STATUS,
                        (source.content_id,),
                    )
                    .unwrap();
                matches!(status, Ok(Ok(status)) if status.content_id == source.content_id
                    && status.chunk_count as usize == source.descriptor.chunks.len())
            }
            CurrentFleetProtocolAction::PublishStoreFixtureChunk { expected, .. } => {
                let response: Result<
                    canic_control_plane::dto::template::StoreCatalogResponse,
                    Error,
                > = pic
                    .query_candid_as(
                        step.target,
                        store_controller,
                        canic::protocol::CANIC_WASM_STORE_CATALOG,
                        (
                            canic_control_plane::dto::template::StoreCatalogRequest::Fixture(
                                expected.content_id,
                            ),
                        ),
                    )
                    .unwrap();
                matches!(response, Ok(canic_control_plane::dto::template::StoreCatalogResponse::Fixture(Ok(status)))
                    if status.content_id == expected.content_id && status.next_chunk >= expected.next_chunk)
            }
            CurrentFleetProtocolAction::PrepareStoreChunkSet { request } => {
                let status = current_store_staging_status(
                    pic,
                    step.target,
                    store_controller,
                    &request.template_id,
                    &request.version,
                );
                status.chunk_set_present
                    && status.expected_chunk_hashes == request.chunk_hashes
                    && status.payload_hash.as_deref() == Some(request.payload_hash.as_slice())
                    && status.payload_size_bytes == Some(request.payload_size_bytes)
            }
            CurrentFleetProtocolAction::PrepareComponentRegistry { expected, request } => {
                matches!(
                    root_status(
                        pic,
                        step.target,
                        RootStatusRequestFragment::ComponentRegistry(request.clone()),
                    ),
                    Ok(RootStatusResponseFragment::ComponentRegistry(observed))
                        if current_component_registry_progresses(expected, &observed)
                )
            }
            CurrentFleetProtocolAction::ProvisionComponents { request, plan_hash } => matches!(
                coordinator_status(
                    pic,
                    step.target,
                    CoordinatorOperationReadRequest::Operation(OperationStatusRequest {
                        operation_id: request.operation_id,
                    }),
                ),
                Ok(CoordinatorOperationReadResponse::Operation(
                    CoordinatorOperationStatusResponse::ComponentProvisioning(observed)
                )) if observed.operation_id == request.operation_id
                    && observed.plan_hash == *plan_hash
                    && observed.phase == FleetComponentProvisioningPhase::RuntimesActivated
                    && observed.published_fleet_registry.is_some()
                    && observed.pending_root_failure.is_none()
            ),
            CurrentFleetProtocolAction::PublishStoreChunk { request } => {
                let status = current_store_staging_status(
                    pic,
                    step.target,
                    store_controller,
                    &request.template_id,
                    &request.version,
                );
                let expected = wasm_hash(&request.bytes);
                status
                    .stored_chunk_hashes
                    .get(request.chunk_index as usize)
                    .is_some_and(|actual| actual.as_ref() == Some(&expected))
            }
            CurrentFleetProtocolAction::StageStoreManifest { request } => {
                let status = current_store_staging_status(
                    pic,
                    step.target,
                    store_controller,
                    &request.template_id,
                    &request.version,
                );
                status.manifest.as_ref() == Some(&current_manifest_response(request))
            }
            CurrentFleetProtocolAction::SynchronizeRegistry { expected, request } => {
                matches!(
                    root_status(
                        pic,
                        step.target,
                        RootStatusRequestFragment::Operation(OperationStatusRequest {
                            operation_id: request.operation_id,
                        }),
                    ),
                    Ok(RootStatusResponseFragment::Operation(
                        RootOperationStatusResponse::SynchronizeRegistry(observed)
                    )) if observed.synchronization == *expected
                )
            }
        }
    }

    #[cfg(test)]
    fn current_store_staging_status(
        pic: &PocketIc,
        store: Principal,
        caller: Principal,
        template_id: &TemplateId,
        version: &TemplateVersion,
    ) -> TemplateStagingStatusResponse {
        let response: Result<StoreCatalogResponse, Error> = pic
            .query_candid_as(
                store,
                caller,
                canic::protocol::CANIC_WASM_STORE_CATALOG,
                (StoreCatalogRequest::Template(TemplateLookupRequest {
                    template_id: template_id.clone(),
                    version: version.clone(),
                }),),
            )
            .expect("query current Store staging transport");
        let StoreCatalogResponse::Template(status) = response.expect("query current Store staging")
        else {
            panic!("Store returned a differently correlated staging status");
        };
        status
    }

    #[cfg(test)]
    fn current_component_registry_progresses(
        expected: &RootComponentRegistryStatusResponse,
        observed: &RootComponentRegistryStatusResponse,
    ) -> bool {
        let authority_matches = observed.fleet_subnet_root == expected.fleet_subnet_root
            && observed.prepared_against_registry == expected.prepared_against_registry
            && observed.release_set == expected.release_set
            && observed.component_topology_digest == expected.component_topology_digest;
        let counters_are_monotonic = observed.next_allocation_sequence
            >= expected.next_allocation_sequence
            && observed.reserved_component_instances >= expected.reserved_component_instances
            && observed.committed_component_instances >= expected.committed_component_instances
            && observed.managed_descendants >= expected.managed_descendants
            && observed.known_created_component_canisters
                >= expected.known_created_component_canisters
            && observed.encoded_bytes >= expected.encoded_bytes;
        authority_matches && counters_are_monotonic
    }

    #[cfg(test)]
    fn current_manifest_response(request: &TemplateManifestInput) -> TemplateManifestResponse {
        TemplateManifestResponse {
            template_id: request.template_id.clone(),
            role: request.role.clone(),
            version: request.version.clone(),
            payload_hash: request.payload_hash.clone(),
            payload_size_bytes: request.payload_size_bytes,
            store_binding: request.store_binding.clone(),
            chunking_mode: request.chunking_mode,
            manifest_state: request.manifest_state,
            approved_at: request.approved_at,
            created_at: request.created_at,
        }
    }

    #[cfg(test)]
    fn current_store_controllers(
        authority: &canic_core::ids::FleetSubnetWasmStoreAuthority,
    ) -> Vec<Principal> {
        let mut controllers = vec![
            authority.fleet_subnet_root,
            authority.installation_controller,
        ];
        controllers.sort();
        controllers
    }

    #[test]
    fn real_coordinator_funds_one_active_root_exactly_once() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_root_funding_journey(false, 30_000_000_000_000);
        let status = await_root_funding(&fixture, |status| status.last_result.is_some());
        let Some(FleetRootFundingResponse::Granted(grant)) = status.last_result.as_ref() else {
            panic!(
                "real Root must retain one Coordinator grant, got {:?}",
                status.last_result
            );
        };
        assert_eq!(grant.request.operation_sequence, 1);
        assert_eq!(status.current_operation, None);
        assert_eq!(status.automatic_grants, 1);
        assert_eq!(status.automatic_cycles, grant.request.granted_cycles);
        assert!(fixture.pic.cycle_balance(fixture.root) > fixture.root_balance_before_activation);

        let CoordinatorObservabilityResponse::Funding(coordinator) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorObservabilityRequest::Funding,
        )
        .expect("query protected Coordinator funding status") else {
            panic!("Coordinator returned a differently correlated funding status");
        };
        let root = coordinator.roots.first().expect("one registered Root");
        assert_eq!(root.last_result, status.last_result);
        assert_eq!(root.automatic_grants, 1);
        assert_eq!(root.window.spent_cycles, grant.request.granted_cycles);
        assert_eq!(root.window.reserved_cycles, Cycles::new(0));

        for _ in 0..8 {
            fixture.pic.advance_time(Duration::from_mins(1));
            fixture.pic.tick();
        }
        let replay_safe = root_funding_status(&fixture.pic, fixture.root);
        assert_eq!(replay_safe.automatic_grants, 1);
        assert_eq!(replay_safe.last_result, status.last_result);
    }

    #[test]
    fn two_roots_use_independent_limits_and_one_coordinator_budget() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_multi_root_funding_journey();

        let mut root_statuses = fixture
            .roots
            .map(|root| root_funding_status(&fixture.pic, root));
        for _ in 0..64 {
            if root_statuses.iter().all(|status| {
                matches!(
                    status.last_result,
                    Some(FleetRootFundingResponse::Granted(_))
                )
            }) {
                break;
            }
            fixture.pic.advance_time(Duration::from_mins(1));
            fixture.pic.tick();
            root_statuses = fixture
                .roots
                .map(|root| root_funding_status(&fixture.pic, root));
        }

        let mut expected_fleet_spend = 0_u128;
        for (root, status) in fixture.roots.into_iter().zip(&root_statuses) {
            let Some(FleetRootFundingResponse::Granted(grant)) = status.last_result.as_ref() else {
                panic!("each registered Root must retain one grant, got {status:?}");
            };
            assert_eq!(status.fleet_subnet_root, root);
            assert_eq!(grant.request.operation_sequence, 1);
            assert_eq!(status.current_operation, None);
            assert_eq!(status.automatic_grants, 1);
            assert_eq!(status.automatic_cycles, grant.request.granted_cycles);
            expected_fleet_spend = expected_fleet_spend
                .checked_add(grant.request.granted_cycles.to_u128())
                .expect("two Root grants fit in u128");
        }

        let CoordinatorObservabilityResponse::Funding(coordinator) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorObservabilityRequest::Funding,
        )
        .expect("query multi-Root Coordinator funding status") else {
            panic!("Coordinator returned a differently correlated funding status");
        };
        assert_eq!(coordinator.automatic_grants, 2);
        assert_eq!(coordinator.automatic_cycles.to_u128(), expected_fleet_spend);
        let fleet_window = coordinator
            .fleet_window
            .expect("active multi-Root Coordinator funding window");
        assert_eq!(fleet_window.spent_cycles.to_u128(), expected_fleet_spend);
        assert_eq!(fleet_window.reserved_cycles, Cycles::new(0));
        assert_eq!(coordinator.roots.len(), 2);
        for root in fixture.roots {
            let status = coordinator
                .roots
                .iter()
                .find(|status| status.fleet_subnet_root == root)
                .expect("registered Root funding projection");
            assert_eq!(status.automatic_grants, 1);
            assert_eq!(status.window.reserved_cycles, Cycles::new(0));
            assert!(status.window.spent_cycles.to_u128() > 0);
        }
    }

    #[test]
    fn automatic_grant_cap_never_renews_after_the_ninety_day_window() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_root_funding_journey_with_policy(
            false,
            30_000_000_000_000,
            |root, coordinator| {
                root.root_funding.maximum_automatic_grants = 1;
                root.root_funding.maximum_automatic_cycles =
                    root.root_funding.target_balance.clone();
                coordinator.maximum_automatic_grants = 1;
                coordinator.maximum_automatic_cycles = root.root_funding.target_balance.clone();
            },
        );
        let granted = await_root_funding(&fixture, |status| {
            matches!(
                status.last_result,
                Some(FleetRootFundingResponse::Granted(_))
            )
        });
        assert_eq!(granted.automatic_grants, 1);
        assert_eq!(granted.root_policy.maximum_automatic_grants, 1);
        let terminal = granted.last_result;

        fixture.pic.advance_time(Duration::from_hours(91 * 24));
        for _ in 0..4 {
            fixture.pic.tick();
        }
        let after_rollover = root_funding_status(&fixture.pic, fixture.root);
        assert_eq!(after_rollover.automatic_grants, 1);
        assert_eq!(after_rollover.root_policy.maximum_automatic_grants, 1);
        assert_eq!(after_rollover.last_result, terminal);
        assert_eq!(after_rollover.current_operation, None);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one real exhausted-generation, staged-rotation and successor-grant journey"
    )]
    fn explicit_policy_rotation_reopens_exhausted_automatic_funding_once() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_root_funding_journey_with_policy(
            false,
            30_000_000_000_000,
            |root, coordinator| {
                root.root_funding.request_threshold = Cycles::new(192_000_000_000_000);
                root.root_funding.target_balance = Cycles::new(385_000_000_000_000);
                root.root_funding.budget.maximum_cycles = root.root_funding.target_balance.clone();
                root.root_funding.maximum_automatic_grants = 1;
                root.root_funding.maximum_automatic_cycles =
                    root.root_funding.target_balance.clone();
                coordinator.maximum_automatic_grants = 1;
                coordinator.maximum_automatic_cycles = root.root_funding.target_balance.clone();
            },
        );
        let predecessor = await_root_funding(&fixture, |status| {
            status.automatic_grants == 1
                && matches!(
                    status.last_result,
                    Some(FleetRootFundingResponse::Granted(_))
                )
        });
        let Some(FleetRootFundingResponse::Granted(first_grant)) = predecessor.last_result.as_ref()
        else {
            panic!("predecessor generation must retain its one real grant");
        };
        assert_eq!(first_grant.request.operation_sequence, 1);
        assert_eq!(
            predecessor.root_policy.request_threshold,
            Cycles::new(192_000_000_000_000)
        );
        assert_eq!(
            predecessor.root_policy.target_balance,
            Cycles::new(385_000_000_000_000)
        );

        fixture.pic.advance_time(Duration::from_hours(91 * 24));
        for _ in 0..4 {
            fixture.pic.tick();
        }
        let still_exhausted = root_funding_status(&fixture.pic, fixture.root);
        assert_eq!(still_exhausted.policy_generation, 1);
        assert_eq!(still_exhausted.automatic_grants, 1);
        assert_eq!(still_exhausted.last_result, predecessor.last_result);

        let plan = current_one_root_rotation_plan(&fixture);
        let plan_digest = fleet_funding_policy_rotation_plan_digest(&plan);
        let operation_id =
            fleet_funding_policy_rotation_operation_id(fixture.coordinator, plan_digest);
        let begin = FleetFundingPolicyRotationBeginRequest {
            operation_id,
            plan_digest,
            header: plan.header.clone(),
        };
        let stage = FleetFundingPolicyRotationStageRootRequest {
            operation_id,
            plan_digest,
            root: plan.roots[0].clone(),
        };
        let apply = FleetFundingPolicyRotationApplyRequest {
            operation_id,
            plan_digest,
            expected_predecessor_generation: plan.header.predecessor_generation,
        };
        assert_rotation_command_accepted(
            &fixture,
            "begin",
            CoordinatorCommand::BeginFundingPolicyRotation(begin.clone()),
            operation_id,
        );
        assert_rotation_command_accepted(
            &fixture,
            "stage Root",
            CoordinatorCommand::StageFundingPolicyRotationRoot(stage.clone()),
            operation_id,
        );
        assert_rotation_command_accepted(
            &fixture,
            "apply",
            CoordinatorCommand::ApplyFundingPolicyRotation(apply.clone()),
            operation_id,
        );

        let terminal = await_policy_rotation(&fixture, operation_id);
        assert_eq!(terminal.predecessor_generation, 1);
        assert_eq!(terminal.successor_generation, 2);
        assert_eq!(terminal.affected_root_count, 1);
        assert_eq!(terminal.retained_historical_automatic_grants, 1);
        assert_eq!(terminal.apply_operator_debit, Cycles::new(0));

        let rotated_root = root_funding_status(&fixture.pic, fixture.root);
        assert_eq!(rotated_root.policy_generation, 2);
        assert_eq!(rotated_root.historical_automatic_grants, 1);
        assert_eq!(rotated_root.automatic_grants, 0);
        assert!(rotated_root.rotation_current.is_none());
        assert!(rotated_root.rotation_last.is_some());
        let rotated_coordinator = coordinator_funding_status(&fixture);
        assert_eq!(rotated_coordinator.policy_generation, 2);
        assert_eq!(rotated_coordinator.historical_automatic_grants, 1);
        assert_eq!(rotated_coordinator.automatic_grants, 0);
        assert_eq!(rotated_coordinator.rotation_checkpoint_count, 1);
        assert_eq!(rotated_coordinator.rotation_checkpoint_root_count, 1);

        assert_rotation_command_accepted(
            &fixture,
            "terminal begin replay",
            CoordinatorCommand::BeginFundingPolicyRotation(begin.clone()),
            operation_id,
        );
        assert_rotation_command_accepted(
            &fixture,
            "terminal stage replay",
            CoordinatorCommand::StageFundingPolicyRotationRoot(stage.clone()),
            operation_id,
        );
        assert_rotation_command_accepted(
            &fixture,
            "terminal apply replay",
            CoordinatorCommand::ApplyFundingPolicyRotation(apply),
            operation_id,
        );
        let mut drifted_begin = begin;
        drifted_begin.header.topology_catalog_digest[0] ^= 1;
        assert!(
            coordinator_command(
                &fixture.pic,
                fixture.coordinator,
                CoordinatorCommand::BeginFundingPolicyRotation(drifted_begin),
            )
            .is_err(),
            "terminal begin replay must reject payload drift"
        );
        let mut drifted_stage = stage;
        drifted_stage.root.placement.node_count += 1;
        assert!(
            coordinator_command(
                &fixture.pic,
                fixture.coordinator,
                CoordinatorCommand::StageFundingPolicyRotationRoot(drifted_stage),
            )
            .is_err(),
            "terminal stage replay must reject payload drift"
        );
        assert_eq!(
            coordinator_funding_status(&fixture).rotation_checkpoint_count,
            1,
            "terminal command replay must not append a second checkpoint"
        );

        let mut drain_sequence = 0_u8;
        loop {
            let observed = root_funding_status(&fixture.pic, fixture.root);
            if observed.automatic_grants == 1
                || fixture.pic.cycle_balance(fixture.root)
                    < rotated_root.root_policy.request_threshold.to_u128()
            {
                break;
            }
            assert!(
                drain_sequence < 40,
                "Root drain must remain bounded: balance={} threshold={} status={observed:?}",
                fixture.pic.cycle_balance(fixture.root),
                rotated_root.root_policy.request_threshold.to_u128(),
            );
            let descendant = if drain_sequence < 20 {
                fixture.descendant
            } else {
                fixture.alternate_descendant
            };
            let transferred = request_descendant_funding(
                &fixture.pic,
                fixture.root,
                descendant,
                descendant_funding_request(
                    &fixture.pic,
                    0x60_u8
                        .checked_add(drain_sequence)
                        .expect("bounded drain request identity"),
                ),
            );
            assert_eq!(transferred, 5_000_000_000_000);
            drain_sequence += 1;
            if fixture.pic.cycle_balance(fixture.root)
                >= rotated_root.root_policy.request_threshold.to_u128()
            {
                fixture.pic.advance_time(Duration::from_secs(61));
                fixture.pic.tick();
            }
        }
        let after_drain = root_funding_status(&fixture.pic, fixture.root);
        assert!(
            fixture.pic.cycle_balance(fixture.root)
                < rotated_root.root_policy.request_threshold.to_u128()
                || after_drain.automatic_grants == 1
        );

        let successor = await_root_funding(&fixture, |status| {
            status.policy_generation == 2
                && status.historical_automatic_grants == 1
                && status.automatic_grants == 1
                && matches!(
                    status.last_result,
                    Some(FleetRootFundingResponse::Granted(ref grant))
                        if grant.request.operation_sequence == 2
                )
        });
        let Some(FleetRootFundingResponse::Granted(successor_grant)) =
            successor.last_result.as_ref()
        else {
            panic!("successor generation must retain its one real grant");
        };
        assert_eq!(successor_grant.request.operation_sequence, 2);
        assert_eq!(successor.historical_automatic_grants, 1);
        assert_eq!(successor.automatic_grants, 1);
        let successor_coordinator = coordinator_funding_status(&fixture);
        assert_eq!(successor_coordinator.historical_automatic_grants, 1);
        assert_eq!(successor_coordinator.automatic_grants, 1);
        assert_eq!(successor_coordinator.rotation_checkpoint_count, 1);
    }

    #[test]
    fn terminal_coordinator_reserve_denial_runs_one_real_icp_fallback() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_system_icp_funding_journey(490_000_000_000_000);
        fixture
            .pic
            .stop_canister(fixture.descendant, Some(fixture.root))
            .expect("stop descendant until the Root refill completes");
        let status = await_root_funding(&fixture, |status| {
            status
                .latest_icp_refill
                .as_ref()
                .is_some_and(|refill| refill.response.status == IcpRefillStatus::Completed)
        });
        let Some(FleetRootFundingResponse::NoGrant(no_grant)) = status.last_result.as_ref() else {
            panic!("Root must retain the terminal Coordinator no-grant");
        };
        assert_eq!(
            no_grant.reason,
            FleetRootFundingNoGrantReason::CoordinatorReserveUnavailable
        );
        let refill = status
            .latest_icp_refill
            .as_ref()
            .expect("one terminal automatic refill");
        assert!(matches!(refill.trigger, IcpRefillTrigger::Automatic { .. }));
        assert_eq!(refill.response.status, IcpRefillStatus::Completed);
        assert!(refill.response.ledger_block_index.is_some());
        assert!(
            refill
                .response
                .cycles_sent
                .as_ref()
                .is_some_and(|cycles| { cycles > &Nat::from(0_u8) })
        );
        assert!(!refill.resumable);
        assert_eq!(status.automatic_grants, 0);
        assert_eq!(status.automatic_icp_refills, 1);
        assert_eq!(status.automatic_icp_refill_e8s, refill.amount_e8s);
        assert!(fixture.pic.cycle_balance(fixture.root) > fixture.root_balance_before_activation);

        let descendant_cycles_before = fixture.pic.cycle_balance(fixture.descendant);
        let request_id = [0x5a; 32];
        let issued_at_ns = fixture.pic.get_time().as_nanos_since_unix_epoch();
        let request = canic::dto::capability::RootCapabilityEnvelopeV1 {
            service: canic::dto::capability::CapabilityService::Root,
            capability_version: canic::dto::capability::CAPABILITY_VERSION_V1,
            capability: canic::dto::rpc::Request::Cycles(canic::dto::rpc::CyclesRequest {
                cycles: 5_000_000_000_000,
                metadata: None,
            }),
            proof: canic::dto::capability::CapabilityProof::Structural,
            metadata: canic::dto::capability::CapabilityRequestMetadata {
                request_id,
                issued_at_ns,
                ttl_ns: 300_000_000_000,
            },
        };
        let transferred = request_descendant_funding(
            &fixture.pic,
            fixture.root,
            fixture.descendant,
            request.clone(),
        );
        assert_eq!(transferred, 5_000_000_000_000);
        let descendant_cycles_after = fixture.pic.cycle_balance(fixture.descendant);
        assert_eq!(
            descendant_cycles_after,
            descendant_cycles_before + transferred
        );
        let replayed =
            request_descendant_funding(&fixture.pic, fixture.root, fixture.descendant, request);
        assert_eq!(replayed, transferred);
        assert_eq!(
            fixture.pic.cycle_balance(fixture.descendant),
            descendant_cycles_after
        );
    }

    #[test]
    fn real_rate_gate_denial_spends_no_icp_and_creates_no_refill() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_system_icp_funding_journey_with_policy(490_000_000_000_000, |policy| {
            policy.min_xdr_permyriad_per_icp = Some(u64::MAX);
        });
        let ledger = Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai")
            .expect("canonical ICP Ledger principal");
        let before_icp = ledger_account_balance(&fixture.pic, ledger, fixture.root);
        let before_cycles = fixture.pic.cycle_balance(fixture.root);

        let denied = await_root_funding(&fixture, |status| {
            matches!(
                status.last_result,
                Some(FleetRootFundingResponse::NoGrant(ref result))
                    if result.reason
                        == FleetRootFundingNoGrantReason::CoordinatorReserveUnavailable
            )
        });
        for _ in 0..8 {
            fixture.pic.advance_time(Duration::from_mins(1));
            fixture.pic.tick();
        }
        let after = root_funding_status(&fixture.pic, fixture.root);
        assert_eq!(after.last_result, denied.last_result);
        assert!(after.latest_icp_refill.is_none());
        assert_eq!(after.automatic_icp_refills, 0);
        assert_eq!(after.automatic_icp_refill_e8s, 0);
        assert_eq!(
            ledger_account_balance(&fixture.pic, ledger, fixture.root),
            before_icp
        );
        assert!(fixture.pic.cycle_balance(fixture.root) <= before_cycles);
    }

    #[test]
    fn insufficient_real_icp_spends_nothing_and_creates_no_refill() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_system_icp_funding_journey_with_balance_and_policy(
            490_000_000_000_000,
            5_000_000,
            |_| {},
        );
        let ledger = Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai")
            .expect("canonical ICP Ledger principal");
        let before_icp = ledger_account_balance(&fixture.pic, ledger, fixture.root);
        let before_cycles = fixture.pic.cycle_balance(fixture.root);

        let denied = await_root_funding(&fixture, |status| {
            matches!(
                status.last_result,
                Some(FleetRootFundingResponse::NoGrant(ref result))
                    if result.reason
                        == FleetRootFundingNoGrantReason::CoordinatorReserveUnavailable
            )
        });
        for _ in 0..8 {
            fixture.pic.advance_time(Duration::from_mins(1));
            fixture.pic.tick();
        }
        let after = root_funding_status(&fixture.pic, fixture.root);
        assert_eq!(after.last_result, denied.last_result);
        assert!(after.latest_icp_refill.is_none());
        assert_eq!(after.automatic_icp_refills, 0);
        assert_eq!(after.automatic_icp_refill_e8s, 0);
        assert_eq!(
            ledger_account_balance(&fixture.pic, ledger, fixture.root),
            before_icp
        );
        assert!(fixture.pic.cycle_balance(fixture.root) <= before_cycles);
    }

    #[test]
    fn uncertain_grant_suppresses_icp_and_direct_topup_remains_available() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_system_icp_funding_journey(30_000_000_000_000);
        let ledger = Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai")
            .expect("canonical ICP Ledger principal");
        let before_icp = ledger_account_balance(&fixture.pic, ledger, fixture.root);
        fixture
            .pic
            .stop_canister(fixture.coordinator, None)
            .expect("stop Coordinator before the first funding request");

        for _ in 0..8 {
            fixture.pic.advance_time(Duration::from_mins(1));
            fixture.pic.tick();
        }
        let status = root_funding_status(&fixture.pic, fixture.root);
        assert!(status.current_operation.is_some());
        assert!(status.last_result.is_none());
        assert!(status.latest_icp_refill.is_none());
        assert_eq!(status.automatic_icp_refills, 0);
        assert_eq!(status.automatic_icp_refill_e8s, 0);
        assert_eq!(
            ledger_account_balance(&fixture.pic, ledger, fixture.root),
            before_icp
        );

        let retained_request = status.current_operation;
        let before_topup = fixture.pic.cycle_balance(fixture.root);
        fixture.pic.add_cycles(fixture.root, 100_000_000_000_000);
        assert_eq!(
            fixture.pic.cycle_balance(fixture.root),
            before_topup + 100_000_000_000_000
        );
        let after_topup = root_funding_status(&fixture.pic, fixture.root);
        assert_eq!(after_topup.current_operation, retained_request);
        assert!(after_topup.last_result.is_none());
        assert!(after_topup.latest_icp_refill.is_none());
    }

    #[test]
    fn production_ledger_and_cmc_exact_replay_never_duplicates_value() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let pic = build_icp_refill_pic();
        let subnet = *pic
            .topology()
            .get_app_subnets()
            .first()
            .expect("one application Subnet");
        let target = pic.create_canister_on_subnet(None, None, subnet);
        pic.add_cycles(target, 1_000_000_000_000);
        let ledger = Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai")
            .expect("canonical ICP Ledger principal");
        let cmc =
            Principal::from_text("rkp4c-7iaaa-aaaaa-aaaca-cai").expect("canonical CMC principal");
        let fee: Nat = pic
            .query_candid(ledger, "icrc1_fee", ())
            .expect("query production ICP Ledger fee");
        let request = QualificationIcrc1TransferArg {
            from_subaccount: None,
            to: QualificationIcrc1Account {
                owner: cmc,
                subaccount: Some(qualification_cmc_topup_subaccount(target)),
            },
            fee: Some(fee),
            created_at_time: Some(pic.get_time().as_nanos_since_unix_epoch()),
            memo: Some(b"TPUP\0\0\0\0".to_vec()),
            amount: Nat::from(100_000_000_u64),
        };
        let first: Result<Nat, QualificationIcrc1TransferError> = pic
            .update_candid(ledger, "icrc1_transfer", (&request,))
            .expect("execute production ICP Ledger transfer");
        let block_index = first.expect("first production transfer must succeed");
        let duplicate: Result<Nat, QualificationIcrc1TransferError> = pic
            .update_candid(ledger, "icrc1_transfer", (&request,))
            .expect("replay production ICP Ledger transfer");
        assert!(matches!(
            duplicate,
            Err(QualificationIcrc1TransferError::Duplicate { duplicate_of })
                if duplicate_of == block_index
        ));

        let block_index = block_index
            .to_string()
            .parse::<u64>()
            .expect("production block index fits u64");
        let notify = QualificationNotifyTopUpArg {
            block_index,
            canister_id: target,
        };
        let cycles_before = pic.cycle_balance(target);
        let first_notify: Result<Nat, QualificationNotifyTopUpError> = pic
            .update_candid(cmc, "notify_top_up", (&notify,))
            .expect("execute production CMC notification");
        let minted = first_notify.expect("first production CMC notification must succeed");
        assert!(minted > 0_u8);
        let cycles_after_first = pic.cycle_balance(target);
        assert!(cycles_after_first > cycles_before);

        let replay: Result<Nat, QualificationNotifyTopUpError> = pic
            .update_candid(cmc, "notify_top_up", (&notify,))
            .expect("replay production CMC notification");
        assert!(replay.is_ok());
        assert_eq!(pic.cycle_balance(target), cycles_after_first);
    }

    #[test]
    fn qualification_ledger_preflight_keeps_1_8_16_32_lanes_independent() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let (_, cycles_ledger_wasm) = build_mainnet_refill_wasms();
        // Protocol warm-up is a separate, excluded cohort.
        assert_qualification_lane_cohort(&cycles_ledger_wasm, 1);
        for width in [1, 8, 16, 32] {
            assert_qualification_lane_cohort(&cycles_ledger_wasm, width);
        }
    }

    #[test]
    fn qualification_reset_preflight_keeps_1_8_16_32_lanes_independent() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workload_wasm = build_qualification_workload_wasm();

        // Each reset journey owns one separate, excluded protocol warm-up.
        assert_qualification_reset_cohort(None, 1);
        assert_qualification_reset_cohort(Some(&workload_wasm), 1);
        for width in [1, 8, 16, 32] {
            assert_qualification_reset_cohort(None, width);
            assert_qualification_reset_cohort(Some(&workload_wasm), width);
        }
    }

    #[test]
    fn qualification_external_effect_envelope_uses_checked_arithmetic() {
        let disposable_assets = qualification_journey_operations(&[1, 8, 16, 32], 3);
        let mainnet_assets = qualification_journey_operations(&[1], 3);

        assert_eq!(disposable_assets, 172);
        assert_eq!(mainnet_assets, 4);
        assert_eq!(
            qualification_funded_exposure(disposable_assets),
            2_590_086_000_000_000
        );
        assert_eq!(
            qualification_funded_exposure(mainnet_assets),
            70_002_000_000_000
        );
    }

    #[test]
    fn qualification_controller_transition_requires_exact_routing_evidence() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let pic = build_pic();
        let subnet = *pic
            .topology()
            .get_app_subnets()
            .first()
            .expect("one application Subnet");
        let source_root = pic.create_canister_on_subnet(None, None, subnet);
        let destination_root = pic.create_canister_on_subnet(None, None, subnet);
        let asset = pic.create_canister_on_subnet(None, None, subnet);
        pic.set_controllers(asset, None, vec![source_root])
            .expect("prepare source-controlled asset");

        assert!(
            qualification_controller_transition(&pic, asset, source_root, destination_root, None,)
                .is_err()
        );
        assert_eq!(
            pic.canister_status(asset, Some(source_root))
                .expect("observe after missing routing evidence")
                .settings
                .controllers,
            vec![source_root]
        );

        assert!(
            qualification_controller_transition(
                &pic,
                asset,
                source_root,
                destination_root,
                Some(Principal::from_slice(&[0x54; 29])),
            )
            .is_err()
        );
        assert_eq!(
            pic.canister_status(asset, Some(source_root))
                .expect("observe after contradictory routing evidence")
                .settings
                .controllers,
            vec![source_root]
        );

        let observations = qualification_controller_transition(
            &pic,
            asset,
            source_root,
            destination_root,
            Some(subnet),
        )
        .expect("exact same-Subnet routing evidence");
        assert_eq!(observations[0], vec![source_root]);
        let mut joint = observations[1].clone();
        joint.sort();
        let mut expected_joint = vec![source_root, destination_root];
        expected_joint.sort();
        assert_eq!(joint, expected_joint);
        assert_eq!(observations[2], vec![destination_root]);
        assert_eq!(pic.get_subnet(source_root), Some(subnet));
        assert_eq!(pic.get_subnet(destination_root), Some(subnet));
        assert_eq!(pic.get_subnet(asset), Some(subnet));
        let terminal = pic
            .canister_status(asset, Some(destination_root))
            .expect("observe destination-controlled asset");
        assert_eq!(terminal.module_hash, None);
        assert_eq!(terminal.settings.controllers, vec![destination_root]);
    }

    #[cfg(test)]
    struct RootFundingJourneyFixture {
        pic: PocketIc,
        coordinator: Principal,
        root: Principal,
        descendant: Principal,
        alternate_descendant: Principal,
        root_balance_before_activation: u128,
    }

    #[cfg(test)]
    struct MultiRootFundingJourneyFixture {
        pic: PocketIc,
        coordinator: Principal,
        roots: [Principal; 2],
        components: [ComponentBinding; 2],
    }

    #[cfg(test)]
    #[derive(CandidType)]
    struct QualificationIcrc1Account {
        owner: Principal,
        subaccount: Option<[u8; 32]>,
    }

    #[cfg(test)]
    #[derive(CandidType)]
    struct QualificationIcrc1TransferArg {
        from_subaccount: Option<[u8; 32]>,
        to: QualificationIcrc1Account,
        fee: Option<Nat>,
        created_at_time: Option<u64>,
        memo: Option<Vec<u8>>,
        amount: Nat,
    }

    #[cfg(test)]
    #[derive(CandidType, Debug, Deserialize)]
    enum QualificationIcrc1TransferError {
        BadBurn { min_burn_amount: Nat },
        BadFee { expected_fee: Nat },
        CreatedInFuture { ledger_time: u64 },
        Duplicate { duplicate_of: Nat },
        GenericError { error_code: Nat, message: String },
        InsufficientFunds { balance: Nat },
        TemporarilyUnavailable,
        TooOld,
    }

    #[cfg(test)]
    #[derive(CandidType)]
    struct QualificationNotifyTopUpArg {
        block_index: u64,
        canister_id: Principal,
    }

    #[cfg(test)]
    #[derive(CandidType, Debug, Deserialize)]
    enum QualificationNotifyTopUpError {
        Refunded {
            block_index: Option<u64>,
            reason: String,
        },
        InvalidTransaction(String),
        Other {
            error_code: u64,
            error_message: String,
        },
        Processing,
        TransactionTooOld(u64),
    }

    #[cfg(test)]
    fn setup_root_funding_journey(
        with_automatic_icp: bool,
        coordinator_reserve_cycles: u128,
    ) -> RootFundingJourneyFixture {
        setup_root_funding_journey_with_policy(
            with_automatic_icp,
            coordinator_reserve_cycles,
            |_, _| {},
        )
    }

    #[cfg(test)]
    fn setup_root_funding_journey_with_policy(
        with_automatic_icp: bool,
        coordinator_reserve_cycles: u128,
        configure: impl FnOnce(
            &mut FleetSubnetRootFundingAuthority,
            &mut FleetCoordinatorRootFundingPolicy,
        ),
    ) -> RootFundingJourneyFixture {
        let root_wasm = build_test_root_wasm();
        let refill_stub_wasm = build_icp_refill_stub_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let store_fixture = build_root_store_fixture();
        let pic = build_pic();
        let subnet = *pic
            .topology()
            .get_app_subnets()
            .first()
            .expect("one application Subnet");
        let ledger = pic.create_canister_on_subnet(None, None, subnet);
        let cmc = pic.create_canister_on_subnet(None, None, subnet);
        pic.add_cycles(ledger, 10_000_000_000_000);
        pic.add_cycles(cmc, 100_000_000_000_000);
        pic.install_canister(
            ledger,
            refill_stub_wasm.clone(),
            encode_one(IcpRefillStubInit::Ledger {
                balance_e8s: 500_000_000,
            })
            .expect("encode ICP Ledger fixture"),
            None,
        );
        pic.install_canister(
            cmc,
            refill_stub_wasm,
            encode_one(IcpRefillStubInit::Cmc {
                xdr_permyriad_per_icp: 1_000_000,
                cycles_per_notify: 50_000_000_000_000,
            })
            .expect("encode CMC fixture"),
            None,
        );

        let coordinator = pic.create_canister_on_subnet(None, None, subnet);
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let mut funding = root_funding_journey_authority(with_automatic_icp, ledger, cmc);
        let mut coordinator_root_funding = FleetCoordinatorRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            minimum_reserve_cycles: Cycles::new(coordinator_reserve_cycles),
            budget: CyclesFundingBudget {
                window_secs: 90 * 24 * 60 * 60,
                maximum_cycles: Cycles::new(450_000_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(880_000_000_000_000),
        };
        configure(&mut funding, &mut coordinator_root_funding);
        let root_fixture = install_bootstrapped_root_on_subnet_with_pool_setup(
            &pic,
            root_wasm,
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: None,
                canister_pool_minimum_size: None,
                canister_pool_cycles: None,
                coordinator_subnet: Some(subnet),
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: Some(subnet),
                component_admission_limits: None,
                fleet_id: None,
                funding: Some(funding),
                coordinator_root_funding: Some(coordinator_root_funding),
            },
            create_prepaid_pool_assets,
        );
        reset_prepaid_pool_assets(&pic, root_fixture.root_id);
        install_fixture_coordinator(&pic, coordinator, coordinator_wasm, &root_fixture);
        let (joining_version, sync_request) =
            join_and_synchronize_root(&pic, coordinator, &root_fixture);
        let root_balance_before_activation = pic.cycle_balance(root_fixture.root_id);
        let active_components = assert_registry_and_root_runtime_activation(
            &pic,
            coordinator,
            &root_fixture,
            joining_version,
            sync_request,
        );
        RootFundingJourneyFixture {
            pic,
            coordinator,
            root: root_fixture.root_id,
            descendant: active_components.issuer.canister_id,
            alternate_descendant: active_components.verifier.canister_id,
            root_balance_before_activation,
        }
    }

    #[cfg(test)]
    fn setup_system_icp_funding_journey(
        coordinator_reserve_cycles: u128,
    ) -> RootFundingJourneyFixture {
        setup_system_icp_funding_journey_with_policy(coordinator_reserve_cycles, |_| {})
    }

    #[cfg(test)]
    fn setup_system_icp_funding_journey_with_policy(
        coordinator_reserve_cycles: u128,
        configure: impl FnOnce(&mut FleetSubnetRootIcpRefillPolicy),
    ) -> RootFundingJourneyFixture {
        setup_system_icp_funding_journey_with_balance_and_policy(
            coordinator_reserve_cycles,
            100_000_000_000,
            configure,
        )
    }

    #[cfg(test)]
    fn setup_system_icp_funding_journey_with_balance_and_policy(
        coordinator_reserve_cycles: u128,
        ledger_balance_e8s: u64,
        configure: impl FnOnce(&mut FleetSubnetRootIcpRefillPolicy),
    ) -> RootFundingJourneyFixture {
        let root_wasm = build_test_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let store_fixture = build_root_store_fixture();
        let pic = build_icp_refill_pic();
        let subnet = *pic
            .topology()
            .get_app_subnets()
            .first()
            .expect("one application Subnet");
        let ledger = Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai")
            .expect("canonical ICP Ledger principal");
        let cmc =
            Principal::from_text("rkp4c-7iaaa-aaaaa-aaaca-cai").expect("canonical CMC principal");

        let coordinator = pic.create_canister_on_subnet(None, None, subnet);
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let mut funding = root_funding_journey_authority(true, ledger, cmc);
        let icp_refill = funding
            .icp_refill
            .as_mut()
            .expect("system ICP journey enables refill policy");
        icp_refill.max_refill_e8s_per_call = 100_000_000_000;
        icp_refill.maximum_refill_e8s = 200_000_000_000;
        icp_refill.min_xdr_permyriad_per_icp = None;
        icp_refill
            .automatic
            .as_mut()
            .expect("system ICP journey enables automatic refill")
            .maximum_automatic_refill_e8s = 200_000_000_000;
        configure(icp_refill);
        let coordinator_root_funding = FleetCoordinatorRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            minimum_reserve_cycles: Cycles::new(coordinator_reserve_cycles),
            budget: CyclesFundingBudget {
                window_secs: 90 * 24 * 60 * 60,
                maximum_cycles: Cycles::new(450_000_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(880_000_000_000_000),
        };
        let root_fixture = install_bootstrapped_root_on_subnet_with_pool_setup(
            &pic,
            root_wasm,
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: None,
                canister_pool_minimum_size: None,
                canister_pool_cycles: None,
                coordinator_subnet: Some(subnet),
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: Some(subnet),
                component_admission_limits: None,
                fleet_id: None,
                funding: Some(funding),
                coordinator_root_funding: Some(coordinator_root_funding),
            },
            create_prepaid_pool_assets,
        );
        fund_real_icp_ledger_account(&pic, ledger, root_fixture.root_id, ledger_balance_e8s);
        reset_prepaid_pool_assets(&pic, root_fixture.root_id);
        install_fixture_coordinator(&pic, coordinator, coordinator_wasm, &root_fixture);
        let (joining_version, sync_request) =
            join_and_synchronize_root(&pic, coordinator, &root_fixture);
        let root_balance_before_activation = pic.cycle_balance(root_fixture.root_id);
        let active_components = assert_registry_and_root_runtime_activation(
            &pic,
            coordinator,
            &root_fixture,
            joining_version,
            sync_request,
        );
        RootFundingJourneyFixture {
            pic,
            coordinator,
            root: root_fixture.root_id,
            descendant: active_components.issuer.canister_id,
            alternate_descendant: active_components.verifier.canister_id,
            root_balance_before_activation,
        }
    }

    #[cfg(test)]
    fn setup_multi_root_funding_journey() -> MultiRootFundingJourneyFixture {
        let root_wasm = build_test_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let pic = build_two_root_pic();
        let mut subnets = pic.topology().get_app_subnets();
        subnets.sort_by_key(|subnet| SubnetId::from_principal(*subnet));
        let [first_subnet, second_subnet] = subnets.as_slice() else {
            panic!("two-Root fixture requires exactly two application Subnets");
        };
        let coordinator = pic.create_canister_on_subnet(None, None, *first_subnet);
        pic.add_cycles(coordinator, 5_000_000_000_000_000);
        let funding = multi_root_funding_authority();
        let coordinator_policy = FleetCoordinatorRootFundingPolicy {
            funding_profile: FleetFundingProfile::MultiSubnet,
            minimum_reserve_cycles: Cycles::new(2_000_000_000_000_000),
            budget: CyclesFundingBudget {
                window_secs: 90 * 24 * 60 * 60,
                maximum_cycles: Cycles::new(2_000_000_000_000_000),
            },
            maximum_automatic_grants: 8,
            maximum_automatic_cycles: Cycles::new(8_000_000_000_000_000),
        };
        let install_root = |subnet| {
            install_bootstrapped_root_on_subnet_with_pool_setup(
                &pic,
                root_wasm.clone(),
                coordinator,
                build_root_store_fixture(),
                BootstrappedRootPlacement {
                    canister_pool_maximum_size: None,
                    canister_pool_minimum_size: None,
                    canister_pool_cycles: None,
                    coordinator_subnet: Some(*first_subnet),
                    existing_root: None,
                    existing_wasm_store: None,
                    root_subnet: Some(subnet),
                    component_admission_limits: Some(RootComponentAdmissionLimits::Uniform(1)),
                    fleet_id: Some(FleetId::from_generated_bytes([0x78; 32])),
                    funding: Some(funding.clone()),
                    coordinator_root_funding: Some(coordinator_policy.clone()),
                },
                create_prepaid_pool_assets,
            )
        };
        let first = install_root(*first_subnet);
        let second = install_root(*second_subnet);
        reset_prepaid_pool_assets(&pic, first.root_id);
        reset_prepaid_pool_assets(&pic, second.root_id);
        install_fixture_coordinator(&pic, coordinator, coordinator_wasm, &first);
        let components = activate_multi_root_registry(&pic, coordinator, [&first, &second]);

        MultiRootFundingJourneyFixture {
            pic,
            coordinator,
            roots: [first.root_id, second.root_id],
            components,
        }
    }

    #[cfg(test)]
    const fn multi_root_funding_authority() -> FleetSubnetRootFundingAuthority {
        FleetSubnetRootFundingAuthority {
            root_funding: FleetSubnetRootFundingPolicy {
                funding_profile: FleetFundingProfile::MultiSubnet,
                request_threshold: Cycles::new(250_000_000_000_000),
                target_balance: Cycles::new(1_000_000_000_000_000),
                cooldown_secs: 30 * 24 * 60 * 60,
                budget: CyclesFundingBudget {
                    window_secs: 90 * 24 * 60 * 60,
                    maximum_cycles: Cycles::new(1_000_000_000_000_000),
                },
                maximum_automatic_grants: 4,
                maximum_automatic_cycles: Cycles::new(4_000_000_000_000_000),
            },
            icp_refill: None,
        }
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "two-Root activation fixture is one ordered proof"
    )]
    fn activate_multi_root_registry(
        pic: &PocketIc,
        coordinator: Principal,
        fixtures: [&BootstrappedRootFixture; 2],
    ) -> [ComponentBinding; 2] {
        let CoordinatorObservabilityResponse::RegistryVersion(mut version) = coordinator_status(
            pic,
            coordinator,
            CoordinatorObservabilityRequest::RegistryVersion,
        )
        .expect("query multi-Root Registry genesis") else {
            panic!("Coordinator returned a differently correlated Registry status");
        };
        for fixture in fixtures {
            let binding = &fixture.init_args.authority.binding;
            let request = FleetSubnetRootJoinRequest {
                expected_registry: version,
                entry: FleetSubnetRootEntry {
                    placement_subnet: binding.placement_subnet,
                    fleet_subnet_root: fixture.root_id,
                    component_admissions: binding.component_admissions.clone(),
                    component_topology_digest: binding.component_topology_digest,
                    active_release_set: fixture.init_args.authority.initial_release_set,
                    funding: binding.funding.clone(),
                    limits: binding.limits.clone(),
                    status: FleetSubnetRootStatus::Joining,
                },
            };
            let CoordinatorCommandResponse::JoinRoot(joined) =
                coordinator_command(pic, coordinator, CoordinatorCommand::JoinRoot(request))
                    .expect("join one Root to the multi-Root Registry")
            else {
                panic!("Coordinator returned a differently correlated join response");
            };
            version = joined.version;
        }

        let sync_requests: [FleetSubnetRootRegistrySyncRequest; 2] =
            std::array::from_fn(|index| FleetSubnetRootRegistrySyncRequest {
                operation_id: [u8::try_from(index + 1).expect("two operation identities fit u8");
                    32],
                expected_registry: version.clone(),
                store_bootstrap: fixtures[index].request.clone(),
            });
        for (fixture, request) in fixtures.into_iter().zip(sync_requests.iter()) {
            let RootCommandResponseFragment::OperationAccepted(receipt) = root_command(
                pic,
                fixture.root_id,
                RootCommandFragment::SynchronizeRegistry(request.clone()),
            )
            .expect("synchronize one Root to the final multi-Root Registry") else {
                panic!("Root returned a differently correlated synchronization response");
            };
            assert_eq!(receipt.operation_id, request.operation_id);
        }

        let CoordinatorCommandResponse::ActivateRegistry(activated) = coordinator_command(
            pic,
            coordinator,
            CoordinatorCommand::ActivateRegistry(FleetRegistryActivationRequest {
                expected_registry: version,
            }),
        )
        .expect("activate the multi-Root Registry") else {
            panic!("Coordinator returned a differently correlated activation response");
        };
        let mut components = Vec::with_capacity(2);
        for (index, (fixture, request)) in fixtures.into_iter().zip(sync_requests).enumerate() {
            let mut mirror_active = false;
            for _ in 0..32 {
                let RootStatusResponseFragment::Operation(
                    RootOperationStatusResponse::SynchronizeRegistry(status),
                ) = root_status(
                    pic,
                    fixture.root_id,
                    RootStatusRequestFragment::Operation(OperationStatusRequest {
                        operation_id: request.operation_id,
                    }),
                )
                .expect("query multi-Root Registry synchronization")
                else {
                    panic!("Root returned a differently correlated synchronization status");
                };
                if status.activation.is_some() {
                    mirror_active = true;
                    break;
                }
                pic.advance_time(Duration::from_secs(1));
                pic.tick();
            }
            assert!(
                mirror_active,
                "each Root must activate the final multi-Root Registry mirror"
            );
            prepare_component_registry(
                pic,
                fixture,
                RootComponentRegistryPreparationRequest {
                    store_bootstrap: fixture.request.clone(),
                    expected_fleet_registry: activated.version.clone(),
                },
            );
            let operation_byte = u8::try_from(index + 3).expect("two operation identities fit u8");
            let component = provision_component(pic, fixture, [operation_byte; 32]);
            assert_eq!(component.allocation_sequence, 1);
            assert_eq!(component.phase, RootComponentAllocationPhase::Committed);
            components.push(installed_component_binding(&component));
            activate_root(pic, fixture.root_id);
        }
        components
            .try_into()
            .expect("two-Root fixture installs exactly two Components")
    }

    #[cfg(test)]
    fn fund_real_icp_ledger_account(
        pic: &PocketIc,
        ledger: Principal,
        owner: Principal,
        amount_e8s: u64,
    ) {
        let transfer: Result<Nat, QualificationIcrc1TransferError> = pic
            .update_candid(
                ledger,
                "icrc1_transfer",
                (QualificationIcrc1TransferArg {
                    from_subaccount: None,
                    to: QualificationIcrc1Account {
                        owner,
                        subaccount: None,
                    },
                    fee: None,
                    created_at_time: None,
                    memo: None,
                    amount: Nat::from(amount_e8s),
                },),
            )
            .expect("fund Root account through the production ICP Ledger");
        transfer.expect("production ICP Ledger transfer must succeed");
    }

    #[cfg(test)]
    fn ledger_account_balance(pic: &PocketIc, ledger: Principal, owner: Principal) -> Nat {
        pic.query_candid(
            ledger,
            "icrc1_balance_of",
            (QualificationIcrc1Account {
                owner,
                subaccount: None,
            },),
        )
        .expect("query the exact ICRC-1 Ledger account balance")
    }

    #[cfg(test)]
    fn qualification_cmc_topup_subaccount(target: Principal) -> [u8; 32] {
        let bytes = target.as_slice();
        assert!(bytes.len() <= 31, "CMC top-up target principal must fit");
        let mut subaccount = [0_u8; 32];
        subaccount[0] = u8::try_from(bytes.len()).expect("principal length fits u8");
        subaccount[1..=bytes.len()].copy_from_slice(bytes);
        subaccount
    }

    #[cfg(test)]
    fn root_funding_journey_authority(
        with_automatic_icp: bool,
        ledger: Principal,
        cmc: Principal,
    ) -> FleetSubnetRootFundingAuthority {
        FleetSubnetRootFundingAuthority {
            root_funding: FleetSubnetRootFundingPolicy {
                funding_profile: FleetFundingProfile::SingleSubnet,
                request_threshold: Cycles::new(210_000_000_000_000),
                target_balance: Cycles::new(450_000_000_000_000),
                cooldown_secs: 30 * 24 * 60 * 60,
                budget: CyclesFundingBudget {
                    window_secs: 90 * 24 * 60 * 60,
                    maximum_cycles: Cycles::new(450_000_000_000_000),
                },
                maximum_automatic_grants: 4,
                maximum_automatic_cycles: Cycles::new(880_000_000_000_000),
            },
            icp_refill: with_automatic_icp.then_some(FleetSubnetRootIcpRefillPolicy {
                max_refill_e8s_per_call: 200_000_000,
                window_secs: 24 * 60 * 60,
                maximum_refill_e8s: 400_000_000,
                minimum_icp_balance_e8s: 10_000_000,
                min_xdr_permyriad_per_icp: Some(500_000),
                ledger_canister_id: Some(ledger),
                cmc_canister_id: Some(cmc),
                allow_ic_system_canister_overrides: true,
                automatic: Some(FleetSubnetRootAutomaticIcpRefillPolicy {
                    emergency_threshold: Cycles::new(200_000_000_000_000),
                    target_balance: Cycles::new(300_000_000_000_000),
                    maximum_automatic_refills: 4,
                    maximum_automatic_refill_e8s: 400_000_000,
                }),
            }),
        }
    }

    #[cfg(test)]
    fn root_funding_status(pic: &PocketIc, root: Principal) -> RootFundingStatusResponse {
        let RootStatusResponseFragment::Funding(status) =
            root_status(pic, root, RootStatusRequestFragment::Funding)
                .expect("query protected Root funding status")
        else {
            panic!("Root returned a differently correlated funding status");
        };
        status
    }

    #[cfg(test)]
    fn await_root_funding(
        fixture: &RootFundingJourneyFixture,
        complete: impl Fn(&RootFundingStatusResponse) -> bool,
    ) -> RootFundingStatusResponse {
        for _ in 0..128 {
            let status = root_funding_status(&fixture.pic, fixture.root);
            if complete(&status) {
                return status;
            }
            fixture.pic.advance_time(Duration::from_mins(1));
            fixture.pic.tick();
        }
        report_canister_diagnostics(
            &fixture.pic,
            fixture.root,
            Principal::anonymous(),
            "Root protected funding journey",
        );
        panic!("Root protected funding journey did not become terminal");
    }

    #[cfg(test)]
    fn coordinator_funding_status(
        fixture: &RootFundingJourneyFixture,
    ) -> CoordinatorFundingStatusResponse {
        let CoordinatorObservabilityResponse::Funding(status) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorObservabilityRequest::Funding,
        )
        .expect("query protected Coordinator funding status") else {
            panic!("Coordinator returned a differently correlated funding status");
        };
        status
    }

    #[cfg(test)]
    fn current_one_root_rotation_plan(
        fixture: &RootFundingJourneyFixture,
    ) -> FleetFundingPolicyRotationPlan {
        let coordinator = coordinator_funding_status(fixture);
        let root = root_funding_status(&fixture.pic, fixture.root);
        let CoordinatorObservabilityResponse::RegistryVersion(predecessor_registry) =
            coordinator_status(
                &fixture.pic,
                fixture.coordinator,
                CoordinatorObservabilityRequest::RegistryVersion,
            )
            .expect("query predecessor Registry version")
        else {
            panic!("Coordinator returned a differently correlated Registry version");
        };
        assert_eq!(coordinator.policy_generation, root.policy_generation);
        assert!(coordinator.rotation.is_none());
        assert!(root.rotation_current.is_none());
        assert!(root.current_operation.is_none());
        let proposed_coordinator_policy = coordinator
            .policy
            .clone()
            .expect("active Coordinator funding policy");
        let predecessor_usage = FleetFundingPolicyUsage {
            historical_automatic_grants: coordinator.historical_automatic_grants,
            historical_automatic_cycles: coordinator.historical_automatic_cycles.clone(),
            generation_automatic_grants: coordinator.automatic_grants,
            generation_automatic_cycles: coordinator.automatic_cycles.clone(),
        };
        let root_usage = FleetFundingPolicyUsage {
            historical_automatic_grants: root.historical_automatic_grants,
            historical_automatic_cycles: root.historical_automatic_cycles.clone(),
            generation_automatic_grants: root.automatic_grants,
            generation_automatic_cycles: root.automatic_cycles.clone(),
        };
        assert_eq!(predecessor_usage, root_usage);
        let placement = |canister| FleetFundingPolicyRotationPlacementEvidence {
            subnet: SubnetId::from_principal(
                fixture
                    .pic
                    .get_subnet(canister)
                    .expect("funding canister placement Subnet"),
            ),
            node_count: 1,
            cost_multiplier_numerator: 1,
            cost_multiplier_denominator: 1,
            fiduciary: false,
            acknowledge_fiduciary_cost: false,
        };
        let roots = vec![FleetFundingPolicyRotationRootPlan {
            fleet_subnet_root: fixture.root,
            predecessor_policy_hash: root.policy_hash,
            predecessor_usage: root_usage,
            proposed_policy: root.root_policy,
            placement: placement(fixture.root),
        }];
        let maximum_new_automatic_cycles =
            proposed_coordinator_policy.maximum_automatic_cycles.clone();
        let mut plan = FleetFundingPolicyRotationPlan {
            header: FleetFundingPolicyRotationPlanHeader {
                predecessor_registry,
                predecessor_generation: coordinator.policy_generation,
                successor_generation: coordinator
                    .policy_generation
                    .checked_add(1)
                    .expect("successor funding generation"),
                predecessor_coordinator_policy_hash: coordinator_root_funding_policy_hash(
                    &proposed_coordinator_policy,
                ),
                predecessor_usage,
                proposed_coordinator_policy,
                topology_catalog_digest: [0; 32],
                coordinator_placement: placement(fixture.coordinator),
                affected_root_count: 1,
                roots_digest: [0; 32],
                maximum_new_automatic_cycles,
                apply_operator_debit: Cycles::new(0),
                funding_source: FleetFundingPolicyRotationFundingSource::CoordinatorTreasury,
            },
            roots,
        };
        plan.header.roots_digest = fleet_funding_policy_rotation_roots_digest(&plan.roots);
        validate_fleet_funding_policy_rotation_plan(&plan).expect("valid live rotation plan");
        plan
    }

    #[cfg(test)]
    fn assert_rotation_command_accepted(
        fixture: &RootFundingJourneyFixture,
        label: &str,
        command: CoordinatorCommand,
        operation_id: [u8; 32],
    ) {
        let response = coordinator_command(&fixture.pic, fixture.coordinator, command)
            .unwrap_or_else(|error| {
                panic!("accept exact funding-policy rotation {label}: {error:?}")
            });
        let CoordinatorCommandResponse::OperationAccepted(receipt) = response else {
            panic!("Coordinator returned a differently correlated rotation response");
        };
        assert_eq!(receipt.operation_id, operation_id);
    }

    #[cfg(test)]
    fn await_policy_rotation(
        fixture: &RootFundingJourneyFixture,
        operation_id: [u8; 32],
    ) -> FleetFundingPolicyRotationReceipt {
        for _ in 0..128 {
            let CoordinatorOperationReadResponse::Operation(
                CoordinatorOperationStatusResponse::FundingPolicyRotation(status),
            ) = coordinator_status(
                &fixture.pic,
                fixture.coordinator,
                CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .expect("query funding-policy rotation status")
            else {
                panic!("Coordinator returned a differently correlated operation status");
            };
            if let FleetFundingPolicyRotationStatusPhase::Completed(receipt) = status.phase {
                return *receipt;
            }
            fixture.pic.advance_time(Duration::from_secs(1));
            fixture.pic.tick();
        }
        report_canister_diagnostics(
            &fixture.pic,
            fixture.coordinator,
            Principal::anonymous(),
            "Coordinator funding-policy rotation journey",
        );
        panic!("Coordinator funding-policy rotation did not become terminal");
    }

    #[cfg(test)]
    fn descendant_funding_request(
        pic: &PocketIc,
        request_byte: u8,
    ) -> canic::dto::capability::RootCapabilityEnvelopeV1 {
        canic::dto::capability::RootCapabilityEnvelopeV1 {
            service: canic::dto::capability::CapabilityService::Root,
            capability_version: canic::dto::capability::CAPABILITY_VERSION_V1,
            capability: canic::dto::rpc::Request::Cycles(canic::dto::rpc::CyclesRequest {
                cycles: 5_000_000_000_000,
                metadata: None,
            }),
            proof: canic::dto::capability::CapabilityProof::Structural,
            metadata: canic::dto::capability::CapabilityRequestMetadata {
                request_id: [request_byte; 32],
                issued_at_ns: pic.get_time().as_nanos_since_unix_epoch(),
                ttl_ns: 300_000_000_000,
            },
        }
    }

    #[cfg(test)]
    const MAINNET_REFILL_READINESS_FLOOR: u128 = 1_900_000_000_000;
    #[cfg(test)]
    const MAINNET_REFILL_EXECUTION_MARGIN: u128 = 1_000_000_000_000;
    #[cfg(test)]
    const MAINNET_REFILL_MANAGEMENT_CREATION_FEE: u128 = 500_000_000_000;
    #[cfg(test)]
    const MAINNET_REFILL_LEDGER_FEE: u128 = 100_000_000;
    #[cfg(test)]
    const MAINNET_REFILL_REPRESENTATIVE_BURN: [u128; 4] = [100_036, 340_125, 250_092, 160_059];

    #[cfg(test)]
    #[derive(Clone, Copy)]
    struct MainnetRefillScenario {
        first_response_pending: bool,
        required_ready_assets: u32,
    }

    #[cfg(test)]
    struct MainnetRefillFixture {
        assets: Vec<Principal>,
        cycles_ledger: Principal,
        pic: PocketIc,
        root: Principal,
    }

    #[cfg(test)]
    fn create_mainnet_refill_pool_and_ledger(
        pic: &PocketIc,
        root: Principal,
        cycles_ledger_wasm: &[u8],
        scenario: MainnetRefillScenario,
    ) -> Vec<Principal> {
        let root_subnet = pic.get_subnet(root).expect("root placement Subnet");
        let assets = MAINNET_REFILL_REPRESENTATIVE_BURN
            .into_iter()
            .take(
                usize::try_from(scenario.required_ready_assets)
                    .expect("bounded Ready-pool requirement"),
            )
            .map(|burn| {
                let asset = pic
                    .create_canister_with_params(
                        None,
                        CreateCanisterParams {
                            cycles: Some(
                                MAINNET_REFILL_READINESS_FLOOR + MAINNET_REFILL_EXECUTION_MARGIN
                                    - burn,
                            ),
                            settings: None,
                            placement: Some(CreateCanisterPlacement::SubnetId(root_subnet)),
                        },
                    )
                    .expect("create returned pool asset with bounded execution burn");
                pic.set_controllers(asset, None, vec![root])
                    .expect("prepare returned pool asset controller");
                asset
            })
            .collect::<Vec<_>>();
        let cycles_ledger = Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai")
            .expect("canonical Cycles Ledger principal");
        pic.create_canister_with_id(None, None, cycles_ledger)
            .expect("create canonical Cycles Ledger stub principal");
        pic.install_canister(
            cycles_ledger,
            cycles_ledger_wasm.to_vec(),
            encode_one(CyclesLedgerStubInitArgs {
                canister_ids: assets.clone(),
                expected_controllers_by_index: None,
                expected_root: root,
                expected_subnet: root_subnet,
                initial_balances: Some(vec![CyclesLedgerStubAccountBalance {
                    balance: Nat::from(
                        u128::from(scenario.required_ready_assets)
                            * (MAINNET_REFILL_READINESS_FLOOR
                                + MAINNET_REFILL_EXECUTION_MARGIN
                                + MAINNET_REFILL_MANAGEMENT_CREATION_FEE
                                + MAINNET_REFILL_LEDGER_FEE),
                    ),
                    owner: root,
                }]),
                pending_first_index: scenario.first_response_pending.then_some(0),
                withdrawal_fee: Some(Nat::from(MAINNET_REFILL_LEDGER_FEE)),
            })
            .expect("encode Cycles Ledger stub init"),
            None,
        );
        assets
    }

    #[cfg(test)]
    fn build_mainnet_refill_fixture(scenario: MainnetRefillScenario) -> MainnetRefillFixture {
        let (root_wasm, cycles_ledger_wasm) = build_mainnet_refill_wasms();
        let _ = build_test_wasm_store_wasm();
        let store_fixture = build_root_store_fixture();
        let pic = build_pic();
        let created_assets = std::cell::RefCell::new(Vec::new());
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = root_canister_config_path(&workspace_root);
        let fixture = install_bootstrapped_root_with_config_and_pool_setup(
            &pic,
            root_wasm,
            Principal::from_slice(&[0x41; 29]),
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: Some(8),
                canister_pool_minimum_size: Some(scenario.required_ready_assets),
                canister_pool_cycles: Some(Cycles::new(MAINNET_REFILL_READINESS_FLOOR)),
                coordinator_subnet: None,
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: None,
                component_admission_limits: None,
                fleet_id: None,
                funding: None,
                coordinator_root_funding: None,
            },
            &config_path,
            |pic, root| {
                let assets =
                    create_mainnet_refill_pool_and_ledger(pic, root, &cycles_ledger_wasm, scenario);
                created_assets.replace(assets);
                Vec::new()
            },
        );
        MainnetRefillFixture {
            assets: created_assets.into_inner(),
            cycles_ledger: Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai")
                .expect("canonical Cycles Ledger principal"),
            pic,
            root: fixture.root_id,
        }
    }

    #[cfg(test)]
    fn converge_mainnet_refill(fixture: &MainnetRefillFixture, required_ready_assets: u32) {
        for _ in 0..required_ready_assets.saturating_mul(4).saturating_add(4) {
            let status = root_pool_status(&fixture.pic, fixture.root);
            if status.ready == required_ready_assets {
                break;
            }
            let RootCommandResponseFragment::MaintainPool(_) = root_command(
                &fixture.pic,
                fixture.root,
                RootCommandFragment::MaintainPool,
            )
            .expect("automatic pool maintenance") else {
                panic!("Root returned a differently correlated pool response");
            };
        }
    }

    #[cfg(test)]
    fn assert_mainnet_refill_result(
        fixture: &MainnetRefillFixture,
        required_ready_assets: u32,
        expected_request_count: u64,
    ) {
        let status = root_pool_status(&fixture.pic, fixture.root);
        assert_eq!(status.ready, required_ready_assets);
        assert_eq!(status.pending_reset, 0);
        for asset in &fixture.assets {
            let entry = status
                .entries
                .iter()
                .find(|entry| entry.canister_id == *asset)
                .expect("automatically created inventory entry");
            assert_eq!(entry.origin, CanisterPoolAssetOrigin::Created);
            assert_eq!(entry.status, CanisterPoolAssetStatus::Ready);
            assert!(entry.cycles.to_u128() >= MAINNET_REFILL_READINESS_FLOOR);
            assert!(
                entry.cycles.to_u128()
                    < MAINNET_REFILL_READINESS_FLOOR + MAINNET_REFILL_EXECUTION_MARGIN
            );
        }
        let request_count: u64 = fixture
            .pic
            .query_candid(fixture.cycles_ledger, "request_count", ())
            .expect("query ledger request count");
        assert_eq!(request_count, expected_request_count);
        let requested_amounts: Vec<Nat> = fixture
            .pic
            .query_candid(fixture.cycles_ledger, "requested_amounts", ())
            .expect("query exact Ledger creation amounts");
        assert_eq!(
            requested_amounts,
            vec![
                Nat::from(
                    MAINNET_REFILL_READINESS_FLOOR
                        + MAINNET_REFILL_EXECUTION_MARGIN
                        + MAINNET_REFILL_MANAGEMENT_CREATION_FEE
                );
                usize::try_from(required_ready_assets).expect("bounded Ready-pool requirement")
            ]
        );

        let RootCommandResponseFragment::MaintainPool(_) = root_command(
            &fixture.pic,
            fixture.root,
            RootCommandFragment::MaintainPool,
        )
        .expect("effect-free terminal pool replay") else {
            panic!("Root returned a differently correlated pool response");
        };
        let replay_request_count: u64 = fixture
            .pic
            .query_candid(fixture.cycles_ledger, "request_count", ())
            .expect("query replayed ledger request count");
        assert_eq!(replay_request_count, expected_request_count);
    }

    #[cfg(test)]
    fn assert_mainnet_refill(
        first_response_pending: bool,
        required_ready_assets: u32,
        expected_request_count: u64,
    ) {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = build_mainnet_refill_fixture(MainnetRefillScenario {
            first_response_pending,
            required_ready_assets,
        });
        converge_mainnet_refill(&fixture, required_ready_assets);
        assert_mainnet_refill_result(&fixture, required_ready_assets, expected_request_count);
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "one bounded harness proves exact lane admission, contradiction and first excess"
    )]
    fn assert_qualification_lane_cohort(cycles_ledger_wasm: &[u8], width: usize) {
        let pic = build_pic();
        let subnet = *pic
            .topology()
            .get_app_subnets()
            .first()
            .expect("one application Subnet");
        let root = Principal::from_slice(&[0x51; 29]);
        let canister_ids = (0..width)
            .map(|_| {
                let canister_id = pic.create_canister_on_subnet(None, None, subnet);
                pic.set_controllers(canister_id, None, vec![root])
                    .expect("prepare one root-controlled lane result");
                canister_id
            })
            .collect::<Vec<_>>();
        let cycles_ledger = pic.create_canister_on_subnet(None, None, subnet);
        pic.install_canister(
            cycles_ledger,
            cycles_ledger_wasm.to_vec(),
            encode_one(CyclesLedgerStubInitArgs {
                canister_ids: canister_ids.clone(),
                expected_controllers_by_index: None,
                expected_root: root,
                expected_subnet: subnet,
                initial_balances: None,
                pending_first_index: Some(0),
                withdrawal_fee: None,
            })
            .expect("encode lane-stub init"),
            None,
        );

        let first = qualification_creation_request(root, subnet, 1);
        let mut wrong_controller = first.clone();
        wrong_controller
            .creation_args
            .as_mut()
            .and_then(|args| args.settings.as_mut())
            .expect("complete controller fixture")
            .controllers = Some(vec![Principal::from_slice(&[0x52; 29])]);
        assert_qualification_generic_error(&pic, cycles_ledger, root, wrong_controller);

        let mut wrong_subnet = first;
        wrong_subnet
            .creation_args
            .as_mut()
            .expect("complete Subnet fixture")
            .subnet_selection = Some(QualificationSubnetSelection::Subnet {
            subnet: Principal::from_slice(&[0x53; 29]),
        });
        assert_qualification_generic_error(&pic, cycles_ledger, root, wrong_subnet);

        let requests = (0..width)
            .map(|index| {
                qualification_creation_request(
                    root,
                    subnet,
                    u64::try_from(index + 1).expect("bounded lane timestamp"),
                )
            })
            .collect::<Vec<_>>();
        let messages = requests
            .iter()
            .map(|request| {
                pic.submit_call(
                    cycles_ledger,
                    root,
                    "create_canister",
                    encode_one(request).expect("encode lane request"),
                )
                .expect("submit independent lane")
            })
            .collect::<Vec<_>>();

        let mut pending_request_index = None;
        let mut completed_canisters = std::collections::BTreeSet::new();
        for (index, message) in messages.into_iter().enumerate() {
            let response = pic.await_call(message).expect("await independent lane");
            let result = decode_one::<
                Result<QualificationCreateCanisterSuccess, QualificationCreateCanisterError>,
            >(&response)
            .expect("decode lane response");
            match result {
                Err(QualificationCreateCanisterError::Duplicate {
                    canister_id: None, ..
                }) => {
                    assert!(
                        pending_request_index.replace(index).is_none(),
                        "exactly one submitted lane may remain pending"
                    );
                }
                Ok(success) => {
                    assert!(success.block_id > 0_u8);
                    assert!(canister_ids.contains(&success.canister_id));
                    assert!(completed_canisters.insert(success.canister_id));
                }
                outcome => panic!("unexpected qualification lane outcome: {outcome:?}"),
            }
        }

        let pending_request_index =
            pending_request_index.expect("one lane must exercise uncertain response recovery");
        let retry: Result<QualificationCreateCanisterSuccess, QualificationCreateCanisterError> =
            pic.update_candid_as(
                cycles_ledger,
                root,
                "create_canister",
                (requests[pending_request_index].clone(),),
            )
            .expect("exact pending-lane retry transport");
        let recovered_canister = match retry {
            Err(QualificationCreateCanisterError::Duplicate {
                canister_id: Some(canister_id),
                ..
            }) => canister_id,
            outcome => panic!("unexpected exact-retry outcome: {outcome:?}"),
        };
        assert!(completed_canisters.insert(recovered_canister));
        assert_eq!(completed_canisters.len(), width);
        assert!(
            canister_ids
                .iter()
                .all(|canister_id| completed_canisters.contains(canister_id))
        );

        let first_excess = qualification_creation_request(
            root,
            subnet,
            u64::try_from(width + 1).expect("bounded excess timestamp"),
        );
        assert_qualification_generic_error(&pic, cycles_ledger, root, first_excess);

        for canister_id in canister_ids {
            assert_eq!(pic.get_subnet(canister_id), Some(subnet));
            let status = pic
                .canister_status(canister_id, Some(root))
                .expect("observe one lane result");
            assert_eq!(status.settings.controllers, vec![root]);
        }

        let request_count: u64 = pic
            .query_candid(cycles_ledger, "request_count", ())
            .expect("query bounded lane request count");
        assert_eq!(
            request_count,
            u64::try_from(width + 4).expect("bounded request count")
        );
    }

    #[cfg(test)]
    fn assert_qualification_reset_cohort(workload_wasm: Option<&[u8]>, width: usize) {
        assert!([1, 8, 16, 32].contains(&width));
        let expected_module_hash = workload_wasm.map(wasm_hash);
        let pic = build_pic();
        let subnet = *pic
            .topology()
            .get_app_subnets()
            .first()
            .expect("one application Subnet");
        let root = pic.create_canister_on_subnet(None, None, subnet);
        let assets = (0..width)
            .map(|_| {
                let asset = pic
                    .create_canister_with_params(
                        None,
                        CreateCanisterParams {
                            cycles: Some(QUALIFICATION_ASSET_CYCLES),
                            settings: None,
                            placement: Some(CreateCanisterPlacement::SubnetId(subnet)),
                        },
                    )
                    .expect("create exact-balance reset asset on selected Subnet");
                pic.set_controllers(asset, None, vec![root])
                    .expect("prepare exact Root-controlled reset asset");
                if let Some(wasm) = workload_wasm {
                    pic.install_canister(
                        asset,
                        wasm.to_vec(),
                        encode_one(()).expect("encode workload init"),
                        Some(root),
                    );
                    pic.tick();
                }
                asset
            })
            .collect::<Vec<_>>();

        for asset in &assets {
            let status = pic
                .canister_status(*asset, Some(root))
                .expect("freeze reset starting observation");
            assert_eq!(format!("{:?}", status.status), "Running");
            assert_eq!(status.settings.controllers, vec![root]);
            assert_eq!(pic.get_subnet(*asset), Some(subnet));
            assert_eq!(
                status.module_hash.as_deref(),
                expected_module_hash.as_deref()
            );
            let balance = pic.cycle_balance(*asset);
            let top_up = QUALIFICATION_ASSET_CYCLES
                .checked_sub(balance)
                .expect("prepared fixture stays below reset starting balance");
            assert_eq!(pic.add_cycles(*asset, top_up), QUALIFICATION_ASSET_CYCLES);
            assert_eq!(pic.cycle_balance(*asset), QUALIFICATION_ASSET_CYCLES);
        }

        let messages = assets
            .iter()
            .map(|asset| {
                pic.submit_call_with_effective_principal(
                    Principal::management_canister(),
                    RawEffectivePrincipal::CanisterId(asset.as_slice().to_vec()),
                    root,
                    "uninstall_code",
                    encode_one(QualificationCanisterIdRecord {
                        canister_id: *asset,
                    })
                    .expect("encode reset lane"),
                )
                .expect("submit independent reset lane")
            })
            .collect::<Vec<_>>();

        for message in messages {
            let response = pic
                .await_call(message)
                .expect("await independent reset lane");
            decode_args::<()>(&response).expect("decode reset lane response");
        }

        for asset in assets {
            let status = pic
                .canister_status(asset, Some(root))
                .expect("observe terminal reset asset");
            assert_eq!(format!("{:?}", status.status), "Running");
            assert_eq!(status.module_hash, None);
            assert_eq!(status.settings.controllers, vec![root]);
            assert_eq!(pic.get_subnet(asset), Some(subnet));
            assert!(pic.cycle_balance(asset) <= QUALIFICATION_ASSET_CYCLES);
        }
    }

    #[cfg(test)]
    fn build_qualification_workload_wasm() -> Vec<u8> {
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let target_dir = test_target_dir(&workspace_root, "estate-qualification-reset");
        let wasms = build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &[QUALIFICATION_WORKLOAD_PACKAGE],
            CanicWasmBuildProfile::Fast,
            &[],
        );
        wasms.wasm(QUALIFICATION_WORKLOAD_PACKAGE)
    }

    #[cfg(test)]
    fn qualification_journey_operations(cohorts: &[u128], repetitions: u128) -> u128 {
        cohorts
            .iter()
            .try_fold(1_u128, |operations, width| {
                width
                    .checked_mul(repetitions)
                    .and_then(|measured| operations.checked_add(measured))
            })
            .expect("qualification operation count must fit u128")
    }

    #[cfg(test)]
    fn qualification_funded_exposure(assets: u128) -> u128 {
        let principal = assets
            .checked_mul(3)
            .and_then(|uses| uses.checked_mul(QUALIFICATION_ASSET_CYCLES))
            .expect("qualification principal exposure must fit u128");
        let fees = assets
            .checked_mul(5)
            .and_then(|rows| rows.checked_mul(QUALIFICATION_FEE_CYCLES))
            .expect("qualification fee exposure must fit u128");
        principal
            .checked_add(fees)
            .and_then(|total| total.checked_add(QUALIFICATION_RESERVE_CYCLES))
            .expect("qualification funded exposure must fit u128")
    }

    #[cfg(test)]
    fn qualification_creation_request(
        root: Principal,
        subnet: Principal,
        created_at_time: u64,
    ) -> QualificationCreateCanisterArgs {
        QualificationCreateCanisterArgs {
            from_subaccount: None,
            created_at_time: Some(created_at_time),
            amount: Nat::from(5_000_000_000_000_u64),
            creation_args: Some(QualificationCmcCreateCanisterArgs {
                settings: Some(QualificationCanisterSettings {
                    controllers: Some(vec![root]),
                    compute_allocation: None,
                    memory_allocation: None,
                    freezing_threshold: None,
                    reserved_cycles_limit: None,
                }),
                subnet_selection: Some(QualificationSubnetSelection::Subnet { subnet }),
            }),
        }
    }

    #[cfg(test)]
    fn assert_qualification_generic_error(
        pic: &PocketIc,
        cycles_ledger: Principal,
        root: Principal,
        request: QualificationCreateCanisterArgs,
    ) {
        let result: Result<QualificationCreateCanisterSuccess, QualificationCreateCanisterError> =
            pic.update_candid_as(cycles_ledger, root, "create_canister", (request,))
                .expect("qualification rejection transport");
        assert!(matches!(
            result,
            Err(QualificationCreateCanisterError::GenericError { .. })
        ));
    }

    #[cfg(test)]
    fn qualification_controller_transition(
        pic: &PocketIc,
        asset: Principal,
        source_root: Principal,
        destination_root: Principal,
        expected_subnet: Option<Principal>,
    ) -> Result<Vec<Vec<Principal>>, &'static str> {
        let expected_subnet = expected_subnet.ok_or("routing evidence is missing")?;
        let actual_subnets = [
            pic.get_subnet(asset),
            pic.get_subnet(source_root),
            pic.get_subnet(destination_root),
        ];
        if actual_subnets != [Some(expected_subnet); 3] {
            return Err("routing evidence contradicts observed placement");
        }

        let initial = pic
            .canister_status(asset, Some(source_root))
            .map_err(|_| "source cannot observe asset")?
            .settings
            .controllers;
        if initial != [source_root] {
            return Err("source controller authority is stale");
        }
        pic.set_controllers(
            asset,
            Some(source_root),
            vec![source_root, destination_root],
        )
        .map_err(|_| "joint controller transition failed")?;
        let joint = pic
            .canister_status(asset, Some(source_root))
            .map_err(|_| "source cannot observe joint authority")?
            .settings
            .controllers;
        pic.set_controllers(asset, Some(source_root), vec![destination_root])
            .map_err(|_| "destination controller transition failed")?;
        let destination = pic
            .canister_status(asset, Some(destination_root))
            .map_err(|_| "destination cannot observe final authority")?
            .settings
            .controllers;
        Ok(vec![initial, joint, destination])
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one PocketIC journey verifies bootstrap, replay, and exact Store reverification"
    )]
    fn prepared_root_bootstraps_and_reverifies_its_exact_local_store() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let root_wasm = build_test_root_wasm();
        let mut store_fixture = build_root_store_fixture();
        let payload = b"reviewed fixture source";
        let descriptor = canic::dto::fixture_provisioning::FixtureDescriptor {
            schema_version: 1,
            format_hash: [1; 32],
            encoded_length: payload.len() as u64,
            chunks: vec![canic::dto::fixture_provisioning::FixtureChunkDescriptor {
                digest: wasm_hash(payload).try_into().unwrap(),
                length: u32::try_from(payload.len()).unwrap(),
            }],
            completion_summary: [3; 32],
        };
        store_fixture
            .manifest
            .fixtures
            .push(canic::dto::root_store::RootStoreFixture {
                role: CanisterRole::new("issuer"),
                content_id:
                    canic_control_plane::api::fixture_content::FixtureContentApi::content_id(
                        &descriptor,
                    )
                    .unwrap(),
                descriptor,
            });
        let pic = build_pic();
        let fixture = install_bootstrapped_root(
            &pic,
            root_wasm,
            Principal::from_slice(&[0x41; 29]),
            store_fixture,
        );

        assert_eq!(fixture.response.fleet_subnet_root, fixture.root_id);
        assert_eq!(
            fixture.response.release_set,
            fixture.init_args.authority.initial_release_set
        );
        let catalog_roles = fixture
            .response
            .catalog
            .iter()
            .map(|entry| entry.role.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            catalog_roles,
            vec![CanisterRole::new("issuer")],
            "root Store catalog must contain the exact canonical application role closure"
        );

        let RootCommandResponseFragment::OperationAccepted(retried) = root_command(
            &pic,
            fixture.root_id,
            RootCommandFragment::BootstrapStore(fixture.request.clone()),
        )
        .expect("root Store bootstrap retry") else {
            panic!("Root returned a differently correlated bootstrap response");
        };
        assert_eq!(retried.operation_id, fixture.request.operation_id);
        let RootStatusResponseFragment::Operation(RootOperationStatusResponse::BootstrapStore(
            observed,
        )) = root_status(
            &pic,
            fixture.root_id,
            RootStatusRequestFragment::Operation(OperationStatusRequest {
                operation_id: fixture.request.operation_id,
            }),
        )
        .expect("root Store status")
        else {
            panic!("Root returned a differently correlated bootstrap status");
        };
        assert_eq!(
            observed, fixture.response,
            "composite status must independently reverify the exact live catalog"
        );

        let payload = b"direct root Store authorization";
        let payload_hash = wasm_hash(payload);
        let prepare = TemplateChunkSetPrepareInput {
            template_id: TemplateId::owned("canary:direct-root-update".to_string()),
            version: TemplateVersion::from(format!(
                "{}-direct-root-update",
                env!("CARGO_PKG_VERSION")
            )),
            payload_hash: payload_hash.clone(),
            payload_size_bytes: payload.len() as u64,
            chunk_hashes: vec![payload_hash.clone()],
        };
        let prepared = store_prepare_as(
            &pic,
            fixture.response.wasm_store,
            fixture.root_id,
            prepare.clone(),
        );
        assert_eq!(
            prepared.expect("direct root Store prepare").chunk_hashes,
            prepare.chunk_hashes
        );

        let denied = store_prepare_as(
            &pic,
            fixture.response.wasm_store,
            Principal::anonymous(),
            prepare,
        );
        assert_eq!(
            denied
                .expect_err("anonymous Store prepare must fail")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );

        let retained_installation_controller = fixture
            .init_args
            .authority
            .wasm_store_authority
            .installation_controller;
        let controller_prepare = TemplateChunkSetPrepareInput {
            template_id: TemplateId::owned("canary:operator-update".to_string()),
            version: TemplateVersion::from(format!(
                "{}-operator-update",
                env!("CARGO_PKG_VERSION")
            )),
            payload_hash: payload_hash.clone(),
            payload_size_bytes: payload.len() as u64,
            chunk_hashes: vec![payload_hash],
        };
        let prepared = store_prepare_as(
            &pic,
            fixture.response.wasm_store,
            retained_installation_controller,
            controller_prepare.clone(),
        );
        assert_eq!(
            prepared
                .expect("retained installation controller must keep Store mutation authority")
                .chunk_hashes,
            controller_prepare.chunk_hashes
        );
        assert_prepared(&pic, fixture.root_id);
    }

    #[test]
    fn co_located_fleets_keep_roots_stores_pools_and_registries_isolated() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let root_wasm = build_test_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let first_store_fixture = build_root_store_fixture();
        let second_store_fixture = build_root_store_fixture();
        let pic = build_pic();
        let subnet = pic
            .topology()
            .get_app_subnets()
            .into_iter()
            .next()
            .expect("co-located Fleet application Subnet");
        let first_coordinator = pic.create_canister_on_subnet(None, None, subnet);
        let second_coordinator = pic.create_canister_on_subnet(None, None, subnet);
        pic.add_cycles(first_coordinator, COORDINATOR_INSTALL_CYCLES);
        pic.add_cycles(second_coordinator, COORDINATOR_INSTALL_CYCLES);
        let first = install_bootstrapped_root_for_fleet_on_subnet(
            &pic,
            root_wasm.clone(),
            first_coordinator,
            first_store_fixture,
            subnet,
            0xa1,
        );
        let second = install_bootstrapped_root_for_fleet_on_subnet(
            &pic,
            root_wasm,
            second_coordinator,
            second_store_fixture,
            subnet,
            0xb2,
        );
        install_fixture_coordinator(&pic, first_coordinator, coordinator_wasm.clone(), &first);
        install_fixture_coordinator(&pic, second_coordinator, coordinator_wasm, &second);

        assert_co_located_physical_authority(&pic, subnet, &first, &second);
        assert_foreign_root_cannot_write_store(&pic, &first, &second);

        let (first_joined, _) = join_and_synchronize_root(&pic, first_coordinator, &first);
        let foreign_sync = FleetSubnetRootRegistrySyncRequest {
            operation_id: [21; 32],
            expected_registry: first_joined,
            store_bootstrap: second.request.clone(),
        };
        let rejected = root_command(
            &pic,
            second.root_id,
            RootCommandFragment::SynchronizeRegistry(foreign_sync),
        );
        assert_eq!(
            application_rejection(
                rejected,
                "another Fleet's Registry must not enter the co-located root",
            )
            .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );
        let _ = join_and_synchronize_root(&pic, second_coordinator, &second);
        assert_isolated_coordinator_registry(
            &pic,
            first_coordinator,
            std::slice::from_ref(&first),
            &second,
        );
        assert_isolated_coordinator_registry(
            &pic,
            second_coordinator,
            std::slice::from_ref(&second),
            &first,
        );
    }

    #[cfg(test)]
    fn assert_co_located_physical_authority(
        pic: &PocketIc,
        subnet: Principal,
        first: &BootstrappedRootFixture,
        second: &BootstrappedRootFixture,
    ) {
        assert_ne!(first.root_id, second.root_id);
        assert_ne!(first.response.wasm_store, second.response.wasm_store);
        assert_ne!(
            first.init_args.authority.binding.authority.binding.fleet,
            second.init_args.authority.binding.authority.binding.fleet
        );
        for fixture in [first, second] {
            assert_eq!(pic.get_subnet(fixture.root_id), Some(subnet));
            assert_eq!(pic.get_subnet(fixture.response.wasm_store), Some(subnet));
            assert_root_local_physical_inventory(pic, fixture);
        }
        let first_pool = first
            .init_args
            .canister_pool_imports
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let second_pool = second
            .init_args
            .canister_pool_imports
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert!(first_pool.is_disjoint(&second_pool));
    }

    #[cfg(test)]
    fn assert_foreign_root_cannot_write_store(
        pic: &PocketIc,
        owner: &BootstrappedRootFixture,
        foreign: &BootstrappedRootFixture,
    ) {
        let payload = b"co-located Fleet Store authority";
        let payload_hash = wasm_hash(payload);
        let request = TemplateChunkSetPrepareInput {
            template_id: TemplateId::owned("canary:co-located-fleet".to_string()),
            version: TemplateVersion::from(format!(
                "{}-fleet-isolation",
                env!("CARGO_PKG_VERSION")
            )),
            payload_hash: payload_hash.clone(),
            payload_size_bytes: payload.len() as u64,
            chunk_hashes: vec![payload_hash],
        };
        let rejected = store_prepare_as(
            pic,
            owner.response.wasm_store,
            foreign.root_id,
            request.clone(),
        );
        assert_eq!(
            rejected
                .expect_err("another Fleet's co-located root must not write this Store")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );
        let accepted = store_prepare_as(pic, owner.response.wasm_store, owner.root_id, request);
        accepted.expect("owning root retains Store update authority");
    }

    #[cfg(test)]
    fn assert_isolated_coordinator_registry(
        pic: &PocketIc,
        coordinator: Principal,
        owned: &[BootstrappedRootFixture],
        foreign: &BootstrappedRootFixture,
    ) {
        let CoordinatorRegistryResponse::Registry(registry) =
            coordinator_status(pic, coordinator, CoordinatorRegistryRequest::Registry)
                .expect("query isolated Coordinator Registry");
        assert_eq!(
            registry.authority.binding.fleet,
            owned[0].init_args.authority.binding.authority.binding.fleet
        );
        assert_eq!(registry.fleet_subnet_roots.len(), owned.len());
        let registered = registry
            .fleet_subnet_roots
            .iter()
            .map(|entry| entry.fleet_subnet_root)
            .collect::<std::collections::BTreeSet<_>>();
        let expected = owned
            .iter()
            .map(|fixture| fixture.root_id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(registered, expected);
        assert!(
            registry
                .fleet_subnet_roots
                .iter()
                .all(|entry| entry.fleet_subnet_root != foreign.root_id)
        );
    }

    #[test]
    fn active_registry_issues_component_role_attestations() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        for _ in 0..2 {
            let fixture = acquire_active_component_registry();
            super::super::role_attestation::assert_registry_bound_role_attestation(
                fixture.pic(),
                fixture.root,
                &fixture.issuer,
                &fixture.verifier,
            );
        }
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one complete real-Fleet add/remove journey"
    )]
    fn fleet_admission_add_and_remove_converge_across_real_root_and_components() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let pic = fixture.pic();
        let added = Principal::self_authenticating([0xd1; 32]);

        let CoordinatorRegistryResponse::Registry(initial_registry) = coordinator_status(
            pic,
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .expect("query initial Fleet Registry");
        assert!(!initial_registry.admission.fleet_principals.contains(&added));

        let mut added_principals = initial_registry.admission.fleet_principals.clone();
        added_principals.push(added);
        added_principals.sort_unstable();
        let added_policy = compile_installed_fleet_admission_policy(
            initial_registry.admission.fleet.clone(),
            initial_registry.admission.generation + 1,
            added_principals,
            initial_registry.admission.rules.clone(),
        )
        .expect("compile added admission policy");
        let add_operation_id = [0xd2; 32];
        let (participant_catalog_digest, participant_count) =
            admission_participant_catalog_authority(pic, &[fixture.root], &added_policy);
        let add_request = FleetAdmissionMutationRequest {
            authority: initial_registry.authority.binding.clone(),
            expected_generation: initial_registry.admission.generation,
            expected_policy_digest: initial_registry.admission.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: add_operation_id,
            successor_policy_digest: added_policy.policy_digest,
            participant_catalog_digest,
            participant_count,
        };
        let CoordinatorCommandResponse::MutateAdmission(planned_add) = coordinator_command(
            pic,
            fixture.coordinator,
            CoordinatorCommand::MutateAdmission(add_request.clone()),
        )
        .expect("plan Fleet admission addition") else {
            panic!("Coordinator returned a differently correlated admission response")
        };
        assert_eq!(planned_add.outcome, FleetAdmissionMutationOutcome::Planned);

        let completed_add =
            await_fleet_admission_convergence(pic, fixture.coordinator, add_operation_id);
        assert_eq!(
            completed_add.outcome,
            FleetAdmissionMutationOutcome::Converged
        );
        assert_eq!(completed_add.generation, added_policy.generation);
        assert_eq!(completed_add.policy_digest, added_policy.policy_digest);

        for target in [fixture.issuer.canister_id, fixture.verifier.canister_id] {
            let status = managed_admission_status(pic, target, fixture.root);
            assert_eq!(status.phase, FleetAdmissionProjectionPhase::Open);
            assert_eq!(status.generation, added_policy.generation);
            assert_eq!(status.policy_digest, added_policy.policy_digest);
            assert!(status.principals.entries.contains(&added));
            assert!(status.prepared.is_none());
        }

        let CoordinatorCommandResponse::MutateAdmission(replayed_add) = coordinator_command(
            pic,
            fixture.coordinator,
            CoordinatorCommand::MutateAdmission(add_request),
        )
        .expect("replay completed Fleet admission addition") else {
            panic!("Coordinator returned a differently correlated admission replay")
        };
        assert_eq!(
            replayed_add.outcome,
            FleetAdmissionMutationOutcome::Converged
        );
        assert_eq!(replayed_add.operation_id, add_operation_id);

        let CoordinatorRegistryResponse::Registry(added_registry) = coordinator_status(
            pic,
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .expect("query added Fleet Registry");
        assert_eq!(added_registry.admission, added_policy);

        let mut removed_principals = added_registry.admission.fleet_principals.clone();
        removed_principals.retain(|principal| *principal != added);
        let removed_policy = compile_installed_fleet_admission_policy(
            added_registry.admission.fleet.clone(),
            added_registry.admission.generation + 1,
            removed_principals,
            added_registry.admission.rules.clone(),
        )
        .expect("compile removed admission policy");
        let remove_operation_id = [0xd3; 32];
        let (participant_catalog_digest, participant_count) =
            admission_participant_catalog_authority(pic, &[fixture.root], &removed_policy);
        let remove_request = FleetAdmissionMutationRequest {
            authority: added_registry.authority.binding.clone(),
            expected_generation: added_registry.admission.generation,
            expected_policy_digest: added_registry.admission.policy_digest,
            action: FleetAdmissionMutationAction::Remove,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: remove_operation_id,
            successor_policy_digest: removed_policy.policy_digest,
            participant_catalog_digest,
            participant_count,
        };
        let CoordinatorCommandResponse::MutateAdmission(planned_remove) = coordinator_command(
            pic,
            fixture.coordinator,
            CoordinatorCommand::MutateAdmission(remove_request),
        )
        .expect("plan Fleet admission removal") else {
            panic!("Coordinator returned a differently correlated admission response")
        };
        assert_eq!(
            planned_remove.outcome,
            FleetAdmissionMutationOutcome::Planned
        );

        let completed_remove =
            await_fleet_admission_convergence(pic, fixture.coordinator, remove_operation_id);
        assert_eq!(
            completed_remove.outcome,
            FleetAdmissionMutationOutcome::Converged
        );
        assert_eq!(completed_remove.generation, removed_policy.generation);
        assert_eq!(completed_remove.policy_digest, removed_policy.policy_digest);

        for target in [fixture.issuer.canister_id, fixture.verifier.canister_id] {
            let status = managed_admission_status(pic, target, fixture.root);
            assert_eq!(status.phase, FleetAdmissionProjectionPhase::Open);
            assert_eq!(status.generation, removed_policy.generation);
            assert_eq!(status.policy_digest, removed_policy.policy_digest);
            assert!(!status.principals.entries.contains(&added));
            assert!(status.prepared.is_none());
        }

        let CoordinatorRegistryResponse::Registry(removed_registry) = coordinator_status(
            pic,
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .expect("query removed Fleet Registry");
        drop(fixture);
        assert_eq!(removed_registry.admission, removed_policy);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "unavailability and post-convergence creation share one Fleet"
    )]
    fn unavailable_admission_participant_blocks_activation_until_exact_retry() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let pic = fixture.pic();
        let added = Principal::self_authenticating([0xd4; 32]);
        let operation_id = [0xd5; 32];

        let CoordinatorRegistryResponse::Registry(initial_registry) = coordinator_status(
            pic,
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .expect("query initial Fleet Registry");
        let mut successor_principals = initial_registry.admission.fleet_principals.clone();
        successor_principals.push(added);
        successor_principals.sort_unstable();
        let successor = compile_installed_fleet_admission_policy(
            initial_registry.admission.fleet.clone(),
            initial_registry.admission.generation + 1,
            successor_principals,
            initial_registry.admission.rules.clone(),
        )
        .expect("compile admission successor");
        let (participant_catalog_digest, participant_count) =
            admission_participant_catalog_authority(pic, &[fixture.root], &successor);
        let request = FleetAdmissionMutationRequest {
            authority: initial_registry.authority.binding.clone(),
            expected_generation: initial_registry.admission.generation,
            expected_policy_digest: initial_registry.admission.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id,
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest,
            participant_count,
        };

        pic.stop_canister(fixture.verifier.canister_id, Some(fixture.root))
            .expect("stop one managed admission participant");
        let CoordinatorCommandResponse::MutateAdmission(planned) = coordinator_command(
            pic,
            fixture.coordinator,
            CoordinatorCommand::MutateAdmission(request),
        )
        .expect("plan admission mutation with unavailable participant") else {
            panic!("Coordinator returned a differently correlated admission response")
        };
        assert_eq!(planned.outcome, FleetAdmissionMutationOutcome::Planned);

        for _ in 0..16 {
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        let CoordinatorOperationReadResponse::Operation(
            CoordinatorOperationStatusResponse::Admission(blocked),
        ) = coordinator_status(
            pic,
            fixture.coordinator,
            CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
        )
        .expect("query blocked admission operation")
        else {
            panic!("Coordinator returned a differently correlated admission operation")
        };
        assert!(matches!(
            blocked.phase,
            FleetAdmissionOperationPhase::Planned { .. }
                | FleetAdmissionOperationPhase::Preparing { .. }
        ));
        let CoordinatorRegistryResponse::Registry(blocked_registry) = coordinator_status(
            pic,
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .expect("query blocked Fleet Registry");
        assert_eq!(blocked_registry.admission, initial_registry.admission);

        let reachable = managed_admission_status(pic, fixture.issuer.canister_id, fixture.root);
        assert_eq!(reachable.generation, initial_registry.admission.generation);
        assert_eq!(
            reachable.policy_digest,
            initial_registry.admission.policy_digest
        );
        match reachable.phase {
            FleetAdmissionProjectionPhase::Open => assert!(reachable.prepared.is_none()),
            FleetAdmissionProjectionPhase::Fenced => {
                let prepared = reachable.prepared.expect("prepared successor while fenced");
                assert_eq!(prepared.generation, successor.generation);
                assert_eq!(prepared.policy_digest, successor.policy_digest);
            }
        }
        let allocation_request = RootComponentAllocationRequest {
            operation_id: [0xd6; 32],
            component_spec: fixture.issuer.component_spec.clone(),
        };
        let rejected = application_rejection(
            root_command(
                pic,
                fixture.root,
                RootCommandFragment::ProvisionComponent(allocation_request.clone()),
            ),
            "active admission transition must fence Component allocation",
        );
        assert_eq!(
            rejected.code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );

        pic.start_canister(fixture.verifier.canister_id, Some(fixture.root))
            .expect("restart managed admission participant");
        let completed = await_fleet_admission_convergence(pic, fixture.coordinator, operation_id);
        assert_eq!(completed.outcome, FleetAdmissionMutationOutcome::Converged);
        assert_eq!(completed.generation, successor.generation);
        for target in [fixture.issuer.canister_id, fixture.verifier.canister_id] {
            let status = managed_admission_status(pic, target, fixture.root);
            assert_eq!(status.phase, FleetAdmissionProjectionPhase::Open);
            assert_eq!(status.generation, successor.generation);
            assert_eq!(status.policy_digest, successor.policy_digest);
            assert!(status.principals.entries.contains(&added));
        }
        let new_component = provision_component_request(pic, fixture.root, allocation_request);
        let new_binding = installed_component_binding(&new_component);
        let new_status = managed_admission_status(pic, new_binding.canister_id, fixture.root);
        drop(fixture);
        assert_eq!(new_status.phase, FleetAdmissionProjectionPhase::Open);
        assert_eq!(new_status.generation, successor.generation);
        assert_eq!(new_status.policy_digest, successor.policy_digest);
        assert!(new_status.principals.entries.contains(&added));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "the stale-catalog release and corrected retry form one recovery proof"
    )]
    fn fleet_admission_catalog_change_releases_before_effect_and_retries_exactly() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let pic = fixture.pic();
        let added = Principal::self_authenticating([0xe0; 32]);

        let CoordinatorRegistryResponse::Registry(initial_registry) = coordinator_status(
            pic,
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .expect("query initial Fleet Registry");
        let mut successor_principals = initial_registry.admission.fleet_principals.clone();
        successor_principals.push(added);
        successor_principals.sort_unstable();
        let successor = compile_installed_fleet_admission_policy(
            initial_registry.admission.fleet.clone(),
            initial_registry.admission.generation + 1,
            successor_principals,
            initial_registry.admission.rules.clone(),
        )
        .expect("compile admission successor");
        let stale_catalog =
            admission_participant_catalog_authority(pic, &[fixture.root], &successor);

        let new_component = provision_component_request(
            pic,
            fixture.root,
            RootComponentAllocationRequest {
                operation_id: [0xe1; 32],
                component_spec: fixture.issuer.component_spec.clone(),
            },
        );
        let new_binding = installed_component_binding(&new_component);
        let new_status = managed_admission_status(pic, new_binding.canister_id, fixture.root);
        assert_eq!(new_status.phase, FleetAdmissionProjectionPhase::Open);
        assert_eq!(new_status.generation, initial_registry.admission.generation);

        let stale_operation_id = [0xe2; 32];
        let stale_request = FleetAdmissionMutationRequest {
            authority: initial_registry.authority.binding.clone(),
            expected_generation: initial_registry.admission.generation,
            expected_policy_digest: initial_registry.admission.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: stale_operation_id,
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest: stale_catalog.0,
            participant_count: stale_catalog.1,
        };
        let CoordinatorCommandResponse::MutateAdmission(planned) = coordinator_command(
            pic,
            fixture.coordinator,
            CoordinatorCommand::MutateAdmission(stale_request.clone()),
        )
        .expect("retain mutation with pre-reservation catalog") else {
            panic!("Coordinator returned a differently correlated admission response")
        };
        assert_eq!(planned.outcome, FleetAdmissionMutationOutcome::Planned);
        let released =
            await_fleet_admission_convergence(pic, fixture.coordinator, stale_operation_id);
        assert_eq!(
            released.outcome,
            FleetAdmissionMutationOutcome::CatalogChanged
        );
        assert_eq!(released.generation, initial_registry.admission.generation);
        assert_eq!(
            released.policy_digest,
            initial_registry.admission.policy_digest
        );
        let CoordinatorCommandResponse::MutateAdmission(replayed_release) = coordinator_command(
            pic,
            fixture.coordinator,
            CoordinatorCommand::MutateAdmission(stale_request),
        )
        .expect("replay released stale-catalog mutation") else {
            panic!("Coordinator returned a differently correlated admission response")
        };
        assert_eq!(replayed_release, released);

        let CoordinatorRegistryResponse::Registry(unchanged_registry) = coordinator_status(
            pic,
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .expect("query Registry after stale-catalog release");
        assert_eq!(unchanged_registry.admission, initial_registry.admission);
        for target in [
            fixture.issuer.canister_id,
            fixture.verifier.canister_id,
            new_binding.canister_id,
        ] {
            let status = managed_admission_status(pic, target, fixture.root);
            assert_eq!(status.phase, FleetAdmissionProjectionPhase::Open);
            assert_eq!(status.generation, initial_registry.admission.generation);
            assert!(status.prepared.is_none());
        }

        let exact_catalog =
            admission_participant_catalog_authority(pic, &[fixture.root], &successor);
        assert_ne!(exact_catalog, stale_catalog);
        let exact_operation_id = [0xe3; 32];
        let exact_request = FleetAdmissionMutationRequest {
            authority: initial_registry.authority.binding,
            expected_generation: initial_registry.admission.generation,
            expected_policy_digest: initial_registry.admission.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: exact_operation_id,
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest: exact_catalog.0,
            participant_count: exact_catalog.1,
        };
        let CoordinatorCommandResponse::MutateAdmission(retry) = coordinator_command(
            pic,
            fixture.coordinator,
            CoordinatorCommand::MutateAdmission(exact_request),
        )
        .expect("retry mutation with exact reserved catalog") else {
            panic!("Coordinator returned a differently correlated admission response")
        };
        assert_eq!(retry.outcome, FleetAdmissionMutationOutcome::Planned);
        let converged =
            await_fleet_admission_convergence(pic, fixture.coordinator, exact_operation_id);
        assert_eq!(converged.outcome, FleetAdmissionMutationOutcome::Converged);
        assert_eq!(converged.generation, successor.generation);
        for target in [
            fixture.issuer.canister_id,
            fixture.verifier.canister_id,
            new_binding.canister_id,
        ] {
            let status = managed_admission_status(pic, target, fixture.root);
            assert_eq!(status.phase, FleetAdmissionProjectionPhase::Open);
            assert_eq!(status.generation, successor.generation);
            assert!(status.principals.entries.contains(&added));
        }
        drop(fixture);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "two-Root restart and add/remove journey is indivisible"
    )]
    fn fleet_admission_add_and_remove_converge_across_two_roots() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_multi_root_funding_journey();
        let added = Principal::self_authenticating([0xd9; 32]);
        let CoordinatorRegistryResponse::Registry(initial) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .expect("query initial two-Root Registry");
        let mut added_principals = initial.admission.fleet_principals.clone();
        added_principals.push(added);
        added_principals.sort_unstable();
        let added_policy = compile_installed_fleet_admission_policy(
            initial.admission.fleet.clone(),
            initial.admission.generation + 1,
            added_principals,
            initial.admission.rules.clone(),
        )
        .expect("compile two-Root admission addition");
        let add_operation_id = [0xd7; 32];
        let (participant_catalog_digest, participant_count) =
            admission_participant_catalog_authority(&fixture.pic, &fixture.roots, &added_policy);
        let add = FleetAdmissionMutationRequest {
            authority: initial.authority.binding.clone(),
            expected_generation: initial.admission.generation,
            expected_policy_digest: initial.admission.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: add_operation_id,
            successor_policy_digest: added_policy.policy_digest,
            participant_catalog_digest,
            participant_count,
        };
        let CoordinatorCommandResponse::MutateAdmission(planned) = coordinator_command(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorCommand::MutateAdmission(add),
        )
        .expect("plan two-Root admission addition") else {
            panic!("Coordinator returned a differently correlated admission response")
        };
        assert_eq!(planned.outcome, FleetAdmissionMutationOutcome::Planned);
        let completed = await_fleet_admission_convergence_across_coordinator_restarts(
            &fixture.pic,
            fixture.coordinator,
            &fixture.roots,
            &fixture
                .components
                .iter()
                .map(|component| component.canister_id)
                .collect::<Vec<_>>(),
            add_operation_id,
        );
        assert_eq!(completed.outcome, FleetAdmissionMutationOutcome::Converged);
        for (root, component) in fixture.roots.into_iter().zip(&fixture.components) {
            let status = managed_admission_status(&fixture.pic, component.canister_id, root);
            assert_eq!(status.phase, FleetAdmissionProjectionPhase::Open);
            assert_eq!(status.generation, added_policy.generation);
            assert_eq!(status.policy_digest, added_policy.policy_digest);
            assert!(status.principals.entries.contains(&added));
        }

        let CoordinatorRegistryResponse::Registry(added_registry) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .expect("query added two-Root Registry");
        let mut removed_principals = added_registry.admission.fleet_principals.clone();
        removed_principals.retain(|principal| *principal != added);
        let removed_policy = compile_installed_fleet_admission_policy(
            added_registry.admission.fleet.clone(),
            added_registry.admission.generation + 1,
            removed_principals,
            added_registry.admission.rules.clone(),
        )
        .expect("compile two-Root admission removal");
        let remove_operation_id = [0xd8; 32];
        let (participant_catalog_digest, participant_count) =
            admission_participant_catalog_authority(&fixture.pic, &fixture.roots, &removed_policy);
        let remove = FleetAdmissionMutationRequest {
            authority: added_registry.authority.binding.clone(),
            expected_generation: added_registry.admission.generation,
            expected_policy_digest: added_registry.admission.policy_digest,
            action: FleetAdmissionMutationAction::Remove,
            selector: FleetAdmissionSelector::Fleet,
            principal: added,
            operation_id: remove_operation_id,
            successor_policy_digest: removed_policy.policy_digest,
            participant_catalog_digest,
            participant_count,
        };
        let CoordinatorCommandResponse::MutateAdmission(planned) = coordinator_command(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorCommand::MutateAdmission(remove),
        )
        .expect("plan two-Root admission removal") else {
            panic!("Coordinator returned a differently correlated admission response")
        };
        assert_eq!(planned.outcome, FleetAdmissionMutationOutcome::Planned);
        let completed = await_fleet_admission_convergence(
            &fixture.pic,
            fixture.coordinator,
            remove_operation_id,
        );
        assert_eq!(completed.outcome, FleetAdmissionMutationOutcome::Converged);
        for (root, component) in fixture.roots.into_iter().zip(&fixture.components) {
            let status = managed_admission_status(&fixture.pic, component.canister_id, root);
            assert_eq!(status.phase, FleetAdmissionProjectionPhase::Open);
            assert_eq!(status.generation, removed_policy.generation);
            assert_eq!(status.policy_digest, removed_policy.policy_digest);
            assert!(!status.principals.entries.contains(&added));
        }
    }

    #[test]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the pooled Fleet fixture lease is intentionally retained for the full test"
    )]
    fn restored_root_preserves_its_inventory_but_cannot_allocate() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let initial = acquire_active_component_registry();
        drop(initial);
        let fixture = acquire_active_component_registry();
        let RootStatusResponseFragment::Inventory(before) = root_status(
            fixture.pic(),
            fixture.root,
            RootStatusRequestFragment::Inventory,
        )
        .expect("query root inventory before snapshot") else {
            panic!("Root returned a differently correlated inventory status");
        };
        assert_root_native_timer_state(
            fixture.pic(),
            fixture.root,
            TimerRegistrationStatus::Scheduled,
        );

        let snapshot_request = AuthoritySnapshotRequest {
            operation_id: [0xb4; 32],
        };
        seal_capture_live_resume_and_restore(&fixture, snapshot_request);

        let RootStatusResponseFragment::AuthorityRestore(restored_fence) = root_status(
            fixture.pic(),
            fixture.root,
            RootStatusRequestFragment::AuthorityRestore,
        )
        .expect("restored root authority fence status") else {
            panic!("Root returned a differently correlated authority status");
        };
        assert_eq!(restored_fence.phase, AuthorityRestoreFencePhase::Sealed);
        assert_root_native_timer_state(
            fixture.pic(),
            fixture.root,
            TimerRegistrationStatus::Unregistered,
        );
        let RootStatusResponseFragment::Inventory(after) = root_status(
            fixture.pic(),
            fixture.root,
            RootStatusRequestFragment::Inventory,
        )
        .expect("query restored root inventory") else {
            panic!("Root returned a differently correlated inventory status");
        };
        assert_eq!(
            after, before,
            "snapshot restore must preserve the exact physical inventory"
        );

        let rejected_resume = root_command(
            fixture.pic(),
            fixture.root,
            RootCommandFragment::ResumeAuthoritySnapshot(snapshot_request),
        );
        assert_eq!(
            application_rejection(
                rejected_resume,
                "restored root authority must remain mutation-fenced",
            )
            .code(),
            canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
        );
        let fresh_allocation = root_command(
            fixture.pic(),
            fixture.root,
            RootCommandFragment::ProvisionComponent(RootComponentAllocationRequest {
                operation_id: [0xb5; 32],
                component_spec: fixture.verifier.component_spec.clone(),
            }),
        );
        assert!(
            fresh_allocation.is_err(),
            "restored root must reject allocation before handler dispatch"
        );
    }

    #[cfg(test)]
    fn seal_capture_live_resume_and_restore(
        fixture: &ActiveComponentRegistryFixture,
        request: AuthoritySnapshotRequest,
    ) {
        let RootCommandResponseFragment::PrepareAuthoritySnapshot(sealed) = root_command(
            fixture.pic(),
            fixture.root,
            RootCommandFragment::PrepareAuthoritySnapshot(request),
        )
        .expect("root authority snapshot prepare") else {
            panic!("Root returned a differently correlated authority response");
        };
        assert_eq!(sealed.phase, AuthorityRestoreFencePhase::Sealed);
        let funding_request = descendant_funding_request(fixture.pic(), 0xb6);
        assert_sealed_root_rejects_child_funding(fixture, funding_request.clone());
        fixture.pic().add_cycles(fixture.root, 4_000_000_000_000);
        assert_sealed_root_rejects_child_funding(fixture, funding_request.clone());
        assert_root_native_timer_state(
            fixture.pic(),
            fixture.root,
            TimerRegistrationStatus::Unregistered,
        );
        let snapshots = fixture
            .pic()
            .capture_controller_snapshots(fixture.root, [fixture.root])
            .expect("root authority snapshot capture");
        let RootCommandResponseFragment::ResumeAuthoritySnapshot(resumed) = root_command(
            fixture.pic(),
            fixture.root,
            RootCommandFragment::ResumeAuthoritySnapshot(request),
        )
        .expect("live root authority snapshot resume") else {
            panic!("Root returned a differently correlated authority response");
        };
        assert_eq!(resumed.phase, AuthorityRestoreFencePhase::Open);
        let child_before = fixture.pic().cycle_balance(fixture.issuer.canister_id);
        assert_eq!(
            request_descendant_funding(
                fixture.pic(),
                fixture.root,
                fixture.issuer.canister_id,
                funding_request.clone(),
            ),
            5_000_000_000_000,
        );
        let child_after = fixture.pic().cycle_balance(fixture.issuer.canister_id);
        assert!(child_after > child_before + 4_000_000_000_000);
        assert_eq!(
            request_descendant_funding(
                fixture.pic(),
                fixture.root,
                fixture.issuer.canister_id,
                funding_request,
            ),
            5_000_000_000_000,
        );
        assert!(fixture.pic().cycle_balance(fixture.issuer.canister_id) <= child_after);
        assert_root_native_timer_state(
            fixture.pic(),
            fixture.root,
            TimerRegistrationStatus::Scheduled,
        );

        fixture
            .pic()
            .restore_snapshots_with_captured_senders_and_funding(
                &snapshots,
                SnapshotRestoreFunding::TopUpTo {
                    minimum_cycles: crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
                },
            )
            .expect("root authority snapshot restore");
    }

    #[cfg(test)]
    fn assert_sealed_root_rejects_child_funding(
        fixture: &ActiveComponentRegistryFixture,
        request: canic::dto::capability::RootCapabilityEnvelopeV1,
    ) {
        let child_before = fixture.pic().cycle_balance(fixture.issuer.canister_id);
        let root_before = fixture.pic().cycle_balance(fixture.root);
        let response = root_command_as(
            fixture.pic(),
            fixture.root,
            fixture.issuer.canister_id,
            RootCommandFragment::RespondCapability(request),
        );
        assert_eq!(
            application_rejection(response, "sealed Root must reject child funding").code(),
            canic_core::diagnostics::codes::AUTHORITY_INACTIVE.raw_code(),
        );
        assert!(fixture.pic().cycle_balance(fixture.issuer.canister_id) <= child_before);
        assert!(
            root_before.saturating_sub(fixture.pic().cycle_balance(fixture.root)) < 1_000_000_000
        );
    }

    #[cfg(test)]
    fn assert_root_native_timer_state(
        pic: &PocketIc,
        root: Principal,
        expected: TimerRegistrationStatus,
    ) {
        let RootStatusResponseFragment::Runtime(status) =
            root_status(pic, root, RootStatusRequestFragment::Runtime)
                .expect("query Root timer inventory")
        else {
            panic!("Root returned a differently correlated Runtime status");
        };
        let scheduled = status
            .timers
            .iter()
            .filter(|timer| timer.registration == TimerRegistrationStatus::Scheduled)
            .count();
        eprintln!(
            "Root timer inventory: declared={} scheduled={scheduled} expected_pool_state={expected:?}",
            status.timers.len()
        );
        let root_topup = status
            .timers
            .iter()
            .filter(|timer| timer.subsystem == "cycles" && timer.name == "topup")
            .collect::<Vec<_>>();
        assert!(
            root_topup.len() <= 1,
            "Root must have at most one top-up timer"
        );
        if expected == TimerRegistrationStatus::Scheduled {
            assert_eq!(
                root_topup.len(),
                1,
                "an active Root must declare its funding timer"
            );
        }
        assert!(
            root_topup
                .iter()
                .all(|timer| timer.owner == "canic" && timer.registration == expected),
            "Root funding timer must follow the authority snapshot fence"
        );
        for (subsystem, name) in [
            ("async_job_recovery", "watchdog"),
            ("canister_pool", "maintain"),
        ] {
            let matching = status
                .timers
                .iter()
                .filter(|timer| timer.subsystem == subsystem && timer.name == name)
                .collect::<Vec<_>>();
            assert_eq!(
                matching.len(),
                1,
                "exactly one Root-native {subsystem}/{name}"
            );
            let timer = matching[0];
            assert_eq!(timer.owner, "canic");
            assert_eq!(timer.registration, expected);
        }
    }

    #[test]
    fn published_draining_root_autonomously_reaches_external_deletion_readiness() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        // Root deletion is deliberately outside the pooled baseline's reset
        // contract: PocketIC cannot restore a captured snapshot after the
        // target canister has been deleted. Keep this destructive journey on
        // an exclusively owned instance so it cannot invalidate the warm
        // baseline used by reset-complete cases.
        let fixture = setup_fresh_active_component_registry();
        qualify_root_retirement(&RootRetirementFixture {
            pic: fixture.pic(),
            coordinator: fixture.coordinator,
            root: fixture.root,
            wasm_store: fixture.wasm_store,
            pool_assets: &fixture.pool_assets,
        });
    }

    /// Exclusively owned Fleet authority and all workload/pool assets to conserve.
    #[cfg(test)]
    struct RootRetirementFixture<'a> {
        pic: &'a PocketIc,
        coordinator: Principal,
        root: Principal,
        wasm_store: Principal,
        pool_assets: &'a [Principal],
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "the complete autonomous deletion journey shares exact inventory and conservation assertions"
    )]
    fn qualify_root_retirement(fixture: &RootRetirementFixture<'_>) {
        let cycles_ledger = install_retirement_ledger(fixture.pic, fixture.root);
        let operator = Principal::from_slice(&[0xe7; 29]);
        let mut controllers = fixture
            .pic
            .canister_status(fixture.coordinator, None)
            .expect("Coordinator controllers")
            .settings
            .controllers;
        controllers.push(operator);
        fixture
            .pic
            .set_controllers(fixture.coordinator, None, controllers)
            .expect("reviewed retirement operator controls Coordinator");

        let CoordinatorRegistryResponse::Registry(registry) = coordinator_status(
            fixture.pic,
            fixture.coordinator,
            CoordinatorRegistryRequest::Registry,
        )
        .expect("query active Registry before root removal");
        let CoordinatorObservabilityResponse::RegistryVersion(version) = coordinator_status(
            fixture.pic,
            fixture.coordinator,
            CoordinatorObservabilityRequest::RegistryVersion,
        )
        .expect("query active Registry version before root removal") else {
            panic!("Coordinator returned a differently correlated Registry status");
        };
        let expected_root = registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == fixture.root)
            .cloned()
            .expect("target root in Coordinator Registry");
        let operation_id = [0xd1; 32];
        let request = FleetSubnetRootDrainingReservationRequest {
            asset_recipient: operator,
            operation_id,
            expected_registry: version,
            expected_root,
        };
        let CoordinatorCommandResponse::OperationAccepted(receipt) = coordinator_command(
            fixture.pic,
            fixture.coordinator,
            CoordinatorCommand::RemoveRoot(request.clone()),
        )
        .expect("submit autonomous root removal") else {
            panic!("Coordinator returned a differently correlated removal response");
        };
        assert_eq!(receipt.operation_id, operation_id);

        let CoordinatorCommandResponse::OperationAccepted(retried) = coordinator_command(
            fixture.pic,
            fixture.coordinator,
            CoordinatorCommand::RemoveRoot(request),
        )
        .expect("retry autonomous root removal") else {
            panic!("Coordinator returned a differently correlated removal response");
        };
        assert_eq!(retried, receipt);

        let mut terminal = None;
        let mut last_status = None;
        let mut progress_transitions = 0_u16;
        let mut stagnant_seconds = 0_u16;
        for _ in 0..ROOT_REMOVAL_MAX_SIMULATED_SECONDS {
            for _ in 0..ROOT_REMOVAL_TICKS_PER_SECOND {
                fixture.pic.tick();
            }
            if let Ok(RootStatusResponseFragment::Operation(
                RootOperationStatusResponse::RemoveRoot(status),
            )) = root_status(
                fixture.pic,
                fixture.root,
                RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
            ) {
                let complete = status.deletion_preparation.is_some();
                if last_status.as_ref() == Some(&status) {
                    stagnant_seconds = stagnant_seconds.saturating_add(1);
                } else {
                    progress_transitions = progress_transitions.saturating_add(1);
                    stagnant_seconds = 0;
                }
                last_status = Some(status.clone());
                if complete {
                    terminal = Some(status);
                    break;
                }
            }
            fixture.pic.advance_time(Duration::from_secs(1));
        }
        let terminal = terminal.unwrap_or_else(|| {
            let storage: Result<StoreCatalogResponse, Error> = fixture.pic.query_candid_as(
                fixture.wasm_store, fixture.root,
                canic::protocol::CANIC_WASM_STORE_CATALOG,
                (StoreCatalogRequest::Storage,),
            ).expect("retirement Store diagnostic transport");
            match storage {
                Ok(StoreCatalogResponse::Storage(storage)) => eprintln!("retirement Store: {storage:?}"),
                Ok(_) => eprintln!("retirement Store returned another response variant"),
                Err(error) => eprintln!("retirement Store status: {error:?}"),
            }
            report_canister_diagnostics(
                fixture.pic,
                fixture.root,
                Principal::anonymous(),
                "autonomous Root removal timeout",
            );
            report_canister_diagnostics(
                fixture.pic,
                fixture.coordinator,
                Principal::anonymous(),
                "autonomous Coordinator Root-removal timeout",
            );
            let coordinator = coordinator_status(
                fixture.pic,
                fixture.coordinator,
                CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .ok()
            .and_then(|response| match response {
                CoordinatorOperationReadResponse::Operation(
                    CoordinatorOperationStatusResponse::RootRemoval(status),
                ) => Some(status),
                CoordinatorOperationReadResponse::Operation(_) => None,
            });
            let root_progress = last_status.as_ref().map(|status| {
                (
                    status.final_inventory.is_some(),
                    status.removal.is_some(),
                    status.store_reclamation.is_some(),
                    status.store_binding_finalization.is_some(),
                    status.store_deletion.is_some(),
                    status.deletion_readiness_intent.is_some(),
                    status.deletion_readiness.is_some(),
                    status.deletion_preparation.is_some(),
                )
            });
            let coordinator_progress = coordinator.as_ref().map(|status| {
                (
                    status.draining.is_some(),
                    status.removal.is_some(),
                    status.readiness_intent.is_some(),
                    status.readiness.is_some(),
                    status.execution.is_some(),
                    status.completion.is_some(),
                )
            });
            let pool = root_pool_status(fixture.pic, fixture.root);
            panic!(
                "Root must autonomously reach external deletion readiness; \
                 root(final_inventory, removal, reclamation, binding, store_deletion, \
                 readiness_intent, readiness, preparation)={root_progress:?}; \
                 pool(tracked, store, store_deletion_pending, workload, handing_off)=({}, {}, {}, {}, {}); \
                 coordinator(draining, removal, readiness_intent, readiness, execution, completion)={coordinator_progress:?}; \
                 progress_transitions={progress_transitions}; stagnant_seconds={stagnant_seconds}",
                pool.tracked,
                pool.store,
                pool.store_deletion_pending,
                pool.workload,
                pool.handing_off,
            )
        });
        assert!(terminal.final_inventory.is_some());
        assert!(terminal.removal.is_some());
        assert!(terminal.store_reclamation.is_some());
        assert!(terminal.store_binding_finalization.is_some());
        assert!(terminal.store_deletion.is_some());
        assert!(terminal.deletion_preparation.is_some());

        let pool = root_pool_status(fixture.pic, fixture.root);
        assert_eq!(pool.tracked, 0);
        assert_eq!(pool.completed_handoffs, fixture.pool_assets.len() as u64);
        for asset in fixture.pool_assets {
            let handed_off = fixture
                .pic
                .canister_status(*asset, Some(operator))
                .expect("operator must control each handed-off Component canister");
            assert_eq!(handed_off.module_hash, None);
            let mut controllers = handed_off.settings.controllers;
            controllers.sort();
            let mut expected_controllers = vec![fixture.root, operator];
            expected_controllers.sort();
            assert_eq!(controllers, expected_controllers);
        }
        assert!(
            fixture
                .pic
                .canister_status(fixture.wasm_store, Some(fixture.root))
                .is_err(),
            "autonomous removal must delete the retained Store"
        );

        let CoordinatorOperationReadResponse::Operation(
            CoordinatorOperationStatusResponse::RootRemoval(coordinator),
        ) = coordinator_status(
            fixture.pic,
            fixture.coordinator,
            CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
        )
        .expect("query Coordinator root-removal status")
        else {
            panic!("Coordinator returned a differently correlated operation status");
        };
        assert!(coordinator.draining.is_some());
        assert!(coordinator.removal.is_some());
        assert!(coordinator.readiness_intent.is_some());
        let readiness = coordinator
            .readiness
            .expect("Coordinator retains Ledger receipt");
        let ledger_receipt = &readiness.request.ledger_receipt;
        assert_eq!(ledger_receipt.intent.source, fixture.root);
        assert_eq!(ledger_receipt.intent.destination, fixture.coordinator);
        assert_eq!(ledger_receipt.intent.balance_before, 1_000_000_000);
        assert_eq!(ledger_receipt.intent.fee, 100_000_000);
        assert!(ledger_receipt.block_index.is_some());
        let transfers: u64 = fixture
            .pic
            .query_candid(cycles_ledger, "transfer_count", ())
            .expect("one transfer despite lost reply");
        assert_eq!(transfers, 1);
        assert_eq!(
            ledger_account_balance(fixture.pic, cycles_ledger, fixture.root),
            Nat::from(0_u8)
        );
        assert_eq!(
            ledger_account_balance(fixture.pic, cycles_ledger, fixture.coordinator),
            Nat::from(900_000_000_u128)
        );
        assert_eq!(
            terminal
                .deletion_preparation
                .as_ref()
                .expect("Root proof")
                .ledger_receipt,
            *ledger_receipt
        );

        assert!(coordinator.execution.is_none());
        assert!(coordinator.completion.is_none());
        finish_coordinator_retirement(fixture, cycles_ledger, operator, readiness);
    }

    #[cfg(test)]
    fn finish_coordinator_retirement(
        fixture: &RootRetirementFixture<'_>,
        ledger: Principal,
        operator: Principal,
        readiness: canic_core::dto::fleet_registry::FleetSubnetRootDeletionReadinessResponse,
    ) {
        use canic_core::dto::fleet_registry::FleetRetirementRequest;
        let pic = fixture.pic;
        let CoordinatorObservabilityResponse::RegistryVersion(version) = coordinator_status(
            pic,
            fixture.coordinator,
            CoordinatorObservabilityRequest::RegistryVersion,
        )
        .expect("removed Registry") else {
            panic!("Registry version correlation");
        };
        let request = FleetRetirementRequest {
            operation_id: [0xe8; 32],
            expected_registry: version,
            destination: operator,
            maximum_ledger_fee: 100_000_000,
        };
        let call = |request| -> Result<CoordinatorCommandResponse, Error> {
            pic.update_candid_as(
                fixture.coordinator,
                operator,
                canic::protocol::CANIC_COORDINATOR_COMMAND,
                (CoordinatorCommand::Retire(request),),
            )
            .expect("retirement transport")
        };
        let Err(premature) = call(request.clone()) else {
            panic!("Root deletion must precede final Coordinator transfer");
        };
        assert_eq!(
            premature.code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );
        delete_prepared_root(fixture, readiness);
        let mut insufficient_fee = request.clone();
        insufficient_fee.maximum_ledger_fee = 0;
        let Err(rejected) = call(insufficient_fee) else {
            panic!("the quoted fee exceeds reviewed authority");
        };
        assert_eq!(
            rejected.code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );
        assert_eq!(
            ledger_account_balance(pic, ledger, fixture.coordinator),
            Nat::from(900_000_000_u128)
        );
        let (): () = pic
            .update_candid(ledger, "lose_next_transfer_reply", ())
            .expect("lose Coordinator transfer reply");
        assert!(call(request.clone()).is_err());
        assert_eq!(
            ledger_account_balance(pic, ledger, fixture.coordinator),
            Nat::from(0_u8)
        );
        let CoordinatorCommandResponse::Retire(completed) =
            call(request.clone()).expect("reconcile exact transfer")
        else {
            panic!("retirement response correlation");
        };
        let receipt = completed
            .ledger_receipt
            .as_ref()
            .expect("terminal Ledger receipt");
        assert_eq!(receipt.intent.source, fixture.coordinator);
        assert_eq!(receipt.intent.destination, operator);
        assert_eq!(receipt.intent.balance_before, 900_000_000);
        assert_eq!(receipt.intent.fee, 100_000_000);
        assert_eq!(
            ledger_account_balance(pic, ledger, operator),
            Nat::from(800_000_000_u128)
        );
        let CoordinatorCommandResponse::Retire(replayed) =
            call(request).expect("effect-free retirement replay")
        else {
            panic!("retirement replay correlation");
        };
        assert_eq!(replayed, completed);
        let transfers: u64 = pic
            .query_candid(ledger, "transfer_count", ())
            .expect("exact transfer count");
        assert_eq!(transfers, 2);
        for asset in fixture.pool_assets {
            let status = pic
                .canister_status(*asset, Some(operator))
                .expect("assets remain controlled after Root deletion");
            assert!(status.settings.controllers.contains(&operator));
            assert_eq!(status.module_hash, None);
        }
    }

    #[cfg(test)]
    fn delete_prepared_root(
        fixture: &RootRetirementFixture<'_>,
        readiness: canic_core::dto::fleet_registry::FleetSubnetRootDeletionReadinessResponse,
    ) {
        use canic_core::dto::fleet_registry::{
            FleetSubnetRootDeletionCompletionRequest, FleetSubnetRootDeletionExecutionRequest,
        };
        let pic = fixture.pic;
        let status = pic
            .canister_status(fixture.root, None)
            .expect("exact Root deletion authority");
        let mut controllers = status.settings.controllers;
        controllers.sort();
        let CoordinatorCommandResponse::PrepareRootDeletionExecution(execution) =
            coordinator_command(
                pic,
                fixture.coordinator,
                CoordinatorCommand::PrepareRootDeletionExecution(
                    FleetSubnetRootDeletionExecutionRequest {
                        operation_id: readiness.request.operation_id,
                        fleet_subnet_root: fixture.root,
                        expected_readiness_hash: readiness.readiness_hash,
                        observed_module_hash: status
                            .module_hash
                            .expect("installed Root")
                            .try_into()
                            .expect("SHA-256"),
                        observed_controllers: controllers,
                        observed_cycles_after_reclamation: u128::try_from(status.cycles.0)
                            .expect("native balance"),
                        observed_reserved_cycles: u128::try_from(status.reserved_cycles.0)
                            .expect("reserved cycles"),
                        observed_idle_cycles_burned_per_day: u128::try_from(
                            status.idle_cycles_burned_per_day.0,
                        )
                        .expect("idle burn"),
                        observed_freezing_threshold_seconds: u128::try_from(
                            status.settings.freezing_threshold.0,
                        )
                        .expect("freezing threshold"),
                    },
                ),
            )
            .expect("freeze Root deletion intent")
        else {
            panic!("execution response correlation");
        };
        pic.stop_canister(fixture.root, None)
            .expect("stop reclaimed Root");
        pic.delete_canister(fixture.root, None)
            .expect("delete only reviewed Root residual");
        let completion = FleetSubnetRootDeletionCompletionRequest {
            operation_id: readiness.request.operation_id,
            fleet_subnet_root: fixture.root,
            expected_execution_hash: execution.execution_hash,
            observed_absent_at_ns: pic.get_time().as_nanos_since_unix_epoch(),
        };
        coordinator_command(
            pic,
            fixture.coordinator,
            CoordinatorCommand::CompleteRootDeletion(completion),
        )
        .expect("retain Root absence receipt");
    }

    #[test]
    fn explicit_root_reinstall_retains_cycle_accounts_and_pool_control() {
        let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_fresh_active_component_registry();
        let ledger = install_retirement_ledger(fixture.pic(), fixture.root);
        let pic = fixture.pic();
        let root_wasm = build_test_root_wasm();
        let fresh = compile_reinstall_root_fixture(&fixture, &root_wasm);
        let assets = std::iter::once(fixture.wasm_store)
            .chain(fixture.pool_assets.iter().copied())
            .collect::<Vec<_>>();
        let mut controllers_before = BTreeMap::new();
        let mut native_before = pic.cycle_balance(fixture.root);
        for asset in &assets {
            let status = pic
                .canister_status(*asset, Some(fixture.root))
                .expect("observe exact Root-controlled asset before reinstall");
            assert!(status.settings.controllers.contains(&fixture.root));
            controllers_before.insert(*asset, status.settings.controllers);
            native_before += pic.cycle_balance(*asset);
        }
        let root_before = pic
            .canister_status(fixture.root, None)
            .expect("management Root authority");
        pic.stop_canister(fixture.root, None)
            .expect("settle Root calls before reinstall");
        pic.reinstall_canister(
            fixture.root,
            root_wasm,
            encode_one(&fresh.init_args).expect("current Root initializer"),
            None,
        )
        .expect("explicit management reinstall without old runtime queries");
        pic.start_canister(fixture.root, None)
            .expect("start reinstalled Root");
        pic.tick();
        let root_after = pic
            .canister_status(fixture.root, None)
            .expect("reinstalled Root");
        assert_eq!(
            root_after.settings.controllers,
            root_before.settings.controllers
        );
        assert!(root_after.version > root_before.version);
        let RootStatusResponseFragment::FleetAuthority(authority) =
            root_status(pic, fixture.root, RootStatusRequestFragment::FleetAuthority)
                .expect("current authority after hard cut")
        else {
            panic!("Root authority response correlation");
        };
        assert_eq!(authority, fresh.init_args.authority);
        let mut native_after = pic.cycle_balance(fixture.root);
        for asset in &assets {
            let status = pic
                .canister_status(*asset, Some(fixture.root))
                .expect("the reinstalled Root still controls every retained asset");
            assert_eq!(status.settings.controllers, controllers_before[asset]);
            native_after += pic.cycle_balance(*asset);
        }
        assert!(native_after <= native_before);
        assert!(native_before - native_after <= 5_000_000_000_000);
        assert_eq!(
            ledger_account_balance(pic, ledger, fixture.root),
            Nat::from(1_000_000_000_u128)
        );
        assert_eq!(
            ledger_account_balance(pic, ledger, fixture.coordinator),
            Nat::from(0_u128)
        );
        let transfers: u64 = pic
            .query_candid(ledger, "transfer_count", ())
            .expect("Ledger transfer count");
        drop(fixture);
        assert_eq!(transfers, 0);
    }

    #[cfg(test)]
    fn compile_reinstall_root_fixture(
        fixture: &ActiveComponentRegistryFixture,
        root_wasm: &[u8],
    ) -> InstalledRootFixture {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        prepare_current_root_fixture(
            fixture.pic(),
            root_wasm,
            &build_test_wasm_store_wasm(),
            fixture.coordinator,
            fixture.root,
            fixture.wasm_store,
            Principal::from_slice(&[0x46; 29]),
            build_root_store_fixture(),
            &BootstrappedRootPlacement {
                canister_pool_maximum_size: None,
                canister_pool_minimum_size: None,
                canister_pool_cycles: None,
                coordinator_subnet: None,
                existing_root: Some(fixture.root),
                existing_wasm_store: Some(fixture.wasm_store),
                root_subnet: None,
                component_admission_limits: None,
                fleet_id: Some(FleetId::from_generated_bytes([0xe9; 32])),
                funding: None,
                coordinator_root_funding: None,
            },
            &root_canister_config_path(workspace),
            fixture.pool_assets.clone(),
        )
    }

    #[cfg(test)]
    fn install_retirement_ledger(pic: &PocketIc, root: Principal) -> Principal {
        let (_, wasm) = build_mainnet_refill_wasms();
        let ledger = Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai").expect("canonical Ledger");
        pic.create_canister_with_id(None, None, ledger)
            .expect("create retirement Ledger stub");
        pic.add_cycles(ledger, 100_000_000_000_000);
        pic.install_canister(
            ledger,
            wasm,
            encode_one(CyclesLedgerStubInitArgs {
                canister_ids: vec![],
                expected_controllers_by_index: None,
                expected_root: root,
                expected_subnet: pic.get_subnet(root).expect("Root subnet"),
                initial_balances: Some(vec![CyclesLedgerStubAccountBalance {
                    owner: root,
                    balance: Nat::from(1_000_000_000_u128),
                }]),
                pending_first_index: None,
                withdrawal_fee: Some(Nat::from(100_000_000_u128)),
            })
            .expect("retirement Ledger init"),
            None,
        );
        let (): () = pic
            .update_candid(ledger, "lose_next_transfer_reply", ())
            .expect("lose committed transfer reply");
        ledger
    }

    /// Acquire one current Coordinator/root/Store fixture with active Registry-issued Components.
    ///
    /// # Panics
    ///
    /// Panics when a fixture artifact cannot be built or any required PocketIC
    /// management, Registry, Store, allocation, installation, or activation call
    /// fails its current protocol contract.
    #[must_use]
    pub fn setup_active_component_registry() -> ActiveComponentRegistryFixture {
        acquire_active_component_registry()
    }

    /// Build a fresh active Component Registry fixture for exclusive native-agent use.
    ///
    /// This deliberately bypasses the immutable shared baseline so its caller
    /// can finish setup before starting an authenticated HTTP ingress gateway.
    ///
    /// # Panics
    ///
    /// Panics when the fresh fixture cannot be prepared.
    #[must_use]
    pub fn setup_fresh_active_component_registry() -> ActiveComponentRegistryFixture {
        let fixture = setup_active_component_registry_fresh();
        wait_for_role_overviews_ready(
            fixture.pic(),
            [
                ("coordinator", fixture.coordinator, Principal::anonymous()),
                ("root", fixture.root, Principal::anonymous()),
                ("wasm_store", fixture.wasm_store, fixture.root),
                ("issuer", fixture.issuer.canister_id, fixture.root),
                ("verifier", fixture.verifier.canister_id, fixture.root),
            ],
            60,
            "fresh active Component Registry fixture",
        )
        .expect("fresh active Component Registry roles must become ready");
        fixture
    }

    #[cfg(test)]
    fn assert_root_local_physical_inventory(pic: &PocketIc, fixture: &BootstrappedRootFixture) {
        let expected_subnet = *fixture
            .init_args
            .authority
            .binding
            .placement_subnet
            .as_principal();
        assert_eq!(pic.get_subnet(fixture.root_id), Some(expected_subnet));
        assert_eq!(
            pic.get_subnet(fixture.response.wasm_store),
            Some(expected_subnet)
        );
        assert!(
            fixture
                .init_args
                .canister_pool_imports
                .iter()
                .all(|canister| { pic.get_subnet(*canister) == Some(expected_subnet) })
        );
        for canister in &fixture.init_args.canister_pool_imports {
            let live = pic
                .canister_status(*canister, Some(fixture.root_id))
                .expect("observe root-owned prepaid pool asset");
            assert_eq!(live.settings.controllers, vec![fixture.root_id]);
            assert_eq!(live.module_hash, None);
        }
    }

    fn acquire_active_component_registry() -> ActiveComponentRegistryFixture {
        let (baseline, outcome) = active_component_registry_pool()
            .acquire()
            .expect("acquire active Component Registry baseline");
        let metadata = baseline.metadata().clone();
        crate::pic::progress::timed(
            "FLEET",
            crate::pic::progress::ProgressStatus::Ready,
            &format!("active baseline (slot {})", outcome.slot()),
            outcome.timings().total(),
        );
        crate::pic::progress::detail("FLEET", &format!("active baseline: {outcome}"));

        ActiveComponentRegistryFixture {
            runtime: ActiveComponentRegistryRuntime::Pooled(baseline),
            coordinator: metadata.coordinator,
            root: metadata.root,
            issuer: metadata.issuer,
            verifier: metadata.verifier,
            issuer_runtime_operation_id: metadata.issuer_runtime_operation_id,
            verifier_runtime_operation_id: metadata.verifier_runtime_operation_id,
            store_bootstrap: metadata.store_bootstrap,
            wasm_store: metadata.wasm_store,
            pool_assets: metadata.pool_assets,
        }
    }

    fn active_component_registry_pool()
    -> &'static CachedPocketIcBaselinePool<ActiveComponentRegistryBaselineRecipe> {
        static POOL: OnceLock<CachedPocketIcBaselinePool<ActiveComponentRegistryBaselineRecipe>> =
            OnceLock::new();
        POOL.get_or_init(|| {
            CachedPocketIcBaselinePool::new(
                NonZeroUsize::new(1).expect("one is nonzero"),
                ActiveComponentRegistryBaselineRecipe::new()
                    .expect("valid active Component Registry baseline recipe"),
            )
        })
    }

    fn setup_active_component_registry_fresh() -> ActiveComponentRegistryFixture {
        setup_active_component_registry_with_pic(build_pic)
    }

    fn setup_active_component_registry_with_pic(
        new_pic: fn() -> PocketIc,
    ) -> ActiveComponentRegistryFixture {
        let total_started = Instant::now();
        let phase_started = Instant::now();
        let root_wasm = build_test_root_wasm();
        progress_elapsed("root artifacts ready", phase_started);
        let phase_started = Instant::now();
        let coordinator_wasm = build_test_coordinator_wasm();
        progress_elapsed("Coordinator artifact ready", phase_started);
        let phase_started = Instant::now();
        let store_fixture = build_root_store_fixture();
        progress_elapsed("Store and Component artifacts ready", phase_started);
        let phase_started = Instant::now();
        let pic = new_pic();
        progress_elapsed("PocketIC topology ready", phase_started);
        let phase_started = Instant::now();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let fixture = install_bootstrapped_root(&pic, root_wasm, coordinator, store_fixture);
        progress_elapsed("Root and Store installed", phase_started);
        let phase_started = Instant::now();
        install_fixture_coordinator(&pic, coordinator, coordinator_wasm, &fixture);
        progress_elapsed("Coordinator installed", phase_started);
        let phase_started = Instant::now();
        let (joining_version, sync_request) =
            join_and_synchronize_root(&pic, coordinator, &fixture);
        progress_elapsed("Root joined and synchronized", phase_started);

        let phase_started = Instant::now();
        let components = assert_registry_and_root_runtime_activation(
            &pic,
            coordinator,
            &fixture,
            joining_version,
            sync_request,
        );
        progress_elapsed("Component Registry activated", phase_started);
        let fixture = ActiveComponentRegistryFixture {
            runtime: ActiveComponentRegistryRuntime::Fresh(Box::new(pic)),
            coordinator,
            root: fixture.root_id,
            issuer: components.issuer,
            verifier: components.verifier,
            issuer_runtime_operation_id: components.issuer_runtime_operation_id,
            verifier_runtime_operation_id: components.verifier_runtime_operation_id,
            store_bootstrap: fixture.request,
            wasm_store: fixture.response.wasm_store,
            pool_assets: fixture.init_args.canister_pool_imports,
        };
        let phase_started = Instant::now();
        assert_root_canister_summary(&fixture);
        progress_elapsed("fresh Fleet validated", phase_started);
        progress_elapsed("fresh Fleet setup complete", total_started);
        fixture
    }

    fn join_and_synchronize_root(
        pic: &PocketIc,
        coordinator: Principal,
        fixture: &BootstrappedRootFixture,
    ) -> (
        canic::dto::fleet_registry::FleetRegistryVersion,
        FleetSubnetRootRegistrySyncRequest,
    ) {
        let CoordinatorObservabilityResponse::RegistryVersion(genesis) = coordinator_status(
            pic,
            coordinator,
            CoordinatorObservabilityRequest::RegistryVersion,
        )
        .expect("query Registry genesis") else {
            panic!("Coordinator returned a differently correlated Registry status");
        };
        let binding = &fixture.init_args.authority.binding;
        let join_request = FleetSubnetRootJoinRequest {
            expected_registry: genesis,
            entry: FleetSubnetRootEntry {
                placement_subnet: binding.placement_subnet,
                fleet_subnet_root: fixture.root_id,
                component_admissions: binding.component_admissions.clone(),
                component_topology_digest: binding.component_topology_digest,
                active_release_set: fixture.init_args.authority.initial_release_set,
                funding: binding.funding.clone(),
                limits: binding.limits.clone(),
                status: FleetSubnetRootStatus::Joining,
            },
        };
        let CoordinatorCommandResponse::JoinRoot(joined) =
            coordinator_command(pic, coordinator, CoordinatorCommand::JoinRoot(join_request))
                .expect("join root")
        else {
            panic!("Coordinator returned a differently correlated join response");
        };
        let sync_request = FleetSubnetRootRegistrySyncRequest {
            operation_id: [20; 32],
            expected_registry: joined.version.clone(),
            store_bootstrap: fixture.request.clone(),
        };
        let RootCommandResponseFragment::OperationAccepted(receipt) = root_command(
            pic,
            fixture.root_id,
            RootCommandFragment::SynchronizeRegistry(sync_request.clone()),
        )
        .expect("root Registry synchronization") else {
            panic!("Root returned a differently correlated synchronization response");
        };
        assert_eq!(receipt.operation_id, sync_request.operation_id);
        let RootStatusResponseFragment::Operation(
            RootOperationStatusResponse::SynchronizeRegistry(synchronization),
        ) = root_status(
            pic,
            fixture.root_id,
            RootStatusRequestFragment::Operation(OperationStatusRequest {
                operation_id: sync_request.operation_id,
            }),
        )
        .expect("root Registry synchronization status")
        else {
            panic!("Root returned a differently correlated synchronization status");
        };
        let synchronized = synchronization.synchronization;
        assert_eq!(synchronized.fleet_subnet_root, fixture.root_id);
        assert_eq!(synchronized.version, joined.version);

        let RootCommandResponseFragment::OperationAccepted(retried) = root_command(
            pic,
            fixture.root_id,
            RootCommandFragment::SynchronizeRegistry(sync_request.clone()),
        )
        .expect("root Registry synchronization retry") else {
            panic!("Root returned a differently correlated synchronization response");
        };
        assert_eq!(retried, receipt);
        let CoordinatorObservabilityResponse::RootAcknowledgements(acknowledgements) =
            coordinator_status(
                pic,
                coordinator,
                CoordinatorObservabilityRequest::RootAcknowledgements,
            )
            .expect("query root acknowledgements")
        else {
            panic!("Coordinator returned a differently correlated acknowledgement status");
        };
        assert_eq!(acknowledgements, vec![synchronized.acknowledgement]);
        (joined.version, sync_request)
    }

    fn reset_unclaimed_pool_assets(
        baseline: &CachedPocketIcBaseline<ActiveComponentRegistryBaselineMetadata>,
    ) -> Result<(), ActiveComponentRegistryBaselineError> {
        let pic = baseline.pocket_ic();
        let metadata = baseline.metadata();
        let workload_canisters = [metadata.issuer.canister_id, metadata.verifier.canister_id];

        for canister_id in metadata
            .pool_assets
            .iter()
            .copied()
            .filter(|canister_id| !workload_canisters.contains(canister_id))
        {
            if !pic.canister_exists(canister_id) {
                return Err(ActiveComponentRegistryBaselineError::Invariant(format!(
                    "pooled asset {canister_id} no longer exists"
                )));
            }
            let status = pic
                .canister_status(canister_id, Some(metadata.root))
                .map_err(|error| {
                    ActiveComponentRegistryBaselineError::Invariant(format!(
                        "inspect pooled asset {canister_id}: {error:?}"
                    ))
                })?;
            if status.module_hash.is_some() {
                pic.uninstall_canister(canister_id, Some(metadata.root))
                    .map_err(|error| {
                        ActiveComponentRegistryBaselineError::Invariant(format!(
                            "uninstall pooled asset {canister_id}: {error:?}"
                        ))
                    })?;
            }
            pic.set_controllers(canister_id, Some(metadata.root), vec![metadata.root])
                .map_err(|error| {
                    ActiveComponentRegistryBaselineError::Invariant(format!(
                        "restore pooled asset {canister_id} controller: {error:?}"
                    ))
                })?;
            pic.start_canister(canister_id, Some(metadata.root))
                .map_err(|error| {
                    ActiveComponentRegistryBaselineError::Invariant(format!(
                        "start pooled asset {canister_id}: {error:?}"
                    ))
                })?;
            let cycles = pic.cycle_balance(canister_id);
            if cycles < PREPAID_POOL_ASSET_CYCLES {
                pic.add_cycles(canister_id, PREPAID_POOL_ASSET_CYCLES - cycles);
            }
        }
        Ok(())
    }

    #[test]
    fn protected_memory_allocations_preserve_stable_state_and_root_authority() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_active_component_registry();
        let pic = fixture.pic();
        let target = fixture.issuer.canister_id;
        let outsider = Principal::from_slice(&[0x7f; 29]);

        for canister in [fixture.root, target] {
            let denied: Result<CanisterObservabilityResponse, Error> = pic
                .query_candid_as(
                    canister,
                    outsider,
                    canic::protocol::CANIC_OBSERVABILITY,
                    (CanisterObservabilityRequest::MemoryAllocations,),
                )
                .expect("direct observation transport");
            assert!(controller_authority_unavailable(&denied));
        }
        let root_report: Result<CanisterObservabilityResponse, Error> = pic
            .query_candid(
                fixture.root,
                canic::protocol::CANIC_OBSERVABILITY,
                (CanisterObservabilityRequest::MemoryAllocations,),
            )
            .expect("Root allocation query");
        let CanisterObservabilityResponse::MemoryAllocations(root_report) =
            root_report.expect("Root allocation report")
        else {
            panic!("expected Root allocations");
        };
        assert_memory_allocation_conservation(
            &root_report,
            pic.get_stable_memory(fixture.root).len() as u64,
        );
        eprintln!(
            "Root stable allocation: physical={} buckets={} virtual={}",
            root_report.physical_extent.bytes,
            root_report.allocated_bucket_bytes,
            root_report
                .memories
                .iter()
                .map(|entry| entry.virtual_extent.bytes)
                .sum::<u64>()
        );

        let relay_request = || {
            RootCommandFragment::ObserveCanister(FleetCanisterObservabilityRequest {
                canister_id: target,
                request: CanisterObservabilityRequest::MemoryAllocations,
            })
        };
        let denied: Result<RootCommandResponseFragment, Error> = pic
            .update_candid_as(
                fixture.root,
                outsider,
                canic::protocol::CANIC_ROOT_COMMAND,
                (relay_request(),),
            )
            .expect("relay refusal transport");
        assert!(controller_authority_unavailable(&denied));

        let before = pic.get_stable_memory(target);
        let physical_extent = before.len() as u64;
        let before_digest = wasm_hash(&before);
        drop(before);
        let mut previous = None;
        for _ in 0..2 {
            let response: Result<RootCommandResponseFragment, Error> = pic
                .update_candid(
                    fixture.root,
                    canic::protocol::CANIC_ROOT_COMMAND,
                    (relay_request(),),
                )
                .expect("relay transport");
            let RootCommandResponseFragment::ObserveCanister(
                CanisterObservabilityResponse::MemoryAllocations(report),
            ) = response.expect("controller relay accepted")
            else {
                panic!("expected current memory allocations");
            };
            assert_memory_allocation_conservation(&report, physical_extent);
            if let Some(previous) = previous {
                assert_eq!(report, previous);
            }
            previous = Some(report);
        }
        assert_eq!(wasm_hash(&pic.get_stable_memory(target)), before_digest);
        drop(fixture);
    }

    #[cfg(test)]
    fn assert_memory_allocation_conservation(
        report: &canic::dto::memory::MemoryAllocationsResponse,
        physical_extent: u64,
    ) {
        assert_eq!(report.physical_extent.bytes, physical_extent);
        assert_eq!(
            report.bucket_size_pages,
            canic::memory::configured_bucket_pages()
        );
        assert_eq!(report.metadata_bytes_read, 34_848);
        assert_eq!(report.memories.len(), 255);
        assert_eq!(
            report.physical_extent.bytes,
            report.manager_metadata_bytes + report.allocated_bucket_bytes + report.unmanaged_bytes
        );
        assert_eq!(
            report.allocated_bucket_bytes,
            report
                .memories
                .iter()
                .map(|entry| entry.allocated_bytes)
                .sum::<u64>()
        );
        assert_eq!(
            report.allocated_bucket_bytes,
            report.known_binding_bytes + report.unknown_binding_bytes
        );
        assert_eq!(
            report.allocated_bucket_bytes,
            report.virtual_extent.bytes + report.bucket_slack_bytes
        );
        assert!(matches!(
            report.memories[0].binding,
            canic::dto::memory::MemoryAllocationBinding::Ledger { .. }
        ));
        assert!(
            report
                .memories
                .iter()
                .all(|entry| entry.payload_bytes.is_none())
        );
        assert!(
            report
                .memories
                .windows(2)
                .all(|pair| pair[0].memory_manager_id < pair[1].memory_manager_id)
        );
        assert!(
            report
                .memories
                .iter()
                .any(|entry| entry.virtual_extent.wasm_pages == 0)
        );
    }

    fn validate_active_component_registry_baseline(
        baseline: &CachedPocketIcBaseline<ActiveComponentRegistryBaselineMetadata>,
    ) -> Result<(), ActiveComponentRegistryBaselineError> {
        let pic = baseline.pocket_ic();
        let metadata = baseline.metadata();
        #[cfg(test)]
        validate_sensitive_observability_authority(pic, metadata)?;
        let CoordinatorRegistryResponse::Registry(registry) = baseline_application_result(
            coordinator_status(
                pic,
                metadata.coordinator,
                CoordinatorRegistryRequest::Registry,
            ),
            "query active Fleet Registry",
        )?;
        if registry.fleet_subnet_roots.len() != 1
            || registry.fleet_subnet_roots[0].fleet_subnet_root != metadata.root
            || registry.fleet_subnet_roots[0].status != FleetSubnetRootStatus::Active
        {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "active Fleet Registry root binding changed".to_string(),
            ));
        }

        let RootStatusResponseFragment::Inventory(inventory) = baseline_application_result(
            root_status(pic, metadata.root, RootStatusRequestFragment::Inventory),
            "query root inventory",
        )?
        else {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Root returned a differently correlated inventory status".to_string(),
            ));
        };
        if inventory.status != FleetSubnetRootStatus::Active {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Fleet Subnet Root is not active".to_string(),
            ));
        }

        for (binding, operation_id) in [
            (&metadata.issuer, metadata.issuer_runtime_operation_id),
            (&metadata.verifier, metadata.verifier_runtime_operation_id),
        ] {
            let runtime: Result<ManagedStatusResponseFragment, Error> = pic.query_candid_as(
                binding.canister_id,
                metadata.root,
                canic::protocol::CANIC_CONTROL_STATUS,
                (ManagedStatusRequestFragment::Operation(
                    OperationStatusRequest { operation_id },
                ),),
            )?;
            let ManagedStatusResponseFragment::Operation(operation) =
                baseline_application_result(runtime, "query Component runtime")?
            else {
                return Err(ActiveComponentRegistryBaselineError::Invariant(
                    "Component returned a differently correlated runtime status".to_string(),
                ));
            };
            let ManagedOperationStatusResponseFragment::ConfigureRuntime(runtime) = *operation;
            if runtime.runtime.phase != ComponentRuntimePhase::Active {
                return Err(ActiveComponentRegistryBaselineError::Invariant(format!(
                    "Component {} is not active",
                    binding.canister_id
                )));
            }
        }

        let RootStatusResponseFragment::Pool(pool) = baseline_application_result(
            root_status(
                pic,
                metadata.root,
                RootStatusRequestFragment::Pool(CanisterPoolStatusRequest {
                    start_after: None,
                    limit: 256,
                }),
            ),
            "query root Canister pool",
        )?
        else {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Root returned a differently correlated pool status".to_string(),
            ));
        };
        let expected_ready = u32::try_from(metadata.pool_assets.len().saturating_sub(2))
            .expect("bounded test asset count");
        if pool.ready != expected_ready || pool.workload != 2 || pool.pending_reset != 0 {
            return Err(ActiveComponentRegistryBaselineError::Invariant(format!(
                "root Canister pool is not at baseline: ready={}, workload={}, pending_reset={}",
                pool.ready, pool.workload, pool.pending_reset
            )));
        }
        Ok(())
    }

    fn baseline_application_result<T>(
        result: Result<T, Error>,
        context: &str,
    ) -> Result<T, ActiveComponentRegistryBaselineError> {
        result.map_err(|error| {
            ActiveComponentRegistryBaselineError::Invariant(format!("{context}: {error:?}"))
        })
    }

    fn assert_root_canister_summary(fixture: &ActiveComponentRegistryFixture) {
        let RootStatusResponseFragment::Inventory(summary) = root_status(
            fixture.pic(),
            fixture.root,
            RootStatusRequestFragment::Inventory,
        )
        .expect("query Fleet Subnet Root Canister summary") else {
            panic!("Root returned a differently correlated inventory status");
        };
        let CoordinatorObservabilityResponse::RegistryVersion(coordinator_version) =
            coordinator_status(
                fixture.pic(),
                fixture.coordinator,
                CoordinatorObservabilityRequest::RegistryVersion,
            )
            .expect("query Coordinator Registry version")
        else {
            panic!("Coordinator returned a differently correlated Registry status");
        };

        assert_eq!(summary.fleet_registry, coordinator_version);
        assert_eq!(summary.fleet_subnet_root, fixture.root);
        assert_eq!(summary.placement_subnet, fixture.issuer.placement_subnet);
        assert_eq!(summary.status, FleetSubnetRootStatus::Active);
        assert_eq!(summary.infrastructure_canisters, 2);
        assert_eq!(summary.component_canisters, 2);
        assert_eq!(
            summary.pooled_canisters,
            u32::try_from(PREPAID_POOL_ASSET_COUNT - 2).expect("bounded fixture pool size")
        );
        assert_eq!(
            summary.total_canisters,
            summary.infrastructure_canisters
                + summary.component_canisters
                + summary.pooled_canisters
        );
    }

    fn install_fixture_coordinator(
        pic: &PocketIc,
        coordinator: Principal,
        coordinator_wasm: Vec<u8>,
        fixture: &BootstrappedRootFixture,
    ) {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root");
        let config_path = root_canister_config_path(workspace_root);
        install_fixture_coordinator_with_config(
            pic,
            coordinator,
            coordinator_wasm,
            fixture,
            &config_path,
        );
    }

    fn install_fixture_coordinator_with_config(
        pic: &PocketIc,
        coordinator: Principal,
        coordinator_wasm: Vec<u8>,
        fixture: &BootstrappedRootFixture,
        config_path: &Path,
    ) {
        let config = AppConfigSnapshot::load(config_path).expect("load root config");
        let coordinator_args = FleetCoordinatorInitArgs {
            configured_app: fixture
                .init_args
                .authority
                .binding
                .authority
                .binding
                .fleet
                .app
                .clone(),
            authority: fixture.init_args.authority.binding.authority.clone(),
            admission: crate::pic::fleet_admission_policy(
                fixture
                    .init_args
                    .authority
                    .binding
                    .authority
                    .binding
                    .fleet
                    .clone(),
            ),
            root_funding: Some(fixture.coordinator_root_funding.clone()),
            component_deployment_configuration: config
                .model()
                .compile_component_deployment_configuration()
                .expect("compile Coordinator Component deployment configuration"),
        };
        pic.install_canister(
            coordinator,
            coordinator_wasm,
            encode_one(coordinator_args).expect("encode Coordinator init"),
            None,
        );
    }

    #[cfg(test)]
    fn fixture_fresh_component_plan(
        config: &canic_core::bootstrap::compiled::ConfigModel,
        registry: &canic::dto::fleet_registry::FleetRegistry,
        operation_id: [u8; 32],
    ) -> CompiledCurrentComponentProvisioning {
        let configuration = config
            .compile_component_deployment_configuration()
            .expect("compile fixture Component deployment configuration");
        let root = registry
            .fleet_subnet_roots
            .first()
            .expect("one registered Root")
            .fleet_subnet_root;
        let placements = configuration
            .deployment_topology
            .component_group_deployments
            .iter()
            .flat_map(|deployment| {
                (0..deployment.initial_placements).map(move |ordinal| {
                    CurrentComponentGroupPlacement {
                        deployment: deployment.deployment.clone(),
                        fleet_subnet_root: root,
                        ordinal,
                    }
                })
            })
            .collect::<Vec<_>>();
        compile_current_component_provisioning(&configuration, registry, operation_id, &placements)
            .expect("compile current typed Component provisioning")
    }

    #[cfg(test)]
    fn begin_fixture_fresh_component_provisioning_with_config(
        pic: &PocketIc,
        coordinator: Principal,
        coordinator_wasm: Vec<u8>,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
        config_path: &Path,
    ) -> RootComponentRegistryPreparationRequest {
        install_fixture_coordinator_with_config(
            pic,
            coordinator,
            coordinator_wasm,
            fixture,
            config_path,
        );
        let (joining_version, sync_request) = join_and_synchronize_root(pic, coordinator, fixture);
        let component_registry_request = activate_registry_and_prepare_component_registry(
            pic,
            coordinator,
            fixture,
            joining_version,
            sync_request,
        );
        let CoordinatorRegistryResponse::Registry(registry) =
            coordinator_status(pic, coordinator, CoordinatorRegistryRequest::Registry)
                .expect("query active Registry");
        let config =
            AppConfigSnapshot::load(config_path).expect("load provisioning fixture config");
        let compiled = fixture_fresh_component_plan(config.model(), &registry, operation_id);
        let CoordinatorCommandResponse::OperationAccepted(receipt) = coordinator_command(
            pic,
            coordinator,
            CoordinatorCommand::ProvisionComponents(compiled.request),
        )
        .expect("begin fresh Component provisioning") else {
            panic!("Coordinator returned a differently correlated provisioning response");
        };
        assert_eq!(receipt.operation_id, operation_id);
        component_registry_request
    }

    fn assert_registry_and_root_runtime_activation(
        pic: &PocketIc,
        coordinator: Principal,
        fixture: &BootstrappedRootFixture,
        joining_version: canic::dto::fleet_registry::FleetRegistryVersion,
        sync_request: FleetSubnetRootRegistrySyncRequest,
    ) -> ActiveComponentBindings {
        let component_registry_request = activate_registry_and_prepare_component_registry(
            pic,
            coordinator,
            fixture,
            joining_version,
            sync_request,
        );
        assert_component_allocation(pic, fixture, component_registry_request)
    }

    fn activate_registry_and_prepare_component_registry(
        pic: &PocketIc,
        coordinator: Principal,
        fixture: &BootstrappedRootFixture,
        joining_version: canic::dto::fleet_registry::FleetRegistryVersion,
        joining_sync_request: FleetSubnetRootRegistrySyncRequest,
    ) -> RootComponentRegistryPreparationRequest {
        let CoordinatorCommandResponse::ActivateRegistry(activated) = coordinator_command(
            pic,
            coordinator,
            CoordinatorCommand::ActivateRegistry(FleetRegistryActivationRequest {
                expected_registry: joining_version,
            }),
        )
        .expect("activate Registry") else {
            panic!("Coordinator returned a differently correlated activation response");
        };
        let CoordinatorRegistryResponse::Registry(active) =
            coordinator_status(pic, coordinator, CoordinatorRegistryRequest::Registry)
                .expect("query active Registry");
        assert_eq!(
            active.fleet_subnet_roots.first().expect("one root").status,
            FleetSubnetRootStatus::Active
        );

        let mut synchronized = None;
        for _ in 0..32 {
            let RootStatusResponseFragment::Operation(
                RootOperationStatusResponse::SynchronizeRegistry(status),
            ) = root_status(
                pic,
                fixture.root_id,
                RootStatusRequestFragment::Operation(OperationStatusRequest {
                    operation_id: joining_sync_request.operation_id,
                }),
            )
            .expect("query active Registry synchronization")
            else {
                panic!("Root returned a differently correlated synchronization status");
            };
            if status.activation.is_some() {
                synchronized = Some(status);
                break;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        let synchronized =
            synchronized.expect("Root must autonomously activate its Registry mirror");
        let activation = synchronized
            .activation
            .expect("Root Registry operation must retain its autonomous activation receipt");
        assert_eq!(
            activation.previous_registry,
            synchronized.synchronization.version
        );
        assert_eq!(activation.version, activated.version);

        prepare_component_registry(
            pic,
            fixture,
            RootComponentRegistryPreparationRequest {
                store_bootstrap: fixture.request.clone(),
                expected_fleet_registry: activated.version,
            },
        )
    }

    fn prepare_component_registry(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        request: RootComponentRegistryPreparationRequest,
    ) -> RootComponentRegistryPreparationRequest {
        let RootCommandResponseFragment::PrepareComponentRegistry(prepared) = root_command(
            pic,
            fixture.root_id,
            RootCommandFragment::PrepareComponentRegistry(request.clone()),
        )
        .expect("prepare root Component Registry") else {
            panic!("Root returned a differently correlated Component Registry response");
        };
        assert_eq!(prepared.fleet_subnet_root, fixture.root_id);
        assert_eq!(
            prepared.release_set,
            fixture.init_args.authority.initial_release_set
        );
        assert_eq!(
            prepared.component_topology_digest,
            fixture
                .init_args
                .authority
                .binding
                .component_topology_digest
        );
        assert_eq!(prepared.next_allocation_sequence, 1);
        assert_eq!(prepared.reserved_component_instances, 0);
        assert_eq!(prepared.committed_component_instances, 0);

        let RootCommandResponseFragment::PrepareComponentRegistry(retried) = root_command(
            pic,
            fixture.root_id,
            RootCommandFragment::PrepareComponentRegistry(request.clone()),
        )
        .expect("retry root Component Registry preparation") else {
            panic!("Root returned a differently correlated Component Registry response");
        };
        assert_eq!(retried, prepared);
        request
    }

    fn assert_component_allocation(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        component_registry_request: RootComponentRegistryPreparationRequest,
    ) -> ActiveComponentBindings {
        let issuer = provision_component(pic, fixture, [0xa1; 32]);
        let RootCommandResponseFragment::PrepareComponentRegistry(advanced) = root_command(
            pic,
            fixture.root_id,
            RootCommandFragment::PrepareComponentRegistry(component_registry_request),
        )
        .expect("read protected Component Registry after allocation without mutation") else {
            panic!("Root returned a differently correlated Component Registry status");
        };
        assert_eq!(advanced.fleet_subnet_root, fixture.root_id);
        assert_eq!(advanced.next_allocation_sequence, 2);
        assert_eq!(advanced.reserved_component_instances, 0);
        assert_eq!(advanced.committed_component_instances, 1);
        let verifier = provision_component(pic, fixture, [0xa2; 32]);
        assert_ne!(issuer.component, verifier.component);
        assert_eq!(issuer.allocation_sequence, 1);
        assert_eq!(verifier.allocation_sequence, 2);
        assert_eq!(issuer.phase, RootComponentAllocationPhase::Committed);
        assert_eq!(verifier.phase, RootComponentAllocationPhase::Committed);

        activate_root(pic, fixture.root_id);
        ActiveComponentBindings::new(
            installed_component_binding(&issuer),
            installed_component_binding(&verifier),
            [0xa1; 32],
            [0xa2; 32],
        )
    }

    fn provision_component(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
    ) -> RootComponentAllocationResponse {
        let request = RootComponentAllocationRequest {
            operation_id,
            component_spec: "issuer".parse().expect("issuer Component Spec"),
        };
        provision_component_request(pic, fixture.root_id, request)
    }

    fn provision_component_request(
        pic: &PocketIc,
        root: Principal,
        request: RootComponentAllocationRequest,
    ) -> RootComponentAllocationResponse {
        let operation_id = request.operation_id;
        let RootCommandResponseFragment::OperationAccepted(receipt) = root_command(
            pic,
            root,
            RootCommandFragment::ProvisionComponent(request.clone()),
        )
        .expect("submit Component provisioning") else {
            panic!("Root returned a differently correlated provisioning response");
        };
        assert_eq!(receipt.operation_id, operation_id);

        let mut last_allocation = None;
        for _ in 0..80 {
            let RootStatusResponseFragment::Operation(
                RootOperationStatusResponse::ProvisionComponent(status),
            ) = root_status(
                pic,
                root,
                RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
            )
            .expect("query Component provisioning")
            else {
                panic!("Root returned a differently correlated provisioning status");
            };
            if status.complete
                && status.allocation.phase == RootComponentAllocationPhase::Committed
                && status.allocation.installation.is_some()
            {
                let RootCommandResponseFragment::OperationAccepted(retried) =
                    root_command(pic, root, RootCommandFragment::ProvisionComponent(request))
                        .expect("retry Component provisioning")
                else {
                    panic!("Root returned a differently correlated provisioning response");
                };
                assert_eq!(retried, receipt);
                return status.allocation;
            }
            last_allocation = Some(status.allocation);
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }

        report_canister_diagnostics(
            pic,
            root,
            Principal::anonymous(),
            "autonomous Component provisioning",
        );
        panic!(
            "Root did not autonomously complete Component provisioning; last allocation: {last_allocation:?}"
        );
    }

    fn installed_component_binding(
        allocation: &RootComponentAllocationResponse,
    ) -> ComponentBinding {
        allocation
            .installation
            .as_ref()
            .expect("Component installation evidence")
            .binding
            .clone()
    }

    fn activate_root(pic: &PocketIc, root: Principal) {
        let RootCommandResponseFragment::OperationAccepted(preparation) =
            root_command(pic, root, RootCommandFragment::PrepareFleetActivation)
                .expect("prepare root Fleet activation")
        else {
            panic!("Root returned a differently correlated activation response");
        };
        let RootStatusResponseFragment::Operation(RootOperationStatusResponse::FleetActivation(
            prepared,
        )) = root_status(
            pic,
            root,
            RootStatusRequestFragment::Operation(OperationStatusRequest {
                operation_id: preparation.operation_id,
            }),
        )
        .expect("query prepared root Fleet activation")
        else {
            panic!("Root returned a differently correlated activation status");
        };
        assert_eq!(prepared.phase, FleetActivationPhase::Prepared);
        let credential = prepared
            .credential
            .expect("prepared root credential generation");

        let RootCommandResponseFragment::OperationAccepted(resumed) = root_command(
            pic,
            root,
            RootCommandFragment::ResumeFleetActivation(FleetActivationResumeRequest {
                operation_id: preparation.operation_id,
                credential,
            }),
        )
        .expect("resume root Fleet activation") else {
            panic!("Root returned a differently correlated activation response");
        };
        assert_eq!(resumed.operation_id, preparation.operation_id);

        for _ in 0..32 {
            let RootStatusResponseFragment::Operation(
                RootOperationStatusResponse::FleetActivation(status),
            ) = root_status(
                pic,
                root,
                RootStatusRequestFragment::Operation(OperationStatusRequest {
                    operation_id: preparation.operation_id,
                }),
            )
            .expect("query active root Fleet activation")
            else {
                panic!("Root returned a differently correlated activation status");
            };
            if status.phase == FleetActivationPhase::Active {
                return;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }

        report_canister_diagnostics(
            pic,
            root,
            Principal::anonymous(),
            "autonomous root Fleet activation",
        );
        panic!("Root did not autonomously complete Fleet activation");
    }

    fn install_bootstrapped_root(
        pic: &PocketIc,
        root_wasm: Vec<u8>,
        coordinator: Principal,
        store_fixture: RootStoreFixture,
    ) -> BootstrappedRootFixture {
        let fixture = install_bootstrapped_root_with_pool_setup(
            pic,
            root_wasm,
            coordinator,
            store_fixture,
            create_prepaid_pool_assets,
        );
        reset_prepaid_pool_assets(pic, fixture.root_id);
        fixture
    }

    struct BootstrappedRootPlacement {
        canister_pool_maximum_size: Option<u32>,
        canister_pool_minimum_size: Option<u32>,
        canister_pool_cycles: Option<canic_core::cdk::types::Cycles>,
        coordinator_subnet: Option<Principal>,
        existing_root: Option<Principal>,
        existing_wasm_store: Option<Principal>,
        root_subnet: Option<Principal>,
        component_admission_limits: Option<RootComponentAdmissionLimits>,
        fleet_id: Option<FleetId>,
        funding: Option<FleetSubnetRootFundingAuthority>,
        coordinator_root_funding: Option<FleetCoordinatorRootFundingPolicy>,
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the exact-Subnet PocketIC variants are test-only while shared fixture installation retains the optional field"
        )
    )]
    enum RootComponentAdmissionLimits {
        Uniform(u32),
    }

    #[cfg(test)]
    fn install_bootstrapped_root_for_fleet_on_subnet(
        pic: &PocketIc,
        root_wasm: Vec<u8>,
        coordinator: Principal,
        store_fixture: RootStoreFixture,
        placement_subnet: Principal,
        fleet_id_byte: u8,
    ) -> BootstrappedRootFixture {
        let coordinator_subnet = pic
            .get_subnet(coordinator)
            .expect("PocketIC Coordinator placement Subnet identity");
        let fixture = install_bootstrapped_root_on_subnet_with_pool_setup(
            pic,
            root_wasm,
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: None,
                canister_pool_minimum_size: None,
                canister_pool_cycles: None,
                coordinator_subnet: Some(coordinator_subnet),
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: Some(placement_subnet),
                component_admission_limits: Some(RootComponentAdmissionLimits::Uniform(1)),
                fleet_id: Some(FleetId::from_generated_bytes([fleet_id_byte; 32])),
                funding: None,
                coordinator_root_funding: None,
            },
            create_prepaid_pool_assets,
        );
        reset_prepaid_pool_assets(pic, fixture.root_id);
        fixture
    }

    fn install_bootstrapped_root_with_pool_setup<F>(
        pic: &PocketIc,
        root_wasm: Vec<u8>,
        coordinator: Principal,
        store_fixture: RootStoreFixture,
        pool_setup: F,
    ) -> BootstrappedRootFixture
    where
        F: FnOnce(&PocketIc, Principal) -> Vec<Principal>,
    {
        install_bootstrapped_root_on_subnet_with_pool_setup(
            pic,
            root_wasm,
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: None,
                canister_pool_minimum_size: None,
                canister_pool_cycles: None,
                coordinator_subnet: None,
                existing_root: None,
                existing_wasm_store: None,
                root_subnet: None,
                component_admission_limits: None,
                fleet_id: None,
                funding: None,
                coordinator_root_funding: None,
            },
            pool_setup,
        )
    }

    fn install_bootstrapped_root_on_subnet_with_pool_setup<F>(
        pic: &PocketIc,
        root_wasm: Vec<u8>,
        coordinator: Principal,
        store_fixture: RootStoreFixture,
        placement: BootstrappedRootPlacement,
        pool_setup: F,
    ) -> BootstrappedRootFixture
    where
        F: FnOnce(&PocketIc, Principal) -> Vec<Principal>,
    {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root");
        let config_path = root_canister_config_path(workspace_root);
        install_bootstrapped_root_with_config_and_pool_setup(
            pic,
            root_wasm,
            coordinator,
            store_fixture,
            placement,
            &config_path,
            pool_setup,
        )
    }

    fn install_bootstrapped_root_with_config_and_pool_setup<F>(
        pic: &PocketIc,
        root_wasm: Vec<u8>,
        coordinator: Principal,
        store_fixture: RootStoreFixture,
        placement: BootstrappedRootPlacement,
        config_path: &Path,
        pool_setup: F,
    ) -> BootstrappedRootFixture
    where
        F: FnOnce(&PocketIc, Principal) -> Vec<Principal>,
    {
        let InstalledRootFixture {
            root_id,
            init_args,
            coordinator_root_funding,
            manifest,
            artifacts,
            manifest_bytes,
            digest,
        } = install_current_root_with_config_and_pool_setup(
            pic,
            root_wasm,
            coordinator,
            store_fixture,
            placement,
            config_path,
            None,
            pool_setup,
        );
        let wasm_store = init_args.authority.wasm_store_authority.wasm_store;
        let installation_controller = init_args
            .authority
            .wasm_store_authority
            .installation_controller;
        let (request, response) = bootstrap_root_store_release_set(
            pic,
            root_id,
            wasm_store,
            installation_controller,
            &init_args,
            &manifest,
            artifacts,
            &manifest_bytes,
            digest,
        );
        BootstrappedRootFixture {
            root_id,
            init_args,
            coordinator_root_funding,
            request,
            response,
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the fixture binds physical placement and the independently installed Store activation identity"
    )]
    fn install_current_root_with_config_and_pool_setup<F>(
        pic: &PocketIc,
        root_wasm: Vec<u8>,
        coordinator: Principal,
        mut store_fixture: RootStoreFixture,
        placement: BootstrappedRootPlacement,
        config_path: &Path,
        store_operation_id: Option<[u8; 32]>,
        pool_setup: F,
    ) -> InstalledRootFixture
    where
        F: FnOnce(&PocketIc, Principal) -> Vec<Principal>,
    {
        let root_id = placement.existing_root.unwrap_or_else(|| {
            let root = placement.root_subnet.map_or_else(
                || pic.create_canister(),
                |subnet| pic.create_canister_on_subnet(None, None, subnet),
            );
            pic.add_cycles(root, ROOT_INSTALL_CYCLES);
            root
        });
        let root_subnet = pic.get_subnet(root_id).expect("root placement Subnet");
        let wasm_store = placement.existing_wasm_store.unwrap_or_else(|| {
            let store = pic.create_canister_on_subnet(None, None, root_subnet);
            pic.add_cycles(store, ROOT_INSTALL_CYCLES);
            store
        });
        let pool_imports = pool_setup(pic, root_id);
        let wasm_store_wasm = store_fixture
            .wasm
            .take()
            .unwrap_or_else(build_test_wasm_store_wasm);
        let installed = prepare_current_root_fixture(
            pic,
            &root_wasm,
            &wasm_store_wasm,
            coordinator,
            root_id,
            wasm_store,
            Principal::from_slice(&[0x46; 29]),
            store_fixture,
            &placement,
            config_path,
            pool_imports,
        );
        let installation_controller = installed
            .init_args
            .authority
            .wasm_store_authority
            .installation_controller;
        let store_init_args = FleetSubnetWasmStoreInitArgs {
            authority: installed.init_args.authority.wasm_store_authority.clone(),
            install_id: store_operation_id
                .unwrap_or(installed.init_args.wasm_store_activation.operation_id),
        };
        prepare_sibling_wasm_store_controllers(pic, wasm_store, installation_controller, root_id);
        pic.install_canister(
            wasm_store,
            wasm_store_wasm,
            encode_one(store_init_args).expect("encode live PocketIC Store authority"),
            Some(installation_controller),
        );
        let init_bytes =
            encode_one(&installed.init_args).expect("encode live PocketIC root authority");
        pic.install_canister(root_id, root_wasm, init_bytes, None);
        installed
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one fixture compiler binds the complete Root/Store installation authority"
    )]
    fn prepare_current_root_fixture(
        pic: &PocketIc,
        root_wasm: &[u8],
        wasm_store_wasm: &[u8],
        coordinator: Principal,
        root_id: Principal,
        wasm_store: Principal,
        installation_controller: Principal,
        store_fixture: RootStoreFixture,
        placement: &BootstrappedRootPlacement,
        config_path: &Path,
        pool_imports: Vec<Principal>,
    ) -> InstalledRootFixture {
        let RootStoreFixture {
            manifest,
            artifacts,
            ..
        } = store_fixture;
        let manifest_bytes = serde_json::to_vec(&manifest).expect("canonical root release set");
        let digest = ReleaseSetDigest::from_bytes(
            wasm_hash(&manifest_bytes)
                .try_into()
                .expect("SHA-256 digest"),
        );
        let init_bytes =
            install_root_args_with_release_set_digest_and_coordinator(ManagedRootInstallInput {
                root_id,
                wasm_store,
                installation_controller,
                coordinator,
                root_wasm,
                wasm_store_wasm,
                config_path,
                release_set_digest: digest,
            })
            .expect("encode exact root authority");
        let mut init_args =
            decode_one::<FleetSubnetRootInitArgs>(&init_bytes).expect("decode root init authority");
        let pool = &mut init_args.authority.binding.limits.canister_pool;
        if let Some(maximum_size) = placement.canister_pool_maximum_size {
            pool.maximum_size = maximum_size;
        }
        if let Some(minimum_size) = placement.canister_pool_minimum_size {
            pool.minimum_size = minimum_size;
        }
        if let Some(canister_cycles) = placement.canister_pool_cycles.clone() {
            pool.canister_cycles = canister_cycles;
        }
        if let Some(funding) = placement.funding.clone() {
            init_args.authority.binding.funding = funding;
        }
        bind_fixture_fleet_id(&mut init_args, placement.fleet_id);
        if let Some(component_admission_limits) = &placement.component_admission_limits {
            for admission in &mut init_args.authority.binding.component_admissions {
                admission.maximum_root_instances = match component_admission_limits {
                    RootComponentAdmissionLimits::Uniform(limit) => *limit,
                };
            }
            let config = AppConfigSnapshot::load(config_path).expect("reload root config");
            init_args.authority.binding.component_topology_digest = config
                .component_topology()
                .project_for_admissions(&init_args.authority.binding.component_admissions)
                .and_then(|projection| projection.digest())
                .expect("compile bounded multi-root topology digest");
            init_args.wasm_store_activation.component_topology_digest =
                init_args.authority.binding.component_topology_digest;
        }
        bind_init_args_to_pocket_ic_subnet(
            pic,
            root_id,
            placement.coordinator_subnet,
            &mut init_args,
        );
        init_args.canister_pool_imports = pool_imports;
        InstalledRootFixture {
            root_id,
            init_args,
            coordinator_root_funding: placement
                .coordinator_root_funding
                .clone()
                .unwrap_or_else(crate::pic::coordinator_root_funding_policy),
            manifest,
            artifacts,
            manifest_bytes,
            digest,
        }
    }

    const fn bind_fixture_fleet_id(
        init_args: &mut FleetSubnetRootInitArgs,
        fleet_id: Option<FleetId>,
    ) {
        let Some(fleet_id) = fleet_id else {
            return;
        };
        init_args
            .authority
            .binding
            .authority
            .binding
            .fleet
            .fleet
            .fleet_id = fleet_id;
        init_args.wasm_store_activation.fleet.fleet.fleet_id = fleet_id;
        init_args
            .authority
            .wasm_store_authority
            .authority
            .binding
            .fleet
            .fleet
            .fleet_id = fleet_id;
    }

    #[cfg(test)]
    const fn rebind_fixture_release_build_id(
        init_args: &mut FleetSubnetRootInitArgs,
        release_build_id: ReleaseBuildId,
    ) {
        init_args.authority.initial_release_set.release_build_id = release_build_id;
        init_args.authority.wasm_store_authority.release_build_id = release_build_id;
        init_args.wasm_store_activation.release_build_id = release_build_id;
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the PocketIC fixture stages one explicit Store release-set boundary"
    )]
    fn bootstrap_root_store_release_set(
        pic: &PocketIc,
        root_id: Principal,
        store: Principal,
        installation_controller: Principal,
        init_args: &FleetSubnetRootInitArgs,
        manifest: &RootStoreReleaseSetManifest,
        artifacts: BTreeMap<CanisterRole, Vec<u8>>,
        manifest_bytes: &[u8],
        digest: ReleaseSetDigest,
    ) -> (RootStoreBootstrapRequest, RootStoreBootstrapResponse) {
        let version = TemplateVersion::owned(manifest.release_build_id.to_string());
        stage_chunked_payload(
            pic,
            store,
            installation_controller,
            TemplateId::owned(format!("{ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX}{digest}")),
            version.clone(),
            manifest_bytes,
        );
        for (role, bytes) in artifacts {
            let template_id =
                TemplateId::owned(format!("{ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX}{role}"));
            let staged = store_stage_manifest_as(
                pic,
                store,
                installation_controller,
                TemplateManifestInput {
                    template_id: template_id.clone(),
                    role,
                    version: version.clone(),
                    payload_hash: wasm_hash(&bytes),
                    payload_size_bytes: bytes.len() as u64,
                    store_binding: WasmStoreBinding::new("bootstrap"),
                    chunking_mode: TemplateChunkingMode::Chunked,
                    manifest_state: TemplateManifestState::Approved,
                    approved_at: Some(0),
                    created_at: 0,
                },
            );
            staged.expect("stage artifact manifest");
            stage_chunked_payload(
                pic,
                store,
                installation_controller,
                template_id,
                version.clone(),
                &bytes,
            );
        }

        adopt_sibling_wasm_store(pic, root_id, init_args);
        assert_prepared(pic, root_id);

        let request = RootStoreBootstrapRequest {
            operation_id: [37; 32],
            manifest_payload_size_bytes: manifest_bytes.len() as u64,
        };
        for source in &manifest.fixtures {
            qualify_root_fixture_publication(
                pic,
                root_id,
                store,
                installation_controller,
                &request,
                source,
            );
        }
        let RootCommandResponseFragment::OperationAccepted(receipt) = root_command(
            pic,
            root_id,
            RootCommandFragment::BootstrapStore(request.clone()),
        )
        .expect("root Store bootstrap") else {
            panic!("Root returned a differently correlated bootstrap response");
        };
        assert_eq!(receipt.operation_id, request.operation_id);
        let RootStatusResponseFragment::Operation(RootOperationStatusResponse::BootstrapStore(
            response,
        )) = root_status(
            pic,
            root_id,
            RootStatusRequestFragment::Operation(OperationStatusRequest {
                operation_id: request.operation_id,
            }),
        )
        .expect("root Store bootstrap status")
        else {
            panic!("Root returned a differently correlated bootstrap status");
        };
        (request, response)
    }

    fn qualify_root_fixture_publication(
        pic: &PocketIc,
        root: Principal,
        store: Principal,
        controller: Principal,
        bootstrap: &RootStoreBootstrapRequest,
        source: &canic::dto::root_store::RootStoreFixture,
    ) {
        let request = RootStoreFixturePrepareRequest {
            bootstrap: bootstrap.clone(),
            role: source.role.clone(),
        };
        let unknown = RootStoreFixturePrepareRequest {
            role: CanisterRole::new("not_selected"),
            ..request.clone()
        };
        assert!(matches!(
            root_command(pic, root, RootCommandFragment::PrepareStoreFixture(unknown)).unwrap(),
            RootCommandResponseFragment::PrepareStoreFixture(Err(FixtureStoreError::NotFound))
        ));
        let mut wrong_size = request.clone();
        wrong_size.bootstrap.manifest_payload_size_bytes += 1;
        assert!(
            root_command(
                pic,
                root,
                RootCommandFragment::PrepareStoreFixture(wrong_size)
            )
            .is_err()
        );
        let RootCommandResponseFragment::PrepareStoreFixture(Ok(prepared)) = root_command(
            pic,
            root,
            RootCommandFragment::PrepareStoreFixture(request.clone()),
        )
        .unwrap() else {
            panic!("Root fixture preparation failed")
        };
        assert_eq!(prepared.content_id, source.content_id);
        assert_eq!(prepared.next_chunk, 0);
        assert!(!prepared.complete);
        // Ignore the first acknowledgement and recover only from a repeated command.
        let RootCommandResponseFragment::PrepareStoreFixture(Ok(repeated)) = root_command(
            pic,
            root,
            RootCommandFragment::PrepareStoreFixture(request.clone()),
        )
        .unwrap() else {
            panic!("Root fixture preparation replay failed")
        };
        assert_eq!(repeated, prepared);
        assert!(
            root_command(
                pic,
                root,
                RootCommandFragment::BootstrapStore(bootstrap.clone())
            )
            .is_err()
        );
        assert_prepared(pic, root);
        let upload = FixtureChunkUpload {
            content_id: source.content_id,
            index: 0,
            bytes: b"reviewed fixture source".to_vec(),
        };
        let uploaded: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> = pic
            .update_candid_as(
                store,
                controller,
                canic::protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
                (upload.clone(),),
            )
            .unwrap();
        let uploaded = uploaded.unwrap().unwrap();
        assert!(uploaded.complete);
        let replayed: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> = pic
            .update_candid_as(
                store,
                controller,
                canic::protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
                (upload,),
            )
            .unwrap();
        assert_eq!(replayed.unwrap().unwrap(), uploaded);
        let RootCommandResponseFragment::PrepareStoreFixture(Ok(recovered)) =
            root_command(pic, root, RootCommandFragment::PrepareStoreFixture(request)).unwrap()
        else {
            panic!("Root completed fixture observation failed")
        };
        assert_eq!(recovered, uploaded);
    }

    fn create_prepaid_pool_assets(pic: &PocketIc, root: Principal) -> Vec<Principal> {
        let root_subnet = pic.get_subnet(root).expect("root placement Subnet");
        (0..PREPAID_POOL_ASSET_COUNT)
            .map(|_| {
                let canister = pic.create_canister_on_subnet(None, None, root_subnet);
                pic.add_cycles(canister, PREPAID_POOL_ASSET_CYCLES);
                pic.set_controllers(canister, None, vec![root])
                    .expect("prepare root-owned prepaid Canister");
                canister
            })
            .collect()
    }

    fn reset_prepaid_pool_assets(pic: &PocketIc, root: Principal) {
        reset_prepaid_pool_assets_for_count(pic, root, PREPAID_POOL_ASSET_COUNT);
    }

    fn reset_prepaid_pool_assets_for_count(pic: &PocketIc, root: Principal, count: usize) {
        reset_prepaid_pool_assets_for_count_as(pic, root, Principal::anonymous(), count);
    }

    fn reset_prepaid_pool_assets_for_count_as(
        pic: &PocketIc,
        root: Principal,
        caller: Principal,
        count: usize,
    ) {
        for _ in 0..count {
            let RootCommandResponseFragment::MaintainPool(response) =
                root_command_as(pic, root, caller, RootCommandFragment::MaintainPool)
                    .expect("reset prepaid Canister")
            else {
                panic!("Root returned a differently correlated pool response");
            };
            assert!(
                matches!(
                    response,
                    PoolMaintenanceResponse::ResetReady { .. }
                        | PoolMaintenanceResponse::Maintained
                ),
                "unexpected pool reset response: {response:?}"
            );
        }
        let status = root_pool_status_as(pic, root, caller);
        assert_eq!(
            status.ready,
            u32::try_from(count).expect("bounded fixture pool size")
        );
        assert_eq!(status.pending_reset, 0);
    }

    fn bind_init_args_to_pocket_ic_subnet(
        pic: &PocketIc,
        root_id: Principal,
        coordinator_subnet: Option<Principal>,
        init_args: &mut FleetSubnetRootInitArgs,
    ) {
        let root_subnet = SubnetId::from_principal(
            pic.get_subnet(root_id)
                .expect("PocketIC root placement Subnet identity"),
        );
        let coordinator_subnet = coordinator_subnet.map_or(root_subnet, SubnetId::from_principal);
        init_args.authority.binding.placement_subnet = root_subnet;
        init_args
            .authority
            .binding
            .authority
            .binding
            .coordinator_subnet = coordinator_subnet;
        init_args.authority.wasm_store_authority.placement_subnet = root_subnet;
        init_args
            .authority
            .wasm_store_authority
            .authority
            .binding
            .coordinator_subnet = coordinator_subnet;
    }

    fn build_test_coordinator_wasm() -> Vec<u8> {
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        build_canonical_fleet_coordinator_wasm(&workspace_root)
    }

    fn build_root_store_fixture() -> RootStoreFixture {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root");
        let config_path = root_canister_config_path(workspace_root);
        build_root_store_fixture_with_config(&config_path, build_test_component_wasms())
    }

    fn build_root_store_fixture_with_config(
        config_path: &Path,
        component_wasms: &BTreeMap<CanisterRole, Vec<u8>>,
    ) -> RootStoreFixture {
        build_root_store_fixture_with_config_for_release(
            config_path,
            component_wasms,
            managed_test_init_identity().release_build_id,
        )
    }

    fn build_root_store_fixture_with_config_for_release(
        config_path: &Path,
        component_wasms: &BTreeMap<CanisterRole, Vec<u8>>,
        release_build_id: ReleaseBuildId,
    ) -> RootStoreFixture {
        let (manifest, artifacts) =
            exact_root_store_fixture(config_path, component_wasms, release_build_id);
        RootStoreFixture {
            wasm: None,
            manifest,
            artifacts,
        }
    }

    fn exact_root_store_fixture(
        config_path: &Path,
        real_modules: &BTreeMap<CanisterRole, Vec<u8>>,
        release_build_id: ReleaseBuildId,
    ) -> (RootStoreReleaseSetManifest, BTreeMap<CanisterRole, Vec<u8>>) {
        let config = AppConfigSnapshot::load(config_path).expect("load root fixture config");
        let topology = config.component_topology();
        let mut entries = Vec::new();
        let mut artifacts = BTreeMap::new();
        for spec in &topology.component_specs {
            entries.push(root_store_entry(
                config.model(),
                &spec.component_spec,
                RootStoreReleaseSetEntryKind::Component,
                &spec.component_role,
                release_build_id,
                real_modules,
                &mut artifacts,
            ));
            entries.extend(spec.children.iter().map(|child| {
                root_store_entry(
                    config.model(),
                    &spec.component_spec,
                    RootStoreReleaseSetEntryKind::ComponentChild,
                    &child.role,
                    release_build_id,
                    real_modules,
                    &mut artifacts,
                )
            }));
        }

        (
            RootStoreReleaseSetManifest {
                fixtures: Vec::new(),
                release_build_id,
                component_topology_digest: topology.digest().expect("fixture topology digest"),
                entries,
            },
            artifacts,
        )
    }

    fn root_store_entry(
        config: &canic_core::bootstrap::compiled::ConfigModel,
        component_spec: &canic_core::ids::ComponentSpecId,
        kind: RootStoreReleaseSetEntryKind,
        role: &CanisterRole,
        release_build_id: canic_core::ids::ReleaseBuildId,
        real_modules: &BTreeMap<CanisterRole, Vec<u8>>,
        artifacts: &mut BTreeMap<CanisterRole, Vec<u8>>,
    ) -> RootStoreReleaseSetEntry {
        let raw = real_modules
            .get(role)
            .cloned()
            .unwrap_or_else(|| format!("raw fixture for {role}").into_bytes());
        let compressed = gzip(&raw);
        let existing = artifacts.insert(role.clone(), compressed.clone());
        assert!(
            existing.as_ref().is_none_or(|bytes| bytes == &compressed),
            "one role must retain one exact artifact payload"
        );
        let declared_package = config
            .roles
            .get(role)
            .expect("fixture role declaration")
            .package
            .as_deref()
            .expect("application fixture package");
        RootStoreReleaseSetEntry {
            component_spec: component_spec.clone(),
            kind,
            artifact: RootStoreArtifact {
                role: role.clone(),
                package: Path::new(declared_package)
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .expect("fixture role package has a canonical Cargo identity")
                    .to_string(),
                release_build_id,
                wasm_relative_path: format!("{role}.wasm"),
                wasm_size_bytes: raw.len() as u64,
                wasm_sha256_hex: hex_bytes(wasm_hash(&raw)),
                wasm_gz_relative_path: format!("{role}.wasm.gz"),
                wasm_gz_size_bytes: compressed.len() as u64,
                wasm_gz_sha256_hex: hex_bytes(wasm_hash(&compressed)),
                candid_sha256: [3; 32],
                protocol_profile_digest:
                    canic_core::role_contract::ProtocolProfileDigest::from_bytes([4; 32]),
            },
        }
    }

    fn build_test_component_wasms() -> &'static BTreeMap<CanisterRole, Vec<u8>> {
        static WASMS: OnceLock<BTreeMap<CanisterRole, Vec<u8>>> = OnceLock::new();
        WASMS.get_or_init(|| {
            let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
            let config_path = root_canister_config_path(&workspace_root);
            build_component_fixture_wasms(
                &workspace_root,
                &config_path,
                "fleet-registry-sync",
                &[("issuer", ISSUER_PACKAGE)],
            )
        })
    }

    #[cfg(test)]
    fn build_five_component_wasms() -> &'static BTreeMap<CanisterRole, Vec<u8>> {
        static WASMS: OnceLock<BTreeMap<CanisterRole, Vec<u8>>> = OnceLock::new();
        WASMS.get_or_init(|| {
            let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
            let config_path = five_component_root_canister_config_path(&workspace_root);
            build_component_fixture_wasms(
                &workspace_root,
                &config_path,
                "fleet-registry-five-components",
                &[("issuer", ISSUER_PACKAGE)],
            )
        })
    }

    #[cfg(test)]
    fn build_initial_shard_component_wasms() -> &'static BTreeMap<CanisterRole, Vec<u8>> {
        static WASMS: OnceLock<BTreeMap<CanisterRole, Vec<u8>>> = OnceLock::new();
        WASMS.get_or_init(|| {
            let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
            let config_path = initial_shard_root_canister_config_path(&workspace_root);
            build_component_fixture_wasms(
                &workspace_root,
                &config_path,
                "fleet-registry-initial-shard",
                &[
                    ("index_child", "canister_index_child"),
                    ("index_hub", "canister_index_hub"),
                    ("scale_hub", "canister_scale_hub"),
                    ("scale_replica", "canister_scale"),
                    ("user_hub", "canister_user_hub"),
                    ("user_shard", "canister_user_shard"),
                ],
            )
        })
    }

    #[cfg(test)]
    fn build_five_trillion_component_wasms() -> &'static BTreeMap<CanisterRole, Vec<u8>> {
        static WASMS: OnceLock<BTreeMap<CanisterRole, Vec<u8>>> = OnceLock::new();
        WASMS.get_or_init(|| {
            let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
            let config_path = five_trillion_component_root_canister_config_path(&workspace_root);
            build_component_fixture_wasms(
                &workspace_root,
                &config_path,
                "fleet-registry-five-trillion-component",
                &[("issuer", ISSUER_PACKAGE)],
            )
        })
    }

    fn build_component_fixture_wasms(
        workspace_root: &Path,
        config_path: &Path,
        target_scope: &str,
        roles_and_packages: &[(&'static str, &'static str)],
    ) -> BTreeMap<CanisterRole, Vec<u8>> {
        let target_dir = test_target_dir(workspace_root, target_scope);
        let canonical_config_path = config_path.to_str().expect("root config path UTF-8");
        let packages = roles_and_packages
            .iter()
            .map(|(_, package)| *package)
            .collect::<Vec<_>>();
        let wasms = build_internal_test_wasm_canisters_with_env(
            workspace_root,
            &target_dir,
            &packages,
            CanicWasmBuildProfile::Fast,
            &[(
                canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
                canonical_config_path,
            )],
        );
        roles_and_packages
            .iter()
            .map(|(role, package)| (CanisterRole::new(role), wasms.wasm(package)))
            .collect()
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes).expect("gzip fixture Wasm");
        encoder.finish().expect("finish fixture Wasm gzip")
    }

    fn stage_chunked_payload(
        pic: &PocketIc,
        store: Principal,
        installation_controller: Principal,
        template_id: TemplateId,
        version: TemplateVersion,
        payload: &[u8],
    ) {
        let chunks = payload
            .chunks(CANIC_WASM_CHUNK_BYTES)
            .map(<[u8]>::to_vec)
            .collect::<Vec<_>>();
        let prepared = store_prepare_as(
            pic,
            store,
            installation_controller,
            TemplateChunkSetPrepareInput {
                template_id: template_id.clone(),
                version: version.clone(),
                payload_hash: wasm_hash(payload),
                payload_size_bytes: payload.len() as u64,
                chunk_hashes: chunks.iter().map(|chunk| wasm_hash(chunk)).collect(),
            },
        );
        prepared.expect("prepare staged payload");
        for (chunk_index, bytes) in chunks.into_iter().enumerate() {
            let published: Result<(), Error> = pic
                .update_candid_as(
                    store,
                    installation_controller,
                    canic::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
                    (TemplateChunkInput {
                        template_id: template_id.clone(),
                        version: version.clone(),
                        chunk_index: u32::try_from(chunk_index).expect("bounded chunk index"),
                        bytes,
                    },),
                )
                .expect("publish staged payload transport");
            published.expect("publish staged payload");
        }
    }

    fn assert_prepared(pic: &PocketIc, root_id: Principal) {
        let RootStatusResponseFragment::FleetAuthority(authority) =
            root_status(pic, root_id, RootStatusRequestFragment::FleetAuthority)
                .expect("query root authority")
        else {
            panic!("Root returned a differently correlated authority status");
        };
        assert_eq!(authority.binding.fleet_subnet_root, root_id);
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "the governed inventory is one explicit ordered list of every serial case"
    )]
    pub fn governed_pocketic_cases() -> Vec<crate::pic::GovernedTestCase> {
        vec![
            (
                "Fleet deployment restore",
                restored_root_preserves_its_inventory_but_cannot_allocate,
            ),
            (
                "autonomous Root removal",
                published_draining_root_autonomously_reaches_external_deletion_readiness,
            ),
            (
                "initial child failure reaches Coordinator and recovers same claim",
                child_reserve::initial_child_failure_reaches_coordinator_and_recovers_same_claim,
            ),
            (
                "low native reserve retains child failure and recovers same claim",
                child_reserve::low_native_reserve_retains_child_failure_and_recovers_same_claim,
            ),
            (
                "sibling topups retain distinct receipts after both replies are lost",
                sibling_funding::sibling_topups_retain_distinct_receipts_after_both_replies_are_lost,
            ),
            (
                "child grant refreshes Root funding deadline without repeating credit",
                funding_deadline::child_grant_refreshes_root_funding_deadline_without_repeating_credit,
            ),
            (
                "live Fleet state cascade preserves partial outcomes and retry",
                state_cascade::live_state_cascade_preserves_partial_outcomes_and_retry,
            ),
            (
                "operator Component public CLI",
                operator_component_public_cli_uses_real_icp_and_exports_terminal_binding,
            ),
            (
                "operator Component lost-response recovery",
                operator_component_recovers_lost_response_and_replays_terminal_binding,
            ),
            (
                "protected current memory allocations",
                protected_memory_allocations_preserve_stable_state_and_root_authority,
            ),
            (
                "reinstall fixture release-cache identity",
                reinstall_fixture_release_cache_binds_distinct_repeatable_identities,
            ),
            (
                "synthetic growth catalog agreement",
                crate::pic::fleet_registry::growth::synthetic_growth_catalog_meets_host_agreement_policy,
            ),
            (
                "explicit Root reinstall preserves cycle control",
                explicit_root_reinstall_retains_cycle_accounts_and_pool_control,
            ),
            (
                "prepared mainnet Root automatic refill",
                prepared_mainnet_root_automatically_refills_one_exact_pool_asset,
            ),
            (
                "autonomous refill margin and exact replay",
                autonomous_refill_margin_survives_burn_and_replays_without_another_debit,
            ),
            (
                "topped-up imported pool asset refresh",
                historical_pool_assets_upgrade_refresh_and_claim_without_losing_cycles,
            ),
            (
                "fresh provisioning automatic pool readiness",
                fresh_five_component_acceptance_seeds_the_root_owned_pool_before_effects,
            ),
            (
                "fixture-bearing Root retirement",
                fixture_bearing_root_retirement_conserves_assets_and_cycles,
            ),
            (
                "pending fixture automatic funding",
                pending_fixture_automatically_funds_within_configured_allowance,
            ),
            (
                "Root restart during Store grant and revocation replies",
                root_restart_reconciles_held_store_grant_and_revocation_replies,
            ),
            (
                "Prepared Root initial-Shard bootstrap",
                prepared_root_initial_shard_bootstrap_reaches_terminal_component_membership,
            ),
            (
                "application Store bootstrap and replay",
                current_store_bootstraps_application_catalog_and_replays_zero_effects,
            ),
            (
                "fresh provisioning terminal runtime activation",
                fresh_five_component_provisioning_reaches_runtime_active_and_publishes_catalog,
            ),
            (
                "inactive Root preserves and suspends permanent activation failure",
                inactive_root_retains_permanent_store_failure_and_suspends_retries,
            ),
            (
                "auth-free Root preserves ordinary Fleet activation",
                auth_free_root_preserves_ordinary_fleet_activation,
            ),
            (
                "live management gateway preserves fresh creation headroom",
                live_management_gateway_preserves_fresh_creation_headroom,
            ),
            (
                "Coordinator attached-cycle grant",
                real_coordinator_funds_one_active_root_exactly_once,
            ),
            (
                "two-Root independent funding limits",
                two_roots_use_independent_limits_and_one_coordinator_budget,
            ),
            (
                "non-renewing automatic grant cap",
                automatic_grant_cap_never_renews_after_the_ninety_day_window,
            ),
            (
                "explicit funding-policy rotation",
                explicit_policy_rotation_reopens_exhausted_automatic_funding_once,
            ),
            (
                "Coordinator reserve-denial ICP fallback",
                terminal_coordinator_reserve_denial_runs_one_real_icp_fallback,
            ),
            (
                "real ICP rate denial",
                real_rate_gate_denial_spends_no_icp_and_creates_no_refill,
            ),
            (
                "insufficient real ICP denial",
                insufficient_real_icp_spends_nothing_and_creates_no_refill,
            ),
            (
                "uncertain grant fallback suppression",
                uncertain_grant_suppresses_icp_and_direct_topup_remains_available,
            ),
            (
                "production Ledger and CMC replay",
                production_ledger_and_cmc_exact_replay_never_duplicates_value,
            ),
            (
                "qualification Ledger cohort isolation",
                qualification_ledger_preflight_keeps_1_8_16_32_lanes_independent,
            ),
            (
                "qualification reset cohort isolation",
                qualification_reset_preflight_keeps_1_8_16_32_lanes_independent,
            ),
            (
                "qualification effect arithmetic",
                qualification_external_effect_envelope_uses_checked_arithmetic,
            ),
            (
                "qualification controller transition",
                qualification_controller_transition_requires_exact_routing_evidence,
            ),
            (
                "prepared Root local Store",
                prepared_root_bootstraps_and_reverifies_its_exact_local_store,
            ),
            (
                "co-located Fleet isolation",
                co_located_fleets_keep_roots_stores_pools_and_registries_isolated,
            ),
            (
                "active Component Registry attestations",
                active_registry_issues_component_role_attestations,
            ),
            (
                "Fleet admission add/remove convergence",
                fleet_admission_add_and_remove_converge_across_real_root_and_components,
            ),
            (
                "Fleet admission unavailable participant recovery",
                unavailable_admission_participant_blocks_activation_until_exact_retry,
            ),
            (
                "Fleet admission stale-catalog release",
                fleet_admission_catalog_change_releases_before_effect_and_retries_exactly,
            ),
            (
                "Fleet admission two-Root convergence",
                fleet_admission_add_and_remove_converge_across_two_roots,
            ),
        ]
    }

    #[cfg(test)]
    pub fn governed_fleet_journey_cases() -> Vec<crate::pic::GovernedTestCase> {
        vec![
            (
                "source-bound activation reset recovers and replays",
                activation_reset::source_bound_activation_reset_recovers_and_replays,
            ),
            (
                "generated reinstall recovers and converges",
                generated_reinstall_recovers_lost_install_and_reaches_working_fleet,
            ),
            (
                "four initial Shards preserve sealed Root activation",
                four_initial_shards_preserve_sealed_root_activation,
            ),
            (
                "funded Failed imports recover withdrawal and reset responses",
                funded_failed_imports_reconcile_with_lost_withdrawal_and_reset_responses,
            ),
            (
                "funded estate recovers transfer and autonomous creation responses",
                funded_estate_recovers_transfer_and_autonomous_creation_responses,
            ),
            (
                "issued provisioning recovers native withdrawal and receipt observation",
                native_funding::issued_provisioning_recovers_native_withdrawal_and_receipt_observation,
            ),
            (
                "native withdrawal recovers the same initial child claim",
                native_funding::native_withdrawal_recovers_the_same_initial_child_claim,
            ),
            (
                "issued creation funding pause resumes reviewed transfer",
                issued_creation_funding_pause_resumes_reviewed_transfer,
            ),
            (
                "four Workloads refill four Ready assets with lost funding and creation responses",
                four_workloads_refill_four_ready_with_lost_funding_and_creation_responses,
            ),
            (
                "four Workloads and four Failed assets repair without new creation",
                four_workloads_and_four_failed_assets_repair_without_new_creation,
            ),
            (
                "generated mixed topology and Ready reserve retain one reviewed operation",
                generated_mixed_topology_and_ready_reserve_recover_one_reviewed_operation,
            ),
        ]
    }
}

pub use tests::{
    ActiveComponentRegistryFixture, setup_active_component_registry,
    setup_fresh_active_component_registry,
};
