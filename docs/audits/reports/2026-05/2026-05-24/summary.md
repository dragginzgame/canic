# Audit Summary - 2026-05-24

## Run Contexts

| Report | Type | Scope | Snapshot | Worktree | Status |
| --- | --- | --- | --- | --- | --- |
| `canic-testkit-artifacts-module-surface-hardening.md` | Modular Tier 1 MSH pilot | `crates/canic-testkit/src/artifacts` and duplicate consumer surface in `canic-testing-internal` | `0eaad7dc` | dirty before report write | complete |

## Risk Index Summary

| Report | Risk | Readout |
| --- | ---: | --- |
| `canic-testkit-artifacts-module-surface-hardening.md` | 2 / 10 | Public artifact helpers are retained with `canic-testkit::artifacts` ownership; one duplicate consumer helper cluster was moved back to that owner. |

## Method and Comparability Notes

- `canic-testkit-artifacts-module-surface-hardening.md` uses `MSH-2.0`.
- This is the first Tier 1 compact Module Surface Hardening pilot for this
  module, so comparability is `non-comparable`.
- The run validates the new compact report shape and evidence log without
  invoking the full Tier 2 MSH table set.

## Key Findings by Severity

### Low

- `canic-testing-internal/src/pic/attestation/build.rs` duplicated wasm
  path/read helpers that already belong to `canic-testkit::artifacts`; the
  cleanup now delegates to the canonical helper.
- `icp_artifact_ready_for_build` is retained as public test-support API despite
  direct self-test-only evidence because it is the simple wrapper around
  `WatchedInputSnapshot` capture. Revisit only if a later testkit surface audit
  shows no downstream use.

## Verification Rollup

| Report | PASS | BLOCKED | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `canic-testkit-artifacts-module-surface-hardening.md` | 5 | 0 | 0 | `cargo fmt --all`, package checks for `canic-testkit` and `canic-testing-internal`, focused artifact helper tests, and `git diff --check` passed. |

## Follow-up Actions

No required follow-up actions.

Watchpoint only:

1. Revisit `icp_artifact_ready_for_build` if the next testkit surface audit
   still finds no consumers outside its own tests.
