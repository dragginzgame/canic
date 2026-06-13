# Audit Summary - 2026-06-13

## Run Contexts

| Report | Type | Scope | Status |
| ---- | ---- | ---- | ---- |
| `audience-target-binding.md` | Recurring invariant | delegated-token audience binding, role-attestation audience binding, root capability target hashing, and current capability proof routing | PASS |
| `capability-scope-enforcement.md` | Recurring invariant | delegated-token verify/bind/scope ordering, root capability envelope/proof validation, authorization/replay ordering, and endpoint-boundary rejection | PASS |
| `change-friction.md` | Recurring system | current feature-slice breadth, CAF/locality, boundary leakage, enum shock radius, and gravity-well pressure | PASS AFTER FIX |

## Risk Index Summary

| Report | Risk | Notes |
| ---- | ----: | ---- |
| `audience-target-binding.md` | 3 / 10 | Active audience-target verifier surfaces passed. Older internal-invocation and delegated-grant wording is now method drift against current code. |
| `capability-scope-enforcement.md` | 3 / 10 | Active scope enforcement and structural capability paths passed. Older standalone delegated-grant and role-attestation proof-mode wording is now method drift against current code. |
| `change-friction.md` | 4 / 10 | Current slices are broader than the prior 0.48 sample; the one workflow/storage type crossing found by the initial scan was fixed before finalization, and broad host test cleanup is treated as structural sweep noise. |

## Method / Comparability Notes

- `audience-target-binding.md` uses `Method V4.1-current-surface`.
- The run is partially comparable with
  `docs/audits/reports/2026-05/2026-05-29/audience-target-binding.md`.
- Delegated-token and role-attestation verifier checks remain comparable.
- Internal-invocation and delegated-grant surfaces named by the prior report
  were not found in the current code scan.
- `capability-scope-enforcement.md` uses `Method V4.2-current-surface`.
- The run is partially comparable with
  `docs/audits/reports/2026-05/2026-05-29/capability-scope-enforcement.md`.
- Delegated-token verify/bind/scope ordering and structural capability paths
  remain comparable.
- Standalone delegated-grant and role-attestation capability proof modes named
  by the prior report were not found as active current runtime proof modes.
- `change-friction.md` uses `change-friction-current-surface`.
- The run is partially comparable with
  `docs/audits/reports/2026-05/2026-05-29/change-friction.md` because the
  current sample covers post-0.65 auth cleanup, operator-helper extraction, and
  host test/module cleanup rather than the 0.48 setup window.

## Key Findings

### Critical

- None.

### High

- None.

### Medium

- Audit method drift: recurring wording still names retired or absent
  internal-invocation and delegated-grant verifier surfaces.
- Capability proof routing currently accepts structural proof mode at runtime;
  target hash binding remains covered as a helper surface by unit tests.
- Capability-scope method drift: older reports name standalone delegated-grant
  and role-attestation capability proof modes that are absent from current
  runtime routing.
- Change friction: sampled routine feature-slice average file count rose from
  28.43 to 36.20, with host deployment-truth decomposition remaining the main
  friction target.

### Low

- Moderate module pressure remains around delegated auth and capability
  workflow helpers.
- Capability DTOs and root capability request handlers remain expected but
  sensitive fan-in points.
- The initial `workflow/pool/mod.rs` storage-record type crossing was
  remediated through an ops-owned pool metadata projection.
- `0.67.1` host test/module cleanup has broad file churn but high locality, so
  it is tracked as a cleanup sweep rather than routine feature friction.

## Verification Readout Rollup

| Report | PASS | FAIL | BLOCKED |
| ---- | ----: | ----: | ----: |
| `audience-target-binding.md` | 7 | 0 | 0 |
| `capability-scope-enforcement.md` | 6 | 0 | 0 |
| `change-friction.md` | 6 validation commands plus source scans | 0 | 0 |

## Follow-up Actions

- Update the recurring audit definition later if the retired surfaces should no
  longer be part of the expected scope.
- Re-run after changes under `dto/auth.rs`, `ops/auth/delegated`,
  `ops/auth/verify/attestation.rs`, or `workflow/rpc/capability`.
- Keep endpoint delegated-token auth ordering, capability DTO/proof enums, and
  root capability authorization/replay changes coordinated across API,
  workflow, metrics, and tests.
- Keep the broader workflow-purity watchpoint active for future record
  carriers and Candid codecs. The direct `workflow/pool` storage-record
  reference was closed in `0.67.2`, and workflow/API registry-record
  projection was closed in the next cleanup slice.
- Keep host deployment-truth decomposition as the main friction target before
  adding more promotion/lifecycle report families.
