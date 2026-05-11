# Audit Summary - 2026-05-12

## Run Contexts

| Report | Type | Scope | Snapshot | Worktree | Status |
| --- | --- | --- | --- | --- | --- |
| `dry-consolidation.md` | Ad hoc system | full maintained codebase: `crates/**`, `canisters/**`, `fleets/**`, `scripts/**`, root build/config context | `3b767536` plus current 0.34.5 worktree | dirty | complete |

## Risk Index Summary

| Report | Risk | Readout |
| --- | ---: | --- |
| `dry-consolidation.md` | 5 / 10 | Boundaries are coherent, but repeated installed-registry loading, large CLI command modules, and partially duplicated response parsing remain the main DRY risks. |

## Method / Comparability Notes

- `dry-consolidation.md` uses `DRY Consolidation V2`.
- It is not directly comparable with the May 7 code DRY report because the
  scope expanded from operator files and scripts to the full maintained
  codebase.
- It is not directly comparable with the May 9 dry-consolidation report because
  that report was docs-only.

## Key Findings by Severity

### Medium

- Installed fleet registry loading is repeated across many CLI commands. The
  repeated path combines install-state lookup, local replica registry query
  preference, ICP CLI fallback, and shared registry parsing.
- Large CLI command modules still mix options, transport, parsing, rendering,
  and tests. The biggest hotspots are `backup`, `endpoints`, `cycles`,
  `metrics`, and the top-level CLI module.

### Medium-Low

- Response parsing primitives are shared in `canic-cli`, but page-level parsing
  is still command-local, and host has its own cycle-balance parser copy.
- CLI family/subcommand glue still repeats the same help/version and usage
  dispatch pattern across several command families.

### Low

- Backup/restore test fixture builders remain duplicated across domain and CLI
  tests.
- Script consolidation is mostly healthy, but `scripts/ci/wasm-audit-report.sh`
  is a large shell subsystem.
- Direct output printing remains in commands without `--out`; this is acceptable
  unless output-file support expands.

## Verification Rollup

| Report | PASS | BLOCKED | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `dry-consolidation.md` | 13 | 0 | 0 | Full source inventory, large-file counts, response parsing scans, registry-loading scans, table/output scans, command-glue scans, and fixture scans completed. |

## Follow-up Actions

1. CLI/host maintainers: continue the shared installed-fleet registry loader
   rollout. `list`, `cycles`, `metrics`, and `endpoints` now use the
   host-owned resolver; `snapshot download`, `backup`, and `status` remain
   candidates.
2. Host/CLI maintainers: keep response parsing primitives needed by both host
   and CLI in `canic-host`; the first shared parsing layer now exists at
   `canic-host::response_parse`.
3. CLI maintainers: keep future command splits on the same options, transport,
   parse, render, and model boundaries used by cycles, metrics, and endpoints.
4. Backup/CLI maintainers: after 0.34 backup/restore functionality stabilizes,
   consolidate repeated fixture builders into crate-local test support modules.
5. CI maintainers: keep `scripts/ci/wasm-audit-report.sh` as-is for now, but
   split it if wasm audit behavior continues to change.
