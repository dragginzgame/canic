# Access Architecture Contract

This document defines how access predicates are composed and enforced at
endpoint boundaries.

## Policy Families

Access checks are grouped into three policy families:
- `fleet`: Fleet mode gates (update/query availability).
- `auth`: caller/topology/registry checks and role-attestation verification.
- `env`: environment/build/network predicates.

Implementation root:
- `crates/canic-core/src/access/`

## DSL Namespaces vs Policy Families

The macro DSL namespaces are:
- `fleet::*`
- `caller::*`
- `env::*`
- `auth::*`
- `deployment::*`

`caller::*` belongs to the auth family. It is a readability namespace, not a
separate policy family.

## Topology Caller Checks

- Parent, child, root, self, controller, whitelist, and subnet-registry checks
  use the raw transport caller, not delegated user identity.
- Canister-to-canister service authorization uses exact receiver-local authority
  or explicit `SignedRoleAttestation` verification with the required role.
- Application session admission remains unavailable while portable Auth is wired.
  A service-authority guard must not be substituted for application account auth.

## Error Boundary

- Access predicates return `Result<_, AccessError>`.
- `AccessError` is internal to access evaluation.
- Endpoint boundaries map access denials to public `canic::Error`
  (`Unauthorized` path).
- By default, guarded endpoints return `Result<T, E>` with `E: From<canic::Error>`.
  An endpoint may explicitly select `on_access_denied = "reject"` to send an IC
  rejection on access refusal while preserving its declared success Candid type.
  This uses `msg_reject`, not a trap: update denial metrics remain committed.
  It changes only access refusal; a handler's declared `Result` remains a Result
  on the wire. Argument decoding and preflight failures retain their own behavior.
- Both modes use the same short-circuit evaluator and default Fleet guards.
  `public` keeps the Fleet guard; application endpoints must not select `internal`
  to obtain a plain reply. Refusal prevents handler execution.
- Endpoint admission is synchronous in both modes. Builtin and custom checks
  use local authority and supplied proofs; they cannot await network I/O. Custom
  checks implement `canic::access::AccessCheck` with synchronous `eval` and
  remain free of application effects. Generated wrappers are async only when
  the handler is async. Proof issuance, provisioning and renewal may still await
  external calls outside admission.

See [endpoint argument and reply controls](../features/runtime/update-payload-limits.md#bounded-decoding-and-plain-replies)
for the public API.

## Supplied role attestations

`auth::attested_local_subnet()` binds a `SignedRoleAttestation` to the caller
and the receiving subnet. The endpoint also checks the required role. With
`decode = LIMITS`, the guard consumes the bounded decoded first argument without
reading or decoding the message again. Root signature, audience, time and epoch
checks remain local.

## Root Authority Source

Root trust anchor comes from environment state (`EnvOps::root_pid()`).
Environment import enforces root immutability after first initialization.

## Metrics Contract

- access denials emit one access-denial metric
- successful access emits no denial metric
- endpoint lifecycle metrics are emitted by macro wrappers, not predicates
- rejection-mode update denials commit the same single access metric; ordinary
  query execution does not persist metric writes

Implementation:
- `crates/canic-core/src/access/metrics.rs`

## Related Contracts

- `docs/contracts/ARCHITECTURE.md`
- `docs/contracts/AUTH_DELEGATED_SIGNATURES.md`

## Continue From Here

- [Read the authentication design](../architecture/authentication.md)
- [Browse the architecture guides](../architecture/README.md)
- [Browse all documentation](../README.md)
- [Back to the main README](../../README.md)
