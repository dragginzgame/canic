# DRY Consolidation Audit - 2026-05-12

## Report Preamble

- Scope: full maintained codebase: `crates/**`, `canisters/**`, `fleets/**`,
  `scripts/**`, root build/config files, and current audit/governance context.
- Exclusions: `target/**`, `.git/**`, generated backup artifacts, historical
  audit reports except as baselines.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-07/dry-consolidation.md`.
- Code snapshot identifier: `3b767536` plus dirty 0.34.5 worktree.
- Method tag/version: `DRY Consolidation V2`.
- Comparability status: non-comparable. This run expands from the May 7
  operator-only scope to the full maintained codebase and includes the current
  ICP CLI hard-cut and backup/restore work.
- Auditor: Codex.
- Run timestamp: 2026-05-12 Europe/Paris.

## Executive Summary

The codebase is in decent shape for a pre-1.0 project that has just hard-cut
from DFX to ICP CLI. The major old duplications called out on May 7 have mostly
been absorbed: table rendering is centralized, registry parsing is shared,
duration formatting is now shared, `IcpCli` owns most ICP process construction,
and CLI response parsing has a common helper module.

The current consolidation risk is **5 / 10**.

The remaining risk is not a broad architectural boundary failure. It is
localized in three places:

1. fleet-installed-state plus live registry loading is repeated across many CLI
   commands;
2. `canic-cli` command modules are carrying transport, parsing, rendering, and
   tests in a few very large files;
3. response parsing has good shared primitives, but page/record parsing patterns
   are still reimplemented in cycles, metrics, host readiness, and install
   funding paths.

## Inventory

| Area | Files / Lines | Readout |
| --- | ---: | --- |
| Full Rust scope under `crates`, `canisters`, `fleets`, `scripts` | 777 files / 118,625 lines | Broad source inventory, excluding generated outputs. |
| Operator slice: `canic-cli`, `canic-host`, `canic-backup` | 111 files / 35,634 lines | Main active consolidation pressure area. |
| Rust files >= 600 LOC across `crates` | 31 files / 24,968 lines | Large-file risk remains concentrated. |
| Operator files >= 600 LOC | 16 files / 13,327 lines | CLI/backup/host command and model hubs dominate. |
| Scripts | 2,565 total lines | One large audit script dominates: `scripts/ci/wasm-audit-report.sh` at 963 lines. |

Largest current files:

| Lines | File |
| ---: | --- |
| 1,294 | `crates/canic-cli/src/backup/mod.rs` |
| 1,049 | `crates/canic-cli/src/endpoints.rs` |
| 1,004 | `crates/canic-cli/src/cycles/mod.rs` |
| 932 | `crates/canic-backup/src/plan/mod.rs` |
| 906 | `crates/canic-cli/src/lib.rs` |
| 827 | `crates/canic-backup/src/manifest/mod.rs` |
| 794 | `crates/canic-host/src/release_set/mod.rs` |
| 719 | `crates/canic-cli/src/metrics/mod.rs` |
| 680 | `crates/canic-cli/src/list/mod.rs` |
| 672 | `crates/canic-host/src/icp.rs` |

## Positive Consolidation Readout

- `canic_host::table` is now the central table renderer used by list, status,
  fleet, backup, medic, metrics, cycles, endpoints, and host config selection.
- `canic_host::format::{cycles_tc, byte_size, compact_duration}` now owns the
  common operator-facing numeric/duration labels.
- `canic_host::registry::parse_registry_entries` is the shared live registry
  parser used by CLI and backup/snapshot logic.
- `canic_host::icp::IcpCli` owns most ICP subprocess construction, network
  selection, `--output`, snapshot id parsing, and stdout/stderr mapping.
- `canic-host::response_parse` centralizes recursive JSON field lookup, basic
  numeric parsing, Candid field extraction, quoted strings, and Candid record
  block scanning.
- `scripts/dev/**` remains documented as intentional maintainer tooling rather
  than stale public surface.

## Findings

### Medium - Installed fleet registry loading is repeated across CLI commands

Evidence:

- `read_named_fleet_install_state` plus `call_subnet_registry` plus
  `parse_registry_entries` appears in:
  - `crates/canic-cli/src/list/live.rs`
  - `crates/canic-cli/src/backup/mod.rs`
  - `crates/canic-cli/src/cycles/mod.rs`
  - `crates/canic-cli/src/metrics/mod.rs`
  - `crates/canic-cli/src/endpoints.rs`
  - `crates/canic-cli/src/snapshot/download.rs`
  - `crates/canic-cli/src/status.rs`
- The local-replica branch
  `replica_query::query_subnet_registry_json(...)` vs ICP CLI
  `canister_call_output(..., "canic_subnet_registry", Some("json"))` is
  restated in several of those modules.

Impact:

- Every fleet-scoped command has to remember the same state, local replica, ICP
  CLI, and parser rules.
- Small behavior changes, such as local query preference or registry error text,
  require multi-file edits and risk command drift.

Recommended consolidation:

- Add one helper in the CLI/host operator boundary, for example
  `canic-cli/src/fleet_registry.rs`, that accepts `{fleet, network, icp}` and
  returns `{install_state, registry_entries}`.
- Keep command-specific error wording at the edge, but make the transport and
  parse path single-source.
- Start with `cycles`, `metrics`, `list`, and `snapshot download`; these share
  the highest-overlap shape.

### Medium - Large CLI command modules mix parsing, transport, rendering, and tests

Evidence:

- `crates/canic-cli/src/backup/mod.rs` is 1,294 lines.
- `crates/canic-cli/src/endpoints.rs` is 1,049 lines.
- `crates/canic-cli/src/cycles/mod.rs` is 1,004 lines.
- `crates/canic-cli/src/metrics/mod.rs` is 719 lines.
- `crates/canic-cli/src/lib.rs` is 906 lines and owns top-level help grouping,
  dispatch, global option rewriting, command specs, and tests.

Impact:

- These files are not necessarily wrong, but they are active edit centers and
  already contain several separable responsibilities.
- New command behavior is likely to be implemented locally instead of being
  promoted into shared helpers, because the nearby code is already broad.

Recommended consolidation:

- Split command modules along the already-emerging pattern:
  `options.rs`, `transport.rs`, `parse.rs`, `render.rs`, and `tests/`.
- Highest-value first split: `cycles` and `endpoints`, because they now carry
  reusable response parsing and Candid rendering logic.
- Keep `backup/mod.rs` until the current backup/restore execution path
  stabilizes; it is larger, but churn there is still feature work.

### Medium-Low - Response parsing primitives are shared, but page parsing is not

Evidence:

- `crates/canic-cli/src/response_parse.rs` already owns
  `find_field`, `parse_json_u64`, `parse_json_u128`,
  `field_value_after_equals`, `parse_u64_digits`, `parse_u128_digits`, and
  `candid_record_blocks`.
- `cycles` still implements separate JSON/text page parsers for cycle tracker
  and top-up event pages.
- `metrics` implements its own JSON/text metric page parser using the same
  recursive field and Candid-record helpers.
- At audit time, `host install_root` had its own cycle-balance response parser
  and leading integer parser that duplicated the former
  `canic-cli::response_parse` behavior because host could not depend on CLI.

Impact:

- The current primitives are useful but too low-level. Every command still
  assembles its own wrapped response logic.
- Host and CLI can diverge because the richer parser helpers live in CLI.

Recommended consolidation:

- Move response parsing primitives that are not CLI-specific into
  `canic-host`, such as `parse_cycle_balance_response`, recursive JSON field
  lookup, and numeric parsing.
- Add small typed helpers for common ICP CLI response shapes:
  - JSON page with `entries` and `total`;
  - `response_candid` wrapper extraction;
  - Candid record-block iteration.
- Then make `cycles`, `metrics`, `list`, and `install_root` use the host helper
  instead of local wrappers.

### Medium-Low - CLI family/subcommand glue is still repetitive

Evidence:

- `print_help_or_version` and `parse_matches` are centralized, but family
  commands still repeat the same dispatch shape in `backup`, `restore`,
  `replica`, `snapshot`, `manifest`, and `fleet`.
- The scan found repeated clusters of:
  - `if print_help_or_version(&args, usage, version_text())`;
  - `parse_subcommand(...)`;
  - command-local `*_usage()` rendering wrappers;
  - `disable_help_flag(true)` per family and child command.

Impact:

- This is low correctness risk, but high friction when adding or renaming
  subcommands.
- Help/version consistency depends on reviewers noticing repeated boilerplate.

Recommended consolidation:

- Add a small `CommandFamily` helper in `canic-cli::args` or a new
  `canic-cli::command_family` module.
- Let family commands register subcommand specs with usage functions and
  handlers, while keeping typed option parsers inside each command module.
- Do this after the current backup/restore command shape settles.

### Low - Test fixture builders remain duplicated across backup and CLI tests

Evidence:

- The May 7 audit already called out duplicated backup/restore test fixture
  builders.
- Current scans still show overlapping helpers:
  - `journal_with_checksum` in both `canic-backup` and `canic-cli` tests;
  - `write_artifact` in both `canic-backup` and `canic-cli` tests;
  - several restore CLI fixture helpers for verified layouts, manifest
    artifacts, fake ICP upload scripts, and journal lock paths;
  - separate `temp_dir` helpers in `canic-cli`, `canic-host`, and
    `canic-backup`.

Impact:

- Test fixtures are allowed to be local, but backup/restore behavior is still
  moving. The duplication makes tests harder to update when manifest/journal
  schemas change.

Recommended consolidation:

- Keep production APIs clean; do not promote fixtures into public runtime code.
- Once 0.34 backup/restore behavior is stable, move shared fixture builders
  into crate-local `test_support` modules:
  - `canic-backup::test_support` for domain model/layout builders;
  - `canic-cli::backup::tests::support` for CLI-only command fixtures.

### Low - Script consolidation is mostly healthy, except the wasm audit script

Evidence:

- Script total is 2,565 lines.
- `scripts/ci/wasm-audit-report.sh` is 963 lines, far larger than any other
  script.
- The next largest scripts are much smaller:
  `scripts/dev/install_dev.sh` at 251 lines,
  packaged downstream verifiers at 165-167 lines,
  and `publish-workspace.sh` at 124 lines.

Impact:

- Most script ownership is clear.
- The wasm audit script is a real subsystem in shell form: build orchestration,
  artifact capture, comparisons, and report writing live in one file.

Recommended consolidation:

- Do not rewrite it during unrelated CLI work.
- If wasm audit churn continues, split shell helpers into
  `scripts/ci/wasm-audit/` fragments or move report assembly into a small Rust
  helper while keeping the CI entrypoint stable.

### Low - Direct output printing remains in commands without `--out`

Evidence:

- `cycles`, `metrics`, `backup`, `manifest`, and `restore` use
  `canic-cli::output` helpers for JSON/text output where `--out` exists.
- `endpoints`, `list`, `status`, and `medic` still print directly.

Impact:

- This is acceptable while those commands have no `--out` surface, but the
  behavior will diverge if output-file support is added piecemeal.

Recommended consolidation:

- Leave direct printing alone until a command needs `--out`.
- If `--out` expands, make `write_text`/`write_pretty_json` the default command
  output path and keep direct `println!` only for streaming/progress output.

## Risk Matrix

| Category | Risk | Notes |
| --- | ---: | --- |
| Ownership boundaries | 3 / 10 | CLI/host/backup layering is still coherent. |
| Runtime code duplication | 4 / 10 | Some host/CLI parser duplication, but not widespread. |
| CLI command duplication | 6 / 10 | Repeated registry loading, family glue, and parse/render structure. |
| Backup/restore fixture duplication | 5 / 10 | Known issue, best fixed after behavior stabilizes. |
| Script duplication | 3 / 10 | Mostly healthy; wasm audit script is the only large shell hub. |
| Overall | 5 / 10 | Worth cleanup before 0.35/1.0, not a release blocker. |

## Recommended Order

1. Add a shared fleet-installed-registry loader and route `list`, `cycles`,
   `metrics`, and `snapshot download` through it.
2. Move CLI response parsing primitives that host also needs into
   `canic-host`, then deduplicate cycle-balance parsing.
3. Split `cycles` into `options`, `transport`, `parse`, and `render`.
4. Split `endpoints` into `options`, `lookup`, `parse`, and `render`.
5. After backup/restore functionality stabilizes, consolidate test fixtures in
   crate-local test support modules.
6. Revisit `scripts/ci/wasm-audit-report.sh` only if the wasm audit workflow
   keeps changing.

## Follow-up Applied

- Started item 1 by adding the initial `canic-cli::installed_fleet`, then
  promoted the resolver to `canic-host::installed_fleet` with
  `InstalledFleetResolution`, `InstalledFleetSource`, `InstalledFleetRegistry`,
  and `ResolvedFleetTopology`.
- Routed `canic list`, `canic cycles`, and `canic metrics` through the shared
  resolver.
- Split `canic endpoints` into command orchestration, transport, Candid parsing,
  rendering, and model modules, and routed its fleet registry lookup through the
  shared installed-fleet resolver. `snapshot download`, `backup`, and `status`
  remain follow-up resolver candidates.
- Split `canic cycles` into command orchestration, options, transport/report
  collection, cycle response parsing, rendering, and model modules.
- Split `canic metrics` into command orchestration, options, transport/report
  collection, metric response parsing, rendering, and model modules.
- Split the top-level CLI command catalog/help rendering and global option
  forwarding out of `canic-cli::lib`.
- Moved shared ICP response parsing primitives from `canic-cli` to
  `canic-host::response_parse`, and switched CLI list/cycles/metrics parsers to
  import the host-owned helpers directly.
- Moved the live subnet registry DTO/parser from `canic-backup::discovery` to
  `canic-host::registry`.
- Promoted the shared installed-fleet resolver from `canic-cli` to
  `canic-host::installed_fleet`, so CLI commands consume the host-owned
  install-state, local-replica preference, ICP CLI fallback, registry parsing,
  and topology projection boundary.

## Deferred Boundary Follow-up

- `snapshot download`, `backup`, and `status` still contain some direct
  registry-loading logic. They should either consume
  `canic-host::installed_fleet` directly or add narrower host helpers where
  their flows intentionally differ from deployed-fleet lookup.
- Receipt normalization remains a future-risk category: backup, restore,
  authority, provider, topology, and snapshot receipts need shared naming,
  timestamp, truncation, and provider-metadata conventions before they diverge.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `sed -n '1,240p' docs/status/current.md` | PASS | Loaded current session handoff and active 0.34 context. |
| `find docs/audits -maxdepth 4 -type f \| sort` | PASS | Located prior dry-consolidation reports and current audit layout. |
| `sed -n '1,260p' docs/audits/reports/2026-05/2026-05-07/dry-consolidation.md` | PASS | Used May 7 code DRY report as historical baseline. |
| `sed -n '1,260p' docs/audits/reports/2026-05/2026-05-09/dry-consolidation.md` | PASS | Confirmed May 9 report was docs-specific, not code-specific. |
| `git rev-parse --short HEAD` | PASS | Captured snapshot `3b767536`. |
| `git status --short` | PASS | Confirmed dirty 0.34.5 worktree before audit. |
| `find crates canisters fleets scripts ...` | PASS | Captured full maintained source inventory. |
| `find ... -name '*.rs' ... wc -l` | PASS | Captured file/line counts and large-file hotspots. |
| `rg "parse_json|parse_.*candid|find_field|response_candid|canister_call_output"` | PASS | Identified response parser overlap. |
| `rg "read_named_fleet_install_state|parse_registry_entries|query_subnet_registry_json"` | PASS | Identified repeated installed-registry loading path. |
| `rg "render_table|ColumnAlign|println!"` | PASS | Confirmed table rendering is mostly consolidated; progress printing remains intentionally custom. |
| `rg "print_help_or_version|parse_matches|disable_help_flag|render_help"` | PASS | Identified remaining command-family glue repetition. |
| `rg "TempDir|write_artifact|journal_with_checksum|backup-plan|backup-execution-journal"` | PASS | Identified repeated test fixture builders. |
