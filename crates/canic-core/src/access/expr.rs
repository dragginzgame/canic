//! Access expression model and evaluator.
//!
//! This module defines the expression tree and async predicate interface
//! used to compose access control without changing existing semantics.

use crate::{
    access::{
        self, AccessError,
        metrics::{AccessMetrics, DelegationMetrics},
    },
    cdk::types::Principal,
    ids::{AccessMetricKind, EndpointCall},
    log,
    log::Topic,
};
use async_trait::async_trait;
use std::{future::Future, pin::Pin, sync::Arc};

///
/// AccessContext
///

#[derive(Clone, Debug)]
pub struct AccessContext {
    pub caller: Principal,
    pub call: EndpointCall,
}

///
/// DefaultAppGuard
///
/// Synchronous app-mode guards used for implicit gating.
///
pub enum DefaultAppGuard {
    AllowsUpdates,
    IsQueryable,
}

///
/// AccessExpr
///

#[derive(Clone)]
pub enum AccessExpr {
    All(Vec<Self>),
    Any(Vec<Self>),
    Not(Box<Self>),
    Pred(AccessPredicate),
}

///
/// AccessPredicate
///

#[derive(Clone)]
pub enum AccessPredicate {
    Builtin(BuiltinPredicate),
    Custom(Arc<dyn AsyncAccessPredicate>),
}

///
/// BuiltinPredicate
///

#[derive(Clone, Copy, Debug)]
pub enum BuiltinPredicate {
    AppAllowsUpdates,
    AppIsQueryable,
    SelfIsPrimeSubnet,
    SelfIsPrimeRoot,
    CallerIsController,
    CallerIsParent,
    CallerIsChild,
    CallerIsRoot,
    CallerIsSameCanister,
    CallerIsRegisteredToSubnet,
    CallerIsWhitelisted,
    Authenticated {
        required_scope: Option<&'static str>,
    },
    BuildIcOnly,
    BuildLocalOnly,
}

#[async_trait]
pub trait AsyncAccessPredicate: Send + Sync {
    ///
    /// Custom predicate contract:
    /// - May be async and perform I/O.
    /// - May depend on application state.
    /// - Must be side-effect free, idempotent, and must not panic.
    ///
    async fn eval(&self, ctx: &AccessContext) -> Result<(), AccessError>;
    fn name(&self) -> &'static str;
}

pub fn all<I>(exprs: I) -> AccessExpr
where
    I: IntoIterator<Item = AccessExpr>,
{
    AccessExpr::All(exprs.into_iter().collect())
}

pub fn any<I>(exprs: I) -> AccessExpr
where
    I: IntoIterator<Item = AccessExpr>,
{
    AccessExpr::Any(exprs.into_iter().collect())
}

#[must_use]
pub fn not(expr: AccessExpr) -> AccessExpr {
    AccessExpr::Not(Box::new(expr))
}

pub fn requires<I>(exprs: I) -> AccessExpr
where
    I: IntoIterator<Item = AccessExpr>,
{
    all(exprs)
}

pub fn custom<P>(pred: P) -> AccessExpr
where
    P: AsyncAccessPredicate + 'static,
{
    AccessExpr::Pred(AccessPredicate::Custom(Arc::new(pred)))
}

pub mod app {
    use super::{AccessExpr, BuiltinPredicate, builtin};

    #[must_use]
    pub const fn allows_updates() -> AccessExpr {
        builtin(BuiltinPredicate::AppAllowsUpdates)
    }

    #[must_use]
    pub const fn is_queryable() -> AccessExpr {
        builtin(BuiltinPredicate::AppIsQueryable)
    }
}

pub mod caller {
    use super::{AccessExpr, BuiltinPredicate, builtin};

    #[must_use]
    pub const fn is_controller() -> AccessExpr {
        builtin(BuiltinPredicate::CallerIsController)
    }

    #[must_use]
    pub const fn is_parent() -> AccessExpr {
        builtin(BuiltinPredicate::CallerIsParent)
    }

    #[must_use]
    pub const fn is_child() -> AccessExpr {
        builtin(BuiltinPredicate::CallerIsChild)
    }

    #[must_use]
    pub const fn is_root() -> AccessExpr {
        builtin(BuiltinPredicate::CallerIsRoot)
    }

    #[must_use]
    pub const fn is_same_canister() -> AccessExpr {
        builtin(BuiltinPredicate::CallerIsSameCanister)
    }

    #[must_use]
    pub const fn is_registered_to_subnet() -> AccessExpr {
        builtin(BuiltinPredicate::CallerIsRegisteredToSubnet)
    }

    #[must_use]
    pub const fn is_whitelisted() -> AccessExpr {
        builtin(BuiltinPredicate::CallerIsWhitelisted)
    }
}

pub mod env {
    use super::{AccessExpr, BuiltinPredicate, builtin};

    #[must_use]
    pub const fn is_prime_subnet() -> AccessExpr {
        builtin(BuiltinPredicate::SelfIsPrimeSubnet)
    }

    #[must_use]
    pub const fn is_prime_root() -> AccessExpr {
        builtin(BuiltinPredicate::SelfIsPrimeRoot)
    }

    #[must_use]
    pub const fn build_ic_only() -> AccessExpr {
        builtin(BuiltinPredicate::BuildIcOnly)
    }

    #[must_use]
    pub const fn build_local_only() -> AccessExpr {
        builtin(BuiltinPredicate::BuildLocalOnly)
    }
}

pub mod auth {
    use super::{AccessExpr, BuiltinPredicate, builtin};

    #[must_use]
    pub const fn authenticated(required_scope: Option<&'static str>) -> AccessExpr {
        builtin(BuiltinPredicate::Authenticated { required_scope })
    }

    #[must_use]
    pub const fn is_authenticated() -> AccessExpr {
        authenticated(None)
    }
}

pub async fn eval_access(expr: &AccessExpr, ctx: &AccessContext) -> Result<(), AccessError> {
    match eval_access_inner(expr, ctx).await {
        Ok(()) => Ok(()),
        Err(failure) => Err(record_access_failure(ctx, failure)),
    }
}

type AccessEvalFuture<'a> = Pin<Box<dyn Future<Output = Result<(), AccessFailure>> + Send + 'a>>;

pub fn eval_default_app_guard(
    guard: DefaultAppGuard,
    ctx: &AccessContext,
) -> Result<(), AccessError> {
    let result = match guard {
        DefaultAppGuard::AllowsUpdates => access::app::guard_app_update(),
        DefaultAppGuard::IsQueryable => access::app::guard_app_query(),
    };

    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            let predicate = match guard {
                DefaultAppGuard::AllowsUpdates => BuiltinPredicate::AppAllowsUpdates,
                DefaultAppGuard::IsQueryable => BuiltinPredicate::AppIsQueryable,
            };
            Err(record_access_failure(
                ctx,
                AccessFailure::from_builtin(predicate, err),
            ))
        }
    }
}

fn eval_access_inner<'a>(expr: &'a AccessExpr, ctx: &'a AccessContext) -> AccessEvalFuture<'a> {
    Box::pin(async move {
        match expr {
            AccessExpr::All(exprs) => {
                if exprs.is_empty() {
                    return Err(AccessFailure::no_predicates("all"));
                }
                for expr in exprs {
                    if let Err(failure) = eval_access_inner(expr, ctx).await {
                        return Err(failure.with_context("all"));
                    }
                }
                Ok(())
            }
            AccessExpr::Any(exprs) => {
                if exprs.is_empty() {
                    return Err(AccessFailure::no_predicates("any"));
                }
                let mut last = None;
                for expr in exprs {
                    match eval_access_inner(expr, ctx).await {
                        Ok(()) => return Ok(()),
                        Err(failure) => last = Some(failure.with_context("any")),
                    }
                }
                Err(last.unwrap_or_else(|| AccessFailure::no_predicates("any")))
            }
            AccessExpr::Not(expr) => match eval_access_inner(expr, ctx).await {
                Ok(()) => Err(AccessFailure::negated()),
                Err(_) => Ok(()),
            },
            AccessExpr::Pred(pred) => match pred {
                AccessPredicate::Builtin(builtin) => eval_builtin(builtin, ctx)
                    .await
                    .map_err(|err| AccessFailure::from_builtin(*builtin, err)),
                AccessPredicate::Custom(custom) => custom
                    .eval(ctx)
                    .await
                    .map_err(|err| AccessFailure::from_custom(custom.name(), err)),
            },
        }
    })
}

async fn eval_builtin(pred: &BuiltinPredicate, ctx: &AccessContext) -> Result<(), AccessError> {
    match pred {
        BuiltinPredicate::AppAllowsUpdates => access::app::guard_app_update(),
        BuiltinPredicate::AppIsQueryable => access::app::guard_app_query(),
        BuiltinPredicate::SelfIsPrimeSubnet => access::env::is_prime_subnet(),
        BuiltinPredicate::SelfIsPrimeRoot => access::env::is_prime_root(),
        BuiltinPredicate::CallerIsController => access::auth::is_controller(ctx.caller).await,
        BuiltinPredicate::CallerIsParent => access::auth::is_parent(ctx.caller).await,
        BuiltinPredicate::CallerIsChild => access::auth::is_child(ctx.caller).await,
        BuiltinPredicate::CallerIsRoot => access::auth::is_root(ctx.caller).await,
        BuiltinPredicate::CallerIsSameCanister => access::auth::is_same_canister(ctx.caller).await,
        BuiltinPredicate::CallerIsRegisteredToSubnet => {
            access::auth::is_registered_to_subnet(ctx.caller).await
        }
        BuiltinPredicate::CallerIsWhitelisted => access::auth::is_whitelisted(ctx.caller).await,
        BuiltinPredicate::Authenticated { required_scope } => {
            let verified =
                access::auth::delegated_token_verified(ctx.caller, *required_scope).await?;
            DelegationMetrics::record_authority(verified.cert.shard_pid);
            Ok(())
        }
        BuiltinPredicate::BuildIcOnly => access::env::build_network_ic(),
        BuiltinPredicate::BuildLocalOnly => access::env::build_network_local(),
    }
}

const fn builtin(pred: BuiltinPredicate) -> AccessExpr {
    AccessExpr::Pred(AccessPredicate::Builtin(pred))
}

#[derive(Debug)]
struct AccessFailure {
    error: AccessError,
    metric_kind: AccessMetricKind,
    predicate: &'static str,
    context: Option<&'static str>,
}

impl AccessFailure {
    const fn from_builtin(pred: BuiltinPredicate, error: AccessError) -> Self {
        Self {
            error,
            metric_kind: pred.metric_kind(),
            predicate: pred.name(),
            context: None,
        }
    }

    const fn from_custom(name: &'static str, error: AccessError) -> Self {
        Self {
            error,
            metric_kind: AccessMetricKind::Custom,
            predicate: name,
            context: None,
        }
    }

    fn no_predicates(context: &'static str) -> Self {
        Self {
            error: AccessError::Denied("one or more rules must be defined".to_string()),
            metric_kind: AccessMetricKind::Auth,
            predicate: "no_rules",
            context: Some(context),
        }
    }

    fn negated() -> Self {
        Self {
            error: AccessError::Denied("negated predicate matched".to_string()),
            metric_kind: AccessMetricKind::Auth,
            predicate: "not",
            context: Some("not"),
        }
    }

    fn with_context(mut self, context: &'static str) -> Self {
        self.context.get_or_insert(context);
        self
    }
}

impl BuiltinPredicate {
    const fn name(self) -> &'static str {
        match self {
            Self::AppAllowsUpdates => "app_allows_updates",
            Self::AppIsQueryable => "app_is_queryable",
            Self::SelfIsPrimeSubnet => "self_is_prime_subnet",
            Self::SelfIsPrimeRoot => "self_is_prime_root",
            Self::CallerIsController => "caller_is_controller",
            Self::CallerIsParent => "caller_is_parent",
            Self::CallerIsChild => "caller_is_child",
            Self::CallerIsRoot => "caller_is_root",
            Self::CallerIsSameCanister => "caller_is_same_canister",
            Self::CallerIsRegisteredToSubnet => "caller_is_registered_to_subnet",
            Self::CallerIsWhitelisted => "caller_is_whitelisted",
            Self::Authenticated { .. } => "authenticated",
            Self::BuildIcOnly => "build_ic_only",
            Self::BuildLocalOnly => "build_local_only",
        }
    }

    const fn metric_kind(self) -> AccessMetricKind {
        match self {
            Self::AppAllowsUpdates | Self::AppIsQueryable => AccessMetricKind::Guard,
            Self::SelfIsPrimeSubnet | Self::SelfIsPrimeRoot => AccessMetricKind::Env,
            Self::BuildIcOnly | Self::BuildLocalOnly => AccessMetricKind::Rule,
            Self::CallerIsController
            | Self::CallerIsParent
            | Self::CallerIsChild
            | Self::CallerIsRoot
            | Self::CallerIsSameCanister
            | Self::CallerIsRegisteredToSubnet
            | Self::CallerIsWhitelisted
            | Self::Authenticated { .. } => AccessMetricKind::Auth,
        }
    }
}

fn record_access_failure(ctx: &AccessContext, failure: AccessFailure) -> AccessError {
    AccessMetrics::increment(ctx.call, failure.metric_kind, failure.predicate);
    log!(
        Topic::Auth,
        Warn,
        "access denied kind={} predicate={} context={:?}: {}",
        failure.metric_kind.as_str(),
        failure.predicate,
        failure.context,
        failure.error,
    );
    failure.error
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access,
        ids::{EndpointCall, EndpointCallKind, EndpointId},
        storage::stable::env::{Env, EnvRecord},
        test::seams,
    };

    struct EnvRestore(EnvRecord);

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            Env::import(self.0.clone());
        }
    }

    #[test]
    fn caller_is_parent_matches_access_auth() {
        let _guard = seams::lock();
        let original = Env::export();
        let _restore = EnvRestore(original);

        let parent = seams::p(1);
        let other = seams::p(2);
        Env::import(EnvRecord {
            parent_pid: Some(parent),
            ..EnvRecord::default()
        });

        let expr = caller::is_parent();

        let ctx_parent = AccessContext {
            caller: parent,
            call: test_call(),
        };
        let ctx_other = AccessContext {
            caller: other,
            call: test_call(),
        };

        let expr_parent = futures::executor::block_on(eval_access(&expr, &ctx_parent));
        let auth_parent = futures::executor::block_on(access::auth::is_parent(parent));
        assert_eq!(expr_parent.is_ok(), auth_parent.is_ok());

        let expr_other = futures::executor::block_on(eval_access(&expr, &ctx_other));
        let auth_other = futures::executor::block_on(access::auth::is_parent(other));
        assert_eq!(expr_other.is_ok(), auth_other.is_ok());
    }

    fn test_call() -> EndpointCall {
        EndpointCall {
            endpoint: EndpointId::new("test"),
            kind: EndpointCallKind::Update,
        }
    }
}
