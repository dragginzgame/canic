# Auth Abstraction Equivalence Invariant Audit - 2026-05-09

## Report Preamble

- Definition path: `docs/audits/recurring/invariants/auth-abstraction-equivalence.md`
- Scope: macro-generated authenticated endpoint expansion, access-expression runtime dispatch, canonical delegated-token verifier parity, delegated-session identity resolution, raw-caller predicate separation, and replay/scope/expiry behavior
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/auth-abstraction-equivalence.md`
- Code snapshot identifier: `518f57dd`
- Method tag/version: `Method V4.2`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-09T12:39:41Z`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This was selected as the oldest recurring audit that had not already been
refreshed in the May 2026 report set. Its latest report was
`docs/audits/reports/2026-04/2026-04-05/auth-abstraction-equivalence.md`.
Several recurring audits were last run on `2026-04-05`; this one was selected
from that tied set because it is first in the invariant suite order after the
`2026-05-07` refreshes of `audience-target-binding` and `token-trust-chain`.

The run is only partially comparable to the April baseline because the macro
surface has moved from the old `canic-dsl-macros` path to
`crates/canic-macros`, and update-call delegated tokens now have explicit
single-use consumption. The invariant itself remains directly comparable:
all generated and helper auth abstractions must converge on the same canonical
verifier and failure semantics.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Auth abstractions identified | PASS | Current abstractions are `#[canic_query]`, `#[canic_update]`, `requires(auth::authenticated(...))`, the access-expression helpers under `access::expr::auth`, and delegated-session identity resolution under `access::auth::identity`. |
| Macro path routes through canonical evaluator | PASS | `crates/canic-macros/src/endpoint/expand.rs:268-284` generates `resolve_authenticated_identity(...)`, builds `AccessContext`, and calls `access::expr::eval_access(...).await`; no generated authenticated branch calls token verification directly. |
| `requires(...)` remains a single evaluator plan | PASS | `crates/canic-macros/src/endpoint/expand.rs:371-375` still lowers collected requirements to one `AccessExpr::All(...)`, preserving the evaluator's short-circuit and metric semantics. |
| Authenticated predicate routes to canonical token verifier | PASS | `crates/canic-core/src/access/expr/evaluators.rs:384-399` sends `BuiltinPredicate::Authenticated` through `access::auth::delegated_token_verified(...)`; `crates/canic-core/src/access/auth/token.rs:40-68` performs canonical token verification, subject binding, scope enforcement, and update-token consumption. |
| Subject binding cannot be omitted by abstraction | PASS | The generated context carries both `transport_caller` and `authenticated_subject`; `crates/canic-core/src/access/auth/token.rs:61` enforces token subject equality against the resolved authenticated subject before granting access. |
| Raw-caller predicates remain separate from delegated auth | PASS | `AccessContext` keeps `caller` and `authenticated_caller` distinct in `crates/canic-core/src/access/expr/mod.rs:22-31`; caller predicates such as whitelist checks continue to read `ctx.caller` in `crates/canic-core/src/access/expr/evaluators.rs:365-373`. |
| Convenience delegated-session path does not weaken verifier semantics | PASS | `crates/canic-core/src/access/auth/identity.rs:25-48` resolves a delegated session subject only after subject validation, otherwise clears the session and falls back to raw caller. The canonical token verifier still validates the endpoint token against the resolved subject. |
| Macro structure rejects ambiguous authenticated signatures | PASS | `crates/canic-macros/src/endpoint/validate.rs:58-60` requires structural validation when authenticated predicates are present, and `validate_authenticated_args` at `133-160` requires first argument type `DelegatedToken`. |
| Scope syntax preserves explicitness | PASS | `crates/canic-macros/src/endpoint/parse.rs:596-665` covers no-scope, literal-scope, path-scope, multi-argument rejection, and bare-alias rejection. |

## Equivalence Matrix

| Scenario | Expected Behavior | Current Evidence | Result |
| --- | --- | --- | --- |
| Valid credential | Generated/helper path accepts after canonical verifier succeeds | `verify_delegated_token_accepts_self_validating_token_without_proof_lookup` passed | PASS |
| Invalid signature | Generated/helper path rejects through canonical verifier failure | `verify_delegated_token_rejects_root_signature_failure` passed | PASS |
| Mismatched subject/caller | Token is rejected after verification if subject binding fails | `subject_binding_rejects_mismatched_subject_and_caller` passed | PASS |
| Expired credential | Token is rejected at canonical verification boundary | `verify_delegated_token_rejects_expired_token_at_boundary` passed | PASS |
| Missing required scope | Token is rejected by the same scope enforcement path | `required_scope_rejects_when_scope_missing` passed | PASS |
| Update replay | Update token consumption rejects active replay | `update_token_consume_rejects_active_replay` passed | PASS |
| Query replay | Query token verification remains stateless and does not consume replay state | `query_token_consume_is_stateless` passed | PASS |
| Delegated-session invalid subject | Delegated-session convenience path rejects obvious infrastructure/canister subjects | `validate_delegated_session_subject_*` tests passed | PASS |

## Code Path Walkthrough

The current generated authenticated endpoint path is:

1. `#[canic_update]` / `#[canic_query]` enters `crates/canic-macros/src/lib.rs`
   and calls `endpoint::expand_entry(...)`.
2. `expand.rs` validates that gated endpoints are fallible and builds an
   `AccessPlan`.
3. `AccessPlan::Expr` generation fetches `msg_caller`, resolves delegated
   session identity through `resolve_authenticated_identity(...)`, builds an
   `AccessContext`, and evaluates `eval_access(...)`.
4. `eval_access(...)` dispatches `BuiltinPredicate::Authenticated` to
   `AuthenticatedEvaluator`.
5. `AuthenticatedEvaluator` calls `access::auth::delegated_token_verified(...)`
   with the resolved authenticated subject, the required scope, and call kind.
6. `access/auth/token.rs` decodes the delegated token from ingress arg zero,
   invokes `AuthOps::verify_token(...)`, enforces subject binding, enforces the
   required endpoint scope, and consumes update tokens once.

No helper or macro-only bypass was found in this path. The only abstraction
choice before verification is whether the authenticated subject is the raw
transport caller or an active delegated-session subject. That decision is
centralized in `access/auth/identity.rs` and keeps raw-caller predicates on the
transport caller lane.

## Comparison to Previous Relevant Run

- Stable: generated authenticated endpoints still converge on the canonical
  `access::expr` evaluator and `access::auth` verifier path.
- Changed: the macro implementation path is now `crates/canic-macros` instead
  of the April report's `crates/canic-dsl-macros` path.
- Improved: update-call delegated tokens now have explicit single-use
  consumption in the canonical verifier path, with query calls intentionally
  stateless.
- Stable: missing-scope, subject-binding, expiry, and invalid-signature failures
  still occur below the abstraction layer, not in helper-specific branches.
- Stable: delegated sessions remain a subject-resolution convenience and do not
  replace transport-caller semantics for caller/topology predicates.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-macros/src/endpoint/expand.rs` | `access_stage`, `build_access_plan` | Generates endpoint access wrappers and decides how `requires(...)` is lowered | High |
| `crates/canic-macros/src/endpoint/validate.rs` | `validate_authenticated_args` | Compile-time guard that keeps authenticated endpoints structurally aligned with ingress token decoding | Medium |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext`, `AccessExpr`, `eval_access` | Shared dispatch surface for generated and handwritten access expressions | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | Boundary between access expression evaluation and canonical auth verification | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | Canonical verifier ordering: decode, verify, bind subject, enforce scope, consume update token | High |
| `crates/canic-core/src/access/auth/identity.rs` | `resolve_authenticated_identity_at` | Delegated-session convenience lane; must preserve raw transport caller separately | Medium |
| `crates/canic-core/src/api/auth/session/mod.rs` | `set_delegated_session_subject` | Session bootstrap verifies token and subject before storing the convenience mapping | Medium |

## Hub Module Pressure

| Module | Fan-In Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/expr/mod.rs` | 5 direct files mention access expression/evaluator symbols in current `rg` scan | 2 | 1 | 6 |
| `crates/canic-core/src/access/auth/*` | 7 direct files mention `access::auth`, identity resolution, or delegated verifier symbols | 4 | 2 | 6 |
| `crates/canic-macros/src/endpoint/expand.rs` | Macro codegen is the only generated endpoint auth wrapper path | 2 | 1 | 5 |
| `crates/canic-core/src/api/auth/session/mod.rs` | Session bootstrap crosses API, ops, storage, and metrics | 4 | 2 | 5 |
| `crates/canic-core/src/dto/auth.rs` token shapes | `DelegationProof` appears in 11 files; delegated-token verifier/claims symbols appear in 10 files | 5 | 2 | 5 |

The apparent fan-in is lower than the April report because the current scan
uses direct symbol/path references instead of broad module substring matches.
The pressure remains real at the same places: generated endpoint auth wiring,
`AccessContext`, the authenticated evaluator, and canonical delegated-token
verification.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| macro/auth drift | `crates/canic-macros/src/endpoint/expand.rs` | The file owns wrapper generation, default app guard injection, access expression synthesis, and metrics-sensitive denial behavior | Medium |
| subject-lane confusion | `crates/canic-core/src/access/expr/mod.rs` | `AccessContext` intentionally carries both raw transport caller and authenticated subject; future predicates must choose the correct lane | Medium |
| verifier ordering drift | `crates/canic-core/src/access/auth/token.rs` | Security behavior depends on preserving verify -> subject binding -> scope -> update consumption order | Medium |
| session convenience pressure | `crates/canic-core/src/api/auth/session/mod.rs` and `access/auth/identity.rs` | Delegated-session bootstrap and resolution are convenience paths adjacent to canonical token verification | Low |
| DTO spread | `crates/canic-core/src/dto/auth.rs` | Delegation proof and token claim shapes are referenced by access, API, ops, macros, and tests | Low |

## Dependency Fan-In Pressure

### Module Fan-In

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `access::expr` / `eval_access` / `AccessExpr` / `BuiltinPredicate` | 5 | `access`, `macros` | Medium |
| `access::auth` / `delegated_token_verified` / identity resolution | 7 | `access`, `api`, `core`, `macros` | Medium |

### Struct / DTO Fan-In

| Struct / Symbol Group | Defined In | Direct Files | Risk |
| --- | --- | ---: | --- |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 11 | Medium |
| `DelegatedTokenClaims` / `VerifiedDelegatedToken` / `VerifyDelegatedToken*` | `crates/canic-core/src/dto/auth.rs`, `ops/auth` | 10 | Medium |

## Risk Score

Risk Score: **3 / 10**

Score contributions:

- `+1` generated endpoint auth remains concentrated in one macro expansion
  module.
- `+1` `AccessContext` deliberately carries two caller identities; future
  predicates can regress if they choose the wrong lane.
- `+1` delegated-token behavior depends on verifier ordering and shared DTO
  shapes across access/API/ops/tests.

Verdict: **Invariant holds with low residual coupling risk.**

No remediation is required from this audit. The useful action is to keep the
watchpoints explicit when changing macro auth wiring, delegated-session
semantics, or delegated-token verification order.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-macros authenticated -- --nocapture` | PASS | 8 tests passed for authenticated parse/validate behavior. |
| `cargo test -p canic-core --lib required_scope_rejects_when_scope_missing -- --nocapture` | PASS | Missing scope rejects in canonical auth path. |
| `cargo test -p canic-core --lib subject_binding_rejects_mismatched_subject_and_caller -- --nocapture` | PASS | Token subject mismatch rejects. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_expired_token_at_boundary -- --nocapture` | PASS | Expired delegated token rejects at verifier boundary. |
| `cargo test -p canic-core --lib update_token_consume_rejects_active_replay -- --nocapture` | PASS | Update replay consumption path rejects reuse. |
| `cargo test -p canic-core --lib verify_delegated_token_accepts_self_validating_token_without_proof_lookup -- --nocapture` | PASS | Valid self-validating token succeeds without local proof-cache lookup. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_root_signature_failure -- --nocapture` | PASS | Invalid root signature rejects through canonical verifier. |
| `cargo test -p canic-core --lib query_token_consume_is_stateless -- --nocapture` | PASS | Query verification remains stateless. |
| `cargo test -p canic-core --lib validate_delegated_session_subject -- --nocapture` | PASS | 2 subject validation tests passed for delegated-session rejection. |
| `rg -l 'access::expr\|eval_access\|AccessExpr\|AccessPredicate\|BuiltinPredicate' crates canisters fleets -g '*.rs'` | PASS | Fan-in scan recorded 5 direct files. |
| `rg -l 'access::auth\|delegated_token_verified\|resolve_authenticated_identity\|AuthenticatedIdentitySource\|ResolvedAuthenticatedIdentity' crates canisters fleets -g '*.rs'` | PASS | Fan-in scan recorded 7 direct files. |
| `rg -l 'DelegationProof' crates canisters fleets -g '*.rs'` | PASS | DTO spread scan recorded 11 direct files. |
| `rg -l 'DelegatedTokenClaims\|VerifiedDelegatedToken\|VerifyDelegatedToken' crates canisters fleets -g '*.rs'` | PASS | Verifier/claims spread scan recorded 10 direct files. |

## Follow-up Actions

1. Keep `crates/canic-macros/src/endpoint/expand.rs`,
   `crates/canic-core/src/access/expr/mod.rs`, and
   `crates/canic-core/src/access/auth/token.rs` aligned whenever
   authenticated endpoint syntax, delegated sessions, or delegated-token replay
   semantics change.
2. Re-run this audit after any change that adds a new authenticated predicate,
   changes `AccessContext`, or moves delegated-token verification out of
   `access/auth/token.rs`.
