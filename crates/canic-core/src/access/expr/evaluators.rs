use super::{AccessContext, AppPredicate, BuiltinPredicate, CallerPredicate, EnvironmentPredicate};
use crate::{
    access::{self, AccessError, metrics::DelegatedAuthMetrics},
    ids::AccessMetricKind,
};

pub(super) const fn name(pred: &BuiltinPredicate) -> &'static str {
    match pred {
        BuiltinPredicate::App(AppPredicate::AllowsUpdates) => "app_allows_updates",
        BuiltinPredicate::App(AppPredicate::IsQueryable) => "app_is_queryable",
        BuiltinPredicate::Caller(CallerPredicate::IsController) => "caller_is_controller",
        BuiltinPredicate::Caller(CallerPredicate::IsParent) => "caller_is_parent",
        BuiltinPredicate::Caller(CallerPredicate::IsChild) => "caller_is_child",
        BuiltinPredicate::Caller(CallerPredicate::IsRoot) => "caller_is_root",
        BuiltinPredicate::Caller(CallerPredicate::IsSameCanister) => "caller_is_same_canister",
        BuiltinPredicate::Caller(CallerPredicate::IsRegisteredToSubnet) => {
            "caller_is_registered_to_subnet"
        }
        BuiltinPredicate::Caller(CallerPredicate::IsWhitelisted) => "caller_is_whitelisted",
        BuiltinPredicate::Environment(EnvironmentPredicate::SelfIsPrimeSubnet) => {
            "self_is_prime_subnet"
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::SelfIsPrimeRoot) => {
            "self_is_prime_root"
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::BuildIcOnly) => "build_ic_only",
        BuiltinPredicate::Environment(EnvironmentPredicate::BuildLocalOnly) => "build_local_only",
        BuiltinPredicate::Authenticated { .. } => "authenticated",
    }
}

pub(super) const fn metric_kind(pred: &BuiltinPredicate) -> AccessMetricKind {
    match pred {
        BuiltinPredicate::App(_) => AccessMetricKind::Guard,
        BuiltinPredicate::Caller(_) | BuiltinPredicate::Authenticated { .. } => {
            AccessMetricKind::Auth
        }
        BuiltinPredicate::Environment(
            EnvironmentPredicate::SelfIsPrimeSubnet | EnvironmentPredicate::SelfIsPrimeRoot,
        ) => AccessMetricKind::Env,
        BuiltinPredicate::Environment(
            EnvironmentPredicate::BuildIcOnly | EnvironmentPredicate::BuildLocalOnly,
        ) => AccessMetricKind::Rule,
    }
}

pub(super) async fn evaluate(
    pred: &BuiltinPredicate,
    ctx: &AccessContext,
) -> Result<(), AccessError> {
    match pred {
        BuiltinPredicate::App(AppPredicate::AllowsUpdates) => access::app::guard_app_update(),
        BuiltinPredicate::App(AppPredicate::IsQueryable) => access::app::guard_app_query(),
        BuiltinPredicate::Caller(CallerPredicate::IsController) => {
            access::auth::is_controller(ctx.caller).await
        }
        BuiltinPredicate::Caller(CallerPredicate::IsParent) => {
            access::auth::is_parent(ctx.caller).await
        }
        BuiltinPredicate::Caller(CallerPredicate::IsChild) => {
            access::auth::is_child(ctx.caller).await
        }
        BuiltinPredicate::Caller(CallerPredicate::IsRoot) => {
            access::auth::is_root(ctx.caller).await
        }
        BuiltinPredicate::Caller(CallerPredicate::IsSameCanister) => {
            access::auth::is_same_canister(ctx.caller).await
        }
        BuiltinPredicate::Caller(CallerPredicate::IsRegisteredToSubnet) => {
            access::auth::is_registered_to_subnet(ctx.caller).await
        }
        BuiltinPredicate::Caller(CallerPredicate::IsWhitelisted) => {
            access::auth::is_whitelisted(ctx.caller).await
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::SelfIsPrimeSubnet) => {
            access::env::is_prime_subnet()
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::SelfIsPrimeRoot) => {
            access::env::is_prime_root()
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::BuildIcOnly) => {
            access::env::build_network_ic()
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::BuildLocalOnly) => {
            access::env::build_network_local()
        }
        BuiltinPredicate::Authenticated { required_scope } => {
            let issuer_shard_pid = access::auth::delegated_token_verified(
                ctx.authenticated_caller,
                *required_scope,
                ctx.call.kind,
            )?;
            DelegatedAuthMetrics::record_authority(issuer_shard_pid);
            Ok(())
        }
    }
}
