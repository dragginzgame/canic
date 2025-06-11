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

    #[error("role '{0}' not found")]
    RoleNotFound(String),
}

///
/// AuthRule
///

pub trait AuthRule {
    fn check(&self, principal: Principal) -> Result<(), AuthError>;
}

impl<F> AuthRule for F
where
    F: Fn(Principal) -> Result<(), AuthError> + 'static,
{
    fn check(&self, principal: Principal) -> Result<(), AuthError> {
        (self)(principal)
    }
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
    fn check(&self, pid: Principal) -> Result<(), AuthError> {
        match self {
            Self::CanisterType(canister) => check_canister_type(pid, canister.to_string()),
            Self::Child => check_child(pid),
            Self::Controller => check_controller(pid),
            Self::Parent => check_parent(pid),
            Self::Root => check_root(pid),
            Self::SameCanister => check_same_canister(pid),
        }
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

#[macro_export]
macro_rules! allow_any {
    ($($rule:expr),* $(,)?) => {{
        let rules: Vec<Box<dyn $crate::auth::AuthRule>> = vec![
            $(Box::new($rule)),*
        ];
        $crate::auth::allow_any(rules)
    }};
}

// allow_any
pub fn allow_any(rules: Vec<Box<dyn AuthRule>>) -> Result<(), Error> {
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(AuthError::NoRulesDefined.into());
    }

    let mut last_error = None;
    for rule in rules {
        match rule.check(caller) {
            Ok(()) => return Ok(()),
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.unwrap_or(AuthError::InvalidState).into())
}

///
/// RULE MACROS
///

// check_canister_type
// check caller against the id of a specific canister path
fn check_canister_type(pid: Principal, canister: String) -> Result<(), AuthError> {
    interface::memory::subnet::index::try_get_canister(&canister)
        .map_err(|_| AuthError::NotCanisterType(pid, canister.clone()))?;

    Ok(())
}

// check_child
fn check_child(pid: Principal) -> Result<(), AuthError> {
    interface::memory::canister::child_index::get_canister(&pid).ok_or(AuthError::NotChild(pid))?;

    Ok(())
}

// check_controller
fn check_controller(pid: Principal) -> Result<(), AuthError> {
    if is_controller(&pid) {
        Ok(())
    } else {
        Err(AuthError::NotController(pid))
    }
}

// check_root
fn check_root(pid: Principal) -> Result<(), AuthError> {
    let root_pid =
        interface::memory::canister::state::get_root_pid().map_err(|_| AuthError::NoRootDefined)?;

    if pid == root_pid {
        Ok(())
    } else {
        Err(AuthError::NotRoot(pid))
    }
}

// check_parent
fn check_parent(pid: Principal) -> Result<(), AuthError> {
    match interface::memory::canister::state::get_parent_pid() {
        Some(parent_pid) if parent_pid == pid => Ok(()),
        _ => Err(AuthError::NotParent(pid)),
    }
}

// check_same_canister
fn check_same_canister(pid: Principal) -> Result<(), AuthError> {
    if pid == canister_self() {
        Ok(())
    } else {
        Err(AuthError::NotThis(pid))
    }
}
