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
/// as the **only approved way** for access predicates and macro-expanded
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
///
/// If this file appears "thin", that is by design.
/// DO NOT bypass it by calling ops::runtime::metrics directly.
///

///
/// AccessMetrics
///
/// Access-denial metrics by predicate kind and name.
///
/// Invariants:
/// - Emitted only on denial and represent the kind where access failed.
/// - Called exactly once per denied request.
/// - Cardinality is bounded by endpoint name + kind + predicate name.
/// - Custom predicates are attributed to AccessMetricKind::Custom.
///
pub struct AccessMetrics;

impl AccessMetrics {
    /// Record one denied endpoint access predicate.
    pub fn increment(call: EndpointCall, kind: AccessMetricKind, predicate: &'static str) {
        // Intentionally forward only the normalized endpoint name.
        // Callers must not depend on backend naming conventions.
        ops::runtime::metrics::access::AccessMetrics::increment(
            call.endpoint.name,
            kind,
            predicate,
        );
    }
}

///
/// DelegatedAuthMetrics
///
/// Delegated authorization authority metrics.
///
/// Records which delegated-auth authority (cert signer) was used to
/// successfully validate a delegated token.
///
/// WHY THIS LIVES HERE:
/// - Access predicates are the *only* place where delegation validity is known.
/// - Downstream layers must not infer authority from request context.
/// - This ensures cryptographic attribution remains tightly scoped.
///
/// Invariants:
/// - Called only after cryptographic verification succeeds.
/// - Must not be called on denied or partially-verified tokens.
/// - Cardinality is bounded by active delegation authorities.
///
pub struct DelegatedAuthMetrics;

impl DelegatedAuthMetrics {
    /// Record which delegated-auth authority verified a request.
    pub fn record_authority(authority: Principal) {
        ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics::record_authority(authority);
    }
}
