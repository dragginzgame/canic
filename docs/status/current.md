# Current Status

Last updated: 2026-05-12

## Purpose

This file is the compact handoff for new agent sessions. Read it first, then
inspect only the files needed for the current task.

## Current Line

- Active minor: `0.35.x`
- Theme: get backup/restore working end-to-end around topology-aware plans,
  executable journals, and clear host/backup boundaries.
- Current release-work area: `0.35.0` backup/restore execution path.

## Recent Work

- Moved the backup/restore design track forward to
  `docs/design/0.35-backup-restore/0.35-design.md` and marked the old 0.34
  draft as superseded.
- Hard-cut the managed child controller policy for 0.35.1: newly allocated
  non-root canisters now receive configured controllers, root, and their direct
  parent as controllers; pool reuse updates the controller set before install.
- Added explicit restore-run stop/start phases so apply journals now schedule
  snapshot upload, target stop, snapshot load, target start, and verification
  operations instead of depending on manual canister state changes.
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
  confirmed. The rerun after reloading ICP CLI used `icp 0.2.6`, clean snapshot
  `09f5d238`, and included the committed `0.33.7` metadata/list slice.
- Started remediating the change-friction follow-up by splitting `canic list`
  live registry projection into `crates/canic-cli/src/list/live.rs`, reducing
  the command root from the audited `902` lines to `506` lines.
- Deduplicated `canic list` table width/separator/alignment rendering through
  `crates/canic-cli/src/list/table.rs` for both config and registry tables.
- Deduplicated the live-list threaded query collector used by local readiness,
  `canic_metadata` version reads, and `canic_cycle_balance` reads.
- Centralized list config-loader host-config error mapping so adding config
  table columns does not repeat install-state conversion boilerplate.
- Split list endpoint response parsing into `crates/canic-cli/src/list/parse.rs`
  so metadata and cycle-balance response-shape tests live beside the parsers
  rather than the live transport code.
- Promoted table rendering to `canic-host::table` and routed list, status,
  fleet-list, backup-list, medic, and install config-choice tables through one
  host/operator header/underline/spacing/alignment helper.
- Split deployed-registry tree traversal into `crates/canic-cli/src/list/tree.rs`
  so `list/render.rs` no longer owns hierarchy selection and presentation at
  the same time.
- Split host root readiness polling and diagnostics into
  `crates/canic-host/src/install_root/readiness.rs`, reducing
  `install_root/mod.rs` from `901` to `586` lines while preserving the install
  orchestration flow.
- Started the 0.34 backup/restore rework by adding `canic-backup::plan` with
  typed backup plans, targets, operations, authority/read preflights,
  quiescence policy, and operation receipts. This is a model-only slice; live
  snapshot execution is unchanged.
- Split backup plan validation from execution readiness: plans can represent
  `Proven`, `Declared`, or `Unknown` control/read authority for dry-run output,
  while mutating backup execution requires proven authority for every selected
  target.
- Added target-scoped control and snapshot-read authority preflight receipts so
  future execution can upgrade a plan only after proof covers every selected
  target.
- Added typed authority preflight request DTOs derived from `BackupPlan`, giving
  root coordination and host-side authority adapters a stable input contract.
- Added typed topology and quiescence preflight request/receipt DTOs plus
  execution-gate validation for topology drift, target-set changes, policy
  mismatches, and rejected quiescence.
- Added a full execution preflight receipt bundle so future backup execution can
  apply authority receipts and validate topology/quiescence gates through one
  typed boundary.
- Added `preflight_id`, `validated_at`, and `expires_at` to preflight receipts
  and the execution preflight bundle so stale or cross-preflight evidence cannot
  authorize later mutation.
- Added `canic-backup::execution` with a model-only backup execution journal
  built from `BackupPlan` phases, including preflight acceptance, ordered
  operation transitions, durable operation receipts, retryable failures, resume
  summaries, and `restart_required` tracking after stops.
- Added typed preflight receipt-bundle acceptance to the execution journal so
  mutation cannot be unblocked by a bundle from a different plan.
- Added `BackupLayout` read/write support for
  `backup-execution-journal.json`, keeping phase execution progress separate
  from the existing artifact download journal.
- Added `BackupLayout` read/write support for `backup-plan.json` so future
  backup runners can resume against the exact validated plan instead of
  reconstructing the operation graph.
- Added execution-layout integrity verification that rejects a persisted
  execution journal when its plan/run ids or operation graph no longer match
  the stored `backup-plan.json`.
- Added the first `canic backup create <fleet> --dry-run` CLI path, including
  optional `--subtree <role-or-principal>` planning, installed-fleet registry
  discovery, persisted `backup-plan.json`, persisted
  `backup-execution-journal.json`, and a compact dry-run summary table while
  keeping real mutation disabled.
- Made `canic backup list` include plan-only dry-run directories as
  `STATUS=dry-run`, using the persisted plan id as `BACKUP_ID` and planned
  target count as `MEMBERS`.
- Made `canic backup status --dir <dry-run-dir>` understand dry-run
  `backup-plan.json` plus `backup-execution-journal.json` layouts and report
  execution-journal progress while `--require-complete` still rejects them as
  non-backups.
- Added `canic backup inspect --dir <dry-run-dir>` with table and JSON output
  for plan metadata, selected targets, authority evidence, operation order, and
  execution-journal state.
- Added a `#` column to `canic backup list` so operators can refer to visible
  backup rows by a short ordinal as well as by `BACKUP_ID`.
- Made `canic backup inspect`, `canic backup status`, and
  `canic backup verify` accept either the `canic backup list` row number or
  `BACKUP_ID` as a positional backup reference, with `--dir <dir>` kept for
  explicit paths and ambiguous backup ids rejected fail-closed.
- Made `canic backup verify` reject dry-run plan layouts with the typed
  `DryRunNotComplete` error instead of falling through to a missing-manifest
  filesystem error.
- Added registry-backed backup plan construction for explicit subtrees and
  non-root fleet scopes, including top-down stop/snapshot phases, bottom-up
  start phases, and post-restart download/verify/finalize phases.
- Added backup selector resolution for explicit principals and unambiguous
  roles, rejecting missing or ambiguous role selectors before planning.
- Reran the oldest latest-run lightweight recurring audit, `publish-surface`,
  at `docs/audits/reports/2026-05/2026-05-11/publish-surface.md`. It reports
  package-surface risk `3/10`: all 11 publishable crates package and verify.
- Completed the publish-surface follow-up by aligning `crates/canic/README.md`
  with the default facade features and refreshing the recurring audit's
  canonical published crate map.
- Ran the full-codebase DRY consolidation audit for 2026-05-12. It reports
  medium consolidation risk at `5/10`, with installed-fleet resolution and
  large CLI command modules as the highest-value follow-ups.
- Added `canic-host::installed_fleet` with `InstalledFleetResolution`,
  `InstalledFleetSource`, `InstalledFleetRegistry`, and
  `ResolvedFleetTopology`, then routed `canic list`, `canic cycles`,
  `canic metrics`, and `canic endpoints` through the shared installed-fleet
  resolver.
- Split `canic endpoints` into command orchestration, endpoint model, Candid
  parsing, transport, and rendering modules while keeping behavior unchanged.
- Split `canic cycles` into command orchestration, options, response parsing,
  transport/report collection, rendering, and model modules while keeping
  behavior unchanged.
- Split `canic metrics` into command orchestration, options, response parsing,
  transport/report collection, rendering, and model modules while keeping
  behavior unchanged.
- Split top-level CLI command catalog/help rendering and global option
  forwarding out of `canic-cli::lib`, leaving the root focused on command
  dispatch and error mapping.
- Moved shared ICP response parsing primitives from `canic-cli` to
  `canic-host::response_parse`, and switched CLI list/cycles/metrics parsers to
  import the host-owned helpers directly.
- Moved the live subnet registry DTO/parser from `canic-backup::discovery` to
  `canic-host::registry`.
- Promoted the shared installed-fleet resolver to
  `canic-host::installed_fleet`; CLI list/cycles/metrics/endpoints now consume
  host-owned install-state lookup, local replica preference, ICP CLI fallback,
  registry parsing, and topology projection.
- Split the old `canic-cli::args` module into the `canic-cli::cli` directory
  with `clap`, `defaults`, `help`, and `globals` modules, removing the broad
  argument-helper drawer while preserving command behavior.
- Moved `path_stamp` and `registry_tree` under `canic-cli::support` to keep the
  `canic-cli` crate root focused on command families and explicit support
  modules.
- Split `canic-cli::backup` command-family help and report rendering into
  `backup::command` and `backup::render`; `backup::mod` is down to about
  `1050` lines.

## Validation Recently Run

- `cargo fmt --all`
- `cargo test -p canic-core workflow::ic::provision::allocation -- --nocapture`
- `cargo check -p canic-core`
- `cargo clippy -p canic-core --all-targets -- -D warnings`
- `cargo test -p canic-backup restore -- --nocapture`
- `cargo test -p canic-cli restore -- --nocapture`
- `cargo check -p canic-backup -p canic-cli`
- `cargo clippy -p canic-backup -p canic-cli --all-targets -- -D warnings`
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
- `cargo test -p canic-backup plan -- --nocapture`
- `cargo test -p canic-backup execution -- --nocapture`
- `cargo test -p canic-backup persistence -- --nocapture`
- `cargo test -p canic-cli backup -- --nocapture`
- `cargo test -p canic-cli command_family_help_returns_ok -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo run -q -p canic-cli --bin canic -- backup create demo --dry-run --out /tmp/canic-backup-plan-demo`
- `cargo run -q -p canic-cli --bin canic -- backup create demo --subtree app --dry-run --out /tmp/canic-backup-plan-demo-app`
- `cargo run -q -p canic-cli --bin canic -- backup list`
- `cargo package -p canic -p canic-backup -p canic-cdk -p canic-cli -p canic-control-plane -p canic-core -p canic-host -p canic-macros -p canic-memory -p canic-testkit -p canic-wasm-store --locked --allow-dirty`
- `cargo metadata --no-deps --format-version 1`
- `cargo run -q -p canic-cli --bin canic -- backup status --dir backups/fleet-demo-20260510-222116`
- `cargo test -p canic-cli endpoints -- --nocapture`
- `cargo test -p canic-cli cycles::tests -- --nocapture`
- `cargo test -p canic-cli metrics::tests -- --nocapture`
- `cargo test -p canic-cli usage_lists_command_families -- --nocapture`
- `cargo test -p canic-cli command_family_help_returns_ok -- --nocapture`
- `cargo test -p canic-cli version_flags_return_ok -- --nocapture`
- `cargo test -p canic-cli global_ -- --nocapture`
- `cargo test -p canic-host install_root -- --nocapture`
- `cargo test -p canic-cli list::parse -- --nocapture`
- `cargo clippy -p canic-host -p canic-cli --all-targets -- -D warnings`
- `cargo test -p canic-cli installed_fleet -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo check -p canic-host -p canic-backup -p canic-cli`
- `cargo test -p canic-host registry -- --nocapture`
- `cargo test -p canic-host installed_fleet -- --nocapture`
- `cargo test -p canic-backup --lib -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-host -p canic-backup -p canic-cli --all-targets -- -D warnings`
- `cargo check -p canic-cli`
- `cargo test -p canic-cli command_family_help_returns_ok -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `git diff --check`
- `cargo test -p canic-cli backup -- --nocapture`
- `cargo run -q -p canic-cli --bin canic -- backup inspect --dir backups/fleet-demo-20260510-222116`
- `cargo run -q -p canic-cli --bin canic -- backup inspect --dir backups/fleet-demo-20260510-222116 --json`
- `cargo run -q -p canic-cli --bin canic -- backup list`
- `cargo run -q -p canic-cli --bin canic -- backup inspect 1`
- `cargo run -q -p canic-cli --bin canic -- backup status 1`
- `cargo run -q -p canic-cli --bin canic -- backup verify 1`
- `cargo run -q -p canic-cli --bin canic -- backup inspect plan-demo-20260510-222116 --json`
- `cargo run -q -p canic-cli --bin canic -- backup status plan-demo-20260510-222116`
- `git show --stat --name-only --format=fuller 8a5814fd`
- `git show --stat --name-only --format=fuller cf24f77e`
- `git show --stat --name-only --format=fuller 53476764`
- `git show --stat --name-only --format=fuller 6ea85fdb`
- `git show --stat --name-only --format=fuller 5b474986`
- `icp --version`
- `icp project show`
- `git show --stat --name-only --format=fuller 09f5d238`
- `cargo test -p canic-cli list:: -- --nocapture`
- `cargo check -p canic-cli`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo test -p canic-host install_root::tests -- --nocapture`
- `cargo check -p canic-host`
- `cargo clippy -p canic-host --all-targets -- -D warnings`

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

1. Continue the module-structure cleanup with host install/release helpers,
   backup manifest/snapshot planning, or the remaining direct registry-loading
   callers in `snapshot download`, `backup`, and `status`.
2. Keep `canic-cli`, `canic-host`, and `canic-backup` boundaries sharp: CLI owns
   UX, host owns ICP CLI/filesystem/build/install mechanics, backup owns
   backup/restore domain logic.
3. Keep new modules on normal Rust directory discovery; do not add `#[path]`.
4. Update `CHANGELOG.md`, `docs/changelog/0.34.md`, and this status file for
   each cleanup slice.
