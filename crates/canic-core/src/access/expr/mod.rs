//! Module: access::expr
//!
//! Responsibility: define and evaluate composable endpoint access expressions.
//! Does not own: concrete predicate storage, endpoint response mapping, or workflow policy.
//! Boundary: endpoint macros build expressions here and receive access-layer denials.

mod evaluators;

use crate::{
    access,
    access::{AccessError, metrics::AccessMetrics},
    cdk::types::Principal,
};
use canic_contracts::ids::{AccessMetricKind, EndpointCall};
use std::sync::Arc;

///
/// AccessContext
///
/// Runtime context available to access predicates during endpoint evaluation.
/// Owned by access expression evaluation and supplied by endpoint macros.
///

#[derive(Clone, Debug)]
pub struct AccessContext {
    // Raw transport identity from msg_caller().
    pub caller: Principal,
    pub call: EndpointCall,
}

impl AccessContext {
    #[must_use]
    pub const fn transport_caller(&self) -> Principal {
        self.caller
    }
}

///
/// DefaultFleetGuard
///
/// Synchronous Fleet-mode guards used for implicit gating.
/// Owned by access expressions and evaluated before custom predicate trees.
///

pub enum DefaultFleetGuard {
    AllowsUpdates,
    IsQueryable,
}

///
/// AccessExpr
///
/// Composable access expression tree for endpoint guards.
/// Owned by access expressions and evaluated by endpoint access plumbing.
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
/// Leaf access predicate backed by a builtin guard or custom synchronous predicate.
/// Owned by access expressions and evaluated by the expression interpreter.
///

#[derive(Clone)]
pub enum AccessPredicate {
    Builtin(BuiltinPredicate),
    Custom(Arc<dyn AccessCheck>),
}

///
/// BuiltinPredicate
///
/// Builtin predicate selector for Fleet, caller, environment, and auth checks.
/// Owned by access expressions and mapped to concrete access modules.
///

#[derive(Clone, Debug)]
pub enum BuiltinPredicate {
    CallerPermission { permission: &'static str },
    Fleet(FleetPredicate),
    Caller(CallerPredicate),
    Environment(EnvironmentPredicate),
    AttestedLocalSubnetArgument(Arc<crate::dto::auth::SignedRoleAttestation>),
    ServiceAuthority { service: &'static str },
}

impl BuiltinPredicate {
    /// name
    ///
    /// Return the stable metrics/logging name for this builtin predicate.
    const fn name(&self) -> &'static str {
        evaluators::name(self)
    }

    /// metric_kind
    ///
    /// Return the metric family used to classify this builtin predicate.
    const fn metric_kind(&self) -> AccessMetricKind {
        evaluators::metric_kind(self)
    }
}

///
/// FleetPredicate
///
/// Fleet-mode predicate selector.
/// Owned by access expressions and evaluated by `access::fleet`.
///

#[derive(Clone, Copy, Debug)]
pub enum FleetPredicate {
    AllowsUpdates,
    IsQueryable,
}

///
/// CallerPredicate
///
/// Caller and topology predicate selector.
/// Owned by access expressions and evaluated by `access::auth`.
///

#[derive(Clone, Debug)]
pub enum CallerPredicate {
    IsController,
    IsFleetAdmitted,
    IsParent,
    IsChild,
    IsRoot,
    IsSameCanister,
}

///
/// EnvironmentPredicate
///
/// Environment and build-network predicate selector.
/// Owned by access expressions and evaluated by `access::env`.
///

#[derive(Clone, Copy, Debug)]
pub enum EnvironmentPredicate {
    SelfIsFleetSubnetRoot,
    BuildIcOnly,
    BuildLocalOnly,
}

///
/// AccessCheck
///
/// Synchronous custom admission check evaluated before endpoint dispatch.
/// Implementations inspect supplied proofs and local authority without network I/O.
///
pub trait AccessCheck: Send + Sync {
    ///
    /// Custom predicate contract:
    /// - Runs synchronously using supplied proofs and local authority; no network I/O.
    /// - May depend on Fleet state.
    /// - Must be side-effect free, idempotent, and must not panic.
    ///
    fn eval(&self, ctx: &AccessContext) -> Result<(), AccessError>;
    fn name(&self) -> &'static str;
}

/// all
///
/// Build an expression that requires all nested predicates to pass.
pub fn all<I>(exprs: I) -> AccessExpr
where
    I: IntoIterator<Item = AccessExpr>,
{
    AccessExpr::All(exprs.into_iter().collect())
}

/// any
///
/// Build an expression that allows any nested predicate to pass.
pub fn any<I>(exprs: I) -> AccessExpr
where
    I: IntoIterator<Item = AccessExpr>,
{
    AccessExpr::Any(exprs.into_iter().collect())
}

/// not
///
/// Build an expression that denies access when the nested predicate passes.
/// Reached internal errors and unavailable build-network configuration propagate
/// as evaluation failures through negation and alternative branches.
#[must_use]
pub fn not(expr: AccessExpr) -> AccessExpr {
    AccessExpr::Not(Box::new(expr))
}

/// custom
///
/// Wrap a custom synchronous predicate as an access expression leaf.
pub fn custom<P>(pred: P) -> AccessExpr
where
    P: AccessCheck + 'static,
{
    AccessExpr::Pred(AccessPredicate::Custom(Arc::new(pred)))
}

pub mod fleet {
    use super::{AccessExpr, BuiltinPredicate, FleetPredicate, builtin};

    #[must_use]
    pub const fn allows_updates() -> AccessExpr {
        builtin(BuiltinPredicate::Fleet(FleetPredicate::AllowsUpdates))
    }

    #[must_use]
    pub const fn is_queryable() -> AccessExpr {
        builtin(BuiltinPredicate::Fleet(FleetPredicate::IsQueryable))
    }
}

pub mod caller {
    use super::{AccessExpr, BuiltinPredicate, CallerPredicate, builtin};

    /// Select one receiver-compiled managed caller permission.
    #[must_use]
    pub const fn has_permission(permission: &'static str) -> AccessExpr {
        builtin(BuiltinPredicate::CallerPermission { permission })
    }

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
    pub const fn is_fleet_admitted() -> AccessExpr {
        builtin(BuiltinPredicate::Caller(CallerPredicate::IsFleetAdmitted))
    }
}

pub mod env {
    use super::{AccessExpr, BuiltinPredicate, EnvironmentPredicate, builtin};

    #[must_use]
    pub const fn is_fleet_subnet_root() -> AccessExpr {
        builtin(BuiltinPredicate::Environment(
            EnvironmentPredicate::SelfIsFleetSubnetRoot,
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
    use crate::dto::role_attestation::SignedRoleAttestation;
    use std::sync::Arc;

    /// Verify an already decoded role attestation without reading the message again.
    /// The receiver must authorize `attestation.payload.role` against its allowed
    /// roles before performing application effects.
    #[must_use]
    pub fn attested_local_subnet_argument(attestation: &SignedRoleAttestation) -> AccessExpr {
        builtin(BuiltinPredicate::AttestedLocalSubnetArgument(Arc::new(
            attestation.clone().into(),
        )))
    }
}

pub mod deployment {
    use super::{AccessExpr, BuiltinPredicate, builtin};

    #[must_use]
    pub const fn is_service_authority(service: &'static str) -> AccessExpr {
        builtin(BuiltinPredicate::ServiceAuthority { service })
    }
}

/// eval_access
///
/// Mandatory managed startup gate, evaluated once before any endpoint access expression.
pub fn eval_application_startup(call: EndpointCall) -> Result<(), AccessError> {
    crate::workflow::runtime::application_startup::require_endpoint_started(call).map_err(|error| {
        record_access_failure(
            call,
            AccessFailure {
                error,
                metric_kind: AccessMetricKind::Auth,
                predicate: "application_startup",
                terminal: true,
            },
        )
    })
}

/// Evaluate an access expression and record a normalized denial on failure.
pub fn eval_access(expr: &AccessExpr, ctx: &AccessContext) -> Result<(), AccessError> {
    eval_access_selected::<true>(expr, ctx)
}

/// Evaluate a generated expression with its statically selected admission reader.
///
/// Endpoint generation selects this reader from the complete declared expression.
/// Manually composed expressions use `eval_access`, which supports every predicate.
/// A mismatched selection is terminal and cannot be inverted by `not` or `any`.
#[doc(hidden)]
pub fn eval_access_selected<const FLEET_ADMISSION: bool>(
    expr: &AccessExpr,
    ctx: &AccessContext,
) -> Result<(), AccessError> {
    match eval_access_inner::<FLEET_ADMISSION>(expr, ctx) {
        Ok(()) => Ok(()),
        Err(failure) => Err(record_access_failure(ctx.call, failure)),
    }
}

/// eval_default_fleet_guard
///
/// Evaluate an implicit Fleet guard and record a normalized denial on failure.
pub fn eval_default_fleet_guard(
    guard: DefaultFleetGuard,
    call: EndpointCall,
) -> Result<(), AccessError> {
    let result = match guard {
        DefaultFleetGuard::AllowsUpdates => access::fleet::guard_fleet_update(),
        DefaultFleetGuard::IsQueryable => access::fleet::guard_fleet_query(),
    };

    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            let predicate = match guard {
                DefaultFleetGuard::AllowsUpdates => {
                    BuiltinPredicate::Fleet(FleetPredicate::AllowsUpdates)
                }
                DefaultFleetGuard::IsQueryable => {
                    BuiltinPredicate::Fleet(FleetPredicate::IsQueryable)
                }
            };
            Err(record_access_failure(
                call,
                AccessFailure::from_builtin(&predicate, err),
            ))
        }
    }
}

fn eval_access_inner<const FLEET_ADMISSION: bool>(
    expr: &AccessExpr,
    ctx: &AccessContext,
) -> Result<(), AccessFailure> {
    match expr {
        AccessExpr::All(exprs) => {
            if exprs.is_empty() {
                return Err(AccessFailure::no_predicates());
            }
            for expr in exprs {
                eval_access_inner::<FLEET_ADMISSION>(expr, ctx)?;
            }
            Ok(())
        }
        AccessExpr::Any(exprs) => {
            if exprs.is_empty() {
                return Err(AccessFailure::no_predicates());
            }
            let mut last = None;
            for expr in exprs {
                match eval_access_inner::<FLEET_ADMISSION>(expr, ctx) {
                    Ok(()) => return Ok(()),
                    Err(failure) if failure.terminal => return Err(failure),
                    Err(failure) => last = Some(failure),
                }
            }
            Err(last.unwrap_or_else(AccessFailure::no_predicates))
        }
        AccessExpr::Not(expr) => match eval_access_inner::<FLEET_ADMISSION>(expr, ctx) {
            Ok(()) => Err(AccessFailure::negated()),
            Err(failure) if failure.terminal => Err(failure),
            Err(_) => Ok(()),
        },
        AccessExpr::Pred(pred) => match pred {
            AccessPredicate::Builtin(builtin) => {
                evaluators::evaluate::<FLEET_ADMISSION>(builtin, ctx).map_err(|failure| {
                    match failure {
                        evaluators::BuiltinFailure::Denied(error) => {
                            AccessFailure::from_builtin(builtin, error)
                        }
                        evaluators::BuiltinFailure::Unselected => {
                            AccessFailure::unselected(builtin)
                        }
                    }
                })
            }
            AccessPredicate::Custom(custom) => custom
                .eval(ctx)
                .map_err(|err| AccessFailure::from_custom(custom.name(), err)),
        },
    }
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
    terminal: bool,
}

impl AccessFailure {
    const fn from_builtin(pred: &BuiltinPredicate, error: AccessError) -> Self {
        Self {
            terminal: matches!(
                error,
                AccessError::Internal(_) | AccessError::BuildNetworkUnavailable
            ),
            error,
            metric_kind: pred.metric_kind(),
            predicate: pred.name(),
        }
    }

    fn unselected(pred: &BuiltinPredicate) -> Self {
        Self {
            terminal: true,
            ..Self::from_builtin(pred, AccessError::ExpressionRuleRequired)
        }
    }

    const fn from_custom(name: &'static str, error: AccessError) -> Self {
        Self {
            terminal: matches!(
                error,
                AccessError::Internal(_) | AccessError::BuildNetworkUnavailable
            ),
            error,
            metric_kind: AccessMetricKind::Custom,
            predicate: name,
        }
    }

    const fn no_predicates() -> Self {
        Self {
            error: AccessError::ExpressionRuleRequired,
            metric_kind: AccessMetricKind::Auth,
            predicate: "no_rules",
            terminal: false,
        }
    }

    const fn negated() -> Self {
        Self {
            error: AccessError::NegatedPredicateMatched,
            metric_kind: AccessMetricKind::Auth,
            predicate: "not",
            terminal: false,
        }
    }
}

fn record_access_failure(call: EndpointCall, failure: AccessFailure) -> AccessError {
    AccessMetrics::increment(call, failure.metric_kind, failure.predicate);
    failure.error
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access,
        config::schema::CanisterKind,
        ops::runtime::{log::LogOps, metrics::access::AccessMetrics as MetricStore},
        storage::stable::{
            env::{Env, EnvData, EnvRecord},
            state::fleet::{FleetMode, FleetState, FleetStateData, FleetStateRecord},
        },
        test::{
            config::{ConfigTestBuilder, install_model_for_role},
            seams,
        },
    };
    use canic_contracts::ids::{EndpointCall, EndpointCallKind, EndpointId};
    use std::sync::Mutex;

    ///
    /// EnvRestore
    ///

    struct EnvRestore(EnvData);

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            Env::import(self.0.clone());
        }
    }

    ///
    /// FleetRestore
    ///

    struct FleetRestore(FleetStateData);

    impl Drop for FleetRestore {
        fn drop(&mut self) {
            FleetState::import(self.0);
        }
    }

    #[test]
    fn caller_is_parent_matches_access_auth() {
        let _guard = seams::lock();
        let original = Env::export();
        let _restore = EnvRestore(original);

        let parent = seams::p(1);
        let other = seams::p(2);
        Env::import(EnvData {
            record: EnvRecord {
                parent_pid: Some(parent),
                ..EnvRecord::default()
            },
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

        let expr_parent = eval_access(&expr, &ctx_parent);
        let auth_parent = access::auth::is_parent(parent);
        assert_eq!(expr_parent.is_ok(), auth_parent.is_ok());

        let expr_other = eval_access(&expr, &ctx_other);
        let auth_other = access::auth::is_parent(other);
        assert_eq!(expr_other.is_ok(), auth_other.is_ok());
    }

    #[test]
    fn fleet_allows_updates_matches_access_fleet_guard() {
        let _guard = seams::lock();
        let original = FleetState::export();
        let _restore = FleetRestore(original);
        let expr = fleet::allows_updates();
        let ctx = AccessContext {
            caller: seams::p(1),
            call: test_call(),
        };

        for mode in [FleetMode::Enabled, FleetMode::Readonly, FleetMode::Disabled] {
            FleetState::import(FleetStateData {
                record: FleetStateRecord {
                    mode,
                    ..FleetStateRecord::default()
                },
            });
            let expr_result = eval_access(&expr, &ctx);
            let direct_result = access::fleet::guard_fleet_update();
            assert_eq!(
                expr_result.is_ok(),
                direct_result.is_ok(),
                "fleet::allows_updates parity mismatch for mode={mode:?}"
            );
        }
    }

    #[test]
    fn fleet_is_queryable_matches_access_fleet_guard() {
        let _guard = seams::lock();
        let original = FleetState::export();
        let _restore = FleetRestore(original);
        let expr = fleet::is_queryable();
        let ctx = AccessContext {
            caller: seams::p(1),
            call: test_call(),
        };

        for mode in [FleetMode::Enabled, FleetMode::Readonly, FleetMode::Disabled] {
            FleetState::import(FleetStateData {
                record: FleetStateRecord {
                    mode,
                    ..FleetStateRecord::default()
                },
            });
            let expr_result = eval_access(&expr, &ctx);
            let direct_result = access::fleet::guard_fleet_query();
            assert_eq!(
                expr_result.is_ok(),
                direct_result.is_ok(),
                "fleet::is_queryable parity mismatch for mode={mode:?}"
            );
        }
    }

    #[test]
    fn build_network_predicates_match_env_access_checks() {
        let ctx = AccessContext {
            caller: seams::p(1),
            call: test_call(),
        };

        let expr_local = eval_access(&env::build_local_only(), &ctx);
        let direct_local = access::env::build_network_local();
        assert_eq!(expr_local.is_ok(), direct_local.is_ok());

        let expr_ic = eval_access(&env::build_ic_only(), &ctx);
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
    fn denial_bursts_preserve_operator_log_entries_and_exact_metrics() {
        let _guard = seams::lock();
        let role = canic_contracts::ids::CanisterRole::new("app");
        let mut config = ConfigTestBuilder::new()
            .with_default_canister_kind(role.clone(), CanisterKind::Service)
            .build();
        config.log.max_entries = 2;
        let _config = install_model_for_role(config, role);
        crate::log::set_ready();
        LogOps::reset_for_tests();
        for (created_at, message) in [(1, "bootstrap failure"), (2, "funding recovery")] {
            LogOps::append_runtime_log(
                "operator",
                None,
                crate::log::Level::Error,
                message,
                created_at,
                2,
                1_024,
            )
            .unwrap();
        }
        let page = crate::dto::page::PageRequest {
            limit: 10,
            offset: 0,
        };
        let retained = LogOps::page_filtered(None, None, None, page)
            .entries
            .into_iter()
            .map(|entry| entry.message)
            .collect::<Vec<_>>();
        let _restore = EnvRestore(Env::export());
        Env::import(EnvData {
            record: EnvRecord {
                parent_pid: Some(seams::p(1)),
                ..EnvRecord::default()
            },
        });
        let ctx = AccessContext {
            caller: seams::p(2),
            call: test_call(),
        };
        MetricStore::reset();
        for _ in 0..32 {
            assert!(matches!(
                eval_access(&caller::is_parent(), &ctx),
                Err(AccessError::ParentRequired)
            ));
        }
        assert_eq!(
            LogOps::page_filtered(None, None, None, page)
                .entries
                .into_iter()
                .map(|entry| entry.message)
                .collect::<Vec<_>>(),
            retained
        );
        let metrics = MetricStore::snapshot().entries;
        assert_eq!(metrics.len(), 1);
        let (key, count) = &metrics[0];
        assert_eq!(key.endpoint, ctx.call.endpoint.name);
        assert_eq!(key.kind, AccessMetricKind::Auth);
        assert_eq!(key.predicate, "caller_is_parent");
        assert_eq!(count.exact_total(), Ok(32));
        LogOps::append_runtime_log(
            "operator",
            None,
            crate::log::Level::Info,
            "ordinary operator log",
            3,
            2,
            1_024,
        )
        .unwrap();
        let logs = LogOps::page_filtered(None, None, None, page);
        assert_eq!(logs.entries.len(), 2);
        assert_eq!(logs.entries[0].message, "ordinary operator log");
        LogOps::reset_for_tests();
    }

    #[test]
    fn caller_predicates_use_the_exact_transport_caller() {
        let _guard = seams::lock();
        let original = Env::export();
        let _restore = EnvRestore(original);

        let parent = seams::p(10);
        Env::import(EnvData {
            record: EnvRecord {
                parent_pid: Some(parent),
                ..EnvRecord::default()
            },
        });

        let expr = caller::is_parent();
        let ctx = AccessContext {
            caller: parent,
            call: test_call(),
        };

        let result = eval_access(&expr, &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn unselected_admission_cannot_be_inverted_or_accepted_by_another_branch() {
        let ctx = AccessContext {
            caller: seams::p(1),
            call: test_call(),
        };
        for expr in [
            caller::is_fleet_admitted(),
            not(caller::is_fleet_admitted()),
            any([caller::is_fleet_admitted(), not(all([]))]),
            not(all([caller::is_fleet_admitted()])),
        ] {
            let result = eval_access_selected::<false>(&expr, &ctx);
            std::assert_matches!(result, Err(AccessError::ExpressionRuleRequired));
        }
    }

    #[test]
    fn evaluation_failures_cannot_be_negated_or_bypassed() {
        struct Check(fn() -> Result<(), AccessError>);

        impl AccessCheck for Check {
            fn eval(&self, _: &AccessContext) -> Result<(), AccessError> {
                (self.0)()
            }

            fn name(&self) -> &'static str {
                "evaluation_failure"
            }
        }

        let ctx = AccessContext {
            caller: seams::p(1),
            call: test_call(),
        };
        let errors: [fn() -> Result<(), AccessError>; 2] = [
            || Err(AccessError::Internal(crate::InternalError::invariant())),
            || Err(AccessError::BuildNetworkUnavailable),
        ];
        for (index, error) in errors.into_iter().enumerate() {
            let failure = || custom(Check(error));
            let unreachable = || custom(Check(|| panic!("evaluation must stop on failure")));
            for expression in [
                not(failure()),
                not(all([failure(), unreachable()])),
                not(any([failure(), unreachable()])),
                any([not(failure()), unreachable()]),
                not(not(failure())),
            ] {
                for result in [
                    eval_access(&expression, &ctx),
                    eval_access_selected::<false>(&expression, &ctx),
                ] {
                    match index {
                        0 => std::assert_matches!(result, Err(AccessError::Internal(_))),
                        _ => {
                            std::assert_matches!(result, Err(AccessError::BuildNetworkUnavailable));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn selected_evaluator_preserves_boolean_order_and_denial_metrics() {
        struct Probe {
            id: u8,
            succeeds: bool,
            calls: Arc<Mutex<Vec<u8>>>,
        }
        impl AccessCheck for Probe {
            fn eval(&self, _: &AccessContext) -> Result<(), AccessError> {
                self.calls.lock().unwrap().push(self.id);
                if self.succeeds {
                    Ok(())
                } else {
                    Err(AccessError::ParentRequired)
                }
            }
            fn name(&self) -> &'static str {
                "probe"
            }
        }
        let ctx = AccessContext {
            caller: seams::p(1),
            call: test_call(),
        };
        let calls = Arc::new(Mutex::new(Vec::new()));
        let probe = |id, succeeds| {
            custom(Probe {
                id,
                succeeds,
                calls: Arc::clone(&calls),
            })
        };
        let cases = [
            (all([probe(1, false), probe(2, true)]), vec![1], false),
            (
                any([probe(1, false), probe(2, true), probe(3, false)]),
                vec![1, 2],
                true,
            ),
            (
                not(all([probe(1, true), probe(2, false)])),
                vec![1, 2],
                true,
            ),
            (
                not(any([probe(1, false), probe(2, true)])),
                vec![1, 2],
                false,
            ),
            (all([]), vec![], false),
            (any([]), vec![], false),
        ];
        for (expr, expected_calls, allowed) in cases {
            let mut observations = Vec::new();
            for full in [false, true] {
                calls.lock().unwrap().clear();
                MetricStore::reset();
                let result = if full {
                    eval_access(&expr, &ctx)
                } else {
                    eval_access_selected::<false>(&expr, &ctx)
                };
                assert_eq!(result.is_ok(), allowed);
                assert_eq!(*calls.lock().unwrap(), expected_calls);
                let entries = MetricStore::snapshot().entries;
                assert_eq!(
                    entries
                        .iter()
                        .map(|(_, count)| count.exact_total().unwrap())
                        .sum::<u128>(),
                    u128::from(!allowed)
                );
                observations.push((
                    result.err().map(|error| std::mem::discriminant(&error)),
                    entries
                        .into_iter()
                        .map(|(key, count)| (key.kind, key.predicate, count))
                        .collect::<Vec<_>>(),
                ));
            }
            assert_eq!(observations[0], observations[1]);
        }
    }
}
