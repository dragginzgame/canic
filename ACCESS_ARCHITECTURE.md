# Access Architecture

This document describes how access control is modeled, evaluated, and surfaced
in Canic. It focuses on the access layer and how it plugs into endpoints,
workflow, and ops without breaking layering rules.

## Policy Families

Access policy is split into three families. Each family has a single purpose
and an explicit module in the access layer.

- app: application mode gating (updates allowed, queries allowed)
- auth: caller identity, topology, registry roles, delegated tokens
- env: build/network and environment predicates

These families map to code in `crates/canic-core/src/access/`.

## DSL Namespaces vs Policy Families

The access DSL is a thin, macro-friendly surface that composes predicates into
an `AccessExpr`. Its namespaces are not one-to-one with policy families.

DSL namespaces:
- app: app mode guards
- caller: caller identity and topology checks
- env: environment checks
- auth: delegated token checks

Policy families:
- app, auth, env

Key point: `caller::*` is part of the auth family. The `caller` DSL namespace
exists to make endpoint predicates read clearly, not to define a new policy
family. `caller` does not replace `access::auth`.

## AccessError vs canic::Error

`AccessError` is an internal, access-layer error that carries only a denial
reason. It is not a public API error and must not cross the API boundary.

`canic::Error` is the public DTO returned to callers. Access errors are
converted into `InternalError` and then into the public error code
(`ErrorCode::Unauthorized`) at the boundary.

Rule: access helpers return `Result<_, AccessError>` and never `canic::Error`.

## Metrics

Access and endpoint metrics are emitted through `access::metrics`.
This facade exists to keep access code independent of ops/runtime metric
backends and to preserve stable call semantics.

Key invariants:
- access denials emit exactly one access metric
- successful access emits no access metric
- endpoint lifecycle metrics are recorded by macro wrappers, not predicates

Implementation lives in `crates/canic-core/src/access/metrics.rs`, with the
backing stores in `ops::runtime::metrics::*`.

## When to Use the DSL vs access::auth

Use the DSL (`requires(...)`, `caller::*`, `app::*`, `env::*`, `auth::*`) when:
- you are defining endpoint access policies via macros
- you want declarative composition (all/any/not/custom)
- the check is part of the endpoint boundary contract

Use `access::auth` directly when:
- you need imperative composition inside workflow or ops
- you are implementing a custom predicate or non-macro guard
- you are already in async control flow and need explicit branching

In both cases, return `AccessError` from access code and let endpoints map it
to public errors.

## Boundary Summary

- endpoints/macros: declare access rules and emit lifecycle metrics
- access::expr: evaluates access expressions and records denial metrics
- access::{app,auth,env}: predicate implementations only
- ops/workflow: may call access::auth directly, never embed access logic in macros
