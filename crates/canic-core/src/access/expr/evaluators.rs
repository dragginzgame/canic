use super::{AccessContext, AppPredicate, BuiltinPredicate, CallerPredicate, EnvironmentPredicate};
use crate::{
    access::{self, AccessError, metrics::DelegatedAuthMetrics},
    ids::AccessMetricKind,
};
use async_trait::async_trait;

pub(super) fn name(pred: BuiltinPredicate) -> &'static str {
    evaluator(pred).name()
}

pub(super) fn metric_kind(pred: BuiltinPredicate) -> AccessMetricKind {
    evaluator(pred).metric_kind()
}

pub(super) async fn evaluate(
    pred: BuiltinPredicate,
    ctx: &AccessContext,
) -> Result<(), AccessError> {
    evaluator(pred).evaluate(ctx, pred).await
}

fn evaluator(pred: BuiltinPredicate) -> &'static dyn BuiltinPredicateEvaluator {
    match pred {
        BuiltinPredicate::App(AppPredicate::AllowsUpdates) => &APP_ALLOWS_UPDATES_EVALUATOR,
        BuiltinPredicate::App(AppPredicate::IsQueryable) => &APP_IS_QUERYABLE_EVALUATOR,
        BuiltinPredicate::Caller(CallerPredicate::IsController) => &CALLER_IS_CONTROLLER_EVALUATOR,
        BuiltinPredicate::Caller(CallerPredicate::IsParent) => &CALLER_IS_PARENT_EVALUATOR,
        BuiltinPredicate::Caller(CallerPredicate::IsChild) => &CALLER_IS_CHILD_EVALUATOR,
        BuiltinPredicate::Caller(CallerPredicate::IsRoot) => &CALLER_IS_ROOT_EVALUATOR,
        BuiltinPredicate::Caller(CallerPredicate::IsSameCanister) => {
            &CALLER_IS_SAME_CANISTER_EVALUATOR
        }
        BuiltinPredicate::Caller(CallerPredicate::IsRegisteredToSubnet) => {
            &CALLER_IS_REGISTERED_TO_SUBNET_EVALUATOR
        }
        BuiltinPredicate::Caller(CallerPredicate::IsWhitelisted) => {
            &CALLER_IS_WHITELISTED_EVALUATOR
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::SelfIsPrimeSubnet) => {
            &SELF_IS_PRIME_SUBNET_EVALUATOR
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::SelfIsPrimeRoot) => {
            &SELF_IS_PRIME_ROOT_EVALUATOR
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::BuildIcOnly) => {
            &BUILD_IC_ONLY_EVALUATOR
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::BuildLocalOnly) => {
            &BUILD_LOCAL_ONLY_EVALUATOR
        }
        BuiltinPredicate::Authenticated { .. } => &AUTHENTICATED_EVALUATOR,
    }
}

// --- Builtin Evaluators ---------------------------------------------------

#[async_trait]
trait BuiltinPredicateEvaluator: Send + Sync {
    // Execute a builtin predicate against the current access context.
    async fn evaluate(
        &self,
        ctx: &AccessContext,
        pred: BuiltinPredicate,
    ) -> Result<(), AccessError>;

    // Return the stable label used for metrics and logs.
    fn name(&self) -> &'static str;

    // Return the metric group for this builtin predicate evaluator.
    fn metric_kind(&self) -> AccessMetricKind;
}

///
/// AppAllowsUpdatesEvaluator
///

struct AppAllowsUpdatesEvaluator;

///
/// AppIsQueryableEvaluator
///

struct AppIsQueryableEvaluator;

///
/// SelfIsPrimeSubnetEvaluator
///

struct SelfIsPrimeSubnetEvaluator;

///
/// SelfIsPrimeRootEvaluator
///

struct SelfIsPrimeRootEvaluator;

///
/// CallerIsControllerEvaluator
///

struct CallerIsControllerEvaluator;

///
/// CallerIsParentEvaluator
///

struct CallerIsParentEvaluator;

///
/// CallerIsChildEvaluator
///

struct CallerIsChildEvaluator;

///
/// CallerIsRootEvaluator
///

struct CallerIsRootEvaluator;

///
/// CallerIsSameCanisterEvaluator
///

struct CallerIsSameCanisterEvaluator;

///
/// CallerIsRegisteredToSubnetEvaluator
///

struct CallerIsRegisteredToSubnetEvaluator;

///
/// CallerIsWhitelistedEvaluator
///

struct CallerIsWhitelistedEvaluator;

///
/// AuthenticatedEvaluator
///

struct AuthenticatedEvaluator;

///
/// BuildIcOnlyEvaluator
///

struct BuildIcOnlyEvaluator;

///
/// BuildLocalOnlyEvaluator
///

struct BuildLocalOnlyEvaluator;

static APP_ALLOWS_UPDATES_EVALUATOR: AppAllowsUpdatesEvaluator = AppAllowsUpdatesEvaluator;
static APP_IS_QUERYABLE_EVALUATOR: AppIsQueryableEvaluator = AppIsQueryableEvaluator;
static SELF_IS_PRIME_SUBNET_EVALUATOR: SelfIsPrimeSubnetEvaluator = SelfIsPrimeSubnetEvaluator;
static SELF_IS_PRIME_ROOT_EVALUATOR: SelfIsPrimeRootEvaluator = SelfIsPrimeRootEvaluator;
static CALLER_IS_CONTROLLER_EVALUATOR: CallerIsControllerEvaluator = CallerIsControllerEvaluator;
static CALLER_IS_PARENT_EVALUATOR: CallerIsParentEvaluator = CallerIsParentEvaluator;
static CALLER_IS_CHILD_EVALUATOR: CallerIsChildEvaluator = CallerIsChildEvaluator;
static CALLER_IS_ROOT_EVALUATOR: CallerIsRootEvaluator = CallerIsRootEvaluator;
static CALLER_IS_SAME_CANISTER_EVALUATOR: CallerIsSameCanisterEvaluator =
    CallerIsSameCanisterEvaluator;
static CALLER_IS_REGISTERED_TO_SUBNET_EVALUATOR: CallerIsRegisteredToSubnetEvaluator =
    CallerIsRegisteredToSubnetEvaluator;
static CALLER_IS_WHITELISTED_EVALUATOR: CallerIsWhitelistedEvaluator = CallerIsWhitelistedEvaluator;
static AUTHENTICATED_EVALUATOR: AuthenticatedEvaluator = AuthenticatedEvaluator;
static BUILD_IC_ONLY_EVALUATOR: BuildIcOnlyEvaluator = BuildIcOnlyEvaluator;
static BUILD_LOCAL_ONLY_EVALUATOR: BuildLocalOnlyEvaluator = BuildLocalOnlyEvaluator;

#[async_trait]
impl BuiltinPredicateEvaluator for AppAllowsUpdatesEvaluator {
    async fn evaluate(
        &self,
        _ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::app::guard_app_update()
    }

    fn name(&self) -> &'static str {
        "app_allows_updates"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Guard
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for AppIsQueryableEvaluator {
    async fn evaluate(
        &self,
        _ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::app::guard_app_query()
    }

    fn name(&self) -> &'static str {
        "app_is_queryable"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Guard
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for SelfIsPrimeSubnetEvaluator {
    async fn evaluate(
        &self,
        _ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::env::is_prime_subnet()
    }

    fn name(&self) -> &'static str {
        "self_is_prime_subnet"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Env
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for SelfIsPrimeRootEvaluator {
    async fn evaluate(
        &self,
        _ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::env::is_prime_root()
    }

    fn name(&self) -> &'static str {
        "self_is_prime_root"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Env
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for CallerIsControllerEvaluator {
    async fn evaluate(
        &self,
        ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::auth::is_controller(ctx.caller).await
    }

    fn name(&self) -> &'static str {
        "caller_is_controller"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Auth
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for CallerIsParentEvaluator {
    async fn evaluate(
        &self,
        ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::auth::is_parent(ctx.caller).await
    }

    fn name(&self) -> &'static str {
        "caller_is_parent"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Auth
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for CallerIsChildEvaluator {
    async fn evaluate(
        &self,
        ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::auth::is_child(ctx.caller).await
    }

    fn name(&self) -> &'static str {
        "caller_is_child"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Auth
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for CallerIsRootEvaluator {
    async fn evaluate(
        &self,
        ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::auth::is_root(ctx.caller).await
    }

    fn name(&self) -> &'static str {
        "caller_is_root"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Auth
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for CallerIsSameCanisterEvaluator {
    async fn evaluate(
        &self,
        ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::auth::is_same_canister(ctx.caller).await
    }

    fn name(&self) -> &'static str {
        "caller_is_same_canister"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Auth
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for CallerIsRegisteredToSubnetEvaluator {
    async fn evaluate(
        &self,
        ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::auth::is_registered_to_subnet(ctx.caller).await
    }

    fn name(&self) -> &'static str {
        "caller_is_registered_to_subnet"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Auth
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for CallerIsWhitelistedEvaluator {
    async fn evaluate(
        &self,
        ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::auth::is_whitelisted(ctx.caller).await
    }

    fn name(&self) -> &'static str {
        "caller_is_whitelisted"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Auth
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for AuthenticatedEvaluator {
    async fn evaluate(
        &self,
        ctx: &AccessContext,
        pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        let BuiltinPredicate::Authenticated { required_scope } = pred else {
            unreachable!("authenticated evaluator only handles authenticated predicates");
        };
        let verified =
            access::auth::delegated_token_verified(ctx.authenticated_caller, required_scope)?;
        DelegatedAuthMetrics::record_authority(verified.issuer_shard_pid);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "authenticated"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Auth
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for BuildIcOnlyEvaluator {
    async fn evaluate(
        &self,
        _ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::env::build_network_ic()
    }

    fn name(&self) -> &'static str {
        "build_ic_only"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Rule
    }
}

#[async_trait]
impl BuiltinPredicateEvaluator for BuildLocalOnlyEvaluator {
    async fn evaluate(
        &self,
        _ctx: &AccessContext,
        _pred: BuiltinPredicate,
    ) -> Result<(), AccessError> {
        access::env::build_network_local()
    }

    fn name(&self) -> &'static str {
        "build_local_only"
    }

    fn metric_kind(&self) -> AccessMetricKind {
        AccessMetricKind::Rule
    }
}
