use crate::{
    Error, Log,
    ic::{api::canister_self, call::Call},
    interface::{InterfaceError, ic::IcError},
    log,
    memory::{AppState, CanisterState, ChildIndex, SubnetIndex},
};

// app_state_cascade
pub async fn app_state_cascade() -> Result<(), Error> {
    let app_state = AppState::export();
    let child_index = ChildIndex::export();

    // iterate child canisters
    for (pid, kind) in child_index {
        log!(Log::Info, "app_state_cascade: -> {pid} ({kind})");

        Call::unbounded_wait(pid, "icu_app_state_cascade")
            .with_arg(app_state)
            .await
            .map_err(IcError::from)
            .map_err(InterfaceError::IcError)?;
    }

    Ok(())
}

// subnet_index_cascade
pub async fn subnet_index_cascade() -> Result<(), Error> {
    let subnet_index = SubnetIndex::export();
    let child_index = ChildIndex::export();

    let canister_self = canister_self();
    let canister_kind = CanisterState::try_get_kind()?;

    // iterate child canisters
    for (pid, kind) in child_index {
        log!(
            Log::Info,
            "subnet_index_cascade: {canister_self} ({canister_kind}) -> {pid} ({kind})",
        );

        Call::unbounded_wait(pid, "icu_subnet_index_cascade")
            .with_arg(&subnet_index)
            .await
            .map_err(IcError::from)
            .map_err(InterfaceError::IcError)?;
    }

    Ok(())
}
