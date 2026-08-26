//! Exact host-boundary canister for the retained Root repair PocketIC journey.
//!
//! This fixture preserves one Root authority and imported pool observation across upgrade. It
//! deliberately implements only the protected calls exercised by the host repair procedure.

use candid::{CandidType, Nat, Principal};
use canic_control_plane::dto::root::{
    RootOperationStatusResponse, RootRegistrySynchronizationOperationStatus,
};
use canic_core::{
    cdk::types::Cycles,
    diagnostics::codes::{AUTHORITY_UNAUTHORIZED, PLATFORM_FAILED},
    dto::{
        component_registry::{
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
        },
        error::Error,
        fleet_registry::{
            FleetRegistry, FleetRegistryActivationResponse, FleetRegistryManifest,
            FleetRegistryVersion, FleetSubnetRootJoinResponse,
            FleetSubnetRootSnapshotAcknowledgement,
        },
        fleet_subnet_root::FleetSubnetRootAuthority,
        pool::{
            CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolAssetStatus,
            CanisterPoolResponse, CanisterPoolStatusRequest, PoolCanisterRequest,
            PoolImportResponse,
        },
        role::{OperationReceipt, OperationStatusRequest},
        root_store::RootStoreBootstrapResponse,
    },
};
use serde::Deserialize;
use std::cell::RefCell;

#[derive(CandidType, Clone, Deserialize)]
struct RepairStubInit {
    authority: FleetSubnetRootAuthority,
    pool_canister: Principal,
    pool_cycles: u128,
    component_registry: RootComponentRegistryStatusResponse,
    store_bootstrap_operation_id: [u8; 32],
    registry_sync_operation_id: [u8; 32],
    store_bootstrap: RootStoreBootstrapResponse,
    registry_synchronization: RootRegistrySynchronizationOperationStatus,
    joining_registry: FleetRegistry,
    active_registry: FleetRegistry,
    joining_manifest: FleetRegistryManifest,
    active_manifest: FleetRegistryManifest,
    joining_version: FleetRegistryVersion,
    active_version: FleetRegistryVersion,
    join_response: FleetSubnetRootJoinResponse,
    root_acknowledgements: Vec<FleetSubnetRootSnapshotAcknowledgement>,
}

#[derive(CandidType, Clone, Deserialize)]
struct RepairStubState {
    authority: FleetSubnetRootAuthority,
    pool_canister: Principal,
    pool_cycles: u128,
    component_registry: RootComponentRegistryStatusResponse,
    store_bootstrap_operation_id: [u8; 32],
    registry_sync_operation_id: [u8; 32],
    store_bootstrap: RootStoreBootstrapResponse,
    registry_synchronization: RootRegistrySynchronizationOperationStatus,
    joining_registry: FleetRegistry,
    active_registry: FleetRegistry,
    joining_manifest: FleetRegistryManifest,
    active_manifest: FleetRegistryManifest,
    joining_version: FleetRegistryVersion,
    active_version: FleetRegistryVersion,
    join_response: FleetSubnetRootJoinResponse,
    root_acknowledgements: Vec<FleetSubnetRootSnapshotAcknowledgement>,
    registry_active: bool,
}

thread_local! {
    static STATE: RefCell<Option<RepairStubState>> = const { RefCell::new(None) };
}

#[derive(CandidType, Deserialize)]
enum StubCommand {
    ImportPoolCanister(PoolCanisterRequest),
    PrepareComponentRegistry(Box<RootComponentRegistryPreparationRequest>),
    JoinRoot(Box<canic_core::dto::fleet_registry::FleetSubnetRootJoinRequest>),
    ActivateRegistry(canic_core::dto::fleet_registry::FleetRegistryActivationRequest),
    SynchronizeRegistry(canic_core::dto::fleet_registry::FleetSubnetRootRegistrySyncRequest),
}

#[derive(CandidType, Deserialize)]
enum StubCommandResponse {
    ImportPoolCanister(PoolImportResponse),
    PrepareComponentRegistry(Box<RootComponentRegistryStatusResponse>),
    JoinRoot(Box<FleetSubnetRootJoinResponse>),
    ActivateRegistry(Box<FleetRegistryActivationResponse>),
    OperationAccepted(OperationReceipt),
}

#[derive(CandidType, Deserialize)]
enum StubStatusRequest {
    FleetAuthority,
    Pool(CanisterPoolStatusRequest),
    Operation(OperationStatusRequest),
    Registry,
    RegistryManifest,
    RegistryVersion,
    RootAcknowledgements,
}

#[derive(CandidType, Deserialize)]
enum StubStatusResponse {
    FleetAuthority(Box<FleetSubnetRootAuthority>),
    Pool(Box<CanisterPoolResponse>),
    Operation(Box<RootOperationStatusResponse>),
    Registry(Box<FleetRegistry>),
    RegistryManifest(FleetRegistryManifest),
    RegistryVersion(FleetRegistryVersion),
    RootAcknowledgements(Vec<FleetSubnetRootSnapshotAcknowledgement>),
}

#[derive(CandidType)]
struct CanisterStatusArgs {
    canister_id: Principal,
}

#[derive(CandidType, Deserialize)]
struct CanisterStatusResult {
    settings: CanisterStatusSettings,
    module_hash: Option<Vec<u8>>,
    cycles: Nat,
}

#[derive(CandidType, Deserialize)]
struct CanisterStatusSettings {
    controllers: Vec<Principal>,
}

#[ic_cdk::init]
fn init(args: RepairStubInit) {
    STATE.with_borrow_mut(|state| {
        *state = Some(RepairStubState {
            authority: args.authority,
            pool_canister: args.pool_canister,
            pool_cycles: args.pool_cycles,
            component_registry: args.component_registry,
            store_bootstrap_operation_id: args.store_bootstrap_operation_id,
            registry_sync_operation_id: args.registry_sync_operation_id,
            store_bootstrap: args.store_bootstrap,
            registry_synchronization: args.registry_synchronization,
            joining_registry: args.joining_registry,
            active_registry: args.active_registry,
            joining_manifest: args.joining_manifest,
            active_manifest: args.active_manifest,
            joining_version: args.joining_version,
            active_version: args.active_version,
            join_response: args.join_response,
            root_acknowledgements: args.root_acknowledgements,
            registry_active: false,
        });
    });
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    STATE.with_borrow(|state| {
        ic_cdk::storage::stable_save((state.as_ref().expect("repair stub initialized"),))
            .expect("retain repair stub state");
    });
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let (state,): (RepairStubState,) =
        ic_cdk::storage::stable_restore().expect("restore repair stub state");
    STATE.with_borrow_mut(|current| *current = Some(state));
}

#[ic_cdk::query]
fn canic_status(request: StubStatusRequest) -> Result<StubStatusResponse, Error> {
    require_controller()?;
    STATE.with_borrow(|state| {
        let state = state.as_ref().expect("repair stub initialized");
        Ok(match request {
            StubStatusRequest::FleetAuthority => {
                StubStatusResponse::FleetAuthority(Box::new(state.authority.clone()))
            }
            StubStatusRequest::Pool(request) => {
                let entries = request
                    .start_after
                    .is_none_or(|start_after| state.pool_canister > start_after)
                    .then(|| pool_asset(state))
                    .into_iter()
                    .take(usize::from(request.limit))
                    .collect();
                StubStatusResponse::Pool(Box::new(CanisterPoolResponse {
                    config: state.authority.binding.limits.canister_pool.clone(),
                    tracked: 1,
                    store: 0,
                    store_deletion_pending: 0,
                    pooled: 1,
                    workload: 0,
                    surplus: 0,
                    ready: 1,
                    pending_reset: 0,
                    claimed: 0,
                    recycling: 0,
                    handing_off: 0,
                    failed: 0,
                    completed_handoffs: 0,
                    pending_creation: None,
                    pending_handoff: None,
                    entries,
                    next_start_after: None,
                }))
            }
            StubStatusRequest::Operation(request) => {
                if request.operation_id == state.store_bootstrap_operation_id {
                    StubStatusResponse::Operation(Box::new(
                        RootOperationStatusResponse::BootstrapStore(state.store_bootstrap.clone()),
                    ))
                } else if request.operation_id == state.registry_sync_operation_id {
                    StubStatusResponse::Operation(Box::new(
                        RootOperationStatusResponse::SynchronizeRegistry(
                            state.registry_synchronization.clone(),
                        ),
                    ))
                } else {
                    return Err(Error::from_registered(PLATFORM_FAILED));
                }
            }
            StubStatusRequest::Registry => StubStatusResponse::Registry(Box::new(
                if state.registry_active {
                    &state.active_registry
                } else {
                    &state.joining_registry
                }
                .clone(),
            )),
            StubStatusRequest::RegistryManifest => StubStatusResponse::RegistryManifest(
                if state.registry_active {
                    &state.active_manifest
                } else {
                    &state.joining_manifest
                }
                .clone(),
            ),
            StubStatusRequest::RegistryVersion => StubStatusResponse::RegistryVersion(
                if state.registry_active {
                    &state.active_version
                } else {
                    &state.joining_version
                }
                .clone(),
            ),
            StubStatusRequest::RootAcknowledgements => {
                StubStatusResponse::RootAcknowledgements(state.root_acknowledgements.clone())
            }
        })
    })
}

#[ic_cdk::update]
async fn canic_command(command: StubCommand) -> Result<StubCommandResponse, Error> {
    require_controller()?;
    match command {
        StubCommand::PrepareComponentRegistry(_) => STATE.with_borrow(|state| {
            Ok(StubCommandResponse::PrepareComponentRegistry(Box::new(
                state
                    .as_ref()
                    .expect("repair stub initialized")
                    .component_registry
                    .clone(),
            )))
        }),
        StubCommand::ImportPoolCanister(request) => {
            let expected = STATE.with_borrow(|state| {
                state
                    .as_ref()
                    .expect("repair stub initialized")
                    .pool_canister
            });
            if request.canister_id != expected {
                return Err(Error::from_registered(PLATFORM_FAILED));
            }
            let response = ic_cdk::call::Call::bounded_wait(
                Principal::management_canister(),
                "canister_status",
            )
            .with_arg(&CanisterStatusArgs {
                canister_id: request.canister_id,
            })
            .await
            .map_err(|_| Error::from_registered(PLATFORM_FAILED))?;
            let status: CanisterStatusResult = response
                .candid()
                .map_err(|_| Error::from_registered(PLATFORM_FAILED))?;
            let valid_empty_import = status.module_hash.is_none()
                && status.settings.controllers == [ic_cdk::api::canister_self()];
            if !valid_empty_import {
                return Err(Error::from_registered(PLATFORM_FAILED));
            }
            let cycles = u128::try_from(status.cycles.0)
                .map_err(|_| Error::from_registered(PLATFORM_FAILED))?;
            STATE.with_borrow_mut(|state| {
                state.as_mut().expect("repair stub initialized").pool_cycles = cycles;
            });
            Ok(StubCommandResponse::ImportPoolCanister(
                PoolImportResponse::Imported {
                    canister_id: request.canister_id,
                },
            ))
        }
        StubCommand::JoinRoot(request) => STATE.with_borrow(|state| {
            let state = state.as_ref().expect("repair stub initialized");
            if request.entry != state.join_response.entry {
                return Err(Error::from_registered(PLATFORM_FAILED));
            }
            Ok(StubCommandResponse::JoinRoot(Box::new(
                state.join_response.clone(),
            )))
        }),
        StubCommand::ActivateRegistry(request) => STATE.with_borrow_mut(|state| {
            let state = state.as_mut().expect("repair stub initialized");
            if request.expected_registry != state.joining_version {
                return Err(Error::from_registered(PLATFORM_FAILED));
            }
            state.registry_active = true;
            Ok(StubCommandResponse::ActivateRegistry(Box::new(
                FleetRegistryActivationResponse {
                    previous_version: state.joining_version.clone(),
                    version: state.active_version.clone(),
                },
            )))
        }),
        StubCommand::SynchronizeRegistry(request) => {
            Ok(StubCommandResponse::OperationAccepted(OperationReceipt {
                operation_id: request.operation_id,
            }))
        }
    }
}

fn require_controller() -> Result<(), Error> {
    if ic_cdk::api::is_controller(&ic_cdk::api::msg_caller()) {
        Ok(())
    } else {
        Err(Error::from_registered(AUTHORITY_UNAUTHORIZED))
    }
}

fn pool_asset(state: &RepairStubState) -> CanisterPoolAsset {
    CanisterPoolAsset {
        canister_id: state.pool_canister,
        cycles: Cycles::new(state.pool_cycles),
        origin: CanisterPoolAssetOrigin::Imported,
        status: CanisterPoolAssetStatus::Ready,
        added_at_ns: 1,
        updated_at_ns: ic_cdk::api::time(),
    }
}

ic_cdk::export_candid!();
