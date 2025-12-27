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
