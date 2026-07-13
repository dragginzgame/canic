# 0.88 Status: Artifact Durability and Typed Config Errors

Last updated: 2026-07-13

## Current State

The post-0.87 audit selects exactly three implementation slices. Slice A is
published as `0.88.0`. Slice B is implemented and release-noted as `0.88.1`:
the existing host durable writer now owns atomic replace and create-new
publication through one private commit engine, all shared CLI file-output
helpers use it, and only deployment-plan `--out` uses the no-clobber entry
point. Package versions remain `0.88.0` until the human-owned release bump.

One 0.87 closeout correction was recorded before `0.88.0`: install-root no
longer converts a boxed ICP command failure to text to detect a missing
canister ID. The existing typed ICP classifier owns all recognized forms.

## Checklist

### Slice A - Durable backup artifact commit

- [x] Add one backup-owned durable artifact-directory commit function.
- [x] Require a unique sibling temporary directory and atomic no-replace
  publication.
- [x] Walk/open without following symlinks and reject unsupported entries.
- [x] Sync regular files, nested directories bottom-up, and the temporary root.
- [x] Sync the artifact parent after publication.
- [x] Use the function from both artifact-finalization paths.
- [x] Recover only a journal-bound, checksum-verified artifact published before
  its durable transition; reject every other existing destination.
- [x] Commit journal state and metric through an uncommitted copy so a failed
  durable journal replacement exposes neither update.
- [x] Prove every injected failure preserves the expected journal state and
  metric.

### Slice B - Publication-atomic CLI file output

- [x] Add explicit `create_new_bytes` beside the existing `write_bytes` entry
  point.
- [x] Delegate both entry points to one private staging, sync, publication, and
  cleanup implementation.
- [x] Require unique sibling staging and fail if the platform cannot provide
  required publication or directory-sync semantics.
- [x] Make create-new publication atomically no-clobber without an
  exists-then-rename race.
- [x] Route every file destination behind the three shared CLI output helpers
  through durable replacement.
- [x] Route only deployment-plan `--out` through durable create-new.
- [x] Preserve shared-output parent creation and deployment-plan missing-parent
  rejection.
- [x] Durably create and sync every newly introduced parent entry.
- [x] Prove replace results are old-complete or new-complete and create-new
  results are absent or new-complete across injected failures.
- [x] Prove no in-scope path opens the final destination for truncating or
  incremental writes and no handled temporary residue remains.
- [x] Keep scaffold, cycles pending-log, and subsystem-owned persistence paths
  outside the slice.

### Slice C - Typed fleet-config failure boundary

- [ ] Define one focused fleet-config error enum.
- [ ] Preserve I/O operation/path context and core-config operation context.
- [ ] Type invalid-input and mutation-conflict conditions.
- [ ] Preserve both typed causes when rollback fails.
- [ ] Remove boxed/string error construction from the config subtree.
- [ ] Erase the type only after exit, finding, and rollback decisions.
- [ ] Preserve successful projection values, serialized TOML, and rollback
  behavior.
- [ ] Keep outer command rendering and exits unchanged.

## Validation

- Four focused artifact-commit tests pass, including no-replace race,
  unsupported-entry, post-publication recovery, and injected sync/publication
  failures.
- Focused direct-capture success, destination-conflict, and failed-journal
  tests pass.
- Focused runner success, post-publication recovery, and checksum-rejection
  tests pass.
- Targeted `canic-backup` library Clippy passes with warnings denied.
- Nine focused host durable-I/O tests pass, including parent persistence,
  staging, sync, replace, atomic no-clobber race, cleanup, and
  post-publication failure cases.
- Three focused shared CLI output tests pass, including partial serialization
  failure before any destination or parent mutation.
- Focused deployment-plan tests preserve create-new, JSON newline, exit-code,
  and missing-parent behavior.
- Targeted `canic-host` and `canic-cli` test-target Clippy passes with warnings
  denied.
- Audit-only layering guard: pass.
- Cargo Machete: pass.
- Cargo audit: no known vulnerabilities; four upstream unmaintained warnings.
- Full workspace, PocketIC, deployment, and broad Wasm suites were not run
  under the targeted-validation policy.

## Next Action

Run the human-owned `0.88.1` release bump and push Slice B. Slice C is next; do
not widen the completed host writer into a filesystem framework or route
excluded multi-file and recovery-owned paths through it.
