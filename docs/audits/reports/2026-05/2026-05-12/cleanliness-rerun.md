# Cleanliness Rerun - 2026-05-12

## Report Preamble

- Scope: `crates/canic-cli`, `crates/canic-host`, and `crates/canic-backup`.
- Focus: module responsibility, DRY pressure, and CLI/host/backup boundary
  ownership after the latest cleanup slice.
- Baseline context: `cli-host-backup-boundary.md` from the same date.
- Code snapshot identifier: `264de543` plus dirty 0.34.6 worktree.
- Method tag/version: `Cleanliness Rerun V1`.
- Auditor: Codex.
- Run timestamp: 2026-05-12 Europe/Paris.

## Executive Summary

Current cleanliness risk: **4 / 10**.

The major boundary issue from the earlier audit is fixed: `canic-backup` no
longer depends on `canic-host`, and backup now owns its neutral registry input
shape. The CLI backup command root has also been split into command-sized
modules, so command dispatch, reference resolution, dry-run creation,
inspection, status, verification, labels, and shared models are no longer
packed into one broad file.

The code is not yet at the `3 / 10` structural risk level because several
backup domain modules are still large and some CLI live adapters still own
direct installed-state or registry-query plumbing. Those remaining issues are
localized; they do not currently show broad ownership collapse.

## Positive Readout

- `canic-backup` has no `canic-host` dependency in `cargo tree -p
  canic-backup`.
- `canic-backup` owns `registry::RegistryEntry`; host-owned live registry JSON
  parsing is converted at the CLI/host adapter boundary.
- `canic-host::installed_fleet::resolve_installed_fleet` is now the shared
  path for list, cycles, metrics, endpoints, backup create initial discovery,
  and status live local checks.
- `canic-cli/src/backup/mod.rs` is now a facade/dispatch module at about 200
  lines, down from the previous broad command module.
- Local replica restart terminology is cleaner: missing local replica roots are
  classified as `lost`, not `stale`; remaining `stale` usage is backup
  preflight-domain terminology.

## Findings

### Medium - Backup domain modules are still large

Evidence:

- `crates/canic-backup/src/plan/mod.rs`: 932 lines.
- `crates/canic-backup/src/manifest/mod.rs`: 827 lines.
- `crates/canic-backup/src/runner/mod.rs`: 637 lines.
- `crates/canic-backup/src/restore/plan/mod.rs`: 620 lines.
- `crates/canic-backup/src/execution/mod.rs`: 562 lines.
- `crates/canic-backup/src/snapshot/mod.rs`: 521 lines.

Assessment:

These are mostly domain-owned files, so this is not a boundary violation. The
risk is change friction: backup planning, manifest validation, execution
journals, and snapshot download orchestration are still dense enough that
future work may mix record conversion, validation, and orchestration in the
same files.

Recommended follow-up:

- Split only when touching the area next. Highest-value candidates are
  `plan/mod.rs` and `manifest/mod.rs`.
- Prefer domain-shaped splits such as `selector`, `authority`, `phases`,
  `hashing`, `integrity`, and `classification` over generic utility modules.

### Medium-Low - Snapshot download still owns direct installed-state and registry plumbing

Evidence:

- `crates/canic-cli/src/snapshot/download/mod.rs` calls
  `read_named_fleet_install_state`.
- It calls `parse_registry_entries`.
- It calls `replica_query::query_subnet_registry_json`.
- It falls back to `IcpCli::canister_call_output(..., "canic_subnet_registry",
  ...)`.

Assessment:

This is an adapter path, not domain code, and it is acceptable for now because
raw process execution remains host-owned. The reason it stays on the audit list
is consistency: status/list/cycles/metrics/endpoints now share
`resolve_installed_fleet`, while snapshot download still has its own live
membership path.

Recommended follow-up:

- When snapshot download is next changed, route fleet membership discovery
  through `canic-host::installed_fleet`.
- Keep snapshot-specific display, target filtering, and legacy command behavior
  in CLI.

### Medium-Low - Backup preflight adapter still directly re-queries registry JSON

Evidence:

- `crates/canic-cli/src/backup/create.rs` uses
  `resolve_installed_fleet` for initial discovery.
- `BackupIcpRunnerExecutor::preflight_receipts` still calls
  `parse_registry_entries` and the live registry query path again.

Assessment:

This second read is reasonable because preflight must prove current topology
right before mutation. The cleanliness risk is small duplication in the adapter
shape, not backup-domain leakage. If this grows, host should expose a
preflight-friendly live topology read helper rather than letting more commands
repeat the branch.

Recommended follow-up:

- Leave as-is until real backup mutation expands.
- If another command needs the same proof read, move the proof-oriented live
  topology helper into `canic-host`.

### Low - Large test files are visible but not urgent

Evidence:

- `crates/canic-backup/src/restore/tests/apply_journal.rs`: 1021 lines.
- `crates/canic-backup/src/plan/tests.rs`: 864 lines.
- `crates/canic-cli/src/backup/tests/mod.rs`: 662 lines.
- `crates/canic-cli/src/restore/tests/run.rs`: 590 lines.

Assessment:

Large test files increase navigation cost but are lower risk than production
module size. Split only around stable behavior groups when editing those tests.

## Boundary Verdict

- CLI/host/backup ownership is now coherent enough to keep building on.
- `canic-backup -> canic-host` dependency leakage is resolved.
- Host is the right home for installed-fleet and live registry mechanics.
- CLI still has a couple of transport adapter seams, but they are thin enough
  to tolerate until the related commands are next touched.
- The next cleanup should target backup-domain file size, not another broad CLI
  sweep.

## Evidence Commands

- `cargo tree -p canic-backup`
- `rg -n "canic_host|canic-host|StaleLocalFleet|detect_stale_local_root|\\bstale\\b" crates/canic-backup crates/canic-backup/Cargo.toml crates/canic-cli/src/backup crates/canic-cli/src/status crates/canic-cli/src/snapshot crates/canic-host/src/installed_fleet.rs -g '*.rs' -g 'Cargo.toml'`
- `rg -n "read_named_fleet_install_state|resolve_installed_fleet|query_subnet_registry_json|canister_call_output\\(.*canic_subnet_registry|parse_registry_entries" crates/canic-cli/src crates/canic-host/src crates/canic-backup/src -g '*.rs'`
- `find crates/canic-cli/src crates/canic-host/src crates/canic-backup/src -maxdepth 3 -type f -name '*.rs' -print0 | xargs -0 wc -l | sort -nr | head -80`

## Validation Already Run In This Cleanup Slice

- `cargo fmt --all`
- `cargo check -p canic-backup`
- `cargo check -p canic-host -p canic-backup -p canic-cli`
- `cargo test -p canic-backup --lib -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-host -p canic-backup -p canic-cli --all-targets -- -D warnings`
