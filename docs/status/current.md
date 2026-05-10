# Current Status

Last updated: 2026-05-10

## Purpose

This file is the compact handoff for new agent sessions. Read it first, then
inspect only the files needed for the current task.

## Current Line

- Active minor: `0.33.x`
- Theme: hard-cut DFX support in favor of ICP CLI, then reduce 0.33 structural
  hotspots without changing behavior.
- Current release-work area: `0.33.7` Canic metadata list visibility slice.

## Recent Work

- Completed the 0.33 ICP CLI hard cut: `icp.yaml`, `.icp`, ICP CLI install/list/
  medic/snapshot/restore flows, native replica controls, and project status.
- Removed default fleet/network state and the old public `canic network`
  command; fleet-scoped commands take positional fleet names.
- Made the standard pre-1.0 `canic` facade capabilities default so fleet
  canisters no longer choose Canic feature flags manually.
- Trimmed the public metrics surface into role-inferred profiles and tiered
  selectors while keeping metrics enabled by default before 1.0.
- Added `canic endpoints` with Candid method/argument output and changed
  generated Candid finalization to require a trailing `canic::finish!()`.
- Made `canic endpoints` fleet-scoped and moved `--icp <path>` and
  `--network <name>` to top-level-only CLI options; command-local placement is
  hard-rejected instead of kept as a hidden compatibility path.
- Removed low-value list/config selectors: `canic list --root` is gone,
  `canic list --from` is now `canic list --subtree`, and `canic config --from`
  is gone.
- Removed `canic endpoints --did`; endpoint lookup now uses fleet metadata and
  known local role `.did` artifacts only, and registered principals infer their
  fallback role from the fleet registry instead of taking `--role`.
- Removed `KIND` from the live `canic list` table, added `CYCLES` in `0.33.6`,
  and added `CANIC` in `0.33.7`; version and cycle balances now use parallel
  `icp canister call canic_metadata` and `canic_cycle_balance` reads.
- Replaced the separate generated `canic_canister_version` and
  `canic_standards` endpoints with a single `canic_metadata` endpoint that
  includes package metadata, Canic version, and IC canister version.
- Local root installs now target at least `100.00 TC` on root, including
  pre-bootstrap and post-ready top-ups for reused local root canisters.
- Grouped `snapshot`, `backup`, `manifest`, and `restore` under a dedicated
  backup/restore section in the top-level `canic help` output.
- Fixed local `canic snapshot download <fleet>` target discovery to use decoded
  local replica registry queries instead of parsing the ICP CLI transport JSON
  wrapper.
- Fixed real snapshot-download id extraction to use
  `icp canister snapshot create --quiet` and hex-only parsing, preventing table
  units such as `MiB` from being treated as snapshot ids.
- Removed `--resume` from fresh snapshot downloads and documented the 0.34
  backup/restore redesign around root-stays-up subtree backup phases.
- Centralized byte-size and TC cycle formatting through shared format helpers
  so list and config output use the same labels.
- Removed public install overrides: `canic install` is now just
  `canic install <fleet>` with fleet config, root target, and readiness timeout
  owned by Canic.
- Added hard fleet identity checks: duplicate discovered `[fleet].name` values
  fail config discovery, and install requires the config identity to match the
  requested fleet directory.
- Moved the `minimal` shared-runtime baseline under `canisters/audit` and made
  `canic status` compare local deployments against bootstrap-required roles.
- Refreshed the module-structure audit and reduced the current structural risk
  readout to `3/10`.
- Split current 0.33 hotspots in `canic-core` IC management/provisioning,
  `canic-control-plane` publication, and `canic-backup` restore
  runner/apply-journal internals into normal directory modules.
- Ran the oldest outstanding recurring audit, `change-friction`, against the
  current 0.33 line. It reports medium friction risk at `5/10`: the broad
  DFX-to-ICP CLI hard cut raised patch radius, but no cross-layer leakage was
  confirmed.

## Validation Recently Run

- `cargo fmt --all`
- `cargo test -p canic-cli list::tests -- --nocapture`
- `cargo test -p canic-cli snapshot -- --nocapture`
- `cargo test -p canic-host snapshot_id -- --nocapture`
- `cargo test -p canic-host snapshot -- --nocapture`
- `cargo test -p canic-backup discovery -- --nocapture`
- `cargo test -p canic-backup snapshot -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo check -p canic-host`
- `cargo test -p canic-host cycle -- --nocapture`
- `cargo clippy -p canic-host --all-targets -- -D warnings`
- `cargo build -p canic-cli --bin canic`
- `time target/debug/canic list test`
- `target/debug/canic list test`
- `target/debug/canic install demo`
- `target/debug/canic list demo`
- `target/debug/canic snapshot download demo --dry-run`
- `cargo run -q -p canic-cli --bin canic -- endpoints test app`
- `cargo run -q -p canic-cli --bin canic -- endpoints test app --json`
- `cargo check -p canic-core`
- `cargo clippy -p canic-core --all-targets -- -D warnings`
- `cargo test -p canic --test canic_metadata -- --nocapture`
- `cargo check -p canic`
- `cargo clippy -p canic --all-targets -- -D warnings`
- `cargo check -p canic-wasm-store`
- `cargo test -p canic-core --lib -- --nocapture`
- `cargo test -p canic-core --lib workflow::ic -- --nocapture`
- `cargo test -p canic-core --lib ops::ic -- --nocapture`
- `cargo check -p canic-control-plane`
- `cargo clippy -p canic-control-plane --all-targets -- -D warnings`
- `cargo test -p canic-control-plane --lib -- --nocapture`
- `cargo check -p canic-backup`
- `cargo clippy -p canic-backup --all-targets -- -D warnings`
- `cargo test -p canic-backup --lib -- --nocapture`
- `git show --stat --name-only --format=fuller 8a5814fd`
- `git show --stat --name-only --format=fuller cf24f77e`
- `git show --stat --name-only --format=fuller 53476764`
- `git show --stat --name-only --format=fuller 6ea85fdb`
- `git show --stat --name-only --format=fuller 5b474986`

## Known Worktree Notes

- The worktree is intentionally dirty during active slice work.
- Do not revert unrelated edits.
- Agents must not stage, commit, push, bump versions, or run release targets.

## Cost-Control Rules

- Prefer scoped searches over broad repo searches.
- Avoid searching `docs/changelog/**`, `docs/audits/reports/**`, and generated
  outputs unless the task is specifically about those files.
- Write detailed findings to files; summarize only the high-signal result in
  chat.
- Keep final responses concise and include validation commands actually run.

## Good Next Tasks

1. Continue the module-structure cleanup with host install/release helpers or
   backup manifest/snapshot planning, while avoiding active `canic-cli` edits.
2. Keep `canic-cli`, `canic-host`, and `canic-backup` boundaries sharp: CLI owns
   UX, host owns ICP CLI/filesystem/build/install mechanics, backup owns
   backup/restore domain logic.
3. Keep new modules on normal Rust directory discovery; do not add `#[path]`.
4. Update `CHANGELOG.md`, `docs/changelog/0.33.md`, and
   `docs/status/0.33-refactor.md` for each cleanup slice.
