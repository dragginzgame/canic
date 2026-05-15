# Audit Summary - 2026-05-14

## Run Contexts

| Report | Type | Scope | Snapshot | Worktree | Status |
| --- | --- | --- | --- | --- | --- |
| `token-trust-chain.md` | Recurring invariant | delegated-token root/shard trust chain, root-key resolution, role-attestation verification, endpoint guard integration | `48213853` | clean before report write | complete |
| `auth-abstraction-equivalence.md` | Recurring invariant | macro-generated authenticated endpoint expansion, access-expression dispatch, delegated-token verifier parity, delegated-session identity resolution, and trust-chain guard integration | `48213853` | dirty before report write | complete |
| `dry-consolidation.md` | Recurring system | maintained Canic source DRY pressure, installed-fleet/registry ownership, response parsing, command-family glue, backup/restore fixtures, scripts | `48213853` | dirty before report write | complete |

## Risk Index Summary

| Report | Risk | Readout |
| --- | ---: | --- |
| `token-trust-chain.md` | 4 / 10 | Invariant holds; residual risk is structural fan-in around auth DTOs and recent edit pressure in runtime verifier/guard code. |
| `auth-abstraction-equivalence.md` | 3 / 10 | Invariant holds; the audit template was tightened around current macro/core paths and the new auth trust-chain guard now mechanically checks public verifier, passive DTO, guard-order, and refresh-scope drift. |
| `dry-consolidation.md` | 4 / 10 | Consolidation risk is down from May 12 after installed-fleet resolution, registry parsing, response parsing primitives, and major CLI command modules gained clearer owners. |

## Method and Comparability Notes

- `token-trust-chain.md` uses `Method V4.3`.
- `auth-abstraction-equivalence.md` uses `Method V4.3`.
- `dry-consolidation.md` uses `DRY Consolidation V3`.
- Same-day baseline is `N/A` because this is the first
  `token-trust-chain` and `auth-abstraction-equivalence` run on 2026-05-14.
- The run is comparable with the 2026-05-07 current-verifier report and not
  directly comparable with older proof-store/current-proof trace reports.
- `auth-abstraction-equivalence.md` is comparable with its 2026-05-09 report;
  the method gained stronger guardrail evidence and current path names.
- `dry-consolidation.md` is partially comparable with its 2026-05-12 report;
  the scope is comparable, but the method is now promoted to a recurring system
  audit definition.

## Key Findings by Severity

### Medium

- `token-trust-chain.md`: no token-trust-chain correctness break was found, but
  `dto::auth` remains the broadest fan-in surface with references across API,
  access, ops, storage, tests, and support canisters.
- `token-trust-chain.md`: `ops/auth/token.rs` and `access/auth/token.rs`
  remain expected sensitive edit centers for runtime verifier and endpoint
  guard behavior.
- `auth-abstraction-equivalence.md`: no abstraction bypass was found, but macro
  expansion, `AccessContext`, and endpoint verifier ordering remain sensitive
  security hotspots.
- `dry-consolidation.md`: no broad DRY regression was found; remaining
  behavior-bearing duplication is narrower around snapshot/status/backup/medic
  installed-state or registry variants and command-family glue.

### Low

- `token-trust-chain.md`: stale proof-store/current-proof trace names are absent
  from active crates, matching the current self-contained delegated-token
  verifier model.
- `auth-abstraction-equivalence.md`: the recurring audit definition still had
  stale `canic-dsl-macros` vocabulary and broad scan commands; the definition
  now uses current `crates/canic-macros` paths and targeted auth scans.
- `dry-consolidation.md`: the repeated ad hoc audit now has a recurring system
  definition with required inventory, installed-fleet, response-parser,
  command-glue, fixture, and output-convention scans.

## Verification Rollup

| Report | PASS | BLOCKED | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `token-trust-chain.md` | 9 | 0 | 0 | Unit verifier/root-key tests and PocketIC root-key cascade plus role-attestation verification paths passed. |
| `auth-abstraction-equivalence.md` | 11 | 0 | 0 | Auth trust-chain guard, macro authenticated/access-stage tests, access-auth/verifier/caller-lane tests, and targeted fan-in/public-verifier scans passed. |
| `dry-consolidation.md` | 11 | 0 | 0 | Source inventory, large-file, installed-fleet/registry, response-parser, script-size, and command-glue scans passed. |

## Follow-up Actions

No follow-up actions required.

Watchpoints only:

1. Keep `dto::auth` passive boundary data only.
2. Keep runtime delegated-token verification centralized in
   `AuthOps::verify_token`.
3. Rerun `audience-target-binding` with `token-trust-chain` after delegated
   audience, role-attestation, or endpoint auth macro changes.
4. Keep macro-generated authenticated endpoints aligned with `AccessContext`,
   `AuthenticatedEvaluator`, and `access/auth/token.rs`.
5. Keep `AuthApi::verify_token_material(...)` private unless a future public
   helper performs full endpoint subject binding and update replay consumption.
6. Keep future shared operator mechanics in `canic-host`, but split host modules
   before they become unrelated helper hubs.
7. Revisit backup/restore fixture support after restore execution stabilizes.
