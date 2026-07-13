# 0.88 Status: Artifact Durability and Typed Config Errors

Last updated: 2026-07-13

## Current State

The post-0.87 audit selects exactly three implementation slices. Slice A is
implemented and release-noted as `0.88.0`: both snapshot-finalization paths use
one backup-owned durable directory commit, and verified post-publication
artifacts can resume without trusting unrelated destinations. Package versions
remain `0.87.1` until the human-owned release bump.

One 0.87 closeout correction is release-noted as `0.87.2`: install-root no
longer converts a boxed ICP command failure to text to detect a missing
canister ID. The existing typed ICP classifier owns all recognized forms.
Package versions remain `0.87.1` until the human-owned release bump.

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

- [ ] Add explicit `create_new_bytes` beside the existing `write_bytes` entry
  point.
- [ ] Delegate both entry points to one private staging, sync, publication, and
  cleanup implementation.
- [ ] Require unique sibling staging and fail if the platform cannot provide
  required publication or directory-sync semantics.
- [ ] Make create-new publication atomically no-clobber without an
  exists-then-rename race.
- [ ] Route every file destination behind the three shared CLI output helpers
  through durable replacement.
- [ ] Route only deployment-plan `--out` through durable create-new.
- [ ] Preserve shared-output parent creation and deployment-plan missing-parent
  rejection.
- [ ] Durably create and sync every newly introduced parent entry.
- [ ] Prove replace results are old-complete or new-complete and create-new
  results are absent or new-complete across injected failures.
- [ ] Prove no in-scope path opens the final destination for truncating or
  incremental writes and no handled temporary residue remains.
- [ ] Keep scaffold, cycles pending-log, and subsystem-owned persistence paths
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
- Audit-only layering guard: pass.
- Cargo Machete: pass.
- Cargo audit: no known vulnerabilities; four upstream unmaintained warnings.
- Full workspace, PocketIC, deployment, and broad Wasm suites were not run
  under the targeted-validation policy.

## Next Action

Run the human-owned `0.88.0` release bump and push Slice A. Slice B is next; do
not extend the completed directory-commit owner into the host single-file
writer.
