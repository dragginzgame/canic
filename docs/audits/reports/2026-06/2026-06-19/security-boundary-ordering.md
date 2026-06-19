# Security Boundary Ordering Audit - 2026-06-19

## Report Preamble

- Definition path:
  `docs/audits/recurring/system/security-boundary-ordering.md`
- Scope: endpoint delegated-token ordering, generated endpoint access
  sequencing, root proof provisioning prepare/get/install ordering,
  issuer-local delegated-token prepare/get ordering, root replay sequencing,
  and capability-envelope proof handling.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-01/security-boundary-ordering.md`
- Code snapshot identifier: `ef55e53c`
- Method tag/version: `security-boundary-ordering/current-root-proof-provisioning`
- Comparability status: `non-comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-19`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This audit was selected as the next stale recurring system audit. The previous
report still described protected internal invocation proof wrappers and the old
root/shard trust-chain wording. The current implementation has hard-cut those
paths and moved the relevant ordering risk into root proof provisioning,
configured root/issuer canister-signature verification, and issuer-local
active proof installation.

## Audit Definition Maintenance

The audit definition was updated before execution. The live checklist now
targets the current split:

- public delegated-token endpoint auth through `auth::authenticated(...)`;
- root proof provisioning through root/controller update paths, direct root
  query retrieval, and issuer-local active proof install;
- signed role-attestation verification as an explicit local proof check;
- retired protected internal proof wrappers and old role/principal delegated
  token audience shapes as negative scans.

The definition now scans `api/auth`, `workflow/runtime/auth`, and
`ops/auth/delegation` because the 0.68 repair makes root proof provisioning a
first-class security ordering surface.

## Executive Summary

Risk: **3 / 10**.

No security boundary ordering bypass was found.

Current endpoint delegated-token auth still follows:

```text
decode token -> verify token material and proof chain -> bind subject/caller
-> enforce scope -> dispatch handler
```

Current root proof provisioning follows:

```text
root policy/prepare update -> direct root get query -> root install update
-> issuer-local active proof verification/storage -> issuer-local token prepare/get
```

The direct root query requirement is represented in code and comments, and the
root canister-signature helper maps missing query data certificates to a typed
`RootDataCertificateUnavailable` public error. Root install does not assemble
proofs during an update; it validates submitted proofs against pending metadata
before calling issuer canisters.

No production remediation was needed. The recurring audit definition was
refreshed to match the hard-cut 0.68 auth/provisioning model.

## Audit Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Endpoint token verification before endpoint authorization | PASS | `access/auth/token.rs` calls `AuthOps::verify_token`, then subject binding, then required-scope enforcement. |
| No verifier-local delegated-token use store | PASS | Access auth scans found no token-use consume/update path; guard test pins the absence. |
| Macro access before dispatch | PASS | `endpoint/expand` emits access evaluation before dispatch; macro test passed. |
| Retired protected internal proof paths absent | PASS | Static scan found no active `verify_internal_invocation_proof`, `InternalInvocationProof`, `caller::has_role`, or `caller::has_any_role` paths. |
| Explicit verifier trust anchors before embedded proof verification | PASS | `AuthOps::verify_token` resolves delegated-token config, current-canister verifier support, local context, and explicit root verifier config before embedded proof verification. |
| Positive proof-cache hits rerun local checks | PASS | Cache hits call `verify_delegated_token_cached_proof_identity(...)` before success. |
| Root batch prepare validates before committing metadata | PASS | Prepare preflight checks metadata, entry limit, issuer policy, TTL, audiences, grants, replay, and quota before proof preparation. |
| Root batch get validates before proof assembly | PASS | Get checks pending metadata, root/issuer/cert hash consistency, and retrieval expiry before calling root proof assembly. |
| Missing query certificate has typed failure | PASS | Root canister-signature error mapping produces `RootDataCertificateUnavailable`; public error mapping test passed. |
| Root batch install preflights before issuer calls | PASS | Workflow install calls `preflight_delegation_proof_batch_install_proof(...)` before any issuer install call. |
| Issuer active proof install verifies before storage | PASS | Active proof install verifies configured root canister/root key and local issuer binding before `set_active_delegation_proof(...)`. |
| Issuer-local token prepare/get stays local after install | PASS | Prepare requires active proof and `issuer_pid == self`; get retrieves by `claims_hash` plus caller/prepared-by key. |
| Root replay commits only after execution success | PASS | Replay-first path aborts on authorization or execution failure and commits after successful execution. |
| Capability proof surface remains hard-cut | PASS | `CapabilityProof` has only `Structural`; capability hash binding tests passed. |

## Verification Ordering Map

| Boundary | Ordering | Verdict |
| --- | --- | --- |
| Public delegated-token endpoint auth | Decode first arg, verify through `AuthOps`, bind verified subject to caller, enforce required scope, then dispatch | Pass |
| Generated endpoint wrappers | Build access context, evaluate default/app or explicit access, return on denial, dispatch only after success | Pass |
| Retired internal proof wrappers | Old protected internal proof and caller-role predicate names are absent from active Rust source | Pass |
| Delegated-token runtime verifier | Config/current-canister/local context checks precede cache/proof success; embedded root and issuer proofs verify before acceptance on cache misses | Pass |
| Root proof prepare | Root/controller update path validates metadata, replay, quotas, issuer registry policy, TTL, audiences, and grants before pending metadata/proof preparation | Pass |
| Root proof get | Controller-gated root query validates pending metadata and retrieval expiry before assembling root proof material | Pass |
| Root proof install | Root update validates submitted proofs against pending metadata before broadcasting issuer-local installs | Pass |
| Issuer active proof install | Issuer verifies configured root authority and local issuer binding before active state storage | Pass |
| Root RPC replay | Replay-first mode aborts fresh reservations on denial/failure and commits only after successful execution | Pass |
| Capability envelope | Structural proof mode only; hash binding covers target canister, version, and canonical capability payload | Pass |

## Trust-Boundary Table

| Trust Boundary | Source Of Truth | Cache/Metric Status | Notes |
| --- | --- | --- | --- |
| Endpoint delegated token | `AuthOps::verify_token(...)` plus delegated verifier modules | Positive proof cache only | Cache hits still rerun canonical, audience, grant, subject, and scope checks. |
| Endpoint subject binding | `VerifiedDelegatedToken.subject` and authenticated caller | No cache | Binding is enforced after proof verification and before required-scope success. |
| Root proof batch metadata | Root pending metadata store | Bounded and pruned | Prepare is replay/idempotency protected and quota checked. |
| Root proof retrieval | Direct root query plus root certified data | No update assembly | Missing query data certificate maps to `RootDataCertificateUnavailable`. |
| Root proof batch install | Pending metadata plus submitted proof payload | No trust in provisioner proof alone | Root preflight validates before issuer call; issuer performs full active proof verification. |
| Issuer active proof state | Issuer-local auth state | Non-secret status query | State is stored only after root proof and issuer-self checks. |
| Root replay receipt | Replay ops store | Pending/committed receipts | Commit follows successful execution; abort handles denial/failure. |
| Capability proof | Structural proof plus canonical hash binding | No auth cache | Retired role-attestation/delegated-grant capability proof variants remain absent. |
| Metrics | Runtime metrics stores | Observability only | Metrics are not authorization inputs. |

## Endpoint Delegated Token Analysis

`crates/canic-core/src/access/auth/token.rs` keeps the endpoint guard order
mechanical:

1. decode `DelegatedToken` from the first ingress argument;
2. call `AuthOps::verify_token(...)`;
3. enforce `verified.subject == caller`;
4. enforce the required endpoint scope;
5. return the issuer principal to the access layer.

The guard has explicit tests for nanosecond time, verify/bind/scope order, and
absence of a verifier-local token-use store.

Static scans found no active old delegated-token audience shapes such as
`RolesOrPrincipals`, `DelegationAudience::Roles`, or plural role/principal
compatibility shims in the auth path.

## Endpoint Macro Sequencing Analysis

Generated wrappers still evaluate access before handler dispatch.
`endpoint/expand/access.rs` resolves the authenticated identity, builds
`AccessContext`, calls `eval_access(...)`, returns on denial, and only then
allows dispatch from `endpoint/expand/mod.rs`.

The macro test
`authenticated_endpoint_expansion_evaluates_access_before_dispatch` passed.

The retired protected internal proof wrapper scan returned no active source
matches for `verify_internal_invocation_proof`, `InternalInvocationProof`,
`request_internal_invocation_proof`, `caller::has_role`, or
`caller::has_any_role`.

## Delegated Token Material Verification

`AuthOps::verify_token(...)` now orders runtime checks as:

```text
record started -> delegated-token config -> current-canister verifier gate
-> local canister/subnet/project context -> canonical cache key
-> positive cache identity check or explicit root verifier config
-> embedded root and issuer proof verification -> cache insert -> success metric
```

The pure verifier still enforces canonical cert/claims hashes, cert issuance
rules, token time windows, audience/grant/scope rules, issuer proof binding,
and issuer proof availability before returning success.

The embedded proof verifier resolves `AuthProofVerifierConfig` explicitly.
Tests cover mainnet, local, PocketIC, and testnet root-key discipline, and
reject missing or wrong root-key configuration.

## Root Proof Provisioning Ordering

The root auth endpoint macro exposes:

- `canic_upsert_root_issuer_policy` as a controller-gated update;
- `canic_prepare_delegation_proof_batch` as a controller-gated update;
- `canic_get_delegation_proof_batch` as a controller-gated query;
- `canic_install_delegation_proof_batch` as a controller-gated update.

`AuthApi` additionally calls `EnvOps::require_root()` on these root paths.

Batch prepare validates issuer registry policy, TTL, audience, grant, request
metadata, replay/idempotency, and quotas before root proof preparation and
pending metadata creation.

Batch get validates pending metadata and retrieval expiry before calling the
root canister-signature proof helper. The module-level invariant says this get
path runs only as a direct root query so the root canister has
`data_certificate()`. If the proof helper observes the IC data-certificate
error, it maps to `RootDataCertificateUnavailable`.

Batch install accepts retrieved proof payloads plus batch identifiers. The root
workflow validates cert hash, issuer, expiry, and pending metadata before
calling issuer canisters. It does not reassemble proofs during the install
update.

## Issuer-Local Prepare/Get Ordering

Issuer-local delegated-token prepare/get remains independent of root after an
active proof is installed:

- `AuthApi::prepare_delegated_token(...)` and `get_delegated_token(...)` first
  require the delegated-token issuer config;
- prepare requires an unexpired active proof and rejects if
  `active_proof.proof.cert.issuer_pid != IcOps::canister_self()`;
- prepare creates delegated-token claims and issuer canister-signature material
  only after the active proof check;
- get retrieves pending token material by `claims_hash` and caller/prepared-by
  binding before attaching the issuer proof.

This preserves the intended availability boundary: root is a provisioning and
renewal authority, not a normal delegated-token login dependency before proof
expiry.

## Replay Sequencing Analysis

Root RPC replay-first handling remains ordered:

1. reserve or return cached replay;
2. authorize the fresh request;
3. abort the reservation if authorization fails;
4. execute the authorized capability;
5. abort the reservation if execution fails;
6. commit replay output after successful execution.

The focused replay tests passed, including denial-before-replay behavior for
the test-only authorize-then-replay mode, replay-before-policy validation,
abort-on-denial, cached duplicate response, conflicting payload rejection,
cross-variant request-id rejection, capacity limits, and recovery-required
receipt preservation.

## RPC Capability Handling Review

The capability surface is currently structural only. `CapabilityProof` and
`RootCapabilityProof` each have a single `Structural` mode. Capability envelope
validation still checks the service and capability version, while
`verify_capability_hash_binding(...)` binds target canister, capability
version, and canonical payload.

The old capability proof modes that embedded role-attestation or delegated-grant
success paths did not reappear.

## Residual Watchpoints

| Area | Risk | Note |
| --- | --- | --- |
| `crates/canic-core/src/access/auth/token.rs` | Medium | Keep verify, subject binding, and scope order source-obvious and tested. |
| `crates/canic-macros/src/endpoint/expand/access.rs` | Medium | Macro emission remains the endpoint access-before-dispatch choke point. |
| `crates/canic-core/src/ops/auth/token.rs` | Medium | Verifier config, proof cache, and embedded proof verification converge here. |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | Medium | Canonical cert/claims and audience/grant/scope checks are security-sensitive. |
| `crates/canic-core/src/ops/auth/delegation/` | Medium | Root proof provisioning is recently changed and spans active, batch, pending, and policy owners. |
| `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` | Medium | Root install orchestration must keep local preflight before issuer calls. |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | Medium | Replay-first mode must preserve abort-on-denial and commit-after-success. |
| `crates/canic-core/src/workflow/rpc/capability/` | Low | Capability proof mode is currently structural only; any new proof mode should re-run this audit. |

## Recommended Guard Additions

No immediate guard additions are required.

Useful future guards if these surfaces change:

- a PocketIC regression that proves nested issuer-to-root proof retrieval fails
  for the data-certificate/direct-query reason, not merely because of ACL;
- a source-order regression around root batch install proving proof assembly is
  not called from the install update path;
- a capability-envelope regression if any non-structural proof mode is added
  back.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'consume|scope|subject|verify|bind|return Err|return Ok|DelegationAudience::Roles|RolesOrPrincipals|roles: Vec|principals: Vec|token_use|consume_update' crates/canic-core/src/access/auth -g '*.rs'` | PASS | Endpoint auth scan found expected verify/bind/scope code and no verifier-local use-store path. |
| `rg -n 'eval_access|eval_default_app_guard|dispatch_|return Err' crates/canic-macros/src/endpoint -g '*.rs'` | PASS | Macro access evaluation precedes dispatch. |
| `rg -n 'protected_internal|verify_internal_invocation_proof|InternalInvocationProof|request_internal_invocation_proof|caller::has_role|caller::has_any_role' crates/canic-macros/src/endpoint crates/canic/src/macros/endpoints crates/canic-core/src -g '*.rs'` | PASS | No active retired internal proof wrapper or caller-role predicate path found. |
| `rg -n 'verify_delegated_token|verify_claims|verify_audience|verify_scopes|auth_proof_verifier_config|verify_root_canister_signature_proof|verify_issuer_canister_signature_proof|positive_cache|record_verify|DelegationAudience::Roles|RolesOrPrincipals|roles: Vec|principals: Vec' crates/canic-core/src/ops/auth -g '*.rs'` | PASS | Delegated-token verifier, root/issuer proof, and positive-cache paths are current. |
| `rg -n 'upsert_root_issuer_policy|prepare_delegation_proof_batch|get_delegation_proof_batch|install_delegation_proof_batch|install_active_delegation_proof|preflight_delegation_proof_batch|pending_delegation_proof_batch|data_certificate|RootDataCertificateUnavailable' crates/canic-core/src/api/auth crates/canic-core/src/ops/auth/delegation crates/canic-core/src/workflow/runtime/auth crates/canic/src -g '*.rs'` | PASS | Root proof provisioning prepare/get/install and active proof install paths are ordered as expected. |
| `rg -n 'prepare_delegated_token|prepare_delegated_token_issuer_proof|get_delegated_token|get_delegated_token_issuer_proof|active_delegation_proof|prepare_issuer_canister_signature|get_issuer_canister_signature_proof|prepared_by' crates/canic-core/src/api/auth crates/canic-core/src/ops/auth crates/canic-core/src/workflow/runtime/auth -g '*.rs'` | PASS | Issuer-local prepare/get requires active proof and caller/prepared-by binding. |
| `rg -n 'check_replay|reserve|authorize|execute|commit_replay|abort_replay|Cached|Duplicate' crates/canic-core/src/workflow/rpc crates/canic-core/src/ops/replay -g '*.rs'` | PASS | Replay reservation, authorization, abort, execute, and commit paths are explicit. |
| `rg -n 'capability_hash|RootCapabilityEnvelope|NonrootCyclesCapabilityEnvelope|attestation|cache|cached_root_response_attestation|CapabilityProof::' crates/canic-core/src/ops/rpc crates/canic-core/src/workflow/rpc crates/canic-core/src/dto/capability -g '*.rs'` | PASS | Capability proof mode remains structural only; hash binding tests exist. |
| `cargo test --locked -p canic-core access::auth::token --lib -- --nocapture` | PASS | 5 endpoint auth guard tests passed. |
| `cargo test --locked -p canic-core ops::auth::delegation --lib -- --nocapture` | PASS | 26 root proof delegation ops tests passed. |
| `cargo test --locked -p canic-core workflow::runtime::auth::provisioning --lib -- --nocapture` | PASS | 2 root install orchestration tests passed. |
| `cargo test --locked -p canic-macros endpoint::expand --lib -- --nocapture` | PASS | 8 endpoint expansion tests passed. |
| `cargo test --locked -p canic-core ops::auth::delegated::verify --lib -- --nocapture` | PASS | 17 delegated-token verifier tests passed. |
| `cargo test --locked -p canic-core ops::auth::token --lib -- --nocapture` | PASS | 14 verifier config/gate tests passed. |
| `cargo test --locked -p canic-core workflow::rpc::capability --lib -- --nocapture` | PASS | 15 capability hash/envelope tests passed. |
| `cargo test --locked -p canic-core workflow::rpc::request::handler --lib -- --nocapture` | PASS | 32 root replay/request handler tests passed. |
| `cargo test --locked -p canic-core ops::auth::root_canister_sig --lib -- --nocapture` | PASS | 3 root canister-signature seed/domain/message tests passed. |
| `cargo test --locked -p canic --test protocol_surface root_delegation_proof_batch -- --nocapture` | PASS | 2 root delegation proof batch protocol-surface tests passed. |
| `cargo test --locked -p canic-core replay_policy::tests::endpoint::delegation_proof_batch_prepare_is_manifested_as_implemented --lib -- --nocapture` | PASS | Root batch prepare endpoint replay-policy manifest test passed. |
| `cargo test --locked -p canic-core ops::auth::error::tests::root_data_certificate_unavailable_maps_to_public_code --lib -- --nocapture` | PASS | Typed root data-certificate public error mapping passed. |

## Final Verdict

Pass with watchpoints.

The current ordering invariants hold. Residual risk is concentrated in future
drift around endpoint macro emission, delegated-token verifier orchestration,
root proof provisioning, and replay-first root capability handling.
