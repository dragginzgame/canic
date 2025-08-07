use crate::{
    Error, Log,
    ic::call::Call,
    interface::{InterfaceError, ic::IcError},
    log, memory,
};

// app_state_cascade
pub async fn app_state_cascade() -> Result<(), Error> {
    let app_state = memory::AppState::export();
    let child_index = memory::ChildIndex::export();

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
    let subnet_index = memory::SubnetIndex::export();
    let child_index = memory::ChildIndex::export();

    // iterate child canisters
    for (pid, kind) in child_index {
        log!(Log::Info, "subnet_index_cascade: -> {pid} ({kind})",);

        Call::unbounded_wait(pid, "icu_subnet_index_cascade")
            .with_arg(&subnet_index)
            .await
            .map_err(IcError::from)
            .map_err(InterfaceError::IcError)?;
    }

    Ok(())
}
