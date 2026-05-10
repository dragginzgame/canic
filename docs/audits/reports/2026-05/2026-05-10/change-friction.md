# Change Friction Audit - 2026-05-10

## Report Preamble

- Scope: `crates/canic`, `crates/canic-core`, `crates/canic-control-plane`,
  `crates/canic-host`, `crates/canic-cli`, `crates/canic-backup`,
  `crates/canic-wasm-store`, `fleets/**`, `canisters/**`, `scripts/**`,
  `icp.yaml`, CI/dev-support files, docs, and recent `0.33.x` feature slices.
- Compared baseline report path:
  `docs/audits/reports/2026-04/2026-04-06/change-friction.md`
- Code snapshot identifier: `09f5d238`
- Method tag/version: `change-friction-v4.1`
- Comparability status: partially comparable: the CAF method is unchanged, but
  the current sample is dominated by the `0.33.x` ICP CLI hard cut, published
  operator crate work, and post-hard-cut cleanup rather than the smaller
  `0.25.x` runtime/testkit slices from the baseline.
- Exclusions applied: generated `target/**`, `.icp/**`, runtime caches,
  lockfile-only noise when judging feature locality, and generated audit report
  artifacts outside this report.
- Notable methodology changes vs baseline: operator crates and active fleet
  configs are now in scope because `canic-cli`, `canic-host`, and
  `canic-backup` became central 0.33 package surfaces.
- ICP CLI reload context: local `icp` resolves to `/home/adam/.local/bin/icp`
  and reports version `0.2.6`; `icp project show` loaded the project after
  refreshing the package cache under `~/.local/share/icp-cli/pkg`.

## Executive Summary

- Risk Score: **5 / 10**
- Delta summary: change friction is materially higher than the April baseline.
  The sampled 0.33 slices average `63.2` touched files, up from `19.25`, and
  the p95 sampled blast radius is `88`, up from `27`.
- Interpretation: this is expected medium release-line friction from the hard
  DFX-to-ICP CLI cut and follow-on CLI/host/runtime cleanup, not a confirmed
  architecture break. No cross-layer dependency breach was found in the
  sampled slices.
- Largest remaining pressure: operator changes still often coordinate
  `canic-cli`, `canic-host`, `icp.yaml`, fleet configs, docs, and selected
  runtime metadata/list endpoints.
- Follow-up required: yes. Future feature work should keep new operator
  behavior behind either CLI UX, host mechanics, or backup domain ownership
  rather than touching all three for routine changes.

## Baseline Delta Summary

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Velocity Risk Index | 2 | 5 | +3 |
| Cross-layer leakage crossings | 0 | 0 | 0 |
| Avg files touched per sampled slice | 19.25 | 63.17 | +43.92 |
| p95 files touched | 27 | 88 | +61 |
| Top gravity-well fan-in | 2 | 5 | +3 |

Notes:

- Current averages use sampled 0.33 commits `8a5814fd`, `cf24f77e`,
  `53476764`, `6ea85fdb`, `5b474986`, and `09f5d238`.
- `0.33.7` is now committed and included in the sample. It touched `32` files,
  which lowers the average patch radius but confirms that metadata/list work
  still crosses runtime, host, CLI, Candid, docs, and audit/status surfaces.
- Changelog and docs files are counted in blast-radius totals because the 0.33
  work changed public commands and operator documentation, but they are not
  treated as architectural leakage.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Feature slices sampled from recent commits | PASS | sampled the main `0.33.0`, `0.33.1`, `0.33.2`, `0.33.5`, `0.33.6`, and `0.33.7` implementation commits |
| Release sweeps separated from routine slices | PASS | `8a5814fd`, `cf24f77e`, and `6ea85fdb` are treated as release-line or cleanup sweeps, not ordinary feature locality |
| Blast radius measured | PASS | sampled committed slices touched `32` to `88` files |
| Operator scope included | PASS | CLI, host, backup, fleet, ICP config, and scripts were included because 0.33 makes them first-class surfaces |
| ICP CLI project context reloaded | PASS | `icp --version` reports `0.2.6`; `icp project show` loaded local/demo/test/ic environments and project canisters |
| Boundary leakage reviewed | PASS | no crate cycle, CLI/host/backup reverse edge, or policy/ops/storage layering breach was confirmed by this run |
| Friction amplification drivers identified | PASS | highest pressure is repeated command/config/list/install coordination across CLI, host, core, docs, and fleet config |

## Amplification Drivers

| Commit | Feature Slice | Slice Type | Files Touched | Subsystems | Layers | Flow Axes | Revised CAF | Density | ELS | Feature Locality Index | Containment Score | Risk |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `8a5814fd` | ICP CLI hard cut and initial operator flow replacement | `release_sweep` | 82 | 8 | 5 | 4 | 32 | 10.25 | 0.61 | 0.06 | 0.45 | High |
| `cf24f77e` | fleet/status/replica command consolidation plus auth/replay follow-up | `release_sweep` | 77 | 8 | 5 | 4 | 32 | 9.63 | 0.58 | 0.08 | 0.50 | High |
| `53476764` | metrics/auth/API audit cleanup and command status refinement | `feature_slice` | 54 | 7 | 4 | 3 | 21 | 7.71 | 0.50 | 0.12 | 0.58 | Medium |
| `6ea85fdb` | core IC management, publication, and backup/restore module split | `cleanup_slice` | 88 | 8 | 5 | 4 | 32 | 11.00 | 0.42 | 0.07 | 0.62 | Medium |
| `5b474986` | Candid discovery, endpoint/list/install surface cleanup | `feature_slice` | 46 | 7 | 4 | 3 | 21 | 6.57 | 0.48 | 0.15 | 0.64 | Medium |
| `09f5d238` | Canic metadata endpoint, list visibility, snapshot-download follow-up, and 0.34 backup/restore design notes | `feature_slice` | 32 | 7 | 4 | 3 | 21 | 4.57 | 0.45 | 0.19 | 0.68 | Medium |

Interpretation:

- `0.33.0` and `0.33.1` are intentionally broad hard-cut slices. They should
  not be used as a normal routine-feature target.
- `0.33.5` is broad but mostly structural cleanup; it reduced future module
  friction even though the individual cleanup slice had a large patch radius.
- `0.33.6` is the best current guide for post-hard-cut routine operator work:
  still broader than April, but bounded around endpoint/list/install behavior.
- `0.33.7` is smaller than the hard-cut slices and now cleanly committed, but
  endpoint-contract changes still have legitimate cross-package blast radius.

## Comparison to Previous Relevant Run

- Regressed: average sampled file blast radius rose from `19.25` to `63.17`.
- Regressed: p95 sampled file blast radius rose from `27` to `88`.
- Stable: no cross-layer leakage was detected in sampled work.
- Changed: friction moved from testkit/runtime harness seams to operator
  command/config/deployment seams.
- Improved within the release line: the large 0.33.5 cleanup removed several
  structural hotspots that would otherwise keep future feature slices broad.

## Structural Hotspots

1. `crates/canic-cli/src/list/mod.rs` and `crates/canic-cli/src/lib.rs` remain
   high-touch operator files.
   Evidence: current line counts are `902` and `872`; recent command/list/status
   changes repeatedly route through these files.

2. `crates/canic-host/src/install_root/mod.rs` and
   `crates/canic-host/src/release_set/mod.rs` remain the main host-side phase
   hubs.
   Evidence: current line counts are `888` and `794`; install, release-set,
   ICP CLI, and fleet config identity work still tend to meet there.

3. Fleet/project surface edits still fan out across `icp.yaml`, `fleets/**`,
   CLI config/list rendering, host release-set config, docs, and tests.
   Evidence: sampled 0.33.6 work touched CLI args/list/install, host config,
   the test fleet config, `icp.yaml`, docs, and reference-surface tests.

4. The committed `0.33.7` metadata/list slice crosses facade, core DTO/API,
   wasm-store DID, host install/list support, CLI table rendering, docs, and
   audit/status files.
   Evidence: `09f5d238` touched `32` files. The scope is justified by endpoint
   replacement, but it remains a current friction signal.

## Hub Module Pressure

Recent-slice repeat-touch scan across the sampled `0.33.x` window, excluding
changelog-only interpretation:

| File / Module | Sampled Slice Touches | Pressure |
| --- | ---: | --- |
| `crates/canic-cli/src/list/mod.rs` | 5 | high |
| `crates/canic-cli/src/lib.rs` | 4 | high |
| `crates/canic-host/src/install_root/mod.rs` | 4 | high |
| `crates/canic-host/src/release_set/mod.rs` | 4 | high |
| `crates/canic-host/src/icp.rs` | 4 | high |
| `crates/canic-cli/src/args/mod.rs` | 3 | medium |
| `crates/canic-cli/src/install/mod.rs` | 3 | medium |
| `crates/canic-cli/src/status.rs` | 3 | medium |
| `docs/status/current.md` | 3 | expected handoff churn |

Interpretation: the repeat-touch center is now the operator surface. That is
reasonable for a DFX-to-ICP CLI transition, but routine post-0.33 work should
avoid requiring CLI root, host root, fleet config, and runtime endpoint edits at
the same time.

## Responsibility Drift Signals

- `PASS`: the sampled slices preserved CLI -> host/backup/core dependency
  direction; no host -> CLI or backup -> CLI edge was observed.
- `PASS`: no sampled runtime slice required workflow to construct storage
  records directly or policy to call platform APIs.
- `WARN`: command behavior changes still often require coordinated edits in
  CLI rendering/options, host config/discovery, docs, and tests.
- `WARN`: endpoint metadata/list changes remain broad because generated facade
  endpoints, core DTO/API exports, wasm-store Candid, host install support, and
  live CLI rendering all legitimately share the same public contract.

## Risk Score

Risk Score: **5 / 10**

Score contributions:

- `+2` hard-cut operator work raised committed-slice blast radius sharply.
- `+1` `canic-cli` list/status/install files remain repeat-touch hubs.
- `+1` host install/release-set mechanics still carry phase-file pressure.
- `+1` committed metadata/list endpoint work crosses runtime, host, and CLI
  surfaces.
- `-1` no cross-layer dependency breach or crate cycle was confirmed.
- `-1` 0.33.5 cleanup reduced future structural friction despite its broad
  local patch radius.

Verdict: **Medium change-friction risk.**

The current line is workable, but it should not be normalized as the desired
routine feature shape. The next few feature slices should aim to be narrower
than the hard-cut commits and should keep ownership boundaries explicit.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git rev-parse --short HEAD` | PASS | captured snapshot `09f5d238` |
| `git status --short` | PASS | confirmed clean worktree before this audit-report refresh |
| `icp --version` | PASS | confirmed local ICP CLI version `0.2.6` |
| `icp project show` | PASS | reloaded project context; required package-cache write access outside the workspace |
| `git log --oneline -n 35` | PASS | selected recent 0.33 implementation samples |
| `git show --stat --name-only --format=fuller 8a5814fd` | PASS | sampled ICP CLI hard-cut implementation |
| `git show --stat --name-only --format=fuller cf24f77e` | PASS | sampled fleet/status/replica and auth follow-up work |
| `git show --stat --name-only --format=fuller 53476764` | PASS | sampled metrics/auth/status cleanup |
| `git show --stat --name-only --format=fuller 6ea85fdb` | PASS | sampled structural cleanup slice |
| `git show --stat --name-only --format=fuller 5b474986` | PASS | sampled endpoint/list/install cleanup |
| `git show --stat --name-only --format=fuller 09f5d238` | PASS | sampled committed metadata/list and snapshot follow-up slice |
| `find ... wc -l` hotspot scan | PASS | captured current CLI/host/core/control-plane/backup file-size pressure |

## Follow-up Actions

1. Operator maintainers: keep routine command changes narrower than the
   hard-cut slices by deciding early whether a change belongs to CLI UX, host
   mechanics, or backup domain logic.
2. CLI maintainers: split or isolate `list` responsibilities before adding more
   live projection columns, fallback logic, or rendering modes.
3. Host maintainers: continue reducing `install_root` and `release_set` phase
   pressure when new install/deployment behavior lands.
4. Runtime/facade maintainers: when replacing generated endpoints, budget for
   facade, core DTO/API, Candid, host install support, and CLI projection work
   as one public-contract change rather than scattering follow-up patches.
