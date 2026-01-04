use crate::{
    Error,
    access::env,
    dto::rpc::{
        CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse, CyclesRequest,
        CyclesResponse, Request, Response, UpgradeCanisterRequest, UpgradeCanisterResponse,
    },
    ops::{
        ic::mgmt::deposit_cycles,
        rpc::request::RequestOpsError,
        storage::{directory::subnet::SubnetDirectoryOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::{
        lifecycle::{LifecycleEvent, orchestrator::LifecycleOrchestrator},
        prelude::*,
    },
};

/// Handle a root-bound orchestration request and produce a [`Response`].
pub async fn response(req: Request) -> Result<Response, Error> {
    env::require_root()?;

    match req {
        Request::CreateCanister(req) => create_canister_response(&req).await,
        Request::UpgradeCanister(req) => upgrade_canister_response(&req).await,
        Request::Cycles(req) => cycles_response(&req).await,
    }
}

// create_canister_response
async fn create_canister_response(req: &CreateCanisterRequest) -> Result<Response, Error> {
    env::require_root()?;

    let caller = msg_caller();
    let role = req.canister_role.clone();
    let parent_desc = format!("{:?}", &req.parent);

    let result: Result<Response, Error> = async {
        // Look up parent
        let parent_pid = match &req.parent {
            CreateCanisterParent::Canister(pid) => *pid,
            CreateCanisterParent::Root => canister_self(),
            CreateCanisterParent::ThisCanister => caller,

            CreateCanisterParent::Parent => SubnetRegistryOps::get_parent(caller)
                .ok_or(RequestOpsError::ParentNotFound(caller))?,

            CreateCanisterParent::Directory(role) => SubnetDirectoryOps::get(role)
                .ok_or_else(|| RequestOpsError::CanisterRoleNotFound(role.clone()))?,
        };

        let event = LifecycleEvent::Create {
            role: req.canister_role.clone(),
            parent: parent_pid,
            extra_arg: req.extra_arg.clone(),
        };

        let lifecycle_result = LifecycleOrchestrator::apply(event).await?;
        let new_canister_pid = lifecycle_result
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
    env::require_root()?;

    let caller = msg_caller();
    let registry_entry = SubnetRegistryOps::get(req.canister_pid)
        .ok_or(RequestOpsError::ChildNotFound(req.canister_pid))?;

    if registry_entry.parent_pid != Some(caller) {
        return Err(RequestOpsError::NotChildOfCaller(req.canister_pid, caller).into());
    }

    let event = LifecycleEvent::Upgrade {
        pid: req.canister_pid,
    };

    LifecycleOrchestrator::apply(event).await?;

    Ok(Response::UpgradeCanister(UpgradeCanisterResponse {}))
}

// cycles_response
async fn cycles_response(req: &CyclesRequest) -> Result<Response, Error> {
    env::require_root()?;

    deposit_cycles(msg_caller(), req.cycles).await?;

    let cycles_transferred = req.cycles;

    Ok(Response::Cycles(CyclesResponse { cycles_transferred }))
}
