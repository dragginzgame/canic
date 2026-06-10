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
  proof and are valid only on protected internal update endpoints. In `0.65`,
  fresh one-shot root ECDSA proof issuance for this surface is disabled.
- Protected internal endpoint macros emit descriptor metadata. These
  descriptors are retained as protocol metadata, but the outbound
  `CanicCall`, `CanicInternalClient`, and `canic_internal_client!` client
  surfaces are removed in `0.65` normal auth because no fresh
  internal-invocation proof can be issued by one update call.
- Historical `CanicCall` transported a protected envelope as raw ingress bytes
  to a no-argument protected wrapper. The retained wrapper decoder still maps
  malformed protected calls to typed Canic errors, but normal fresh calls should
  use delegated-token endpoints until a replacement protected-internal proof
  protocol exists.
- The generated descriptor accessor name is
  `canic_internal_endpoint_<endpoint>()`. Single-role descriptors expose the
  accepted caller role; multi-role descriptors require any future replacement
  client to choose the caller role explicitly.
- Cross-canister callers that cannot depend on the target implementation crate
  should put shared descriptors in a protocol module with
  `canic_protected_endpoint!`. Shared protocol modules may define a small
  descriptor table in one macro invocation. The descriptor remains the source
  of truth for method name and accepted-role metadata.
- The project hub/instance test fixture is the canonical app-style pattern:
  target canister keeps a protected `caller::has_role(...)` endpoint and a
  shared protocol crate owns the descriptor, but no normal client path calls it
  while fresh proof issuance is disabled.
- Protected internal endpoint descriptors must name a concrete exported method
  and at least one accepted caller role. Empty or whitespace-only descriptor
  metadata is invalid because it would create a generated client method that
  cannot request a method-scoped proof. Empty, whitespace-only, or duplicate
  caller roles are also invalid; role metadata is the protected client's
  authorization contract.
- The old AppIndex-only `caller::has_app_role(role)` predicate was removed in
  0.40 because verifier-local AppIndex state is not sufficient authorization
  for sibling Canic RPC.
- Subnet-registry caller predicates are internal-only endpoint rules. Public
  user ingress should use `auth::authenticated(...)`.

## Protected Internal Call Recipes

`0.65` status: fresh protected-internal calls are disabled in normal auth, and
the old outbound client APIs are removed. Use public delegated-token
authenticated endpoints for new hub-to-shard or parent-to-child application
calls. The example below documents the retained descriptor shape for existing
verification/rejection coverage and future replacement work; it is not a
working fresh-call recipe in `0.65`.

The target endpoint declares the accepted caller role:

```rust
use canic::cdk::types::Principal;

#[canic::canic_update(
    internal,
    name = "wire_assign_project",
    requires(caller::has_role("project_hub"))
)]
async fn assign_project(user_id: Principal, project_id: Principal) -> Result<(), canic::Error> {
    Ok(())
}
```

The endpoint macro emits `canic_internal_endpoint_assign_project()` as retained
metadata:

```rust
let descriptor = canic_internal_endpoint_assign_project();
assert_eq!(descriptor.method(), "wire_assign_project");
```

Shared protocol crates can publish the same metadata with
`canic_protected_endpoint!`:

```rust
use canic::api::canister::CanisterRole;

canic::canic_protected_endpoint! {
    pub fn shared_assign_project =
        "wire_assign_project",
        role = CanisterRole::new("project_hub");
}
```

For normal calls, expose a public delegated-token authenticated endpoint:

```rust
#[canic::canic_update(requires(auth::authenticated("project.assign")))]
async fn assign_project(token: canic::dto::auth::DelegatedToken, request: AssignRequest)
    -> Result<(), canic::Error>
{
    let _ = token;
    assign_project_impl(request).await
}
```

Raw `icp canister call` commands must call public, non-internal application
endpoints. A raw call to a protected internal endpoint with the original Candid
arguments is malformed because the protected wrapper expects Canic's internal
envelope and proof. For external scripts, expose a public endpoint and call it
with ordinary Candid:

```bash
env -u ICP_NETWORK icp canister call <shard> public_assign_project \
  '(principal "<user>", principal "<project>")' -e academic
```

## Error Boundary

- Access predicates return `Result<_, AccessError>`.
- `AccessError` is internal to access evaluation.
- Endpoint boundaries map access denials to public `canic::Error`
  (`Unauthorized` path).
- Protected internal wrapper decoding is a protocol boundary before access
  evaluation. Malformed raw ingress, unsupported envelope versions, and target
  binding mismatches map to `InternalRpcMalformed`, not `Unauthorized`.

## Endpoint Types

### Direct endpoints (supported)
- Caller provides a delegated token as the first candid argument.
- Endpoint guards apply `auth::authenticated("<required_scope>")`.
- Verification binds identity to transport principal:
  `verified.subject == ic_cdk::caller()`.

### Relayed endpoints (not supported)
- No relay authentication envelope is supported.
- No `presenter_pid` model is supported.
- No mode-branching between direct and relayed auth paths is supported.

## Delegated Auth Checks at Access Boundary

`access::auth::authenticated` enforces:
- token decode from ingress first argument succeeds
- root authority principal is available from env
- delegated token cryptographic and structural verification succeeds
- `token.claims.subject == caller`
- required scope exists in the token grant for the local canister role

Cryptographic and structural verification is delegated to
`ops::auth::AuthOps::verify_token`.

## Audience Binding

Audience answers which Canic boundary may accept the token:
- `Canic` is accepted by any Canic verifier.
- `Project(project_id)` is accepted only when the verifier's local project id
  matches `project_id`.

Authorization is carried by signed role grants:
- token audience must be accepted locally and be a subset of cert audience
- token grants must be a subset of cert grants
- local canister role must have a token grant
- endpoint required scopes must be present in the local-role grant

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
