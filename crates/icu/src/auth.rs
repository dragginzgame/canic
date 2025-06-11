use crate::{
    Error,
    ic::api::{canister_self, is_controller, msg_caller},
    interface,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// AuthRule
///

pub trait AuthRule {
    fn check(&self, principal: Principal) -> Result<(), String>;
}

///
/// Auth
///

#[remain::sorted]
pub enum Auth {
    CanisterType(String),
    Child,
    Controller,
    Parent,
    Root,
    SameCanister,
}

impl AuthRule for Auth {
    fn check(&self, pid: Principal) -> Result<(), String> {
        match self {
            Self::CanisterType(canister) => check_canister_type(pid, canister.to_string()),
            Self::Child => check_child(pid),
            Self::Controller => check_controller(pid),
            Self::Parent => check_parent(pid),
            Self::Root => check_root(pid),
            Self::SameCanister => check_same_canister(pid),
        }
        .map_err(|e| e.to_string())
    }
}

///
/// AccessPolicy
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AccessPolicy {
    Allow,
    Deny,
    //  Custom - WIP
}

// allow_any
pub fn allow_any(rules: Vec<&dyn AuthRule>) -> Result<(), Error> {
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(Error::AuthError("no rules defined".into()));
    }

    let mut last_error = None;
    for rule in rules {
        match rule.check(caller) {
            Ok(()) => return Ok(()),
            Err(e) => last_error = Some(e),
        }
    }

    last_error.map_or(Ok(()), |e| Err(Error::AuthError(e)))
}

///
/// CheckError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum CheckError {
    #[error("the root canister is not defined")]
    NoRootDefined,

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

    #[error("role '{0}' not found")]
    RoleNotFound(String),
}

///
/// CHECK METHODS
///

// check_canister_type
// check caller against the id of a specific canister path
fn check_canister_type(pid: Principal, canister: String) -> Result<(), CheckError> {
    interface::memory::subnet::index::try_get_canister(&canister)
        .map_err(|_| CheckError::NotCanisterType(pid, canister.clone()))?;

    Ok(())
}

// check_child
fn check_child(pid: Principal) -> Result<(), CheckError> {
    interface::memory::canister::child_index::get_canister(&pid)
        .ok_or(CheckError::NotChild(pid))?;

    Ok(())
}

// check_controller
fn check_controller(pid: Principal) -> Result<(), CheckError> {
    if is_controller(&pid) {
        Ok(())
    } else {
        Err(CheckError::NotController(pid))
    }
}

// check_root
fn check_root(pid: Principal) -> Result<(), CheckError> {
    let root_pid = interface::memory::canister::state::get_root_pid()
        .map_err(|_| CheckError::NoRootDefined)?;

    if pid == root_pid {
        Ok(())
    } else {
        Err(CheckError::NotRoot(pid))
    }
}

// check_parent
fn check_parent(pid: Principal) -> Result<(), CheckError> {
    match interface::memory::canister::state::get_parent_pid() {
        Some(parent_pid) if parent_pid == pid => Ok(()),
        _ => Err(CheckError::NotParent(pid)),
    }
}

// check_same_canister
fn check_same_canister(pid: Principal) -> Result<(), CheckError> {
    if pid == canister_self() {
        Ok(())
    } else {
        Err(CheckError::NotThis(pid))
    }
}
