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
