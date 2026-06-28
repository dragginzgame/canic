# Audit Summary - 2026-06-28

## Run Contexts

| Report | Type | Scope | Status |
| ---- | ---- | ---- | ---- |
| `capability-surface.md` | Recurring capability surface audit | endpoint macro bundles, retained-fleet DID service surface, protocol constants, RPC/capability DTO variants, root renewal/proof and issuer-local surfaces | PASS |
| `capability-scope-enforcement.md` | Recurring capability/scope invariant audit | delegated-token scope enforcement, root capability proof/envelope validation, replay and authorization ordering | PASS |
| `canonical-auth-boundary.md` | Recurring canonical auth boundary invariant audit | macro endpoint auth expansion, delegated-token verifier ordering, role attestation, root proof provisioning | PASS with supplemental validation blocked |

## Risk Index Summary

| Report | Risk | Notes |
| ---- | ----: | ---- |
| `capability-surface.md` | 4 / 10 | Endpoint definitions and protocol export lines grew since 2026-06-19, with retained DID growth concentrated on six root-only delegation-renewal methods; no hard placement violation or over-bundled global family was found. |
| `capability-scope-enforcement.md` | 3 / 10 | No scope-as-identity or authorization-before-authentication break found; endpoint verify/bind/scope ordering and root capability replay/authorization coverage passed, including PocketIC checks outside the sandbox. Follow-up visibility cleanup reduced the counted capability-facing surface from 20 to 18 items. |
| `canonical-auth-boundary.md` | 4 / 10 | Required scans and focused tests passed; broad auth DTO/proof fan-in and recent auth API churn remain watchpoints. Supplemental PocketIC role-attestation parity check blocked on local runner health. |

## Method / Comparability Notes

- `canonical-auth-boundary.md` uses `canonical-auth-boundary/current` and is
  comparable with the 2026-06-19 run. No audit definition change was required
  for this run.
- `capability-scope-enforcement.md` uses
  `capability-scope-enforcement-current` and is comparable with the
  2026-06-19 run. No audit definition change was required for this run.
- `capability-surface.md` uses `capability-surface-current` and is comparable
  with the 2026-06-19 run. Retained DID counts are service-block scoped and the
  generated artifacts were refreshed from `fleets/test/canic.toml`.

## Follow-up

- Rerun the supplemental PocketIC role-attestation verification path after the
  local PocketIC runner is known healthy.
- Keep `verify_token_material(...)` private and keep role-attestation/root
  proof provisioning out of delegated-token endpoint authorization.
- Keep capability DTOs passive, endpoint macros thin, and replay/
  authorization sequencing covered when the root capability surface changes.
- Keep root-managed renewal endpoints root-only, keep blob-storage billing
  role-scoped when retained in future rosters, and watch protocol table fan-out
  before adding more constants.
