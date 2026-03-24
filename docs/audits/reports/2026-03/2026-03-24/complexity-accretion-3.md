# Complexity Accretion Audit - 2026-03-24 (Rerun 3)

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
| Auth decomposition order constrained | PASS | rerun now requires shared invariant extraction before file/module split |
| Metrics typing priority clarified | PASS | rerun now treats typed rollout metrics as high-priority, before next auth feature |
| Label semantics documentation requirement captured | PASS | follow-up now requires metrics README or module-header documentation |
| Refactor anti-goals recorded | PASS | rerun explicitly rejects traits/interfaces, over-splitting, and API-surface churn |
| Same-day baseline comparison preserved | PASS | comparison remains anchored to `complexity-accretion.md`, not prior reruns |

## Comparison to Baseline

- Stable: no new correctness break was found in audience-binding paths.
- Refined: auth decomposition must begin with shared audience invariant extraction, otherwise the refactor only moves duplication around.
- Refined: typed rollout metrics are now classified as a high-priority fix before the next auth feature.
- Refined: label cleanup is acceptable only if semantics are documented at the module/README level.
- Added: explicit anti-goals now constrain the refactor away from indirection-heavy or API-shaping churn.

## Findings

### Medium

1. `DelegationApi` remains the main coordination hotspot, but the immediate corrective action is sequencing, not broad restructuring.
   - Evidence:
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L222)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L350)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L484)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L850)
     - [crates/canic-core/src/api/auth/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/mod.rs#L978)
     - [crates/canic-core/src/ops/auth/verify.rs](/home/adam/projects/canic/crates/canic-core/src/ops/auth/verify.rs#L212)
     - [crates/canic-core/src/ops/auth/verify.rs](/home/adam/projects/canic/crates/canic-core/src/ops/auth/verify.rs#L223)
   - Why this matters: splitting files before extracting shared invariants would preserve divergence risk under a cleaner directory tree.
   - Required order:
     1. extract `audience.rs`
     2. update all call sites
     3. then split the remaining auth modules

2. Rollout metrics coupling is still the most dangerous latent issue because it can fail silently.
   - Evidence:
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L339)
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L379)
     - [crates/canic-core/src/ops/runtime/metrics/mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/mapper.rs#L185)
     - [crates/canic-core/src/ops/runtime/metrics/mapper.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/mapper.rs#L223)
     - [crates/canic-core/src/workflow/metrics/query.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/metrics/query.rs#L145)
   - Why this matters: unlike the control hub, string-coupled metrics can remove production visibility without breaking tests or compilation.
   - Priority: HIGH, before the next auth feature lands.

### Low

3. Label semantics remain a cleanup item, but only if documentation is added with the change.
   - Evidence:
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L6)
     - [crates/canic-core/src/ops/runtime/metrics/auth.rs](/home/adam/projects/canic/crates/canic-core/src/ops/runtime/metrics/auth.rs#L339)
   - Why this matters: without a metrics README or module-header semantic block, label drift will recur after cleanup.

## Refactor Constraints

### Do

- extract shared audience invariant helpers first
- keep `mod.rs` thin once extraction is complete
- type rollout metrics before adding more auth behavior
- document label semantics where the metrics are defined or exported

### Avoid

- premature abstraction via traits/interfaces
- over-splitting into many tiny modules
- changing external API surface during structural refactor

Goal: reduce synchronization points without increasing indirection.

## Risk Score

Risk Score: **6.5 / 10**

Score contributions:
- `+3` auth control-hub concentration in `api/auth/mod.rs`
- `+2.5` string-coupled rollout metrics with silent-failure risk
- `+1` label-semantics ambiguity pending documentation

Verdict: **Semantically strong, structurally stressed, with clear refactor ordering now defined.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `sed -n '1,240p' docs/audits/reports/2026-03/2026-03-24/complexity-accretion-2.md` | PASS | prior rerun re-read before refining actions |
| `sed -n '1,220p' docs/audits/reports/2026-03/2026-03-24/complexity-accretion.md` | PASS | daily baseline remains comparison anchor |
| `rg -n "fn ensure_token_claim_audience_subset|fn ensure_target_in_proof_audience" crates/canic-core/src/api/auth/mod.rs crates/canic-core/src/ops/auth/verify.rs` | PASS | invariant-sharing target points confirmed |

## Follow-up Actions

1. Owner boundary: `api/auth`
   Action: extract shared audience invariants into `api/auth/audience.rs`, update all call sites, then split remaining auth concerns into `admin`, `session`, `issuance`, and `proof_store`.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/complexity-accretion.md`

2. Owner boundary: `ops/runtime/metrics`
   Action: prioritize typed rollout metrics before the next auth feature slice; use an enum-backed pipeline and keep string literals at the export boundary only.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/complexity-accretion.md`

3. Owner boundary: `ops/runtime/metrics`
   Action: add metrics semantics documentation in a README or module header before changing label structure for verifier/install locality.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/complexity-accretion.md`
