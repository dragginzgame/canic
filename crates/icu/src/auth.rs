use crate::{
    Error,
    ic::api::{canister_self, msg_caller},
    interface,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// AuthError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum AuthError {
    #[error("{0}")]
    Custom(String),

    #[error("invalid error state - this should never happen")]
    InvalidState,

    #[error("the root canister is not defined")]
    NoRootDefined,

    #[error("one or more rules must be defined")]
    NoRulesDefined,

    #[error("there has to be a user canister defined in the schema")]
    NoUserCanister,

    #[error("this action is not allowed due to configuration settings")]
    NotAllowed,

    #[error("principal '{0}' does not match canister type '{1}'")]
    NotCanisterType(Principal, String),

    #[error("principal '{0}' is not a child of this canister")]
    NotChild(Principal),

    #[error("principal '{0}' is not a controller of this canister")]
    NotController(Principal),

    #[error("principal '{0}' is not the parent of this canister")]
    NotParent(Principal),

    #[error("principal '{0}' is not root")]
    NotRoot(Principal),

    #[error("principal '{0}' is not the current canister")]
    NotThis(Principal),
}

impl From<&str> for AuthError {
    fn from(s: &str) -> Self {
        AuthError::Custom(s.to_string())
    }
}

impl From<String> for AuthError {
    fn from(s: String) -> Self {
        AuthError::Custom(s)
    }
}

///
/// Auth Functions
///

// require_any
pub async fn require_any<F, R>(rules: Vec<F>) -> Result<(), Error>
where
    F: Fn(Principal) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send,
{
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(AuthError::NoRulesDefined.into());
    }

    let mut last_error = None;
    for rule in rules {
        match rule(caller).await {
            Ok(()) => return Ok(()),
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.unwrap_or_else(|| AuthError::InvalidState.into()))
}

// require_all
pub async fn require_all<F, R>(rules: Vec<F>) -> Result<(), Error>
where
    F: Fn(Principal) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send,
{
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(AuthError::NoRulesDefined.into());
    }

    for rule in rules {
        rule(caller).await?; // early return on failure
    }

    Ok(())
}

///
/// RULE MACROS
///

// is_canister_type
// check caller against the id of a specific canister path
pub async fn is_canister_type(pid: Principal, canister: String) -> Result<(), Error> {
    interface::memory::subnet::index::try_get_canister(&canister)
        .map_err(|_| AuthError::NotCanisterType(pid, canister.clone()))?;

    Ok(())
}

// is_child
pub async fn is_child(pid: Principal) -> Result<(), Error> {
    interface::memory::canister::child_index::get_canister(&pid).ok_or(AuthError::NotChild(pid))?;

    Ok(())
}

// is_controller
pub async fn is_controller(pid: Principal) -> Result<(), Error> {
    if crate::ic::api::is_controller(&pid) {
        Ok(())
    } else {
        Err(AuthError::NotController(pid))?
    }
}

// is_root
pub async fn is_root(pid: Principal) -> Result<(), Error> {
    let root_pid =
        interface::memory::canister::state::get_root_pid().map_err(|_| AuthError::NoRootDefined)?;

    if pid == root_pid {
        Ok(())
    } else {
        Err(AuthError::NotRoot(pid))?
    }
}

// is_parent
pub async fn is_parent(pid: Principal) -> Result<(), Error> {
    match interface::memory::canister::state::get_parent_pid() {
        Some(parent_pid) if parent_pid == pid => Ok(()),
        _ => Err(AuthError::NotParent(pid))?,
    }
}

// is_same_canister
pub async fn is_same_canister(pid: Principal) -> Result<(), Error> {
    if pid == canister_self() {
        Ok(())
    } else {
        Err(AuthError::NotThis(pid))?
    }
}
