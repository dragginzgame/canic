use crate::{
    Error,
    config::Config,
    ic::api::{canister_self, msg_caller},
    memory,
};
use candid::Principal;
use std::pin::Pin;
use thiserror::Error as ThisError;

///
/// AuthError
///

#[derive(Debug, ThisError)]
pub enum AuthError {
    #[error("{0}")]
    Custom(String),

    #[error("invalid error state - this should never happen")]
    InvalidState,

    #[error("one or more rules must be defined")]
    NoRulesDefined,

    #[error("principal '{0}' does not match canister type '{1}'")]
    NotCanisterType(Principal, String),

    #[error("principal '{0}' is not a child of this canister")]
    NotChild(Principal),

    #[error("principal '{0}' is not a controller of this canister")]
    NotController(Principal),

    #[error("principal '{0}' is not the parent of this canister")]
    NotParent(Principal),

    #[error("expected principal '{1}' got '{0}'")]
    NotPrincipal(Principal, Principal),

    #[error("principal '{0}' is not root")]
    NotRoot(Principal),

    #[error("principal '{0}' is not the current canister")]
    NotSameCanister(Principal),

    #[error("principal '{0}' is not on the whitelist")]
    NotWhitelisted(Principal),
}

impl AuthError {
    #[must_use]
    pub fn custom(s: &str) -> Self {
        Self::Custom(s.to_string())
    }
}

///
/// Rule
///

pub type RuleFn =
    Box<dyn Fn(Principal) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> + Send + Sync>;

pub type RuleResult = Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;

///
/// Auth Functions
///

// require_all
pub async fn require_all(rules: Vec<RuleFn>) -> Result<(), Error> {
    let caller = msg_caller();

    if rules.is_empty() {
        return Err(AuthError::NoRulesDefined.into());
    }

    for rule in rules {
        rule(caller).await?; // early return on failure
    }

    Ok(())
}

// require_any
pub async fn require_any(rules: Vec<RuleFn>) -> Result<(), Error> {
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

///
/// RULE MACROS
///

#[macro_export]
macro_rules! auth_require_all {
    ($($f:expr),* $(,)?) => {{
        $crate::auth::require_all(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}

#[macro_export]
macro_rules! auth_require_any {
    ($($f:expr),* $(,)?) => {{
        $crate::auth::require_any(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}

///
/// RULE FUNCTIONS
///

// is_canister_type
// check caller against the id of a specific canister path
#[must_use]
pub fn is_canister_type(caller: Principal, canister: String) -> RuleResult {
    Box::pin(async move {
        memory::SubnetIndex::try_get(&canister)
            .map_err(|_| AuthError::NotCanisterType(caller, canister.clone()))?;

        Ok(())
    })
}

// is_child
#[must_use]
pub fn is_child(caller: Principal) -> RuleResult {
    Box::pin(async move {
        memory::ChildIndex::get(&caller).ok_or(AuthError::NotChild(caller))?;

        Ok(())
    })
}

// is_controller
#[must_use]
pub fn is_controller(caller: Principal) -> RuleResult {
    Box::pin(async move {
        if crate::ic::api::is_controller(&caller) {
            Ok(())
        } else {
            Err(AuthError::NotController(caller).into())
        }
    })
}

// is_root
#[must_use]
pub fn is_root(caller: Principal) -> RuleResult {
    Box::pin(async move {
        let root_pid = memory::CanisterState::get_root_pid();

        if caller == root_pid {
            Ok(())
        } else {
            Err(AuthError::NotRoot(caller))?
        }
    })
}

// is_parent
#[must_use]
pub fn is_parent(caller: Principal) -> RuleResult {
    Box::pin(async move {
        if memory::CanisterState::has_parent_pid(&caller) {
            Ok(())
        } else {
            Err(AuthError::NotParent(caller))?
        }
    })
}

// is_principal
#[must_use]
pub fn is_principal(caller: Principal, expected: Principal) -> RuleResult {
    Box::pin(async move {
        if caller == expected {
            Ok(())
        } else {
            Err(AuthError::NotPrincipal(caller, expected))?
        }
    })
}

// is_same_canister
#[must_use]
pub fn is_same_canister(caller: Principal) -> RuleResult {
    Box::pin(async move {
        if caller == canister_self() {
            Ok(())
        } else {
            Err(AuthError::NotSameCanister(caller))?
        }
    })
}

// is_whitelisted
#[must_use]
pub fn is_whitelisted(caller: Principal) -> RuleResult {
    Box::pin(async move {
        let config = Config::get()?;

        if config.whitelist.contains(&caller) {
            Ok(())
        } else {
            Err(AuthError::NotWhitelisted(caller))?
        }
    })
}
