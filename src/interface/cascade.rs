use crate::{
    Error, Log,
    ic::call::Call,
    interface::{self, InterfaceError, ic::IcError},
    log,
};

// app_state_cascade
pub async fn app_state_cascade() -> Result<(), Error> {
    let app_state = interface::state::core::app_state::get_data();
    let child_index = interface::state::core::child_index::get_data();

    // iterate child canisters
    for (pid, path) in child_index {
        log!(Log::Info, "app_state_cascade: -> {pid} ({path})");

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
    let subnet_index = interface::state::core::subnet_index::get_data();
    let child_index = interface::state::core::child_index::get_data();

    // iterate child canisters
    for (pid, path) in child_index {
        log!(Log::Info, "subnet_index_cascade: -> {pid} ({path})",);

        Call::unbounded_wait(pid, "icu_subnet_index_cascade")
            .with_arg(&subnet_index)
            .await
            .map_err(IcError::from)
            .map_err(InterfaceError::IcError)?;
    }

    Ok(())
}
