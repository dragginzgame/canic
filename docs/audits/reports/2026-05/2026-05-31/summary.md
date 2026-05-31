# Audit Summary - 2026-05-31

## Included Reports

| Report | Status | Summary |
| --- | --- | --- |
| `capability-surface.md` | PASS WITH TEMPLATE CLEANUP | Current fleet artifacts refreshed successfully, retained public DID scans passed, and workspace clippy passed. Risk remains `3 / 10`; endpoint directory-module, core/facade protocol, and roster-filtering template cleanups were applied after the run. |

## Findings

- No hard surface violations were found in the retained public fleet roster.
- Root/admin surfaces remained root-only, non-root cascade endpoints remained
  non-root-only, and retired `canic_delegation_set_*` endpoints remained absent.
- `canic_memory_ledger` is present on the retained fleet as an intentional,
  controller-gated default diagnostic.
- The recurring audit template needed one more path refresh:
  `crates/canic/src/macros/endpoints.rs` has become
  `crates/canic/src/macros/endpoints/**`; that cleanup was applied after the
  run.

## Verification Rollup

| Report | PASS | BLOCKED | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `capability-surface.md` | 8 | 0 | 0 | Artifact refresh, macro scans, DID scans, wire/DTO scans, and workspace clippy passed. |

## Follow-ups

1. Completed after the run: update the capability-surface recurring template to use
   `crates/canic/src/macros/endpoints/**` in all macro scan examples.
2. Completed after the run: require future DID scans to be tied to the selected
   fleet role list and explicitly filter stale local `.icp` artifacts.
3. Completed after the run: include both `canic-core/src/protocol.rs` and
   `canic/src/protocol.rs` in future capability-surface wire scans.
