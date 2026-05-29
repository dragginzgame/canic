# Audit Summary - 2026-05-29

## Included Reports

| Report | Status | Summary |
| --- | --- | --- |
| `audience-target-binding.md` | PASS AFTER FIX | Audience binding still holds in verifier paths; artifact role resolution now fails closed to exactly one scoped package metadata role under the selected canister root. |
| `capability-scope-enforcement.md` | PASS WITH TEMPLATE CLEANUP | Capability and scope authorization still runs after authentication and identity binding; the recurring audit template now points at the current endpoint auth hotspot. |
| `dependency-hygiene.md` | PASS | Published crates still avoid unpublished workspace-member dependencies; operator crates remain facade-free, `ic-testkit` stays test-only, and `canic` defaults are narrow again. |
| `expiry-replay-single-use.md` | PASS AFTER FIX | Delegated grants and role/internal attestations now reject at the exact expiry timestamp, aligning them with delegated tokens, replay metadata, consumed-token state, and sessions. |
| `subject-caller-binding.md` | PASS | Delegated-token subject binding still routes through the canonical access-auth verifier; generated endpoint wrappers preserve transport-caller and authenticated-subject lanes. |
| `token-trust-chain.md` | PASS | Delegated-token trust-chain validation still requires verifier-local root trust, root-certified shard authority, canonical cert/claim hashes, shard signatures, and endpoint guard ordering. |

## Findings

- No remaining blockers.
- Focused code remediation was applied for exclusive expiry-boundary alignment
  and scoped canister role manifest resolution.
- Audit-template remediation was applied for the capability-scope recurring
  hotspot table after endpoint auth ownership moved into `access/auth/token.rs`.
- Residual risk remains low and unchanged from the previous comparable run:
  `3 / 10` for subject-caller binding, expiry/replay/single-use,
  capability-scope enforcement, token-trust-chain, and post-remediation
  audience/target binding.
- Dependency hygiene risk remains low at `2 / 10`; the public facade default
  feature pressure improved because `control-plane`, `sharding`, and
  `auth-crypto` are no longer default-on.

## Follow-ups

- Keep `access/auth/token.rs`, `access/expr/mod.rs`, and endpoint macro
  expansion aligned whenever authenticated endpoint syntax or delegated-session
  identity resolution changes.
- Keep `AuthApi::verify_token_material(...)` private unless a future public
  helper performs the full endpoint authorization boundary.
- Keep delegated-token verification, delegated grants, role attestations,
  replay metadata, sessions, and consumed-token markers on the same exclusive
  `now >= expires_at` boundary.
- Keep every configured fleet role backed by exactly one package under the
  selected canister root with matching `[package.metadata.canic] role`.
- Keep delegated-token verifier orchestration, pure chain verification,
  root-key resolution, endpoint access guards, and role-attestation refresh
  behavior aligned whenever delegated issuer formats or subnet-state
  propagation change.
- Keep capability proof DTOs, endpoint auth ordering, and root capability
  workflow authorization/replay changes coordinated across API, ops, workflow,
  metrics, and tests.
- Keep `ic-testkit` restricted to internal test harnesses and test/audit
  canisters, and keep `canic` default features narrow unless a release line
  explicitly chooses broader defaults.
