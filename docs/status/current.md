# Current Status

Last updated: 2026-05-15

## Purpose

This file is the compact handoff for new agent sessions. Read it first, then
inspect only the files needed for the current task.

## Current Line

- Active minor: `0.36.x` prep
- Theme: prove and harden the existing backup/restore execution code into an
  operator-working flow with durable journals, resume/retry behavior, and
  verification gates.
- Current release-work area: first pushable `0.36.0` backup/restore proof
  slice.

## Recent Work

- Moved the backup/restore design track forward to
  `docs/design/0.35-backup-restore/0.35-design.md` and marked the old 0.34
  draft as superseded.
- Added the 0.35.2 controller-policy follow-up: root init and post-upgrade now
  retain the installing or upgrading root controller in the runtime controller
  set used for newly allocated managed children.
- Added the 0.35.3 changelog entry covering local replica port visibility,
  `canic replica start --port <port>`, configured-port local queries, ownership
  diagnostics, `canic fleet sync`, automatic `icp.yaml` sync after
  `canic fleet create <name>`, explicit `topup = {}` default top-up config
  blocks, and the default top-up amount change from `4T` to `5T`.
- Started the 0.35.4 endpoint cleanup by removing stale root wasm-store
  bootstrap upload endpoints, controller-gating root state/app-registry/log
  diagnostics, simplifying `canic_canister_status` to controller-only access,
  and updating wasm-store reconcile coverage to current managed release roles.
- Collapsed the root wasm-store endpoint surface by removing the duplicate
  publish-to-current shortcut plus split publication/retired status endpoints;
  current publication uses `canic_wasm_store_admin` and controller reads use
  `canic_wasm_store_overview`.
- Ran the 2026-05-13 recurring `instruction-footprint` performance audit as
  the first `0.35` baseline. It reports risk `3 / 10`; root delegation is the
  highest sampled endpoint at `800834` average local instructions, and the
  first-run baseline deltas are intentionally `N/A`.
- Reran the 2026-05-13 recurring `audience-target-binding` invariant audit. It
  reports risk `3 / 10` and confirms role-attestation, delegated-token,
  delegated-grant, and capability-proof audience/target binding still fails
  closed.
- Reran the 2026-05-14 oldest latest-run recurring audit,
  `token-trust-chain`, at
  `docs/audits/reports/2026-05/2026-05-14/token-trust-chain.md`. It reports
  risk `4 / 10`, finds no trust-chain correctness break, and leaves only
  structural watchpoints around `dto::auth` fan-in plus runtime verifier/guard
  edit pressure.
- Reran the next oldest latest-run recurring audit,
  `auth-abstraction-equivalence`, at
  `docs/audits/reports/2026-05/2026-05-14/auth-abstraction-equivalence.md`.
  It reports risk `3 / 10`, finds no abstraction bypass, and the recurring
  definition now uses current `crates/canic-macros` paths, targeted auth scans,
  and the auth trust-chain guard as required evidence.
- Promoted the repeated ad hoc `dry-consolidation` audit into the recurring
  system suite and reran it at
  `docs/audits/reports/2026-05/2026-05-14/dry-consolidation.md`. It reports
  risk `4 / 10`, down from May 12 after installed-fleet resolution, registry
  parsing, response parsing primitives, and major CLI command modules gained
  clearer owners.
- Applied a small dry-consolidation follow-up: `snapshot download` now uses the
  host installed-fleet resolver/cache for installed fleets, and `medic` reads
  installed-fleet state through the host installed-fleet boundary.
- Added the proposed 0.36 backup/restore v1 design at
  `docs/design/0.36-backup-restore/0.36-design.md`. The 0.36 release focus is
  proving and hardening the existing backup/restore execution code into a full
  operator-working backup and in-place restore flow with durable journals,
  receipts, resume/retry behavior, and status/verify gates.
- Started the first pushable 0.36.0 proof slice by adding backup runner tests
  for max-step resume without replaying completed/preflight work and failed
  snapshot retry from the recorded failed operation.
- Kept backup resume proof at the runner/test layer instead of exposing a public
  manual pause flag for `canic backup create`; 0.36 should start with the
  smallest operator surface that works.
- Added backup status coverage for execution layouts so durable
  plan/journal/manifest state reports `running`, `failed`, and `complete`
  without introducing new operator flags.
- Tightened `canic backup status --require-complete` to require the complete
  execution layout, including the finalized manifest, instead of accepting a
  completed execution journal by itself.
- Tightened `canic backup verify` for execution-backed backups so manifest and
  artifact verification also requires the persisted backup plan and execution
  journal to match and be complete.
- Changed backup create persistence to preserve an existing output layout and
  its progressed execution journal, so the CLI wrapper now honors the same
  resume boundary that the backup runner already supported.
- Changed `canic backup list` to surface execution-backed manifest state
  (`running`, `complete`, `failed`, or invalid plan/journal) instead of
  reporting all manifest-bearing layouts as `ok`.
- Started `0.36.1` by tightening `canic backup create --out <dir>` resume
  safety: existing layouts are preserved only when the stored plan matches the
  requested fleet, network, root, scope, target set, and operation graph.
- Extended backup create resume compatibility to authority and quiescence
  policy fields so dry-run layouts are not reused as executable backup layouts.
- Added a `LAYOUT` column to `canic backup create` output so fresh and resumed
  output layouts are visible to operators.
- Tightened `canic backup list` so manifest-plus-plan layouts with no execution
  journal report `invalid-plan-journal`, not `dry-run`.
- Tightened `canic backup create --out <dir>` so manifest-backed layouts with a
  missing execution journal are treated as incomplete instead of having a new
  journal synthesized.
- Tightened backup status, inspect, and verify so manifest-backed layouts with
  missing execution journals use the same incomplete-layout error instead of
  falling through to raw file-read failures.
- Tightened backup execution integrity so terminal mutating operations require
  matching operation receipts; preflight-completed validation operations remain
  receiptless as intended.
- Started `0.36.3` restore-runner hardening by making upload-snapshot commands
  fail if successful output does not include the uploaded snapshot id required
  by later load-snapshot operations.
- Added explicit `canic restore run --retry-failed` recovery so failed restore
  operations can be moved back to ready after inspection without hand-editing
  the apply journal.
- Tightened legacy restore upload-id parsing so only uploaded-snapshot-labelled
  text can satisfy a successful upload command without structured JSON.
- Tightened restore-runner journal loading so completed or failed operations
  must have matching command receipts before any runner mode proceeds.
- Started `0.36.4` by rejecting duplicate restore operation receipt attempts
  and adding an active-line changelog width check for root and detailed notes.
- Started `0.36.5` by requiring backup execution operation receipts to carry
  `updated_at` so terminal outcomes stay auditable in persisted journals.
- Tightened backup execution receipt recording so invalid receipts roll back
  the attempted operation transition instead of leaving partial in-memory
  state.
- Adjusted the changelog check so root `CHANGELOG.md` patch bullets stay on
  one line while detailed changelog notes keep the 88-column prose wrap.
- Started `0.36.6` by making backup execution integrity compare terminal
  mutating operation state with the latest matching receipt, so stale retry
  history cannot hide a hand-edited journal state mismatch.
- Folded persisted backup execution `restart_required` validation into the
  `0.36.6` slice so edited journals cannot hide a required restart window.
- Tightened `0.36.6` further by requiring backup execution transition
  timestamps before mutation and rejecting persisted pending or terminal
  operation states without `state_updated_at`.
- Added `0.36.6` persistence integrity coverage that rejects terminal backup
  operation timestamp drift from the latest durable operation receipt.
- Started `0.36.7` by requiring restore apply-journal command receipts to keep
  their update timestamp, command preview, exit status, and bounded
  stdout/stderr audit payloads.
- Folded stale local-replica status handling into `0.36.7`: ICP CLI local
  status is now treated as stale unless the configured gateway port is
  actually reachable, so `canic replica start` no longer reports a dead
  configured port as already running.
- Started `0.36.8` by tightening restore-runner journal loading so terminal
  restore operations must be backed by the latest matching command receipt
  attempt with the same state timestamp.
- Folded a `canic list --subtree` role-anchor fix into `0.36.8`: unique role
  names now resolve to their canister principal, while repeated roles require a
  concrete principal.
- Extended the same role-or-principal subtree selector to
  `canic cycles --subtree`, filtering the registry before cycle history,
  balance, and top-up queries run.
- Added a config-schema regression proving obsolete per-canister delegated-auth
  verifier tables are rejected instead of accepted through compatibility shims.
- Updated the internal audit scaling probe to use `scale_replica` and
  `policy.initial_workers = 0` so the dry-run planning probe no longer tries
  to allocate startup workers in a standalone PocketIC scenario.
- Renamed the test fleet scaling worker role from `scale` to `scale_replica`,
  changed role cycle config from `topup_policy` to `topup`, and enabled explicit
  default `topup = {}` policy blocks for the main test app, hub, shard, and
  scaling roles.
- Slimmed the ICP build hook path: `icp.yaml` now invokes
  `cargo run -q -p canic-host --example build_artifact -- <role>` directly,
  the Rust builder owns `ICP_WASM_OUTPUT_PATH` copying, and the old
  `scripts/app/build.sh` wrapper has been removed.
- Tightened local replica ownership checks so `canic replica start --background`
  and `canic status` use project-scoped ICP network status instead of broad
  local ping, while `canic replica stop` distinguishes "this project is already
  stopped" from "port 8000 is owned by a different ICP network/project".
- Added configured local gateway port output to `canic status` and
  `canic replica status`, plus `canic replica start --port <port>` to update
  this project's `icp.yaml` `gateway.port` before starting.
- Hard-cut the managed child controller policy for 0.35.1: newly allocated
  non-root canisters now receive configured controllers, root, and their direct
  parent as controllers; pool reuse updates the controller set before install.
- Tightened `canic install <fleet>` build output by hiding unset requested
  profile noise, using operator labels for build context, omitting duplicate
  ICP root context, adding `WASM_GZ` sizes to the build table, and making
  local root top-up output show the checkpoint phase, exact amount, and target.
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
- Local root installs keep a `100.00 TC` root ready target, including
  pre-bootstrap and post-ready top-up checkpoints for reused local root
  canisters.
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
- `bash -n scripts/ci/build-ci-wasm-artifacts.sh scripts/ci/wasm-audit-report.sh`
- `cargo check -p canic-host --examples`
- `cargo check -p canic-host --examples -p canic-tests`
- `cargo test -p canic-host canister_build -- --nocapture`
- `cargo clippy -p canic-host --examples -- -D warnings`
- `cargo check -p canic-core -p canic-host -p canic-testing-internal -p canister_scale`
- `cargo test -p canic build_support -- --nocapture`
- `cargo test -p canic-core config::schema -- --nocapture`
- `cargo test -p canic-core config::schema::subnet -- --nocapture`
- `cargo test -p canic-host release_set -- --nocapture`
- `cargo test -p canic-host install_root::tests::config_selection -- --nocapture`
- `cargo test -p canic-cli list::tests -- --nocapture`
- `cargo clippy -p canic -p canic-core -p canic-host -p canic-testing-internal --all-targets -- -D warnings`
- `git diff --check`
- `cargo test -p canic-core workflow::ic::provision::allocation -- --nocapture`
- `cargo check -p canic-core`
- `cargo clippy -p canic-core --all-targets -- -D warnings`
- `cargo check -p canic-host`
- `cargo test -p canic-host install_root -- --nocapture`
- `cargo clippy -p canic-host --all-targets -- -D warnings`
- `git diff --check`
- `cargo test -p canic-backup restore -- --nocapture`
- `cargo test -p canic-cli restore -- --nocapture`
- `cargo check -p canic-backup -p canic-cli`
- `cargo clippy -p canic-backup -p canic-cli --all-targets -- -D warnings`
- `cargo test -p canic-cli list::tests -- --nocapture`
- `cargo test -p canic-cli snapshot -- --nocapture`
- `cargo test -p canic-cli replica -- --nocapture`
- `cargo test -p canic-cli status -- --nocapture`
- `cargo test -p canic-host icp -- --nocapture`
- `cargo test -p canic-host icp_config -- --nocapture`
- `cargo test -p canic-host replica_query -- --nocapture`
- `cargo clippy -p canic-cli -p canic-host --all-targets -- -D warnings`
- `cargo run -p canic-cli -- status`
- `cargo run -p canic-cli -- replica status`
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
- `git show --stat --name-only --format=fuller 09f5d238`
- `cargo test -p canic-cli list:: -- --nocapture`
- `cargo check -p canic-cli`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo test -p canic-host install_root::tests -- --nocapture`
- `cargo check -p canic-host`
- `cargo clippy -p canic-host --all-targets -- -D warnings`
- `bash scripts/ci/instruction-audit-report.sh`
- `cargo test -p canic-core --lib verify_root_delegated_grant_claims_rejects_audience_mismatch -- --nocapture`
- `cargo test -p canic-core --lib verify_delegated_token_rejects_audience_subset_drift -- --nocapture`
- `cargo test -p canic-core --lib verify_delegated_token_rejects_missing_local_role_for_role_audience -- --nocapture`
- `cargo test -p canic-core --lib mint_delegated_token_rejects_audience_expansion -- --nocapture`
- `cargo test -p canic-core config::schema::subnet::tests::canister_config_rejects_legacy_delegated_auth_table -- --nocapture`
- `cargo test -p canic-core config::schema -- --nocapture`
- `cargo check -p canic-control-plane -p canic -p canic-tests --tests`
- `cargo test -p canic-control-plane publication -- --nocapture`
- `cargo test -p canic-tests --test root_wasm_store_reconcile -- --test-threads=1 --nocapture`
- `cargo test -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture`
- `cargo test -p canic-tests --test pic_role_attestation capability_endpoint_role_attestation_proof_paths -- --test-threads=1 --nocapture`
- `cargo fmt --all --check`
- `cargo check -p canic-tests --tests`
- `git diff --check`

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
