# CLI / Host / Backup Boundary Audit - 2026-05-12

## Report Preamble

- Scope: `crates/canic-cli`, `crates/canic-host`, and `crates/canic-backup`,
  with emphasis on backup/snapshot/status installed-fleet and registry flows.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-12/dry-consolidation.md`.
- Code snapshot identifier: `264de543` plus dirty 0.34.6 worktree.
- Method tag/version: `DRY Consolidation V2 / operator-boundary focus`.
- Auditor: Codex.
- Run timestamp: 2026-05-12 Europe/Paris.

## Boundary Rule Used

- `canic-cli` owns command UX: args, command dispatch, operator-facing report
  shapes, labels, and error wording.
- `canic-host` owns host/operator mechanics: ICP CLI process calls, local
  replica query preference, filesystem install state, workspace discovery,
  build/install helpers, and table/format helpers.
- `canic-backup` owns backup/restore domain contracts: plans, manifests,
  journals, persistence layouts, execution state machines, integrity checks,
  and pure backup/restore validation.

## Executive Summary

The post-split `canic-cli::backup` structure is much healthier: the command
root is now dispatch/error/facade code, and backup listing/status/inspect/verify
are separated. The remaining risk is not broad duplication; it is concentrated
at the CLI/host/backup boundary.

Initial focused boundary risk: **6 / 10**.

After the follow-up remediation in this slice, current focused boundary risk is
**4 / 10**. The main structural issue found by this audit was fixed:
`canic-backup` no longer depends on `canic-host`. The remaining pressure is in
thin CLI adapters and status/list classification polish.

## Positive Readout

- `canic-cli::backup` is now split into `create`, `reference`, `status`,
  `inspect`, `verify`, `labels`, `model`, `render`, `options`, and `command`.
- `canic-host::installed_fleet` already centralizes the installed fleet state +
  registry query path for list/cycles/metrics/endpoints.
- `canic-host::icp::IcpCli` owns raw ICP CLI subprocess construction and
  snapshot command primitives.
- `canic-backup::runner` and `canic-backup::snapshot` use traits for live
  side effects, which is the right shape: backup owns sequencing and state,
  while CLI/host adapters perform transport.

## Findings

### Medium-High - `canic-backup` depends on `canic-host` for registry contracts

Evidence:

- `crates/canic-backup/Cargo.toml` depends on `canic-host`.
- `crates/canic-backup/src/plan/build.rs:11` imports
  `canic_host::registry::RegistryEntry`.
- `crates/canic-backup/src/discovery/mod.rs:5` imports
  `canic_host::registry::RegistryEntry`.
- `crates/canic-backup/src/snapshot/mod.rs:19` imports
  `canic_host::registry::parse_registry_entries`.
- `crates/canic-backup/src/snapshot/types.rs:5` exposes
  `canic_host::registry::RegistryParseError` through backup errors.

Why this is a boundary problem:

- `canic-backup` is the domain crate. It should not require host/operator
  plumbing to build plans or validate backup layouts.
- The current dependency means a host registry parser change can affect backup
  public/domain contracts.
- `RegistryParseError` leaking through `SnapshotDownloadError` makes the backup
  error surface partly host-owned.

Recommended fix:

- Remove `canic-host` from `canic-backup` dependencies.
- Add a backup-owned neutral input shape, for example
  `canic_backup::topology::RegistryTopologyEntry` or
  `canic_backup::discovery::RegistryMemberInput`.
- Keep `canic-host::registry::parse_registry_entries` host-owned if it is
  parsing live ICP/root transport output, then convert host entries into backup
  input records at the CLI/host boundary.
- Alternatively, if the registry JSON contract is considered a stable Canic
  domain contract rather than host transport, move the DTO/parser down into
  `canic-backup` and make host import it. Do not keep backup importing host.

Follow-up status:

- Completed in the same cleanup slice. `canic-backup` now owns a neutral
  registry input type, host-owned registry parsing remains in `canic-host`, and
  CLI backup/snapshot adapters convert host entries at the boundary.

### Medium - `backup create` reimplements installed-fleet resolution already owned by host

Evidence:

- `crates/canic-cli/src/backup/create.rs:35-43` reads install state, queries
  `canic_subnet_registry`, parses registry entries, and hashes topology.
- `crates/canic-cli/src/backup/create.rs:308-320` repeats the same local replica
  vs ICP CLI registry query branch already centralized in
  `canic-host/src/installed_fleet.rs:99-168`.

Impact:

- Local replica preference, stale local root handling, and registry error
  behavior can drift from list/cycles/metrics/endpoints.
- The recent stale-status issue is exactly the kind of behavior that should
  have one host-owned implementation.

Recommended fix:

- Route `backup create` through `canic_host::installed_fleet::resolve_installed_fleet`.
- Keep backup-specific planning decisions in CLI/domain code:
  selected subtree, dry-run authority defaults, backup output path, and report
  labels.
- If backup needs a topology hash, expose a host or backup-owned conversion
  helper so CLI does not hand-roll `RegistryEntry -> TopologyRecord`.

Follow-up status:

- Completed in the same cleanup slice. `backup create` now uses
  `canic_host::installed_fleet::resolve_installed_fleet` for initial
  install-state and registry resolution.

### Medium - `status` still has bespoke stale/partial installed-fleet logic

Evidence:

- `crates/canic-cli/src/status/mod.rs:172` calls
  `read_named_fleet_install_state` directly.
- `crates/canic-cli/src/status/mod.rs:212-219` queries and parses the local
  registry directly.
- `crates/canic-cli/src/status/mod.rs:241-243` duplicates host's local
  canister-not-found classifier from
  `crates/canic-host/src/installed_fleet.rs:170-187`.

Impact:

- `status` can classify `stale`, `partial`, `yes`, or `error` differently from
  other installed-fleet commands.
- This is the highest-risk command for operator confusion because it is the
  command users run after replica restarts.

Recommended fix:

- Move the local deployment classifier into `canic-host`, probably beside
  `InstalledFleetResolution`.
- Make `status` call host for each configured fleet and only render host's
  deployed classification.
- Keep config discovery and table rendering in CLI.

Follow-up status:

- Partially completed in the same cleanup slice. `status` now uses the
  host-owned installed-fleet resolver for live local root checks and reports
  missing local roots as `lost`, not `stale`. The small role-completeness
  classifier is still CLI-local.

### Medium - snapshot download and backup create duplicate live snapshot transport adapters

Evidence:

- `crates/canic-cli/src/snapshot/download/mod.rs:261-316` defines
  `IcpSnapshotDriver` and delegates registry/stop/start/create/download/display
  calls.
- `crates/canic-cli/src/snapshot/download/mod.rs:338-388` wraps the same
  `IcpCli` registry and snapshot methods used by backup.
- `crates/canic-cli/src/backup/create.rs:168-306` defines
  `BackupIcpRunnerExecutor` and maps the same live stop/start/create/download
  operations into backup runner errors.

Impact:

- The raw process execution is centralized in `IcpCli`, so this is not a
  correctness breach today.
- The adapter layer is still duplicated enough that future snapshot command
  behavior can drift between legacy `snapshot download` and the newer backup
  runner.

Recommended fix:

- Keep `canic-backup` traits as-is; backup should not know ICP CLI.
- Add host-owned neutral helpers for live snapshot operations and command
  display if the wrappers grow further.
- CLI can keep thin adapter structs that implement `canic-backup` traits by
  delegating to host primitives.

### Medium-Low - backup layout status/listing may want a backup-owned classifier

Evidence:

- `crates/canic-cli/src/backup/reference.rs` detects manifest vs plan-only
  layouts and emits `invalid-manifest`, `invalid-plan`,
  `invalid-plan-journal`, `dry-run`, `running`, `failed`, or `complete`.
- `crates/canic-cli/src/backup/status.rs` performs similar plan/journal layout
  branching for status output.

Impact:

- This is CLI-facing today, so keeping labels in CLI is fine.
- The underlying classification of a persisted backup layout is domain logic.
  If restore or other commands need the same layout state, duplication will
  return.

Recommended fix:

- Do not move labels into `canic-backup`.
- Consider a backup-owned `BackupLayoutState` enum if another command needs
  the same manifest/plan/journal classification.

## Recommended Next Split

1. Break the `canic-backup -> canic-host` dependency.
   - Add backup-owned registry/topology input records or move the registry
     contract fully into backup.
   - Update backup plan/discovery/snapshot code to use the backup-owned type.
   - Keep live JSON parsing/conversion at host/CLI boundary.

2. Route `backup create` through `canic_host::installed_fleet`.
   - This removes duplicated registry query logic from CLI backup.
   - It also aligns backup planning with list/cycles/metrics/endpoints.

3. Route `status` stale/partial checks through host.
   - This is the user-visible correctness follow-up for the replica restart
     confusion.

4. Revisit snapshot/backup live adapters only after the first two changes.
   - The current duplication is tolerable because `IcpCli` owns the raw command
     behavior.

## Validation / Evidence Commands

- `rg -n "canic-host|canic-backup" crates/canic-backup/Cargo.toml crates/canic-host/Cargo.toml crates/canic-cli/Cargo.toml`
- `rg -n "read_named_fleet_install_state|query_subnet_registry_json|canister_call_output\\(.*canic_subnet_registry|parse_registry_entries|is_canister_not_found_error" crates/canic-cli/src crates/canic-host/src crates/canic-backup/src -g '*.rs'`
- `find crates/canic-cli/src -maxdepth 3 -type f -name '*.rs' -print0 | xargs -0 wc -l | sort -nr | head -40`
- `find crates/canic-host/src crates/canic-backup/src -maxdepth 3 -type f -name '*.rs' -print0 | xargs -0 wc -l | sort -nr | head -60`
- `cargo test -p canic-cli backup -- --nocapture`
- `cargo check -p canic-cli`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
