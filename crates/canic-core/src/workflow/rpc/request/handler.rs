use crate::{
    Error,
    access::env,
    dto::rpc::{
        CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse, CyclesRequest,
        CyclesResponse, Request, Response, UpgradeCanisterRequest, UpgradeCanisterResponse,
    },
    ops::{
        ic::mgmt::MgmtOps,
        storage::{directory::subnet::SubnetDirectoryOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::{
        canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
        prelude::*,
        rpc::RpcWorkflowError,
    },
};

///
/// RootResponseWorkflow
///

pub struct RootResponseWorkflow;

impl RootResponseWorkflow {
    /// Handle a root-bound orchestration request and produce a [`Response`].
    pub async fn response(req: Request) -> Result<Response, Error> {
        env::require_root()?;

        match req {
            Request::CreateCanister(req) => Self::create_canister_response(&req).await,
            Request::UpgradeCanister(req) => Self::upgrade_canister_response(&req).await,
            Request::Cycles(req) => Self::cycles_response(&req).await,
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
                    .ok_or(RpcWorkflowError::ParentNotFound(caller))?,

                CreateCanisterParent::Directory(role) => SubnetDirectoryOps::get(role)
                    .ok_or_else(|| RpcWorkflowError::CanisterRoleNotFound(role.clone()))?,
            };

            let event = CanisterLifecycleEvent::Create {
                role: req.canister_role.clone(),
                parent: parent_pid,
                extra_arg: req.extra_arg.clone(),
            };

            let lifecycle_result = CanisterLifecycleWorkflow::apply(event).await?;
            let new_canister_pid = lifecycle_result
                .new_canister_pid
                .ok_or(RpcWorkflowError::MissingNewCanisterPid)?;

            Ok(Response::CreateCanister(CreateCanisterResponse {
                new_canister_pid,
            }))
        }
        .await;

        if let Err(err) = &result {
            log!(
                Topic::Rpc,
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
            .ok_or(RpcWorkflowError::ChildNotFound(req.canister_pid))?;

        if registry_entry.parent_pid != Some(caller) {
            return Err(RpcWorkflowError::NotChildOfCaller(req.canister_pid, caller).into());
        }

        let event = CanisterLifecycleEvent::Upgrade {
            pid: req.canister_pid,
        };

        CanisterLifecycleWorkflow::apply(event).await?;

        Ok(Response::UpgradeCanister(UpgradeCanisterResponse {}))
    }

    // cycles_response
    async fn cycles_response(req: &CyclesRequest) -> Result<Response, Error> {
        env::require_root()?;

        MgmtOps::deposit_cycles(msg_caller(), req.cycles).await?;

        let cycles_transferred = req.cycles;

        Ok(Response::Cycles(CyclesResponse { cycles_transferred }))
    }
}
