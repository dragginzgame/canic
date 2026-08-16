//! Prepared-root Fleet Registry and Component Registry PocketIC journey.

#[cfg(test)]
use super::build::build_mainnet_refill_wasms;
use super::build::{
    build_pic, build_test_root_wasm, build_test_wasm_store_wasm, root_canister_config_path,
};
#[cfg(test)]
use super::build::{
    build_test_toko_root_wasm, build_three_application_subnet_pic,
    build_two_application_subnet_pic, toko_root_canister_config_path,
};
use candid::Principal;
use ic_testkit::pic::{CandidCallExt, PocketIc};
use std::path::{Path, PathBuf};

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;
const PREPAID_POOL_ASSET_COUNT: usize = 10;
const PREPAID_POOL_ASSET_CYCLES: u128 = 6_000_000_000_000;

mod tests {
    use super::*;
    #[cfg(test)]
    use candid::CandidType;
    use candid::{decode_one, encode_one};
    #[cfg(test)]
    use canic::dto::pool::{
        CanisterPoolAssetOrigin, CanisterPoolAssetStatus, CanisterPoolClaim,
        CanisterPoolRecycleReset,
    };
    use canic::dto::pool::{
        CanisterPoolResponse, CanisterPoolStatusRequest, PoolAdminCommand, PoolAdminResponse,
    };
    #[cfg(test)]
    use canic::ids::ComponentChildBinding;
    use canic::protocol::{CANIC_POOL_ADMIN, CANIC_POOL_LIST};
    use canic::{
        CANIC_WASM_CHUNK_BYTES,
        dto::{
            component_registry::{
                ComponentDirectoryHead, ComponentDirectoryHeadRequest, ComponentLifecycleStatus,
                ComponentProvisioningOrigin, ComponentRegistryPartitionRequest,
                ComponentRegistryPartitionResponse, ComponentRuntimePhase,
                ComponentRuntimeStatusResponse, RootComponentAllocationPhase,
                RootComponentAllocationRequest, RootComponentAllocationResponse,
                RootComponentAllocationStatusRequest, RootComponentCommitRequest,
                RootComponentCommitResponse, RootComponentCreationRequest,
                RootComponentDirectoryPreparationRequest,
                RootComponentDirectoryPreparationResponse, RootComponentInitialInventoryStatus,
                RootComponentInstallRequest, RootComponentMembershipActivationRequest,
                RootComponentMembershipActivationResponse, RootComponentRegistryPreparationRequest,
                RootComponentRegistryStatusResponse, RootComponentRuntimeActivationRequest,
                RootComponentRuntimeActivationResponse,
            },
            fleet_registry::{
                FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistry,
                FleetRegistryActivationRequest, FleetRegistryActivationResponse,
                FleetSubnetRootDirectoryEntry, FleetSubnetRootEntry, FleetSubnetRootJoinRequest,
                FleetSubnetRootJoinResponse, FleetSubnetRootRegistryMirrorActivationRequest,
                FleetSubnetRootRegistryMirrorActivationResponse,
                FleetSubnetRootRegistrySyncRequest, FleetSubnetRootRegistrySyncResponse,
                FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootStatus,
            },
            fleet_subnet_root::{
                FleetSubnetRootAuthority, FleetSubnetRootCanisterSummary, FleetSubnetRootInitArgs,
                FleetSubnetWasmStoreInitArgs,
            },
            root_store::{
                ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX, ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX,
                RootStoreArtifact, RootStoreBootstrapRequest, RootStoreBootstrapResponse,
                RootStoreReleaseSetEntry, RootStoreReleaseSetEntryKind,
                RootStoreReleaseSetManifest,
            },
        },
        ids::{
            CanisterRole, ComponentBinding, ComponentInstanceId, ComponentSpecId, FleetId,
            ManagedCanisterBinding, ReleaseSetDigest, SubnetId,
        },
    };
    use canic::{
        Error,
        dto::fleet_activation::{
            FleetActivationPhase, FleetActivationResumeRequest, FleetActivationStatusResponse,
        },
        protocol::{
            CANIC_COMPONENT_RUNTIME_STATUS, CANIC_FLEET_ACTIVATION_STATUS, CANIC_FLEET_REGISTRY,
            CANIC_FLEET_REGISTRY_ACTIVATE, CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
            CANIC_FLEET_REGISTRY_MIRROR_STATUS, CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS,
            CANIC_FLEET_REGISTRY_SYNC_STATUS, CANIC_FLEET_REGISTRY_SYNCHRONIZE,
            CANIC_FLEET_REGISTRY_VERSION, CANIC_FLEET_SUBNET_ROOT_AUTHORITY,
            CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY, CANIC_FLEET_SUBNET_ROOT_JOIN,
            CANIC_PREPARE_FLEET_ACTIVATION, CANIC_RESUME_FLEET_ACTIVATION,
            CANIC_ROOT_COMPONENT_ALLOCATE, CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
            CANIC_ROOT_COMPONENT_COMMIT, CANIC_ROOT_COMPONENT_CREATE,
            CANIC_ROOT_COMPONENT_DIRECTORY_HEAD, CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
            CANIC_ROOT_COMPONENT_INSTALL, CANIC_ROOT_COMPONENT_MEMBERSHIP_ACTIVATE,
            CANIC_ROOT_COMPONENT_REGISTRY_PARTITION, CANIC_ROOT_COMPONENT_REGISTRY_PREPARE,
            CANIC_ROOT_COMPONENT_REGISTRY_STATUS, CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE,
            CANIC_ROOT_STORE_BOOTSTRAP, CANIC_TEMPLATE_PREPARE_ADMIN,
            CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN, CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
        },
    };
    #[cfg(test)]
    use canic::{
        dto::{
            authority_restore::{
                AuthorityRestoreFencePhase, AuthorityRestoreFenceStatusResponse,
                AuthoritySnapshotRequest,
            },
            component_deployment::ProtectedComponentDeployment,
            component_provisioning::{
                ComponentGroupPlacementPlan, ComponentGroupPlanEntry,
                FleetComponentProvisioningAdvanceRequest, FleetComponentProvisioningOperation,
                FleetComponentProvisioningPhase, FleetComponentProvisioningPlan,
                FleetComponentProvisioningPrepareRequest, FleetComponentProvisioningStatusResponse,
                FleetSubnetRootProvisioningBatch, RootComponentProvisioningAcceptanceRequest,
                RootComponentProvisioningAdvanceRequest, RootComponentProvisioningPhase,
                RootComponentProvisioningStatusRequest, RootComponentProvisioningStatusResponse,
            },
            component_registry::{PeerComponentRequester, RootPeerComponentAllocationRequest},
            fleet_registry::{FleetRegistryVersion, FleetServiceMode},
            placement::index::PlacementIndexStatusResponse,
        },
        ids::ComponentGroupPlacementId,
        protocol::{
            CANIC_AUTHORITY_RESTORE_FENCE_STATUS, CANIC_AUTHORITY_SNAPSHOT_PREPARE,
            CANIC_AUTHORITY_SNAPSHOT_RESUME, CANIC_FLEET_COMPONENT_PROVISIONING_ADVANCE,
            CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE, CANIC_ROOT_COMPONENT_PROVISIONING_ACCEPT,
            CANIC_ROOT_COMPONENT_PROVISIONING_ADVANCE, CANIC_ROOT_COMPONENT_PROVISIONING_STATUS,
        },
    };
    use canic_control_plane::{
        dto::fleet_coordinator::FleetCoordinatorInitArgs,
        dto::template::{
            TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
            TemplateManifestInput,
        },
        ids::{
            TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion,
            WasmStoreBinding,
        },
    };
    use canic_core::cdk::utils::hash::{hex_bytes, wasm_hash};
    #[cfg(test)]
    use canic_core::control_plane_support::config::{
        ComponentDeploymentPurpose, ComponentGroupDeploymentSpec, FleetServiceMemberPurpose,
    };
    #[cfg(test)]
    use canic_core::control_plane_support::ops::{
        component_provisioning_plan::ComponentProvisioningPlanOps, fleet_registry::FleetRegistryOps,
    };
    use canic_host::release_set::AppConfigSnapshot;
    use flate2::{Compression, write::GzEncoder};
    use std::{
        collections::BTreeMap, error::Error as StdError, fmt, io::Write, num::NonZeroUsize,
        sync::OnceLock,
    };

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
    use ic_testkit::artifacts::{read_wasm, test_target_dir, workspace_root_for};
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
    use ic_testkit::pic::{PocketIcCapturedSnapshotExt, PocketIcSnapshotExt, PocketIcTimeExt};

    use crate::pic::CanicPicExt;
    #[cfg(test)]
    use canic::protocol::{
        CANIC_ROOT_COMPONENT_DIRECTORY_PAGE, CANIC_ROOT_PEER_COMPONENT_ALLOCATE,
        CANIC_ROOT_PEER_COMPONENT_ALLOCATION_STATUS, CANIC_ROOT_PEER_COMPONENT_COMMIT,
        CANIC_ROOT_PEER_COMPONENT_CREATE, CANIC_ROOT_PEER_COMPONENT_DIRECTORY_PREPARE,
        CANIC_ROOT_PEER_COMPONENT_INSTALL, CANIC_ROOT_PEER_COMPONENT_MEMBERSHIP_ACTIVATE,
        CANIC_ROOT_PEER_COMPONENT_RUNTIME_ACTIVATE, CANIC_ROOT_STORE_BOOTSTRAP_STATUS,
        CANIC_WASM_STORE_CATALOG, CANIC_WASM_STORE_PREPARE, CANIC_WASM_STORE_STATUS,
    };
    #[cfg(test)]
    use canic::{
        dto::component_registry::{
            ComponentDirectoryChildEntry, ComponentDirectoryPageRequest,
            ComponentDirectoryPageResponse, RootComponentChildAllocationRequest,
            RootComponentChildAllocationResponse, RootComponentChildCommitRequest,
            RootComponentChildCommitResponse, RootComponentChildCreationRequest,
            RootComponentChildDirectoryPreparationRequest,
            RootComponentChildDirectoryPreparationResponse, RootComponentChildInstallRequest,
            RootComponentChildMembershipActivationRequest,
            RootComponentChildMembershipActivationResponse,
            RootComponentChildRuntimeActivationRequest,
            RootComponentChildRuntimeActivationResponse, RootComponentDeletionPhase,
            RootComponentDeletionRequest, RootComponentDeletionResponse,
            RootComponentDeletionStatusRequest, RootComponentDrainingAdvancePhase,
            RootComponentDrainingAdvanceRequest, RootComponentDrainingAdvanceResponse,
            RootComponentDrainingRequest, RootComponentDrainingResponse,
            RootComponentFinalInventoryRequest, RootComponentFinalInventoryResponse,
            RootComponentQuiescencePhase, RootComponentQuiescenceRequest,
            RootComponentQuiescenceResponse, RootComponentSubtreeRemovalAdvanceRequest,
            RootComponentSubtreeRemovalDeletePreparationRequest,
            RootComponentSubtreeRemovalDeleteRequest, RootComponentSubtreeRemovalPhase,
            RootComponentSubtreeRemovalRequest, RootComponentSubtreeRemovalResponse,
            RootComponentSubtreeRemovalStatusRequest,
            RootComponentSubtreeRemovalStopPreparationRequest,
            RootComponentSubtreeRemovalStopRequest,
        },
        dto::fleet_registry::{
            FleetSubnetRootDeletionExecutionRequest, FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionStatusRequest, FleetSubnetRootDrainingPublicationRequest,
            FleetSubnetRootDrainingPublicationResponse, FleetSubnetRootDrainingReservationRequest,
            FleetSubnetRootDrainingReservationResponse, FleetSubnetRootRemovalPublicationResponse,
        },
        dto::fleet_subnet_root::{
            FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES,
            FleetSubnetRootDeletionPreparationRequest, FleetSubnetRootDeletionPreparationResponse,
            FleetSubnetRootDeletionPreparationStatusRequest, FleetSubnetRootDrainingRequest,
            FleetSubnetRootDrainingResponse, FleetSubnetRootDrainingStatusRequest,
            FleetSubnetRootFinalInventoryRequest, FleetSubnetRootFinalInventoryResponse,
            FleetSubnetRootFinalInventoryStatusRequest, FleetSubnetRootRemovalRequest,
            FleetSubnetRootRemovalStatusRequest, FleetSubnetRootStoreBindingFinalizationRequest,
            FleetSubnetRootStoreBindingFinalizationResponse,
            FleetSubnetRootStoreBindingFinalizationStatusRequest,
            FleetSubnetRootStoreDeletionRequest, FleetSubnetRootStoreDeletionResponse,
            FleetSubnetRootStoreDeletionStatusRequest, FleetSubnetRootStoreReclamationRequest,
            FleetSubnetRootStoreReclamationResponse, FleetSubnetRootStoreReclamationStatusRequest,
        },
        protocol::{
            CANIC_CYCLE_BALANCE, CANIC_FLEET_REGISTRY_PUBLISH_ROOT_DRAINING,
            CANIC_FLEET_REGISTRY_ROOT_DELETION_EXECUTION_BEGIN,
            CANIC_FLEET_REGISTRY_ROOT_DELETION_EXECUTION_STATUS,
            CANIC_FLEET_REGISTRY_ROOT_DRAINING_RESERVATION_PREPARE,
            CANIC_FLEET_SUBNET_ROOT_DELETION_PREPARATION_STATUS,
            CANIC_FLEET_SUBNET_ROOT_DELETION_PREPARE, CANIC_FLEET_SUBNET_ROOT_DRAINING_BEGIN,
            CANIC_FLEET_SUBNET_ROOT_DRAINING_INVENTORY_FINALIZE,
            CANIC_FLEET_SUBNET_ROOT_DRAINING_INVENTORY_STATUS,
            CANIC_FLEET_SUBNET_ROOT_DRAINING_STATUS, CANIC_FLEET_SUBNET_ROOT_REMOVAL_PUBLISH,
            CANIC_FLEET_SUBNET_ROOT_REMOVAL_STATUS,
            CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZATION_STATUS,
            CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZE, CANIC_FLEET_SUBNET_ROOT_STORE_DELETE,
            CANIC_FLEET_SUBNET_ROOT_STORE_DELETION_STATUS, CANIC_FLEET_SUBNET_ROOT_STORE_RECLAIM,
            CANIC_FLEET_SUBNET_ROOT_STORE_RECLAMATION_STATUS, CANIC_ROOT_COMPONENT_CHILD_ALLOCATE,
            CANIC_ROOT_COMPONENT_CHILD_COMMIT, CANIC_ROOT_COMPONENT_CHILD_CREATE,
            CANIC_ROOT_COMPONENT_CHILD_DIRECTORY_PREPARE, CANIC_ROOT_COMPONENT_CHILD_INSTALL,
            CANIC_ROOT_COMPONENT_CHILD_MEMBERSHIP_ACTIVATE,
            CANIC_ROOT_COMPONENT_CHILD_RUNTIME_ACTIVATE, CANIC_ROOT_COMPONENT_DELETE,
            CANIC_ROOT_COMPONENT_DELETION_STATUS, CANIC_ROOT_COMPONENT_DRAINING_ADVANCE,
            CANIC_ROOT_COMPONENT_DRAINING_BEGIN, CANIC_ROOT_COMPONENT_DRAINING_INVENTORY_FINALIZE,
            CANIC_ROOT_COMPONENT_MEMBERSHIP_REMOVE, CANIC_ROOT_COMPONENT_QUIESCE,
            CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_ADVANCE,
            CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_BEGIN,
            CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DELETE,
            CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DELETE_PREPARE,
            CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STATUS, CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP,
            CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP_PREPARE,
        },
    };
    #[cfg(test)]
    use canic::{
        dto::{
            canister::CanisterInfo,
            capability::{
                CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata,
                CapabilityService, RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
            },
            page::{Page, PageRequest},
            rpc::{
                CreateCanisterParent, CreateCanisterRequest, RecycleCanisterRequest, Request,
                Response, RootRequestMetadata,
            },
        },
        protocol::{CANIC_CANISTER_CHILDREN, CANIC_RESPONSE_CAPABILITY_V1},
    };
    #[cfg(test)]
    use canic_control_plane::{
        dto::template::{
            WasmStoreCatalogEntryResponse, WasmStoreOverviewResponse, WasmStoreStatusResponse,
        },
        ids::WasmStoreGcMode,
    };

    const ISSUER_PACKAGE: &str = "delegation_issuer_stub";
    #[cfg(test)]
    const DATABASE_A_PACKAGE: &str = "database_a_stub";
    #[cfg(test)]
    const DATABASE_B_PACKAGE: &str = "database_b_stub";
    #[cfg(test)]
    const DATABASE_C_PACKAGE: &str = "database_c_stub";
    const PROJECT_HUB_PACKAGE: &str = "project_hub_stub";
    const PROJECT_INSTANCE_PACKAGE: &str = "project_instance_stub";
    const PROJECT_LEDGER_PACKAGE: &str = "project_ledger_stub";
    const PROJECT_MACHINE_PACKAGE: &str = "project_machine_stub";
    const COORDINATOR_INSTALL_CYCLES: u128 = 500_000_000_000_000;

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
        Contract(BaselinePoolContractError),
        Invariant(String),
        Snapshot(ControllerSnapshotError),
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
            baseline.pocket_ic().wait_for_all_ready(
                [
                    metadata.root,
                    metadata.wasm_store,
                    metadata.issuer.canister_id,
                    metadata.verifier.canister_id,
                ],
                60,
                "restored active Component Registry baseline",
            );
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
                Self::Contract(error) => Some(error),
                Self::Invariant(_) => None,
                Self::Snapshot(error) => Some(error),
            }
        }
    }

    struct ActiveComponentBindings {
        issuer: ComponentBinding,
        verifier: ComponentBinding,
    }

    impl ActiveComponentBindings {
        const fn new(issuer: ComponentBinding, verifier: ComponentBinding) -> Self {
            Self { issuer, verifier }
        }
    }

    struct BootstrappedRootFixture {
        root_id: Principal,
        init_args: FleetSubnetRootInitArgs,
        request: RootStoreBootstrapRequest,
        response: RootStoreBootstrapResponse,
        configuration: RootFixtureConfiguration,
    }

    #[cfg(test)]
    struct PreparedGroupedProvisioningFixture {
        pic: PocketIc,
        coordinator: Principal,
        root: BootstrappedRootFixture,
        request: RootComponentProvisioningAcceptanceRequest,
    }

    #[cfg(test)]
    struct ActiveCrossRootPeerFixture {
        pic: PocketIc,
        coordinator: Principal,
        requester_root: BootstrappedRootFixture,
        target_root: BootstrappedRootFixture,
        requester: ComponentBinding,
        initial_registry: FleetRegistryVersion,
        service_registry: FleetRegistryVersion,
    }

    #[cfg(test)]
    struct TokoTopologyFixture {
        pic: PocketIc,
        coordinator: Principal,
        roots: Vec<BootstrappedRootFixture>,
        initial_registry: FleetRegistryVersion,
        second_coordinator: Principal,
        second_root: BootstrappedRootFixture,
        wasm_footprints: BTreeMap<&'static str, (usize, usize)>,
    }

    struct RootStoreFixture {
        manifest: RootStoreReleaseSetManifest,
        artifacts: BTreeMap<CanisterRole, Vec<u8>>,
        configuration: RootFixtureConfiguration,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum RootFixtureConfiguration {
        Delegation,
        #[cfg(test)]
        Toko,
    }

    impl RootFixtureConfiguration {
        fn config_path(self, workspace_root: &Path) -> PathBuf {
            match self {
                Self::Delegation => root_canister_config_path(workspace_root),
                #[cfg(test)]
                Self::Toko => toko_root_canister_config_path(workspace_root),
            }
        }
    }

    #[cfg(test)]
    #[derive(CandidType)]
    struct CyclesLedgerStubInitArgs {
        canister_id: Principal,
        expected_root: Principal,
        expected_subnet: Principal,
    }

    #[test]
    fn prepared_mainnet_root_automatically_refills_one_exact_pool_asset() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let (root_wasm, cycles_ledger_wasm) = build_mainnet_refill_wasms();
        let _ = build_test_wasm_store_wasm();
        let store_fixture = build_root_store_fixture();
        let pic = build_pic();
        let created_asset = std::cell::Cell::new(None);
        let fixture = install_bootstrapped_root_with_pool_setup(
            &pic,
            root_wasm,
            Principal::from_slice(&[0x41; 29]),
            store_fixture,
            |pic, root| {
                let root_subnet = pic.get_subnet(root).expect("root placement Subnet");
                let asset = pic.create_canister_on_subnet(None, None, root_subnet);
                pic.set_controllers(asset, None, vec![root])
                    .expect("prepare returned pool asset controller");
                let cycles_ledger = Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai")
                    .expect("canonical Cycles Ledger principal");
                pic.create_canister_with_id(None, None, cycles_ledger)
                    .expect("create canonical Cycles Ledger stub principal");
                pic.install_canister(
                    cycles_ledger,
                    cycles_ledger_wasm,
                    encode_one(CyclesLedgerStubInitArgs {
                        canister_id: asset,
                        expected_root: root,
                        expected_subnet: root_subnet,
                    })
                    .expect("encode Cycles Ledger stub init"),
                    None,
                );
                created_asset.set(Some(asset));
                Vec::new()
            },
        );
        let asset = created_asset.get().expect("prepared pool asset");

        for _ in 0..4 {
            let status = root_pool_status(&pic, fixture.root_id);
            if status.ready == 1 {
                break;
            }
            let response: Result<PoolAdminResponse, Error> = pic
                .update_candid(
                    fixture.root_id,
                    CANIC_POOL_ADMIN,
                    (PoolAdminCommand::Maintain,),
                )
                .expect("automatic pool maintenance transport");
            response.expect("automatic pool maintenance");
        }

        let status = root_pool_status(&pic, fixture.root_id);
        assert_eq!(status.ready, 1);
        assert_eq!(status.pending_reset, 0);
        let entry = status
            .entries
            .iter()
            .find(|entry| entry.canister_id == asset)
            .expect("automatically created inventory entry");
        assert_eq!(entry.origin, CanisterPoolAssetOrigin::Created);
        assert_eq!(entry.status, CanisterPoolAssetStatus::Ready);
        let cycles_ledger = Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai")
            .expect("canonical Cycles Ledger principal");
        let request_count: u64 = pic
            .query_candid(cycles_ledger, "request_count", ())
            .expect("query ledger request count");
        assert_eq!(request_count, 1);
    }

    #[test]
    fn prepared_root_bootstraps_and_reverifies_its_exact_local_store() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let root_wasm = build_test_root_wasm();
        let store_fixture = build_root_store_fixture();
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
            vec![
                CanisterRole::new("issuer"),
                CanisterRole::new("project_hub"),
                CanisterRole::new("project_instance"),
                CanisterRole::new("project_ledger"),
                CanisterRole::new("project_machine"),
            ],
            "root Store catalog must contain the exact canonical application role closure"
        );

        let retried: Result<RootStoreBootstrapResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_STORE_BOOTSTRAP,
                (fixture.request.clone(),),
            )
            .expect("root Store bootstrap retry transport");
        assert_eq!(
            retried.expect("root Store bootstrap retry"),
            fixture.response,
            "exact update retry must return the same Store evidence"
        );
        let observed: Result<RootStoreBootstrapResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_STORE_BOOTSTRAP_STATUS,
                (fixture.request,),
            )
            .expect("root Store status transport");
        assert_eq!(
            observed.expect("root Store status"),
            fixture.response,
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
            chunk_hashes: vec![payload_hash],
        };
        let prepared: Result<TemplateChunkSetInfoResponse, Error> = pic
            .update_candid_as(
                fixture.response.wasm_store,
                fixture.root_id,
                CANIC_WASM_STORE_PREPARE,
                (prepare.clone(),),
            )
            .expect("direct root Store prepare transport");
        assert_eq!(
            prepared.expect("direct root Store prepare").chunk_hashes,
            prepare.chunk_hashes
        );

        let denied: Result<TemplateChunkSetInfoResponse, Error> = pic
            .update_candid_as(
                fixture.response.wasm_store,
                Principal::anonymous(),
                CANIC_WASM_STORE_PREPARE,
                (prepare,),
            )
            .expect("anonymous Store prepare transport");
        assert_eq!(
            denied
                .expect_err("anonymous Store prepare must fail")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
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
            expected_registry: first_joined,
            store_bootstrap: second.request.clone(),
        };
        let rejected: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
            .update_candid(
                second.root_id,
                CANIC_FLEET_REGISTRY_SYNCHRONIZE,
                (foreign_sync,),
            )
            .expect("foreign-Fleet Registry synchronization transport");
        assert_eq!(
            rejected
                .expect_err("another Fleet's Registry must not enter the co-located root")
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

    #[test]
    fn toko_qualification_config_reuses_database_specs_in_nested_project_cells() {
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config = AppConfigSnapshot::load(&toko_root_canister_config_path(&workspace_root))
            .expect("load Toko qualification configuration");
        let deployments = config
            .model()
            .compile_component_group_deployment_topology()
            .expect("compile Toko qualification deployments");
        let authoritative = deployments
            .get(
                &"authoritative_databases"
                    .parse()
                    .expect("Authority deployment ID"),
            )
            .expect("Authority database deployment");
        let project_cells = deployments
            .get(
                &"project_data_cells"
                    .parse()
                    .expect("project-cell deployment ID"),
            )
            .expect("nested project-cell deployment");

        assert_eq!(authoritative.members.len(), 3);
        assert_eq!(project_cells.members.len(), 4);
        for database in ["database_a", "database_b", "database_c"] {
            let authority = authoritative
                .members
                .iter()
                .find(|member| member.component_spec.as_str() == database)
                .expect("Authority database member");
            let replica = project_cells
                .members
                .iter()
                .find(|member| member.component_spec.as_str() == database)
                .expect("nested Replica database member");
            assert_eq!(authority.component_spec_hash, replica.component_spec_hash);
            assert_eq!(authority.member_path.as_slice().len(), 1);
            assert_eq!(replica.member_path.as_slice().len(), 2);
            assert!(matches!(
                &authority.purpose,
                ComponentDeploymentPurpose::FleetServiceMember {
                    service,
                    member_purpose: FleetServiceMemberPurpose::Authority,
                } if service.as_str() == database
            ));
            assert!(matches!(
                &replica.purpose,
                ComponentDeploymentPurpose::FleetServiceMember {
                    service,
                    member_purpose: FleetServiceMemberPurpose::Replica,
                } if service.as_str() == database
            ));
        }

        let hub = project_cells
            .members
            .iter()
            .find(|member| member.component_spec.as_str() == "projects")
            .expect("project-cell Hub member");
        assert_eq!(hub.limits.maximum_descendants, 20_000);
        assert_eq!(hub.limits.maximum_registry_bytes, 16_777_216);
        assert!(matches!(
            &hub.purpose,
            ComponentDeploymentPurpose::FleetServiceMember {
                service,
                member_purpose: FleetServiceMemberPurpose::PoolMember,
            } if service.as_str() == "project_hubs"
        ));

        let packed = deployments
            .get(&"packed_projects".parse().expect("packed deployment ID"))
            .expect("packed ActivePool deployment");
        assert_eq!(packed.initial_placements, 2);
        assert_eq!(packed.maximum_placements, 3);
        assert_eq!(packed.placement.maximum_per_root, 2);
        assert_eq!(packed.component_group.as_str(), "grouped_projects");
        assert_eq!(packed.members[0].limits.spawn_grant_reductions.len(), 1);
        assert_eq!(
            packed.members[0].limits.spawn_grant_reductions[0].maximum_instances_per_parent,
            2_000
        );

        let grouped = deployments
            .get(&"grouped_projects".parse().expect("grouped deployment ID"))
            .expect("grouped Project Hub deployment");
        assert_eq!(grouped.component_group, packed.component_group);
        assert!(grouped.members[0].limits.spawn_grant_reductions.is_empty());

        let topology = config.component_topology();
        let projects = topology
            .component_specs
            .iter()
            .find(|spec| spec.component_spec.as_str() == "projects")
            .expect("Project Hub Component Spec");
        let instance_grant = projects
            .spawn_grant(
                &CanisterRole::new("project_hub"),
                &CanisterRole::new("project_instance"),
            )
            .expect("Project Hub to Project Instance grant");
        assert_eq!(instance_grant.maximum_instances_per_parent, 10_000);
    }

    #[test]
    fn toko_topology_qualifies_scale_out_descendants_packing_and_fleet_isolation() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_toko_topology_qualification();
        let registry = query_fleet_registry(&fixture.pic, fixture.coordinator);
        let plan = toko_initial_provisioning_plan(&fixture, &registry);
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config = AppConfigSnapshot::load(&toko_root_canister_config_path(&workspace_root))
            .expect("load Toko qualification configuration");
        let initial_plan_bytes =
            ComponentProvisioningPlanOps::canonical_bytes(config.model(), &registry, &plan)
                .expect("encode Toko initial provisioning plan")
                .len();
        let prepared =
            prepare_fleet_provisioning(&fixture.pic, fixture.coordinator, [0xf1; 32], plan);
        let activated = drive_coordinator_provisioning(
            &fixture.pic,
            fixture.coordinator,
            prepared,
            FleetComponentProvisioningPhase::RuntimesActivated,
        );

        assert_eq!(activated.root_batch_count, 3);
        assert_eq!(activated.accepted_root_count, 3);
        assert_eq!(activated.provisioned_root_count, 3);
        assert_eq!(activated.directory_confirmed_root_count, 3);
        assert_eq!(activated.runtime_activated_root_count, 3);
        let published = query_fleet_registry(&fixture.pic, fixture.coordinator);
        assert_toko_initial_service_topology(&fixture, &published);

        let project_cell_plan =
            toko_scale_out_plan(&fixture, &published, "project_data_cells", 1, 2, 1, 2);
        let project_cell_prepare = FleetComponentProvisioningPrepareRequest {
            operation_id: [0xf2; 32],
            plan: project_cell_plan,
        };
        let project_cell = prepare_fleet_provisioning_request(
            &fixture.pic,
            fixture.coordinator,
            &project_cell_prepare,
        );
        let project_cell = drive_coordinator_provisioning_with_restarts(
            &fixture.pic,
            fixture.coordinator,
            project_cell,
        );
        assert_terminal_scale_out(&project_cell, 3);

        let after_project_cell = query_fleet_registry(&fixture.pic, fixture.coordinator);
        let packed_plan =
            toko_scale_out_plan(&fixture, &after_project_cell, "packed_projects", 2, 3, 2, 2);
        let packed_prepare = FleetComponentProvisioningPrepareRequest {
            operation_id: [0xf3; 32],
            plan: packed_plan,
        };
        let packed =
            prepare_fleet_provisioning_request(&fixture.pic, fixture.coordinator, &packed_prepare);
        let packed =
            drive_coordinator_provisioning_with_restarts(&fixture.pic, fixture.coordinator, packed);
        assert_terminal_scale_out(&packed, 2);
        let packed_replay =
            prepare_fleet_provisioning_request(&fixture.pic, fixture.coordinator, &packed_prepare);
        assert_eq!(packed_replay, packed);

        let final_registry = query_fleet_registry(&fixture.pic, fixture.coordinator);
        assert_toko_scaled_service_topology(&fixture, &final_registry);
        provision_toko_project_trees(&fixture, &final_registry);
        report_toko_qualification_metrics(
            &fixture,
            config.model(),
            &final_registry,
            initial_plan_bytes,
        );
        assert_eq!(
            fixture.pic.get_subnet(fixture.roots[1].root_id),
            fixture.pic.get_subnet(fixture.second_root.root_id)
        );
        assert_isolated_coordinator_registry(
            &fixture.pic,
            fixture.coordinator,
            &fixture.roots,
            &fixture.second_root,
        );
        assert_isolated_coordinator_registry(
            &fixture.pic,
            fixture.second_coordinator,
            std::slice::from_ref(&fixture.second_root),
            &fixture.roots[0],
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
        let rejected: Result<TemplateChunkSetInfoResponse, Error> = pic
            .update_candid_as(
                owner.response.wasm_store,
                foreign.root_id,
                CANIC_WASM_STORE_PREPARE,
                (request.clone(),),
            )
            .expect("foreign root Store update transport");
        assert_eq!(
            rejected
                .expect_err("another Fleet's co-located root must not write this Store")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );
        let accepted: Result<TemplateChunkSetInfoResponse, Error> = pic
            .update_candid_as(
                owner.response.wasm_store,
                owner.root_id,
                CANIC_WASM_STORE_PREPARE,
                (request,),
            )
            .expect("owning root Store update transport");
        accepted.expect("owning root retains Store update authority");
    }

    #[cfg(test)]
    fn assert_isolated_coordinator_registry(
        pic: &PocketIc,
        coordinator: Principal,
        owned: &[BootstrappedRootFixture],
        foreign: &BootstrappedRootFixture,
    ) {
        let registry: Result<FleetRegistry, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query isolated Coordinator Registry transport");
        let registry = registry.expect("query isolated Coordinator Registry");
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
    fn prepared_root_freezes_one_exact_provisioned_group_result_without_publishing_directories() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_prepared_grouped_provisioning();
        let accepted: Result<RootComponentProvisioningStatusResponse, Error> = fixture
            .pic
            .update_candid_as(
                fixture.root.root_id,
                fixture.coordinator,
                CANIC_ROOT_COMPONENT_PROVISIONING_ACCEPT,
                (fixture.request.clone(),),
            )
            .expect("accept grouped provisioning batch transport");
        let accepted = accepted.expect("accept grouped provisioning batch");
        assert_grouped_provisioning_progress(&accepted, 0, 0, 0, 0);

        let reserved = advance_grouped_provisioning(&fixture, advance_request(&accepted));
        assert_grouped_provisioning_progress(&reserved, 1, 0, 0, 0);
        let claim_request = advance_request(&reserved);
        let claimed = advance_grouped_provisioning(&fixture, claim_request);
        assert_grouped_provisioning_progress(&claimed, 1, 1, 0, 0);

        let (canister_id, claim) = one_grouped_workload(&fixture);
        let created = grouped_allocation_status(&fixture, claim.operation_id);
        assert_eq!(created.phase, RootComponentAllocationPhase::Created);
        assert_eq!(created.component, claim.component);
        assert_eq!(
            created.provisioning_origin,
            ComponentProvisioningOrigin::ComponentGroup {
                operation_id: fixture.request.operation_id,
                plan_hash: fixture.request.plan_hash,
                group_placement: fixture.request.batch.placements[0].group_placement.clone(),
                member_path: fixture.request.batch.placements[0].entries[0]
                    .member_path
                    .clone(),
            }
        );
        assert_grouped_claim_replay(&fixture, claim_request, &claimed, canister_id, &claim);

        let install_request = advance_request(&claimed);
        let installed = advance_grouped_provisioning(&fixture, install_request);
        assert_grouped_provisioning_progress(&installed, 1, 1, 1, 0);
        assert_grouped_member_install(&fixture, canister_id, &claim);

        let install_replay = advance_grouped_provisioning(&fixture, install_request);
        assert_eq!(install_replay, installed);

        let registry_request = advance_request(&installed);
        let registered = advance_grouped_provisioning(&fixture, registry_request);
        assert_grouped_provisioning_progress(&registered, 1, 1, 1, 1);
        let partition = assert_grouped_member_registry_commit(&fixture, &claim);
        let registry_replay = advance_grouped_provisioning(&fixture, registry_request);
        assert_eq!(registry_replay, registered);

        let provision_request = advance_request(&registered);
        let provisioned = advance_grouped_provisioning(&fixture, provision_request);
        assert_eq!(
            provisioned.phase,
            RootComponentProvisioningPhase::Provisioned
        );
        assert!(provisioned.provisioned_at_ns.is_some());
        let result = provisioned
            .result
            .as_ref()
            .expect("provisioned root result");
        assert_eq!(result.placements.len(), 1);
        let planned_placement = &fixture.request.batch.placements[0];
        let placement = &result.placements[0];
        assert_eq!(placement.group_placement, planned_placement.group_placement);
        assert_eq!(placement.component_group, planned_placement.component_group);
        assert_eq!(placement.members.len(), 1);
        let planned_member = &planned_placement.entries[0];
        let member = &placement.members[0];
        assert_eq!(member.member_path, planned_member.member_path);
        assert_eq!(member.component_spec, planned_member.component_spec);
        assert_eq!(member.purpose, planned_member.purpose);
        assert_eq!(member.limits, planned_member.limits);
        assert_eq!(member.binding.component, claim.component);
        assert_eq!(member.binding.canister_id, canister_id);
        assert_eq!(member.component_registry_revision, partition.head.revision);
        assert_eq!(
            member.component_registry_content_hash,
            partition.head.content_hash
        );
        assert_ne!(
            provisioned.receipt_content_hash,
            registered.receipt_content_hash
        );
        let provision_replay = advance_grouped_provisioning(&fixture, provision_request);
        assert_eq!(provision_replay, provisioned);
        let observed: Result<RootComponentProvisioningStatusResponse, Error> = fixture
            .pic
            .query_candid_as(
                fixture.root.root_id,
                fixture.coordinator,
                CANIC_ROOT_COMPONENT_PROVISIONING_STATUS,
                (RootComponentProvisioningStatusRequest {
                    operation_id: fixture.request.operation_id,
                    plan_hash: fixture.request.plan_hash,
                },),
            )
            .expect("query grouped provisioning status transport");
        assert_eq!(
            observed.expect("query grouped provisioning status"),
            provisioned
        );
        assert_prepared(&fixture.pic, fixture.root.root_id);
    }

    #[test]
    fn coordinator_confirms_directories_before_activating_grouped_runtimes() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_prepared_grouped_provisioning();
        let prepared = prepare_coordinator_grouped_plan(&fixture);
        let confirmed = drive_coordinator_directory_confirmation(&fixture, prepared);
        assert_confirmed_grouped_directories(&fixture, &confirmed);
        let activated = drive_coordinator_runtime_activation(&fixture, confirmed);
        assert_activated_grouped_runtimes(&fixture, &activated);
    }

    #[test]
    fn grouped_component_child_inherits_the_exact_owner_deployment_context() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_prepared_grouped_provisioning();
        let prepared = prepare_coordinator_grouped_plan(&fixture);
        let confirmed = drive_coordinator_directory_confirmation(&fixture, prepared);
        let activated = drive_coordinator_runtime_activation(&fixture, confirmed);
        assert_activated_grouped_runtimes(&fixture, &activated);

        let root = grouped_root_provisioning_status(&fixture);
        let member = &root
            .result
            .as_ref()
            .expect("active root retains provisioned result")
            .placements[0]
            .members[0];
        let owner_runtime = grouped_component_runtime(&fixture, member.binding.canister_id);
        let expected_deployment = owner_runtime.deployment;
        let request_id = [0xd3; 32];
        let envelope = RootCapabilityEnvelopeV1 {
            service: CapabilityService::Root,
            capability_version: CAPABILITY_VERSION_V1,
            capability: Request::CreateCanister(CreateCanisterRequest {
                canister_role: CanisterRole::new("project_instance"),
                parent: CreateCanisterParent::ThisCanister,
                extra_arg: None,
                metadata: Some(RootRequestMetadata {
                    request_id,
                    ttl_ns: 60_000_000_000,
                }),
            }),
            proof: CapabilityProof::Structural,
            metadata: CapabilityRequestMetadata {
                request_id,
                issued_at_ns: fixture.pic.current_time_nanos(),
                ttl_ns: 60_000_000_000,
            },
        };
        let provisioned: Result<RootCapabilityResponseV1, Error> = fixture
            .pic
            .update_candid_as(
                fixture.root.root_id,
                member.binding.canister_id,
                CANIC_RESPONSE_CAPABILITY_V1,
                (envelope.clone(),),
            )
            .expect("grouped Component child capability transport");
        let provisioned = provisioned.expect("grouped Component child capability");
        let Response::CreateCanister(provisioned) = provisioned.response else {
            panic!("root capability must return a create-Canister response");
        };
        let child = provisioned.new_canister_pid;

        let replayed: Result<RootCapabilityResponseV1, Error> = fixture
            .pic
            .update_candid_as(
                fixture.root.root_id,
                member.binding.canister_id,
                CANIC_RESPONSE_CAPABILITY_V1,
                (envelope,),
            )
            .expect("grouped Component child capability replay transport");
        let Response::CreateCanister(replayed) = replayed
            .expect("grouped Component child capability replay")
            .response
        else {
            panic!("root capability replay must return a create-Canister response");
        };
        assert_eq!(replayed.new_canister_pid, child);

        let child_runtime = grouped_component_runtime(&fixture, child);
        assert_eq!(child_runtime.phase, ComponentRuntimePhase::Active);
        assert_eq!(child_runtime.deployment, expected_deployment);
        let ManagedCanisterBinding::ComponentChild(child_binding) = child_runtime.binding else {
            panic!("provisioned descendant must retain a Component Child binding");
        };
        assert_eq!(child_binding.component, member.binding);
        assert_eq!(child_binding.parent_canister_id, member.binding.canister_id);
    }

    #[test]
    fn grouped_project_tree_is_provisioned_locally_with_exact_immediate_parents() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_prepared_grouped_provisioning();
        let prepared = prepare_coordinator_grouped_plan(&fixture);
        let confirmed = drive_coordinator_directory_confirmation(&fixture, prepared);
        let activated = drive_coordinator_runtime_activation(&fixture, confirmed);
        assert_activated_grouped_runtimes(&fixture, &activated);

        let root = grouped_root_provisioning_status(&fixture);
        let hub = root
            .result
            .as_ref()
            .expect("active root retains provisioned result")
            .placements[0]
            .members[0]
            .binding
            .clone();
        let registry_before = fleet_registry_version(&fixture.pic, fixture.coordinator);
        let instance = resolve_project_instance(&fixture.pic, hub.canister_id, "project-alpha");
        assert_eq!(
            resolve_project_instance(&fixture.pic, hub.canister_id, "project-alpha"),
            instance
        );

        let ledger =
            create_project_descendant(&fixture.pic, instance, "create_project_ledger", [0xe1; 32])
                .expect("Project Instance creates its Ledger");
        assert_eq!(
            create_project_descendant(&fixture.pic, instance, "create_project_ledger", [0xe1; 32],)
                .expect("exact Ledger creation retry"),
            ledger
        );
        let machine =
            create_project_descendant(&fixture.pic, instance, "create_project_machine", [0xe2; 32])
                .expect("Project Instance creates its optional Machine");

        assert_project_singleton_and_parent_guards(&fixture.pic, hub.canister_id, instance);
        let entries = project_directory_entries(&fixture.pic, fixture.root.root_id, &hub);
        assert_project_child(
            &entries,
            instance,
            hub.canister_id,
            "project_instance",
            &hub,
        );
        assert_project_child(&entries, ledger, instance, "project_ledger", &hub);
        assert_project_child(&entries, machine, instance, "project_machine", &hub);
        assert_project_tree_subnet(
            &fixture.pic,
            fixture.root.root_id,
            &[instance, ledger, machine],
        );
        assert_eq!(
            fleet_registry_version(&fixture.pic, fixture.coordinator),
            registry_before
        );
    }

    #[test]
    fn project_cell_scale_out_resumes_after_coordinator_restart_without_duplicates() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_active_cross_root_peer();
        let registry: Result<FleetRegistry, Error> = fixture
            .pic
            .query_candid(fixture.coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query pre-scale-out Fleet Registry transport");
        let registry = registry.expect("query pre-scale-out Fleet Registry");
        let plan = project_cell_scale_out_plan(&fixture, &registry);
        let prepare_request = FleetComponentProvisioningPrepareRequest {
            operation_id: [0xd8; 32],
            plan,
        };
        let prepared: Result<FleetComponentProvisioningStatusResponse, Error> = fixture
            .pic
            .update_candid(
                fixture.coordinator,
                CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
                (prepare_request.clone(),),
            )
            .expect("prepare project-cell scale-out transport");
        let prepared = prepared.expect("prepare project-cell scale-out");
        assert_eq!(prepared.phase, FleetComponentProvisioningPhase::Planned);

        let activated = drive_scale_out_with_coordinator_restarts(&fixture, prepared);
        assert_eq!(
            activated.phase,
            FleetComponentProvisioningPhase::RuntimesActivated
        );
        assert_eq!(activated.accepted_root_count, 1);
        assert_eq!(activated.provisioned_root_count, 1);
        assert_eq!(activated.directory_confirmed_root_count, 2);
        assert_eq!(activated.runtime_activated_root_count, 1);
        let published = activated
            .published_fleet_registry
            .as_ref()
            .expect("scale-out publishes one Fleet Registry revision");
        assert_eq!(
            published.revision,
            fixture.service_registry.revision + 1,
            "the complete PoolMember addition must publish atomically"
        );

        let target = scale_out_root_status(&fixture, &activated);
        let result = target.result.expect("scaled root retains its exact result");
        let [placement] = result.placements.as_slice() else {
            panic!("project-cell scale-out must materialize one placement")
        };
        assert_eq!(placement.group_placement.ordinal, 1);
        let [member] = placement.members.as_slice() else {
            panic!("project-cell scale-out must materialize one Project Hub")
        };
        let target_hub = member.binding.clone();
        let target_subnet = *fixture
            .target_root
            .init_args
            .authority
            .binding
            .placement_subnet
            .as_principal();
        assert_eq!(
            fixture.pic.get_subnet(target_hub.canister_id),
            Some(target_subnet)
        );
        assert_project_service_members(&fixture, &target_hub, 2);
        assert_project_service_members(&fixture, &fixture.requester, 2);

        let instance =
            resolve_project_instance(&fixture.pic, target_hub.canister_id, "scaled-project-alpha");
        let ledger =
            create_project_descendant(&fixture.pic, instance, "create_project_ledger", [0xd9; 32])
                .expect("scaled Project Instance creates its Ledger");
        assert_eq!(fixture.pic.get_subnet(instance), Some(target_subnet));
        assert_eq!(fixture.pic.get_subnet(ledger), Some(target_subnet));

        let replayed: Result<FleetComponentProvisioningStatusResponse, Error> = fixture
            .pic
            .update_candid(
                fixture.coordinator,
                CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
                (prepare_request,),
            )
            .expect("replay terminal project-cell scale-out prepare transport");
        assert_eq!(
            replayed.expect("replay terminal project-cell scale-out prepare"),
            activated,
            "terminal retry must not create another placement or Component"
        );
    }

    #[cfg(test)]
    fn project_cell_scale_out_plan(
        fixture: &ActiveCrossRootPeerFixture,
        registry: &FleetRegistry,
    ) -> FleetComponentProvisioningPlan {
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config = AppConfigSnapshot::load(&root_canister_config_path(&workspace_root))
            .expect("load project-cell scale-out configuration");
        let deployments = config
            .model()
            .compile_component_group_deployment_topology()
            .expect("compile project-cell scale-out topology");
        let deployment = deployments
            .get(
                &"grouped_projects"
                    .parse()
                    .expect("grouped projects deployment ID"),
            )
            .expect("grouped projects deployment");
        let entries = deployment
            .members
            .iter()
            .map(|member| ComponentGroupPlanEntry {
                member_path: member.member_path.clone(),
                component_spec: member.component_spec.clone(),
                spec_hash: member.component_spec_hash,
                purpose: member.purpose.clone(),
                labels: member.labels.clone(),
                limits: member.limits.clone(),
            })
            .collect();
        let mut directory_confirmation_roots =
            vec![fixture.requester_root.root_id, fixture.target_root.root_id];
        directory_confirmation_roots.sort();
        FleetComponentProvisioningPlan {
            fleet: registry.authority.binding.fleet.clone(),
            fleet_registry: fixture.service_registry.clone(),
            configuration_digest: config
                .model()
                .compile_component_deployment_configuration_digest()
                .expect("compile project-cell scale-out configuration digest"),
            operation: FleetComponentProvisioningOperation::ScaleOut {
                deployment: deployment.deployment.clone(),
                previous_placements: 1,
                requested_placements: 2,
            },
            directory_confirmation_roots,
            batches: vec![FleetSubnetRootProvisioningBatch {
                root: fixture.target_root.init_args.authority.binding.clone(),
                active_release_set: fixture.target_root.init_args.authority.initial_release_set,
                placements: vec![ComponentGroupPlacementPlan {
                    group_placement: ComponentGroupPlacementId {
                        deployment: deployment.deployment.clone(),
                        ordinal: 1,
                    },
                    component_group: deployment.component_group.clone(),
                    entries,
                }],
            }],
        }
    }

    #[cfg(test)]
    fn drive_scale_out_with_coordinator_restarts(
        fixture: &ActiveCrossRootPeerFixture,
        status: FleetComponentProvisioningStatusResponse,
    ) -> FleetComponentProvisioningStatusResponse {
        drive_coordinator_provisioning_with_restarts(&fixture.pic, fixture.coordinator, status)
    }

    #[cfg(test)]
    fn drive_coordinator_provisioning_with_restarts(
        pic: &PocketIc,
        coordinator: Principal,
        mut status: FleetComponentProvisioningStatusResponse,
    ) -> FleetComponentProvisioningStatusResponse {
        while status.phase != FleetComponentProvisioningPhase::RuntimesActivated {
            let request = coordinator_advance_request(&status);
            let advanced: Result<FleetComponentProvisioningStatusResponse, Error> = pic
                .update_candid(
                    coordinator,
                    CANIC_FLEET_COMPONENT_PROVISIONING_ADVANCE,
                    (request,),
                )
                .expect("advance project-cell scale-out transport");
            let advanced = advanced.unwrap_or_else(|error| {
                panic!(
                    "advance project-cell scale-out from phase {:?}, root {:?}, synchronization {:?}, publication {:?}, activation {:?}: {error:?}",
                    status.phase,
                    status.current_root,
                    status.current_synchronization,
                    status.current_publication,
                    status.current_activation,
                )
            });
            pic.stop_canister(coordinator, None)
                .expect("stop Coordinator after durable scale-out step");
            pic.start_canister(coordinator, None)
                .expect("restart Coordinator after durable scale-out step");
            let replayed: Result<FleetComponentProvisioningStatusResponse, Error> = pic
                .update_candid(
                    coordinator,
                    CANIC_FLEET_COMPONENT_PROVISIONING_ADVANCE,
                    (request,),
                )
                .expect("replay interrupted project-cell scale-out transport");
            assert_eq!(
                replayed.expect("replay interrupted project-cell scale-out"),
                advanced
            );
            status = advanced;
        }
        status
    }

    #[cfg(test)]
    fn scale_out_root_status(
        fixture: &ActiveCrossRootPeerFixture,
        status: &FleetComponentProvisioningStatusResponse,
    ) -> RootComponentProvisioningStatusResponse {
        let response: Result<RootComponentProvisioningStatusResponse, Error> = fixture
            .pic
            .query_candid_as(
                fixture.target_root.root_id,
                fixture.coordinator,
                CANIC_ROOT_COMPONENT_PROVISIONING_STATUS,
                (RootComponentProvisioningStatusRequest {
                    operation_id: status.operation_id,
                    plan_hash: status.plan_hash,
                },),
            )
            .expect("query scaled root provisioning status transport");
        response.expect("query scaled root provisioning status")
    }

    #[cfg(test)]
    fn assert_project_service_members(
        fixture: &ActiveCrossRootPeerFixture,
        component: &ComponentBinding,
        expected: usize,
    ) {
        let response: Result<ComponentRuntimeStatusResponse, Error> = fixture
            .pic
            .query_candid_as(
                component.canister_id,
                component.fleet_subnet_root,
                CANIC_COMPONENT_RUNTIME_STATUS,
                (),
            )
            .expect("query project service member runtime transport");
        let runtime = response.expect("query project service member runtime");
        let authority = runtime.authority.expect("active Project Hub authority");
        let service = authority
            .fleet
            .services
            .iter()
            .find(|service| service.service.as_str() == "projects")
            .expect("projects service Directory entry");
        assert_eq!(service.members.len(), expected);
    }

    #[cfg(test)]
    const fn coordinator_advance_request(
        status: &FleetComponentProvisioningStatusResponse,
    ) -> FleetComponentProvisioningAdvanceRequest {
        FleetComponentProvisioningAdvanceRequest {
            operation_id: status.operation_id,
            plan_hash: status.plan_hash,
            expected_phase: status.phase,
            expected_accepted_root_count: status.accepted_root_count,
            expected_provisioned_root_count: status.provisioned_root_count,
            expected_current_root: status.current_root,
            expected_directory_confirmed_root_count: status.directory_confirmed_root_count,
            expected_current_synchronization: status.current_synchronization,
            expected_current_publication: status.current_publication,
            expected_runtime_activated_root_count: status.runtime_activated_root_count,
            expected_current_activation: status.current_activation,
        }
    }

    #[cfg(test)]
    fn fleet_registry_version(pic: &PocketIc, coordinator: Principal) -> FleetRegistryVersion {
        let version: Result<FleetRegistryVersion, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY_VERSION, ())
            .expect("query Fleet Registry version transport");
        version.expect("query Fleet Registry version")
    }

    #[cfg(test)]
    fn resolve_project_instance(pic: &PocketIc, hub: Principal, project: &str) -> Principal {
        let response: Result<PlacementIndexStatusResponse, Error> = pic
            .update_candid(hub, "resolve_project", (project.to_string(),))
            .expect("resolve Project Instance transport");
        let PlacementIndexStatusResponse::Bound { instance_pid, .. } =
            response.expect("resolve Project Instance")
        else {
            panic!("Project Instance resolution must finish bound");
        };
        instance_pid
    }

    #[cfg(test)]
    fn create_project_descendant(
        pic: &PocketIc,
        instance: Principal,
        method: &str,
        operation_id: [u8; 32],
    ) -> Result<Principal, Error> {
        pic.update_candid(instance, method, (operation_id,))
            .unwrap_or_else(|error| panic!("{method} transport: {error}"))
    }

    #[cfg(test)]
    fn assert_project_singleton_and_parent_guards(
        pic: &PocketIc,
        hub: Principal,
        instance: Principal,
    ) {
        let duplicate =
            create_project_descendant(pic, instance, "create_project_ledger", [0xe3; 32])
                .expect_err("a Project Instance may own only one Ledger");
        assert_eq!(
            duplicate.code(),
            canic_core::diagnostics::codes::CAPACITY_LIMIT.raw_code()
        );
        let wrong_parent =
            create_project_descendant(pic, hub, "attempt_project_ledger", [0xe4; 32]);
        assert_eq!(
            wrong_parent
                .expect_err("Project Hub has no direct Ledger spawn grant")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );
    }

    #[cfg(test)]
    fn project_directory_entries(
        pic: &PocketIc,
        root: Principal,
        hub: &ComponentBinding,
    ) -> Vec<ComponentDirectoryChildEntry> {
        let head: Result<ComponentDirectoryHead, Error> = pic
            .query_candid(
                root,
                CANIC_ROOT_COMPONENT_DIRECTORY_HEAD,
                (ComponentDirectoryHeadRequest {
                    component: hub.component,
                },),
            )
            .expect("query Project Hub Directory head transport");
        let head = head.expect("query Project Hub Directory head");
        let page: Result<ComponentDirectoryPageResponse, Error> = pic
            .query_candid_as(
                root,
                hub.canister_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_PAGE,
                (ComponentDirectoryPageRequest {
                    directory: head,
                    parent_canister_id: None,
                    role: None,
                    status: Some(ComponentLifecycleStatus::Active),
                    cursor: None,
                    limit: 100,
                },),
            )
            .expect("query Project Hub Directory page transport");
        let page = page.expect("query Project Hub Directory page");
        assert!(page.next_cursor.is_none());
        page.entries
    }

    #[cfg(test)]
    fn assert_project_child(
        entries: &[ComponentDirectoryChildEntry],
        canister_id: Principal,
        parent_canister_id: Principal,
        role: &'static str,
        component: &ComponentBinding,
    ) {
        let entry = entries
            .iter()
            .find(|entry| entry.binding.canister_id == canister_id)
            .unwrap_or_else(|| panic!("missing {role} from Project Hub Directory"));
        assert_eq!(entry.binding.component, *component);
        assert_eq!(entry.binding.parent_canister_id, parent_canister_id);
        assert_eq!(entry.binding.role, CanisterRole::new(role));
        assert_eq!(entry.status, ComponentLifecycleStatus::Active);
    }

    #[cfg(test)]
    fn assert_project_tree_subnet(pic: &PocketIc, root: Principal, descendants: &[Principal]) {
        let root_subnet = pic.get_subnet(root).expect("Project root Subnet");
        for descendant in descendants {
            assert_eq!(pic.get_subnet(*descendant), Some(root_subnet));
            let observed: Result<Principal, Error> = pic
                .query_candid(*descendant, "canister_id", ())
                .expect("query live Project descendant transport");
            assert_eq!(
                observed.expect("query live Project descendant"),
                *descendant
            );
        }
    }

    #[test]
    fn active_fleet_service_component_provisions_one_cross_root_peer() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = setup_active_cross_root_peer();
        let allocation =
            cross_root_peer_allocation([0xd5; 32], fixture.service_registry.clone(), "projects");
        let response: Result<
            (
                RootComponentAllocationResponse,
                RootComponentAllocationResponse,
                RootComponentMembershipActivationResponse,
            ),
            Error,
        > = fixture
            .pic
            .update_candid(
                fixture.requester.canister_id,
                "provision_cross_root_peer",
                (fixture.target_root.root_id, allocation),
            )
            .expect("direct cross-root service provisioning transport");
        let (reserved, retried, membership) =
            response.expect("direct cross-root service provisioning");
        assert_eq!(retried, reserved);
        assert_eq!(membership.registry.status, ComponentLifecycleStatus::Active);
        assert_eq!(
            membership.registry.binding.fleet_subnet_root,
            fixture.target_root.root_id
        );
        assert_eq!(
            membership.registry.binding.component_spec.as_str(),
            "issuer"
        );
        assert_eq!(
            fixture
                .pic
                .get_subnet(membership.registry.binding.canister_id),
            Some(
                *fixture
                    .target_root
                    .init_args
                    .authority
                    .binding
                    .placement_subnet
                    .as_principal()
            )
        );
        let ComponentProvisioningOrigin::FleetServiceComponent {
            requester,
            registry,
            grant,
        } = &reserved.provisioning_origin
        else {
            panic!("cross-root reservation must retain Fleet-service requester authority")
        };
        assert_eq!(requester.component, fixture.requester);
        assert_eq!(requester.service.as_str(), "projects");
        assert_eq!(registry.as_ref(), &fixture.service_registry);
        assert_eq!(grant.requester_component_spec.as_str(), "projects");
        assert_eq!(grant.target_component_spec.as_str(), "issuer");

        assert_cross_root_invalid_proofs_reject(&fixture, membership.registry.binding.canister_id);
    }

    #[cfg(test)]
    fn assert_cross_root_invalid_proofs_reject(
        fixture: &ActiveCrossRootPeerFixture,
        ordinary_component: Principal,
    ) {
        let stale = call_cross_root_service(
            fixture,
            cross_root_peer_allocation([0xd6; 32], fixture.initial_registry.clone(), "projects"),
        );
        assert_eq!(
            stale
                .expect_err("stale Fleet Registry proof must reject")
                .code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );
        let wrong_service = call_cross_root_service(
            fixture,
            cross_root_peer_allocation(
                [0xd7; 32],
                fixture.service_registry.clone(),
                "not-projects",
            ),
        );
        assert_eq!(
            wrong_service
                .expect_err("wrong Fleet service proof must reject")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );
        assert_forwarded_cross_root_allocation_rejects(
            fixture,
            ordinary_component,
            [0xd8; 32],
            "ordinary Component forwarding",
        );
        let child = provision_cross_root_fixture_child(fixture);
        assert_forwarded_cross_root_allocation_rejects(
            fixture,
            child,
            [0xda; 32],
            "Component Child forwarding",
        );
    }

    #[cfg(test)]
    fn cross_root_peer_allocation(
        operation_id: [u8; 32],
        expected_registry: FleetRegistryVersion,
        service: &str,
    ) -> RootPeerComponentAllocationRequest {
        RootPeerComponentAllocationRequest {
            operation_id,
            component_spec: "issuer".parse().expect("issuer Component Spec"),
            requester: PeerComponentRequester::FleetService {
                service: service.parse().expect("Fleet service ID"),
                expected_registry: Box::new(expected_registry),
            },
        }
    }

    #[cfg(test)]
    fn call_cross_root_service(
        fixture: &ActiveCrossRootPeerFixture,
        allocation: RootPeerComponentAllocationRequest,
    ) -> Result<
        (
            RootComponentAllocationResponse,
            RootComponentAllocationResponse,
            RootComponentMembershipActivationResponse,
        ),
        Error,
    > {
        fixture
            .pic
            .update_candid(
                fixture.requester.canister_id,
                "provision_cross_root_peer",
                (fixture.target_root.root_id, allocation),
            )
            .expect("invalid cross-root service proof transport")
    }

    #[cfg(test)]
    fn assert_forwarded_cross_root_allocation_rejects(
        fixture: &ActiveCrossRootPeerFixture,
        forwarding_canister: Principal,
        operation_id: [u8; 32],
        subject: &str,
    ) {
        let response: Result<RootComponentAllocationResponse, Error> = fixture
            .pic
            .update_candid(
                forwarding_canister,
                "forward_peer_allocation",
                (
                    fixture.target_root.root_id,
                    cross_root_peer_allocation(
                        operation_id,
                        fixture.service_registry.clone(),
                        "projects",
                    ),
                ),
            )
            .unwrap_or_else(|error| panic!("{subject} transport: {error}"));
        let error = match response {
            Ok(response) => panic!("{subject} unexpectedly allocated {response:?}"),
            Err(error) => error,
        };
        assert_eq!(
            error.code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );
    }

    #[cfg(test)]
    fn provision_cross_root_fixture_child(fixture: &ActiveCrossRootPeerFixture) -> Principal {
        let request_id = [0xd9; 32];
        let envelope = RootCapabilityEnvelopeV1 {
            service: CapabilityService::Root,
            capability_version: CAPABILITY_VERSION_V1,
            capability: Request::CreateCanister(CreateCanisterRequest {
                canister_role: CanisterRole::new("project_instance"),
                parent: CreateCanisterParent::ThisCanister,
                extra_arg: None,
                metadata: Some(RootRequestMetadata {
                    request_id,
                    ttl_ns: 60_000_000_000,
                }),
            }),
            proof: CapabilityProof::Structural,
            metadata: CapabilityRequestMetadata {
                request_id,
                issued_at_ns: fixture.pic.current_time_nanos(),
                ttl_ns: 60_000_000_000,
            },
        };
        let response: Result<RootCapabilityResponseV1, Error> = fixture
            .pic
            .update_candid_as(
                fixture.requester_root.root_id,
                fixture.requester.canister_id,
                CANIC_RESPONSE_CAPABILITY_V1,
                (envelope,),
            )
            .expect("provision cross-root fixture child transport");
        let Response::CreateCanister(created) = response
            .expect("provision cross-root fixture child")
            .response
        else {
            panic!("cross-root fixture child must return create-Canister response")
        };
        created.new_canister_pid
    }

    #[cfg(test)]
    fn grouped_root_provisioning_status(
        fixture: &PreparedGroupedProvisioningFixture,
    ) -> RootComponentProvisioningStatusResponse {
        let response: Result<RootComponentProvisioningStatusResponse, Error> = fixture
            .pic
            .query_candid_as(
                fixture.root.root_id,
                fixture.coordinator,
                CANIC_ROOT_COMPONENT_PROVISIONING_STATUS,
                (RootComponentProvisioningStatusRequest {
                    operation_id: fixture.request.operation_id,
                    plan_hash: fixture.request.plan_hash,
                },),
            )
            .expect("query grouped root provisioning status transport");
        response.expect("query grouped root provisioning status")
    }

    #[cfg(test)]
    fn grouped_component_runtime(
        fixture: &PreparedGroupedProvisioningFixture,
        canister_id: Principal,
    ) -> ComponentRuntimeStatusResponse {
        let response: Result<ComponentRuntimeStatusResponse, Error> = fixture
            .pic
            .query_candid_as(
                canister_id,
                fixture.root.root_id,
                CANIC_COMPONENT_RUNTIME_STATUS,
                (),
            )
            .expect("query grouped managed Canister runtime transport");
        response.expect("query grouped managed Canister runtime")
    }

    #[cfg(test)]
    fn prepare_coordinator_grouped_plan(
        fixture: &PreparedGroupedProvisioningFixture,
    ) -> FleetComponentProvisioningStatusResponse {
        let registry: Result<FleetRegistry, Error> = fixture
            .pic
            .query_candid(fixture.coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query grouped provisioning Registry transport");
        let registry = registry.expect("query grouped provisioning Registry");
        let plan = FleetComponentProvisioningPlan {
            fleet: registry.authority.binding.fleet.clone(),
            fleet_registry: fixture.request.fleet_registry.clone(),
            configuration_digest: fixture.request.configuration_digest,
            operation: FleetComponentProvisioningOperation::FreshInstall,
            directory_confirmation_roots: vec![fixture.root.root_id],
            batches: vec![fixture.request.batch.clone()],
        };
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config = AppConfigSnapshot::load(&root_canister_config_path(&workspace_root))
            .expect("load grouped root config");
        assert_eq!(
            ComponentProvisioningPlanOps::hash(config.model(), &registry, &plan)
                .expect("hash Coordinator plan"),
            fixture.request.plan_hash
        );
        let prepared: Result<FleetComponentProvisioningStatusResponse, Error> = fixture
            .pic
            .update_candid(
                fixture.coordinator,
                CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
                (FleetComponentProvisioningPrepareRequest {
                    operation_id: fixture.request.operation_id,
                    plan,
                },),
            )
            .expect("prepare Coordinator provisioning transport");
        let status = prepared.expect("prepare Coordinator provisioning");
        assert_eq!(status.phase, FleetComponentProvisioningPhase::Planned);
        status
    }

    #[cfg(test)]
    fn drive_coordinator_directory_confirmation(
        fixture: &PreparedGroupedProvisioningFixture,
        status: FleetComponentProvisioningStatusResponse,
    ) -> FleetComponentProvisioningStatusResponse {
        drive_coordinator_provisioning(
            &fixture.pic,
            fixture.coordinator,
            status,
            FleetComponentProvisioningPhase::DirectoriesConfirmed,
        )
    }

    #[cfg(test)]
    fn drive_coordinator_runtime_activation(
        fixture: &PreparedGroupedProvisioningFixture,
        status: FleetComponentProvisioningStatusResponse,
    ) -> FleetComponentProvisioningStatusResponse {
        drive_coordinator_provisioning(
            &fixture.pic,
            fixture.coordinator,
            status,
            FleetComponentProvisioningPhase::RuntimesActivated,
        )
    }

    #[cfg(test)]
    fn drive_coordinator_provisioning(
        pic: &PocketIc,
        coordinator: Principal,
        mut status: FleetComponentProvisioningStatusResponse,
        target_phase: FleetComponentProvisioningPhase,
    ) -> FleetComponentProvisioningStatusResponse {
        while status.phase != target_phase {
            let request = FleetComponentProvisioningAdvanceRequest {
                operation_id: status.operation_id,
                plan_hash: status.plan_hash,
                expected_phase: status.phase,
                expected_accepted_root_count: status.accepted_root_count,
                expected_provisioned_root_count: status.provisioned_root_count,
                expected_current_root: status.current_root,
                expected_directory_confirmed_root_count: status.directory_confirmed_root_count,
                expected_current_synchronization: status.current_synchronization,
                expected_current_publication: status.current_publication,
                expected_runtime_activated_root_count: status.runtime_activated_root_count,
                expected_current_activation: status.current_activation,
            };
            let advanced: Result<FleetComponentProvisioningStatusResponse, Error> = pic
                .update_candid(
                    coordinator,
                    CANIC_FLEET_COMPONENT_PROVISIONING_ADVANCE,
                    (request,),
                )
                .expect("advance Coordinator provisioning transport");
            let advanced = advanced.unwrap_or_else(|error| {
                panic!(
                    "advance Coordinator provisioning from phase {:?}, root cursor {:?}, Directory cursor {:?}, and activation cursor {:?}: {error:?}",
                    status.phase,
                    status.current_root,
                    status.current_publication,
                    status.current_activation,
                )
            });
            let replayed: Result<FleetComponentProvisioningStatusResponse, Error> = pic
                .update_candid(
                    coordinator,
                    CANIC_FLEET_COMPONENT_PROVISIONING_ADVANCE,
                    (request,),
                )
                .expect("replay Coordinator provisioning transport");
            assert_eq!(replayed.expect("replay Coordinator provisioning"), advanced);
            status = advanced;
        }
        status
    }

    #[cfg(test)]
    fn assert_confirmed_grouped_directories(
        fixture: &PreparedGroupedProvisioningFixture,
        status: &FleetComponentProvisioningStatusResponse,
    ) {
        assert_eq!(status.accepted_root_count, 1);
        assert_eq!(status.provisioned_root_count, 1);
        assert!(status.components_provisioned_at_ns.is_some());
        assert!(status.published_fleet_registry.is_some());
        assert!(status.service_topology_published_at_ns.is_some());
        assert_eq!(status.directory_confirmed_root_count, 1);
        assert!(status.directories_confirmed_at_ns.is_some());
        let published_registry: Result<FleetRegistry, Error> = fixture
            .pic
            .query_candid(fixture.coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query published grouped provisioning Registry transport");
        let published_registry =
            published_registry.expect("query published grouped provisioning Registry");
        assert_eq!(published_registry.services.len(), 1);
        let root: Result<RootComponentProvisioningStatusResponse, Error> = fixture
            .pic
            .query_candid_as(
                fixture.root.root_id,
                fixture.coordinator,
                CANIC_ROOT_COMPONENT_PROVISIONING_STATUS,
                (RootComponentProvisioningStatusRequest {
                    operation_id: fixture.request.operation_id,
                    plan_hash: fixture.request.plan_hash,
                },),
            )
            .expect("query root provisioning status transport");
        let root = root.expect("query root provisioning status");
        assert_eq!(root.phase, RootComponentProvisioningPhase::Published);
        assert_eq!(root.published_component_count, root.component_count);
        let publication = root
            .publication
            .as_ref()
            .expect("Published root has exact Directory evidence");
        assert_eq!(
            publication.component_directories.len(),
            usize::try_from(root.component_count).expect("bounded Component count")
        );
        assert_eq!(publication.component_group_directories.len(), 1);
        for member in root
            .result
            .as_ref()
            .expect("Published root retains provisioned result")
            .placements
            .iter()
            .flat_map(|placement| &placement.members)
        {
            let runtime: Result<ComponentRuntimeStatusResponse, Error> = fixture
                .pic
                .query_candid_as(
                    member.binding.canister_id,
                    fixture.root.root_id,
                    CANIC_COMPONENT_RUNTIME_STATUS,
                    (),
                )
                .expect("query published Component runtime transport");
            let runtime = runtime.expect("query published Component runtime");
            assert_eq!(runtime.phase, ComponentRuntimePhase::DirectoryPrepared);
            let authority = runtime
                .authority
                .expect("Directory-prepared Component retains authority");
            assert_eq!(authority.fleet.services.len(), 1);
            assert!(authority.component_group.is_some());
        }
        assert_prepared(&fixture.pic, fixture.root.root_id);
    }

    #[cfg(test)]
    fn assert_activated_grouped_runtimes(
        fixture: &PreparedGroupedProvisioningFixture,
        status: &FleetComponentProvisioningStatusResponse,
    ) {
        assert_eq!(status.runtime_activated_root_count, 1);
        assert_eq!(status.current_activation, None);
        assert_eq!(status.activation_in_flight_root, None);
        assert!(status.runtimes_activated_at_ns.is_some());
        let root: Result<RootComponentProvisioningStatusResponse, Error> = fixture
            .pic
            .query_candid_as(
                fixture.root.root_id,
                fixture.coordinator,
                CANIC_ROOT_COMPONENT_PROVISIONING_STATUS,
                (RootComponentProvisioningStatusRequest {
                    operation_id: fixture.request.operation_id,
                    plan_hash: fixture.request.plan_hash,
                },),
            )
            .expect("query activated root provisioning status transport");
        let root = root.expect("query activated root provisioning status");
        assert_eq!(root.phase, RootComponentProvisioningPhase::RuntimesActive);
        assert_eq!(root.activated_component_count, root.component_count);
        assert!(root.root_runtime_active);
        assert!(root.activation.is_some());
        for member in root
            .result
            .as_ref()
            .expect("active root retains provisioned result")
            .placements
            .iter()
            .flat_map(|placement| &placement.members)
        {
            let runtime: Result<ComponentRuntimeStatusResponse, Error> = fixture
                .pic
                .query_candid_as(
                    member.binding.canister_id,
                    fixture.root.root_id,
                    CANIC_COMPONENT_RUNTIME_STATUS,
                    (),
                )
                .expect("query active grouped Component runtime transport");
            assert_eq!(
                runtime
                    .expect("query active grouped Component runtime")
                    .phase,
                ComponentRuntimePhase::Active
            );
            let partition: Result<ComponentRegistryPartitionResponse, Error> = fixture
                .pic
                .query_candid(
                    fixture.root.root_id,
                    CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                    (ComponentRegistryPartitionRequest {
                        component: member.binding.component,
                    },),
                )
                .expect("query active grouped Component partition transport");
            assert_eq!(
                partition
                    .expect("active grouped Component partition")
                    .status,
                ComponentLifecycleStatus::Active
            );
        }
        let root_runtime: Result<FleetActivationStatusResponse, Error> = fixture
            .pic
            .query_candid(fixture.root.root_id, CANIC_FLEET_ACTIVATION_STATUS, ())
            .expect("query active root runtime transport");
        assert_eq!(
            root_runtime.expect("active root runtime").phase,
            FleetActivationPhase::Active
        );
    }

    #[test]
    fn active_registry_issues_component_and_component_child_role_attestations() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        super::super::role_attestation::assert_registry_bound_role_attestation(
            fixture.pic(),
            fixture.root,
            &fixture.issuer,
            &fixture.verifier,
        );
        let (child, _) = create_active_project_instance(&fixture);
        super::super::role_attestation::assert_registry_bound_child_role_attestation(
            fixture.pic(),
            fixture.root,
            &ComponentChildBinding {
                component: fixture.verifier.clone(),
                parent_canister_id: fixture.verifier.canister_id,
                role: CanisterRole::new("project_instance"),
                canister_id: child,
            },
        );
    }

    #[test]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the pooled Fleet fixture lease is intentionally retained for the full test"
    )]
    fn restored_root_preserves_its_allocation_head_but_cannot_allocate() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let active_registry: Result<FleetRegistryVersion, Error> = fixture
            .pic()
            .query_candid(fixture.coordinator, CANIC_FLEET_REGISTRY_VERSION, ())
            .expect("query active Fleet Registry version transport");
        let registry_request = RootComponentRegistryPreparationRequest {
            store_bootstrap: fixture.store_bootstrap.clone(),
            expected_fleet_registry: active_registry.expect("active Fleet Registry version"),
        };
        let before: Result<RootComponentRegistryStatusResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                (registry_request.clone(),),
            )
            .expect("query root Component Registry before snapshot transport");
        let before = before.expect("root Component Registry before snapshot");

        let snapshot_request = AuthoritySnapshotRequest {
            operation_id: [0xb4; 32],
        };
        seal_capture_live_resume_and_restore(&fixture, snapshot_request);

        let restored_fence: Result<AuthorityRestoreFenceStatusResponse, Error> = fixture
            .pic()
            .query_candid(fixture.root, CANIC_AUTHORITY_RESTORE_FENCE_STATUS, ())
            .expect("restored root authority fence status transport");
        assert_eq!(
            restored_fence
                .expect("restored root authority fence status")
                .phase,
            AuthorityRestoreFencePhase::Sealed
        );
        let after: Result<RootComponentRegistryStatusResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                (registry_request,),
            )
            .expect("query restored root Component Registry transport");
        assert_eq!(
            after.expect("restored root Component Registry"),
            before,
            "snapshot restore must preserve the exact allocation head"
        );

        let rejected_resume: Result<AuthorityRestoreFenceStatusResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_AUTHORITY_SNAPSHOT_RESUME,
                (snapshot_request,),
            )
            .expect("restored root authority snapshot resume transport");
        assert_eq!(
            rejected_resume
                .expect_err("restored root authority must remain mutation-fenced")
                .code(),
            canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
        );
        let fresh_allocation: Result<Result<RootComponentAllocationResponse, Error>, _> =
            fixture.pic().update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (RootComponentAllocationRequest {
                    operation_id: [0xb5; 32],
                    component_spec: fixture.verifier.component_spec.clone(),
                },),
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
        let sealed: Result<AuthorityRestoreFenceStatusResponse, Error> = fixture
            .pic()
            .update_candid(fixture.root, CANIC_AUTHORITY_SNAPSHOT_PREPARE, (request,))
            .expect("root authority snapshot prepare transport");
        assert_eq!(
            sealed.expect("root authority snapshot prepare").phase,
            AuthorityRestoreFencePhase::Sealed
        );
        let snapshots = fixture
            .pic()
            .capture_controller_snapshots(fixture.root, [fixture.root])
            .expect("root authority snapshot capture");
        let resumed: Result<AuthorityRestoreFenceStatusResponse, Error> = fixture
            .pic()
            .update_candid(fixture.root, CANIC_AUTHORITY_SNAPSHOT_RESUME, (request,))
            .expect("live root authority snapshot resume transport");
        assert_eq!(
            resumed.expect("live root authority snapshot resume").phase,
            AuthorityRestoreFencePhase::Open
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

    #[test]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the pooled Fleet fixture lease is intentionally retained for the full test"
    )]
    fn active_component_provisions_a_registered_child_through_root_capability() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let request_id = [0xd1; 32];
        let ttl_ns = 60_000_000_000;
        let envelope = RootCapabilityEnvelopeV1 {
            service: CapabilityService::Root,
            capability_version: CAPABILITY_VERSION_V1,
            capability: Request::CreateCanister(CreateCanisterRequest {
                canister_role: CanisterRole::new("project_instance"),
                parent: CreateCanisterParent::ThisCanister,
                extra_arg: Some(vec![9, 8, 7]),
                metadata: Some(RootRequestMetadata { request_id, ttl_ns }),
            }),
            proof: CapabilityProof::Structural,
            metadata: CapabilityRequestMetadata {
                request_id,
                issued_at_ns: fixture.pic().current_time_nanos(),
                ttl_ns,
            },
        };

        let provisioned = root_capability_response(&fixture, envelope.clone());
        let Response::CreateCanister(provisioned) = provisioned.response else {
            panic!("root capability must return a create-Canister response");
        };
        let child = provisioned.new_canister_pid;
        let repeated = root_capability_response(&fixture, envelope);
        let Response::CreateCanister(repeated) = repeated.response else {
            panic!("root capability retry must return a create-Canister response");
        };
        assert_eq!(
            repeated.new_canister_pid, child,
            "exact retry must return the original Component Child"
        );

        fixture
            .pic()
            .wait_for_ready(child, 50, "root-capability Component Child");
        let children: Result<Page<CanisterInfo>, Error> = fixture
            .pic()
            .query_candid(
                fixture.verifier.canister_id,
                CANIC_CANISTER_CHILDREN,
                (PageRequest {
                    limit: 100,
                    offset: 0,
                },),
            )
            .expect("query Component local children transport");
        assert!(
            children
                .expect("query Component local children")
                .entries
                .into_iter()
                .any(|entry| entry.pid == child
                    && entry.role == CanisterRole::new("project_instance")),
            "activated child must converge into its parent's local child cache"
        );
    }

    #[test]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the pooled Fleet fixture lease is intentionally retained for the full test"
    )]
    fn active_component_recycles_a_child_through_component_registry_removal() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let (child, _) = create_active_project_instance(&fixture);
        let request_id = [0xd2; 32];
        let ttl_ns = 60_000_000_000;
        let envelope = RootCapabilityEnvelopeV1 {
            service: CapabilityService::Root,
            capability_version: CAPABILITY_VERSION_V1,
            capability: Request::RecycleCanister(RecycleCanisterRequest {
                canister_pid: child,
                metadata: Some(RootRequestMetadata { request_id, ttl_ns }),
            }),
            proof: CapabilityProof::Structural,
            metadata: CapabilityRequestMetadata {
                request_id,
                issued_at_ns: fixture.pic().current_time_nanos(),
                ttl_ns,
            },
        };

        assert!(matches!(
            root_capability_response(&fixture, envelope.clone()).response,
            Response::RecycleCanister
        ));
        assert_recycled_pool_asset(&fixture, child);
        let children: Result<Page<CanisterInfo>, Error> = fixture
            .pic()
            .query_candid(
                fixture.verifier.canister_id,
                CANIC_CANISTER_CHILDREN,
                (PageRequest {
                    limit: 100,
                    offset: 0,
                },),
            )
            .expect("query Component local children transport");
        assert!(
            children
                .expect("query Component local children")
                .entries
                .into_iter()
                .all(|entry| entry.pid != child),
            "completed recycle must remove the child from its parent's local authority"
        );
        assert!(matches!(
            root_capability_response(&fixture, envelope).response,
            Response::RecycleCanister
        ));
    }

    #[cfg(test)]
    fn assert_recycled_pool_asset(
        fixture: &ActiveComponentRegistryFixture,
        canister_id: Principal,
    ) {
        let live = fixture
            .pic()
            .canister_status(canister_id, Some(fixture.root))
            .expect("recycled physical Canister remains present");
        assert_eq!(live.settings.controllers, vec![fixture.root]);
        assert_eq!(live.module_hash, None);

        let status = pool_status(fixture);
        let asset = status
            .entries
            .iter()
            .find(|asset| asset.canister_id == canister_id)
            .expect("recycled Canister remains in paid asset inventory");
        assert_eq!(asset.origin, CanisterPoolAssetOrigin::Recycled);
        match &asset.status {
            CanisterPoolAssetStatus::Ready => {
                assert!(asset.cycles >= status.config.canister_cycles);
            }
            CanisterPoolAssetStatus::Failed { reason } => {
                assert!(!reason.is_empty());
                assert!(asset.cycles < status.config.canister_cycles);
            }
            status => panic!("recycled asset has unexpected status {status:?}"),
        }
    }

    #[cfg(test)]
    fn assert_reset_recycling_asset(
        fixture: &ActiveComponentRegistryFixture,
        canister_id: Principal,
        component: ComponentInstanceId,
    ) {
        let live = fixture
            .pic()
            .canister_status(canister_id, Some(fixture.root))
            .expect("reset recycling Canister remains present");
        assert_eq!(live.settings.controllers, vec![fixture.root]);
        assert_eq!(live.module_hash, None);

        let status = pool_status(fixture);
        let asset = status
            .entries
            .iter()
            .find(|asset| asset.canister_id == canister_id)
            .expect("reset Canister remains in exclusive physical inventory");
        assert_eq!(asset.origin, CanisterPoolAssetOrigin::Recycled);
        assert!(matches!(
            &asset.status,
            CanisterPoolAssetStatus::Recycling { claim, reset }
                if claim.component == component && *reset == CanisterPoolRecycleReset::Ready
        ));
    }

    #[cfg(test)]
    fn pool_status(fixture: &ActiveComponentRegistryFixture) -> CanisterPoolResponse {
        root_pool_status(fixture.pic(), fixture.root)
    }

    #[cfg(test)]
    fn root_pool_status(pic: &PocketIc, root: Principal) -> CanisterPoolResponse {
        let status: Result<CanisterPoolResponse, Error> = pic
            .query_candid(
                root,
                CANIC_POOL_LIST,
                (CanisterPoolStatusRequest {
                    start_after: None,
                    limit: 256,
                },),
            )
            .expect("query Canister pool transport");
        status.expect("query Canister pool")
    }

    #[cfg(test)]
    fn handoff_all_pool_assets(fixture: &ActiveComponentRegistryFixture) {
        let before = pool_status(fixture);
        assert_eq!(before.store, 1, "the adopted Store remains root-owned");
        let assets = before
            .entries
            .into_iter()
            .filter(|asset| {
                matches!(
                    asset.status,
                    CanisterPoolAssetStatus::Ready | CanisterPoolAssetStatus::Failed { .. }
                )
            })
            .collect::<Vec<_>>();
        assert!(!assets.is_empty(), "root draining must retain paid assets");
        let expected_handoffs = u64::try_from(assets.len()).expect("bounded pool length");
        for asset in assets {
            assert!(matches!(
                asset.status,
                CanisterPoolAssetStatus::Ready | CanisterPoolAssetStatus::Failed { .. }
            ));
            let handed_off: Result<PoolAdminResponse, Error> = fixture
                .pic()
                .update_candid(
                    fixture.root,
                    CANIC_POOL_ADMIN,
                    (PoolAdminCommand::Handoff {
                        canister_id: asset.canister_id,
                        recipient: fixture.coordinator,
                    },),
                )
                .expect("handoff pool asset transport");
            let handed_off = handed_off.expect("handoff pool asset");
            assert_eq!(
                handed_off,
                PoolAdminResponse::HandedOff {
                    canister_id: asset.canister_id,
                    recipient: fixture.coordinator,
                }
            );
            let live = fixture
                .pic()
                .canister_status(asset.canister_id, Some(fixture.root))
                .expect("handed-off asset remains present");
            assert_eq!(live.settings.controllers.len(), 2);
            assert!(live.settings.controllers.contains(&fixture.root));
            assert!(live.settings.controllers.contains(&fixture.coordinator));

            let replay: Result<PoolAdminResponse, Error> = fixture
                .pic()
                .update_candid(
                    fixture.root,
                    CANIC_POOL_ADMIN,
                    (PoolAdminCommand::Handoff {
                        canister_id: asset.canister_id,
                        recipient: fixture.coordinator,
                    },),
                )
                .expect("replay pool asset handoff transport");
            assert_eq!(replay.expect("replay pool asset handoff"), handed_off);
        }
        let status = pool_status(fixture);
        assert_eq!(status.tracked, 1);
        assert_eq!(status.store, 1);
        assert_eq!(status.pooled, 0);
        assert!(matches!(
            status.entries.as_slice(),
            [asset] if matches!(asset.status, CanisterPoolAssetStatus::Store)
        ));
        assert_eq!(status.completed_handoffs, expected_handoffs);
    }

    #[cfg(test)]
    fn root_capability_response(
        fixture: &ActiveComponentRegistryFixture,
        envelope: RootCapabilityEnvelopeV1,
    ) -> RootCapabilityResponseV1 {
        let response: Result<RootCapabilityResponseV1, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_RESPONSE_CAPABILITY_V1,
                (envelope,),
            )
            .expect("root capability transport");
        response.expect("root capability application")
    }

    #[test]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the pooled Fleet fixture lease is intentionally retained for the full test"
    )]
    fn active_component_provisions_one_same_root_peer_without_parentage() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let requester = fixture.verifier.clone();
        let operation_id = [0xb1; 32];
        let reserved = reserve_peer_component(&fixture, &requester, operation_id);
        let membership = activate_peer_component(&fixture, &requester, operation_id, &reserved);
        assert_eq!(membership.registry.status, ComponentLifecycleStatus::Active);
        assert_eq!(
            membership.registry.provisioning_origin,
            reserved.provisioning_origin
        );
        assert_ne!(membership.registry.binding.component, requester.component);
        assert_eq!(membership.registry.binding.fleet_subnet_root, fixture.root);
        assert_peer_grant_exhausted(&fixture, &requester);
    }

    #[cfg(test)]
    fn reserve_peer_component(
        fixture: &ActiveComponentRegistryFixture,
        requester: &ComponentBinding,
        operation_id: [u8; 32],
    ) -> RootComponentAllocationResponse {
        let allocation_request = RootPeerComponentAllocationRequest {
            operation_id,
            component_spec: "issuer".parse().expect("issuer Component Spec"),
            requester: PeerComponentRequester::SameRoot,
        };
        let denied: Result<RootComponentAllocationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_PEER_COMPONENT_ALLOCATE,
                (allocation_request.clone(),),
            )
            .expect("anonymous peer Component reservation transport");
        assert_eq!(
            denied
                .expect_err("anonymous caller must not reserve a peer Component")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );
        let reserved: Result<RootComponentAllocationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                requester.canister_id,
                CANIC_ROOT_PEER_COMPONENT_ALLOCATE,
                (allocation_request.clone(),),
            )
            .expect("reserve peer Component transport");
        let reserved = reserved.expect("reserve peer Component");
        let ComponentProvisioningOrigin::Component {
            requester: recorded_requester,
            grant,
        } = &reserved.provisioning_origin
        else {
            panic!("peer reservation must retain its Component origin");
        };
        assert_eq!(recorded_requester.as_ref(), requester);
        assert_eq!(grant.requester_component_spec, requester.component_spec);
        assert_eq!(
            grant.target_component_spec,
            allocation_request.component_spec
        );
        assert_eq!(grant.maximum_instances_per_requester_per_root, 1);
        assert_eq!(reserved.phase, RootComponentAllocationPhase::Reserved);
        let retried: Result<RootComponentAllocationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                requester.canister_id,
                CANIC_ROOT_PEER_COMPONENT_ALLOCATE,
                (allocation_request,),
            )
            .expect("retry peer Component reservation transport");
        assert_eq!(retried.expect("retry peer Component reservation"), reserved);
        let status: Result<RootComponentAllocationResponse, Error> = fixture
            .pic()
            .query_candid_as(
                fixture.root,
                requester.canister_id,
                CANIC_ROOT_PEER_COMPONENT_ALLOCATION_STATUS,
                (RootComponentAllocationStatusRequest { operation_id },),
            )
            .expect("query peer Component reservation transport");
        assert_eq!(status.expect("peer Component reservation status"), reserved);
        reserved
    }

    #[cfg(test)]
    fn activate_peer_component(
        fixture: &ActiveComponentRegistryFixture,
        requester: &ComponentBinding,
        operation_id: [u8; 32],
        reserved: &RootComponentAllocationResponse,
    ) -> RootComponentMembershipActivationResponse {
        let created: Result<RootComponentAllocationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                requester.canister_id,
                CANIC_ROOT_PEER_COMPONENT_CREATE,
                (RootComponentCreationRequest { operation_id },),
            )
            .expect("create peer Component transport");
        assert_eq!(
            created.expect("create peer Component").phase,
            RootComponentAllocationPhase::Created
        );
        let installed: Result<RootComponentAllocationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                requester.canister_id,
                CANIC_ROOT_PEER_COMPONENT_INSTALL,
                (RootComponentInstallRequest { operation_id },),
            )
            .expect("install peer Component transport");
        assert_eq!(
            installed.expect("install peer Component").phase,
            RootComponentAllocationPhase::Verified
        );
        let committed: Result<RootComponentCommitResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                requester.canister_id,
                CANIC_ROOT_PEER_COMPONENT_COMMIT,
                (RootComponentCommitRequest { operation_id },),
            )
            .expect("commit peer Component transport");
        let committed = committed.expect("commit peer Component");
        assert_eq!(
            committed.registry.provisioning_origin,
            reserved.provisioning_origin
        );
        assert_eq!(
            committed.registry.status,
            ComponentLifecycleStatus::Prepared
        );
        let prepared: Result<RootComponentDirectoryPreparationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                requester.canister_id,
                CANIC_ROOT_PEER_COMPONENT_DIRECTORY_PREPARE,
                (RootComponentDirectoryPreparationRequest { operation_id },),
            )
            .expect("prepare peer Component Directory transport");
        assert_eq!(
            prepared
                .expect("prepare peer Component Directory")
                .target
                .phase,
            ComponentRuntimePhase::DirectoryPrepared
        );
        let activated: Result<RootComponentRuntimeActivationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                requester.canister_id,
                CANIC_ROOT_PEER_COMPONENT_RUNTIME_ACTIVATE,
                (RootComponentRuntimeActivationRequest { operation_id },),
            )
            .expect("activate peer Component runtime transport");
        assert_eq!(
            activated
                .expect("activate peer Component runtime")
                .target
                .phase,
            ComponentRuntimePhase::Active
        );
        let membership: Result<RootComponentMembershipActivationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                requester.canister_id,
                CANIC_ROOT_PEER_COMPONENT_MEMBERSHIP_ACTIVATE,
                (RootComponentMembershipActivationRequest { operation_id },),
            )
            .expect("activate peer Component membership transport");
        membership.expect("activate peer Component membership")
    }

    #[cfg(test)]
    fn assert_peer_grant_exhausted(
        fixture: &ActiveComponentRegistryFixture,
        requester: &ComponentBinding,
    ) {
        let exhausted: Result<RootComponentAllocationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                requester.canister_id,
                CANIC_ROOT_PEER_COMPONENT_ALLOCATE,
                (RootPeerComponentAllocationRequest {
                    operation_id: [0xb2; 32],
                    component_spec: "issuer".parse().expect("issuer Component Spec"),
                    requester: PeerComponentRequester::SameRoot,
                },),
            )
            .expect("exhausted peer Component reservation transport");
        assert_eq!(
            exhausted
                .expect_err("peer provisioning grant must be exhausted")
                .code(),
            canic_core::diagnostics::codes::CAPACITY_LIMIT.raw_code()
        );
    }

    #[test]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the pooled Fleet fixture lease is intentionally retained for the full test"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "one PocketIC journey keeps the fence, stop and deletion boundary coherent"
    )]
    fn active_root_deletes_one_exact_registered_child_before_membership_removal() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let (child, registry) = create_active_project_instance(&fixture);
        let operation_id = [0xd1; 32];
        let begin_request = RootComponentSubtreeRemovalRequest {
            operation_id,
            component: fixture.verifier.component,
            target_canister_id: child,
            expected_registry: registry,
        };
        let fenced: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_BEGIN,
                (begin_request,),
            )
            .expect("begin subtree removal transport");
        assert_eq!(
            fenced.expect("begin subtree removal").phase,
            RootComponentSubtreeRemovalPhase::Fenced
        );

        let advanced: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_ADVANCE,
                (RootComponentSubtreeRemovalAdvanceRequest {
                    operation_id,
                    component: fixture.verifier.component,
                    expected_traversal_steps: 0,
                },),
            )
            .expect("advance subtree removal transport");
        let advanced = advanced.expect("advance subtree removal");
        assert_eq!(advanced.traversal_steps, 1);
        assert!(matches!(
            &advanced.phase,
            RootComponentSubtreeRemovalPhase::LeafSelected(leaf)
                if leaf.canister_id == child
                    && leaf.parent_canister_id == fixture.verifier.canister_id
        ));

        let stop_request = RootComponentSubtreeRemovalStopRequest {
            operation_id,
            component: fixture.verifier.component,
            expected_traversal_steps: advanced.traversal_steps,
            expected_leaf_canister_id: child,
            expected_leaf_parent_canister_id: fixture.verifier.canister_id,
        };
        let prepared: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP_PREPARE,
                (RootComponentSubtreeRemovalStopPreparationRequest {
                    operation_id: stop_request.operation_id,
                    component: stop_request.component,
                    expected_traversal_steps: stop_request.expected_traversal_steps,
                    expected_leaf_canister_id: stop_request.expected_leaf_canister_id,
                    expected_leaf_parent_canister_id: stop_request.expected_leaf_parent_canister_id,
                },),
            )
            .expect("prepare subtree leaf stop transport");
        assert!(matches!(
            prepared.expect("prepare subtree leaf stop").phase,
            RootComponentSubtreeRemovalPhase::StopIntent(_)
        ));

        let stopped: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP,
                (stop_request,),
            )
            .expect("stop subtree leaf transport");
        let stopped = stopped.expect("stop subtree leaf");
        let RootComponentSubtreeRemovalPhase::Stopped(receipt) = &stopped.phase else {
            panic!("subtree leaf must retain an independently observed stopped receipt");
        };
        assert_eq!(receipt.stop.leaf.canister_id, child);
        assert_eq!(
            receipt.stop.leaf.parent_canister_id,
            fixture.verifier.canister_id
        );
        assert_eq!(receipt.stop.controller, fixture.root);
        assert_ne!(receipt.observed_module_hash, [0; 32]);
        let live = fixture
            .pic()
            .canister_status(child, Some(fixture.root))
            .expect("stopped child status");
        assert_eq!(
            format!("{:?}", live.status),
            "Stopped",
            "the independently receipted child must be stopped in PocketIC"
        );
        assert_eq!(live.settings.controllers, vec![fixture.root]);
        assert_eq!(
            live.module_hash.as_deref(),
            Some(receipt.observed_module_hash.as_slice())
        );

        let retry: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP,
                (stop_request,),
            )
            .expect("retry stopped subtree leaf transport");
        assert_eq!(retry.expect("retry stopped subtree leaf"), stopped);

        let durable: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STATUS,
                (RootComponentSubtreeRemovalStatusRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("query stopped subtree receipt transport");
        assert_eq!(durable.expect("query stopped subtree receipt"), stopped);

        let delete_request = RootComponentSubtreeRemovalDeleteRequest {
            operation_id,
            component: fixture.verifier.component,
            expected_traversal_steps: advanced.traversal_steps,
            expected_leaf_canister_id: child,
            expected_leaf_parent_canister_id: fixture.verifier.canister_id,
        };
        let prepared_delete: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DELETE_PREPARE,
                (RootComponentSubtreeRemovalDeletePreparationRequest {
                    operation_id: delete_request.operation_id,
                    component: delete_request.component,
                    expected_traversal_steps: delete_request.expected_traversal_steps,
                    expected_leaf_canister_id: delete_request.expected_leaf_canister_id,
                    expected_leaf_parent_canister_id: delete_request
                        .expected_leaf_parent_canister_id,
                },),
            )
            .expect("prepare subtree leaf deletion transport");
        assert!(matches!(
            prepared_delete
                .expect("prepare subtree leaf deletion")
                .phase,
            RootComponentSubtreeRemovalPhase::DeleteIntent(_)
        ));

        let deleted: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DELETE,
                (delete_request,),
            )
            .expect("delete subtree leaf transport");
        let deleted = deleted.expect("delete subtree leaf");
        let RootComponentSubtreeRemovalPhase::Deleted(receipt) = &deleted.phase else {
            panic!("subtree leaf must retain an independently observed deleted receipt");
        };
        assert_eq!(receipt.deletion.stopped.stop.leaf.canister_id, child);
        assert_eq!(
            receipt.deletion.stopped.stop.leaf.parent_canister_id,
            fixture.verifier.canister_id
        );
        assert_eq!(receipt.deletion.stopped.stop.controller, fixture.root);
        assert_reset_recycling_asset(&fixture, child, fixture.verifier.component);

        let retry: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DELETE,
                (delete_request,),
            )
            .expect("retry deleted subtree leaf transport");
        assert_eq!(retry.expect("retry deleted subtree leaf"), deleted);

        let durable: Result<RootComponentSubtreeRemovalResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STATUS,
                (RootComponentSubtreeRemovalStatusRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("query deleted subtree receipt transport");
        assert_eq!(durable.expect("query deleted subtree receipt"), deleted);
        assert_eq!(
            deleted.target_status,
            ComponentLifecycleStatus::Active,
            "deletion must not silently mutate Registry membership"
        );
    }

    #[test]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the pooled Fleet fixture lease is intentionally retained for the full test"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "one PocketIC journey proves qualified top-level stop, deletion and membership removal"
    )]
    fn published_draining_root_removes_one_exact_empty_component() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let root_draining = assert_root_draining_fence(&fixture);
        let published = assert_coordinator_root_draining_publication(&fixture, &root_draining);
        assert_root_draining_mirror_activation(&fixture, &root_draining, &published);
        let partition: Result<ComponentRegistryPartitionResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: fixture.verifier.component,
                },),
            )
            .expect("query top-level Component partition transport");
        let partition = partition.expect("query top-level Component partition");
        assert_eq!(partition.committed_descendants, 0);

        let operation_id = [0xd2; 32];
        let drained: Result<RootComponentDrainingResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_DRAINING_BEGIN,
                (RootComponentDrainingRequest {
                    operation_id,
                    component: fixture.verifier.component,
                    expected_registry: partition.head,
                },),
            )
            .expect("begin top-level Component draining transport");
        let drained = drained.expect("begin top-level Component draining");

        let quiescent: Result<RootComponentQuiescenceResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_QUIESCE,
                (RootComponentQuiescenceRequest {
                    operation_id,
                    component: fixture.verifier.component,
                    expected_registry: drained.registry,
                },),
            )
            .expect("quiesce top-level Component transport");
        let quiescent = quiescent.expect("quiesce top-level Component");
        let RootComponentQuiescencePhase::Quiescent(quiescence) = &quiescent.phase else {
            panic!("top-level Component must retain an independently observed stopped receipt");
        };
        assert_eq!(quiescence.stop.canister_id, fixture.verifier.canister_id);
        assert_eq!(quiescence.stop.controller, fixture.root);
        assert_ne!(quiescence.observed_module_hash, [0; 32]);

        let empty: Result<RootComponentDrainingAdvanceResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_DRAINING_ADVANCE,
                (RootComponentDrainingAdvanceRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("observe empty top-level Component inventory transport");
        let empty = empty.expect("observe empty top-level Component inventory");
        let RootComponentDrainingAdvancePhase::DescendantsEmpty(empty) = empty.phase else {
            panic!("top-level Component must have exact empty descendant inventory");
        };

        let final_inventory: Result<RootComponentFinalInventoryResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_DRAINING_INVENTORY_FINALIZE,
                (RootComponentFinalInventoryRequest {
                    operation_id,
                    component: fixture.verifier.component,
                    expected_registry: empty.registry,
                },),
            )
            .expect("finalize top-level Component inventory transport");
        let final_inventory = final_inventory.expect("finalize top-level Component inventory");
        let delete_request = RootComponentDeletionRequest {
            operation_id,
            component: fixture.verifier.component,
            expected_inventory_hash: final_inventory.inventory.inventory_hash,
        };
        let deleted: Result<RootComponentDeletionResponse, Error> = fixture
            .pic()
            .update_candid(fixture.root, CANIC_ROOT_COMPONENT_DELETE, (delete_request,))
            .expect("delete top-level Component transport");
        let deleted = deleted.expect("delete top-level Component");
        let RootComponentDeletionPhase::Deleted(receipt) = &deleted.phase else {
            panic!("top-level Component must retain independently observed workload deletion");
        };
        assert_eq!(receipt.deletion.final_inventory, final_inventory.inventory);
        assert_eq!(
            receipt.deletion.quiescence.stop.canister_id,
            fixture.verifier.canister_id
        );
        assert_eq!(receipt.deletion.quiescence.stop.controller, fixture.root);
        assert_reset_recycling_asset(
            &fixture,
            fixture.verifier.canister_id,
            fixture.verifier.component,
        );

        let retry: Result<RootComponentDeletionResponse, Error> = fixture
            .pic()
            .update_candid(fixture.root, CANIC_ROOT_COMPONENT_DELETE, (delete_request,))
            .expect("retry top-level Component deletion transport");
        assert_eq!(retry.expect("retry top-level Component deletion"), deleted);
        let durable: Result<RootComponentDeletionResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_DELETION_STATUS,
                (RootComponentDeletionStatusRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("query top-level Component deletion receipt transport");
        assert_eq!(
            durable.expect("query top-level Component deletion receipt"),
            deleted
        );

        let retained: Result<ComponentRegistryPartitionResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: fixture.verifier.component,
                },),
            )
            .expect("query retained Component membership transport");
        assert_eq!(
            retained.expect("retained Component membership").status,
            ComponentLifecycleStatus::Draining
        );

        let removed: Result<RootComponentDeletionResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_MEMBERSHIP_REMOVE,
                (delete_request,),
            )
            .expect("remove top-level Component membership transport");
        let removed = removed.expect("remove top-level Component membership");
        let RootComponentDeletionPhase::MembershipRemoved(removal) = &removed.phase else {
            panic!("top-level Component must retain terminal membership-removal authority");
        };
        assert_eq!(&removal.deleted, receipt);
        assert_eq!(removal.root_committed_component_instances, 1);
        assert_eq!(removal.root_known_created_component_canisters, 1);
        assert_ne!(removal.allocation_operation_id, [0; 32]);
        assert_ne!(removal.removal_hash, [0; 32]);

        let removal_retry: Result<RootComponentDeletionResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_MEMBERSHIP_REMOVE,
                (delete_request,),
            )
            .expect("retry top-level Component membership removal transport");
        assert_eq!(
            removal_retry.expect("retry top-level Component membership removal"),
            removed
        );
        let deletion_retry: Result<RootComponentDeletionResponse, Error> = fixture
            .pic()
            .update_candid(fixture.root, CANIC_ROOT_COMPONENT_DELETE, (delete_request,))
            .expect("retry top-level Component deletion after membership removal transport");
        assert_eq!(
            deletion_retry.expect("retry top-level Component deletion after membership removal"),
            removed
        );
        let durable_removed: Result<RootComponentDeletionResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_DELETION_STATUS,
                (RootComponentDeletionStatusRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("query terminal top-level Component removal transport");
        assert_eq!(
            durable_removed.expect("query terminal top-level Component removal"),
            removed
        );

        let absent_membership: Result<ComponentRegistryPartitionResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: fixture.verifier.component,
                },),
            )
            .expect("query removed Component membership transport");
        assert_eq!(
            absent_membership
                .expect_err("removed Component membership must be absent")
                .code(),
            canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
        );
        let durable_fence: Result<FleetSubnetRootDrainingResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_DRAINING_STATUS,
                (FleetSubnetRootDrainingStatusRequest {
                    operation_id: root_draining.operation_id,
                },),
            )
            .expect("query root draining fence after Component removal transport");
        assert_eq!(
            durable_fence.expect("root draining fence after Component removal"),
            root_draining
        );

        let issuer_removed = remove_empty_component(&fixture, &fixture.issuer, [0xd3; 32]);
        let RootComponentDeletionPhase::MembershipRemoved(issuer_removal) = issuer_removed.phase
        else {
            panic!("issuer Component must retain terminal membership-removal authority");
        };
        assert_eq!(issuer_removal.root_committed_component_instances, 0);
        assert_eq!(issuer_removal.root_known_created_component_canisters, 0);

        handoff_all_pool_assets(&fixture);

        let inventory_request = FleetSubnetRootFinalInventoryRequest {
            operation_id: root_draining.operation_id,
            expected_registry: published.version,
        };
        let final_inventory: Result<FleetSubnetRootFinalInventoryResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_DRAINING_INVENTORY_FINALIZE,
                (inventory_request.clone(),),
            )
            .expect("finalize Fleet Subnet Root inventory transport");
        let final_inventory = final_inventory.expect("finalize Fleet Subnet Root inventory");
        assert_eq!(final_inventory.operation_id, root_draining.operation_id);
        assert_eq!(
            final_inventory.registry,
            inventory_request.expected_registry
        );
        assert_eq!(final_inventory.removed_component_instances, 2);
        assert_eq!(
            final_inventory.root_registry_encoded_bytes,
            issuer_removal.root_registry_encoded_bytes
        );
        assert_ne!(final_inventory.terminal_component_history_hash, [0; 32]);
        assert_ne!(final_inventory.wasm_store_catalog_hash, [0; 32]);
        assert!(final_inventory.wasm_store_catalog_entries > 0);
        assert!(final_inventory.wasm_store_occupied_bytes > 0);
        assert!(final_inventory.wasm_store_gc_prepared_at_secs > 0);
        assert_ne!(final_inventory.inventory_hash, [0; 32]);

        let retry: Result<FleetSubnetRootFinalInventoryResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_DRAINING_INVENTORY_FINALIZE,
                (inventory_request,),
            )
            .expect("retry Fleet Subnet Root inventory transport");
        assert_eq!(
            retry.expect("retry Fleet Subnet Root inventory"),
            final_inventory
        );
        let durable: Result<FleetSubnetRootFinalInventoryResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_DRAINING_INVENTORY_STATUS,
                (FleetSubnetRootFinalInventoryStatusRequest {
                    operation_id: root_draining.operation_id,
                },),
            )
            .expect("query Fleet Subnet Root inventory transport");
        assert_eq!(
            durable.expect("query Fleet Subnet Root inventory"),
            final_inventory
        );

        let removal_request = FleetSubnetRootRemovalRequest {
            operation_id: root_draining.operation_id,
            expected_registry: final_inventory.registry.clone(),
        };
        let removed: Result<FleetSubnetRootRemovalPublicationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_REMOVAL_PUBLISH,
                (removal_request.clone(),),
            )
            .expect("publish Fleet Subnet Root removal transport");
        let removed = removed.expect("publish Fleet Subnet Root removal");
        assert_eq!(removed.final_inventory, final_inventory);
        assert_eq!(removed.previous_version, removal_request.expected_registry);
        assert_eq!(
            removed.version.revision,
            removed.previous_version.revision + 1
        );

        let retry: Result<FleetSubnetRootRemovalPublicationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_REMOVAL_PUBLISH,
                (removal_request,),
            )
            .expect("retry Fleet Subnet Root removal transport");
        assert_eq!(retry.expect("retry Fleet Subnet Root removal"), removed);
        let durable: Result<FleetSubnetRootRemovalPublicationResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_REMOVAL_STATUS,
                (FleetSubnetRootRemovalStatusRequest {
                    operation_id: root_draining.operation_id,
                },),
            )
            .expect("query Fleet Subnet Root removal transport");
        assert_eq!(durable.expect("query Fleet Subnet Root removal"), removed);

        let registry: Result<FleetRegistry, Error> = fixture
            .pic()
            .query_candid(fixture.coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query Removed Fleet Registry transport");
        let registry = registry.expect("query Removed Fleet Registry");
        assert_eq!(registry.revision, removed.version.revision);
        assert_eq!(
            registry.fleet_subnet_roots[0].status,
            FleetSubnetRootStatus::Removed
        );

        let reclamation_request = FleetSubnetRootStoreReclamationRequest {
            operation_id: root_draining.operation_id,
            expected_final_inventory_hash: final_inventory.inventory_hash,
        };
        let reclamation: Result<FleetSubnetRootStoreReclamationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_STORE_RECLAIM,
                (reclamation_request,),
            )
            .expect("reclaim Fleet Subnet Root Store transport");
        let reclamation = reclamation.expect("reclaim Fleet Subnet Root Store");
        assert_eq!(reclamation.wasm_store, final_inventory.wasm_store);
        assert_eq!(
            reclamation.final_inventory_hash,
            final_inventory.inventory_hash
        );
        assert_eq!(
            reclamation.reclaimed_store_bytes,
            final_inventory.wasm_store_occupied_bytes
        );
        assert_eq!(
            reclamation.reclaimed_catalog_entries,
            final_inventory.wasm_store_catalog_entries
        );
        assert_eq!(
            reclamation.reclaimed_template_count,
            final_inventory.wasm_store_template_count
        );
        assert_eq!(
            reclamation.reclaimed_release_count,
            final_inventory.wasm_store_release_count
        );
        assert_eq!(
            reclamation.gc_prepared_at_secs,
            final_inventory.wasm_store_gc_prepared_at_secs
        );
        assert!(reclamation.gc_started_at_secs >= reclamation.gc_prepared_at_secs);
        assert!(reclamation.gc_completed_at_secs >= reclamation.gc_started_at_secs);
        assert_eq!(reclamation.gc_runs_completed, 1);
        assert_ne!(reclamation.reclamation_hash, [0; 32]);

        let retry: Result<FleetSubnetRootStoreReclamationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_STORE_RECLAIM,
                (reclamation_request,),
            )
            .expect("retry Fleet Subnet Root Store reclamation transport");
        assert_eq!(
            retry.expect("retry Fleet Subnet Root Store reclamation"),
            reclamation
        );
        let durable: Result<FleetSubnetRootStoreReclamationResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_STORE_RECLAMATION_STATUS,
                (FleetSubnetRootStoreReclamationStatusRequest {
                    operation_id: root_draining.operation_id,
                },),
            )
            .expect("query Fleet Subnet Root Store reclamation transport");
        assert_eq!(
            durable.expect("query Fleet Subnet Root Store reclamation"),
            reclamation
        );

        let store_status: Result<WasmStoreStatusResponse, Error> = fixture
            .pic()
            .query_candid_as(
                final_inventory.wasm_store,
                fixture.root,
                CANIC_WASM_STORE_STATUS,
                (),
            )
            .expect("query reclaimed Store status transport");
        let store_status = store_status.expect("query reclaimed Store status");
        assert_eq!(store_status.gc.mode, WasmStoreGcMode::Complete);
        assert_eq!(store_status.gc.runs_completed, 1);
        assert_eq!(store_status.occupied_store_bytes, 0);
        assert_eq!(store_status.template_count, 0);
        assert_eq!(store_status.release_count, 0);
        assert!(store_status.templates.is_empty());
        let store_catalog: Result<Vec<WasmStoreCatalogEntryResponse>, Error> = fixture
            .pic()
            .query_candid_as(
                final_inventory.wasm_store,
                fixture.root,
                CANIC_WASM_STORE_CATALOG,
                (),
            )
            .expect("query reclaimed Store catalog transport");
        assert!(
            store_catalog
                .expect("query reclaimed Store catalog")
                .is_empty()
        );

        let binding_request = FleetSubnetRootStoreBindingFinalizationRequest {
            operation_id: root_draining.operation_id,
            expected_reclamation_hash: reclamation.reclamation_hash,
        };
        let finalization: Result<FleetSubnetRootStoreBindingFinalizationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZE,
                (binding_request,),
            )
            .expect("finalize Fleet Subnet Root Store binding transport");
        let finalization = finalization.expect("finalize Fleet Subnet Root Store binding");
        assert_eq!(finalization.wasm_store, final_inventory.wasm_store);
        assert_eq!(
            finalization.final_inventory_hash,
            final_inventory.inventory_hash
        );
        assert_eq!(finalization.reclamation_hash, reclamation.reclamation_hash);
        assert_eq!(
            finalization.finalized_generation,
            finalization.source_generation + 3
        );
        assert!(finalization.finalized_at_secs > 0);
        assert_ne!(finalization.finalization_hash, [0; 32]);

        let retry: Result<FleetSubnetRootStoreBindingFinalizationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZE,
                (binding_request,),
            )
            .expect("retry Fleet Subnet Root Store binding finalization transport");
        assert_eq!(
            retry.expect("retry Fleet Subnet Root Store binding finalization"),
            finalization
        );
        let durable: Result<FleetSubnetRootStoreBindingFinalizationResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZATION_STATUS,
                (FleetSubnetRootStoreBindingFinalizationStatusRequest {
                    operation_id: root_draining.operation_id,
                },),
            )
            .expect("query Fleet Subnet Root Store binding finalization transport");
        assert_eq!(
            durable.expect("query Fleet Subnet Root Store binding finalization"),
            finalization
        );

        let overview: Result<WasmStoreOverviewResponse, Error> = fixture
            .pic()
            .query_candid(fixture.root, canic::protocol::CANIC_WASM_STORE_OVERVIEW, ())
            .expect("query finalized Store binding overview transport");
        let overview = overview.expect("query finalized Store binding overview");
        assert_eq!(overview.publication.active_binding, None);
        assert_eq!(overview.publication.detached_binding, None);
        assert_eq!(overview.publication.retired_binding, None);
        assert_eq!(
            overview.publication.generation,
            finalization.finalized_generation
        );
        assert_eq!(overview.stores.len(), 1);
        assert_eq!(overview.stores[0].pid, final_inventory.wasm_store);
        assert_eq!(overview.stores[0].publication_slot, None);
        assert_eq!(overview.stores[0].gc.mode, WasmStoreGcMode::Complete);

        let root_cycles_before_deletion = canister_cycle_balance(fixture.pic(), fixture.root);
        let store_cycles_before_deletion =
            canister_cycle_balance(fixture.pic(), final_inventory.wasm_store);
        let deletion_request = FleetSubnetRootStoreDeletionRequest {
            operation_id: root_draining.operation_id,
            expected_binding_finalization_hash: finalization.finalization_hash,
        };
        let deletion: Result<FleetSubnetRootStoreDeletionResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_STORE_DELETE,
                (deletion_request,),
            )
            .expect("delete Fleet Subnet Root Store transport");
        let deletion = deletion.expect("delete Fleet Subnet Root Store");
        assert_eq!(deletion.wasm_store, final_inventory.wasm_store);
        assert_eq!(
            deletion.binding_finalization_hash,
            finalization.finalization_hash
        );
        assert_ne!(deletion.observed_module_hash, [0; 32]);
        assert!(deletion.observed_controllers.contains(&fixture.root));
        assert!(deletion.observed_cycles_before_reclamation <= store_cycles_before_deletion);
        assert!(deletion.observed_cycles_before_reclamation > deletion.retained_cycles_target);
        assert!(deletion.observed_cycles_after_reclamation <= deletion.retained_cycles_target);
        assert!(deletion.cycles_reclaimed_at_ns >= deletion.prepared_at_ns);
        assert!(deletion.observed_absent_at_ns >= deletion.prepared_at_ns);
        assert!(deletion.completed_at_ns >= deletion.observed_absent_at_ns);
        assert_ne!(deletion.deletion_hash, [0; 32]);
        assert!(
            fixture
                .pic()
                .canister_status(final_inventory.wasm_store, Some(fixture.root))
                .is_err(),
            "Store deletion receipt requires typed physical absence"
        );
        assert!(
            canister_cycle_balance(fixture.pic(), fixture.root) > root_cycles_before_deletion,
            "Store deletion must return excess cycles to the surviving root"
        );

        let retry: Result<FleetSubnetRootStoreDeletionResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_STORE_DELETE,
                (deletion_request,),
            )
            .expect("retry Fleet Subnet Root Store deletion transport");
        assert_eq!(
            retry.expect("retry Fleet Subnet Root Store deletion"),
            deletion
        );
        let durable: Result<FleetSubnetRootStoreDeletionResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_STORE_DELETION_STATUS,
                (FleetSubnetRootStoreDeletionStatusRequest {
                    operation_id: root_draining.operation_id,
                },),
            )
            .expect("query Fleet Subnet Root Store deletion transport");
        assert_eq!(
            durable.expect("query Fleet Subnet Root Store deletion"),
            deletion
        );

        let overview: Result<WasmStoreOverviewResponse, Error> = fixture
            .pic()
            .query_candid(fixture.root, canic::protocol::CANIC_WASM_STORE_OVERVIEW, ())
            .expect("query deleted Store overview transport");
        let overview = overview.expect("query deleted Store overview");
        assert_eq!(
            overview.publication.generation,
            finalization.finalized_generation
        );
        assert_eq!(overview.publication.active_binding, None);
        assert_eq!(overview.publication.detached_binding, None);
        assert_eq!(overview.publication.retired_binding, None);
        assert!(overview.stores.is_empty());

        assert_root_deletion_is_prepared_for_external_executor(
            &fixture,
            root_draining.operation_id,
            final_inventory.inventory_hash,
            deletion.deletion_hash,
        );
    }

    #[cfg(test)]
    fn assert_root_deletion_is_prepared_for_external_executor(
        fixture: &ActiveComponentRegistryFixture,
        operation_id: [u8; 32],
        final_inventory_hash: [u8; 32],
        store_deletion_hash: [u8; 32],
    ) {
        let status = fixture
            .pic()
            .canister_status(fixture.root, Some(Principal::anonymous()))
            .expect("observe root before external deletion");
        let idle_cycles_burned_per_day = nat_u128(&status.idle_cycles_burned_per_day);
        let freezing_threshold_seconds = nat_u128(&status.settings.freezing_threshold);
        let freezing_reserve = idle_cycles_burned_per_day
            .checked_mul(freezing_threshold_seconds)
            .expect("root freezing reserve")
            .div_ceil(86_400);
        let retained_cycles_target = freezing_reserve
            .checked_add(FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES)
            .expect("root deletion reserve");
        let coordinator_cycles_before =
            management_cycle_balance(fixture.pic(), fixture.coordinator);
        let preparation_request = FleetSubnetRootDeletionPreparationRequest {
            operation_id,
            expected_store_deletion_hash: store_deletion_hash,
            retained_cycles_target,
            observed_reserved_cycles: nat_u128(&status.reserved_cycles),
            observed_idle_cycles_burned_per_day: idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: freezing_threshold_seconds,
        };
        let preparation: Result<FleetSubnetRootDeletionPreparationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_DELETION_PREPARE,
                (preparation_request,),
            )
            .expect("prepare Fleet Subnet Root deletion transport");
        let preparation = preparation.expect("prepare Fleet Subnet Root deletion");
        assert_eq!(preparation.fleet_subnet_root, fixture.root);
        assert_eq!(preparation.coordinator, fixture.coordinator);
        assert_eq!(preparation.final_inventory_hash, final_inventory_hash);
        assert_eq!(preparation.store_deletion_hash, store_deletion_hash);
        assert!(
            preparation.observed_cycles_before_reclamation > preparation.retained_cycles_target
        );
        assert!(
            preparation.observed_cycles_after_reclamation <= preparation.retained_cycles_target
        );
        assert_ne!(preparation.coordinator_intent_hash, [0; 32]);
        assert_ne!(preparation.coordinator_readiness_hash, [0; 32]);
        assert!(
            management_cycle_balance(fixture.pic(), fixture.coordinator)
                > coordinator_cycles_before,
            "root deletion must return excess cycles to the surviving Coordinator"
        );
        let durable: Result<FleetSubnetRootDeletionPreparationResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_DELETION_PREPARATION_STATUS,
                (FleetSubnetRootDeletionPreparationStatusRequest { operation_id },),
            )
            .expect("query root deletion preparation transport");
        assert_eq!(
            durable.expect("query root deletion preparation"),
            preparation
        );

        assert_root_deletion_executor_intent(
            fixture,
            operation_id,
            &preparation,
            idle_cycles_burned_per_day,
            freezing_threshold_seconds,
        );
    }

    #[cfg(test)]
    fn assert_root_deletion_executor_intent(
        fixture: &ActiveComponentRegistryFixture,
        operation_id: [u8; 32],
        preparation: &FleetSubnetRootDeletionPreparationResponse,
        idle_cycles_burned_per_day: u128,
        freezing_threshold_seconds: u128,
    ) {
        let status = fixture
            .pic()
            .canister_status(fixture.root, Some(Principal::anonymous()))
            .expect("independently observe deletion-ready root");
        let mut controllers = status.settings.controllers.clone();
        controllers.sort();
        controllers.dedup();
        let observed_module_hash: [u8; 32] = status
            .module_hash
            .as_deref()
            .expect("installed root module")
            .try_into()
            .expect("root module hash");
        let execution_request = FleetSubnetRootDeletionExecutionRequest {
            operation_id,
            fleet_subnet_root: fixture.root,
            expected_readiness_hash: preparation.coordinator_readiness_hash,
            observed_module_hash,
            observed_controllers: controllers,
            observed_cycles_after_reclamation: nat_u128(&status.cycles),
            observed_reserved_cycles: nat_u128(&status.reserved_cycles),
            observed_idle_cycles_burned_per_day: idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: freezing_threshold_seconds,
        };
        let execution: Result<FleetSubnetRootDeletionExecutionResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.coordinator,
                CANIC_FLEET_REGISTRY_ROOT_DELETION_EXECUTION_BEGIN,
                (execution_request,),
            )
            .expect("begin external root deletion transport");
        let execution = execution.expect("begin external root deletion");
        assert_eq!(execution.executor, Principal::anonymous());
        assert_ne!(execution.execution_hash, [0; 32]);
        let execution_status: Result<FleetSubnetRootDeletionExecutionResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.coordinator,
                CANIC_FLEET_REGISTRY_ROOT_DELETION_EXECUTION_STATUS,
                (FleetSubnetRootDeletionStatusRequest {
                    operation_id,
                    fleet_subnet_root: fixture.root,
                },),
            )
            .expect("query external root deletion intent transport");
        assert_eq!(
            execution_status.expect("query external root deletion intent"),
            execution
        );
    }

    #[cfg(test)]
    fn nat_u128(value: &candid::Nat) -> u128 {
        u128::try_from(value.0.clone()).expect("management cycle value fits u128")
    }

    #[cfg(test)]
    fn management_cycle_balance(pic: &PocketIc, canister_id: Principal) -> u128 {
        let status = pic
            .canister_status(canister_id, Some(Principal::anonymous()))
            .expect("observe Canister cycle balance through management status");
        nat_u128(&status.cycles)
    }

    #[cfg(test)]
    fn canister_cycle_balance(pic: &PocketIc, canister_id: Principal) -> u128 {
        let response: Result<u128, Error> = pic
            .query_candid(canister_id, CANIC_CYCLE_BALANCE, ())
            .expect("query Canister cycle balance transport");
        response.expect("query Canister cycle balance")
    }

    #[cfg(test)]
    fn remove_empty_component(
        fixture: &ActiveComponentRegistryFixture,
        binding: &ComponentBinding,
        operation_id: [u8; 32],
    ) -> RootComponentDeletionResponse {
        let partition: Result<ComponentRegistryPartitionResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: binding.component,
                },),
            )
            .expect("query empty Component partition transport");
        let partition = partition.expect("query empty Component partition");
        assert_eq!(partition.committed_descendants, 0);
        let draining: Result<RootComponentDrainingResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_DRAINING_BEGIN,
                (RootComponentDrainingRequest {
                    operation_id,
                    component: binding.component,
                    expected_registry: partition.head,
                },),
            )
            .expect("begin empty Component draining transport");
        let draining = draining.expect("begin empty Component draining");
        let quiescent: Result<RootComponentQuiescenceResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_QUIESCE,
                (RootComponentQuiescenceRequest {
                    operation_id,
                    component: binding.component,
                    expected_registry: draining.registry,
                },),
            )
            .expect("quiesce empty Component transport");
        quiescent.expect("quiesce empty Component");
        let empty: Result<RootComponentDrainingAdvanceResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_DRAINING_ADVANCE,
                (RootComponentDrainingAdvanceRequest {
                    operation_id,
                    component: binding.component,
                },),
            )
            .expect("observe empty Component inventory transport");
        let RootComponentDrainingAdvancePhase::DescendantsEmpty(empty) =
            empty.expect("observe empty Component inventory").phase
        else {
            panic!("Component must have exact empty descendant inventory");
        };
        let inventory: Result<RootComponentFinalInventoryResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_DRAINING_INVENTORY_FINALIZE,
                (RootComponentFinalInventoryRequest {
                    operation_id,
                    component: binding.component,
                    expected_registry: empty.registry,
                },),
            )
            .expect("finalize empty Component inventory transport");
        let request = RootComponentDeletionRequest {
            operation_id,
            component: binding.component,
            expected_inventory_hash: inventory
                .expect("finalize empty Component inventory")
                .inventory
                .inventory_hash,
        };
        let deleted: Result<RootComponentDeletionResponse, Error> = fixture
            .pic()
            .update_candid(fixture.root, CANIC_ROOT_COMPONENT_DELETE, (request,))
            .expect("delete empty Component transport");
        assert!(matches!(
            deleted.expect("delete empty Component").phase,
            RootComponentDeletionPhase::Deleted(_)
        ));
        let removed: Result<RootComponentDeletionResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_MEMBERSHIP_REMOVE,
                (request,),
            )
            .expect("remove empty Component membership transport");
        removed.expect("remove empty Component membership")
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

    #[cfg(test)]
    fn setup_prepared_grouped_provisioning() -> PreparedGroupedProvisioningFixture {
        let root_wasm = build_test_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let store_fixture = build_root_store_fixture();
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let root = install_bootstrapped_root(&pic, root_wasm, coordinator, store_fixture);
        install_fixture_coordinator(&pic, coordinator, coordinator_wasm, &root);
        let (joining_version, sync_request) = join_and_synchronize_root(&pic, coordinator, &root);
        let component_registry = activate_registry_and_prepare_component_registry(
            &pic,
            coordinator,
            &root,
            joining_version,
            sync_request,
        );
        let registry: Result<FleetRegistry, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query grouped provisioning Fleet Registry transport");
        let registry = registry.expect("query grouped provisioning Fleet Registry");
        let request = grouped_projects_provisioning_request(
            &root,
            &registry,
            component_registry.expected_fleet_registry,
        );
        PreparedGroupedProvisioningFixture {
            pic,
            coordinator,
            root,
            request,
        }
    }

    #[cfg(test)]
    fn setup_active_cross_root_peer() -> ActiveCrossRootPeerFixture {
        let root_wasm = build_test_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let requester_store_fixture = build_root_store_fixture();
        let target_store_fixture = build_root_store_fixture();
        let pic = build_two_application_subnet_pic();
        let mut application_subnets = pic.topology().get_app_subnets();
        application_subnets.sort();
        let [requester_subnet, target_subnet] = application_subnets.as_slice() else {
            panic!("cross-root qualification requires exactly two application Subnets")
        };
        let coordinator = pic.create_canister_on_subnet(None, None, *requester_subnet);
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let requester_root = install_bootstrapped_root_on_subnet(
            &pic,
            root_wasm.clone(),
            coordinator,
            requester_store_fixture,
            *requester_subnet,
        );
        let target_root = install_bootstrapped_root_on_subnet(
            &pic,
            root_wasm,
            coordinator,
            target_store_fixture,
            *target_subnet,
        );
        assert_root_local_physical_inventory(&pic, &requester_root);
        assert_root_local_physical_inventory(&pic, &target_root);
        install_fixture_coordinator(&pic, coordinator, coordinator_wasm, &requester_root);
        let roots = [&requester_root, &target_root];
        let (joining_version, sync_requests) =
            join_and_synchronize_roots(&pic, coordinator, &roots);
        let initial_registry = activate_registry_and_prepare_component_registries(
            &pic,
            coordinator,
            &roots,
            joining_version,
            &sync_requests,
        );
        let (requester, service_registry) = provision_cross_root_service_topology(
            &pic,
            coordinator,
            &requester_root,
            &target_root,
            initial_registry.clone(),
        );
        ActiveCrossRootPeerFixture {
            pic,
            coordinator,
            requester_root,
            target_root,
            requester,
            initial_registry,
            service_registry,
        }
    }

    #[cfg(test)]
    fn setup_toko_topology_qualification() -> TokoTopologyFixture {
        let root_wasm = build_test_toko_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let store_wasm = build_test_wasm_store_wasm();
        let components = build_toko_component_wasms();
        let wasm_footprints =
            toko_wasm_footprints(&root_wasm, &coordinator_wasm, &store_wasm, components);
        let pic = build_three_application_subnet_pic();
        let mut application_subnets = pic.topology().get_app_subnets();
        application_subnets.sort();
        let [
            authority_subnet,
            first_project_subnet,
            second_project_subnet,
        ] = application_subnets.as_slice()
        else {
            panic!("Toko qualification requires exactly three application Subnets")
        };
        let coordinator = pic.create_canister_on_subnet(None, None, *authority_subnet);
        let second_coordinator = pic.create_canister_on_subnet(None, None, *first_project_subnet);
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        pic.add_cycles(second_coordinator, COORDINATOR_INSTALL_CYCLES);

        let roots = vec![
            install_qualification_root_on_subnet(
                &pic,
                root_wasm.clone(),
                coordinator,
                build_toko_root_store_fixture(),
                *authority_subnet,
                qualification_admissions(1, 1),
            ),
            install_qualification_root_on_subnet(
                &pic,
                root_wasm.clone(),
                coordinator,
                build_toko_root_store_fixture(),
                *first_project_subnet,
                qualification_admissions(1, 10),
            ),
            install_qualification_root_on_subnet(
                &pic,
                root_wasm.clone(),
                coordinator,
                build_toko_root_store_fixture(),
                *second_project_subnet,
                qualification_admissions(1, 4),
            ),
        ];
        let second_root = install_qualification_root_for_fleet_on_subnet(
            &pic,
            root_wasm,
            second_coordinator,
            build_toko_root_store_fixture(),
            *first_project_subnet,
            0xf2,
        );
        install_fixture_coordinator(&pic, coordinator, coordinator_wasm.clone(), &roots[0]);
        install_fixture_coordinator(&pic, second_coordinator, coordinator_wasm, &second_root);

        let root_refs = roots.iter().collect::<Vec<_>>();
        let (joining_version, sync_requests) =
            join_and_synchronize_roots(&pic, coordinator, &root_refs);
        let initial_registry = activate_registry_and_prepare_component_registries(
            &pic,
            coordinator,
            &root_refs,
            joining_version,
            &sync_requests,
        );
        let (second_joining, second_sync) =
            join_and_synchronize_root(&pic, second_coordinator, &second_root);
        let _ = activate_registry_and_prepare_component_registry(
            &pic,
            second_coordinator,
            &second_root,
            second_joining,
            second_sync,
        );

        TokoTopologyFixture {
            pic,
            coordinator,
            roots,
            initial_registry,
            second_coordinator,
            second_root,
            wasm_footprints,
        }
    }

    #[cfg(test)]
    fn toko_wasm_footprints(
        root_wasm: &[u8],
        coordinator_wasm: &[u8],
        store_wasm: &[u8],
        components: &BTreeMap<CanisterRole, Vec<u8>>,
    ) -> BTreeMap<&'static str, (usize, usize)> {
        BTreeMap::from([
            ("coordinator", wasm_footprint(coordinator_wasm)),
            ("fleet_subnet_root", wasm_footprint(root_wasm)),
            ("wasm_store", wasm_footprint(store_wasm)),
            (
                "database_a",
                wasm_footprint(component_fixture_wasm(components, "database_a")),
            ),
            (
                "project_hub",
                wasm_footprint(component_fixture_wasm(components, "project_hub")),
            ),
            (
                "project_instance",
                wasm_footprint(component_fixture_wasm(components, "project_instance")),
            ),
            (
                "project_ledger",
                wasm_footprint(component_fixture_wasm(components, "project_ledger")),
            ),
            (
                "project_machine",
                wasm_footprint(component_fixture_wasm(components, "project_machine")),
            ),
        ])
    }

    #[cfg(test)]
    fn wasm_footprint(wasm: &[u8]) -> (usize, usize) {
        (wasm.len(), gzip(wasm).len())
    }

    #[cfg(test)]
    fn component_fixture_wasm<'a>(
        wasms: &'a BTreeMap<CanisterRole, Vec<u8>>,
        role: &'static str,
    ) -> &'a [u8] {
        wasms
            .get(&CanisterRole::new(role))
            .unwrap_or_else(|| panic!("missing fixture Wasm for role {role}"))
    }

    #[cfg(test)]
    fn qualification_admissions(
        database_instances: u32,
        project_instances: u32,
    ) -> BTreeMap<ComponentSpecId, u32> {
        let component_spec = |value: &str| value.parse().expect("qualification Component Spec ID");
        BTreeMap::from([
            (component_spec("database_a"), database_instances),
            (component_spec("database_b"), database_instances),
            (component_spec("database_c"), database_instances),
            (component_spec("issuer"), 1),
            (component_spec("projects"), project_instances),
        ])
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

    #[cfg(test)]
    fn provision_cross_root_service_topology(
        pic: &PocketIc,
        coordinator: Principal,
        requester_root: &BootstrappedRootFixture,
        target_root: &BootstrappedRootFixture,
        initial_registry: FleetRegistryVersion,
    ) -> (ComponentBinding, FleetRegistryVersion) {
        let registry: Result<FleetRegistry, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query pre-service Registry transport");
        let registry = registry.expect("query pre-service Registry");
        assert_eq!(registry.services, vec![]);
        let (plan, plan_hash) = cross_root_projects_provisioning_plan(
            requester_root,
            target_root,
            &registry,
            initial_registry,
        );
        let operation_id = [0xd4; 32];
        let prepared: Result<FleetComponentProvisioningStatusResponse, Error> = pic
            .update_candid(
                coordinator,
                CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
                (FleetComponentProvisioningPrepareRequest { operation_id, plan },),
            )
            .expect("prepare cross-root service topology transport");
        let prepared = prepared.expect("prepare cross-root service topology");
        assert_eq!(prepared.plan_hash, plan_hash);
        let activated = drive_coordinator_provisioning(
            pic,
            coordinator,
            prepared,
            FleetComponentProvisioningPhase::RuntimesActivated,
        );
        assert_eq!(activated.accepted_root_count, 2);
        assert_eq!(activated.provisioned_root_count, 2);
        assert_eq!(activated.directory_confirmed_root_count, 2);
        assert_eq!(activated.runtime_activated_root_count, 2);
        let service_registry = activated
            .published_fleet_registry
            .expect("cross-root service Registry publication");
        let status: Result<RootComponentProvisioningStatusResponse, Error> = pic
            .query_candid_as(
                requester_root.root_id,
                coordinator,
                CANIC_ROOT_COMPONENT_PROVISIONING_STATUS,
                (RootComponentProvisioningStatusRequest {
                    operation_id,
                    plan_hash,
                },),
            )
            .expect("query cross-root requester provisioning status transport");
        let status = status.expect("query cross-root requester provisioning status");
        let result = status
            .result
            .expect("cross-root requester provisioning result");
        let [placement] = result.placements.as_slice() else {
            panic!("cross-root service plan must materialize one requester placement")
        };
        let [member] = placement.members.as_slice() else {
            panic!("cross-root service placement must materialize one requester member")
        };
        assert_eq!(member.binding.component_spec.as_str(), "projects");
        (member.binding.clone(), service_registry)
    }

    #[cfg(test)]
    fn cross_root_projects_provisioning_plan(
        requester_root: &BootstrappedRootFixture,
        target_root: &BootstrappedRootFixture,
        registry: &FleetRegistry,
        fleet_registry: FleetRegistryVersion,
    ) -> (FleetComponentProvisioningPlan, [u8; 32]) {
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config = AppConfigSnapshot::load(&root_canister_config_path(&workspace_root))
            .expect("load cross-root Component configuration");
        let deployments = config
            .model()
            .compile_component_group_deployment_topology()
            .expect("compile cross-root deployment topology");
        let deployment = deployments
            .get(
                &"grouped_projects"
                    .parse()
                    .expect("grouped projects deployment ID"),
            )
            .expect("grouped projects deployment");
        let entries = deployment
            .members
            .iter()
            .map(|member| ComponentGroupPlanEntry {
                member_path: member.member_path.clone(),
                component_spec: member.component_spec.clone(),
                spec_hash: member.component_spec_hash,
                purpose: member.purpose.clone(),
                labels: member.labels.clone(),
                limits: member.limits.clone(),
            })
            .collect();
        let requester_batch = FleetSubnetRootProvisioningBatch {
            root: requester_root.init_args.authority.binding.clone(),
            active_release_set: requester_root.init_args.authority.initial_release_set,
            placements: vec![ComponentGroupPlacementPlan {
                group_placement: ComponentGroupPlacementId {
                    deployment: deployment.deployment.clone(),
                    ordinal: 0,
                },
                component_group: deployment.component_group.clone(),
                entries,
            }],
        };
        let target_batch = FleetSubnetRootProvisioningBatch {
            root: target_root.init_args.authority.binding.clone(),
            active_release_set: target_root.init_args.authority.initial_release_set,
            placements: vec![],
        };
        let mut batches = vec![requester_batch, target_batch];
        batches.sort_by_key(|batch| batch.root.fleet_subnet_root);
        let mut directory_confirmation_roots = vec![requester_root.root_id, target_root.root_id];
        directory_confirmation_roots.sort();
        let plan = FleetComponentProvisioningPlan {
            fleet: registry.authority.binding.fleet.clone(),
            fleet_registry,
            configuration_digest: config
                .model()
                .compile_component_deployment_configuration_digest()
                .expect("compile cross-root deployment configuration digest"),
            operation: FleetComponentProvisioningOperation::FreshInstall,
            directory_confirmation_roots,
            batches,
        };
        let plan_hash = ComponentProvisioningPlanOps::hash(config.model(), registry, &plan)
            .expect("hash cross-root provisioning plan");
        (plan, plan_hash)
    }

    #[cfg(test)]
    fn grouped_projects_provisioning_request(
        root: &BootstrappedRootFixture,
        registry: &FleetRegistry,
        fleet_registry: canic::dto::fleet_registry::FleetRegistryVersion,
    ) -> RootComponentProvisioningAcceptanceRequest {
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = root_canister_config_path(&workspace_root);
        let config = AppConfigSnapshot::load(&config_path).expect("load grouped root config");
        let deployments = config
            .model()
            .compile_component_group_deployment_topology()
            .expect("compile grouped deployment topology");
        let deployment = deployments
            .get(
                &"grouped_projects"
                    .parse()
                    .expect("grouped projects deployment ID"),
            )
            .expect("grouped projects deployment");
        let entries = deployment
            .members
            .iter()
            .map(|member| ComponentGroupPlanEntry {
                member_path: member.member_path.clone(),
                component_spec: member.component_spec.clone(),
                spec_hash: member.component_spec_hash,
                purpose: member.purpose.clone(),
                labels: member.labels.clone(),
                limits: member.limits.clone(),
            })
            .collect::<Vec<_>>();
        let batch = FleetSubnetRootProvisioningBatch {
            root: root.init_args.authority.binding.clone(),
            active_release_set: root.init_args.authority.initial_release_set,
            placements: vec![ComponentGroupPlacementPlan {
                group_placement: ComponentGroupPlacementId {
                    deployment: deployment.deployment.clone(),
                    ordinal: 0,
                },
                component_group: deployment.component_group.clone(),
                entries,
            }],
        };
        let configuration_digest = config
            .model()
            .compile_component_deployment_configuration_digest()
            .expect("compile grouped configuration digest");
        let plan = FleetComponentProvisioningPlan {
            fleet: registry.authority.binding.fleet.clone(),
            fleet_registry: fleet_registry.clone(),
            configuration_digest,
            operation: FleetComponentProvisioningOperation::FreshInstall,
            directory_confirmation_roots: vec![root.root_id],
            batches: vec![batch.clone()],
        };
        let plan_hash = ComponentProvisioningPlanOps::hash(config.model(), registry, &plan)
            .expect("hash exact grouped provisioning plan");
        RootComponentProvisioningAcceptanceRequest {
            fleet_registry,
            configuration_digest,
            operation_id: [0xd1; 32],
            plan_hash,
            batch,
        }
    }

    #[cfg(test)]
    fn toko_initial_provisioning_plan(
        fixture: &TokoTopologyFixture,
        registry: &FleetRegistry,
    ) -> FleetComponentProvisioningPlan {
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config = AppConfigSnapshot::load(&toko_root_canister_config_path(&workspace_root))
            .expect("load Toko provisioning configuration");
        let deployments = config
            .model()
            .compile_component_group_deployment_topology()
            .expect("compile Toko provisioning deployments");
        let deployment = |name: &str| {
            let id = name.parse().expect("Toko deployment ID");
            deployments.get(&id).expect("Toko deployment")
        };
        let authority = planned_group_placement(deployment("authoritative_databases"), 0);
        let mut first_project_placements = vec![
            planned_group_placement(deployment("packed_projects"), 0),
            planned_group_placement(deployment("packed_projects"), 1),
            planned_group_placement(deployment("project_data_cells"), 0),
        ];
        first_project_placements.sort_by(|left, right| {
            left.group_placement
                .deployment
                .cmp(&right.group_placement.deployment)
                .then(
                    left.group_placement
                        .ordinal
                        .cmp(&right.group_placement.ordinal),
                )
        });
        let mut batches = vec![
            root_provisioning_batch(&fixture.roots[0], vec![authority]),
            root_provisioning_batch(&fixture.roots[1], first_project_placements),
            root_provisioning_batch(
                &fixture.roots[2],
                vec![planned_group_placement(deployment("grouped_projects"), 0)],
            ),
        ];
        batches.sort_by_key(|batch| batch.root.fleet_subnet_root);
        let mut directory_confirmation_roots = fixture
            .roots
            .iter()
            .map(|root| root.root_id)
            .collect::<Vec<_>>();
        directory_confirmation_roots.sort();

        FleetComponentProvisioningPlan {
            fleet: registry.authority.binding.fleet.clone(),
            fleet_registry: fixture.initial_registry.clone(),
            configuration_digest: config
                .model()
                .compile_component_deployment_configuration_digest()
                .expect("compile Toko configuration digest"),
            operation: FleetComponentProvisioningOperation::FreshInstall,
            directory_confirmation_roots,
            batches,
        }
    }

    #[cfg(test)]
    fn planned_group_placement(
        deployment: &ComponentGroupDeploymentSpec,
        ordinal: u32,
    ) -> ComponentGroupPlacementPlan {
        let entries = deployment
            .members
            .iter()
            .map(|member| ComponentGroupPlanEntry {
                member_path: member.member_path.clone(),
                component_spec: member.component_spec.clone(),
                spec_hash: member.component_spec_hash,
                purpose: member.purpose.clone(),
                labels: member.labels.clone(),
                limits: member.limits.clone(),
            })
            .collect();
        ComponentGroupPlacementPlan {
            group_placement: ComponentGroupPlacementId {
                deployment: deployment.deployment.clone(),
                ordinal,
            },
            component_group: deployment.component_group.clone(),
            entries,
        }
    }

    #[cfg(test)]
    fn root_provisioning_batch(
        root: &BootstrappedRootFixture,
        placements: Vec<ComponentGroupPlacementPlan>,
    ) -> FleetSubnetRootProvisioningBatch {
        FleetSubnetRootProvisioningBatch {
            root: root.init_args.authority.binding.clone(),
            active_release_set: root.init_args.authority.initial_release_set,
            placements,
        }
    }

    #[cfg(test)]
    fn prepare_fleet_provisioning(
        pic: &PocketIc,
        coordinator: Principal,
        operation_id: [u8; 32],
        plan: FleetComponentProvisioningPlan,
    ) -> FleetComponentProvisioningStatusResponse {
        let prepared: Result<FleetComponentProvisioningStatusResponse, Error> = pic
            .update_candid(
                coordinator,
                CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
                (FleetComponentProvisioningPrepareRequest { operation_id, plan },),
            )
            .expect("prepare Fleet provisioning transport");
        let prepared = prepared.expect("prepare Fleet provisioning");
        assert_eq!(prepared.phase, FleetComponentProvisioningPhase::Planned);
        prepared
    }

    #[cfg(test)]
    fn prepare_fleet_provisioning_request(
        pic: &PocketIc,
        coordinator: Principal,
        request: &FleetComponentProvisioningPrepareRequest,
    ) -> FleetComponentProvisioningStatusResponse {
        let prepared: Result<FleetComponentProvisioningStatusResponse, Error> = pic
            .update_candid(
                coordinator,
                CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
                (request.clone(),),
            )
            .expect("prepare Fleet scale-out transport");
        prepared.expect("prepare Fleet scale-out")
    }

    #[cfg(test)]
    fn toko_scale_out_plan(
        fixture: &TokoTopologyFixture,
        registry: &FleetRegistry,
        deployment_name: &str,
        previous_placements: u32,
        requested_placements: u32,
        ordinal: u32,
        target_root: usize,
    ) -> FleetComponentProvisioningPlan {
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config = AppConfigSnapshot::load(&toko_root_canister_config_path(&workspace_root))
            .expect("load Toko scale-out configuration");
        let deployments = config
            .model()
            .compile_component_group_deployment_topology()
            .expect("compile Toko scale-out deployments");
        let deployment_id = deployment_name
            .parse()
            .expect("Toko scale-out deployment ID");
        let deployment = deployments
            .get(&deployment_id)
            .expect("Toko scale-out deployment");
        let root = &fixture.roots[target_root];
        let batch =
            root_provisioning_batch(root, vec![planned_group_placement(deployment, ordinal)]);
        let mut confirmation_roots = std::collections::BTreeSet::from([root.root_id]);
        for member in &deployment.members {
            let ComponentDeploymentPurpose::FleetServiceMember { service, .. } = &member.purpose
            else {
                continue;
            };
            let existing = fleet_service(registry, service.as_str());
            confirmation_roots.extend(
                existing
                    .members
                    .iter()
                    .map(|binding| binding.fleet_subnet_root),
            );
        }

        FleetComponentProvisioningPlan {
            fleet: registry.authority.binding.fleet.clone(),
            fleet_registry: fleet_registry_version(&fixture.pic, fixture.coordinator),
            configuration_digest: config
                .model()
                .compile_component_deployment_configuration_digest()
                .expect("compile Toko scale-out configuration digest"),
            operation: FleetComponentProvisioningOperation::ScaleOut {
                deployment: deployment.deployment.clone(),
                previous_placements,
                requested_placements,
            },
            directory_confirmation_roots: confirmation_roots.into_iter().collect(),
            batches: vec![batch],
        }
    }

    #[cfg(test)]
    fn query_fleet_registry(pic: &PocketIc, coordinator: Principal) -> FleetRegistry {
        let registry: Result<FleetRegistry, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query Fleet Registry transport");
        registry.expect("query Fleet Registry")
    }

    #[cfg(test)]
    fn assert_toko_initial_service_topology(
        fixture: &TokoTopologyFixture,
        registry: &FleetRegistry,
    ) {
        for database in ["database_a", "database_b", "database_c"] {
            let service = fleet_service(registry, database);
            assert_eq!(service.mode, FleetServiceMode::AuthorityReplica);
            assert_eq!(service.members.len(), 2);
            assert!(service.members.iter().any(|member| {
                member.member_purpose == FleetServiceMemberPurpose::Authority
                    && member.fleet_subnet_root == fixture.roots[0].root_id
            }));
            assert!(service.members.iter().any(|member| {
                member.member_purpose == FleetServiceMemberPurpose::Replica
                    && member.fleet_subnet_root == fixture.roots[1].root_id
            }));
        }
        let project_hubs = fleet_service(registry, "project_hubs");
        assert_eq!(project_hubs.mode, FleetServiceMode::ActivePool);
        assert_eq!(project_hubs.members.len(), 1);
        assert_eq!(
            project_hubs.members[0].fleet_subnet_root,
            fixture.roots[1].root_id
        );
        let projects = fleet_service(registry, "projects");
        assert_eq!(projects.mode, FleetServiceMode::ActivePool);
        assert_eq!(projects.members.len(), 3);
        assert_eq!(
            projects
                .members
                .iter()
                .filter(|member| member.fleet_subnet_root == fixture.roots[1].root_id)
                .count(),
            2
        );
        assert!(
            projects
                .members
                .iter()
                .any(|member| member.fleet_subnet_root == fixture.roots[2].root_id)
        );
        for service in &registry.services {
            for member in &service.members {
                assert_eq!(
                    fixture.pic.get_subnet(member.canister_id),
                    fixture.pic.get_subnet(member.fleet_subnet_root)
                );
            }
        }
    }

    #[cfg(test)]
    fn assert_terminal_scale_out(
        status: &FleetComponentProvisioningStatusResponse,
        expected_confirmation_roots: u32,
    ) {
        assert_eq!(
            status.phase,
            FleetComponentProvisioningPhase::RuntimesActivated
        );
        assert_eq!(status.accepted_root_count, 1);
        assert_eq!(status.provisioned_root_count, 1);
        assert_eq!(
            status.directory_confirmed_root_count,
            expected_confirmation_roots
        );
        assert_eq!(status.runtime_activated_root_count, 1);
        assert!(status.published_fleet_registry.is_some());
    }

    #[cfg(test)]
    fn assert_toko_scaled_service_topology(
        fixture: &TokoTopologyFixture,
        registry: &FleetRegistry,
    ) {
        for database in ["database_a", "database_b", "database_c"] {
            let service = fleet_service(registry, database);
            assert_eq!(service.members.len(), 3);
            for (root_index, purpose) in [
                (0, FleetServiceMemberPurpose::Authority),
                (1, FleetServiceMemberPurpose::Replica),
                (2, FleetServiceMemberPurpose::Replica),
            ] {
                assert!(service.members.iter().any(|member| {
                    member.fleet_subnet_root == fixture.roots[root_index].root_id
                        && member.member_purpose == purpose
                }));
            }
        }

        let project_hubs = fleet_service(registry, "project_hubs");
        assert_eq!(project_hubs.members.len(), 2);
        assert_eq!(
            project_hubs
                .members
                .iter()
                .map(|member| member.fleet_subnet_root)
                .collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from([fixture.roots[1].root_id, fixture.roots[2].root_id,])
        );

        let projects = fleet_service(registry, "projects");
        assert_eq!(projects.members.len(), 4);
        for root in [&fixture.roots[1], &fixture.roots[2]] {
            assert_eq!(
                projects
                    .members
                    .iter()
                    .filter(|member| member.fleet_subnet_root == root.root_id)
                    .count(),
                2
            );
        }
        assert_project_service_runtime_limits(fixture, projects);
        for service in &registry.services {
            for member in &service.members {
                assert_eq!(
                    fixture.pic.get_subnet(member.canister_id),
                    fixture.pic.get_subnet(member.fleet_subnet_root)
                );
            }
        }
    }

    #[cfg(test)]
    fn assert_project_service_runtime_limits(
        fixture: &TokoTopologyFixture,
        service: &canic::dto::fleet_registry::FleetServiceBinding,
    ) {
        let mut deployment_counts = BTreeMap::<String, usize>::new();
        for member in &service.members {
            let runtime = active_service_runtime(&fixture.pic, member);
            let ProtectedComponentDeployment::GroupMember {
                group_placement,
                limits,
                ..
            } = runtime.deployment.as_ref()
            else {
                panic!("Project Hub service member must retain grouped deployment authority")
            };
            *deployment_counts
                .entry(group_placement.deployment.to_string())
                .or_default() += 1;
            match group_placement.deployment.as_str() {
                "grouped_projects" => assert!(limits.spawn_grant_reductions.is_empty()),
                "packed_projects" => {
                    assert_eq!(limits.spawn_grant_reductions.len(), 1);
                    assert_eq!(
                        limits.spawn_grant_reductions[0].maximum_instances_per_parent,
                        2_000
                    );
                }
                deployment => panic!("unexpected Project Hub deployment {deployment}"),
            }
        }
        assert_eq!(deployment_counts.get("grouped_projects"), Some(&1));
        assert_eq!(deployment_counts.get("packed_projects"), Some(&3));
    }

    #[cfg(test)]
    fn provision_toko_project_trees(fixture: &TokoTopologyFixture, registry: &FleetRegistry) {
        let registry_before = fleet_registry_version(&fixture.pic, fixture.coordinator);
        let service = fleet_service(registry, "project_hubs");
        let mut hubs = service
            .members
            .iter()
            .map(|member| active_service_component(&fixture.pic, member))
            .collect::<Vec<_>>();
        hubs.sort_by_key(|hub| hub.fleet_subnet_root);
        let [first_hub, second_hub] = hubs.as_slice() else {
            panic!("Toko qualification requires two project-data-cell Hubs")
        };
        let first_tree = provision_qualified_project_tree(
            &fixture.pic,
            first_hub,
            &[
                ("qualification-project-alpha", [0xf4; 32]),
                ("qualification-project-beta", [0xf5; 32]),
            ],
            None,
        );
        let second_tree = provision_qualified_project_tree(
            &fixture.pic,
            second_hub,
            &[("qualification-project-gamma", [0xf6; 32])],
            Some([0xf7; 32]),
        );
        let first_entries = assert_qualified_project_tree(&fixture.pic, &first_tree);
        let second_entries = assert_qualified_project_tree(&fixture.pic, &second_tree);
        assert_eq!(
            first_entries
                .iter()
                .chain(&second_entries)
                .filter(|entry| entry.binding.role.as_str() == "project_machine")
                .count(),
            1
        );
        assert_eq!(
            fleet_registry_version(&fixture.pic, fixture.coordinator),
            registry_before,
            "dynamic descendants must not mutate Fleet Registry topology"
        );
    }

    #[cfg(test)]
    struct QualifiedProjectTree {
        hub: ComponentBinding,
        instances: Vec<Principal>,
        ledgers: Vec<Principal>,
        machine: Option<Principal>,
    }

    #[cfg(test)]
    fn provision_qualified_project_tree(
        pic: &PocketIc,
        hub: &ComponentBinding,
        projects: &[(&str, [u8; 32])],
        machine_operation_id: Option<[u8; 32]>,
    ) -> QualifiedProjectTree {
        let instances = projects
            .iter()
            .map(|(project, _)| resolve_project_instance(pic, hub.canister_id, project))
            .collect::<Vec<_>>();
        assert_eq!(
            resolve_project_instance(pic, hub.canister_id, projects[0].0),
            instances[0]
        );
        let ledgers = instances
            .iter()
            .zip(projects)
            .map(|(instance, (_, operation_id))| {
                create_project_descendant(pic, *instance, "create_project_ledger", *operation_id)
                    .expect("Project Instance creates its Ledger")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            create_project_descendant(pic, instances[0], "create_project_ledger", projects[0].1,)
                .expect("Ledger exact retry"),
            ledgers[0]
        );
        let machine = machine_operation_id.map(|operation_id| {
            create_project_descendant(pic, instances[0], "create_project_machine", operation_id)
                .expect("one Project Instance creates its optional Machine")
        });
        QualifiedProjectTree {
            hub: hub.clone(),
            instances,
            ledgers,
            machine,
        }
    }

    #[cfg(test)]
    fn assert_qualified_project_tree(
        pic: &PocketIc,
        tree: &QualifiedProjectTree,
    ) -> Vec<ComponentDirectoryChildEntry> {
        let entries = project_directory_entries(pic, tree.hub.fleet_subnet_root, &tree.hub);
        for instance in &tree.instances {
            assert_project_child(
                &entries,
                *instance,
                tree.hub.canister_id,
                "project_instance",
                &tree.hub,
            );
        }
        for (instance, ledger) in tree.instances.iter().zip(&tree.ledgers) {
            assert_project_child(&entries, *ledger, *instance, "project_ledger", &tree.hub);
        }
        if let Some(machine) = tree.machine {
            assert_project_child(
                &entries,
                machine,
                tree.instances[0],
                "project_machine",
                &tree.hub,
            );
        }
        let mut descendants = tree.instances.clone();
        descendants.extend(&tree.ledgers);
        descendants.extend(tree.machine);
        assert_project_tree_subnet(pic, tree.hub.fleet_subnet_root, &descendants);
        entries
    }

    #[cfg(test)]
    fn active_service_component(
        pic: &PocketIc,
        member: &canic::dto::fleet_registry::FleetServiceComponentBinding,
    ) -> ComponentBinding {
        let runtime = active_service_runtime(pic, member);
        let ManagedCanisterBinding::Component(binding) = runtime.binding else {
            panic!("Fleet-service member must remain a top-level Component")
        };
        assert_eq!(binding.component, member.component);
        binding
    }

    #[cfg(test)]
    fn active_service_runtime(
        pic: &PocketIc,
        member: &canic::dto::fleet_registry::FleetServiceComponentBinding,
    ) -> ComponentRuntimeStatusResponse {
        let response: Result<ComponentRuntimeStatusResponse, Error> = pic
            .query_candid_as(
                member.canister_id,
                member.fleet_subnet_root,
                CANIC_COMPONENT_RUNTIME_STATUS,
                (),
            )
            .expect("query active Fleet-service member transport");
        let runtime = response.expect("query active Fleet-service member");
        assert_eq!(runtime.phase, ComponentRuntimePhase::Active);
        runtime
    }

    #[cfg(test)]
    fn report_toko_qualification_metrics(
        fixture: &TokoTopologyFixture,
        config: &canic_core::bootstrap::compiled::ConfigModel,
        registry: &FleetRegistry,
        initial_plan_bytes: usize,
    ) {
        let topology = config
            .compile_component_topology()
            .expect("compile Toko qualification Component topology");
        let registry_bytes =
            FleetRegistryOps::canonical_bytes(&registry.authority, &topology, registry)
                .expect("encode final Toko Fleet Registry")
                .len();
        let maximum_directory_bytes = fixture
            .roots
            .iter()
            .map(|root| {
                let directory = FleetRegistryOps::directory_for_root(
                    &registry.authority,
                    &topology,
                    registry,
                    root.root_id,
                )
                .expect("derive Toko Fleet Directory");
                encode_one(directory)
                    .expect("encode Toko Fleet Directory")
                    .len()
            })
            .max()
            .expect("Toko qualification has roots");
        let service_members = registry
            .services
            .iter()
            .map(|service| service.members.len())
            .sum::<usize>();
        eprintln!(
            "Toko qualification envelope: roots={} deployments=4 placements=7 top_level_components=15 services={} service_members={} initial_plan_bytes={} final_registry_bytes={} maximum_directory_candid_bytes={}",
            registry.fleet_subnet_roots.len(),
            registry.services.len(),
            service_members,
            initial_plan_bytes,
            registry_bytes,
            maximum_directory_bytes,
        );
        for (role, (raw_bytes, gzip_bytes)) in &fixture.wasm_footprints {
            eprintln!(
                "Toko qualification Wasm: role={role} raw_bytes={raw_bytes} gzip_bytes={gzip_bytes}"
            );
        }
    }

    #[cfg(test)]
    fn fleet_service<'a>(
        registry: &'a FleetRegistry,
        service: &str,
    ) -> &'a canic::dto::fleet_registry::FleetServiceBinding {
        registry
            .services
            .iter()
            .find(|binding| binding.service.as_str() == service)
            .unwrap_or_else(|| panic!("missing Fleet service {service}"))
    }

    #[cfg(test)]
    const fn advance_request(
        status: &RootComponentProvisioningStatusResponse,
    ) -> RootComponentProvisioningAdvanceRequest {
        RootComponentProvisioningAdvanceRequest {
            operation_id: status.operation_id,
            plan_hash: status.plan_hash,
            expected_reserved_component_count: status.reserved_component_count,
            expected_claimed_component_count: status.claimed_component_count,
            expected_installed_component_count: status.installed_component_count,
            expected_registry_committed_component_count: status.registry_committed_component_count,
        }
    }

    #[cfg(test)]
    fn advance_grouped_provisioning(
        fixture: &PreparedGroupedProvisioningFixture,
        request: RootComponentProvisioningAdvanceRequest,
    ) -> RootComponentProvisioningStatusResponse {
        let response: Result<RootComponentProvisioningStatusResponse, Error> = fixture
            .pic
            .update_candid_as(
                fixture.root.root_id,
                fixture.coordinator,
                CANIC_ROOT_COMPONENT_PROVISIONING_ADVANCE,
                (request,),
            )
            .expect("advance grouped provisioning transport");
        response.expect("advance grouped provisioning")
    }

    #[cfg(test)]
    fn assert_grouped_provisioning_progress(
        status: &RootComponentProvisioningStatusResponse,
        reserved: u32,
        claimed: u32,
        installed: u32,
        registry_committed: u32,
    ) {
        assert_eq!(status.phase, RootComponentProvisioningPhase::Accepted);
        assert_eq!(status.placement_count, 1);
        assert_eq!(status.component_count, 1);
        assert_eq!(status.reserved_component_count, reserved);
        assert_eq!(status.claimed_component_count, claimed);
        assert_eq!(status.installed_component_count, installed);
        assert_eq!(
            status.registry_committed_component_count,
            registry_committed
        );
        assert_ne!(status.receipt_content_hash, [0; 32]);
    }

    #[cfg(test)]
    fn one_grouped_workload(
        fixture: &PreparedGroupedProvisioningFixture,
    ) -> (Principal, CanisterPoolClaim) {
        let status = root_pool_status(&fixture.pic, fixture.root.root_id);
        assert_eq!(status.ready, 9);
        assert_eq!(status.workload, 1);
        let workloads = status
            .entries
            .into_iter()
            .filter_map(|asset| match asset.status {
                CanisterPoolAssetStatus::Workload { claim } => Some((asset.canister_id, claim)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let [workload] = workloads.as_slice() else {
            panic!("grouped provisioning must own one exact workload")
        };
        workload.clone()
    }

    #[cfg(test)]
    fn assert_grouped_claim_replay(
        fixture: &PreparedGroupedProvisioningFixture,
        request: RootComponentProvisioningAdvanceRequest,
        expected: &RootComponentProvisioningStatusResponse,
        expected_canister_id: Principal,
        expected_claim: &CanisterPoolClaim,
    ) {
        let replayed = advance_grouped_provisioning(fixture, request);
        assert_eq!(&replayed, expected);
        let (canister_id, claim) = one_grouped_workload(fixture);
        assert_eq!(canister_id, expected_canister_id);
        assert_eq!(&claim, expected_claim);
    }

    #[cfg(test)]
    fn grouped_allocation_status(
        fixture: &PreparedGroupedProvisioningFixture,
        operation_id: [u8; 32],
    ) -> RootComponentAllocationResponse {
        let response: Result<RootComponentAllocationResponse, Error> = fixture
            .pic
            .query_candid(
                fixture.root.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
                (RootComponentAllocationStatusRequest { operation_id },),
            )
            .expect("query grouped Component allocation transport");
        response.expect("query grouped Component allocation")
    }

    #[cfg(test)]
    fn assert_grouped_member_install(
        fixture: &PreparedGroupedProvisioningFixture,
        canister_id: Principal,
        claim: &CanisterPoolClaim,
    ) {
        let allocation = grouped_allocation_status(fixture, claim.operation_id);
        assert_eq!(allocation.phase, RootComponentAllocationPhase::Verified);
        let creation = allocation
            .creation
            .as_ref()
            .expect("grouped creation evidence");
        let installation = allocation
            .installation
            .as_ref()
            .expect("grouped install evidence");
        assert_eq!(creation.canister, Some(canister_id));
        assert_eq!(installation.binding.component, claim.component);
        assert_eq!(installation.binding.canister_id, canister_id);

        let entry = &fixture.request.batch.placements[0].entries[0];
        let placement = &fixture.request.batch.placements[0];
        let expected_deployment = ProtectedComponentDeployment::GroupMember {
            binding: installation.binding.clone(),
            configuration_digest: fixture.request.configuration_digest,
            group_placement: placement.group_placement.clone(),
            component_group: placement.component_group.clone(),
            member_path: entry.member_path.clone(),
            purpose: entry.purpose.clone(),
            labels: entry.labels.clone(),
            limits: entry.limits.clone(),
        };
        let runtime: Result<ComponentRuntimeStatusResponse, Error> = fixture
            .pic
            .query_candid_as(
                canister_id,
                fixture.root.root_id,
                CANIC_COMPONENT_RUNTIME_STATUS,
                (),
            )
            .expect("query grouped Component runtime transport");
        let runtime = runtime.expect("query grouped Component runtime");
        assert_eq!(runtime.operation_id, claim.operation_id);
        assert_eq!(
            runtime.binding,
            ManagedCanisterBinding::Component(installation.binding.clone())
        );
        assert_eq!(runtime.deployment.as_ref(), &expected_deployment);
        assert_eq!(runtime.phase, ComponentRuntimePhase::AwaitingDirectory);
        assert_eq!(runtime.authority, None);
        assert_eq!(runtime.authority_hash, None);
        assert_eq!(runtime.direct_children_hash, None);
        assert_eq!(runtime.activation, None);

        let live = fixture
            .pic
            .canister_status(canister_id, Some(fixture.root.root_id))
            .expect("grouped Component Canister status");
        assert_eq!(live.settings.controllers, vec![fixture.root.root_id]);
        assert_eq!(live.module_hash, Some(creation.payload_hash.to_vec()));
        let partition: Result<ComponentRegistryPartitionResponse, Error> = fixture
            .pic
            .query_candid(
                fixture.root.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: claim.component,
                },),
            )
            .expect("query unpublished grouped Component partition transport");
        assert_eq!(
            partition
                .expect_err("grouped install must not publish Registry membership")
                .code(),
            canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
        );
    }

    #[cfg(test)]
    fn assert_grouped_member_registry_commit(
        fixture: &PreparedGroupedProvisioningFixture,
        claim: &CanisterPoolClaim,
    ) -> ComponentRegistryPartitionResponse {
        let allocation = grouped_allocation_status(fixture, claim.operation_id);
        assert_eq!(allocation.phase, RootComponentAllocationPhase::Committed);
        let installation = allocation
            .installation
            .as_ref()
            .expect("grouped Registry installation evidence");
        let partition: Result<ComponentRegistryPartitionResponse, Error> = fixture
            .pic
            .query_candid(
                fixture.root.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: claim.component,
                },),
            )
            .expect("query committed grouped Component partition transport");
        let partition = partition.expect("committed grouped Component partition");
        assert_eq!(partition.binding, installation.binding);
        assert_eq!(
            partition.provisioning_origin,
            allocation.provisioning_origin
        );
        assert_eq!(partition.release_set, allocation.release_set);
        assert_eq!(partition.status, ComponentLifecycleStatus::Prepared);
        assert_eq!(partition.reserved_descendants, 0);
        assert_eq!(partition.committed_descendants, 0);
        assert!(partition.head.revision > 0);
        assert_ne!(partition.head.content_hash, [0; 32]);

        let runtime: Result<ComponentRuntimeStatusResponse, Error> = fixture
            .pic
            .query_candid_as(
                installation.binding.canister_id,
                fixture.root.root_id,
                CANIC_COMPONENT_RUNTIME_STATUS,
                (),
            )
            .expect("query Registry-committed grouped runtime transport");
        let runtime = runtime.expect("query Registry-committed grouped runtime");
        assert_eq!(runtime.phase, ComponentRuntimePhase::AwaitingDirectory);
        assert_eq!(runtime.authority, None);
        assert_eq!(runtime.activation, None);
        partition
    }

    fn acquire_active_component_registry() -> ActiveComponentRegistryFixture {
        let (baseline, outcome) = active_component_registry_pool()
            .acquire()
            .expect("acquire active Component Registry baseline");
        let metadata = baseline.metadata().clone();
        eprintln!("[pic_fleet_registry] active baseline {outcome}");

        ActiveComponentRegistryFixture {
            runtime: ActiveComponentRegistryRuntime::Pooled(baseline),
            coordinator: metadata.coordinator,
            root: metadata.root,
            issuer: metadata.issuer,
            verifier: metadata.verifier,
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
        let root_wasm = build_test_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let store_fixture = build_root_store_fixture();
        let pic = build_pic();
        let coordinator = pic.create_canister();
        pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
        let fixture = install_bootstrapped_root(&pic, root_wasm, coordinator, store_fixture);
        install_fixture_coordinator(&pic, coordinator, coordinator_wasm, &fixture);
        let (joining_version, sync_request) =
            join_and_synchronize_root(&pic, coordinator, &fixture);

        let components = assert_registry_and_root_runtime_activation(
            &pic,
            coordinator,
            &fixture,
            joining_version,
            sync_request,
        );
        let fixture = ActiveComponentRegistryFixture {
            runtime: ActiveComponentRegistryRuntime::Fresh(Box::new(pic)),
            coordinator,
            root: fixture.root_id,
            issuer: components.issuer,
            verifier: components.verifier,
            store_bootstrap: fixture.request,
            wasm_store: fixture.response.wasm_store,
            pool_assets: fixture.init_args.canister_pool_imports,
        };
        assert_root_canister_summary(&fixture);
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
        let genesis: Result<canic::dto::fleet_registry::FleetRegistryVersion, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY_VERSION, ())
            .expect("query Registry genesis");
        let binding = &fixture.init_args.authority.binding;
        let join_request = FleetSubnetRootJoinRequest {
            expected_registry: genesis.expect("Registry genesis"),
            entry: FleetSubnetRootEntry {
                placement_subnet: binding.placement_subnet,
                fleet_subnet_root: fixture.root_id,
                component_admissions: binding.component_admissions.clone(),
                component_topology_digest: binding.component_topology_digest,
                active_release_set: fixture.init_args.authority.initial_release_set,
                limits: binding.limits.clone(),
                status: FleetSubnetRootStatus::Joining,
            },
        };
        let joined: Result<FleetSubnetRootJoinResponse, Error> = pic
            .update_candid(coordinator, CANIC_FLEET_SUBNET_ROOT_JOIN, (join_request,))
            .expect("join root transport");
        let joined = joined.expect("join root");
        let sync_request = FleetSubnetRootRegistrySyncRequest {
            expected_registry: joined.version.clone(),
            store_bootstrap: fixture.request.clone(),
        };
        let synchronized: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_SYNCHRONIZE,
                (sync_request.clone(),),
            )
            .expect("root Registry synchronization transport");
        let synchronized = synchronized.expect("root Registry synchronization");
        assert_eq!(synchronized.fleet_subnet_root, fixture.root_id);
        assert_eq!(synchronized.version, joined.version);

        let retried: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_SYNCHRONIZE,
                (sync_request.clone(),),
            )
            .expect("root Registry synchronization retry transport");
        assert_eq!(
            retried.expect("root Registry synchronization retry"),
            synchronized
        );
        let observed: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_SYNC_STATUS,
                (sync_request.clone(),),
            )
            .expect("root Registry synchronization status transport");
        assert_eq!(
            observed.expect("root Registry synchronization status"),
            synchronized
        );
        let acknowledgements: Result<Vec<FleetSubnetRootSnapshotAcknowledgement>, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS, ())
            .expect("query root acknowledgements");
        assert_eq!(
            acknowledgements.expect("root acknowledgements"),
            vec![synchronized.acknowledgement]
        );
        (joined.version, sync_request)
    }

    #[cfg(test)]
    fn join_and_synchronize_roots(
        pic: &PocketIc,
        coordinator: Principal,
        fixtures: &[&BootstrappedRootFixture],
    ) -> (
        FleetRegistryVersion,
        Vec<FleetSubnetRootRegistrySyncRequest>,
    ) {
        let current: Result<FleetRegistryVersion, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY_VERSION, ())
            .expect("query multi-root Registry genesis transport");
        let mut current = current.expect("query multi-root Registry genesis");
        for fixture in fixtures {
            let binding = &fixture.init_args.authority.binding;
            let joined: Result<FleetSubnetRootJoinResponse, Error> = pic
                .update_candid(
                    coordinator,
                    CANIC_FLEET_SUBNET_ROOT_JOIN,
                    (FleetSubnetRootJoinRequest {
                        expected_registry: current,
                        entry: FleetSubnetRootEntry {
                            placement_subnet: binding.placement_subnet,
                            fleet_subnet_root: fixture.root_id,
                            component_admissions: binding.component_admissions.clone(),
                            component_topology_digest: binding.component_topology_digest,
                            active_release_set: fixture.init_args.authority.initial_release_set,
                            limits: binding.limits.clone(),
                            status: FleetSubnetRootStatus::Joining,
                        },
                    },),
                )
                .expect("join one multi-root fixture transport");
            current = joined.expect("join one multi-root fixture").version;
        }

        let mut sync_requests = Vec::with_capacity(fixtures.len());
        let mut expected_acknowledgements = Vec::with_capacity(fixtures.len());
        for fixture in fixtures {
            let request = FleetSubnetRootRegistrySyncRequest {
                expected_registry: current.clone(),
                store_bootstrap: fixture.request.clone(),
            };
            let synchronized: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
                .update_candid(
                    fixture.root_id,
                    CANIC_FLEET_REGISTRY_SYNCHRONIZE,
                    (request.clone(),),
                )
                .expect("synchronize one multi-root fixture transport");
            let synchronized = synchronized.expect("synchronize one multi-root fixture");
            assert_eq!(synchronized.fleet_subnet_root, fixture.root_id);
            assert_eq!(synchronized.version, current);
            expected_acknowledgements.push(synchronized.acknowledgement);
            sync_requests.push(request);
        }
        expected_acknowledgements.sort_by_key(|acknowledgement| acknowledgement.fleet_subnet_root);
        let acknowledgements: Result<Vec<FleetSubnetRootSnapshotAcknowledgement>, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS, ())
            .expect("query multi-root acknowledgements transport");
        assert_eq!(
            acknowledgements.expect("query multi-root acknowledgements"),
            expected_acknowledgements
        );
        (current, sync_requests)
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

    fn validate_active_component_registry_baseline(
        baseline: &CachedPocketIcBaseline<ActiveComponentRegistryBaselineMetadata>,
    ) -> Result<(), ActiveComponentRegistryBaselineError> {
        let pic = baseline.pocket_ic();
        let metadata = baseline.metadata();
        let registry: Result<FleetRegistry, Error> =
            pic.query_candid(metadata.coordinator, CANIC_FLEET_REGISTRY, ())?;
        let registry = baseline_application_result(registry, "query active Fleet Registry")?;
        if registry.fleet_subnet_roots.len() != 1
            || registry.fleet_subnet_roots[0].fleet_subnet_root != metadata.root
            || registry.fleet_subnet_roots[0].status != FleetSubnetRootStatus::Active
        {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "active Fleet Registry root binding changed".to_string(),
            ));
        }

        let activation: Result<FleetActivationStatusResponse, Error> =
            pic.query_candid(metadata.root, CANIC_FLEET_ACTIVATION_STATUS, ())?;
        if baseline_application_result(activation, "query root activation")?.phase
            != FleetActivationPhase::Active
        {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Fleet Subnet Root is not active".to_string(),
            ));
        }

        for binding in [&metadata.issuer, &metadata.verifier] {
            let runtime: Result<ComponentRuntimeStatusResponse, Error> = pic.query_candid_as(
                binding.canister_id,
                metadata.root,
                CANIC_COMPONENT_RUNTIME_STATUS,
                (),
            )?;
            let runtime = baseline_application_result(runtime, "query Component runtime")?;
            if runtime.phase != ComponentRuntimePhase::Active {
                return Err(ActiveComponentRegistryBaselineError::Invariant(format!(
                    "Component {} is not active",
                    binding.canister_id
                )));
            }
        }

        let pool: Result<CanisterPoolResponse, Error> = pic.query_candid(
            metadata.root,
            CANIC_POOL_LIST,
            (CanisterPoolStatusRequest {
                start_after: None,
                limit: 256,
            },),
        )?;
        let pool = baseline_application_result(pool, "query root Canister pool")?;
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

    #[cfg(test)]
    #[expect(
        clippy::too_many_lines,
        reason = "the fixture must drive every real child lifecycle phase before removal"
    )]
    fn create_active_project_instance(
        fixture: &ActiveComponentRegistryFixture,
    ) -> (
        Principal,
        canic::dto::component_registry::ComponentRegistryHead,
    ) {
        let partition: Result<ComponentRegistryPartitionResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: fixture.verifier.component,
                },),
            )
            .expect("query projects Component Registry transport");
        let partition = partition.expect("query projects Component Registry");
        let operation_id = [0xc1; 32];
        let reserved: Result<RootComponentChildAllocationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_ALLOCATE,
                (RootComponentChildAllocationRequest {
                    operation_id,
                    component: fixture.verifier.component,
                    expected_registry: partition.head,
                    child_role: CanisterRole::new("project_instance"),
                    application_init_args: None,
                },),
            )
            .expect("reserve project instance transport");
        assert_eq!(
            reserved.expect("reserve project instance").phase,
            RootComponentAllocationPhase::Reserved
        );

        let created: Result<RootComponentChildAllocationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_CREATE,
                (RootComponentChildCreationRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("create project instance transport");
        let created = created.expect("create project instance");
        assert_eq!(created.phase, RootComponentAllocationPhase::Created);
        let child = created
            .creation
            .as_ref()
            .and_then(|creation| creation.canister)
            .expect("created project instance Canister");

        let installed: Result<RootComponentChildAllocationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_INSTALL,
                (RootComponentChildInstallRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("install project instance transport");
        assert_eq!(
            installed.expect("install project instance").phase,
            RootComponentAllocationPhase::Verified
        );

        let committed: Result<RootComponentChildCommitResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_COMMIT,
                (RootComponentChildCommitRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("commit project instance transport");
        let committed = committed.expect("commit project instance");
        assert_eq!(
            committed.allocation.phase,
            RootComponentAllocationPhase::Committed
        );

        let prepared: Result<RootComponentChildDirectoryPreparationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_DIRECTORY_PREPARE,
                (RootComponentChildDirectoryPreparationRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("prepare project instance Directory transport");
        assert_eq!(
            prepared
                .expect("prepare project instance Directory")
                .child
                .phase,
            ComponentRuntimePhase::DirectoryPrepared
        );

        let activated: Result<RootComponentChildRuntimeActivationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_RUNTIME_ACTIVATE,
                (RootComponentChildRuntimeActivationRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("activate project instance runtime transport");
        assert_eq!(
            activated
                .expect("activate project instance runtime")
                .child
                .phase,
            ComponentRuntimePhase::Active
        );

        let membership: Result<RootComponentChildMembershipActivationResponse, Error> = fixture
            .pic()
            .update_candid_as(
                fixture.root,
                fixture.verifier.canister_id,
                CANIC_ROOT_COMPONENT_CHILD_MEMBERSHIP_ACTIVATE,
                (RootComponentChildMembershipActivationRequest {
                    operation_id,
                    component: fixture.verifier.component,
                },),
            )
            .expect("activate project instance membership transport");
        let membership = membership.expect("activate project instance membership");
        assert_eq!(membership.registry.status, ComponentLifecycleStatus::Active);
        assert_eq!(membership.child.phase, ComponentRuntimePhase::Active);
        (child, membership.registry.head)
    }

    fn assert_root_canister_summary(fixture: &ActiveComponentRegistryFixture) {
        let summary: Result<FleetSubnetRootCanisterSummary, Error> = fixture
            .pic()
            .query_candid(fixture.root, CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY, ())
            .expect("query Fleet Subnet Root Canister summary");
        let summary = summary.expect("Fleet Subnet Root Canister summary");
        let coordinator_version: Result<canic::dto::fleet_registry::FleetRegistryVersion, Error> =
            fixture
                .pic()
                .query_candid(fixture.coordinator, CANIC_FLEET_REGISTRY_VERSION, ())
                .expect("query Coordinator Registry version");

        assert_eq!(
            summary.fleet_registry,
            coordinator_version.expect("Coordinator Registry version")
        );
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

    #[cfg(test)]
    fn assert_root_draining_fence(
        fixture: &ActiveComponentRegistryFixture,
    ) -> FleetSubnetRootDrainingResponse {
        let version: Result<canic::dto::fleet_registry::FleetRegistryVersion, Error> = fixture
            .pic()
            .query_candid(fixture.coordinator, CANIC_FLEET_REGISTRY_VERSION, ())
            .expect("query Coordinator Registry version before root draining");
        let request = FleetSubnetRootDrainingRequest {
            operation_id: [0xd1; 32],
            expected_registry: version.expect("Coordinator Registry version before root draining"),
        };
        let reservation = prepare_root_draining_reservation(fixture, &request);
        let begun: Result<FleetSubnetRootDrainingResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_DRAINING_BEGIN,
                (request.clone(),),
            )
            .expect("begin Fleet Subnet Root draining transport");
        let begun = begun.expect("begin Fleet Subnet Root draining");
        assert_eq!(begun.operation_id, request.operation_id);
        assert_eq!(begun.active_registry, request.expected_registry);
        assert_eq!(begun.reservation_hash, reservation.reservation_hash);
        assert_eq!(begun.fleet_subnet_root, fixture.root);
        assert_eq!(begun.placement_subnet, fixture.verifier.placement_subnet);
        assert_eq!(begun.next_allocation_sequence, 3);
        assert_eq!(begun.reserved_component_instances, 0);
        assert_eq!(begun.committed_component_instances, 2);
        assert_eq!(begun.managed_descendants, 0);
        assert_eq!(begun.known_created_component_canisters, 2);
        assert!(begun.root_registry_encoded_bytes > 0);
        assert!(begun.started_at_ns > 0);

        let repeated: Result<FleetSubnetRootDrainingResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_DRAINING_BEGIN,
                (request,),
            )
            .expect("retry Fleet Subnet Root draining transport");
        assert_eq!(repeated.expect("retry Fleet Subnet Root draining"), begun);
        let status: Result<FleetSubnetRootDrainingResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_FLEET_SUBNET_ROOT_DRAINING_STATUS,
                (FleetSubnetRootDrainingStatusRequest {
                    operation_id: begun.operation_id,
                },),
            )
            .expect("query Fleet Subnet Root draining status transport");
        assert_eq!(status.expect("Fleet Subnet Root draining status"), begun);
        assert_root_allocation_is_fenced(fixture);
        begun
    }

    #[cfg(test)]
    fn prepare_root_draining_reservation(
        fixture: &ActiveComponentRegistryFixture,
        request: &FleetSubnetRootDrainingRequest,
    ) -> FleetSubnetRootDrainingReservationResponse {
        let registry: Result<FleetRegistry, Error> = fixture
            .pic()
            .query_candid(fixture.coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query Coordinator Registry before root draining reservation");
        let expected_root = registry
            .expect("Coordinator Registry before root draining reservation")
            .fleet_subnet_roots
            .into_iter()
            .find(|entry| entry.fleet_subnet_root == fixture.root)
            .expect("target root in Coordinator Registry");
        let reservation: Result<FleetSubnetRootDrainingReservationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.coordinator,
                CANIC_FLEET_REGISTRY_ROOT_DRAINING_RESERVATION_PREPARE,
                (FleetSubnetRootDrainingReservationRequest {
                    operation_id: request.operation_id,
                    expected_registry: request.expected_registry.clone(),
                    expected_root,
                },),
            )
            .expect("prepare Fleet Subnet Root draining reservation transport");
        reservation.expect("prepare Fleet Subnet Root draining reservation")
    }

    #[cfg(test)]
    fn assert_root_allocation_is_fenced(fixture: &ActiveComponentRegistryFixture) {
        let existing: Result<RootComponentAllocationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (RootComponentAllocationRequest {
                    operation_id: [0xa2; 32],
                    component_spec: fixture.verifier.component_spec.clone(),
                },),
            )
            .expect("retry existing Component allocation after root draining transport");
        assert_eq!(
            existing
                .expect("retry existing Component allocation after root draining")
                .phase,
            RootComponentAllocationPhase::Committed
        );

        let rejected: Result<RootComponentAllocationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (RootComponentAllocationRequest {
                    operation_id: [0xa4; 32],
                    component_spec: fixture.verifier.component_spec.clone(),
                },),
            )
            .expect("attempt new Component allocation after root draining transport");
        assert_eq!(
            rejected
                .expect_err("root draining must reject a new top-level Component allocation")
                .code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );
    }

    #[cfg(test)]
    fn assert_coordinator_root_draining_publication(
        fixture: &ActiveComponentRegistryFixture,
        root_draining: &FleetSubnetRootDrainingResponse,
    ) -> FleetSubnetRootDrainingPublicationResponse {
        let request = FleetSubnetRootDrainingPublicationRequest {
            expected_registry: root_draining.active_registry.clone(),
            root_draining: root_draining.clone(),
        };
        let published: Result<FleetSubnetRootDrainingPublicationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.coordinator,
                CANIC_FLEET_REGISTRY_PUBLISH_ROOT_DRAINING,
                (request.clone(),),
            )
            .expect("publish Fleet Subnet Root draining transport");
        let published = published.expect("publish Fleet Subnet Root draining");
        assert_eq!(&published.root_draining, root_draining);
        assert_eq!(published.previous_version, request.expected_registry);
        assert_eq!(
            published.version.revision,
            published.previous_version.revision + 1
        );

        let repeated: Result<FleetSubnetRootDrainingPublicationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.coordinator,
                CANIC_FLEET_REGISTRY_PUBLISH_ROOT_DRAINING,
                (request,),
            )
            .expect("retry Fleet Subnet Root draining publication transport");
        assert_eq!(
            repeated.expect("retry Fleet Subnet Root draining publication"),
            published
        );
        let registry: Result<FleetRegistry, Error> = fixture
            .pic()
            .query_candid(fixture.coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query published Draining Registry transport");
        let registry = registry.expect("query published Draining Registry");
        assert_eq!(registry.revision, published.version.revision);
        assert_eq!(registry.fleet_subnet_roots.len(), 1);
        assert_eq!(
            registry.fleet_subnet_roots[0].status,
            FleetSubnetRootStatus::Draining
        );
        published
    }

    #[cfg(test)]
    fn assert_root_draining_mirror_activation(
        fixture: &ActiveComponentRegistryFixture,
        root_draining: &FleetSubnetRootDrainingResponse,
        published: &FleetSubnetRootDrainingPublicationResponse,
    ) {
        let registry: Result<FleetRegistry, Error> = fixture
            .pic()
            .query_candid(fixture.coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query Draining Registry for mirror activation transport");
        let registry = registry.expect("Draining Registry for mirror activation");
        let directory = FleetDirectorySnapshot {
            provenance: FleetDirectoryProvenance {
                registry: published.version.clone(),
                source_fleet_subnet_root: fixture.root,
            },
            fleet_subnet_roots: registry
                .fleet_subnet_roots
                .iter()
                .map(|entry| FleetSubnetRootDirectoryEntry {
                    placement_subnet: entry.placement_subnet,
                    fleet_subnet_root: entry.fleet_subnet_root,
                    status: entry.status,
                })
                .collect(),
            services: vec![],
        };
        let request = FleetSubnetRootRegistryMirrorActivationRequest {
            previous_registry: root_draining.active_registry.clone(),
            expected_registry: published.version.clone(),
            expected_directory: directory,
            store_bootstrap: fixture.store_bootstrap.clone(),
        };
        let activated: Result<FleetSubnetRootRegistryMirrorActivationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
                (request.clone(),),
            )
            .expect("activate Draining root Registry mirror transport");
        let activated = activated.expect("activate Draining root Registry mirror");
        assert_eq!(activated.previous_registry, request.previous_registry);
        assert_eq!(activated.version, request.expected_registry);
        assert_eq!(activated.directory, request.expected_directory);

        let repeated: Result<FleetSubnetRootRegistryMirrorActivationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
                (request.clone(),),
            )
            .expect("retry Draining root Registry mirror activation transport");
        assert_eq!(
            repeated.expect("retry Draining root Registry mirror activation"),
            activated
        );
        let status: Result<FleetSubnetRootRegistryMirrorActivationResponse, Error> = fixture
            .pic()
            .query_candid(fixture.root, CANIC_FLEET_REGISTRY_MIRROR_STATUS, (request,))
            .expect("query Draining root Registry mirror status transport");
        assert_eq!(
            status.expect("query Draining root Registry mirror status"),
            activated
        );

        let summary: Result<FleetSubnetRootCanisterSummary, Error> = fixture
            .pic()
            .query_candid(fixture.root, CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY, ())
            .expect("query Draining root summary transport");
        let summary = summary.expect("query Draining root summary");
        assert_eq!(summary.fleet_registry, published.version);
        assert_eq!(summary.status, FleetSubnetRootStatus::Draining);

        let prepared: Result<RootComponentRegistryStatusResponse, Error> = fixture
            .pic()
            .query_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                (RootComponentRegistryPreparationRequest {
                    store_bootstrap: fixture.store_bootstrap.clone(),
                    expected_fleet_registry: root_draining.active_registry.clone(),
                },),
            )
            .expect("query Component Registry after mirror advancement transport");
        assert_eq!(
            prepared
                .expect("query Component Registry after mirror advancement")
                .prepared_against_registry,
            root_draining.active_registry
        );

        let rejected: Result<RootComponentAllocationResponse, Error> = fixture
            .pic()
            .update_candid(
                fixture.root,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (RootComponentAllocationRequest {
                    operation_id: [0xa5; 32],
                    component_spec: fixture.verifier.component_spec.clone(),
                },),
            )
            .expect("attempt Component allocation after Draining mirror activation transport");
        assert_eq!(
            rejected
                .expect_err("Draining mirror activation must not reopen root allocation")
                .code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
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
        let config_path = fixture.configuration.config_path(workspace_root);
        let config = AppConfigSnapshot::load(&config_path).expect("load root config");
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
        sync_request: FleetSubnetRootRegistrySyncRequest,
    ) -> RootComponentRegistryPreparationRequest {
        let activated: Result<FleetRegistryActivationResponse, Error> = pic
            .update_candid(
                coordinator,
                CANIC_FLEET_REGISTRY_ACTIVATE,
                (FleetRegistryActivationRequest {
                    expected_registry: joining_version,
                },),
            )
            .expect("activate Registry transport");
        let activated = activated.expect("activate Registry");
        assert_eq!(activated.version.revision, 3);
        let active: Result<FleetRegistry, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query active Registry");
        let active = active.expect("active Registry");
        assert_eq!(
            active.fleet_subnet_roots.first().expect("one root").status,
            FleetSubnetRootStatus::Active
        );
        let directory = FleetDirectorySnapshot {
            provenance: FleetDirectoryProvenance {
                registry: activated.version.clone(),
                source_fleet_subnet_root: fixture.root_id,
            },
            fleet_subnet_roots: active
                .fleet_subnet_roots
                .iter()
                .map(|entry| FleetSubnetRootDirectoryEntry {
                    placement_subnet: entry.placement_subnet,
                    fleet_subnet_root: entry.fleet_subnet_root,
                    status: entry.status,
                })
                .collect(),
            services: vec![],
        };
        let activation_request = FleetSubnetRootRegistryMirrorActivationRequest {
            previous_registry: activated.previous_version,
            expected_registry: activated.version,
            expected_directory: directory,
            store_bootstrap: fixture.request.clone(),
        };
        let mirror: Result<FleetSubnetRootRegistryMirrorActivationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
                (activation_request.clone(),),
            )
            .expect("activate root Registry mirror transport");
        let mirror = mirror.expect("activate root Registry mirror");
        let mirror_retry: Result<FleetSubnetRootRegistryMirrorActivationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
                (activation_request.clone(),),
            )
            .expect("retry root Registry mirror activation transport");
        assert_eq!(
            mirror_retry.expect("retry root Registry mirror activation"),
            mirror
        );
        let mirror_status: Result<FleetSubnetRootRegistryMirrorActivationResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_MIRROR_STATUS,
                (activation_request.clone(),),
            )
            .expect("query root Registry mirror status transport");
        assert_eq!(mirror_status.expect("root Registry mirror status"), mirror);

        let component_registry_request =
            prepare_component_registry(pic, fixture, activation_request);

        let old_candidate: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_FLEET_REGISTRY_SYNC_STATUS,
                (sync_request,),
            )
            .expect("query private Joining candidate after Registry activation");
        assert_eq!(
            old_candidate
                .expect_err("Joining candidate must be replaced")
                .code(),
            canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
        );
        component_registry_request
    }

    #[cfg(test)]
    fn activate_registry_and_prepare_component_registries(
        pic: &PocketIc,
        coordinator: Principal,
        fixtures: &[&BootstrappedRootFixture],
        joining_version: FleetRegistryVersion,
        sync_requests: &[FleetSubnetRootRegistrySyncRequest],
    ) -> FleetRegistryVersion {
        assert_eq!(fixtures.len(), sync_requests.len());
        let activated: Result<FleetRegistryActivationResponse, Error> = pic
            .update_candid(
                coordinator,
                CANIC_FLEET_REGISTRY_ACTIVATE,
                (FleetRegistryActivationRequest {
                    expected_registry: joining_version,
                },),
            )
            .expect("activate multi-root Registry transport");
        let activated = activated.expect("activate multi-root Registry");
        let active: Result<FleetRegistry, Error> = pic
            .query_candid(coordinator, CANIC_FLEET_REGISTRY, ())
            .expect("query active multi-root Registry transport");
        let active = active.expect("query active multi-root Registry");
        assert_eq!(active.fleet_subnet_roots.len(), fixtures.len());
        assert!(
            active
                .fleet_subnet_roots
                .iter()
                .all(|root| root.status == FleetSubnetRootStatus::Active)
        );

        for (fixture, sync_request) in fixtures.iter().zip(sync_requests) {
            let directory = FleetDirectorySnapshot {
                provenance: FleetDirectoryProvenance {
                    registry: activated.version.clone(),
                    source_fleet_subnet_root: fixture.root_id,
                },
                fleet_subnet_roots: active
                    .fleet_subnet_roots
                    .iter()
                    .map(|entry| FleetSubnetRootDirectoryEntry {
                        placement_subnet: entry.placement_subnet,
                        fleet_subnet_root: entry.fleet_subnet_root,
                        status: entry.status,
                    })
                    .collect(),
                services: vec![],
            };
            let activation_request = FleetSubnetRootRegistryMirrorActivationRequest {
                previous_registry: activated.previous_version.clone(),
                expected_registry: activated.version.clone(),
                expected_directory: directory,
                store_bootstrap: fixture.request.clone(),
            };
            let mirror: Result<FleetSubnetRootRegistryMirrorActivationResponse, Error> = pic
                .update_candid(
                    fixture.root_id,
                    CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
                    (activation_request.clone(),),
                )
                .expect("activate one multi-root Registry mirror transport");
            mirror.expect("activate one multi-root Registry mirror");
            prepare_component_registry(pic, fixture, activation_request);
            let old_candidate: Result<FleetSubnetRootRegistrySyncResponse, Error> = pic
                .query_candid(
                    fixture.root_id,
                    CANIC_FLEET_REGISTRY_SYNC_STATUS,
                    (sync_request.clone(),),
                )
                .expect("query replaced multi-root Joining candidate transport");
            assert_eq!(
                old_candidate
                    .expect_err("multi-root Joining candidate must be replaced")
                    .code(),
                canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
            );
        }
        activated.version
    }

    fn prepare_component_registry(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        activation_request: FleetSubnetRootRegistryMirrorActivationRequest,
    ) -> RootComponentRegistryPreparationRequest {
        let component_registry_request = RootComponentRegistryPreparationRequest {
            store_bootstrap: activation_request.store_bootstrap,
            expected_fleet_registry: activation_request.expected_registry,
        };
        let component_registry: Result<RootComponentRegistryStatusResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_PREPARE,
                (component_registry_request.clone(),),
            )
            .expect("prepare root Component Registry transport");
        let component_registry = component_registry.expect("prepare root Component Registry");
        assert_eq!(component_registry.fleet_subnet_root, fixture.root_id);
        assert_eq!(
            component_registry.release_set,
            fixture.init_args.authority.initial_release_set
        );
        assert_eq!(
            component_registry.component_topology_digest,
            fixture
                .init_args
                .authority
                .binding
                .component_topology_digest
        );
        assert_eq!(component_registry.next_allocation_sequence, 1);
        assert_eq!(component_registry.reserved_component_instances, 0);
        assert_eq!(component_registry.committed_component_instances, 0);
        assert_eq!(component_registry.managed_descendants, 0);
        assert_eq!(component_registry.known_created_component_canisters, 0);
        assert_eq!(component_registry.encoded_bytes, 0);

        let component_registry_retry: Result<RootComponentRegistryStatusResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_PREPARE,
                (component_registry_request.clone(),),
            )
            .expect("retry root Component Registry preparation transport");
        assert_eq!(
            component_registry_retry.expect("retry root Component Registry preparation"),
            component_registry
        );
        let component_registry_status: Result<RootComponentRegistryStatusResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                (component_registry_request.clone(),),
            )
            .expect("query root Component Registry status transport");
        assert_eq!(
            component_registry_status.expect("root Component Registry status"),
            component_registry
        );

        component_registry_request
    }

    fn assert_component_allocation(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        component_registry_request: RootComponentRegistryPreparationRequest,
    ) -> ActiveComponentBindings {
        let (issuer_request, issuer) = assert_issuer_component_allocation(pic, fixture);
        let issuer_binding = installed_component_binding(&issuer);
        let projects_request = RootComponentAllocationRequest {
            operation_id: [0xa2; 32],
            component_spec: "projects".parse().expect("projects Component Spec"),
        };
        let projects: Result<RootComponentAllocationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (projects_request.clone(),),
            )
            .expect("reserve projects Component transport");
        let projects = projects.expect("reserve projects Component");
        assert_eq!(projects.allocation_sequence, 2);
        assert_ne!(projects.component, issuer.component);
        assert_eq!(projects.role, CanisterRole::new("project_hub"));
        assert_eq!(projects.phase, RootComponentAllocationPhase::Reserved);
        let projects_component = projects.component;

        let conflicting_retry: Result<RootComponentAllocationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (RootComponentAllocationRequest {
                    operation_id: issuer_request.operation_id,
                    component_spec: projects_request.component_spec.clone(),
                },),
            )
            .expect("conflicting Component reservation retry transport");
        assert_eq!(
            conflicting_retry
                .expect_err("conflicting Component reservation retry must fail")
                .code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );

        let component_registry: Result<RootComponentRegistryStatusResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                (component_registry_request.clone(),),
            )
            .expect("query allocated root Component Registry status transport");
        let component_registry =
            component_registry.expect("allocated root Component Registry status");
        assert_eq!(component_registry.next_allocation_sequence, 3);
        assert_eq!(component_registry.reserved_component_instances, 1);
        assert_eq!(component_registry.committed_component_instances, 1);
        assert_eq!(component_registry.managed_descendants, 0);
        assert_eq!(component_registry.known_created_component_canisters, 1);
        assert!(component_registry.encoded_bytes > 0);
        assert_prepared(pic, fixture.root_id);
        let incomplete_activation: Result<FleetActivationStatusResponse, Error> = pic
            .update_candid(fixture.root_id, CANIC_PREPARE_FLEET_ACTIVATION, ())
            .expect("prepare incomplete root Fleet activation transport");
        assert_eq!(
            incomplete_activation
                .expect_err("reserved Component must prevent root activation preparation")
                .code(),
            canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
        );
        assert_eq!(
            component_registry_status(pic, fixture, component_registry_request.clone())
                .initial_inventory,
            None
        );

        let active_projects = create_component(pic, fixture, projects_request.operation_id);
        assert_eq!(active_projects.component, projects_component);
        assert_eq!(
            active_projects.component_spec,
            projects_request.component_spec
        );
        assert_eq!(active_projects.role, CanisterRole::new("project_hub"));
        assert_eq!(
            active_projects.phase,
            RootComponentAllocationPhase::Committed
        );
        let complete = component_registry_status(pic, fixture, component_registry_request.clone());
        assert_eq!(complete.reserved_component_instances, 0);
        assert_eq!(complete.committed_component_instances, 2);
        assert_eq!(complete.known_created_component_canisters, 2);
        assert_eq!(complete.initial_inventory, None);
        assert_root_runtime_activation(pic, fixture, component_registry_request);
        ActiveComponentBindings::new(
            issuer_binding,
            installed_component_binding(&active_projects),
        )
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

    fn assert_root_runtime_activation(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        component_registry_request: RootComponentRegistryPreparationRequest,
    ) {
        let prepared: Result<FleetActivationStatusResponse, Error> = pic
            .update_candid(fixture.root_id, CANIC_PREPARE_FLEET_ACTIVATION, ())
            .expect("prepare root Fleet activation transport");
        let prepared = prepared.expect("prepare root Fleet activation");
        assert_eq!(prepared.phase, FleetActivationPhase::Prepared);
        let manifest = prepared
            .cascade_manifest
            .as_ref()
            .expect("prepared root infrastructure cascade manifest");
        assert_eq!(manifest.len(), 1);
        assert_eq!(manifest[0].principal, fixture.response.wasm_store);
        let credential = prepared
            .credential
            .expect("prepared root credential generation");

        let sealed = component_registry_status(pic, fixture, component_registry_request.clone())
            .initial_inventory
            .expect("sealed initial Component inventory");
        assert_initial_inventory(&sealed, prepared.identity.operation_id, 2, false, false);

        let request = FleetActivationResumeRequest {
            operation_id: prepared.identity.operation_id,
            credential,
        };
        let activated: Result<FleetActivationStatusResponse, Error> = pic
            .update_candid(fixture.root_id, CANIC_RESUME_FLEET_ACTIVATION, (request,))
            .expect("resume root Fleet activation transport");
        let activated = activated.expect("resume root Fleet activation");
        assert_eq!(activated.phase, FleetActivationPhase::Active);
        assert!(activated.activated_at_ns.is_some_and(|time| time > 0));

        let terminal = component_registry_status(pic, fixture, component_registry_request)
            .initial_inventory
            .expect("terminal initial Component inventory");
        assert_initial_inventory(&terminal, request.operation_id, 2, true, true);
        assert_eq!(terminal.inventory_hash, sealed.inventory_hash);

        let retried: Result<FleetActivationStatusResponse, Error> = pic
            .update_candid(fixture.root_id, CANIC_RESUME_FLEET_ACTIVATION, (request,))
            .expect("retry root Fleet activation transport");
        assert_eq!(
            retried.expect("retry root Fleet activation"),
            activated,
            "root activation retry must preserve its original receipt"
        );
    }

    fn component_registry_status(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        request: RootComponentRegistryPreparationRequest,
    ) -> RootComponentRegistryStatusResponse {
        let status: Result<RootComponentRegistryStatusResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                (request,),
            )
            .expect("query root Component Registry activation status transport");
        status.expect("root Component Registry activation status")
    }

    fn assert_initial_inventory(
        inventory: &RootComponentInitialInventoryStatus,
        operation_id: [u8; 32],
        component_count: u32,
        directories_converged: bool,
        root_runtime_activated: bool,
    ) {
        assert_eq!(inventory.fleet_activation_operation_id, operation_id);
        assert_eq!(inventory.component_count, component_count);
        assert_ne!(inventory.inventory_hash, [0; 32]);
        assert!(inventory.sealed_at_ns > 0);
        assert_eq!(inventory.directories_converged, directories_converged);
        assert_eq!(inventory.root_runtime_activated, root_runtime_activated);
    }

    fn assert_issuer_component_allocation(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
    ) -> (
        RootComponentAllocationRequest,
        RootComponentAllocationResponse,
    ) {
        let issuer_request = RootComponentAllocationRequest {
            operation_id: [0xa1; 32],
            component_spec: "issuer".parse().expect("issuer Component Spec"),
        };
        let issuer: Result<RootComponentAllocationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (issuer_request.clone(),),
            )
            .expect("reserve issuer Component transport");
        let issuer = issuer.expect("reserve issuer Component");
        assert_eq!(issuer.operation_id, issuer_request.operation_id);
        assert_eq!(issuer.allocation_sequence, 1);
        assert_eq!(issuer.component_spec, issuer_request.component_spec);
        assert_eq!(issuer.role, CanisterRole::new("issuer"));
        assert_eq!(
            issuer.component,
            ComponentInstanceId::from_root_allocation(
                fixture
                    .init_args
                    .authority
                    .binding
                    .authority
                    .binding
                    .fleet
                    .fleet,
                fixture.init_args.authority.binding.authority.epoch,
                fixture.root_id,
                1,
            )
        );
        assert_eq!(
            issuer.provisioning_origin,
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: Principal::anonymous(),
            }
        );
        assert_eq!(
            issuer.release_set,
            fixture.init_args.authority.initial_release_set
        );
        assert_eq!(issuer.phase, RootComponentAllocationPhase::Reserved);

        let issuer_retry: Result<RootComponentAllocationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                (issuer_request.clone(),),
            )
            .expect("retry issuer Component reservation transport");
        assert_eq!(
            issuer_retry.expect("retry issuer Component reservation"),
            issuer
        );
        let issuer_status: Result<RootComponentAllocationResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
                (RootComponentAllocationStatusRequest {
                    operation_id: issuer_request.operation_id,
                },),
            )
            .expect("query issuer Component reservation transport");
        assert_eq!(
            issuer_status.expect("issuer Component reservation status"),
            issuer
        );

        let created = create_component(pic, fixture, issuer_request.operation_id);
        (issuer_request, created)
    }

    fn create_component(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
    ) -> RootComponentAllocationResponse {
        let created: Result<RootComponentAllocationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_CREATE,
                (RootComponentCreationRequest { operation_id },),
            )
            .expect("create Component transport");
        let created = created.expect("create Component");
        assert_eq!(created.phase, RootComponentAllocationPhase::Created);
        let creation = created.creation.as_ref().expect("creation evidence");
        let canister = creation.canister.expect("created Canister");
        assert_eq!(creation.wasm_store, fixture.response.wasm_store);
        assert_eq!(creation.controller, fixture.root_id);
        assert!(creation.initial_cycles.to_u128() > 0);
        let artifact = fixture
            .response
            .catalog
            .iter()
            .find(|entry| entry.role == created.role)
            .expect("Component Store artifact");
        assert_eq!(creation.payload_hash, artifact.payload_hash);
        assert_eq!(creation.payload_size_bytes, artifact.payload_size_bytes);

        let canister_status = pic
            .canister_status(canister, Some(fixture.root_id))
            .expect("created Component Canister status");
        assert_eq!(canister_status.settings.controllers, vec![fixture.root_id]);
        assert_eq!(canister_status.module_hash, None);
        assert!(
            canister_status.cycles >= creation.initial_cycles.to_u128(),
            "the claimed prepaid asset must meet the Component's frozen minimum balance"
        );

        let retry: Result<RootComponentAllocationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_CREATE,
                (RootComponentCreationRequest { operation_id },),
            )
            .expect("retry Component creation transport");
        assert_eq!(retry.expect("retry Component creation"), created);
        let created_status: Result<RootComponentAllocationResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
                (RootComponentAllocationStatusRequest { operation_id },),
            )
            .expect("query created Component transport");
        assert_eq!(created_status.expect("created Component status"), created);

        install_component(pic, fixture, operation_id, created)
    }

    fn install_component(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
        created: RootComponentAllocationResponse,
    ) -> RootComponentAllocationResponse {
        let creation = created.creation.as_ref().expect("creation evidence");
        let artifact = fixture
            .response
            .catalog
            .iter()
            .find(|entry| entry.role == created.role)
            .expect("Component Store artifact");
        let installed: Result<RootComponentAllocationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_INSTALL,
                (RootComponentInstallRequest { operation_id },),
            )
            .expect("install Component transport");
        let installed = installed.expect("install Component");
        assert_eq!(installed.phase, RootComponentAllocationPhase::Verified);
        let installation = installed.installation.as_ref().expect("install evidence");
        assert_eq!(installation.raw_module_hash, artifact.raw_module_hash);
        assert_eq!(
            installation.binding.canister_id,
            creation.canister.expect("created Canister")
        );
        assert_eq!(
            installation.binding.component, installed.component,
            "target binding must retain the reserved Component identity"
        );
        let observed_binding: Result<ManagedCanisterBinding, Error> = pic
            .query_candid_as(
                creation.canister.expect("created Canister"),
                fixture.root_id,
                canic::protocol::CANIC_MANAGED_CANISTER_BINDING,
                (),
            )
            .expect("query installed Component binding transport");
        assert_eq!(
            observed_binding.expect("installed Component binding"),
            ManagedCanisterBinding::Component(installation.binding.clone())
        );
        let installed_status = pic
            .canister_status(
                creation.canister.expect("created Canister"),
                Some(fixture.root_id),
            )
            .expect("installed Component Canister status");
        assert_eq!(
            installed_status.module_hash,
            Some(creation.payload_hash.to_vec())
        );

        let install_retry: Result<RootComponentAllocationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_INSTALL,
                (RootComponentInstallRequest { operation_id },),
            )
            .expect("retry Component install transport");
        assert_eq!(install_retry.expect("retry Component install"), installed);
        commit_component(pic, fixture, operation_id, installed)
    }

    fn commit_component(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
        installed: RootComponentAllocationResponse,
    ) -> RootComponentAllocationResponse {
        let committed: Result<RootComponentCommitResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_COMMIT,
                (RootComponentCommitRequest { operation_id },),
            )
            .expect("commit Component transport");
        let committed = committed.expect("commit Component");
        assert_eq!(
            committed.allocation.phase,
            RootComponentAllocationPhase::Committed
        );
        assert_eq!(committed.allocation.component, installed.component);
        assert_eq!(
            committed.allocation.installation, installed.installation,
            "Registry commitment must retain the verified install evidence"
        );
        assert_eq!(committed.registry.head.component, installed.component);
        assert_eq!(committed.registry.head.revision, 1);
        assert_ne!(committed.registry.head.content_hash, [0; 32]);
        assert_eq!(
            committed.registry.binding,
            installed
                .installation
                .as_ref()
                .expect("verified installation")
                .binding
        );
        assert_eq!(
            committed.registry.provisioning_origin,
            installed.provisioning_origin
        );
        assert_eq!(committed.registry.release_set, installed.release_set);
        assert_eq!(
            committed.registry.status,
            ComponentLifecycleStatus::Prepared
        );
        assert!(committed.registry.encoded_bytes > 0);
        assert_eq!(
            committed.directory.provenance.component,
            committed.registry.binding
        );
        assert_eq!(
            committed.directory.provenance.source_fleet_subnet_root,
            fixture.root_id
        );
        assert_eq!(
            committed.directory.provenance.component_registry_revision,
            committed.registry.head.revision
        );
        assert_eq!(
            committed
                .directory
                .provenance
                .component_registry_content_hash,
            committed.registry.head.content_hash
        );
        assert!(
            committed.directory.provenance.synchronized_at_ns > 0,
            "the first Directory must retain its derivation time"
        );
        assert_eq!(committed.directory.descendant_count, 0);

        let retry: Result<RootComponentCommitResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_COMMIT,
                (RootComponentCommitRequest { operation_id },),
            )
            .expect("retry Component commitment transport");
        assert_eq!(retry.expect("retry Component commitment"), committed);
        assert_committed_component_queries(pic, fixture, operation_id, &committed);
        prepare_component_directories(pic, fixture, operation_id, committed)
    }

    fn prepare_component_directories(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
        committed: RootComponentCommitResponse,
    ) -> RootComponentAllocationResponse {
        let target = committed.registry.binding.canister_id;
        let awaiting: Result<ComponentRuntimeStatusResponse, Error> = pic
            .query_candid_as(target, fixture.root_id, CANIC_COMPONENT_RUNTIME_STATUS, ())
            .expect("query awaiting Component runtime Directory transport");
        let awaiting = awaiting.expect("awaiting Component runtime Directory");
        assert_eq!(awaiting.operation_id, operation_id);
        assert_eq!(
            awaiting.binding,
            ManagedCanisterBinding::Component(committed.registry.binding.clone())
        );
        assert_eq!(awaiting.phase, ComponentRuntimePhase::AwaitingDirectory);
        assert_eq!(awaiting.authority, None);
        assert_eq!(awaiting.authority_hash, None);
        assert_eq!(awaiting.activation, None);

        let request = RootComponentDirectoryPreparationRequest { operation_id };
        let prepared: Result<RootComponentDirectoryPreparationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
                (request,),
            )
            .expect("prepare Component Directories transport");
        let prepared = prepared.expect("prepare Component Directories");
        assert_eq!(prepared.committed, committed);
        assert_eq!(
            prepared.target.phase,
            ComponentRuntimePhase::DirectoryPrepared
        );
        assert_eq!(prepared.target.operation_id, operation_id);
        assert_eq!(
            prepared.target.binding,
            ManagedCanisterBinding::Component(prepared.committed.registry.binding.clone())
        );
        let authority = prepared
            .target
            .authority
            .as_ref()
            .expect("retained Component runtime Directory authority");
        assert_eq!(authority.component, prepared.committed.directory);
        assert_eq!(
            authority.fleet.provenance.source_fleet_subnet_root,
            fixture.root_id
        );
        assert!(
            authority
                .fleet
                .fleet_subnet_roots
                .iter()
                .all(|entry| entry.status == FleetSubnetRootStatus::Active)
        );
        assert_ne!(
            prepared
                .target
                .authority_hash
                .expect("Directory authority hash"),
            [0; 32]
        );
        assert_eq!(prepared.target.activation, None);

        let retry: Result<RootComponentDirectoryPreparationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
                (request,),
            )
            .expect("retry Component Directory preparation transport");
        assert_eq!(
            retry.expect("retry Component Directory preparation"),
            prepared
        );
        let observed: Result<ComponentRuntimeStatusResponse, Error> = pic
            .query_candid_as(target, fixture.root_id, CANIC_COMPONENT_RUNTIME_STATUS, ())
            .expect("query prepared Component runtime Directory transport");
        assert_eq!(
            observed.expect("prepared Component runtime Directory"),
            prepared.target
        );
        assert_prepared(pic, fixture.root_id);
        let target_activation: Result<FleetActivationStatusResponse, Error> = pic
            .query_candid_as(target, fixture.root_id, CANIC_FLEET_ACTIVATION_STATUS, ())
            .expect("query prepared Component activation transport");
        assert_eq!(
            target_activation
                .expect("prepared Component activation")
                .phase,
            FleetActivationPhase::Prepared
        );
        activate_component_runtime(pic, fixture, request, prepared)
    }

    fn activate_component_runtime(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        directory_request: RootComponentDirectoryPreparationRequest,
        prepared: RootComponentDirectoryPreparationResponse,
    ) -> RootComponentAllocationResponse {
        let target = prepared.committed.registry.binding.canister_id;
        let request = RootComponentRuntimeActivationRequest {
            operation_id: directory_request.operation_id,
        };
        let activated: Result<RootComponentRuntimeActivationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE,
                (request,),
            )
            .expect("activate Component runtime transport");
        let activated = activated.expect("activate Component runtime");
        assert_eq!(activated.committed, prepared.committed);
        assert_eq!(activated.target.operation_id, request.operation_id);
        assert_eq!(activated.target.binding, prepared.target.binding);
        assert_eq!(activated.target.phase, ComponentRuntimePhase::Active);
        assert_eq!(activated.target.authority, prepared.target.authority);
        assert_eq!(
            activated.target.authority_hash,
            prepared.target.authority_hash
        );
        let activation = activated
            .target
            .activation
            .as_ref()
            .expect("Component runtime activation evidence");
        assert_eq!(
            Some(activation.directory_authority_hash),
            activated.target.authority_hash
        );
        assert!(activation.activated_at_ns > 0);

        let retry: Result<RootComponentRuntimeActivationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE,
                (request,),
            )
            .expect("retry Component runtime activation transport");
        assert_eq!(
            retry.expect("retry Component runtime activation"),
            activated
        );
        let observed: Result<ComponentRuntimeStatusResponse, Error> = pic
            .query_candid_as(target, fixture.root_id, CANIC_COMPONENT_RUNTIME_STATUS, ())
            .expect("query active Component runtime transport");
        assert_eq!(
            observed.expect("active Component runtime"),
            activated.target
        );
        let target_activation: Result<FleetActivationStatusResponse, Error> = pic
            .query_candid_as(target, fixture.root_id, CANIC_FLEET_ACTIVATION_STATUS, ())
            .expect("query active Component Fleet status transport");
        let target_activation = target_activation.expect("active Component Fleet status");
        assert_eq!(target_activation.phase, FleetActivationPhase::Active);
        assert_eq!(target_activation.cascade, None);
        assert_eq!(target_activation.credential, None);
        assert_eq!(
            target_activation.activated_at_ns,
            Some(activation.activated_at_ns)
        );

        let prepared_retry: Result<RootComponentDirectoryPreparationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
                (directory_request,),
            )
            .expect("retry Directory preparation after runtime activation transport");
        assert_eq!(
            prepared_retry.expect("retry Directory preparation after runtime activation"),
            prepared
        );
        assert_committed_component_queries(
            pic,
            fixture,
            request.operation_id,
            &activated.committed,
        );
        assert_prepared(pic, fixture.root_id);
        activate_component_membership(pic, fixture, directory_request, prepared, activated)
    }

    fn activate_component_membership(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        directory_request: RootComponentDirectoryPreparationRequest,
        prepared: RootComponentDirectoryPreparationResponse,
        activated: RootComponentRuntimeActivationResponse,
    ) -> RootComponentAllocationResponse {
        let request = RootComponentMembershipActivationRequest {
            operation_id: directory_request.operation_id,
        };
        let membership: Result<RootComponentMembershipActivationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_MEMBERSHIP_ACTIVATE,
                (request,),
            )
            .expect("activate Component membership transport");
        let membership = membership.expect("activate Component membership");
        assert_eq!(membership.allocation, activated.committed.allocation);
        assert_eq!(membership.registry.status, ComponentLifecycleStatus::Active);
        assert_eq!(membership.registry.head.revision, 2);
        assert_ne!(
            membership.registry.head.content_hash,
            activated.committed.registry.head.content_hash
        );
        assert_eq!(
            membership.directory.provenance.component_registry_revision,
            membership.registry.head.revision
        );
        assert_eq!(
            membership
                .directory
                .provenance
                .component_registry_content_hash,
            membership.registry.head.content_hash
        );
        assert!(
            membership.directory.provenance.synchronized_at_ns
                > activated.committed.directory.provenance.synchronized_at_ns
        );
        assert_eq!(membership.target.phase, ComponentRuntimePhase::Active);
        assert_eq!(
            membership
                .target
                .authority
                .as_ref()
                .expect("current active Directory")
                .component,
            membership.directory
        );
        assert_eq!(
            membership
                .target
                .activation
                .expect("immutable activation receipt")
                .directory_authority_hash,
            activated
                .target
                .authority_hash
                .expect("prepared activation authority hash")
        );

        assert_active_membership_queries(pic, fixture, &membership);
        let retry: Result<RootComponentMembershipActivationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_MEMBERSHIP_ACTIVATE,
                (request,),
            )
            .expect("retry Component membership transport");
        assert_eq!(retry.expect("retry Component membership"), membership);

        assert_pre_membership_retries(pic, fixture, directory_request, prepared, activated);
        assert_prepared(pic, fixture.root_id);
        membership.allocation
    }

    fn assert_active_membership_queries(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        membership: &RootComponentMembershipActivationResponse,
    ) {
        let registry: Result<ComponentRegistryPartitionResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: membership.allocation.component,
                },),
            )
            .expect("query active Component Registry partition transport");
        assert_eq!(
            registry.expect("active Component Registry partition"),
            membership.registry
        );
        let directory: Result<ComponentDirectoryHead, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_HEAD,
                (ComponentDirectoryHeadRequest {
                    component: membership.allocation.component,
                },),
            )
            .expect("query active Component Directory transport");
        assert_eq!(
            directory.expect("active Component Directory"),
            membership.directory
        );
        let target: Result<ComponentRuntimeStatusResponse, Error> = pic
            .query_candid_as(
                membership.registry.binding.canister_id,
                fixture.root_id,
                CANIC_COMPONENT_RUNTIME_STATUS,
                (),
            )
            .expect("query membership-active Component runtime transport");
        assert_eq!(
            target.expect("membership-active Component runtime"),
            membership.target
        );
    }

    fn assert_pre_membership_retries(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        directory_request: RootComponentDirectoryPreparationRequest,
        prepared: RootComponentDirectoryPreparationResponse,
        activated: RootComponentRuntimeActivationResponse,
    ) {
        let commit: Result<RootComponentCommitResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_COMMIT,
                (RootComponentCommitRequest {
                    operation_id: directory_request.operation_id,
                },),
            )
            .expect("retry Component commitment after membership activation transport");
        assert_eq!(
            commit.expect("retry Component commitment after membership activation"),
            activated.committed
        );
        let prepared_retry: Result<RootComponentDirectoryPreparationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
                (directory_request,),
            )
            .expect("retry Directory preparation after membership activation transport");
        assert_eq!(
            prepared_retry.expect("retry Directory preparation after membership activation"),
            prepared
        );
        let activated_retry: Result<RootComponentRuntimeActivationResponse, Error> = pic
            .update_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE,
                (RootComponentRuntimeActivationRequest {
                    operation_id: directory_request.operation_id,
                },),
            )
            .expect("retry runtime activation after membership activation transport");
        assert_eq!(
            activated_retry.expect("retry runtime activation after membership activation"),
            activated
        );
    }

    fn assert_committed_component_queries(
        pic: &PocketIc,
        fixture: &BootstrappedRootFixture,
        operation_id: [u8; 32],
        committed: &RootComponentCommitResponse,
    ) {
        let allocation_status: Result<RootComponentAllocationResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
                (RootComponentAllocationStatusRequest { operation_id },),
            )
            .expect("query committed Component transport");
        assert_eq!(
            allocation_status.expect("committed Component status"),
            committed.allocation
        );

        let registry: Result<ComponentRegistryPartitionResponse, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
                (ComponentRegistryPartitionRequest {
                    component: committed.allocation.component,
                },),
            )
            .expect("query Component Registry partition transport");
        assert_eq!(
            registry.expect("Component Registry partition"),
            committed.registry
        );

        let directory: Result<ComponentDirectoryHead, Error> = pic
            .query_candid(
                fixture.root_id,
                CANIC_ROOT_COMPONENT_DIRECTORY_HEAD,
                (ComponentDirectoryHeadRequest {
                    component: committed.allocation.component,
                },),
            )
            .expect("query Component Directory head transport");
        assert_eq!(
            directory.expect("Component Directory head"),
            committed.directory
        );
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
        coordinator_subnet: Option<Principal>,
        root_subnet: Option<Principal>,
        component_admission_limits: Option<RootComponentAdmissionLimits>,
        fleet_id: Option<FleetId>,
        configuration: RootFixtureConfiguration,
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
        Exact(BTreeMap<ComponentSpecId, u32>),
    }

    #[cfg(test)]
    fn install_bootstrapped_root_on_subnet(
        pic: &PocketIc,
        root_wasm: Vec<u8>,
        coordinator: Principal,
        store_fixture: RootStoreFixture,
        placement_subnet: Principal,
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
                coordinator_subnet: Some(coordinator_subnet),
                root_subnet: Some(placement_subnet),
                component_admission_limits: Some(RootComponentAdmissionLimits::Uniform(1)),
                fleet_id: None,
                configuration: RootFixtureConfiguration::Delegation,
            },
            create_prepaid_pool_assets,
        );
        reset_prepaid_pool_assets(pic, fixture.root_id);
        fixture
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
                coordinator_subnet: Some(coordinator_subnet),
                root_subnet: Some(placement_subnet),
                component_admission_limits: Some(RootComponentAdmissionLimits::Uniform(1)),
                fleet_id: Some(FleetId::from_generated_bytes([fleet_id_byte; 32])),
                configuration: RootFixtureConfiguration::Delegation,
            },
            create_prepaid_pool_assets,
        );
        reset_prepaid_pool_assets(pic, fixture.root_id);
        fixture
    }

    #[cfg(test)]
    fn install_qualification_root_on_subnet(
        pic: &PocketIc,
        root_wasm: Vec<u8>,
        coordinator: Principal,
        store_fixture: RootStoreFixture,
        placement_subnet: Principal,
        component_admission_limits: BTreeMap<ComponentSpecId, u32>,
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
                coordinator_subnet: Some(coordinator_subnet),
                root_subnet: Some(placement_subnet),
                component_admission_limits: Some(RootComponentAdmissionLimits::Exact(
                    component_admission_limits,
                )),
                fleet_id: None,
                configuration: RootFixtureConfiguration::Toko,
            },
            create_prepaid_pool_assets,
        );
        reset_prepaid_pool_assets(pic, fixture.root_id);
        fixture
    }

    #[cfg(test)]
    fn install_qualification_root_for_fleet_on_subnet(
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
                coordinator_subnet: Some(coordinator_subnet),
                root_subnet: Some(placement_subnet),
                component_admission_limits: Some(RootComponentAdmissionLimits::Uniform(1)),
                fleet_id: Some(FleetId::from_generated_bytes([fleet_id_byte; 32])),
                configuration: RootFixtureConfiguration::Toko,
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
                coordinator_subnet: None,
                root_subnet: None,
                component_admission_limits: None,
                fleet_id: None,
                configuration: RootFixtureConfiguration::Delegation,
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
        let config_path = placement.configuration.config_path(workspace_root);
        let RootStoreFixture {
            manifest,
            artifacts,
            configuration,
        } = store_fixture;
        assert_eq!(configuration, placement.configuration);
        let manifest_bytes = serde_json::to_vec(&manifest).expect("canonical root release set");
        let digest = ReleaseSetDigest::from_bytes(
            wasm_hash(&manifest_bytes)
                .try_into()
                .expect("SHA-256 digest"),
        );
        let root_id = placement.root_subnet.map_or_else(
            || pic.create_canister(),
            |subnet| pic.create_canister_on_subnet(None, None, subnet),
        );
        pic.add_cycles(root_id, ROOT_INSTALL_CYCLES);
        let root_subnet = pic.get_subnet(root_id).expect("root placement Subnet");
        let wasm_store = pic.create_canister_on_subnet(None, None, root_subnet);
        pic.add_cycles(wasm_store, ROOT_INSTALL_CYCLES);
        let wasm_store_wasm = build_test_wasm_store_wasm();
        let installation_controller = Principal::from_slice(&[0x46; 29]);
        let init_bytes =
            install_root_args_with_release_set_digest_and_coordinator(ManagedRootInstallInput {
                root_id,
                wasm_store,
                installation_controller,
                coordinator,
                root_wasm: &root_wasm,
                wasm_store_wasm: &wasm_store_wasm,
                config_path: &config_path,
                release_set_digest: digest,
            })
            .expect("encode exact root authority");
        let mut init_args =
            decode_one::<FleetSubnetRootInitArgs>(&init_bytes).expect("decode root init authority");
        bind_fixture_fleet_id(&mut init_args, placement.fleet_id);
        if let Some(component_admission_limits) = placement.component_admission_limits {
            for admission in &mut init_args.authority.binding.component_admissions {
                admission.maximum_root_instances = match &component_admission_limits {
                    RootComponentAdmissionLimits::Uniform(limit) => *limit,
                    RootComponentAdmissionLimits::Exact(limits) => {
                        *limits.get(&admission.component_spec).unwrap_or_else(|| {
                            panic!(
                                "missing qualification admission for Component Spec '{}'",
                                admission.component_spec
                            )
                        })
                    }
                };
            }
            let config = AppConfigSnapshot::load(&config_path).expect("reload root config");
            init_args.authority.binding.component_topology_digest = config
                .component_topology()
                .project_for_admissions(&init_args.authority.binding.component_admissions)
                .and_then(|projection| projection.digest())
                .expect("compile bounded multi-root topology digest");
        }
        bind_init_args_to_pocket_ic_subnet(
            pic,
            root_id,
            placement.coordinator_subnet,
            &mut init_args,
        );
        init_args.canister_pool_imports = pool_setup(pic, root_id);
        let store_init_args = FleetSubnetWasmStoreInitArgs {
            authority: init_args.authority.wasm_store_authority.clone(),
            install_id: init_args.install_id,
        };
        prepare_sibling_wasm_store_controllers(pic, wasm_store, installation_controller, root_id);
        pic.install_canister(
            wasm_store,
            wasm_store_wasm,
            encode_one(store_init_args).expect("encode live PocketIC Store authority"),
            Some(installation_controller),
        );
        let init_bytes = encode_one(&init_args).expect("encode live PocketIC root authority");
        pic.install_canister(root_id, root_wasm, init_bytes, None);
        adopt_sibling_wasm_store(pic, root_id, &init_args);
        assert_prepared(pic, root_id);
        let (request, response) = bootstrap_root_store_release_set(
            pic,
            root_id,
            &manifest,
            artifacts,
            &manifest_bytes,
            digest,
        );
        BootstrappedRootFixture {
            root_id,
            init_args,
            request,
            response,
            configuration,
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
        init_args
            .authority
            .wasm_store_authority
            .authority
            .binding
            .fleet
            .fleet
            .fleet_id = fleet_id;
    }

    fn bootstrap_root_store_release_set(
        pic: &PocketIc,
        root_id: Principal,
        manifest: &RootStoreReleaseSetManifest,
        artifacts: BTreeMap<CanisterRole, Vec<u8>>,
        manifest_bytes: &[u8],
        digest: ReleaseSetDigest,
    ) -> (RootStoreBootstrapRequest, RootStoreBootstrapResponse) {
        let version = TemplateVersion::owned(manifest.release_build_id.to_string());
        stage_chunked_payload(
            pic,
            root_id,
            TemplateId::owned(format!("{ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX}{digest}")),
            version.clone(),
            manifest_bytes,
        );
        for (role, bytes) in artifacts {
            let template_id =
                TemplateId::owned(format!("{ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX}{role}"));
            let staged: Result<(), Error> = pic
                .update_candid(
                    root_id,
                    CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
                    (TemplateManifestInput {
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
                    },),
                )
                .expect("stage artifact manifest transport");
            staged.expect("stage artifact manifest");
            stage_chunked_payload(pic, root_id, template_id, version.clone(), &bytes);
        }

        let request = RootStoreBootstrapRequest {
            manifest_payload_size_bytes: manifest_bytes.len() as u64,
        };
        let response: Result<RootStoreBootstrapResponse, Error> = pic
            .update_candid(root_id, CANIC_ROOT_STORE_BOOTSTRAP, (request.clone(),))
            .expect("root Store bootstrap transport");
        (request, response.expect("root Store bootstrap"))
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
        for _ in 0..PREPAID_POOL_ASSET_COUNT {
            let response: Result<PoolAdminResponse, Error> = pic
                .update_candid(root, CANIC_POOL_ADMIN, (PoolAdminCommand::Maintain,))
                .expect("reset prepaid Canister transport");
            assert!(matches!(
                response.expect("reset prepaid Canister"),
                PoolAdminResponse::ResetReady { .. } | PoolAdminResponse::Maintained
            ));
        }
        let status: Result<CanisterPoolResponse, Error> = pic
            .query_candid(
                root,
                CANIC_POOL_LIST,
                (CanisterPoolStatusRequest {
                    start_after: None,
                    limit: 256,
                },),
            )
            .expect("query prepared prepaid inventory transport");
        let status = status.expect("query prepared prepaid inventory");
        assert_eq!(
            status.ready,
            u32::try_from(PREPAID_POOL_ASSET_COUNT).expect("bounded fixture pool size")
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
        let configuration = RootFixtureConfiguration::Delegation;
        let config_path = configuration.config_path(workspace_root);
        let (manifest, artifacts) =
            exact_root_store_fixture(&config_path, build_test_component_wasms());
        RootStoreFixture {
            manifest,
            artifacts,
            configuration,
        }
    }

    #[cfg(test)]
    fn build_toko_root_store_fixture() -> RootStoreFixture {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root");
        let configuration = RootFixtureConfiguration::Toko;
        let config_path = configuration.config_path(workspace_root);
        let (manifest, artifacts) =
            exact_root_store_fixture(&config_path, build_toko_component_wasms());
        RootStoreFixture {
            manifest,
            artifacts,
            configuration,
        }
    }

    fn exact_root_store_fixture(
        config_path: &Path,
        real_modules: &BTreeMap<CanisterRole, Vec<u8>>,
    ) -> (RootStoreReleaseSetManifest, BTreeMap<CanisterRole, Vec<u8>>) {
        let config = AppConfigSnapshot::load(config_path).expect("load root fixture config");
        let topology = config.component_topology();
        let release_build_id = managed_test_init_identity().release_build_id;
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
        RootStoreReleaseSetEntry {
            component_spec: component_spec.clone(),
            kind,
            artifact: RootStoreArtifact {
                role: role.clone(),
                package: config
                    .roles
                    .get(role)
                    .expect("fixture role declaration")
                    .package
                    .clone(),
                release_build_id,
                wasm_relative_path: format!("{role}.wasm"),
                wasm_size_bytes: raw.len() as u64,
                wasm_sha256_hex: hex_bytes(wasm_hash(&raw)),
                wasm_gz_relative_path: format!("{role}.wasm.gz"),
                wasm_gz_size_bytes: compressed.len() as u64,
                wasm_gz_sha256_hex: hex_bytes(wasm_hash(&compressed)),
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
                &[
                    ("issuer", ISSUER_PACKAGE),
                    ("project_hub", PROJECT_HUB_PACKAGE),
                    ("project_instance", PROJECT_INSTANCE_PACKAGE),
                    ("project_ledger", PROJECT_LEDGER_PACKAGE),
                    ("project_machine", PROJECT_MACHINE_PACKAGE),
                ],
            )
        })
    }

    #[cfg(test)]
    fn build_toko_component_wasms() -> &'static BTreeMap<CanisterRole, Vec<u8>> {
        static WASMS: OnceLock<BTreeMap<CanisterRole, Vec<u8>>> = OnceLock::new();
        WASMS.get_or_init(|| {
            let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
            let config_path = toko_root_canister_config_path(&workspace_root);
            build_component_fixture_wasms(
                &workspace_root,
                &config_path,
                "toko-fleet-registry-sync",
                &[
                    ("database_a", DATABASE_A_PACKAGE),
                    ("database_b", DATABASE_B_PACKAGE),
                    ("database_c", DATABASE_C_PACKAGE),
                    ("issuer", ISSUER_PACKAGE),
                    ("project_hub", PROJECT_HUB_PACKAGE),
                    ("project_instance", PROJECT_INSTANCE_PACKAGE),
                    ("project_ledger", PROJECT_LEDGER_PACKAGE),
                    ("project_machine", PROJECT_MACHINE_PACKAGE),
                ],
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
        build_internal_test_wasm_canisters_with_env(
            workspace_root,
            &target_dir,
            &packages,
            CanicWasmBuildProfile::Fast,
            &[(
                canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
                canonical_config_path,
            )],
        );
        let profile = CanicWasmBuildProfile::Fast.target_dir_name();
        roles_and_packages
            .iter()
            .map(|(role, package)| {
                (
                    CanisterRole::new(role),
                    read_wasm(&target_dir, package, profile),
                )
            })
            .collect()
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes).expect("gzip fixture Wasm");
        encoder.finish().expect("finish fixture Wasm gzip")
    }

    fn stage_chunked_payload(
        pic: &PocketIc,
        root_id: Principal,
        template_id: TemplateId,
        version: TemplateVersion,
        payload: &[u8],
    ) {
        let chunks = payload
            .chunks(CANIC_WASM_CHUNK_BYTES)
            .map(<[u8]>::to_vec)
            .collect::<Vec<_>>();
        let prepared: Result<TemplateChunkSetInfoResponse, Error> = pic
            .update_candid(
                root_id,
                CANIC_TEMPLATE_PREPARE_ADMIN,
                (TemplateChunkSetPrepareInput {
                    template_id: template_id.clone(),
                    version: version.clone(),
                    payload_hash: wasm_hash(payload),
                    payload_size_bytes: payload.len() as u64,
                    chunk_hashes: chunks.iter().map(|chunk| wasm_hash(chunk)).collect(),
                },),
            )
            .expect("prepare staged payload transport");
        prepared.expect("prepare staged payload");
        for (chunk_index, bytes) in chunks.into_iter().enumerate() {
            let published: Result<(), Error> = pic
                .update_candid(
                    root_id,
                    CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
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
        let status: Result<FleetActivationStatusResponse, Error> = pic
            .query_candid(root_id, CANIC_FLEET_ACTIVATION_STATUS, ())
            .expect("query root activation status");
        assert_eq!(
            status.expect("root activation status").phase,
            FleetActivationPhase::Prepared
        );
        let authority: Result<FleetSubnetRootAuthority, Error> = pic
            .query_candid(root_id, CANIC_FLEET_SUBNET_ROOT_AUTHORITY, ())
            .expect("query root authority");
        assert_eq!(
            authority.expect("root authority").binding.fleet_subnet_root,
            root_id
        );
    }
}

pub use tests::{ActiveComponentRegistryFixture, setup_active_component_registry};
