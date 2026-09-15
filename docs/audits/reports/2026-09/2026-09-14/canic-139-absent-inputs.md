# CANIC-139: absent inputs discovered by Cargo

Date: 2026-09-14. Follow-up to the published 0.110.16 build-reuse behavior;
implementation belongs to the open 0.110.17 draft.

## Finding

Toko Miner's September 14 CANIC-139 update reports a first staging release
build that compiled eight Wasms and then rejected reuse recording. Both its
retained runtime and declaration `.d` files name the nonexistent path
`apps/toko_miner/game_shard/apps/toko_miner/game_shard/src/build.rs` beneath
the workspace. The retained `changed-inputs.json` is empty. The no-edit retry
completed release construction, then stopped at a separate retained-canister
availability check. No staging action is part of this Canic correction.

Evidence was inspected read-only under Toko Miner's
`artifacts/staging-recovery-2026-09-14/`: `prepare.log`, `prepare-retry.log`,
`changed-inputs.json`, `runtime-game-shard.d` and `declaration-game-shard.d`.
Both repositories' relevant published Canic source is
`a875c6498721bd89ca98549e38389b8280910d68` (`v0.110.16`).

The dependency reader correctly represents a missing input as `absent`.
Snapshot validation instead treated every newly named path beneath a scanned
directory as changed source, even when the new observation was absence.
Cargo inventory growth therefore produced a false source-drift diagnostic.
The historical pre-build dependency inventory was not retained, so this does
not reconstruct every difference in that invocation or the older .13 failure.

## Correction

Snapshot validation admits a newly recorded absent path only when the complete
pre-build scan proves an ancestor entry was missing. The proof uses the nearest
recorded directory, normal descendant path components and the same exclusion
predicate used by the source scanner. Skipped output/metadata directories do
not acquire source authority merely because their parent was scanned.

Actual additions beneath a proven missing entry remain `ChangedInput`.
Newly observed inputs without that proof remain `UnobservedInput`. Previously
recorded files and dropped Cargo entries retain their byte/existence checks.
Every new entry is checked, so admitting an absent input cannot hide a later
source addition. The final verified inventory remains the recorded cache key.

No dependency-path rewriting, file-existence bypass, modification-time
authority, cache schema change or finalized cross-release Wasm reuse is added.
Complete-release identity and output verification retain their existing owners.

## Qualification scope

The controlled regression creates a tiny Cargo/Wasm package with its actual
build script at `src/build.rs`, whose rerun declaration names the missing
package-prefixed `app/src/build.rs`. It starts without runtime or declaration
records, builds both targets in the release profile, verifies the emitted `.d`
paths, admits the final snapshot, then proves unchanged replay uses the same
final digest. Editing the actual build script still rejects the snapshot.

Additional cases cover skipped directories containing a file before the build
but absent afterward, unknown external absence, parent traversal, a later
source addition after an admitted absent input, and creation of a previously
recorded absent input. Existing reuse tests cover changed/deleted source,
replaced inventories, output corruption and symlinks.

This is a Canic-owned reproducer and source-verifier qualification. It does not
rerun Toko Miner's eight-artifact build, qualify a staging recovery, or measure
a cold-build speedup. Publication and downstream adoption remain separate.

Focused validation logs are retained locally as
`/tmp/canic-139-absent-input-tests.log` and
`/tmp/canic-139-absent-input-clippy.log`; final outcomes are recorded in the
[current handoff](../../../../status/current.md).
