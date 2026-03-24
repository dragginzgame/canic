# Complexity Accretion Audit - 2026-03-24

## Report Preamble

- Scope: `crates/canic-core/src/api/auth/mod.rs`, `crates/canic-core/src/workflow/auth.rs`, `crates/canic-core/src/ops/runtime/metrics/{auth,mapper}.rs`, `crates/canic-core/tests/pic_role_attestation.rs`
- Compared baseline report path: `N/A` (first run for this scope on 2026-03-24)
- Code snapshot identifier: `76eba1a1`
- Method tag/version: `Method V4.1`
- Comparability status: `non-comparable` (targeted 0.16 auth-slice audit; prior complexity runs were broader crate-wide baselines)
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-24T15:58:55Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Auth control hub size captured | PASS | `api/auth/mod.rs` = `1102 LOC`, `55 if`, `13 match` |
| Workflow/admin orchestration size captured | PASS | `workflow/auth.rs` = `347 LOC`, `11 match` |
| Metrics accretion surface captured | PASS | `ops/runtime/metrics/auth.rs` = `935 LOC`; `mapper.rs` = `442 LOC` |
| Test complexity tracked separately | PASS | `pic_role_attestation.rs` = `2759 LOC`, helper-heavy integration harness |
| Hub import pressure recorded | PASS | required import-pressure block included below; daily baseline delta = `N/A` |

## Findings

### Medium

1. `DelegationApi` has crossed into a multi-responsibility control hub, increasing path-divergence risk across auth ingress, issuance, proof install, admin push, and delegated-session bootstrap.
   - Evidence:
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L222)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L350)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L484)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L629)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L850)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L956)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L978)
   - Why this matters: audience and proof invariants are currently coherent, but they now require edits across too many local helper seams inside one façade.

2. Auth rollout metrics are string-coupled across emission, reclassification, and query layers, which raises silent dashboard/regression risk whenever predicates are renamed or added.
   - Evidence:
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L339)
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L379)
     - [crates/canic-core/src/ops/runtime/metrics/mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/mapper.rs#L185)
     - [crates/canic-core/src/ops/runtime/metrics/mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/mapper.rs#L223)
     - [crates/canic-core/src/ops/runtime/metrics/mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/mapper.rs#L254)
     - [crates/canic-core/src/workflow/metrics/query.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/metrics/query.rs#L145)
     - [crates/canic-core/src/workflow/metrics/query.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/metrics/query.rs#L256)
   - Why this matters: the rollout page is derived behavior, but it has no typed join between producer and classifier.

### Low

3. Install-path metrics currently report through the signer auth endpoint label even for verifier-local proof-store failures, which weakens operator locality when reading auth pages.
   - Evidence:
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L6)
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L339)
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L379)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L484)
     - [crates/canic-core/tests/pic_role_attestation.rs](/home/adam/projects/canic/crates/canic-core/tests/pic_role_attestation.rs#L732)
   - Why this matters: not a correctness break, but it obscures which canister actually observed the failure.

## Structural Hotspots

| File / Module | Role | LOC | Branching Signals | Risk |
| --- | --- | ---: | --- | --- |
| `crates/canic-core/src/api/auth/mod.rs` | auth façade / control hub | 1102 | `55 if`, `13 match` | High |
| `crates/canic-core/src/ops/runtime/metrics/auth.rs` | auth metric emission hub | 935 | `2 if`, `11 match` | Medium |
| `crates/canic-core/src/ops/runtime/metrics/mapper.rs` | derived rollout classifier | 442 | `2 if`, `3 match` | Medium |
| `crates/canic-core/src/workflow/auth.rs` | root orchestration / fanout | 347 | `11 match` | Medium |

## Hub Import Pressure

Daily baseline delta policy:
- First run of day for `complexity-accretion`, so all deltas are `N/A`.

| Module | Top Imports / Subsystems | Unique Sibling Subsystems | Cross-Layer Dependency Count | Delta vs Daily Baseline |
| --- | --- | ---: | ---: | --- |
| `crates/canic-core/src/api/auth/mod.rs` | `access, config, dto, log, ops, protocol, storage, workflow, cdk` | 9 | 4 | `N/A` |
| `crates/canic-core/src/workflow/auth.rs` | `dto, log, ops, protocol, cdk` | 5 | 1 | `N/A` |
| `crates/canic-core/src/ops/runtime/metrics/auth.rs` | `access, config, dto, ids, ops` | 5 | 2 | `N/A` |
| `crates/canic-core/src/ops/runtime/metrics/mapper.rs` | `access, dto, ids, ops, cdk` | 5 | 2 | `N/A` |

## Primary Architectural Pressure

`crates/canic-core/src/api/auth/mod.rs`

Reasons:
- highest control-surface density in the audited slice
- mixes auth ingress, delegated session state, proof install, issuance fallback, and admin push preparation
- imports across `access`, `ops`, `storage`, and `workflow`, which makes invariant edits expensive

## Risk Score

Risk Score: **6 / 10**

Score contributions:
- `+3` high control-hub concentration in `api/auth/mod.rs`
- `+2` string-coupled rollout metric derivation path
- `+1` endpoint-locality ambiguity in install metrics
- `+0` test hotspot contribution (tracked, not scored directly)

Moderate structural risk. No audience-binding correctness break was found in this pass, but the auth slice is now complex enough that future changes should bias toward decomposition before adding more behavior.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n "DelegationAdminCommand|prepare_explicit_verifier_push|ensure_target_in_proof_audience|ensure_token_claim_audience_subset|record_delegation_install_|AuthRollout"` | PASS | located accreted auth/admin/metrics seams for review |
| `cargo test -p canic-core --lib api::auth::tests -- --nocapture` | PASS | `38 passed; 0 failed` |
| `cargo clippy -p canic-core --lib -- -D warnings` | PASS | no lint regressions in audited slice |
| `wc -l crates/canic-core/src/api/auth/mod.rs ...` | PASS | hotspot LOC baselines captured for report tables |

## Follow-up Actions

1. Owner boundary: `api/auth`
   Action: split [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs) into concern-scoped modules (`admin`, `session`, `issuance`, `proof_store`) while keeping endpoint surface stable.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/complexity-accretion.md`

2. Owner boundary: `ops/runtime/metrics`
   Action: replace string-matching rollout classification with typed signal mapping or shared predicate constants to remove producer/classifier drift risk.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/complexity-accretion.md`

3. Owner boundary: `ops/runtime/metrics` + `api/auth`
   Action: decide whether install-path metrics should retain the logical `auth_signer` endpoint label or emit verifier-local labels for target-side failures; document the choice either way.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/complexity-accretion.md`
