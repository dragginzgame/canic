# Caller-Subject Binding Audit — 2026-03-08

## Run Context

- Audit run: `caller-subject-binding`
- Definition: `docs/audits/recurring/caller-subject-binding.md`
- Auditor: `codex`
- Date (UTC): `2026-03-08 17:08:39Z`
- Branch: `eleven`
- Commit: `c98bb574`
- Worktree: `dirty`
- Scope: delegated-token auth boundary (`access`, `api/auth`, auth ops)

## Canonical Invariant

`DelegatedToken` is only accepted for authenticated access when:

`verified.claims.sub == ic_cdk::caller()`

## Checklist

### 1. Access Layer Enforcement

- [x] Canonical enforcement point exists in `access/auth.rs`.
- [x] Subject binding is enforced before required-scope checks.
- [x] Subject binding occurs before access success is returned.
- [x] No alternate access predicate path bypasses this check.

Evidence:
- `access/auth.rs:37-77` (`delegated_token_verified` -> `verify_token`)
- `access/auth.rs:73` `enforce_subject_binding(verified.claims.sub, caller)?`
- `access/auth.rs:79-87` explicit mismatch denial

### 2. No Bearer Fallback Paths

- [x] No `AuthenticatedRequest` relay type found.
- [x] No `presenter_pid` or relay-mode acceptance path found.
- [x] No `CANIC_DEV_AUTH` bypass found.

Search terms run:
- `AuthenticatedRequest`, `presenter_pid`, `relay`, `canic_response_authenticated`, `CANIC_DEV_AUTH`

### 3. No Ignored Caller Inputs

- [x] No `_caller`-ignored delegated auth verification signatures detected.
- [x] Caller is threaded from access context into delegated-token verification.

Evidence:
- `access/expr.rs:315` calls `delegated_token_verified(ctx.caller, ...)`
- `access/auth.rs:37-57` passes `caller` into `verify_token`
- `access/auth.rs:73` compares verified subject with caller

### 4. DSL / Macro Expansion Path

- [x] Authenticated predicate path routes through access evaluator and canonical subject-binding check.
- [~] `cargo expand` was not required in this pass because call path is direct and explicit in source.

### 5. Test Enforcement

- [x] Unit coverage exists for subject/caller mismatch rejection.
- [x] Role-attestation integration coverage includes mismatched caller rejection (`crates/canic-core/tests/pic_role_attestation.rs`).
- [x] Dedicated PocketIC delegated-token mismatch coverage exists in endpoint auth wiring.

Evidence:
- `access/auth.rs:289-294` (`subject_binding_rejects_mismatched_subject_and_caller`)
- `crates/canic-core/tests/pic_role_attestation.rs:62-90` (mismatched caller fails)
- `crates/canic/tests/delegation_flow.rs` (`authenticated_rpc_flow`): valid minted token succeeds for subject caller and is rejected for mismatched ingress caller with caller-subject binding error.

## Findings

1. **Pass**: Canonical delegated-token subject binding is enforced at access boundary and ordered correctly before scope authorization.
2. **Pass**: No legacy bearer/relay fallback path detected.
3. **Pass**: End-to-end PocketIC delegated-token mismatch test now covers `token.sub = A`, `caller = B` rejection in authenticated endpoint flow.

## Verdict

- Bearer-regression status: **No regression found**.
- Security invariant status: **Enforced**.
- Follow-up required: **None for caller-subject binding coverage**.
