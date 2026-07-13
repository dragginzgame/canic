# 0.87 Status: Operator Boundary Hygiene

Last updated: 2026-07-12

## Current State

The fresh post-0.86 audit defines a bounded three-slice line. Slice A is
changelog-finalized for `0.87.0`; Slices B and C have not started. Package
versions remain `0.86.8` pending the human-owned release flow.

Slice A routes scaffold workspace replacement through the existing durable
host writer. Project and canister scaffolds use one rollback function, restore
captured workspace/fleet bytes exactly, and remove only preflight-proven new
directories. Rollback failure is typed and retains the original operation
failure.

The next slice will converge repeated ICP and installed-deployment error flows
at the existing host ICP adapter. It will not add a global error framework or
move command-specific exit and hint policy into the host.

## Checklist

### Slice A - Scaffold failure atomicity

- [x] Expose one existing durable host file-replacement entry point.
- [x] Replace the scaffold workspace manifest's truncating write.
- [x] Capture the exact pre-mutation workspace and fleet bytes.
- [x] Add one rollback function for new scaffold directories and changed files.
- [x] Preserve the original failure and report any rollback failure.
- [x] Prove successful output and failure restoration with focused tests.

### Slice B - Typed ICP failure boundary

- [ ] Inventory the exact external ICP diagnostics Canic acts upon.
- [ ] Add one typed classifier under `canic-host::icp`.
- [ ] Remove consumer-local ICP diagnostic string matching.
- [ ] Remove repeated command-side `IcpCommandError` reconstruction.
- [ ] Remove repeated installed-deployment transport reconstruction.
- [ ] Preserve focused exit, hint, source, and raw-diagnostic behavior.

### Slice C - Pure path precedence inputs

- [ ] Extract focused pure precedence functions behind current public wrappers.
- [ ] Convert release-set path tests to explicit inputs.
- [ ] Convert install-root selection/preflight tests to explicit inputs.
- [ ] Delete host test environment mutation helpers and locks.
- [ ] Prove existing precedence and path normalization with focused tests.

## Validation

- All 23 focused scaffold tests pass.
- Targeted `canic-cli` library Clippy passes with warnings denied.
- Cargo-machete passes after the two audit-identified manifest metadata fixes.
- Cargo metadata, formatting, changelog governance, and diff-hygiene checks
  pass.
- Full workspace, PocketIC, and release suites were not run under the targeted
  validation policy.

## Next Action

Run the human-owned `0.87.0` release flow after reviewing the finalized patch,
then implement Slice B as a hard cut. `0.87.0` opens the line; it does not close
the broader design. Do not broaden Slice B into a general error architecture.
