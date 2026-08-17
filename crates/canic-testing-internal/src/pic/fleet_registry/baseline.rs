//! Prepared-root Fleet Registry and Component Registry PocketIC journey.

#[cfg(test)]
use super::build::build_mainnet_refill_wasms;
use super::build::{
    build_pic, build_test_root_wasm, build_test_wasm_store_wasm, root_canister_config_path,
};
use candid::Principal;
use ic_testkit::pic::{CandidCallExt, PocketIc};
use std::path::Path;

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;
const PREPAID_POOL_ASSET_COUNT: usize = 10;
const PREPAID_POOL_ASSET_CYCLES: u128 = 6_000_000_000_000;

mod tests {
    use super::*;
    use candid::{CandidType, Deserialize, decode_one, encode_one};
    #[cfg(test)]
    use canic::dto::authority_restore::{
        AuthorityRestoreFencePhase, AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest,
    };
    #[cfg(test)]
    use canic::dto::pool::{CanisterPoolAssetOrigin, CanisterPoolAssetStatus};
    use canic::dto::pool::{
        CanisterPoolResponse, CanisterPoolStatusRequest, PoolMaintenanceResponse,
    };
    use canic::{
        CANIC_WASM_CHUNK_BYTES,
        dto::{
            component_registry::{
                ComponentRuntimePhase, RootComponentAllocationPhase,
                RootComponentAllocationRequest, RootComponentAllocationResponse,
                RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
            },
            fleet_registry::{
                FleetRegistryActivationRequest, FleetSubnetRootEntry, FleetSubnetRootJoinRequest,
                FleetSubnetRootRegistrySyncRequest, FleetSubnetRootStatus,
            },
            fleet_subnet_root::{
                FleetSubnetRootAuthority, FleetSubnetRootCanisterSummary, FleetSubnetRootInitArgs,
                FleetSubnetWasmStoreInitArgs,
            },
            role::{ComponentRuntimeOperationStatus, OperationReceipt, OperationStatusRequest},
            root_store::{
                ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX, ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX,
                RootStoreArtifact, RootStoreBootstrapRequest, RootStoreBootstrapResponse,
                RootStoreReleaseSetEntry, RootStoreReleaseSetEntryKind,
                RootStoreReleaseSetManifest,
            },
        },
        ids::{CanisterRole, ComponentBinding, FleetId, ReleaseSetDigest, SubnetId},
    };
    use canic::{
        Error,
        dto::fleet_activation::{FleetActivationPhase, FleetActivationResumeRequest},
    };
    use canic_control_plane::{
        dto::template::{
            StoreCommand, StoreCommandResponse, TemplateChunkInput, TemplateChunkSetInfoResponse,
            TemplateChunkSetPrepareInput, TemplateManifestInput,
        },
        dto::{
            fleet_coordinator::{
                CoordinatorCommand, CoordinatorCommandResponse, CoordinatorStatusRequest,
                CoordinatorStatusResponse, FleetCoordinatorInitArgs,
            },
            root::RootOperationStatusResponse,
        },
        ids::{
            TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion,
            WasmStoreBinding,
        },
    };
    use canic_core::cdk::utils::hash::{hex_bytes, wasm_hash};
    use canic_host::release_set::AppConfigSnapshot;
    use flate2::{Compression, write::GzEncoder};
    use std::{
        collections::BTreeMap, error::Error as StdError, fmt, io::Write, num::NonZeroUsize,
        sync::OnceLock, time::Duration,
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
        FailureDisposition, FixtureRecipeId, PocketIcBaselineRecipe, PocketIcDiagnosticsExt,
        PreparedBaseline, ReadinessReceipt, RebuildReason, ResetAchievement, ResetReceipt,
        ResetRequirement, ResetRequirements, SnapshotRestoreFunding, TimeResetPolicy,
        ValidationReceipt, is_dead_pocket_ic_transport_error,
    };
    #[cfg(test)]
    use ic_testkit::pic::{PocketIcCapturedSnapshotExt, PocketIcSnapshotExt};

    use crate::pic::CanicPicExt;
    #[cfg(test)]
    use canic::dto::fleet_registry::FleetSubnetRootDrainingReservationRequest;
    #[cfg(test)]
    use canic_control_plane::dto::fleet_coordinator::CoordinatorOperationStatusResponse;

    const ISSUER_PACKAGE: &str = "delegation_issuer_stub";
    const COORDINATOR_INSTALL_CYCLES: u128 = 500_000_000_000_000;

    #[derive(CandidType)]
    enum RootCommandFragment {
        BootstrapStore(RootStoreBootstrapRequest),
        MaintainPool,
        #[cfg(test)]
        PrepareAuthoritySnapshot(AuthoritySnapshotRequest),
        PrepareComponentRegistry(RootComponentRegistryPreparationRequest),
        PrepareFleetActivation,
        ProvisionComponent(RootComponentAllocationRequest),
        #[cfg(test)]
        ResumeAuthoritySnapshot(AuthoritySnapshotRequest),
        ResumeFleetActivation(FleetActivationResumeRequest),
        SynchronizeRegistry(FleetSubnetRootRegistrySyncRequest),
    }

    #[derive(CandidType, Debug, Deserialize)]
    #[expect(
        clippy::large_enum_variant,
        reason = "the PocketIC decoder mirrors the direct Root command wire"
    )]
    enum RootCommandResponseFragment {
        MaintainPool(PoolMaintenanceResponse),
        OperationAccepted(OperationReceipt),
        #[cfg(test)]
        PrepareAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
        PrepareComponentRegistry(RootComponentRegistryStatusResponse),
        #[cfg(test)]
        ResumeAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
    }

    #[derive(CandidType)]
    enum RootStatusRequestFragment {
        #[cfg(test)]
        AuthorityRestore,
        FleetAuthority,
        Inventory,
        Operation(OperationStatusRequest),
        Pool(CanisterPoolStatusRequest),
    }

    #[derive(CandidType, Deserialize)]
    #[expect(
        clippy::large_enum_variant,
        reason = "the PocketIC decoder mirrors the direct Root status wire"
    )]
    enum RootStatusResponseFragment {
        #[cfg(test)]
        AuthorityRestore(AuthorityRestoreFenceStatusResponse),
        FleetAuthority(FleetSubnetRootAuthority),
        Inventory(FleetSubnetRootCanisterSummary),
        Operation(RootOperationStatusResponse),
        Pool(CanisterPoolResponse),
    }

    #[derive(CandidType)]
    enum ManagedStatusRequestFragment {
        Operation(OperationStatusRequest),
    }

    #[derive(CandidType, Deserialize)]
    enum ManagedStatusResponseFragment {
        Operation(ManagedOperationStatusResponseFragment),
    }

    #[derive(CandidType, Deserialize)]
    enum ManagedOperationStatusResponseFragment {
        ConfigureRuntime(ComponentRuntimeOperationStatus),
    }

    fn root_command(
        pic: &PocketIc,
        root: Principal,
        command: RootCommandFragment,
    ) -> Result<RootCommandResponseFragment, Error> {
        pic.update_candid(root, canic::protocol::CANIC_COMMAND, (command,))
            .expect("Root command transport")
    }

    fn root_status(
        pic: &PocketIc,
        root: Principal,
        request: RootStatusRequestFragment,
    ) -> Result<RootStatusResponseFragment, Error> {
        pic.query_candid(root, canic::protocol::CANIC_STATUS, (request,))
            .expect("Root status transport")
    }

    fn root_pool_status(pic: &PocketIc, root: Principal) -> CanisterPoolResponse {
        let RootStatusResponseFragment::Pool(status) = root_status(
            pic,
            root,
            RootStatusRequestFragment::Pool(CanisterPoolStatusRequest {
                start_after: None,
                limit: 256,
            }),
        )
        .expect("query Canister pool") else {
            panic!("Root returned a differently correlated pool status");
        };
        status
    }

    fn coordinator_command(
        pic: &PocketIc,
        coordinator: Principal,
        command: CoordinatorCommand,
    ) -> Result<CoordinatorCommandResponse, Error> {
        pic.update_candid(coordinator, canic::protocol::CANIC_COMMAND, (command,))
            .expect("Coordinator command transport")
    }

    fn coordinator_status(
        pic: &PocketIc,
        coordinator: Principal,
        request: CoordinatorStatusRequest,
    ) -> Result<CoordinatorStatusResponse, Error> {
        pic.query_candid(coordinator, canic::protocol::CANIC_STATUS, (request,))
            .expect("Coordinator status transport")
    }

    fn store_command_as(
        pic: &PocketIc,
        store: Principal,
        caller: Principal,
        command: StoreCommand,
    ) -> Result<StoreCommandResponse, Error> {
        pic.update_candid_as(store, caller, canic::protocol::CANIC_COMMAND, (command,))
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
        request: RootStoreBootstrapRequest,
        response: RootStoreBootstrapResponse,
    }

    struct RootStoreFixture {
        manifest: RootStoreReleaseSetManifest,
        artifacts: BTreeMap<CanisterRole, Vec<u8>>,
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
            let RootCommandResponseFragment::MaintainPool(_) =
                root_command(&pic, fixture.root_id, RootCommandFragment::MaintainPool)
                    .expect("automatic pool maintenance")
            else {
                panic!("Root returned a differently correlated pool response");
            };
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
            chunk_hashes: vec![payload_hash],
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
            prepare.clone(),
        );
        assert_eq!(
            denied
                .expect_err("anonymous Store prepare must fail")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );

        let former_installation_controller = fixture
            .init_args
            .authority
            .wasm_store_authority
            .installation_controller;
        let denied = store_prepare_as(
            &pic,
            fixture.response.wasm_store,
            former_installation_controller,
            prepare,
        );
        assert_eq!(
            denied
                .expect_err("former installation controller must lose Store mutation authority")
                .code(),
            canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
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
        let CoordinatorStatusResponse::Registry(registry) =
            coordinator_status(pic, coordinator, CoordinatorStatusRequest::Registry)
                .expect("query isolated Coordinator Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status");
        };
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
        let fixture = acquire_active_component_registry();
        super::super::role_attestation::assert_registry_bound_role_attestation(
            fixture.pic(),
            fixture.root,
            &fixture.issuer,
            &fixture.verifier,
        );
    }

    #[test]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the pooled Fleet fixture lease is intentionally retained for the full test"
    )]
    fn restored_root_preserves_its_inventory_but_cannot_allocate() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let RootStatusResponseFragment::Inventory(before) = root_status(
            fixture.pic(),
            fixture.root,
            RootStatusRequestFragment::Inventory,
        )
        .expect("query root inventory before snapshot") else {
            panic!("Root returned a differently correlated inventory status");
        };

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
            rejected_resume
                .expect_err("restored root authority must remain mutation-fenced")
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
    fn published_draining_root_autonomously_reaches_external_deletion_readiness() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let CoordinatorStatusResponse::Registry(registry) = coordinator_status(
            fixture.pic(),
            fixture.coordinator,
            CoordinatorStatusRequest::Registry,
        )
        .expect("query active Registry before root removal") else {
            panic!("Coordinator returned a differently correlated Registry status");
        };
        let CoordinatorStatusResponse::RegistryVersion(version) = coordinator_status(
            fixture.pic(),
            fixture.coordinator,
            CoordinatorStatusRequest::RegistryVersion,
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
            operation_id,
            expected_registry: version,
            expected_root,
        };
        let CoordinatorCommandResponse::OperationAccepted(receipt) = coordinator_command(
            fixture.pic(),
            fixture.coordinator,
            CoordinatorCommand::RemoveRoot(request.clone()),
        )
        .expect("submit autonomous root removal") else {
            panic!("Coordinator returned a differently correlated removal response");
        };
        assert_eq!(receipt.operation_id, operation_id);

        let CoordinatorCommandResponse::OperationAccepted(retried) = coordinator_command(
            fixture.pic(),
            fixture.coordinator,
            CoordinatorCommand::RemoveRoot(request),
        )
        .expect("retry autonomous root removal") else {
            panic!("Coordinator returned a differently correlated removal response");
        };
        assert_eq!(retried, receipt);

        let mut terminal = None;
        let mut last_status = None;
        for _ in 0..256 {
            if let Ok(RootStatusResponseFragment::Operation(
                RootOperationStatusResponse::RemoveRoot(status),
            )) = root_status(
                fixture.pic(),
                fixture.root,
                RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
            ) {
                let complete = status.deletion_preparation.is_some();
                last_status = Some(status.clone());
                if complete {
                    terminal = Some(status);
                    break;
                }
            }
            fixture.pic().advance_time(Duration::from_secs(1));
            fixture.pic().tick();
        }
        let terminal = terminal.unwrap_or_else(|| {
            let coordinator = coordinator_status(
                fixture.pic(),
                fixture.coordinator,
                CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .ok()
            .and_then(|response| match response {
                CoordinatorStatusResponse::Operation(
                    CoordinatorOperationStatusResponse::RootRemoval(status),
                ) => Some(status),
                _ => None,
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
            let pool = root_pool_status(fixture.pic(), fixture.root);
            panic!(
                "Root must autonomously reach external deletion readiness; \
                 root(final_inventory, removal, reclamation, binding, store_deletion, \
                 readiness_intent, readiness, preparation)={root_progress:?}; \
                 pool(tracked, store, store_deletion_pending, workload, handing_off)=({}, {}, {}, {}, {}); \
                 coordinator(draining, removal, readiness_intent, readiness, execution, completion)={coordinator_progress:?}",
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

        let pool = root_pool_status(fixture.pic(), fixture.root);
        assert_eq!(pool.tracked, 0);
        assert_eq!(pool.completed_handoffs, PREPAID_POOL_ASSET_COUNT as u64);
        for component in [&fixture.issuer, &fixture.verifier] {
            let handed_off = fixture
                .pic()
                .canister_status(component.canister_id, Some(fixture.coordinator))
                .expect("Coordinator must control each handed-off Component canister");
            assert_eq!(handed_off.module_hash, None);
            let mut controllers = handed_off.settings.controllers;
            controllers.sort();
            let mut expected_controllers = vec![fixture.root, fixture.coordinator];
            expected_controllers.sort();
            assert_eq!(controllers, expected_controllers);
        }
        assert!(
            fixture
                .pic()
                .canister_status(fixture.wasm_store, Some(fixture.root))
                .is_err(),
            "autonomous removal must delete the retained Store"
        );

        let CoordinatorStatusResponse::Operation(CoordinatorOperationStatusResponse::RootRemoval(
            coordinator,
        )) = coordinator_status(
            fixture.pic(),
            fixture.coordinator,
            CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
        )
        .expect("query Coordinator root-removal status")
        else {
            panic!("Coordinator returned a differently correlated operation status");
        };
        assert!(coordinator.draining.is_some());
        assert!(coordinator.removal.is_some());
        assert!(coordinator.readiness_intent.is_some());
        assert!(coordinator.readiness.is_some());
        assert!(coordinator.execution.is_none());
        assert!(coordinator.completion.is_none());
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
        eprintln!("[pic_fleet_registry] active baseline {outcome}");

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
            issuer_runtime_operation_id: components.issuer_runtime_operation_id,
            verifier_runtime_operation_id: components.verifier_runtime_operation_id,
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
        let CoordinatorStatusResponse::RegistryVersion(genesis) =
            coordinator_status(pic, coordinator, CoordinatorStatusRequest::RegistryVersion)
                .expect("query Registry genesis")
        else {
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
        let CoordinatorStatusResponse::RootAcknowledgements(acknowledgements) = coordinator_status(
            pic,
            coordinator,
            CoordinatorStatusRequest::RootAcknowledgements,
        )
        .expect("query root acknowledgements") else {
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

    fn validate_active_component_registry_baseline(
        baseline: &CachedPocketIcBaseline<ActiveComponentRegistryBaselineMetadata>,
    ) -> Result<(), ActiveComponentRegistryBaselineError> {
        let pic = baseline.pocket_ic();
        let metadata = baseline.metadata();
        let CoordinatorStatusResponse::Registry(registry) = baseline_application_result(
            coordinator_status(
                pic,
                metadata.coordinator,
                CoordinatorStatusRequest::Registry,
            ),
            "query active Fleet Registry",
        )?
        else {
            return Err(ActiveComponentRegistryBaselineError::Invariant(
                "Coordinator returned a differently correlated Registry status".to_string(),
            ));
        };
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
                canic::protocol::CANIC_STATUS,
                (ManagedStatusRequestFragment::Operation(
                    OperationStatusRequest { operation_id },
                ),),
            )?;
            let ManagedStatusResponseFragment::Operation(
                ManagedOperationStatusResponseFragment::ConfigureRuntime(runtime),
            ) = baseline_application_result(runtime, "query Component runtime")?;
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
        let CoordinatorStatusResponse::RegistryVersion(coordinator_version) = coordinator_status(
            fixture.pic(),
            fixture.coordinator,
            CoordinatorStatusRequest::RegistryVersion,
        )
        .expect("query Coordinator Registry version") else {
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
        let CoordinatorStatusResponse::Registry(active) =
            coordinator_status(pic, coordinator, CoordinatorStatusRequest::Registry)
                .expect("query active Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status");
        };
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
        _component_registry_request: RootComponentRegistryPreparationRequest,
    ) -> ActiveComponentBindings {
        let issuer = provision_component(pic, fixture, [0xa1; 32]);
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
        let RootCommandResponseFragment::OperationAccepted(receipt) = root_command(
            pic,
            fixture.root_id,
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
                fixture.root_id,
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
                let RootCommandResponseFragment::OperationAccepted(retried) = root_command(
                    pic,
                    fixture.root_id,
                    RootCommandFragment::ProvisionComponent(request),
                )
                .expect("retry Component provisioning") else {
                    panic!("Root returned a differently correlated provisioning response");
                };
                assert_eq!(retried, receipt);
                return status.allocation;
            }
            last_allocation = Some(status.allocation);
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }

        pic.dump_canister_debug(fixture.root_id, "autonomous Component provisioning");
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

        pic.dump_canister_debug(root, "autonomous root Fleet activation");
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
        coordinator_subnet: Option<Principal>,
        root_subnet: Option<Principal>,
        component_admission_limits: Option<RootComponentAdmissionLimits>,
        fleet_id: Option<FleetId>,
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
                coordinator_subnet: Some(coordinator_subnet),
                root_subnet: Some(placement_subnet),
                component_admission_limits: Some(RootComponentAdmissionLimits::Uniform(1)),
                fleet_id: Some(FleetId::from_generated_bytes([fleet_id_byte; 32])),
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
        let RootStoreFixture {
            manifest,
            artifacts,
        } = store_fixture;
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
            request,
            response,
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
            let RootCommandResponseFragment::MaintainPool(response) =
                root_command(pic, root, RootCommandFragment::MaintainPool)
                    .expect("reset prepaid Canister")
            else {
                panic!("Root returned a differently correlated pool response");
            };
            assert!(matches!(
                response,
                PoolMaintenanceResponse::ResetReady { .. } | PoolMaintenanceResponse::Maintained
            ));
        }
        let status = root_pool_status(pic, root);
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
        let config_path = root_canister_config_path(workspace_root);
        let (manifest, artifacts) =
            exact_root_store_fixture(&config_path, build_test_component_wasms());
        RootStoreFixture {
            manifest,
            artifacts,
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
}

pub use tests::{ActiveComponentRegistryFixture, setup_active_component_registry};
