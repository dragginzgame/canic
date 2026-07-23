# 0.87 Status: Operator Boundary Hygiene

Last updated: 2026-07-13

## Current State

The fresh post-0.86 audit defines a bounded three-slice line. Slice A is
published as `0.87.0`. Slices B and C are published as `0.87.1`. Slice C's
environment-input audit is complete: public profile, path/config,
cache-retention, target-directory, and Candid-refresh shortcuts are hard-cut;
three unread child values are deleted; and the remaining Cargo handoffs have
private names owned by core constants. Package versions are `0.87.1`.

Slice A routes scaffold workspace replacement through the existing durable
host writer. Project and canister scaffolds use one rollback function, restore
captured workspace/fleet bytes exactly, and remove only preflight-proven new
directories. Rollback failure is typed and retains the original operation
failure.

Slice B gives the host ICP adapter one typed classifier for external diagnostics
on which Canic acts. Commands retain the original `IcpCommandError` rather than
copying command/output strings and losing I/O or JSON sources. Command-specific
exit and hint policy remains local.

A post-release closeout scan found one install-root helper that still matched
three missing-canister-ID phrases after erasing `IcpCommandError`. The current
correction moves those phrases into the existing classifier, keeps the command
failure typed through root resolution, and removes the local helper. It is
release-noted as `0.87.2`: a conformance correction to completed Slice B, not a
fourth slice or 0.88 carry-over. Package versions remain `0.87.1` until the
human-owned release bump.

## Checklist

### Slice A - Scaffold failure atomicity

- [x] Expose one existing durable host file-replacement entry point.
- [x] Replace the scaffold workspace manifest's truncating write.
- [x] Capture the exact pre-mutation workspace and fleet bytes.
- [x] Add one rollback function for new scaffold directories and changed files.
- [x] Preserve the original failure and report any rollback failure.
- [x] Prove successful output and failure restoration with focused tests.

### Slice B - Typed ICP failure boundary

- [x] Inventory the exact external ICP diagnostics Canic acts upon.
- [x] Add one typed classifier under `canic-host::icp`.
- [x] Remove consumer-local ICP diagnostic string matching.
- [x] Remove repeated command-side `IcpCommandError` reconstruction.
- [x] Remove repeated installed-deployment transport reconstruction.
- [x] Preserve focused exit, hint, source, and raw-diagnostic behavior.

### Slice C - Environment-input audit and hard cuts

- [x] Inventory product-level `CANIC_*` environment inputs.
- [x] Record preliminary remove, internalize, or keep decisions.
- [x] Remove variables with no production reader.
- [x] Replace public Wasm profile selection with existing typed inputs.
- [x] Remove public path/config shortcuts and use explicit inputs or discovery.
- [x] Internalize only the required config and ICP-root Cargo handoff values.
- [x] Convert path/config precedence tests to explicit inputs.
- [x] Delete host test environment mutation helpers and locks.
- [x] Remove public shortcuts duplicated by explicit inputs or discovery.
- [x] Internalize required Cargo/build-script handoff values.
- [x] Remove stale help, README, script, and fixture references.

## Validation

- All 23 focused scaffold tests pass.
- Targeted `canic-cli` library Clippy passes with warnings denied.
- Cargo-machete passes after the two audit-identified manifest metadata fixes.
- Cargo metadata, formatting, changelog governance, and diff-hygiene checks
  pass.
- All 24 focused host ICP tests, two installed-deployment tests, and five
  install-readiness parsing tests pass for Slice B.
- Focused auth, blob-storage, cycles, token, backup-create, snapshot, list,
  metrics, environment-export, inspect, replica, and Medic CLI tests pass.
- Targeted host and CLI library Clippy passes with warnings denied.
- The active source/script/documentation scan contains no remaining
  `CANIC_WASM_PROFILE` or removed unread child-build environment references.
- The active source/script/help scan contains no removed public path/config
  names; all host test environment mutations and both locks are gone.
- The final source/script/help scan contains no removed cache-retention,
  target-directory, Candid-refresh, or old embedded-artifact marker names.
- Full workspace, PocketIC, and release suites were not run under the targeted
  validation policy.

## Next Action

The three planned 0.87 slices are complete and the narrow Slice B closeout is
prepared as `0.87.2`. Run the human-owned release bump, then begin the
separately designed 0.88 work. Do not add another 0.87 slice or preserve
removed environment names as aliases.
