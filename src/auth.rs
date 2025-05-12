pub use crate::state::auth::{Permission, Role};

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
    #[error("one or more rules must be defined")]
    NoRulesDefined,

    #[error("there has to be a user canister defined in the schema")]
    NoUserCanister,

    #[error("this action is not allowed due to configuration settings")]
    NotAllowed,

    #[error("principal '{0}' does not match canister type '{1}'")]
    NotCanisterType(Principal, String),

    #[error("principal '{0}' is not a child of this canister'")]
    NotChild(Principal),

    #[error("principal '{0}' is not a controller of this canister'")]
    NotController(Principal),

    #[error("principal '{0}' is not the parent of this canister'")]
    NotParent(Principal),

    #[error("permission '{0}' is required")]
    NotPermitted(Permission),

    #[error("principal '{0}' is not root")]
    NotRoot(Principal),

    #[error("principal '{0}' is not from this subnet")]
    NotSubnet(Principal),

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
    pub async fn result(self, pid: Principal) -> Result<(), Error> {
        match self {
            Self::CanisterType(canister) => rule_canister_type(pid, canister),
            Self::Child => rule_child(pid),
            Self::Controller => rule_controller(pid),
            Self::Parent => rule_parent(pid),
            //       Self::Permission(permission) => rule_permission(pid, permission).await,
            // Self::Policy(policy) => rule_policy(pid, policy).await,
            Self::Root => rule_root(pid),
            Self::SameSubnet => rule_same_subnet(pid).await,
            Self::SameCanister => rule_same_canister(pid),
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
    Permission(Permission),
}

// allow_any
pub async fn allow_any(rules: Vec<Auth>) -> Result<(), Error> {
    // only works for caller now
    let caller = msg_caller();

    // in case rules are accidentally blank / commented out
    if rules.is_empty() {
        Err(AuthError::NoRulesDefined)?;
    }

    // check rules
    let mut last_error = None;
    for rule in rules {
        match rule.result(caller).await {
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
fn rule_canister_type(pid: Principal, canister: String) -> Result<(), Error> {
    interface::state::core::subnet_index::try_get_canister(&canister)
        .map_err(|_| AuthError::NotCanisterType(pid, canister.clone()))?;

    Ok(())
}

// rule_child
fn rule_child(pid: Principal) -> Result<(), Error> {
    interface::state::core::child_index::get_canister(&pid).ok_or(AuthError::NotChild(pid))?;

    Ok(())
}

// rule_controller
fn rule_controller(pid: Principal) -> Result<(), Error> {
    if is_controller(&pid) {
        Ok(())
    } else {
        Err(AuthError::NotController(pid))?
    }
}

// rule_root
fn rule_root(pid: Principal) -> Result<(), Error> {
    let root_pid = interface::state::core::canister_state::get_root_pid()?;

    if pid == root_pid {
        Ok(())
    } else {
        Err(AuthError::NotRoot(pid))?
    }
}

// rule_parent
fn rule_parent(pid: Principal) -> Result<(), Error> {
    match interface::state::core::canister_state::get_parent_pid() {
        Some(parent_id) if parent_id == pid => Ok(()),
        _ => Err(AuthError::NotParent(pid))?,
    }
}

// rule_permission
// will find the user canister from the schema
/*
pub async fn rule_permission(pid: Principal, permission: Permission) -> Result<(), AuthError> {
    let user_canister_pid = SUBNET_INDEX.with_borrow(|this| this.try_get_canister("user")?);

    Call::unbounded_wait(user_canister_pid, "guard_permission")
        .with_args(&(pid, permission))
        .await
        .map_err(|_| AuthError::NotPermitted(permission))?;

    Ok(())
}

// rule_policy
// only from non-PlayerHub canisters
async fn rule_policy(pid: Principal, policy: AccessPolicy) -> Result<(), AuthError> {
    match policy {
        AccessPolicy::Allow => Ok(()),
        AccessPolicy::Deny => Err(AuthError::NotAllowed)?,
        AccessPolicy::Permission(permission) => rule_permission(pid, permission).await,
    }
}

// roles_have_permission_api
fn roles_have_permission_api(
    roles: &[Role],
    permission: &Permission,
) -> Result<(), InterfaceError> {
    if roles.iter().any(|role| role.has_permission(permission)) {
        Ok(())
    } else {
        Err(InterfaceError::AuthError(AuthError::NotPermitted(
            *permission,
        )))
    }
}
    */

// rule_same_subnet
#[expect(clippy::unused_async)]
pub async fn rule_same_subnet(_id: Principal) -> Result<(), Error> {
    // @todo - we need gabriel code here

    Ok(())
}

// rule_same_canister
fn rule_same_canister(pid: Principal) -> Result<(), Error> {
    if pid == canister_self() {
        Ok(())
    } else {
        Err(AuthError::NotThis(pid))?
    }
}
