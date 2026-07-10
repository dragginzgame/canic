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
- Canister-to-canister service authorization uses explicit
  `SignedRoleAttestation` verification or public delegated-token authenticated
  endpoints.
- Subnet-registry caller predicates are internal-only endpoint rules. Public
  user ingress should use `auth::authenticated(...)`.

## Service Call Recipes

For application calls, expose a public delegated-token authenticated endpoint:

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
endpoints. For external scripts, expose a public endpoint and call it with
ordinary Candid:

```bash
env -u ICP_NETWORK icp canister call <shard> public_assign_project \
  '(principal "<user>", principal "<project>")' -e academic
```

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
- `Canister(canister_id)` is accepted only by that canister.
- `CanicSubnet(subnet_id)` is accepted only by a verifier on that Canic subnet.
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
