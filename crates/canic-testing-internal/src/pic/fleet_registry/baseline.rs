//! Prepared-root Fleet Registry and Component Registry PocketIC journey.

#[cfg(test)]
use super::build::{
    build_five_component_root_wasm, build_five_trillion_component_root_wasm, build_icp_refill_pic,
    build_icp_refill_stub_wasm, build_mainnet_five_component_refill_wasms,
    build_mainnet_refill_wasms, build_toko_shaped_singleton_root_wasm, build_two_root_pic,
    five_component_root_canister_config_path, five_trillion_component_root_canister_config_path,
    toko_shaped_singleton_root_canister_config_path,
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
pub(in crate::pic) use tests::governed_pocketic_cases;

mod tests {
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
    use canic::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest;
    #[cfg(test)]
    use canic::dto::pool::{
        CanisterPoolAssetOrigin, CanisterPoolAssetStatus, PoolCanisterRequest, PoolImportResponse,
        PoolLedgerRecoveryArtifact, PoolLedgerRecoveryPhase, PoolLedgerRecoveryReceipt,
        PoolLedgerRecoveryRequest,
    };
    use canic::dto::pool::{
        CanisterPoolResponse, CanisterPoolStatusRequest, PoolMaintenanceResponse,
    };
    #[cfg(test)]
    use canic::dto::runtime::{CanicRuntimeStatus, TimerRegistrationStatus};
    #[cfg(test)]
    use canic::dto::{fleet_admission::FleetAdmissionProjectionStatusResponse, page::PageRequest};
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
            role::{
                ComponentRuntimeOperationStatus, OperationReceipt, OperationStatusRequest,
                RoleOverviewResponse,
            },
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
    #[cfg(test)]
    use canic_control_plane::dto::root::RootFundingStatusResponse;
    #[cfg(test)]
    use canic_control_plane::dto::template::{
        StoreStatusRequest, StoreStatusResponse, TemplateLookupRequest, TemplateManifestResponse,
        TemplateStagingStatusResponse,
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
    #[cfg(test)]
    use canic_core::{
        cdk::types::Cycles,
        dto::{
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
        ids::{FleetCoordinatorRootFundingPolicy, FleetSubnetRootFundingAuthority},
    };
    #[cfg(test)]
    use canic_host::fleet_ensure::model::{
        CurrentFleetProtocolAction, DesiredFleet, FLEET_ENSURE_SCHEMA_VERSION,
        FleetEnsureStateRecord,
    };
    #[cfg(test)]
    use canic_host::fleet_ensure::{
        CompiledCurrentComponentProvisioning, CompiledCurrentProtocolStep,
        CompiledCurrentStoreSequence, CurrentComponentGroupPlacement, CurrentRegistryStage,
        append_qualified_pool_ledger_recovery_artifact, compile_current_component_provisioning,
        compile_current_protocol_sequence, compile_current_registry_sequence,
        compile_current_registry_sequence_with_status, compile_current_store_sequence_from_union,
    };
    use canic_host::release_set::AppConfigSnapshot;
    #[cfg(test)]
    use canic_host::release_set::{ApplicationArtifactEntry, ApplicationArtifactUnion};
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

    use crate::pic::fleet_registry::fixture::progress_elapsed;
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
    use pocket_ic::{
        CreateCanisterParams, CreateCanisterPlacement, common::rest::RawEffectivePrincipal,
    };

    #[cfg(test)]
    use canic::dto::component_provisioning::{
        FleetComponentProvisioningPhase, FleetComponentProvisioningRetryStage,
    };
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
    #[derive(CandidType)]
    enum RootCommandFragment {
        #[cfg(test)]
        AdoptStore(FleetSubnetWasmStoreAdoptionRequest),
        BootstrapStore(RootStoreBootstrapRequest),
        #[cfg(test)]
        ImportPoolCanister(PoolCanisterRequest),
        MaintainPool,
        #[cfg(test)]
        PrepareAuthoritySnapshot(AuthoritySnapshotRequest),
        PrepareComponentRegistry(RootComponentRegistryPreparationRequest),
        PrepareFleetActivation,
        ProvisionComponent(RootComponentAllocationRequest),
        #[cfg(test)]
        RecoverPoolLedger(PoolLedgerRecoveryRequest),
        #[cfg(test)]
        RespondCapability(canic::dto::capability::RootCapabilityEnvelopeV1),
        #[cfg(test)]
        ResumeAuthoritySnapshot(AuthoritySnapshotRequest),
        ResumeFleetActivation(FleetActivationResumeRequest),
        SynchronizeRegistry(FleetSubnetRootRegistrySyncRequest),
    }

    #[derive(CandidType, Debug, Deserialize)]
    #[cfg_attr(
        not(test),
        expect(
            clippy::large_enum_variant,
            reason = "the non-test decoder mirrors the direct Root command wire"
        )
    )]
    enum RootCommandResponseFragment {
        #[cfg(test)]
        ImportPoolCanister(PoolImportResponse),
        MaintainPool(PoolMaintenanceResponse),
        OperationAccepted(OperationReceipt),
        #[cfg(test)]
        PrepareAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
        PrepareComponentRegistry(RootComponentRegistryStatusResponse),
        #[cfg(test)]
        RecoverPoolLedger(PoolLedgerRecoveryReceipt),
        #[cfg(test)]
        ResumeAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
        #[cfg(test)]
        RespondCapability(canic::dto::capability::RootCapabilityResponseV1),
    }

    #[derive(CandidType)]
    #[cfg_attr(
        test,
        expect(
            clippy::large_enum_variant,
            reason = "the PocketIC encoder mirrors the direct Root status wire"
        )
    )]
    enum RootStatusRequestFragment {
        #[cfg(test)]
        Admission(PageRequest),
        #[cfg(test)]
        AuthorityRestore,
        #[cfg(test)]
        ComponentRegistry(RootComponentRegistryPreparationRequest),
        FleetAuthority,
        #[cfg(test)]
        Funding,
        Inventory,
        Operation(OperationStatusRequest),
        Pool(CanisterPoolStatusRequest),
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
        FleetAuthority(FleetSubnetRootAuthority),
        #[cfg(test)]
        Funding(RootFundingStatusResponse),
        Inventory(FleetSubnetRootCanisterSummary),
        Operation(RootOperationStatusResponse),
        Pool(CanisterPoolResponse),
        #[cfg(test)]
        Runtime(Box<CanicRuntimeStatus>),
    }

    #[derive(CandidType)]
    enum ManagedStatusRequestFragment {
        Operation(OperationStatusRequest),
    }

    #[derive(CandidType, Deserialize)]
    enum ManagedStatusResponseFragment {
        Operation(ManagedOperationStatusResponseFragment),
    }

    #[cfg(test)]
    #[derive(CandidType)]
    enum ManagedAdmissionStatusRequestFragment {
        Admission(PageRequest),
    }

    #[cfg(test)]
    #[derive(CandidType, Deserialize)]
    enum ManagedAdmissionStatusResponseFragment {
        Admission(FleetAdmissionProjectionStatusResponse),
    }

    #[derive(CandidType, Deserialize)]
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
        pic.update_candid(root, canic::protocol::CANIC_ROOT_COMMAND, (command,))
            .expect("Root command transport")
    }

    #[cfg(test)]
    fn request_descendant_funding(
        pic: &PocketIc,
        root: Principal,
        descendant: Principal,
        request: canic::dto::capability::RootCapabilityEnvelopeV1,
    ) -> u128 {
        let response: Result<RootCommandResponseFragment, Error> = pic
            .update_candid_as(
                root,
                descendant,
                canic::protocol::CANIC_ROOT_COMMAND,
                (RootCommandFragment::RespondCapability(request),),
            )
            .expect("request exact descendant funding from Root");
        let RootCommandResponseFragment::RespondCapability(response) =
            response.expect("Root accepts registered descendant funding request")
        else {
            panic!("Root returned a differently correlated capability response");
        };
        let canic::dto::rpc::Response::Cycles(canic::dto::rpc::CyclesResponse::Transferred {
            cycles_transferred,
        }) = response.response
        else {
            panic!("Root returned a differently correlated cycles response");
        };
        cycles_transferred
    }

    fn root_status(
        pic: &PocketIc,
        root: Principal,
        request: RootStatusRequestFragment,
    ) -> Result<RootStatusResponseFragment, Error> {
        pic.query_candid(root, canic::protocol::CANIC_STATUS, (request,))
            .expect("Root status transport")
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
            canic::protocol::CANIC_STATUS,
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

    fn coordinator_status(
        pic: &PocketIc,
        coordinator: Principal,
        request: CoordinatorStatusRequest,
    ) -> Result<CoordinatorStatusResponse, Error> {
        pic.query_candid(coordinator, canic::protocol::CANIC_STATUS, (request,))
            .expect("Coordinator status transport")
    }

    #[cfg(test)]
    fn await_fleet_admission_convergence(
        pic: &PocketIc,
        coordinator: Principal,
        operation_id: [u8; 32],
    ) -> FleetAdmissionMutationResponse {
        let mut last_phase = String::new();
        for _ in 0..128 {
            let status = coordinator_status(
                pic,
                coordinator,
                CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .expect("query Fleet admission operation");
            let CoordinatorStatusResponse::Operation(
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
                CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .expect("query interrupted Fleet admission operation");
            let CoordinatorStatusResponse::Operation(
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
                    let expected_root_boundaries = roots
                        .iter()
                        .flat_map(|root| {
                            ["activating", "opening", "perimeter_fenced", "preparing"]
                                .map(|phase| (*root, phase))
                        })
                        .collect::<std::collections::BTreeSet<_>>();
                    assert_eq!(restarted_roots, expected_root_boundaries);
                    return response.clone();
                }
            };
            if let Some(boundary) = boundary
                && restarted.insert(boundary)
            {
                if boundary == "activating" {
                    for (root, target) in roots.iter().zip(targets) {
                        pic.stop_canister(*target, Some(*root))
                            .expect("hold target before Root activation effect");
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
                canic::protocol::CANIC_STATUS,
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
        manifest: RootStoreReleaseSetManifest,
        artifacts: BTreeMap<CanisterRole, Vec<u8>>,
    }

    #[cfg(test)]
    #[derive(CandidType)]
    struct CyclesLedgerStubInitArgs {
        canister_ids: Vec<Principal>,
        expected_root: Principal,
        expected_subnet: Principal,
        pending_first_index: Option<u64>,
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

    #[test]
    fn prepared_mainnet_root_automatically_refills_one_exact_pool_asset() {
        assert_mainnet_refill(false, 1);
    }

    #[test]
    fn uncertain_mainnet_refill_reuses_the_exact_paid_request() {
        assert_mainnet_refill(true, 2);
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
            let CoordinatorStatusResponse::Operation(
                CoordinatorOperationStatusResponse::ComponentProvisioning(status),
            ) = coordinator_status(
                &pic,
                coordinator,
                CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
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
                        let asset = pic.create_canister_on_subnet(None, None, root_subnet);
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
                        expected_root: root,
                        expected_subnet: root_subnet,
                        pending_first_index: None,
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

        let mut pending_root_failure = None;
        let mut provisioned = None;
        let mut last_status = None;
        for _ in 0..240 {
            let CoordinatorStatusResponse::Operation(
                CoordinatorOperationStatusResponse::ComponentProvisioning(status),
            ) = coordinator_status(
                &pic,
                coordinator,
                CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
            )
            .expect("query fresh Component provisioning")
            else {
                panic!("Coordinator returned a differently correlated operation status");
            };
            if status.pending_root_failure.is_some() {
                pending_root_failure = status.pending_root_failure;
            }
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
            report_canister_diagnostics_batch(
                &pic,
                [
                    ("Coordinator", coordinator, Principal::anonymous()),
                    ("Root", fixture.root_id, Principal::anonymous()),
                ],
                "fresh provisioning automatic pool readiness",
            );
            panic!("fresh Component provisioning did not complete: {last_status:?}")
        });
        assert_eq!(provisioned.component_count, 5);
        let failure = pending_root_failure.expect("one automatic-capacity retry is observable");
        assert_eq!(failure.fleet_subnet_root, fixture.root_id);
        assert_eq!(
            failure.stage,
            FleetComponentProvisioningRetryStage::RootProvisioning
        );
        assert_eq!(
            failure.diagnostic_code,
            canic_core::diagnostics::codes::STATE_CONFLICT
                .raw_code()
                .raw()
        );
        assert!(failure.failed_at_ns > 0);
        assert!(provisioned.pending_root_failure.is_none());

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

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one governed journey keeps exact Store publication, Root bootstrap, helper staging, and replay evidence together"
    )]
    fn current_store_stages_recovery_helper_after_root_bootstrap_and_replays_zero_effects() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = five_component_root_canister_config_path(&workspace_root);
        let config = AppConfigSnapshot::load(&config_path).expect("load five-Component config");
        let configuration = config
            .model()
            .compile_component_deployment_configuration()
            .expect("compile five-Component deployment configuration");
        let root_wasm = build_five_component_root_wasm();
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
        let mut store_sequence = compile_current_store_sequence_from_union(
            &artifact_root,
            &configuration.component_topology,
            &installed.init_args.authority,
            operation_id,
            &union,
        )
        .expect("compile current Store sequence");
        let helper_manifest = append_fixture_pool_ledger_recovery_artifact(
            &mut store_sequence,
            installed.manifest.release_build_id,
        );
        assert!(
            store_sequence
                .expected_bootstrap
                .catalog
                .iter()
                .all(|entry| entry.role.as_str() != "pool_ledger_recovery"),
            "the temporary helper must remain outside the application catalog"
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
        let bootstrap_position = store_sequence
            .actions
            .iter()
            .position(|action| matches!(action, CurrentFleetProtocolAction::BootstrapStore { .. }))
            .expect("Root bootstrap protocol step");
        let helper_position = store_sequence
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    CurrentFleetProtocolAction::StageStoreManifest { request }
                        if request.role.as_str() == "pool_ledger_recovery"
                )
            })
            .expect("recovery helper Store protocol step");
        assert!(
            bootstrap_position < helper_position,
            "Root must bootstrap its application catalog before helper staging"
        );
        let actions = store_sequence
            .actions
            .iter()
            .enumerate()
            .map(|(index, action)| CompiledCurrentProtocolStep {
                action: action.clone(),
                name: format!("store-helper-{index}"),
                target: match action {
                    CurrentFleetProtocolAction::AdoptStore { .. }
                    | CurrentFleetProtocolAction::BootstrapStore { .. } => installed.root_id,
                    CurrentFleetProtocolAction::PrepareStoreChunkSet { .. }
                    | CurrentFleetProtocolAction::PublishStoreChunk { .. }
                    | CurrentFleetProtocolAction::StageStoreManifest { .. } => wasm_store,
                    _ => panic!("Store sequence emitted a non-Store/Root action"),
                },
            })
            .collect::<Vec<_>>();
        for step in &actions {
            issue_current_protocol_step(&pic, step, installation_controller);
            await_current_protocol_step(&pic, step, installation_controller);
        }
        let helper_status = current_store_staging_status(
            &pic,
            wasm_store,
            installation_controller,
            &helper_manifest.template_id,
            &helper_manifest.version,
        );
        assert_eq!(
            helper_status.manifest.as_ref(),
            Some(&current_manifest_response(&helper_manifest)),
            "the real Store must retain the exact post-bootstrap helper manifest"
        );
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
    #[expect(
        clippy::too_many_lines,
        reason = "one governed journey proves the full ordered graph and its zero-effect replay"
    )]
    fn fresh_five_component_provisioning_reaches_runtime_active_and_publishes_catalog() {
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
        let mut store_sequence = compile_current_store_sequence_from_union(
            &artifact_root,
            &configuration.component_topology,
            &installed.init_args.authority,
            operation_id,
            &union,
        )
        .expect("compile current Store sequence");
        let helper_manifest = append_fixture_pool_ledger_recovery_artifact(
            &mut store_sequence,
            installed.manifest.release_build_id,
        );
        assert!(
            store_sequence
                .expected_bootstrap
                .catalog
                .iter()
                .all(|entry| entry.role.as_str() != "pool_ledger_recovery"),
            "the temporary helper must remain outside the application catalog"
        );
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
        let CoordinatorStatusResponse::Registry(genesis) =
            coordinator_status(&pic, coordinator, CoordinatorStatusRequest::Registry)
                .expect("query current genesis Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status");
        };
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
        let bootstrap_position = actions
            .iter()
            .position(|step| {
                matches!(
                    step.action,
                    CurrentFleetProtocolAction::BootstrapStore { .. }
                )
            })
            .expect("Root bootstrap protocol step");
        let helper_position = actions
            .iter()
            .position(|step| {
                matches!(
                    &step.action,
                    CurrentFleetProtocolAction::StageStoreManifest { request }
                        if request.role.as_str() == "pool_ledger_recovery"
                )
            })
            .expect("recovery helper Store protocol step");
        assert!(
            bootstrap_position < helper_position,
            "Root must bootstrap its application catalog before helper staging"
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
                            CoordinatorStatusRequest::Operation(OperationStatusRequest {
                                operation_id: request.operation_id,
                            }),
                        );
                        let failure = match status {
                            Ok(CoordinatorStatusResponse::Operation(
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
            } else {
                issue_current_protocol_step(&pic, step, installation_controller);
            }
            await_current_protocol_step(&pic, step, installation_controller);
        }
        assert!(replayed_component_command);

        let CoordinatorStatusResponse::Registry(terminal_registry) =
            coordinator_status(&pic, coordinator, CoordinatorStatusRequest::Registry)
                .expect("query terminal Fleet Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status");
        };
        assert_eq!(terminal_registry.revision, 4);
        let CoordinatorStatusResponse::Operation(
            CoordinatorOperationStatusResponse::ComponentProvisioning(terminal_status),
        ) = coordinator_status(
            &pic,
            coordinator,
            CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
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
        let helper_status = current_store_staging_status(
            &pic,
            wasm_store,
            installation_controller,
            &helper_manifest.template_id,
            &helper_manifest.version,
        );
        assert_eq!(
            helper_status.manifest.as_ref(),
            Some(&current_manifest_response(&helper_manifest)),
            "the real Store must retain the exact post-bootstrap helper manifest"
        );
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
        std::fs::remove_dir_all(artifact_root).expect("remove artifact-union fixture");
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one governed zero-estate gate keeps host effect replay, real control-plane convergence, conservation and terminal replay together"
    )]
    fn literal_zero_estate_reaches_one_workload_and_one_ready_pool_asset() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
        let config_path = toko_shaped_singleton_root_canister_config_path(&workspace_root);
        let config =
            AppConfigSnapshot::load(&config_path).expect("load singleton Component config");
        let mut component_specs = config.model().component_specs.values();
        let component_spec = component_specs
            .next()
            .expect("one singleton Component Spec");
        assert!(component_specs.next().is_none());
        assert_eq!(component_spec.initial_cycles.to_u128(), 1_900_000_000_000);
        let configuration = config
            .model()
            .compile_component_deployment_configuration()
            .expect("compile singleton Component deployment configuration");
        let root_wasm = build_toko_shaped_singleton_root_wasm();
        let coordinator_wasm = build_test_coordinator_wasm();
        let store_fixture = build_root_store_fixture_with_config(
            &config_path,
            build_toko_shaped_singleton_component_wasms(),
        );
        let pic = build_pic();

        let readiness_floor = 1_900_000_000_000_u128;
        let pool_creation_funding =
            canic_host::fleet_ensure::fresh_pool_creation_funding(readiness_floor)
                .expect("generated fresh-pool creation funding");
        assert_eq!(pool_creation_funding, 3_000_000_000_000);
        let requested = [
            ("coordinator", COORDINATOR_INSTALL_CYCLES),
            ("root", ROOT_INSTALL_CYCLES),
            ("store", ROOT_INSTALL_CYCLES),
            ("pool-0", pool_creation_funding),
            ("pool-1", pool_creation_funding),
        ];
        let mut creations = BTreeMap::<String, (Principal, String, u32)>::new();
        let mut placement_subnet = None;
        for (name, cycles) in requested {
            let create = |pic: &PocketIc, placement_subnet: Option<Principal>| {
                pic.create_canister_with_params(
                    None,
                    CreateCanisterParams {
                        cycles: Some(cycles),
                        placement: placement_subnet.map(CreateCanisterPlacement::SubnetId),
                        ..CreateCanisterParams::default()
                    },
                )
                .expect("apply exact fresh Create")
            };
            let principal = create(&pic, placement_subnet);
            placement_subnet.get_or_insert_with(|| {
                pic.get_subnet(principal)
                    .expect("fresh Coordinator placement Subnet")
            });
            let receipt = format!("zero-estate-create:{name}:{principal}");
            assert!(
                creations
                    .insert(name.to_string(), (principal, receipt.clone(), 1))
                    .is_none()
            );

            // Lose every Create response, then recover the same retained receipt and Principal.
            let replayed = creations.get(name).expect("retained Create response");
            assert_eq!(replayed.0, principal);
            assert_eq!(replayed.1, receipt);
            assert_eq!(replayed.2, 1);
        }
        assert_eq!(creations.len(), 5);
        assert!(
            creations
                .values()
                .all(|(_, receipt, count)| { !receipt.is_empty() && *count == 1 })
        );

        let coordinator = creations["coordinator"].0;
        let root = creations["root"].0;
        let store = creations["store"].0;
        let pools = [creations["pool-0"].0, creations["pool-1"].0];
        let installed = install_current_root_with_config_and_pool_setup(
            &pic,
            root_wasm,
            coordinator,
            store_fixture,
            BootstrappedRootPlacement {
                canister_pool_maximum_size: Some(2),
                canister_pool_minimum_size: Some(2),
                canister_pool_cycles: Some(Cycles::new(readiness_floor)),
                coordinator_subnet: placement_subnet,
                existing_root: Some(root),
                existing_wasm_store: Some(store),
                root_subnet: placement_subnet,
                component_admission_limits: Some(RootComponentAdmissionLimits::Uniform(1)),
                fleet_id: Some(FleetId::from_generated_bytes([0x79; 32])),
                funding: None,
                coordinator_root_funding: None,
            },
            &config_path,
            |_pic, _root| pools.to_vec(),
        );
        assert_eq!(installed.root_id, root);
        assert_eq!(
            installed
                .init_args
                .authority
                .wasm_store_authority
                .wasm_store,
            store
        );

        let mut controller_mutations = BTreeMap::new();
        for pool in pools {
            pic.set_controllers(pool, None, vec![root])
                .expect("finalize fresh pool controller");
            controller_mutations.insert(pool, 1_u32);

            // Lose each update response; exact live Root-only control closes the intent.
            let status = pic
                .canister_status(pool, Some(root))
                .expect("observe Root-owned pool after lost controller response");
            assert_eq!(status.settings.controllers, vec![root]);
            assert_eq!(controller_mutations[&pool], 1);
        }

        // Each Root import/reset is allowed to lose its response; replay observes Maintained.
        reset_prepaid_pool_assets_for_count(&pic, root, 2);
        let RootCommandResponseFragment::MaintainPool(replayed_reset) =
            root_command(&pic, root, RootCommandFragment::MaintainPool)
                .expect("replay lost pool reset response")
        else {
            panic!("Root returned a differently correlated pool response");
        };
        assert!(matches!(
            replayed_reset,
            PoolMaintenanceResponse::Maintained
        ));
        let ready_pool = root_pool_status(&pic, root);
        assert_eq!(ready_pool.ready, 2);
        assert_eq!(ready_pool.failed, 0);
        let ready_imports = ready_pool
            .entries
            .iter()
            .filter(|entry| pools.contains(&entry.canister_id))
            .collect::<Vec<_>>();
        assert_eq!(ready_imports.len(), 2);
        assert!(ready_imports.iter().all(|entry| {
            entry.origin == CanisterPoolAssetOrigin::Imported
                && entry.status == CanisterPoolAssetStatus::Ready
                && entry.cycles.to_u128() >= readiness_floor
        }));

        let operation_id = [0x7a; 32];
        let artifact_root = test_target_dir(&workspace_root, "literal-zero-estate-protocol")
            .join(format!("artifact-union-{}", std::process::id()));
        if artifact_root.exists() {
            std::fs::remove_dir_all(&artifact_root)
                .expect("clear prior literal zero-estate fixture");
        }
        std::fs::create_dir_all(&artifact_root)
            .expect("create literal zero-estate artifact fixture");
        let union = fixture_application_artifact_union(&artifact_root, &installed);
        let mut store_sequence = compile_current_store_sequence_from_union(
            &artifact_root,
            &configuration.component_topology,
            &installed.init_args.authority,
            operation_id,
            &union,
        )
        .expect("compile singleton Store sequence");
        append_fixture_pool_ledger_recovery_artifact(
            &mut store_sequence,
            installed.manifest.release_build_id,
        );
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
        let CoordinatorStatusResponse::Registry(genesis) =
            coordinator_status(&pic, coordinator, CoordinatorStatusRequest::Registry)
                .expect("query zero-estate Registry genesis")
        else {
            panic!("Coordinator returned a differently correlated Registry status");
        };
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
        .expect("compile zero-estate Registry sequence");
        let stores = BTreeMap::from([(root, store_sequence)]);
        let actions = compile_current_protocol_sequence(
            &desired,
            &state,
            &configuration,
            &registry_sequence,
            &authorities,
            &stores,
            operation_id,
        )
        .expect("compile complete zero-estate production protocol");
        let installation_controller = installed
            .init_args
            .authority
            .wasm_store_authority
            .installation_controller;
        for step in &actions {
            issue_current_protocol_step(&pic, step, installation_controller);
            if matches!(
                step.action,
                CurrentFleetProtocolAction::ProvisionComponents { .. }
            ) {
                // Lose the claim response and replay the exact operation before observing it.
                issue_current_protocol_step(&pic, step, installation_controller);
            }
            await_current_protocol_step(&pic, step, installation_controller);
        }

        let terminal_pool = root_pool_status(&pic, root);
        assert_eq!(terminal_pool.workload, 1);
        assert_eq!(terminal_pool.ready, 1);
        assert_eq!(terminal_pool.failed, 0);
        assert_eq!(
            terminal_pool
                .entries
                .iter()
                .filter(|entry| pools.contains(&entry.canister_id))
                .count(),
            2
        );
        let workload = terminal_pool
            .entries
            .iter()
            .filter(|entry| pools.contains(&entry.canister_id))
            .find(|entry| matches!(entry.status, CanisterPoolAssetStatus::Workload { .. }))
            .expect("one singleton Workload");
        let ready = terminal_pool
            .entries
            .iter()
            .filter(|entry| pools.contains(&entry.canister_id))
            .find(|entry| entry.status == CanisterPoolAssetStatus::Ready)
            .expect("one retained Ready pool asset");
        assert_ne!(workload.canister_id, ready.canister_id);
        for pool in pools {
            let status = pic
                .canister_status(pool, Some(root))
                .expect("observe terminal pool controller set");
            assert_eq!(status.settings.controllers, vec![root]);
        }

        let requested_controlled_cycles = requested
            .into_iter()
            .map(|(_, cycles)| cycles)
            .sum::<u128>();
        let final_controlled_cycles = [coordinator, root, store, pools[0], pools[1]]
            .into_iter()
            .map(|canister| pic.cycle_balance(canister))
            .sum::<u128>();
        let measured_execution_burn_cycles = requested_controlled_cycles
            .checked_sub(final_controlled_cycles)
            .expect("fresh estate cannot gain unreviewed controlled cycles");
        assert_eq!(
            final_controlled_cycles + measured_execution_burn_cycles,
            requested_controlled_cycles
        );

        let CoordinatorStatusResponse::Registry(terminal_registry) =
            coordinator_status(&pic, coordinator, CoordinatorStatusRequest::Registry)
                .expect("query terminal zero-estate Registry")
        else {
            panic!("Coordinator returned a differently correlated terminal Registry");
        };
        let CoordinatorStatusResponse::Operation(
            CoordinatorOperationStatusResponse::ComponentProvisioning(terminal_status),
        ) = coordinator_status(
            &pic,
            coordinator,
            CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
        )
        .expect("query terminal singleton Component operation")
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
        .expect("recognize terminal zero-estate Registry");
        let replay = compile_current_protocol_sequence(
            &desired,
            &state,
            &configuration,
            &terminal_sequence,
            &authorities,
            &stores,
            operation_id,
        )
        .expect("compile immediate zero-estate replay");
        assert!(replay.iter().all(|step| {
            current_protocol_step_is_terminal(&pic, step, installation_controller)
        }));
        std::fs::remove_dir_all(artifact_root)
            .expect("remove literal zero-estate artifact fixture");
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
    fn append_fixture_pool_ledger_recovery_artifact(
        sequence: &mut CompiledCurrentStoreSequence,
        release_build_id: canic_core::ids::ReleaseBuildId,
    ) -> TemplateManifestInput {
        let helper_raw = b"\0asm\x01\0\0\0";
        let helper_compressed = gzip(helper_raw);
        let helper_payload_hash: [u8; 32] = wasm_hash(&helper_compressed)
            .try_into()
            .expect("recovery helper payload SHA-256");
        append_qualified_pool_ledger_recovery_artifact(
            sequence,
            PoolLedgerRecoveryArtifact {
                candid_sha256: [0x17; 32],
                payload_hash: helper_payload_hash,
                payload_size_bytes: helper_compressed.len() as u64,
                raw_module_hash: wasm_hash(helper_raw)
                    .try_into()
                    .expect("recovery helper raw SHA-256"),
                release_build_id,
            },
            &helper_compressed,
        )
        .expect("append qualified pool Ledger recovery helper");
        sequence
            .actions
            .iter()
            .find_map(|action| match action {
                CurrentFleetProtocolAction::StageStoreManifest { request }
                    if request.role.as_str() == "pool_ledger_recovery" =>
                {
                    Some(request.clone())
                }
                _ => None,
            })
            .expect("post-bootstrap recovery helper manifest")
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
            CurrentFleetProtocolAction::PrepareStoreChunkSet { .. }
            | CurrentFleetProtocolAction::PublishStoreChunk { .. }
            | CurrentFleetProtocolAction::StageStoreManifest { .. }
            | CurrentFleetProtocolAction::AdoptStore { .. }
            | CurrentFleetProtocolAction::BootstrapStore { .. } => 0,
            CurrentFleetProtocolAction::RecoverPoolLedger { .. } => 1,
            CurrentFleetProtocolAction::JoinRoot { .. } => 2,
            CurrentFleetProtocolAction::SynchronizeRegistry { .. } => 3,
            CurrentFleetProtocolAction::ActivateRegistry { .. } => 4,
            CurrentFleetProtocolAction::ActivateRegistryMirror { .. } => 5,
            CurrentFleetProtocolAction::PrepareComponentRegistry { .. } => 6,
            CurrentFleetProtocolAction::ProvisionComponents { .. } => 7,
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
        operator: Principal,
    ) {
        match &step.action {
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
            CurrentFleetProtocolAction::PrepareStoreChunkSet { request } => {
                store_prepare_as(pic, step.target, operator, request.clone())
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
            CurrentFleetProtocolAction::RecoverPoolLedger { request } => {
                let response = root_command(
                    pic,
                    step.target,
                    RootCommandFragment::RecoverPoolLedger(request.clone()),
                )
                .expect("recover current pool Ledger balance");
                assert!(matches!(
                    response,
                    RootCommandResponseFragment::RecoverPoolLedger(_)
                ));
            }
            CurrentFleetProtocolAction::PublishStoreChunk { request } => {
                let response: Result<(), Error> = pic
                    .update_candid_as(
                        step.target,
                        operator,
                        canic::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
                        (request.clone(),),
                    )
                    .expect("publish current Store chunk transport");
                response.expect("publish current Store chunk");
            }
            CurrentFleetProtocolAction::StageStoreManifest { request } => {
                store_stage_manifest_as(pic, step.target, operator, request.clone())
                    .expect("stage current Store manifest");
            }
        }
    }

    #[cfg(test)]
    fn await_current_protocol_step(
        pic: &PocketIc,
        step: &CompiledCurrentProtocolStep,
        operator: Principal,
    ) {
        for _ in 0..160 {
            if current_protocol_step_is_terminal(pic, step, operator) {
                return;
            }
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        if let CurrentFleetProtocolAction::ProvisionComponents { request, .. } = &step.action {
            let status = coordinator_status(
                pic,
                step.target,
                CoordinatorStatusRequest::Operation(OperationStatusRequest {
                    operation_id: request.operation_id,
                }),
            );
            let detail = match status {
                Ok(CoordinatorStatusResponse::Operation(
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
        operator: Principal,
    ) -> bool {
        match &step.action {
            CurrentFleetProtocolAction::ActivateRegistry {
                expected_registry, ..
            }
            | CurrentFleetProtocolAction::JoinRoot {
                expected_registry, ..
            } => matches!(
                coordinator_status(pic, step.target, CoordinatorStatusRequest::Registry),
                Ok(CoordinatorStatusResponse::Registry(observed)) if observed == *expected_registry
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
            CurrentFleetProtocolAction::PrepareStoreChunkSet { request } => {
                let status = current_store_staging_status(
                    pic,
                    step.target,
                    operator,
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
                    CoordinatorStatusRequest::Operation(OperationStatusRequest {
                        operation_id: request.operation_id,
                    }),
                ),
                Ok(CoordinatorStatusResponse::Operation(
                    CoordinatorOperationStatusResponse::ComponentProvisioning(observed)
                )) if observed.operation_id == request.operation_id
                    && observed.plan_hash == *plan_hash
                    && observed.phase == FleetComponentProvisioningPhase::RuntimesActivated
                    && observed.published_fleet_registry.is_some()
                    && observed.pending_root_failure.is_none()
            ),
            CurrentFleetProtocolAction::RecoverPoolLedger { request } => matches!(
                root_status(
                    pic,
                    step.target,
                    RootStatusRequestFragment::Operation(OperationStatusRequest {
                        operation_id: request.operation_id,
                    }),
                ),
                Ok(RootStatusResponseFragment::Operation(
                    RootOperationStatusResponse::RecoverPoolLedger(observed)
                )) if observed.request == *request
                    && observed.phase == PoolLedgerRecoveryPhase::Complete
                    && observed.receipt.is_some()
            ),
            CurrentFleetProtocolAction::PublishStoreChunk { request } => {
                let status = current_store_staging_status(
                    pic,
                    step.target,
                    operator,
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
                    operator,
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
        let response: Result<StoreStatusResponse, Error> = pic
            .query_candid_as(
                store,
                caller,
                canic::protocol::CANIC_STATUS,
                (StoreStatusRequest::Template(TemplateLookupRequest {
                    template_id: template_id.clone(),
                    version: version.clone(),
                }),),
            )
            .expect("query current Store staging transport");
        let StoreStatusResponse::Template(status) = response.expect("query current Store staging")
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

        let CoordinatorStatusResponse::Funding(coordinator) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorStatusRequest::Funding,
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

        let CoordinatorStatusResponse::Funding(coordinator) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorStatusRequest::Funding,
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
        let before_icp = real_icp_balance(&fixture.pic, ledger, fixture.root);
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
            real_icp_balance(&fixture.pic, ledger, fixture.root),
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
        let before_icp = real_icp_balance(&fixture.pic, ledger, fixture.root);
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
            real_icp_balance(&fixture.pic, ledger, fixture.root),
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
        let before_icp = real_icp_balance(&fixture.pic, ledger, fixture.root);
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
            real_icp_balance(&fixture.pic, ledger, fixture.root),
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
        let CoordinatorStatusResponse::RegistryVersion(mut version) =
            coordinator_status(pic, coordinator, CoordinatorStatusRequest::RegistryVersion)
                .expect("query multi-Root Registry genesis")
        else {
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
    fn real_icp_balance(pic: &PocketIc, ledger: Principal, owner: Principal) -> Nat {
        pic.query_candid(
            ledger,
            "icrc1_balance_of",
            (QualificationIcrc1Account {
                owner,
                subaccount: None,
            },),
        )
        .expect("query production ICP Ledger balance")
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
        let CoordinatorStatusResponse::Funding(status) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorStatusRequest::Funding,
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
        let CoordinatorStatusResponse::RegistryVersion(predecessor_registry) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorStatusRequest::RegistryVersion,
        )
        .expect("query predecessor Registry version") else {
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
            let CoordinatorStatusResponse::Operation(
                CoordinatorOperationStatusResponse::FundingPolicyRotation(status),
            ) = coordinator_status(
                &fixture.pic,
                fixture.coordinator,
                CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
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
    fn assert_mainnet_refill(first_response_pending: bool, expected_request_count: u64) {
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
                        canister_ids: vec![asset],
                        expected_root: root,
                        expected_subnet: root_subnet,
                        pending_first_index: first_response_pending.then_some(0),
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
        assert_eq!(request_count, expected_request_count);
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
                expected_root: root,
                expected_subnet: subnet,
                pending_first_index: Some(0),
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
        build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &[QUALIFICATION_WORKLOAD_PACKAGE],
            CanicWasmBuildProfile::Fast,
            &[],
        );
        read_wasm(
            &target_dir,
            QUALIFICATION_WORKLOAD_PACKAGE,
            CanicWasmBuildProfile::Fast.target_dir_name(),
        )
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
        clippy::too_many_lines,
        reason = "one complete real-Fleet add/remove journey"
    )]
    fn fleet_admission_add_and_remove_converge_across_real_root_and_components() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let fixture = acquire_active_component_registry();
        let pic = fixture.pic();
        let added = Principal::self_authenticating([0xd1; 32]);

        let CoordinatorStatusResponse::Registry(initial_registry) =
            coordinator_status(pic, fixture.coordinator, CoordinatorStatusRequest::Registry)
                .expect("query initial Fleet Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status")
        };
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

        let CoordinatorStatusResponse::Registry(added_registry) =
            coordinator_status(pic, fixture.coordinator, CoordinatorStatusRequest::Registry)
                .expect("query added Fleet Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status")
        };
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

        let CoordinatorStatusResponse::Registry(removed_registry) =
            coordinator_status(pic, fixture.coordinator, CoordinatorStatusRequest::Registry)
                .expect("query removed Fleet Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status")
        };
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

        let CoordinatorStatusResponse::Registry(initial_registry) =
            coordinator_status(pic, fixture.coordinator, CoordinatorStatusRequest::Registry)
                .expect("query initial Fleet Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status")
        };
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
        let CoordinatorStatusResponse::Operation(CoordinatorOperationStatusResponse::Admission(
            blocked,
        )) = coordinator_status(
            pic,
            fixture.coordinator,
            CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
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
        let CoordinatorStatusResponse::Registry(blocked_registry) =
            coordinator_status(pic, fixture.coordinator, CoordinatorStatusRequest::Registry)
                .expect("query blocked Fleet Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status")
        };
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
        let rejected = root_command(
            pic,
            fixture.root,
            RootCommandFragment::ProvisionComponent(allocation_request.clone()),
        )
        .expect_err("active admission transition must fence Component allocation");
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

        let CoordinatorStatusResponse::Registry(initial_registry) =
            coordinator_status(pic, fixture.coordinator, CoordinatorStatusRequest::Registry)
                .expect("query initial Fleet Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status")
        };
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

        let CoordinatorStatusResponse::Registry(unchanged_registry) =
            coordinator_status(pic, fixture.coordinator, CoordinatorStatusRequest::Registry)
                .expect("query Registry after stale-catalog release")
        else {
            panic!("Coordinator returned a differently correlated Registry status")
        };
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
        let CoordinatorStatusResponse::Registry(initial) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorStatusRequest::Registry,
        )
        .expect("query initial two-Root Registry") else {
            panic!("Coordinator returned a differently correlated Registry status")
        };
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

        let CoordinatorStatusResponse::Registry(added_registry) = coordinator_status(
            &fixture.pic,
            fixture.coordinator,
            CoordinatorStatusRequest::Registry,
        )
        .expect("query added two-Root Registry") else {
            panic!("Coordinator returned a differently correlated Registry status")
        };
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
    #[expect(
        clippy::significant_drop_tightening,
        clippy::too_many_lines,
        reason = "the pooled Fleet fixture lease and complete autonomous deletion journey stay together"
    )]
    fn published_draining_root_autonomously_reaches_external_deletion_readiness() {
        let _unit_test_serial = crate::pic::acquire_pic_unit_test_serial_guard();
        // Root deletion is deliberately outside the pooled baseline's reset
        // contract: PocketIC cannot restore a captured snapshot after the
        // target canister has been deleted. Keep this destructive journey on
        // an exclusively owned instance so it cannot invalidate the warm
        // baseline used by reset-complete cases.
        let fixture = setup_fresh_active_component_registry();
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
        let mut progress_transitions = 0_u16;
        let mut stagnant_seconds = 0_u16;
        for _ in 0..ROOT_REMOVAL_MAX_SIMULATED_SECONDS {
            for _ in 0..ROOT_REMOVAL_TICKS_PER_SECOND {
                fixture.pic().tick();
            }
            if let Ok(RootStatusResponseFragment::Operation(
                RootOperationStatusResponse::RemoveRoot(status),
            )) = root_status(
                fixture.pic(),
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
            fixture.pic().advance_time(Duration::from_secs(1));
        }
        let terminal = terminal.unwrap_or_else(|| {
            report_canister_diagnostics(
                fixture.pic(),
                fixture.root,
                Principal::anonymous(),
                "autonomous Root removal timeout",
            );
            report_canister_diagnostics(
                fixture.pic(),
                fixture.coordinator,
                Principal::anonymous(),
                "autonomous Coordinator Root-removal timeout",
            );
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
        let pic = build_pic();
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
        let CoordinatorStatusResponse::Registry(registry) =
            coordinator_status(pic, coordinator, CoordinatorStatusRequest::Registry)
                .expect("query active Registry")
        else {
            panic!("Coordinator returned a differently correlated Registry status");
        };
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

    fn install_current_root_with_config_and_pool_setup<F>(
        pic: &PocketIc,
        root_wasm: Vec<u8>,
        coordinator: Principal,
        store_fixture: RootStoreFixture,
        placement: BootstrappedRootPlacement,
        config_path: &Path,
        pool_setup: F,
    ) -> InstalledRootFixture
    where
        F: FnOnce(&PocketIc, Principal) -> Vec<Principal>,
    {
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
        if let Some(component_admission_limits) = placement.component_admission_limits {
            for admission in &mut init_args.authority.binding.component_admissions {
                admission.maximum_root_instances = match &component_admission_limits {
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
        init_args.canister_pool_imports = pool_setup(pic, root_id);
        let store_init_args = FleetSubnetWasmStoreInitArgs {
            authority: init_args.authority.wasm_store_authority.clone(),
            install_id: init_args.wasm_store_activation.operation_id,
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
        InstalledRootFixture {
            root_id,
            init_args,
            coordinator_root_funding: placement
                .coordinator_root_funding
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
        reset_prepaid_pool_assets_for_count(pic, root, PREPAID_POOL_ASSET_COUNT);
    }

    fn reset_prepaid_pool_assets_for_count(pic: &PocketIc, root: Principal, count: usize) {
        for _ in 0..count {
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
        let (manifest, artifacts) = exact_root_store_fixture(config_path, component_wasms);
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
        let declared_package = &config
            .roles
            .get(role)
            .expect("fixture role declaration")
            .package;
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

    #[cfg(test)]
    fn build_toko_shaped_singleton_component_wasms() -> &'static BTreeMap<CanisterRole, Vec<u8>> {
        static WASMS: OnceLock<BTreeMap<CanisterRole, Vec<u8>>> = OnceLock::new();
        WASMS.get_or_init(|| {
            let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
            let config_path = toko_shaped_singleton_root_canister_config_path(&workspace_root);
            build_component_fixture_wasms(
                &workspace_root,
                &config_path,
                "fleet-registry-toko-shaped-singleton",
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
                "prepared mainnet Root automatic refill",
                prepared_mainnet_root_automatically_refills_one_exact_pool_asset,
            ),
            (
                "uncertain mainnet refill replay",
                uncertain_mainnet_refill_reuses_the_exact_paid_request,
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
                "post-bootstrap recovery helper Store publication",
                current_store_stages_recovery_helper_after_root_bootstrap_and_replays_zero_effects,
            ),
            (
                "fresh provisioning terminal runtime activation",
                fresh_five_component_provisioning_reaches_runtime_active_and_publishes_catalog,
            ),
            (
                "literal zero-estate host/control-plane convergence",
                literal_zero_estate_reaches_one_workload_and_one_ready_pool_asset,
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
}

pub use tests::{
    ActiveComponentRegistryFixture, setup_active_component_registry,
    setup_fresh_active_component_registry,
};
