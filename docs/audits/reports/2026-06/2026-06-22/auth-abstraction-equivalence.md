# Auth Abstraction Equivalence Invariant Audit - 2026-06-22

## Report Preamble

- Scope: macro-generated authenticated endpoint expansion, access-expression
  dispatch, delegated-token verifier parity, delegated-session identity
  resolution, transport-caller lane separation, canister/subnet/project
  audience binding, role grants, root proof provisioning endpoint bundles, and
  blob-storage endpoint guard bundles
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/auth-abstraction-equivalence.md`
- Code snapshot identifier: `5bc5a458` with dirty worktree
- Method tag/version: `auth-abstraction-equivalence-v2`
- Comparability status: partially comparable. The verifier-equivalence
  invariant is unchanged, while the method now explicitly covers root/nonroot
  auth endpoint bundles and blob-storage endpoint guard bundles.
- Auditor: `codex`

## Audit Definition Update

Before running the audit, `docs/audits/recurring/invariants/auth-abstraction-equivalence.md`
was reviewed and refreshed.

Method changes:

- added explicit method tag `auth-abstraction-equivalence-v2`;
- added root proof provisioning endpoint bundles to the current auth surface and
  hotspot coverage;
- added blob-storage and blob-storage billing endpoint bundles to the current
  auth surface, scan terms, and hotspot coverage;
- clarified that root proof provisioning must remain
  controller/registered-caller scoped and must not become a product
  delegated-token auth shortcut;
- clarified that gateway protocol checks must not substitute for product
  delegated-token authentication;
- tightened risk scoring so passive DTO/protocol fan-in is recorded as pressure
  but only increases risk when behavior or public helper ownership spreads with
  the data shape.

## Executive Summary

Verdict: **Pass with watchpoints**.

Risk score: **3 / 10**.

Generated and helper auth abstractions still preserve the canonical verifier
path:

`macro expansion -> AccessContext -> eval_access -> AuthenticatedEvaluator -> access::auth::delegated_token_verified -> AuthOps::verify_token`.

No generated/helper bypass, subject-lane collapse, public material-only
verifier, old role/principal token audience model, root-proof auth shortcut, or
gateway-protocol auth shortcut was found.

No production code changes were made for this audit. Only the recurring audit
definition, report, summary, and handoff were updated.

## Checklist Results

| Check | Result | Evidence |
| --- | --- | --- |
| Auth abstractions identified | PASS | Current abstractions are `#[canic_query]`, `#[canic_update]`, `requires(auth::authenticated(...))`, access-expression helpers, delegated-session identity resolution, root/nonroot auth endpoint bundles, and blob-storage endpoint bundles. |
| Macro generated path preserves identity lanes | PASS | `access_stage_default_guard_marks_identity_source_raw_caller` and `access_stage_expr_builds_context_from_resolved_identity` passed. |
| Authenticated endpoint shape is compile-time guarded | PASS | Macro authenticated tests still require first argument type `DelegatedToken`. |
| Authenticated predicate routes to canonical verifier | PASS | `AuthenticatedEvaluator` calls `access::auth::delegated_token_verified(...)`; access-auth tests passed. |
| Current delegated-token audience shape | PASS | Active code uses `DelegationAudience::{Canister, CanicSubnet, Project}` and `DelegatedRoleGrant`. |
| Removed plural/mixed audience shapes absent from active code | PASS | Active-code scans found no `Roles`, `Principals`, `RolesOrPrincipals`, role/principal audience variants, or `verifier_role_hash`; matches are historical docs/audit text only. |
| Public material-only verifier remains blocked | PASS | No public `AuthApi::verify_token` or public `verify_token_material(...)`; `verify_token_material` remains private, and the public verifier surface is `AuthOps::verify_token`. |
| Delegated-session convenience path stays narrower than endpoint auth | PASS | Session bootstrap still uses private material verification; endpoint auth still performs subject/caller binding and required-scope checks in access. |
| Root proof endpoint bundles preserve authority model | PASS | Root proof batch prepare/get/install are controller guarded; role-attestation prepare/get are registered-subnet internal endpoints; active proof install is controller guarded on nonroot issuers. |
| Blob-storage endpoint bundles preserve guard model | PASS | Certificate creation and billing sync/funding/status use explicit access expressions; protocol gateway liveness/deletion checks remain separate from product frontend auth. |
| Integration path exercises generated authenticated endpoint | PASS | PocketIC sharding suite passed issuer-local delegated-token verification after active root proof install. |

## Equivalence Matrix

| Scenario | Expected Behavior | Current Evidence | Result |
| --- | --- | --- | --- |
| Valid delegated token | Generated endpoint accepts after canonical verifier succeeds | PocketIC sharding delegated-token path passed after active proof installation. | PASS |
| Invalid proof/signature | Canonical verifier rejects before endpoint success | `verify_delegated_token_rejects_root_proof_failure` and issuer-proof tests passed. | PASS |
| Mismatched subject/caller | Access rejects after material verification | `subject_binding_rejects_mismatched_subject_and_caller` passed. | PASS |
| Expired token | Verifier rejects at expiry boundary | `verify_delegated_token_rejects_expired_token_at_boundary` passed. | PASS |
| Missing scope | Access/verifier rejects required-scope mismatch | `required_scope_rejects_when_scope_missing` and required-scope verifier tests passed. | PASS |
| Delegated-session subject resolution | Session lane remains separate from transport caller lane | Delegated-session fallback tests and caller-lane predicate test passed. | PASS |
| Canister/subnet/project audience | Accepted only when local verifier context matches | `delegated::audience` tests passed. | PASS |
| Token grants vs cert grants | Token cannot expand grants/scopes beyond cert | Grant expansion and per-role scope tests passed. | PASS |
| Root proof provisioning endpoint bundle | Generated endpoints keep controller/registered-caller authority | Static scan of `root.rs` and `nonroot.rs`, plus protocol surface tests, passed. | PASS |
| Blob-storage endpoint bundle | Generated guards stay explicit and protocol checks stay separate | Static scan plus blob endpoint/protocol surface tests passed. | PASS |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk |
| --- | --- | --- | --- |
| `crates/canic-macros/src/endpoint/expand/access.rs` | `access_stage`, `build_access_plan` | Auth wrapper generation and identity-lane setup | Medium |
| `crates/canic-macros/src/endpoint/validate/mod.rs` | authenticated argument validation | Compile-time token-bearing endpoint shape guard | Medium |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext`, `eval_access` | Shared generated/handwritten access dispatch | Medium |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | Access evaluator to canonical auth verifier edge | Medium |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | Endpoint verifier ordering and subject/scope binding | High |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material` | Private partial verifier for session bootstrap only | Medium |
| `crates/canic-core/src/ops/auth/delegated/audience.rs` | audience/grant helpers | Canister/subnet/project and role-grant binding | Medium |
| `crates/canic-core/src/ops/auth/delegation/*` | root proof provisioning helpers | Active proof install and batch provisioning are high-consequence auth support paths | Medium |
| `crates/canic-core/src/dto/auth.rs` | auth DTOs | Passive wire shapes with broad protocol fan-in | Medium |
| `crates/canic/src/macros/endpoints/root.rs` | root auth endpoint emitters | Controller/registered-caller root proof authority surface | Medium |
| `crates/canic/src/macros/endpoints/nonroot.rs` | nonroot auth endpoint emitters | Issuer-local prepare/get and active proof install surface | Medium |
| `crates/canic/src/macros/endpoints/blob_storage.rs` | blob-storage endpoint emitter | Explicit product guard plus protocol gateway checks | Medium |
| `crates/canic/src/macros/endpoints/blob_storage_billing.rs` | billing endpoint emitter | Explicit product guards for billing sync/funding/status | Medium |

## Hub Module Pressure

| Module / Symbol Group | Evidence | Pressure Score |
| --- | --- | ---: |
| `access::expr` and endpoint macro access expansion | 8 files mention access expression/evaluator symbols, concentrated in access and macro crates. | 5 / 10 |
| `access::auth` identity/verifier path | 9 files mention access auth, delegated verifier, or identity lane symbols. | 5 / 10 |
| `DelegationProof` | 31 files across runtime, storage, workflow, tests, protocol, and provisioning paths. | 6 / 10 |
| Delegated-token verifier symbols | 14 files, mostly verifier/protocol/test fan-in. | 5 / 10 |
| Root/nonroot/blob endpoint emitters | 9 files cover the emitter macros and their compile/protocol tests. | 4 / 10 |

## Early Warning Signals

- `DelegationProof` fan-in is high but mostly passive protocol/test/storage
  usage; keep behavior in ops/workflow/access rather than DTOs.
- `verify_token_material(...)` remains private and intentionally incomplete for
  endpoint auth. Keep it private unless a future public helper also performs
  endpoint subject binding and replay-sensitive mutation checks.
- Keep root proof provisioning endpoint bundles controller/registered-caller
  scoped; they must not become delegated-token frontend login endpoints.
- Keep blob-storage gateway protocol checks separate from product
  delegated-token authentication.

## Dependency Fan-In Pressure

| Symbol Group | Direct Files | Pressure |
| --- | ---: | --- |
| `access::expr` / `eval_access` / `AccessExpr` / `BuiltinPredicate` | 8 | Medium |
| `access::auth` / `delegated_token_verified` / identity resolution | 9 | Medium |
| `DelegationProof` | 31 | High but expected passive protocol fan-in |
| `DelegatedTokenClaims` / `VerifiedDelegatedToken` / `VerifyDelegatedToken*` | 14 | Medium-high, mostly verifier/protocol/test fan-in |
| generated root/nonroot/blob endpoint emitters | 9 | Medium |

## Risk Score

Risk Score: **3 / 10**.

Derivation:

- `+2` for security-sensitive hotspots in macro expansion, access auth, private
  material verifier helpers, and generated root/blob endpoint bundles;
- `+1` for high but expected passive auth DTO/proof fan-in;
- `0` for confirmed parity breaks; none found.

## Verification Readout

| Check | Result | Notes |
| --- | --- | --- |
| Auth abstraction symbol scan | PASS | Found expected macro/access/authenticated surfaces. |
| Access auth/identity symbol scan | PASS | Found expected access and session helper surfaces. |
| `DelegationProof` fan-in scan | PASS with watchpoints | 31 direct files; passive protocol fan-in remains expected. |
| Delegated-token verifier fan-in scan | PASS with watchpoints | 14 files, mostly verifier/protocol/test usage. |
| Generated endpoint bundle scan | PASS | Root/nonroot/blob endpoint emitters and compile/protocol tests found. |
| Removed plural/mixed audience scan | PASS | Active code clean; matches are historical docs/audit text. |
| Role-attestation vs token-audience scan | PASS | No active `DelegationAudience::Role`, `DelegationAudience::Principal`, or `verifier_role_hash` in Rust code. |
| Canister/subnet/project audience scan | PASS | Current audience/grant model is active in DTO, ops, workflow, storage mapping, tests, and docs. |
| Public material-only verifier scan | PASS | No public `AuthApi::verify_token` or public `verify_token_material(...)`. |
| Root/blob endpoint guard scan | PASS | Root proof endpoints are controller/registered-caller guarded; blob endpoint guards are explicit where product authorization is needed. |
| `cargo test --locked -p canic-macros authenticated -- --nocapture` | PASS | 9 tests. |
| `cargo test --locked -p canic-macros access_stage_ -- --nocapture` | PASS | 2 tests. |
| `cargo test --locked -p canic-core --lib access::auth -- --nocapture` | PASS | 18 tests. |
| `cargo test --locked -p canic-core --lib verify_delegated_token -- --nocapture` | PASS | 17 tests. |
| `cargo test --locked -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture` | PASS | 1 test. |
| `cargo test --locked -p canic-core --lib delegated::audience -- --nocapture` | PASS | 4 tests. |
| `cargo test --locked -p canic --test endpoint_macro -- --nocapture` | PASS | 3 tests. |
| `cargo test --locked -p canic --test blob_storage_endpoint_macro -- --nocapture` | PASS | Compile-only test target passed with 0 runtime tests. |
| `cargo test --locked -p canic --test protocol_surface -- --nocapture` | PASS | 17 tests. |
| `POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test root_suite sharding -- --nocapture` | PASS after unsandboxed retry | Sandboxed run failed because PocketIC could not bind `127.0.0.1:0`; approved unsandboxed retry passed 4 tests. |

## Follow-up Actions

No required remediation.

Watchpoints:

- Keep `DelegationProof` and auth DTO fan-in passive; do not move verifier,
  signing, replay, or endpoint auth behavior into DTOs.
- Keep generated root proof provisioning endpoints controller/registered-caller
  scoped.
- Keep blob-storage gateway protocol checks separate from delegated-token
  product auth.

## Final Verdict

Pass with watchpoints.

Generated and helper auth abstractions remain equivalent to the canonical
verifier path, and the newer root/nonroot/blob endpoint bundles preserve their
intended authority boundaries.
