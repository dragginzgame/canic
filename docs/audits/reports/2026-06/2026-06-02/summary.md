# Audit Summary - 2026-06-02

## Run Contexts

| Report | Type | Scope | Status |
| ---- | ---- | ---- | ---- |
| `dry-consolidation.md` | Recurring system | repo-wide DRY consolidation | PASS |
| `canic-cli-deploy-promote-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/deploy/promote.rs` | PASS |
| `canic-cli-deploy-check-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/deploy/check.rs` | PASS |
| `canic-cli-deploy-external-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/deploy/external/` | PASS |
| `canic-cli-deploy-authority-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/deploy/authority.rs` | PASS |
| `canic-cli-deploy-root-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/deploy/root.rs` | PASS |
| `canic-cli-deploy-install-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/deploy/install.rs` | PASS |
| `canic-cli-metrics-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/metrics/` | PASS |
| `canic-cli-endpoints-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/endpoints/` | PASS |

## Risk Index Summary

| Report | Risk | Notes |
| ---- | ----: | ---- |
| `dry-consolidation.md` | 4 / 10 | No blocker or high-severity duplication issue found. |
| `canic-cli-deploy-promote-module-surface-hardening.md` | 3 / 10 | Passive CLI promotion report surface is retained with owner; no safe immediate cleanup found. |
| `canic-cli-deploy-check-module-surface-hardening.md` | 3 / 10 | Local deployment-check evidence envelope surface is retained with owner; no safe immediate cleanup found. |
| `canic-cli-deploy-external-module-surface-hardening.md` | 4 / 10 | Broad passive external lifecycle report surface is retained with owner; no safe immediate cleanup found. |
| `canic-cli-deploy-authority-module-surface-hardening.md` | 3 / 10 | Dry-run controller authority report surface is retained with owner; no safe immediate cleanup found. |
| `canic-cli-deploy-root-module-surface-hardening.md` | 4 / 10 | Deployment-root inspect/verify surface is retained with owner; no safe immediate cleanup found. |
| `canic-cli-deploy-install-module-surface-hardening.md` | 5 / 10 | Active install-runner CLI boundary is retained with owner; no safe immediate cleanup found. |
| `canic-cli-metrics-module-surface-hardening.md` | 4 / 10 | Query-only runtime telemetry surface is retained with owner; no cleanup found. |
| `canic-cli-endpoints-module-surface-hardening.md` | 4 / 10 | Read-only endpoint discovery surface is retained with owner; no cleanup found. |

## Method / Comparability Notes

- `dry-consolidation.md` follows the recurring DRY consolidation method.
- `canic-cli-deploy-promote-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-deploy-check-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-deploy-external-module-surface-hardening.md` uses `MSH-2.0` and
  is non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-deploy-authority-module-surface-hardening.md` uses `MSH-2.0` and
  is non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-deploy-root-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-deploy-install-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-metrics-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-endpoints-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.

## Key Findings

- No High or Critical findings were recorded in the retained reports.
- The deploy promote module remains passive and request-file based.
- The deploy check module remains evidence-only and uses the shared deployment
  truth loader plus host evidence-envelope DTOs.
- The deploy external module remains passive, request-file or local-check
  driven, and delegates lifecycle construction to host-owned helpers.
- The deploy authority module remains dry-run only and delegates authority
  artifact construction to host-owned helpers.
- The deploy root module keeps passive inspect and explicit registered-root
  verification separate, with state transition authority delegated to host.
- The deploy install module keeps plan-file decoding and option construction in
  the CLI while delegating install mutation to the host install runner.
- The metrics module remains query-only and delegates installed deployment
  resolution to host helpers before querying canister telemetry.
- The endpoints module remains read-only and delegates installed deployment
  resolution to host helpers before reading live `candid:service` metadata or
  local role `.did` artifacts.
- No mutation, ICP/DFX, network, or live deployment-truth observation primitive
  was found in the promote CLI path.
- No install/register/write primitive was found in the deploy check CLI module.
- No consent request, external execution, install/register/write primitive, or
  DFX/ICP primitive was found in inspected deploy external code.
- No controller mutation, install/register/write primitive, or DFX/ICP
  primitive was found in inspected deploy authority code.
- No canister/controller mutation, install-code primitive, or DFX/ICP primitive
  was found in inspected deploy root code.
- No direct management-canister primitive, registration write, promotion
  execution path, or DFX/ICP command was found in inspected deploy install code.
- No update/install/create/delete/register primitive was found in inspected
  metrics code.
- No update/install/create/delete/register primitive was found in inspected
  endpoints code.

## Verification Readout Rollup

| Report | PASS | FAIL | BLOCKED |
| ---- | ----: | ----: | ----: |
| `dry-consolidation.md` | documented in report | 0 | 0 |
| `canic-cli-deploy-promote-module-surface-hardening.md` | 2 | 0 | 0 |
| `canic-cli-deploy-check-module-surface-hardening.md` | 2 | 0 | 0 |
| `canic-cli-deploy-external-module-surface-hardening.md` | 3 | 0 | 0 |
| `canic-cli-deploy-authority-module-surface-hardening.md` | 2 | 0 | 0 |
| `canic-cli-deploy-root-module-surface-hardening.md` | 2 | 0 | 0 |
| `canic-cli-deploy-install-module-surface-hardening.md` | 2 | 0 | 0 |
| `canic-cli-metrics-module-surface-hardening.md` | 2 | 0 | 0 |
| `canic-cli-endpoints-module-surface-hardening.md` | 2 | 0 | 0 |

## Follow-up Actions

- No required follow-up from the promote MSH run.
- No required follow-up from the deploy-check MSH run.
- No required follow-up from the deploy-external MSH run.
- No required follow-up from the deploy-authority MSH run.
- No required follow-up from the deploy-root MSH run.
- No required follow-up from the deploy-install MSH run.
- No required follow-up from the metrics MSH run.
- No required follow-up from the endpoints MSH run.
- Carry forward the DRY consolidation watchpoints documented in
  `dry-consolidation.md`.
