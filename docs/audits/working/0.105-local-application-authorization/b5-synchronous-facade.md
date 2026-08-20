# 0.105 B5 Synchronous-Facade Evidence

Date: 2026-08-19

This working evidence records B5's framework-neutral synchronous decision
surface and generic consumer. It is not a release artifact or a substitute for
the maintainer-owned complete validation gate.

## One public decision owner

`canic::access::auth::authorize_local_application` is the sole public local
application authorization function. The same module owns the request and
re-exports the model/policy-owned scope, decision, denial and authorized-subject
types. The removed `access::application_authorization` module has no alias or
facade re-export.

The synchronous adapter reads `msg_caller` and current IC time exactly once.
Anonymous or observer-mismatched calls close before protected authority or
session state is read. Otherwise it projects one protected local authority,
performs one exact-caller session lookup, determines current subject
admissibility and passes borrowed values to the pure policy. It performs no
await, timer, cleanup, storage mutation, logging or application-data access.

Both local-session establishment and the facade consume
`AuthOps::local_application_authorization_authority`; there is no second
configuration/Fleet/role/generation projection. The proof-bearing native guard
calls `AuthOps::verify_token` once with the caller and required scopes. Its
former access-layer scope check was deleted, so presenter binding and scope
verification remain in the one verifier projection.

## Duplicate-path hard cut

B5 deletes the subject-only `resolve_authenticated_identity` fallback,
`AuthenticatedIdentitySource`, `ResolvedAuthenticatedIdentity`, the
`AccessContext::authenticated_caller` field and every generated Root, managed
and Wasm Store use of them. Endpoint access expressions now receive only the
exact transport caller. A repository source scan finds no retained fallback,
identity-source field or authenticated-caller context.

Literal `auth::authenticated("scope")` expansion passes through
`canic::application_scope!`, whose const block invokes the one model-owned
scope grammar at compile time. Path expressions remain typed application
scope inputs and the verifier performs the one runtime authorization check.

## Generic consumer

The `delegation_issuer_stub` test canister contains one generic application
query with a token-free application ABI. It passes its observed transport
caller plus a statically validated scope to the synchronous facade, returns the
authorized subject on `Allow` and maps every closed Canic denial to the
fixture's compact unauthorized error. No Canic token, session DTO, framework
type, registry, remote call or async decision appears in that application
contract; the function is declared async only because the existing endpoint
macro accepts async application functions, while its facade call is
synchronous.

The PEM-backed native-agent PocketIC journey proves the endpoint denies before
establishment, returns the exact caller after establishment, and denies
immediately after caller clear. The same journey retains B4's exact-retry,
caller-self status and proof-tombstone non-resurrection assertions.

## Focused validation

- `cargo test --locked -p canic-core synchronous_local_application_facade --lib`:
  two passed.
- `cargo test --locked -p canic-macros access_stage_ --lib`: two passed.
- `cargo test --locked -p canic-macros authenticated_endpoint_expansion_fences_before_access_and_dispatch --lib`:
  one passed and proves literal scopes use `application_scope!`.
- `cargo test --locked -p canic --doc application_scope`: one passing example
  and one passing compile-fail example.
- `cargo test --locked -p canic --test protocol_surface local_application_authorization_facade_has_one_public_owner`:
  one passed.
- `cargo test --locked -p canic-core caller_predicates_use_the_exact_transport_caller --lib`:
  one passed.
- `cargo check --locked -p canic-wasm-store -p canic-control-plane -p canic -p delegation_issuer_stub`:
  passed.
- Warning-denied Clippy passed for `canic-core`, `canic`, `canic-macros`,
  `canic-wasm-store`, `delegation_issuer_stub` and the focused
  `native_agent_delegation` integration target.
- `cargo test --locked -p canic-tests --test native_agent_delegation pem_backed_native_agent_prepares_retrieves_and_presents_delegated_token -- --test-threads=1 --nocapture`
  against repository-pinned PocketIC 15.0.0: one passed in 224.96 seconds,
  including cold canonical Store, Root and issuer artifact builds.

The first final PocketIC attempt exposed stale generated Root and Wasm Store
references to the deleted identity fallback before a canister was installed.
Those generated paths were hard-cut to the exact transport caller; their
focused host compilation and the subsequent complete PocketIC journey pass.

B5 adds no production endpoint, Candid method, stable field, timer, lifecycle
owner, dependency or framework-specific contract. It changes the public Rust
facade and test-only generic consumer, removes the duplicate identity path and
converges proof/session authority reads. B6 owns operator inspection, aggregate
metrics and bounded resource measurements.
