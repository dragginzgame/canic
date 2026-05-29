# Audit Summary - 2026-05-29

## Included Reports

| Report | Status | Summary |
| --- | --- | --- |
| `audience-target-binding.md` | PASS AFTER FIX | Audience binding still holds in verifier paths; artifact role resolution now fails closed to exactly one scoped package metadata role under the selected canister root. |
| `expiry-replay-single-use.md` | PASS AFTER FIX | Delegated grants and role/internal attestations now reject at the exact expiry timestamp, aligning them with delegated tokens, replay metadata, consumed-token state, and sessions. |
| `subject-caller-binding.md` | PASS | Delegated-token subject binding still routes through the canonical access-auth verifier; generated endpoint wrappers preserve transport-caller and authenticated-subject lanes. |

## Findings

- No remaining blockers.
- Focused code remediation was applied for exclusive expiry-boundary alignment
  and scoped canister role manifest resolution.
- Residual risk remains low and unchanged from the previous comparable run:
  `3 / 10` for subject-caller binding, expiry/replay/single-use, and
  post-remediation audience/target binding.

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
