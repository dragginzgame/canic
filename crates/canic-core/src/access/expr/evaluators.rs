//! Module: access::expr::evaluators
//!
//! Responsibility: map builtin access predicates to names, metrics, and checks.
//! Does not own: expression tree construction, custom predicates, or metrics storage.
//! Boundary: `access::expr` calls this while interpreting builtin predicate leaves.

use super::{
    AccessContext, BuiltinPredicate, CallerPredicate, EnvironmentPredicate, FleetPredicate,
};
use crate::{
    access::{self, AccessError, metrics::DelegatedAuthMetrics},
    ids::AccessMetricKind,
};

pub(super) const fn name(pred: &BuiltinPredicate) -> &'static str {
    match pred {
        BuiltinPredicate::Fleet(FleetPredicate::AllowsUpdates) => "fleet_allows_updates",
        BuiltinPredicate::Fleet(FleetPredicate::IsQueryable) => "fleet_is_queryable",
        BuiltinPredicate::Caller(CallerPredicate::IsController) => "caller_is_controller",
        BuiltinPredicate::Caller(CallerPredicate::IsParent) => "caller_is_parent",
        BuiltinPredicate::Caller(CallerPredicate::IsChild) => "caller_is_child",
        BuiltinPredicate::Caller(CallerPredicate::IsRoot) => "caller_is_root",
        BuiltinPredicate::Caller(CallerPredicate::IsSameCanister) => "caller_is_same_canister",
        BuiltinPredicate::Caller(CallerPredicate::IsWhitelisted) => "caller_is_whitelisted",
        BuiltinPredicate::Environment(EnvironmentPredicate::SelfIsFleetSubnetRoot) => {
            "self_is_fleet_subnet_root"
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::BuildIcOnly) => "build_ic_only",
        BuiltinPredicate::Environment(EnvironmentPredicate::BuildLocalOnly) => "build_local_only",
        BuiltinPredicate::Authenticated { .. } => "authenticated",
        BuiltinPredicate::AttestedLocalSubnet => "attested_local_subnet",
        BuiltinPredicate::ServiceAuthority { .. } => "deployment_service_authority",
    }
}

pub(super) const fn metric_kind(pred: &BuiltinPredicate) -> AccessMetricKind {
    match pred {
        BuiltinPredicate::Fleet(_) => AccessMetricKind::Guard,
        BuiltinPredicate::Caller(_)
        | BuiltinPredicate::Authenticated { .. }
        | BuiltinPredicate::AttestedLocalSubnet
        | BuiltinPredicate::ServiceAuthority { .. } => AccessMetricKind::Auth,
        BuiltinPredicate::Environment(EnvironmentPredicate::SelfIsFleetSubnetRoot) => {
            AccessMetricKind::Env
        }
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
        BuiltinPredicate::Fleet(FleetPredicate::AllowsUpdates) => {
            access::fleet::guard_fleet_update()
        }
        BuiltinPredicate::Fleet(FleetPredicate::IsQueryable) => access::fleet::guard_fleet_query(),
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
        BuiltinPredicate::Caller(CallerPredicate::IsWhitelisted) => {
            access::auth::is_whitelisted(ctx.caller).await
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::SelfIsFleetSubnetRoot) => {
            access::env::is_fleet_subnet_root()
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::BuildIcOnly) => {
            access::env::build_network_ic()
        }
        BuiltinPredicate::Environment(EnvironmentPredicate::BuildLocalOnly) => {
            access::env::build_network_local()
        }
        BuiltinPredicate::Authenticated { required_scope } => {
            let issuer_pid =
                access::auth::delegated_token_verified(ctx.authenticated_caller, *required_scope)?;
            DelegatedAuthMetrics::record_authority(issuer_pid);
            Ok(())
        }
        BuiltinPredicate::AttestedLocalSubnet => {
            access::auth::is_attested_local_subnet(ctx.caller).await
        }
        BuiltinPredicate::ServiceAuthority { service } => {
            access::deployment::require_service_authority(service)
        }
    }
}
