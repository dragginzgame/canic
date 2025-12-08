//! Root-side handlers that fulfil orchestration requests.
//!
//! The root canister exposes `canic_response`, which accepts a
//! [`Request`](crate::ops::request::Request) and returns a [`Response`]. This
//! module contains the implementations for create/upgrade/cycle flows plus the
//! corresponding response payloads.

use crate::{
    Error,
    interface::ic::{canister::upgrade_canister, deposit_cycles},
    log::Topic,
    ops::{
        canister::create_and_install_canister,
        model::memory::topology::subnet::SubnetCanisterRegistryOps,
        prelude::*,
        request::{
            CreateCanisterParent, CreateCanisterRequest, CyclesRequest, Request, RequestOpsError,
            UpgradeCanisterRequest,
        },
        wasm::WasmOps,
    },
};

///
/// Response
/// Response payloads produced by root for orchestration requests.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Response {
    CreateCanister(CreateCanisterResponse),
    UpgradeCanister(UpgradeCanisterResponse),
    Cycles(CyclesResponse),
}

///
/// CreateCanisterResponse
/// Result of creating and installing a new canister.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterResponse {
    pub new_canister_pid: Principal,
}

///
/// UpgradeCanisterResponse
/// Result of an upgrade request (currently empty, reserved for metadata)
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterResponse {}

///
/// CyclesResponse
/// Result of transferring cycles to a child canister
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesResponse {
    pub cycles_transferred: u128,
}

/// Handle a root-bound orchestration request and produce a [`Response`].
pub async fn response(req: Request) -> Result<Response, Error> {
    OpsError::require_root()?;

    match req {
        Request::CreateCanister(req) => create_canister_response(&req).await,
        Request::UpgradeCanister(req) => upgrade_canister_response(&req).await,
        Request::Cycles(req) => cycles_response(&req).await,
    }
}

// create_canister_response
async fn create_canister_response(req: &CreateCanisterRequest) -> Result<Response, Error> {
    let caller = msg_caller();
    let role = req.canister_role.clone();
    let parent_desc = format!("{:?}", &req.parent);

    let result: Result<Response, Error> = (|| async {
        // Look up parent
        let parent_pid = match &req.parent {
            CreateCanisterParent::Canister(pid) => *pid,
            CreateCanisterParent::Root => canister_self(),
            CreateCanisterParent::ThisCanister => caller,

            CreateCanisterParent::Parent => SubnetCanisterRegistryOps::try_get_parent(caller)
                .map_err(|_| RequestOpsError::ParentNotFound(caller))?,

            CreateCanisterParent::Directory(ty) => {
                SubnetCanisterRegistryOps::try_get_type(ty)
                    .map_err(|_| RequestOpsError::CanisterRoleNotFound(ty.clone()))?
                    .pid
            }
        };

        let new_canister_pid =
            create_and_install_canister(&req.canister_role, parent_pid, req.extra_arg.clone())
                .await?;

        Ok(Response::CreateCanister(CreateCanisterResponse {
            new_canister_pid,
        }))
    })()
    .await;

    if let Err(err) = &result {
        log!(
            Topic::CanisterLifecycle,
            Warn,
            "create_canister_response failed (caller={caller}, role={role}, parent={parent_desc}): {err}"
        );
    }

    result
}

// upgrade_canister_response
async fn upgrade_canister_response(req: &UpgradeCanisterRequest) -> Result<Response, Error> {
    let caller = msg_caller();
    let registry_entry = SubnetCanisterRegistryOps::try_get(req.canister_pid)
        .map_err(|_| RequestOpsError::ChildNotFound(req.canister_pid))?;

    if registry_entry.parent_pid != Some(caller) {
        return Err(RequestOpsError::NotChildOfCaller(req.canister_pid, caller).into());
    }

    // Use the registry's type to avoid trusting request payload.
    let wasm = WasmOps::try_get(&registry_entry.ty)?;
    upgrade_canister(registry_entry.pid, wasm.bytes()).await?;
    SubnetCanisterRegistryOps::update_module_hash(registry_entry.pid, wasm.module_hash())?;

    Ok(Response::UpgradeCanister(UpgradeCanisterResponse {}))
}

// cycles_response
async fn cycles_response(req: &CyclesRequest) -> Result<Response, Error> {
    deposit_cycles(msg_caller(), req.cycles).await?;

    let cycles_transferred = req.cycles;

    Ok(Response::Cycles(CyclesResponse { cycles_transferred }))
}
