//! Access expression model and evaluator.
//!
//! This module defines the expression tree and async predicate interface
//! used to compose access control without changing existing semantics.

mod evaluators;

use crate::{
    access::{self, AccessError, metrics::AccessMetrics},
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
    // Raw transport identity from msg_caller().
    pub caller: Principal,
    // Resolved app/auth subject (raw caller or delegated session subject).
    pub authenticated_caller: Principal,
    // Source of the resolved authenticated subject.
    pub identity_source: access::auth::AuthenticatedIdentitySource,
    pub call: EndpointCall,
}

impl AccessContext {
    #[must_use]
    pub const fn transport_caller(&self) -> Principal {
        self.caller
    }

    #[must_use]
    pub const fn authenticated_subject(&self) -> Principal {
        self.authenticated_caller
    }
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
    App(AppPredicate),
    Caller(CallerPredicate),
    Environment(EnvironmentPredicate),
    Authenticated {
        required_scope: Option<&'static str>,
    },
}

impl BuiltinPredicate {
    /// name
    ///
    /// Return the stable metrics/logging name for this builtin predicate.
    fn name(self) -> &'static str {
        evaluators::name(self)
    }

    /// metric_kind
    ///
    /// Return the metric family used to classify this builtin predicate.
    fn metric_kind(self) -> AccessMetricKind {
        evaluators::metric_kind(self)
    }
}

///
/// AppPredicate
///

#[derive(Clone, Copy, Debug)]
pub enum AppPredicate {
    AllowsUpdates,
    IsQueryable,
}

///
/// CallerPredicate
///

#[derive(Clone, Copy, Debug)]
pub enum CallerPredicate {
    IsController,
    IsParent,
    IsChild,
    IsRoot,
    IsSameCanister,
    IsRegisteredToSubnet,
    IsWhitelisted,
}

///
/// EnvironmentPredicate
///

#[derive(Clone, Copy, Debug)]
pub enum EnvironmentPredicate {
    SelfIsPrimeSubnet,
    SelfIsPrimeRoot,
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
    use super::{AccessExpr, AppPredicate, BuiltinPredicate, builtin};

    #[must_use]
    pub const fn allows_updates() -> AccessExpr {
        builtin(BuiltinPredicate::App(AppPredicate::AllowsUpdates))
    }

    #[must_use]
    pub const fn is_queryable() -> AccessExpr {
        builtin(BuiltinPredicate::App(AppPredicate::IsQueryable))
    }
}

pub mod caller {
    use super::{AccessExpr, BuiltinPredicate, CallerPredicate, builtin};

    #[must_use]
    pub const fn is_controller() -> AccessExpr {
        builtin(BuiltinPredicate::Caller(CallerPredicate::IsController))
    }

    #[must_use]
    pub const fn is_parent() -> AccessExpr {
        builtin(BuiltinPredicate::Caller(CallerPredicate::IsParent))
    }

    #[must_use]
    pub const fn is_child() -> AccessExpr {
        builtin(BuiltinPredicate::Caller(CallerPredicate::IsChild))
    }

    #[must_use]
    pub const fn is_root() -> AccessExpr {
        builtin(BuiltinPredicate::Caller(CallerPredicate::IsRoot))
    }

    #[must_use]
    pub const fn is_same_canister() -> AccessExpr {
        builtin(BuiltinPredicate::Caller(CallerPredicate::IsSameCanister))
    }

    #[must_use]
    pub const fn is_registered_to_subnet() -> AccessExpr {
        builtin(BuiltinPredicate::Caller(
            CallerPredicate::IsRegisteredToSubnet,
        ))
    }

    #[must_use]
    pub const fn is_whitelisted() -> AccessExpr {
        builtin(BuiltinPredicate::Caller(CallerPredicate::IsWhitelisted))
    }
}

pub mod env {
    use super::{AccessExpr, BuiltinPredicate, EnvironmentPredicate, builtin};

    #[must_use]
    pub const fn is_prime_subnet() -> AccessExpr {
        builtin(BuiltinPredicate::Environment(
            EnvironmentPredicate::SelfIsPrimeSubnet,
        ))
    }

    #[must_use]
    pub const fn is_prime_root() -> AccessExpr {
        builtin(BuiltinPredicate::Environment(
            EnvironmentPredicate::SelfIsPrimeRoot,
        ))
    }

    #[must_use]
    pub const fn build_ic_only() -> AccessExpr {
        builtin(BuiltinPredicate::Environment(
            EnvironmentPredicate::BuildIcOnly,
        ))
    }

    #[must_use]
    pub const fn build_local_only() -> AccessExpr {
        builtin(BuiltinPredicate::Environment(
            EnvironmentPredicate::BuildLocalOnly,
        ))
    }
}

pub mod auth {
    use super::{AccessExpr, BuiltinPredicate, builtin};

    #[must_use]
    pub const fn authenticated(required_scope: Option<&'static str>) -> AccessExpr {
        builtin(BuiltinPredicate::Authenticated { required_scope })
    }

    #[must_use]
    pub const fn authenticated_with_scope(required_scope: &'static str) -> AccessExpr {
        authenticated(Some(required_scope))
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
                DefaultAppGuard::AllowsUpdates => {
                    BuiltinPredicate::App(AppPredicate::AllowsUpdates)
                }
                DefaultAppGuard::IsQueryable => BuiltinPredicate::App(AppPredicate::IsQueryable),
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
                AccessPredicate::Builtin(builtin) => evaluators::evaluate(*builtin, ctx)
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

const fn builtin(pred: BuiltinPredicate) -> AccessExpr {
    AccessExpr::Pred(AccessPredicate::Builtin(pred))
}

///
/// AccessFailure
///

#[derive(Debug)]
struct AccessFailure {
    error: AccessError,
    metric_kind: AccessMetricKind,
    predicate: &'static str,
    context: Option<&'static str>,
}

impl AccessFailure {
    fn from_builtin(pred: BuiltinPredicate, error: AccessError) -> Self {
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
        storage::stable::state::app::{AppMode, AppState, AppStateRecord},
        test::seams,
    };

    ///
    /// EnvRestore
    ///

    struct EnvRestore(EnvRecord);

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            Env::import(self.0.clone());
        }
    }

    ///
    /// AppRestore
    ///

    struct AppRestore(AppStateRecord);

    impl Drop for AppRestore {
        fn drop(&mut self) {
            AppState::import(self.0);
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
            authenticated_caller: parent,
            identity_source: access::auth::AuthenticatedIdentitySource::RawCaller,
            call: test_call(),
        };
        let ctx_other = AccessContext {
            caller: other,
            authenticated_caller: other,
            identity_source: access::auth::AuthenticatedIdentitySource::RawCaller,
            call: test_call(),
        };

        let expr_parent = futures::executor::block_on(eval_access(&expr, &ctx_parent));
        let auth_parent = futures::executor::block_on(access::auth::is_parent(parent));
        assert_eq!(expr_parent.is_ok(), auth_parent.is_ok());

        let expr_other = futures::executor::block_on(eval_access(&expr, &ctx_other));
        let auth_other = futures::executor::block_on(access::auth::is_parent(other));
        assert_eq!(expr_other.is_ok(), auth_other.is_ok());
    }

    #[test]
    fn app_allows_updates_matches_access_app_guard() {
        let _guard = seams::lock();
        let original = AppState::export();
        let _restore = AppRestore(original);
        let expr = app::allows_updates();
        let ctx = AccessContext {
            caller: seams::p(1),
            authenticated_caller: seams::p(1),
            identity_source: access::auth::AuthenticatedIdentitySource::RawCaller,
            call: test_call(),
        };

        for mode in [AppMode::Enabled, AppMode::Readonly, AppMode::Disabled] {
            AppState::import(AppStateRecord {
                mode,
                ..AppStateRecord::default()
            });
            let expr_result = futures::executor::block_on(eval_access(&expr, &ctx));
            let direct_result = access::app::guard_app_update();
            assert_eq!(
                expr_result.is_ok(),
                direct_result.is_ok(),
                "app::allows_updates parity mismatch for mode={mode:?}"
            );
        }
    }

    #[test]
    fn app_is_queryable_matches_access_app_guard() {
        let _guard = seams::lock();
        let original = AppState::export();
        let _restore = AppRestore(original);
        let expr = app::is_queryable();
        let ctx = AccessContext {
            caller: seams::p(1),
            authenticated_caller: seams::p(1),
            identity_source: access::auth::AuthenticatedIdentitySource::RawCaller,
            call: test_call(),
        };

        for mode in [AppMode::Enabled, AppMode::Readonly, AppMode::Disabled] {
            AppState::import(AppStateRecord {
                mode,
                ..AppStateRecord::default()
            });
            let expr_result = futures::executor::block_on(eval_access(&expr, &ctx));
            let direct_result = access::app::guard_app_query();
            assert_eq!(
                expr_result.is_ok(),
                direct_result.is_ok(),
                "app::is_queryable parity mismatch for mode={mode:?}"
            );
        }
    }

    #[test]
    fn build_network_predicates_match_env_access_checks() {
        let ctx = AccessContext {
            caller: seams::p(1),
            authenticated_caller: seams::p(1),
            identity_source: access::auth::AuthenticatedIdentitySource::RawCaller,
            call: test_call(),
        };

        let expr_local = futures::executor::block_on(eval_access(&env::build_local_only(), &ctx));
        let direct_local = access::env::build_network_local();
        assert_eq!(expr_local.is_ok(), direct_local.is_ok());

        let expr_ic = futures::executor::block_on(eval_access(&env::build_ic_only(), &ctx));
        let direct_ic = access::env::build_network_ic();
        assert_eq!(expr_ic.is_ok(), direct_ic.is_ok());
    }

    fn test_call() -> EndpointCall {
        EndpointCall {
            endpoint: EndpointId::new("test"),
            kind: EndpointCallKind::Update,
        }
    }

    #[test]
    fn caller_predicates_use_transport_caller_not_authenticated_subject() {
        let _guard = seams::lock();
        let original = Env::export();
        let _restore = EnvRestore(original);

        let parent = seams::p(10);
        let delegated_subject = seams::p(11);
        Env::import(EnvRecord {
            parent_pid: Some(parent),
            ..EnvRecord::default()
        });

        let expr = caller::is_parent();
        let ctx = AccessContext {
            caller: parent,
            authenticated_caller: delegated_subject,
            identity_source: access::auth::AuthenticatedIdentitySource::DelegatedSession,
            call: test_call(),
        };

        let result = futures::executor::block_on(eval_access(&expr, &ctx));
        assert!(result.is_ok());
    }
}
