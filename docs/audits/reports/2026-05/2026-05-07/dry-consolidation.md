# DRY Consolidation Audit - 2026-05-07

## Report Preamble

- Scope: `crates/canic-cli/src/**`, `crates/canic-host/src/**`, `crates/canic-backup/src/**`, `scripts/**`
- Definition path: `N/A`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `6e72960b`
- Method tag/version: `DRY Consolidation V1`
- Comparability status: `non-comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-07T16:03:28Z`
- Branch: `main`
- Worktree: `dirty`
- Dirty files observed: `CHANGELOG.md`, `crates/canic/tests/reference_surface.rs`, `docs/changelog/0.32.md`, `scripts/app/reference_canisters.sh`, `scripts/ci/build-ci-wasm-artifacts.sh`, `scripts/ci/wasm-audit-report.sh`, `scripts/dev/README.md`

## Method

This is an ad hoc DRY/consolidation audit, not one of the formal recurring
audit definitions. The scan focused on duplicated argument parsing, duplicated
dfx/process boundaries, duplicated backup/restore model ownership, repeated
script target derivation, and cleanup candidates left after moving operator
flows into the `canic` binary.

Commands used included:

- `git status --short`
- `git rev-parse --short HEAD`
- `find crates/canic-cli/src crates/canic-host/src crates/canic-backup/src scripts -type f`
- `rg "Command::new|dfx\\s+|Dfx|canic_fleet_canisters|fleet canisters|parse_matches|first_arg_is_help|first_arg_is_version"`
- `rg "RestoreApplyJournal|DownloadJournal|BackupLayout|Manifest"`
- `find crates/canic-cli/src crates/canic-host/src crates/canic-backup/src -name '*.rs' -print0 | xargs -0 wc -l | sort -n`

## Executive Summary

The current boundary is mostly coherent:

- `canic-cli` owns Clap surfaces, operator commands, output formatting, and CLI-to-library adaptation.
- `canic-host` owns host process/dfx interactions, workspace discovery, release-set staging, install state, and simple table rendering.
- `canic-backup` owns backup manifests, journals, artifact layout, snapshot planning, restore planning, and restore runner state machines.
- `scripts/dev/**` are intentional local helper scripts and should not be treated as stale cleanup candidates.

The main remaining duplication is now around shell scripts and command glue,
not backup model design. The highest-value cleanup is making `canic fleet
canisters` the only source of truth for configured fleet canister views, with CI
scripts as dumb consumers and no shell-owned ordering rules.

Follow-up applied after the scan:

- `canic fleet canisters --ci-order` emits the canonical CI view.
- `canic fleet canisters --root-last` emits ordinary canisters followed by root.
- `canic fleet canisters --exclude-root` emits only ordinary non-root canisters.
- CI scripts now consume those target views instead of reordering targets in shell.
- CLI command modules now use a shared help/version prelude helper.
- Local install and release-set staging now construct dfx commands through shared host helpers.

Risk score: **3 / 10**

## Findings

| ID | Severity | Area | Finding | Recommendation |
| --- | --- | --- | --- | --- |
| DRY-1 | Medium | `scripts/ci` | `build-ci-wasm-artifacts.sh` and `wasm-audit-report.sh` both defined a shared target-listing helper plus nearly identical root-last target ordering. | Resolved in follow-up: `canic fleet canisters` now owns `--ci-order`, `--root-last`, and `--exclude-root`, and CI scripts consume those views directly. |
| DRY-2 | Medium-Low | `canic-cli` | Help/version pre-parsing was repeated across command modules. The scan found 16 direct `first_arg_is_help` / `first_arg_is_version` checks. | Resolved in follow-up: command modules now use one shared help/version prelude helper. |
| DRY-3 | Low | `canic-host` | DFX execution is correctly contained in host/CLI adapter layers, but `install_root` and `release_set::stage` still built several `Command::new("dfx")` calls directly instead of going through the shared `Dfx` helper. | Resolved in follow-up: host has shared default dfx command helpers, and install/release-set paths use them. |
| DRY-4 | Low | tests | Backup and restore tests duplicate fixture builders for manifests and journals across `canic-backup` and `canic-cli`. | Move shared test fixture builders into crate-local `test_support` modules. Do not promote fixture helpers into production APIs. |
| DRY-5 | Info | `canic-backup` | `canic-backup` still has large files, especially restore runner and apply journal code, but the ownership is clear and not obviously redundant. | Avoid another broad backup refactor before functional testing. Split only when a concrete behavior change forces it. |
| DRY-6 | Info | scripts | `scripts/dev/**` now has an explicit README documenting the directory as intentional local helpers. | Keep these out of stale-script cleanup passes unless the user explicitly asks to remove or migrate one. |

## Boundary Readout

`canic-cli` should remain the executable interface. It should not grow backup
state machines or dfx subprocess internals.

`canic-host` is the right place for live dfx/process behavior. The only tension
is that host currently has both a reusable `dfx::Dfx` wrapper and some
host-local direct `Command::new("dfx")` calls. This is an internal host cleanup
issue, not a CLI/backup ownership violation.

`canic-backup` should not expose its own Clap surface. It is a library for
typed backup/restore data, validation, planning, journals, and runner state.
The CLI can continue to map user intent into those typed APIs.

## Recommended Next Steps

1. Move repeated backup/restore test fixtures into crate-local `test_support` modules only after the next functional test pass identifies the fixtures that are still stable.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git status --short` | PASS | Confirmed the audit ran against an already dirty worktree. |
| `git rev-parse --short HEAD` | PASS | Snapshot id recorded as `6e72960b`. |
| `find crates/canic-cli/src crates/canic-host/src crates/canic-backup/src scripts -type f` | PASS | Scope inventory captured. |
| `rg "Command::new|dfx\\s+|Dfx|canic_fleet_canisters|fleet canisters|parse_matches|first_arg_is_help|first_arg_is_version"` | PASS | Found script target duplication and CLI command glue repetition. |
| `rg "RestoreApplyJournal|DownloadJournal|BackupLayout|Manifest"` | PASS | Confirmed backup/restore data ownership is primarily in `canic-backup`; CLI usage is adapter-level. |
| `find crates/canic-cli/src crates/canic-host/src crates/canic-backup/src -name '*.rs' -print0 | xargs -0 wc -l | sort -n` | PASS | Size scan found large files, but no immediate ownership violation. |
