# 0.86 Status: Structural Maintainability

Last updated: 2026-07-12

## Purpose

This file tracks the bounded mechanical splits defined by
[0.86-design.md](0.86-design.md) so the work is not reconstructed from chat
history.

## Current State

The three Medic slices are published as `v0.86.0` through `v0.86.2`.
Auth-renewal, blob-storage, project configuration, state audit, role-package,
resolved role-contract, and deployment diagnostics have focused owners. The
Medic structural pass is complete.

The first deploy-plan slice is published as `v0.86.3`. Report rendering, JSON
output persistence, and exit classification have one focused owner; command
options, root discovery, parsing, and usage have another.

The verified-evidence slice is published as `v0.86.4`. Context, identity,
artifact, inventory, authority, trust-domain, and verifier-readiness evidence
have one focused owner.

The diagnostics slice is published as `v0.86.5`. Target-resolution blockers,
unsupported and blocking assumptions, local-state warnings, unresolved
assumptions, stable diagnostic codes, and diagnostic-specific next actions have
one focused owner.

The deploy-plan closeout is published as `v0.86.6` and completes its structural
pass. Proposed
operation labels, global next actions, aggregate status, comparison status, and
deterministic diagnostic/operation ordering move to one final-outcome module.
Serialized report fields, statuses, diagnostics, proposed-operation labels, and
their stable strings move to one report-model module. The 587-line parent
retains command orchestration, report assembly, path/profile helpers, and
focused tests. Policy, CLI behavior, JSON field order, text output, and exit
behavior are unchanged.

The first Slice C batch is published as `v0.86.7`. State-manifest
package/config and built-in `wasm_store`
resolution now has one focused owner; the parent facade re-exports the existing
resolution type and function. Resolution behavior, blocking findings,
descriptor materialization, manifests, and reports are unchanged. The same
batch bounds release disk use by disabling disposable incremental state,
avoiding duplicate main/tag Clippy and PocketIC jobs, and cleaning local Cargo
artifacts after a successful release push.

The current unreleased slice completes Slice C and the bounded 0.86 structural
pass. All audit-check construction moves into one focused owner. Schema,
role/domain identity, memory-ID, storage, naming, snapshot, migration,
test-coverage, lifecycle, invariant, and reserved-memory checks move together
with their typed category/source constants. Status aggregation, next-action
projection, and deterministic check ordering move to a second focused owner.
Descriptor joining already has one canonical owner in the role-contract
descriptor subsystem and is not duplicated here.

The 846-line parent retains the public report model, orchestration, facade, and
focused tests. Finding codes, details, severity, order, next actions, reports,
serialization, state contracts, and persisted bytes are unchanged. The parent
is 726 lines smaller than `0.86.7` and 892 lines smaller than the 1,738-line
Slice C baseline. This closeout is changelog-finalized for `0.86.8`; package
versions remain `0.86.7` pending the human-owned release flow.

## Checklist

### Slice A - Medic

- [x] Extract auth-renewal checks.
- [x] Extract blob-storage checks and passive endpoint detection.
- [x] Extract role-package and resolved role-contract checks.
- [x] Extract project configuration and state-audit checks.
- [x] Extract deployment context, state, receipt, registry, and readiness checks.
- [x] Complete the Medic structural pass and record its final module boundary.

### Slice B - Deploy plan

- [x] Extract rendering, output persistence, and exit classification.
- [x] Extract command inputs, root discovery, parsing, and usage.
- [x] Extract verified evidence construction.
- [x] Extract blocker, warning, and assumption diagnostics.
- [x] Extract comparison, status aggregation, and proposed operations.
- [x] Extract serialized report types and stable labels.
- [x] Preserve command, exit, and report contracts exactly.

### Slice C - State manifest

- [x] Extract package/config and built-in-role resolution.
- [x] Keep descriptor joining with its existing role-contract descriptor owner.
- [x] Extract audit-category construction.
- [x] Extract aggregation and next-action projection.
- [x] Preserve state-contract, report, and serialized contracts exactly.

## Validation

- `cargo test -p canic-cli medic:: --lib`: 51 passed.
- `cargo test -p canic-cli deploy::tests::plan --lib`: 18 passed.
- `cargo test -p canic-cli deploy::plan::tests --lib`: 12 passed.
- Focused cached subnet-catalog host test against `ic-query 0.10.4`: passed.
- `cargo clippy -p canic-cli --lib -- -D warnings`: passed.
- `cargo test --locked -p canic-host state_manifest:: --lib`: 21 passed.
- `cargo clippy --locked -p canic-host --lib -- -D warnings`: passed.
- Post-`0.86.7` audit-owner extraction: the same 21 focused state-manifest
  tests and targeted host-library Clippy pass.
- Slice C closeout after aggregation extraction: the same 21 focused
  state-manifest tests and targeted host-library Clippy pass.

## Next Action

Run the human-owned `0.86.8` release flow after reviewing the finalized patch.
After publication, close the bounded 0.86 line; any further structural program
requires a fresh audit and design rather than extending this scope.
