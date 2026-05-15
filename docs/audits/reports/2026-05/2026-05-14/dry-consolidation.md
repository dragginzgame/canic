# DRY Consolidation Audit - 2026-05-14

## Report Preamble

- Definition path: `docs/audits/recurring/system/dry-consolidation.md`
- Scope: maintained Canic source under `crates/**`, `canisters/**`,
  `fleets/**`, `scripts/**`, plus current audit/governance context.
- Exclusions: `target/**`, `.git/**`, generated backup artifacts, and
  historical audit reports except as baselines.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-12/dry-consolidation.md`
- Code snapshot identifier: `48213853`
- Method tag/version: `DRY Consolidation V3`
- Comparability status: `partially comparable`; the scope is comparable with
  May 12, while the method is now promoted to a recurring system audit.
- Auditor: `codex`
- Run timestamp: `2026-05-14`
- Worktree state: `dirty before report write`

## Executive Summary

The May 12 findings were useful, but the report is now stale. Most of the
highest-value consolidation follow-up has landed: installed-fleet resolution,
registry parsing, response parsing primitives, cycle/metric/list endpoint
parsers, and command catalog/help ownership have moved toward clearer owners.

Current consolidation risk is **4 / 10**, down from **5 / 10** on May 12.

Remaining DRY pressure is narrower:

1. parts of `backup` and `status` still carry local installed-state or
   registry-loading variants.
2. CLI family/subcommand glue is still repeated across several command modules.
3. Backup/restore fixtures are intentionally local for now, but the duplicated
   journal/manifest setup should be revisited after restore execution settles.
4. `scripts/ci/wasm-audit-report.sh` remains a large shell subsystem.

## Inventory

| Area | Files / Lines | Readout |
| --- | ---: | --- |
| Full maintained source under `crates`, `canisters`, `fleets`, `scripts` | 935 files / 128,237 lines | Broad source inventory, excluding generated outputs. |
| Operator slice: `canic-cli`, `canic-host`, `canic-backup` | 40,145 Rust lines | Still the main consolidation pressure area. |
| Rust files >= 600 LOC across `crates` | 24 files above threshold plus aggregate total row | Large-file risk has shifted away from CLI command roots toward host modules and tests. |
| Scripts | 2,612 total lines | One large audit script dominates: `scripts/ci/wasm-audit-report.sh` at 963 lines. |

Largest current Rust files above the operator threshold:

| Lines | File |
| ---: | --- |
| 1,096 | `crates/canic-backup/src/restore/tests/apply_journal.rs` |
| 1,058 | `crates/canic-host/src/icp.rs` |
| 865 | `crates/canic-backup/src/plan/tests.rs` |
| 840 | `crates/canic-host/src/install_root/tests.rs` |
| 830 | `crates/canic-host/src/release_set/mod.rs` |
| 747 | `crates/canic-host/src/install_root/mod.rs` |
| 662 | `crates/canic-cli/src/backup/tests/mod.rs` |
| 627 | `crates/canic-host/src/icp_config.rs` |
| 603 | `crates/canic-host/src/replica_query.rs` |
| 596 | `crates/canic-cli/src/restore/tests/run.rs` |
| 591 | `crates/canic-host/src/release_set/config.rs` |
| 580 | `crates/canic-cli/src/replica/mod.rs` |

## Positive Consolidation Readout

- `canic-host::installed_fleet` now owns installed-fleet resolution,
  local-replica preference, ICP CLI fallback, registry parsing, and topology
  projection for the common deployed-fleet path.
- `canic-host::registry::parse_registry_entries` owns live registry parsing.
- `canic-host::response_parse` owns shared recursive JSON field lookup,
  `response_candid` extraction, numeric parsing, Candid record scanning, and
  cycle-balance parsing.
- `canic-cli` list, cycles, metrics, and endpoints are split into focused
  options/transport/parse/render-style modules instead of broad command roots.
- Top-level command catalog/help grouping has moved out of `canic-cli::lib`.
- `canic-host::table` and `canic-host::format` remain shared output helpers.

## Findings

### Medium-Low - Some installed-fleet and registry paths still bypass the shared resolver

Evidence:

- Common path users now include list, cycles, metrics, endpoints, status, and
  backup create through `canic-host::installed_fleet`.
- Remaining local variants are still visible in:
  - `crates/canic-cli/src/backup/create.rs`
  - `crates/canic-cli/src/status/mod.rs`

Impact:

- The main DRY risk from May 12 is reduced, but not gone.
- Commands with intentionally different preflight or diagnostic behavior still
  need local code. That is acceptable, but it should stay deliberate.

Recommended consolidation:

- For `backup create`, keep backup-specific topology/preflight behavior local,
  but avoid re-parsing registry JSON after a successful installed-fleet
  resolution unless a separate authority proof requires a fresh read.
- For `status`, keep the split between state-only rows and live verification
  explicit; do not reintroduce local registry parsing.

### Medium-Low - Host ICP and replica modules are becoming the new broad owners

Evidence:

- `crates/canic-host/src/icp.rs` is 1,058 lines.
- `crates/canic-host/src/replica_query.rs` is 603 lines.
- `crates/canic-host/src/icp_config.rs` is 627 lines.
- `crates/canic-host/src/install_root/mod.rs` remains 747 lines.

Impact:

- Moving behavior out of CLI into host is the right boundary, but `canic-host`
  now needs internal ownership discipline.
- The risk is not duplication yet; it is broad owner modules accumulating many
  unrelated helper families.

Recommended consolidation:

- Split only when adding behavior. Good future splits would be
  `icp::canister`, `icp::network`, `icp::snapshot`, or narrower
  `replica_query::status` / `replica_query::registry` modules.
- Do not split solely for line count while the current code is stable.

### Medium-Low - CLI command-family glue remains repetitive

Evidence:

- Repeated `print_help_or_version`, `parse_subcommand`, `disable_help_flag`,
  and `render_help` patterns remain across backup, restore, replica, snapshot,
  manifest, fleet, and standalone commands.
- `CommandSpec` centralizes top-level help, but family subcommand dispatch still
  uses local boilerplate.

Impact:

- This is not a correctness risk today.
- It creates friction when adding or renaming subcommands and leaves help/version
  consistency dependent on local review.

Recommended consolidation:

- Add a small command-family helper only after current backup/restore and
  replica command shapes settle.
- Keep typed option parsing in each command module; centralize only family
  dispatch/help/version plumbing.

### Low - Response parsing is mostly consolidated; page parsing remains local by domain

Evidence:

- `cycles` and `metrics` import primitives from `canic-host::response_parse`.
- `list` reuses host-owned cycle-balance and Candid field helpers.
- `install_root` uses host-owned cycle-balance parsing.

Impact:

- This is now a healthy split: low-level parsing primitives are shared, while
  domain-specific pages remain in command modules.
- Further consolidation should happen only for repeated envelope shapes, not
  every page-specific parser.

Recommended consolidation:

- Add a typed host helper for the common `response_candid` / JSON fallback
  pattern only if a third page parser repeats the same shape.

### Low - Backup/restore fixture duplication is still acceptable while behavior is moving

Evidence:

- Large backup/restore test files are now the largest operator files:
  `restore/tests/apply_journal.rs`, `plan/tests.rs`,
  `canic-cli/src/backup/tests/mod.rs`, and `restore/tests/run.rs`.
- Fixture helpers around journals, artifacts, backup plans, manifests, and fake
  scripts remain local.

Impact:

- Tests are large, but much of the duplication is protecting still-changing
  backup/restore behavior.
- Premature shared fixtures would make domain rules harder to see.

Recommended consolidation:

- Defer broad fixture sharing until restore execution stabilizes.
- When stable, move only durable builders into crate-local `test_support`
  modules; do not expose them through production APIs.

### Low - Script consolidation remains dominated by wasm audit reporting

Evidence:

- `scripts/ci/wasm-audit-report.sh` is 963 lines.
- The next largest script-like files are far smaller:
  `scripts/dev/install_dev.sh` at 251 lines and packaged downstream verifiers
  at 165-167 lines.

Impact:

- The script is a real subsystem, but it is isolated.
- Splitting it now is optional unless wasm audit churn continues.

Recommended consolidation:

- Keep the entrypoint stable.
- If it changes again, split helper fragments under `scripts/ci/wasm-audit/` or
  move report assembly into a small Rust helper.

## Risk Matrix

| Category | Risk | Notes |
| --- | ---: | --- |
| Ownership boundaries | 3 / 10 | CLI/host/backup direction remains coherent; host is the right owner for shared operator mechanics. |
| Runtime code duplication | 3 / 10 | Response parsing and registry parsing have clearer host ownership. |
| CLI command duplication | 5 / 10 | Family glue and some registry-loading variants remain. |
| Backup/restore fixture duplication | 5 / 10 | Large tests remain, but deferral is intentional while restore behavior moves. |
| Script duplication | 3 / 10 | One large wasm audit script, otherwise contained. |
| Overall | 4 / 10 | Lower than May 12 because behavior-bearing installed-fleet and response parsing duplication was reduced. |

## Recommended Order

1. Keep `backup create` registry/preflight duplication local only where a fresh
   authority or topology proof requires it.
2. Add command-family dispatch/help glue after backup/restore and replica
   command shapes settle.
3. Split `canic-host::icp` only when adding the next ICP command family.
4. Revisit backup/restore test support after restore execution stops changing.
5. Leave `wasm-audit-report.sh` alone unless the audit workflow changes again.

## Follow-up Applied

- Routed `snapshot download` through `canic-host::installed_fleet` for the
  installed-fleet path, while preserving explicit `--canister --root` fallback
  behavior when no fleet state exists.
- Reused cached installed-fleet registry entries for snapshot membership
  validation and snapshot driver registry traversal when the requested root is
  the installed root.
- Changed `medic` to use the host installed-fleet state reader instead of
  calling the install-root state loader directly.
- Added focused CLI tests for cached snapshot membership validation and medic's
  missing-installed-fleet warning classification.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git rev-parse --short HEAD` | PASS | Captured snapshot `48213853`. |
| `git status --short` | PASS | Confirmed dirty worktree before report write. |
| `find crates canisters fleets scripts ... wc -l` | PASS | Counted 935 maintained source files. |
| `find crates canisters fleets scripts ... xargs wc -l` | PASS | Counted 128,237 maintained source lines. |
| `find crates/canic-cli crates/canic-host crates/canic-backup ... xargs wc -l` | PASS | Counted 40,145 operator Rust lines. |
| `find crates ... awk '$1 >= 600'` | PASS | Captured large Rust file inventory. |
| `find crates/canic-cli crates/canic-host crates/canic-backup ... awk '$1 >= 500'` | PASS | Captured operator large-file inventory. |
| `find scripts -type f ... wc -l` | PASS | Captured script size inventory. |
| `rg "read_named_fleet_install_state\|parse_registry_entries\|query_subnet_registry_json\|InstalledFleetResolution\|installed_fleet"` | PASS | Confirmed shared resolver adoption and remaining local variants. |
| `rg "parse_json\|parse_.*candid\|find_field\|response_candid\|canister_call_output\|response_parse"` | PASS | Confirmed host-owned response parsing primitives and local page parsers. |
| `rg "print_help_or_version\|parse_subcommand\|disable_help_flag\|render_help\|CommandSpec\|command_catalog"` | PASS | Confirmed remaining command-family glue repetition. |

## Follow-up Actions

No immediate remediation required from this audit.

Watchpoints:

1. Keep future shared operator mechanics in `canic-host`, but split host modules
   before they become unrelated helper hubs.
2. Keep command-family glue consolidation scoped to dispatch/help/version
   behavior only.
3. Revisit backup/restore fixture support after restore execution stabilizes.
