# Complexity Accretion Audit - 2026-03-24 (Rerun 2)

## Report Preamble

- Scope: `crates/canic-core/src/api/auth/mod.rs`, `crates/canic-core/src/workflow/auth.rs`, `crates/canic-core/src/ops/runtime/metrics/{auth,mapper}.rs`, `crates/canic-core/tests/pic_role_attestation.rs`
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-24/complexity-accretion.md`
- Code snapshot identifier: `76eba1a1`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-24T16:02:42Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Auth control hub still concentrated | PASS | `api/auth/mod.rs` remains `1102 LOC`, `55 if`, `13 match`, `9` imported subsystems |
| Responsibility overlap evaluated by invariant boundary | PASS | ingress validation, audience invariants, admin fanout prep, proof install, issuance fallback, session logic are still co-located |
| Rollout metrics coupling risk re-evaluated | PASS | producer/classifier/query still joined only by predicate strings |
| Required hub import pressure retained | PASS | baseline import-pressure block remains valid; no code delta since baseline |
| Silent observability failure risk assessed separately | PASS | metrics coupling classified as higher latent risk than module size alone |

## Comparison to Baseline

- Stable: no new correctness break was found in audience-binding paths.
- Refined: the primary structural risk is not raw LOC, but invariant synchronization across too many auth responsibilities inside one façade.
- Refined: rollout metrics coupling is the highest latent failure mode in this slice because drift can blind operators without tripping compiler or runtime checks.
- Stable: install-path endpoint labeling remains low-severity and operational, not correctness-critical.

## Findings

### Medium

1. `DelegationApi` is now a coordination surface, not just a large module, and its current split is by convenience rather than invariant boundary.
   - Evidence:
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L222)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L350)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L484)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L629)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L850)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L956)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L978)
     - [crates/canic-core/src/ops/auth/verify.rs](/home/adam/projects/canic/crates/canic-core/src/ops/auth/verify.rs#L212)
     - [crates/canic-core/src/ops/auth/verify.rs](/home/adam/projects/canic/crates/canic-core/src/ops/auth/verify.rs#L223)
   - Why this matters: correctness is currently preserved by review discipline across multiple local helpers and duplicate runtime checks, not by structural separation. The next auth feature is likely to update one path and miss another.
   - Direction: split by invariant boundary, not file size. Minimal viable target:
     - `api/auth/mod.rs` as thin orchestration only
     - `api/auth/audience.rs`
     - `api/auth/admin.rs`
     - `api/auth/session.rs`
     - `api/auth/issuance.rs`
     - `api/auth/proof_store.rs`

2. Rollout metrics remain a string-based protocol without a schema, which is the highest latent failure risk in this slice.
   - Evidence:
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L339)
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L379)
     - [crates/canic-core/src/ops/runtime/metrics/mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/mapper.rs#L185)
     - [crates/canic-core/src/ops/runtime/metrics/mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/mapper.rs#L223)
     - [crates/canic-core/src/ops/runtime/metrics/mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/mapper.rs#L254)
     - [crates/canic-core/src/workflow/metrics/query.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/metrics/query.rs#L145)
     - [crates/canic-core/src/workflow/metrics/query.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/metrics/query.rs#L256)
   - Why this matters: control-hub complexity creates future development bugs; metrics string coupling creates silent production blindness. Producers, classifiers, and queries can drift without compiler errors.
   - Required direction: move to a typed metric predicate/signal model and confine string literals to the export boundary only.

### Low

3. Install-path metric labeling still conflates role and observation location.
   - Evidence:
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L6)
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L339)
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L379)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L484)
     - [crates/canic-core/tests/pic_role_attestation.rs](/home/adam/projects/canic/crates/canic-core/tests/pic_role_attestation.rs#L732)
   - Why this matters: the current `auth_signer` endpoint label is operationally confusing for verifier-local failures, though not functionally incorrect.
   - Direction: add orthogonal role/location or role/phase dimensions, keeping current labels only as compatibility shims if needed.

## Structural Hotspots

| File / Module | Role | LOC | Branching Signals | Risk |
| --- | --- | ---: | --- | --- |
| `crates/canic-core/src/api/auth/mod.rs` | auth façade / coordination surface | 1102 | `55 if`, `13 match` | High |
| `crates/canic-core/src/ops/runtime/metrics/auth.rs` | auth metric producer hub | 935 | `2 if`, `11 match` | Medium |
| `crates/canic-core/src/ops/runtime/metrics/mapper.rs` | rollout signal classifier | 442 | `2 if`, `3 match` | Medium |
| `crates/canic-core/src/workflow/auth.rs` | root orchestration / fanout | 347 | `11 match` | Medium |

## Hub Import Pressure

Daily baseline delta policy:
- Same-day rerun compares against baseline `docs/audits/reports/2026-03/2026-03-24/complexity-accretion.md`.
- No code delta in audited files since baseline, so numeric pressure deltas are `0`.

| Module | Top Imports / Subsystems | Unique Sibling Subsystems | Cross-Layer Dependency Count | Delta vs Daily Baseline |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/api/auth/mod.rs` | `access, config, dto, log, ops, protocol, storage, workflow, cdk` | 9 | 4 | 0 |
| `crates/canic-core/src/workflow/auth.rs` | `dto, log, ops, protocol, cdk` | 5 | 1 | 0 |
| `crates/canic-core/src/ops/runtime/metrics/auth.rs` | `access, config, dto, ids, ops` | 5 | 2 | 0 |
| `crates/canic-core/src/ops/runtime/metrics/mapper.rs` | `access, dto, ids, ops, cdk` | 5 | 2 | 0 |

## Primary Architectural Pressure

`crates/canic-core/src/ops/runtime/metrics/{auth,mapper}.rs`

Reasons:
- string literals currently form an untyped contract between producer and classifier
- failures here are more likely to remove observability than to trigger local test failures
- this creates a short-term silent-failure mode that is more urgent than the longer-term façade decomposition

## Risk Score

Risk Score: **6.5 / 10**

Score contributions:
- `+3` auth control-hub concentration in `api/auth/mod.rs`
- `+2.5` string-coupled rollout metric derivation with silent-failure potential
- `+1` endpoint-locality ambiguity in install metrics
- `+0` test hotspot contribution (tracked, not scored directly)

Verdict: **No correctness regression in audience-binding, but the auth slice is semantically strong and structurally stressed.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `sed -n '1,220p' docs/audits/reports/2026-03/2026-03-24/complexity-accretion.md` | PASS | baseline report re-read for same-day comparison |
| `rg -n "pub async fn admin|pub fn set_delegated_session_subject|pub async fn store_proof|fn proof_is_reusable_for_claims|fn ensure_token_claim_audience_subset|fn ensure_target_in_proof_audience" crates/canic-core/src/api/auth/mod.rs` | PASS | concentrated auth seams confirmed |
| `rg -n "AUTH_ROLLOUT_SIGNAL_SPECS|predicate_is_proof_miss|predicate_is_prewarm_failure" crates/canic-core/src/ops/runtime/metrics/mapper.rs` | PASS | string-classifier seam confirmed |
| `rg -n "AUTH_SIGNER_ENDPOINT|record_delegation_install_validation_failed" crates/canic-core/src/ops/runtime/metrics/auth.rs` | PASS | endpoint-label locality issue confirmed |

## Follow-up Actions

1. Owner boundary: `api/auth`
   Action: extract shared audience invariant helpers first into `api/auth/audience.rs`, then update all callers including runtime verification paths before splitting the rest of `api/auth`.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/complexity-accretion.md`

2. Owner boundary: `ops/runtime/metrics`
   Action: introduce a typed auth metric predicate/signal enum, dual-write temporarily, convert mapper/query to the typed path, then remove string matching.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/complexity-accretion.md`

3. Owner boundary: `ops/runtime/metrics` + `api/auth`
   Action: document metric label semantics (`role`, `phase`, optional location) in module docs or a metrics README before changing labels, so compatibility shims do not drift.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/complexity-accretion.md`
