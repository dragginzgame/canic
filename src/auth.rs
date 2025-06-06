use crate::{
    Error,
    ic::api::{canister_self, is_controller, msg_caller},
    interface,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// AuthError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum AuthError {
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

    #[error("principal '{0}' is not from this subnet (DOESNT WORK YET)")]
    NotThisSubnet(Principal),

    #[error("principal '{0}' is not the current canister")]
    NotThis(Principal),

    #[error("role '{0}' not found")]
    RoleNotFound(String),
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
    //   Permission(Permission),
    // Policy(AccessPolicy),
    Root,
    SameCanister,
    SameSubnet,
}

impl Auth {
    pub fn result(self, pid: Principal) -> Result<(), Error> {
        match self {
            Self::CanisterType(canister) => rule_canister_type(pid, canister),
            Self::Child => rule_child(pid),
            Self::Controller => rule_controller(pid),
            Self::Parent => rule_parent(pid),
            Self::Root => rule_root(pid),
            Self::SameSubnet => rule_same_subnet(pid),
            Self::SameCanister => rule_same_canister(pid),
        }
        .map_err(Error::from)
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
pub fn allow_any(rules: Vec<Auth>) -> Result<(), Error> {
    // only works for caller now
    let caller = msg_caller();

    // in case rules are accidentally blank / commented out
    if rules.is_empty() {
        Err(AuthError::NoRulesDefined)?;
    }

    // check rules
    let mut last_error = None;
    for rule in rules {
        match rule.result(caller) {
            Ok(()) => return Ok(()),
            Err(e) => last_error = Some(e),
        }
    }

    last_error.map_or(Ok(()), Err)
}

///
/// RULE MACROS
///

// rule_canister_type
// check caller against the id of a specific canister path
fn rule_canister_type(pid: Principal, canister: String) -> Result<(), AuthError> {
    interface::memory::subnet::index::try_get_canister(&canister)
        .map_err(|_| AuthError::NotCanisterType(pid, canister.clone()))?;

    Ok(())
}

// rule_child
fn rule_child(pid: Principal) -> Result<(), AuthError> {
    interface::memory::canister::child_index::get_canister(&pid).ok_or(AuthError::NotChild(pid))?;

    Ok(())
}

// rule_controller
fn rule_controller(pid: Principal) -> Result<(), AuthError> {
    if is_controller(&pid) {
        Ok(())
    } else {
        Err(AuthError::NotController(pid))
    }
}

// rule_root
fn rule_root(pid: Principal) -> Result<(), AuthError> {
    let root_pid =
        interface::memory::canister::state::get_root_pid().map_err(|_| AuthError::NoRootDefined)?;

    if pid == root_pid {
        Ok(())
    } else {
        Err(AuthError::NotRoot(pid))
    }
}
// rule_parent
fn rule_parent(pid: Principal) -> Result<(), AuthError> {
    match interface::memory::canister::state::get_parent_pid() {
        Some(parent_pid) if parent_pid == pid => Ok(()),
        _ => Err(AuthError::NotParent(pid)),
    }
}

// rule_same_subnet
pub const fn rule_same_subnet(id: Principal) -> Result<(), AuthError> {
    Err(AuthError::NotThisSubnet(id))
}

// rule_same_canister
fn rule_same_canister(pid: Principal) -> Result<(), AuthError> {
    if pid == canister_self() {
        Ok(())
    } else {
        Err(AuthError::NotThis(pid))
    }
}
