//! Exact host-boundary canister for the retained Root repair PocketIC journey.
//!
//! This fixture preserves one Root authority and imported pool observation across upgrade. It
//! deliberately implements only the protected calls exercised by the host repair procedure.

use candid::{CandidType, Nat, Principal};
use canic_core::{
    cdk::types::Cycles,
    diagnostics::codes::{AUTHORITY_UNAUTHORIZED, PLATFORM_FAILED},
    dto::{
        component_registry::{
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
        },
        error::Error,
        fleet_subnet_root::FleetSubnetRootAuthority,
        pool::{
            CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolAssetStatus,
            CanisterPoolResponse, CanisterPoolStatusRequest, PoolCanisterRequest,
            PoolImportResponse,
        },
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
}

#[derive(CandidType, Clone, Deserialize)]
struct RepairStubState {
    authority: FleetSubnetRootAuthority,
    pool_canister: Principal,
    pool_cycles: u128,
    component_registry: RootComponentRegistryStatusResponse,
}

thread_local! {
    static STATE: RefCell<Option<RepairStubState>> = const { RefCell::new(None) };
}

#[derive(CandidType, Deserialize)]
enum RootCommand {
    ImportPoolCanister(PoolCanisterRequest),
    PrepareComponentRegistry(Box<RootComponentRegistryPreparationRequest>),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponse {
    ImportPoolCanister(PoolImportResponse),
    PrepareComponentRegistry(Box<RootComponentRegistryStatusResponse>),
}

#[derive(CandidType, Deserialize)]
enum RootStatusRequest {
    FleetAuthority,
    Pool(CanisterPoolStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RootStatusResponse {
    FleetAuthority(Box<FleetSubnetRootAuthority>),
    Pool(Box<CanisterPoolResponse>),
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
fn canic_status(request: RootStatusRequest) -> Result<RootStatusResponse, Error> {
    require_controller()?;
    STATE.with_borrow(|state| {
        let state = state.as_ref().expect("repair stub initialized");
        Ok(match request {
            RootStatusRequest::FleetAuthority => {
                RootStatusResponse::FleetAuthority(Box::new(state.authority.clone()))
            }
            RootStatusRequest::Pool(request) => {
                let entries = request
                    .start_after
                    .is_none_or(|start_after| state.pool_canister > start_after)
                    .then(|| pool_asset(state))
                    .into_iter()
                    .take(usize::from(request.limit))
                    .collect();
                RootStatusResponse::Pool(Box::new(CanisterPoolResponse {
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
        })
    })
}

#[ic_cdk::update]
async fn canic_command(command: RootCommand) -> Result<RootCommandResponse, Error> {
    require_controller()?;
    match command {
        RootCommand::PrepareComponentRegistry(_) => STATE.with_borrow(|state| {
            Ok(RootCommandResponse::PrepareComponentRegistry(Box::new(
                state
                    .as_ref()
                    .expect("repair stub initialized")
                    .component_registry
                    .clone(),
            )))
        }),
        RootCommand::ImportPoolCanister(request) => {
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
            Ok(RootCommandResponse::ImportPoolCanister(
                PoolImportResponse::Imported {
                    canister_id: request.canister_id,
                },
            ))
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
