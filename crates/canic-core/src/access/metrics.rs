use crate::{
    cdk::types::Principal,
    ids::{AccessMetricKind, EndpointCall},
    ops,
};

///
/// Access-layer metrics façade.
///
/// WHY THIS FILE EXISTS
/// ---------------------
/// This module intentionally sits at the *access layer* boundary and serves
/// as the **only approved way** for access rules, guards, and macro-expanded
/// endpoints to emit metrics.
///
/// It exists to enforce the following architectural invariants:
///
/// 1. **Layering discipline**
///    Access logic MUST NOT depend directly on ops/runtime metric backends.
///    All metric emission from access control flows through this façade.
///
/// 2. **Stable call surface**
///    Endpoint identity, metric kinds, and authority attribution are
///    intentionally normalized here so internal metric schemas may evolve
///    without touching callers.
///
/// 3. **Future-proofing**
///    This layer is the designated place to introduce:
///      - metric sampling or rate limiting
///      - cardinality controls
///      - backend changes (heap → stable → off-canister)
///      - lifecycle validation (attempted → completed → result)
///
/// If this file appears "thin", that is by design.
/// DO NOT bypass it by calling ops::runtime::metrics directly.
///

///
/// AccessMetrics
///
/// Access-denial metrics by stage.
///
/// Invariants:
/// - Emitted only on denial and represent the stage where access failed.
/// - Called exactly once per denied request.
/// - Cardinality is bounded by endpoint name + AccessMetricKind.
///
pub struct AccessMetrics;

impl AccessMetrics {
    pub fn increment(call: EndpointCall, kind: AccessMetricKind) {
        // Intentionally forward only the normalized endpoint name.
        // Callers must not depend on backend naming conventions.
        ops::runtime::metrics::access::AccessMetrics::increment(call.endpoint.name, kind);
    }
}

///
/// EndpointAttemptMetrics
///
/// Endpoint lifecycle metrics.
///
/// These metrics describe *execution flow*, not authorization:
///   attempted → completed → (ok | err)
///
/// Invariants:
/// - `increment_attempted` is emitted before user code runs.
/// - `increment_completed` is emitted exactly once per attempt.
/// - This module does NOT enforce ordering; enforcement may be added here later.
///
pub struct EndpointAttemptMetrics;

impl EndpointAttemptMetrics {
    pub fn increment_attempted(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointAttemptMetrics::increment_attempted(
            call.endpoint.name,
        );
    }

    pub fn increment_completed(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointAttemptMetrics::increment_completed(
            call.endpoint.name,
        );
    }
}

///
/// EndpointResultMetrics
///
/// Endpoint result metrics (success vs failure).
///
/// These metrics intentionally exclude *why* a call failed; that information
/// belongs in logs, not in high-cardinality metric labels.
///
/// Invariants:
/// - Exactly one of `increment_ok` or `increment_err` is emitted per completed call.
/// - Must be emitted after `increment_completed`.
///
pub struct EndpointResultMetrics;

impl EndpointResultMetrics {
    pub fn increment_ok(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointResultMetrics::increment_ok(call.endpoint.name);
    }

    pub fn increment_err(call: EndpointCall) {
        ops::runtime::metrics::endpoint::EndpointResultMetrics::increment_err(call.endpoint.name);
    }
}

///
/// DelegationMetrics
///
/// Delegated authorization authority metrics.
///
/// Records which delegation authority (cert signer) was used to
/// successfully validate a delegated token.
///
/// WHY THIS LIVES HERE:
/// - Access rules are the *only* place where delegation validity is known.
/// - Downstream layers must not infer authority from request context.
/// - This ensures cryptographic attribution remains tightly scoped.
///
/// Invariants:
/// - Called only after cryptographic verification succeeds.
/// - Must not be called on denied or partially-verified tokens.
/// - Cardinality is bounded by active delegation authorities.
///
pub struct DelegationMetrics;

impl DelegationMetrics {
    pub fn record_authority(authority: Principal) {
        ops::runtime::metrics::delegation::DelegationMetrics::record_authority(authority);
    }
}
