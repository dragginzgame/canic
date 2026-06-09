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
- `CanicCall` transports the protected envelope as raw ingress bytes to a
  no-argument protected wrapper. The envelope is decoded inside Canic code so
  malformed protected calls return typed Canic errors instead of pre-dispatch
  Candid decode traps.
- The generated descriptor accessor name is
  `canic_internal_endpoint_<endpoint>()`; `canic_internal_client!` consumes
  those accessors to generate typed protected update client methods. Single-role
  descriptors can infer the caller role; multi-role descriptors require an
  explicit `role = ...` clause in the generated client method declaration.
- Cross-canister callers that cannot depend on the target implementation crate
  should put shared descriptors in a protocol module with
  `canic_protected_endpoint!`, then bind `canic_internal_client!` methods to
  those shared descriptor functions. Shared protocol modules may define a small
  descriptor table in one macro invocation. The descriptor remains the source of
  truth for method name and accepted-role metadata.
- The project hub/instance test fixture is the canonical app-style pattern:
  target canister implements a protected `caller::has_role(...)` endpoint, a
  shared protocol crate owns the descriptor, and the caller canister generates
  a typed client from that descriptor.
- Protected internal endpoint descriptors must name a concrete exported method
  and at least one accepted caller role. Empty or whitespace-only descriptor
  metadata is invalid because it would create a generated client method that
  cannot request a method-scoped proof. Empty, whitespace-only, or duplicate
  caller roles are also invalid; role metadata is the protected client's
  authorization contract.
- Generated protected clients carry `CanicInternalCallOptions` for wait mode,
  attached cycles, and requested proof TTL. These transport knobs must stay on
  the protected client path and must not require callers to bypass descriptor
  metadata or use raw `Call`.
- The old AppIndex-only `caller::has_app_role(role)` predicate was removed in
  0.40 because verifier-local AppIndex state is not sufficient authorization
  for sibling Canic RPC.
- Subnet-registry caller predicates are internal-only endpoint rules. Public
  user ingress should use `auth::authenticated(...)`.

## Protected Internal Call Recipes

For hub-to-shard or parent-to-child calls, prefer a generated internal client
bound to the protected endpoint descriptor. The target endpoint declares the
accepted caller role:

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

The endpoint macro emits `canic_internal_endpoint_assign_project()`. The caller
can use that descriptor with `canic_internal_client!`:

```rust
canic::canic_internal_client! {
    pub struct ProjectHubInternalClient {
        fn assign_project = canic_internal_endpoint_assign_project; (
            user_id: Principal,
            project_id: Principal,
        ) -> ();
    }
}

let client = ProjectHubInternalClient::new(shard_pid);
client.assign_project(user_id, project_id).await?;
```

If the endpoint accepts multiple caller roles, choose the role explicitly in the
client method declaration:

```rust
use canic::api::canister::CanisterRole;

canic::canic_internal_client! {
    pub struct ProjectHubInternalClient {
        fn admin_repair = shared_multi_role_project_endpoint,
            role = CanisterRole::new("admin_hub"); (
            project_id: Principal,
        ) -> ();
    }
}
```

For a one-off lower-level call, still pass the generated descriptor and the
parent/hub caller role explicitly:

```rust
use canic::{
    api::{canister::CanisterRole, ic::CanicInternalClient},
    cdk::types::Principal,
};

let response: MyResponse = CanicInternalClient::new(shard_pid)
    .call_update_result(
        &canic_internal_endpoint_assign_project(),
        CanisterRole::new("project_hub"),
        (user_id, project_id),
    )
    .await?;
```

The third argument is a Candid argument tuple. Use `(arg,)` for one argument and
`(a, b)` for two arguments.

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
