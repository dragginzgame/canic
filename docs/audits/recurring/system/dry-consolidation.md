# Audit: DRY Consolidation

## Purpose

Find avoidable duplication, repeated ownership decisions, and copy/paste
workflow patterns before they become change-friction or correctness drift.

This audit is not a mandate to abstract everything. It distinguishes useful
local duplication from repeated behavior that should have one owner.

## Risk Model

Duplication is risky when multiple files must remember the same rule, transport
path, parser shape, output convention, or fixture schema. The highest-risk DRY
problems are repeated side-effectful workflows and repeated parse/validation
logic, not local formatting or tests that intentionally keep setup nearby.

## Run This Audit After

- broad CLI, host, or backup/restore changes
- ICP CLI response-shape changes
- command-family additions or option rewrites
- shared parser, registry, table, or installed-fleet changes
- large module splits or follow-up cleanup passes

## Report Preamble

Every report generated from this audit must include:

- Scope
- Exclusions
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status
- Worktree state

## Required Inventory

Run and record current size evidence:

```bash
find crates canisters fleets scripts -type f \( -name '*.rs' -o -name '*.sh' -o -name '*.toml' -o -name '*.md' \) -not -path '*/target/*' | wc -l
find crates canisters fleets scripts -type f \( -name '*.rs' -o -name '*.sh' -o -name '*.toml' -o -name '*.md' \) -not -path '*/target/*' -print0 | xargs -0 wc -l | tail -1
find crates/canic-cli crates/canic-host crates/canic-backup -type f -name '*.rs' -print0 | xargs -0 wc -l | tail -1
find crates -type f -name '*.rs' -print0 | xargs -0 wc -l | awk '$1 >= 600 { print }' | sort -nr | head -30
find crates/canic-cli crates/canic-host crates/canic-backup -type f -name '*.rs' -print0 | xargs -0 wc -l | awk '$1 >= 500 { print }' | sort -nr
find scripts -type f -print0 | xargs -0 wc -l | sort -nr | head -20
```

## Required Scans

Installed-fleet and registry ownership:

```bash
rg -n "read_named_fleet_install_state|parse_registry_entries|query_subnet_registry_json|InstalledFleetResolution|installed_fleet" crates/canic-cli crates/canic-host crates/canic-backup -g '*.rs'
```

Response parsing ownership:

```bash
rg -n "parse_json|parse_.*candid|find_field|response_candid|canister_call_output|response_parse" crates/canic-cli crates/canic-host crates/canic-backup -g '*.rs'
```

Command-family glue:

```bash
rg -n "print_help_or_version|parse_subcommand|disable_help_flag|render_help|CommandSpec|command_catalog" crates/canic-cli/src -g '*.rs'
```

Test fixture duplication:

```bash
rg -n "TempDir|write_artifact|journal_with_checksum|backup-plan|backup-execution-journal|fixture|fake_" crates/canic-cli crates/canic-host crates/canic-backup -g '*.rs'
```

Output conventions:

```bash
rg -n "render_table|ColumnAlign|write_text|write_pretty_json|println!" crates/canic-cli crates/canic-host crates/canic-backup -g '*.rs'
```

## Evaluation Checklist

### Ownership Duplication

Check whether the same behavior is implemented in more than one crate or
command family:

- installed fleet lookup
- live registry loading and parsing
- ICP CLI response envelopes
- local-replica vs ICP CLI fallback selection
- table rendering and output-file handling
- command family help/version/subcommand dispatch
- backup/restore manifest, journal, and receipt fixture construction

### Consolidation Quality

For each repeated pattern, decide whether to:

- centralize now because it is behavior-bearing and stable;
- leave local because it is command-specific, test-only, or still changing;
- split a large file first because local ownership is unclear;
- defer because abstraction would hide domain rules.

### Positive Evidence

Reports must call out consolidation that is already working, such as shared
table rendering, host-owned response parsing, host-owned registry parsing, or
crate-local support modules.

## Findings Format

Use severity headings:

- High: repeated behavior can cause correctness or safety drift
- Medium: repeated behavior increases multi-file patch radius or operator UX
  drift
- Low: local duplication is annoying but currently contained
- Watchpoint: expected hotspot, no action until it changes again

For each finding include:

- Evidence
- Impact
- Recommended consolidation
- Deferral reason, if not acting now

## Risk Matrix

Reports must include:

| Category | Risk | Notes |
| --- | ---: | --- |
| Ownership boundaries | `<0-10>` | `<notes>` |
| Runtime code duplication | `<0-10>` | `<notes>` |
| CLI command duplication | `<0-10>` | `<notes>` |
| Backup/restore fixture duplication | `<0-10>` | `<notes>` |
| Script duplication | `<0-10>` | `<notes>` |
| Overall | `<0-10>` | `<notes>` |

## Risk Score

Risk Score: **X / 10**

Interpretation scale:

- 0-2 = negligible risk
- 3-4 = low risk
- 5-6 = moderate risk
- 7-8 = high risk
- 9-10 = critical architectural risk

The score should move down only when behavior-bearing duplication was removed
or guarded by a clear owner. It should not move down just because large files
were split.
