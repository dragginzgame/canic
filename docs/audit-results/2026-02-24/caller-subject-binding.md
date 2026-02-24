# Audit Result: Caller-Subject Binding

## Run Context

- Date: `2026-02-24`
- Audit definition: `docs/audits/caller-subject-binding.md`
- Branch: `main`
- Commit: `a2bdce13`
- Worktree: `clean`

## Verdict

Pass (with one medium-risk testing caveat).

Core invariant is enforced in access control:

```rust
if verified.claims.sub != caller {
    return Err(...);
}
```

## Checklist Results

### 1. Access Layer Enforcement

- [x] `access/auth.rs` enforces `verified.claims.sub == caller`.
  - Evidence: `crates/canic-core/src/access/auth.rs` (`verify_token`, subject comparison).
- [~] Subject binding runs before scope checks.
  - Evidence: `required_scope` is currently unused (`_required_scope`) in `verify_token`.
  - Note: no endpoint scope check is currently enforced in this path, so ordering against scope checks is not currently applicable.
- [x] Subject binding runs before business logic.
  - Evidence: endpoint macro evaluates access before dispatch:
    - `crates/canic-dsl-macros/src/endpoint/expand.rs` (`access_stage` runs before handler call).
- [x] Subject binding runs before state mutation.
  - Evidence: access denial returns before endpoint implementation executes.
- [x] No alternate auth path skips this check for `authenticated(...)` endpoints.
  - Evidence: `BuiltinPredicate::Authenticated` routes through `delegated_token_verified(...)`.

### 2. No Bearer Fallback Paths

- [x] No envelope-based relay model exists.
- [x] No path accepts token proof but ignores caller identity.
- [x] No legacy auth bypass found in production code.

Searches used:

```text
AuthenticatedRequest
presenter_pid
canic_response_authenticated
relay
```

No matches found in `crates/**.rs`.

### 3. No Ignored Caller Inputs

- [x] `_caller` is not ignored in verification.
- [x] Caller identity is explicitly compared with token subject.
- [x] No unused caller parameter exists in auth verification paths.

Searches used:

```text
fn authenticated(_caller
fn delegated_token_verified(_caller
```

No matches found.

### 4. DSL / Macro Expansion

- [x] Expanded path retains subject-binding call chain.
  - Evidence:
    - `authenticated(...)` DSL maps to `BuiltinPredicate::Authenticated`.
    - Predicate evaluation invokes `access::auth::delegated_token_verified(ctx.caller, ...)`.
    - Access evaluation occurs before endpoint handler dispatch.
- [x] No macro branch found that omits caller-subject comparison for authenticated predicates.

### 5. Test Enforcement

- [x] Mismatch test exists:
  - `crates/canic-core/tests/pic_delegation_provision.rs`:
    - `delegated_token_flow_enforces_subject_binding`
    - asserts rejection for mismatched caller.
- [~] Local run executed, but mismatch assertions were skipped due unavailable threshold key in this environment.
  - Command run:
    - `cargo test -p canic-core --test pic_delegation_provision delegated_token_flow_enforces_subject_binding -- --nocapture`
  - Outcome:
    - test process passed
    - logs indicate skip path on unknown threshold key
- [x] CI includes this test file:
  - workspace test job runs all targets
  - dedicated `delegated-crypto` job runs `pic_delegation_provision` when `CANIC_REQUIRE_THRESHOLD_KEYS=1`.

## Findings

### Medium

1. Crypto-dependent subject-binding integration assertions are conditionally skippable when threshold keys are unavailable.
   - Impact: local/default CI may not always execute full cryptographic path for mismatch-case test.
   - Evidence: skip path in `provision_or_skip(...)` in `pic_delegation_provision.rs`.

### Low

1. `required_scope` parameter in access verification is currently unused.
   - Impact: checklist item "subject binding before scope checks" is presently non-applicable because scope checks are not enforced in `access/auth.rs`.
   - Evidence: `_required_scope` in `verify_token(...)`.

### Critical / High

- None found in this audit run.
