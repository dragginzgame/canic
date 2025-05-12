use crate::{
    Error, Log,
    ic::call::Call,
    log,
    state::core::{APP_STATE, AppState, CHILD_INDEX, ChildIndex, SUBNET_INDEX, SubnetIndex},
};

// app_state_cascade_api
pub async fn app_state_cascade_api() -> Result<(), Error> {
    app_state_cascade().await
}

// app_state_cascade
pub async fn app_state_cascade() -> Result<(), Error> {
    let app_state = APP_STATE.with_borrow(AppState::get_data);
    let child_index = CHILD_INDEX.with_borrow(ChildIndex::get_data);

    // iterate child canisters
    for (pid, path) in child_index {
        log!(Log::Info, "app_state_cascade: -> {pid} ({path})");

        Call::unbounded_wait(pid, "app_state_cascade")
            .with_arg(app_state)
            .await
            .map_err(|e| Error::CallFailed(e.to_string()))?;
    }

    Ok(())
}

// subnet_index_cascade_api
pub async fn subnet_index_cascade_api() -> Result<(), Error> {
    subnet_index_cascade().await?;

    Ok(())
}

// subnet_index_cascade
pub async fn subnet_index_cascade() -> Result<(), Error> {
    let subnet_index = SUBNET_INDEX.with_borrow(SubnetIndex::get_data);
    let child_index = CHILD_INDEX.with_borrow(ChildIndex::get_data);

    // iterate child canisters
    for (pid, path) in child_index {
        log!(Log::Info, "subnet_index_cascade: -> {pid} ({path})",);

        Call::unbounded_wait(pid, "subnet_index_cascade")
            .with_arg(&subnet_index)
            .await
            .map_err(|e| Error::CallFailed(e.to_string()))?;
    }

    Ok(())
}
