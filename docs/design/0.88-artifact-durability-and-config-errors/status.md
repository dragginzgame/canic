# 0.88 Status: Artifact Durability and Typed Config Errors

Last updated: 2026-07-13

## Current State

The post-0.87 audit selects exactly three implementation slices. No 0.88 code
has started. Package versions remain `0.87.1`.

One 0.87 closeout correction is release-noted as `0.87.2`: install-root no
longer converts a boxed ICP command failure to text to detect a missing
canister ID. The existing typed ICP classifier owns all recognized forms.
Package versions remain `0.87.1` until the human-owned release bump.

## Checklist

### Slice A - Durable backup artifact commit

- [ ] Add one backup-owned durable artifact-directory commit function.
- [ ] Sync supported entries and directories before final rename.
- [ ] Sync the artifact parent after rename.
- [ ] Use the function from both artifact-finalization paths.
- [ ] Advance durable journal state only after commit success.
- [ ] Prove failure leaves the journal non-durable.

### Slice B - Failure-atomic CLI file output

- [ ] Add bounded durable create-new support beside the existing host writer.
- [ ] Route shared CLI JSON/text file output through durable replacement.
- [ ] Preserve deployment-plan no-overwrite behavior through create-new.
- [ ] Reuse create-new for scaffold files when behavior remains unchanged.
- [ ] Prove existing files are unchanged on serialization/write failure.
- [ ] Prove no temporary files remain after success or handled failure.

### Slice C - Typed fleet-config failure boundary

- [ ] Define one focused fleet-config error enum.
- [ ] Preserve I/O and core config errors as sources.
- [ ] Type invalid-input and mutation-conflict conditions.
- [ ] Remove boxed/string error construction from the config subtree.
- [ ] Preserve successful config bytes and rollback behavior.
- [ ] Keep outer command rendering and exits unchanged.

## Validation

- Audit-only layering guard: pass.
- Cargo Machete: pass.
- Cargo audit: no known vulnerabilities; four upstream unmaintained warnings.
- Full workspace, PocketIC, deployment, and broad Wasm suites were not run
  under the targeted-validation policy.

## Next Action

Run the human-owned `0.87.2` release bump, then begin 0.88 Slice A. Do not carry
unfinished 0.87 acceptance work into the 0.88 design.
