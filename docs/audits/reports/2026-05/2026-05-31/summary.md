# Audit Summary - 2026-05-31

## Included Reports

| Report | Status | Summary |
| --- | --- | --- |
| `capability-surface.md` | PASS WITH TEMPLATE CLEANUP | Current fleet artifacts refreshed successfully, retained public DID scans passed, and workspace clippy passed. Risk remains `3 / 10`; endpoint directory-module, core/facade protocol, and roster-filtering template cleanups were applied after the run. |
| `complexity-accretion.md` | PASS WITH MODULE CLEANUP | Runtime file/LOC and enum growth were measured against the 2026-05-09 baseline. Risk remains `3 / 10` after splitting the only non-test `>= 600 LOC` hotspot, `api/ic/canic.rs`, then decomposing sharding and directory placement facades. |

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
- The complexity run found one actionable production-named hotspot:
  `crates/canic-core/src/api/ic/canic.rs` mixed protected internal-call facade
  code with 24 focused tests and reached `908` logical LOC. It is now a
  directory module with endpoint descriptor, envelope, proof-cache, facade, and
  test responsibilities split; the largest protected internal-call production
  file is `api/ic/canic/mod.rs` at `375` logical LOC.
- After the complexity cleanup, the current `>= 600` LOC files in
  `canic-core/src` are test-only harnesses.
- The follow-up sharding cleanup moved allocation, bootstrap, and registry
  helpers out of `workflow/placement/sharding/mod.rs`, reducing that facade
  from `560` to `291` logical LOC.
- The follow-up directory cleanup moved classification, create/finalize,
  cleanup/recovery, and config resolution helpers out of
  `workflow/placement/directory/mod.rs`, reducing that facade from `529` to
  `210` logical LOC.

## Verification Rollup

| Report | PASS | BLOCKED | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `capability-surface.md` | 8 | 0 | 0 | Artifact refresh, macro scans, DID scans, wire/DTO scans, and workspace clippy passed. |
| `complexity-accretion.md` | 11 | 0 | 0 | Runtime LOC, subsystem, enum/reference, large-file, and branch-density scans passed; `cargo fmt --all`, focused `api::ic::canic` tests, focused sharding compile, and focused placement tests passed. |

## Follow-ups

1. Completed after the run: update the capability-surface recurring template to use
   `crates/canic/src/macros/endpoints/**` in all macro scan examples.
2. Completed after the run: require future DID scans to be tied to the selected
   fleet role list and explicitly filter stale local `.icp` artifacts.
3. Completed after the run: include both `canic-core/src/protocol.rs` and
   `canic/src/protocol.rs` in future capability-surface wire scans.
4. Completed after the run: split protected internal-call facade tests,
   endpoint descriptors, envelope encoding, and proof-cache state out of
   `crates/canic-core/src/api/ic/canic.rs`; update the complexity recurring
   template to distinguish production large-file pressure from test harness
   size.
5. Completed after the run: split sharding placement allocation, bootstrap, and
   registry helpers into focused sibling modules under
   `workflow/placement/sharding/`.
6. Completed after the run: split directory placement classification,
   create/finalize, cleanup/recovery, and config resolution helpers into
   focused sibling modules under `workflow/placement/directory/`.
7. Carry forward: keep new request-handler branch axes in focused helper
   modules before `workflow/rpc/request/handler/*` crosses the production
   large-file threshold.
