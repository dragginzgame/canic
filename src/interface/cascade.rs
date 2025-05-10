use crate::{
    Log,
    ic::call::Call,
    interface::InterfaceError,
    log,
    state::{APP_STATE, AppState, CHILD_INDEX, ChildIndex, SUBNET_INDEX, SubnetIndex},
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// CascadeError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum CascadeError {
    #[error("api error: {0}")]
    ApiError(String),
}

// app_state_cascade_api
pub async fn app_state_cascade_api() -> Result<(), InterfaceError> {
    app_state_cascade()
        .await
        .map_err(InterfaceError::CascadeError)
}

// app_state_cascade
pub async fn app_state_cascade() -> Result<(), CascadeError> {
    let app_state = APP_STATE.with_borrow(AppState::get_data);
    let child_index = CHILD_INDEX.with_borrow(ChildIndex::get_data);

    // iterate child canisters
    for (pid, ty) in child_index {
        log!(Log::Info, "app_state_cascade: -> {pid} ({ty})");

        Call::unbounded_wait(pid, "app_state_cascade")
            .with_arg(app_state)
            .await
            .map_err(|e| CascadeError::ApiError(e.to_string()))?;
    }

    Ok(())
}

// subnet_index_cascade_api
pub async fn subnet_index_cascade_api() -> Result<(), InterfaceError> {
    subnet_index_cascade()
        .await
        .map_err(|e| CascadeError::ApiError(e.to_string()))?;

    Ok(())
}

// subnet_index_cascade
pub async fn subnet_index_cascade() -> Result<(), CascadeError> {
    let subnet_index = SUBNET_INDEX.with_borrow(SubnetIndex::get_data);
    let child_index = CHILD_INDEX.with_borrow(ChildIndex::get_data);

    // iterate child canisters
    for (pid, ty) in child_index {
        log!(Log::Info, "subnet_index_cascade: -> {pid} ({ty})",);

        Call::unbounded_wait(pid, "subnet_index_cascade")
            .with_arg(&subnet_index)
            .await
            .map_err(|e| CascadeError::ApiError(e.to_string()))?;
    }

    Ok(())
}
