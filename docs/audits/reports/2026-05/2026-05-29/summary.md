# Audit Summary - 2026-05-29

## Included Reports

| Report | Status | Summary |
| --- | --- | --- |
| `expiry-replay-single-use.md` | PASS AFTER FIX | Delegated grants and role/internal attestations now reject at the exact expiry timestamp, aligning them with delegated tokens, replay metadata, consumed-token state, and sessions. |
| `subject-caller-binding.md` | PASS | Delegated-token subject binding still routes through the canonical access-auth verifier; generated endpoint wrappers preserve transport-caller and authenticated-subject lanes. |

## Findings

- No remaining blockers.
- One focused code remediation was applied for exclusive expiry-boundary
  alignment.
- Residual risk remains low and unchanged from the previous comparable run:
  `3 / 10` for subject-caller binding and post-remediation `3 / 10` for
  expiry/replay/single-use.

## Follow-ups

- Keep `access/auth/token.rs`, `access/expr/mod.rs`, and endpoint macro
  expansion aligned whenever authenticated endpoint syntax or delegated-session
  identity resolution changes.
- Keep `AuthApi::verify_token_material(...)` private unless a future public
  helper performs the full endpoint authorization boundary.
- Keep delegated-token verification, delegated grants, role attestations,
  replay metadata, sessions, and consumed-token markers on the same exclusive
  `now >= expires_at` boundary.
