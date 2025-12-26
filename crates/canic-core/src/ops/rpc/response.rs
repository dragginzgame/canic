//! Root-side handlers that fulfil orchestration requests.
//!
//! The root canister exposes `canic_response`, which accepts a
//! [`Request`](crate::ops::request::Request) and returns a [`Response`]. This
//! module contains the implementations for create/upgrade/cycle flows plus the
//! corresponding response payloads.

use crate::{
    Error,
    log::Topic,
    ops::{
        directory::SubnetDirectoryOps,
        ic::deposit_cycles,
        orchestration::orchestrator::{CanisterLifecycleOrchestrator, LifecycleEvent},
        prelude::*,
        rpc::{
            CreateCanisterParent, CreateCanisterRequest, CyclesRequest, Request, RequestOpsError,
            UpgradeCanisterRequest,
        },
        topology::subnet::SubnetCanisterRegistryOps,
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

    let result: Result<Response, Error> = async {
        // Look up parent
        let parent_pid = match &req.parent {
            CreateCanisterParent::Canister(pid) => *pid,
            CreateCanisterParent::Root => canister_self(),
            CreateCanisterParent::ThisCanister => caller,

            CreateCanisterParent::Parent => SubnetCanisterRegistryOps::get_parent(caller)
                .ok_or(RequestOpsError::ParentNotFound(caller))?,

            CreateCanisterParent::Directory(role) => SubnetDirectoryOps::get(role)
                .ok_or_else(|| RequestOpsError::CanisterRoleNotFound(role.clone()))?,
        };

        let event = LifecycleEvent::Create {
            role: req.canister_role.clone(),
            parent: parent_pid,
            extra_arg: req.extra_arg.clone(),
        };

        let result = CanisterLifecycleOrchestrator::apply(event).await?;
        let new_canister_pid = result
            .new_canister_pid
            .ok_or(RequestOpsError::MissingNewCanisterPid)?;

        Ok(Response::CreateCanister(CreateCanisterResponse {
            new_canister_pid,
        }))
    }
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

    let registry_entry = SubnetCanisterRegistryOps::get(req.canister_pid)
        .ok_or(RequestOpsError::ChildNotFound(req.canister_pid))?;

    if registry_entry.parent_pid != Some(caller) {
        return Err(RequestOpsError::NotChildOfCaller(req.canister_pid, caller).into());
    }

    let event = LifecycleEvent::Upgrade {
        pid: registry_entry.pid,
    };

    CanisterLifecycleOrchestrator::apply(event).await?;

    Ok(Response::UpgradeCanister(UpgradeCanisterResponse {}))
}

// cycles_response
async fn cycles_response(req: &CyclesRequest) -> Result<Response, Error> {
    deposit_cycles(msg_caller(), req.cycles).await?;

    let cycles_transferred = req.cycles;

    Ok(Response::Cycles(CyclesResponse { cycles_transferred }))
}
