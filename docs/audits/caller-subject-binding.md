# Audit: Caller-Subject Binding (No Bearer Token Regression)

## Purpose

Ensure authentication tokens cannot be used as transferable bearer tokens.

Security invariant:

> A `DelegatedToken` is valid only when `token.claims.sub == ic_cdk::caller()`.

If this invariant breaks, any principal that obtains a valid token can impersonate another user.

Run this audit after:
- auth refactors
- DSL/macro changes
- access layer changes
- token DTO changes
- root/shard signing changes

## Canonical Invariant

For every authenticated endpoint:

```rust
token.claims.sub == ic_cdk::caller()
```

This must be enforced before authorization logic.

No exceptions.
No relay mode.
No alternate auth path.

## Audit Checklist

### 1. Access Layer Enforcement

- [ ] `access/auth.rs` (or equivalent) enforces `verified.claims.sub == caller`.
- [ ] Subject binding runs before scope checks.
- [ ] Subject binding runs before business logic.
- [ ] Subject binding runs before state mutation.
- [ ] No alternate auth path skips this check.

Search terms:

```text
sub == caller
claims.sub
authenticated(
delegated_token_verified(
```

Confirm there is one canonical enforcement point.

### 2. No Bearer Fallback Paths

Search terms:

```text
AuthenticatedRequest
presenter_pid
relay
envelope
ignore caller
```

Verify:
- [ ] No envelope-based relay model exists.
- [ ] No path accepts token proof but ignores caller identity.
- [ ] No legacy auth bypass exists in production code.

### 3. No Ignored Caller Inputs

Search terms:

```text
fn authenticated(_caller
fn delegated_token_verified(_caller
```

Verify:
- [ ] `_caller` is not ignored in verification.
- [ ] Caller identity is explicitly compared with token subject.
- [ ] No unused caller parameter exists in auth verification paths.

### 4. DSL / Macro Expansion

For macro usage such as `authenticated("scope")`:
- [ ] Inspect expansion (`cargo expand`) if needed.
- [ ] Confirm expanded path retains subject binding.
- [ ] Confirm no macro branch omits caller-subject comparison.

### 5. Test Enforcement

- [ ] At least one test presents a valid token for user A with caller B and expects rejection.
- [ ] That test fails if subject binding is removed.
- [ ] `crates/canic-core/tests/pic_delegation_provision.rs` includes the mismatch case and is part of CI auth-flow coverage.

If missing, add it.

## Red Flags

- Signature verification returns success without a subject check.
- Conditional logic skips subject binding for specific endpoints.
- `presenter_pid` appears without a non-bearer relay proof model.
- Comments imply caller identity is trusted by convention.

## Expected Architecture

- root signs shard certificate
- shard signs user token
- user presents token directly
- verifier enforces root binding, shard binding, expiry, audience, scope, and `sub == caller`

No relay model.
No bearer semantics.

## Severity If Broken

Critical: allows impersonation across principals.

## Audit Frequency

Run this audit:
- before every release
- after auth-layer refactors
- after macro/DSL changes
- after environment/runtime changes that affect caller identity
