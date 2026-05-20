# Access Architecture Contract

This document defines how access predicates are composed and enforced at
endpoint boundaries.

## Policy Families

Access checks are grouped into three policy families:
- `app`: app mode gates (update/query availability).
- `auth`: caller/topology/registry checks and delegated-token verification.
- `env`: environment/build/network predicates.

Implementation root:
- `crates/canic-core/src/access/`

## DSL Namespaces vs Policy Families

The macro DSL namespaces are:
- `app::*`
- `caller::*`
- `env::*`
- `auth::*`

`caller::*` belongs to the auth family. It is a readability namespace, not a
separate policy family.

## Topology Caller Checks

- Parent, child, root, self, controller, whitelist, and subnet-registry checks
  use the raw transport caller, not delegated user identity.
- `caller::has_role(role)` and `caller::has_any_role([...])` are protected
  internal-call predicates. They require a root-signed method-scoped invocation
  proof and are valid only on protected internal update endpoints.
- Protected internal endpoint macros emit descriptor metadata for generated
  clients. Typed internal clients should use `CanicInternalClient` and those
  descriptors so method names, envelope proof scope, and accepted caller roles
  come from the protected endpoint declaration instead of being duplicated at
  call sites.
- The generated descriptor accessor name is
  `canic_internal_endpoint_<endpoint>()`; `canic_internal_client!` consumes
  those accessors to generate typed protected update client methods. Single-role
  descriptors can infer the caller role; multi-role descriptors require an
  explicit `role = ...` clause in the generated client method declaration.
- Generated protected clients carry `CanicInternalCallOptions` for wait mode,
  attached cycles, and requested proof TTL. These transport knobs must stay on
  the protected client path and must not require callers to bypass descriptor
  metadata or use raw `Call`.
- The old AppIndex-only `caller::has_app_role(role)` predicate was removed in
  0.40 because verifier-local AppIndex state is not sufficient authorization
  for sibling Canic RPC.
- Subnet-registry caller predicates are internal-only endpoint rules. Public
  user ingress should use `auth::authenticated(...)`.

## Error Boundary

- Access predicates return `Result<_, AccessError>`.
- `AccessError` is internal to access evaluation.
- Endpoint boundaries map access denials to public `canic::Error`
  (`Unauthorized` path).

## Endpoint Types

### Direct endpoints (supported)
- Caller provides a delegated token as the first candid argument.
- Endpoint guards apply `auth::authenticated("<required_scope>")`.
- Verification binds identity to transport principal:
  `verified.claims.sub == ic_cdk::caller()`.

### Relayed endpoints (not supported)
- No relay authentication envelope is supported.
- No `presenter_pid` model is supported.
- No mode-branching between direct and relayed auth paths is supported.

## Delegated Auth Checks at Access Boundary

`access::auth::authenticated` enforces:
- token decode from ingress first argument succeeds
- root authority principal is available from env
- delegated token cryptographic and structural verification succeeds
- `token.claims.sub == caller`
- required scope exists in token claims

Cryptographic and structural verification is delegated to
`ops::auth::AuthOps::verify_token`.

## Audience Binding

Audience is explicit allow-listing:
- verifier canister must be in token audience (`self_pid in token.claims.aud`)
- token audience entries must be allowed by cert audience

This is enforced in `ops::auth` before access is granted.

## Root Authority Source

Root trust anchor comes from environment state (`EnvOps::root_pid()`).
Environment import enforces root immutability after first initialization.

## Metrics Contract

- access denials emit one access-denial metric
- successful access emits no denial metric
- endpoint lifecycle metrics are emitted by macro wrappers, not predicates

Implementation:
- `crates/canic-core/src/access/metrics.rs`

## Related Contracts

- `docs/contracts/ARCHITECTURE.md`
- `docs/contracts/AUTH_DELEGATED_SIGNATURES.md`
