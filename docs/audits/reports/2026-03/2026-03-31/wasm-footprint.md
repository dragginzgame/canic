# Wasm Footprint Audit - 2026-03-31

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `7ad87779`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-31T07:25:20Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `wasm-release`
- Target canisters in scope: `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `shard_hub` `shard` `test` `root` 
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `dfx`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/wasm-release/raw/` and shrunk artifacts under `artifacts/wasm-size/wasm-release/shrunk/` were recorded for `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `shard_hub` `shard` `test` `root` . |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint/size-report.json) plus per-canister `*.size-report.json` files. |
| Twiggy top captured | PASS | `*.twiggy-top.txt` and `*.twiggy-top.csv` emitted for each canister when `twiggy` is available. |
| Twiggy dominators captured | PASS | `*.twiggy-dominators.txt` emitted for each canister when `twiggy` is available. |
| Twiggy monos captured | PASS | `*.twiggy-monos.txt` emitted for each canister when `twiggy` is available. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint`; baseline path resolves to `N/A`. |
| Size deltas versus baseline recorded when baseline exists | PARTIAL | First run of day; baseline deltas are \`N/A\`. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- First run of day for `wasm-footprint`; this report establishes the daily baseline.
- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 1039585 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint/app.md) |
| `minimal` | leaf-canister | `table[0]` | 1039585 | shared-runtime floor; use this to judge workspace baseline pressure | [minimal.md](artifacts/wasm-footprint/minimal.md) |
| `user_hub` | leaf-canister | `table[0]` | 1189742 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1139858 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1094072 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale` | leaf-canister | `table[0]` | 1039585 | largest retained symbol from raw-built twiggy analysis | [scale.md](artifacts/wasm-footprint/scale.md) |
| `shard_hub` | leaf-canister | `table[0]` | 1195744 | largest retained symbol from raw-built twiggy analysis | [shard_hub.md](artifacts/wasm-footprint/shard_hub.md) |
| `shard` | leaf-canister | `table[0]` | 1039585 | largest retained symbol from raw-built twiggy analysis | [shard.md](artifacts/wasm-footprint/shard.md) |
| `test` | leaf-canister | `table[0]` | 1109376 | largest retained symbol from raw-built twiggy analysis | [test.md](artifacts/wasm-footprint/test.md) |
| `root` | bundle-canister | `table[0]` | 2405361 | bundle-canister outlier; embeds child .wasm.gz artifacts and must not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint/root.md) |

## Dependency Fan-In Pressure

- `minimal` remains the shared-runtime floor. If `minimal` stays close to feature canisters, size pressure is coming from shared crates rather than role-specific logic.
- `root` is always interpreted as a bundle canister because it embeds child `.wasm.gz` artifacts during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Minimal floor close to feature canisters | WARN | `minimal` shrunk wasm = 1897556, `app` shrunk wasm = 1897571. |
| Root bundle outlier | WARN | `root` shrunk wasm = 3112916. |
| Shrink delta unexpectedly low | WARN | `minimal` shrink delta = -10486 bytes. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 1887070 | 1897571 | -10501 | 535699 | 517812 | N/A | 3527 | 3731 | 22 | [app.md](artifacts/wasm-footprint/app.md) |
| `minimal` | leaf-canister | 1887070 | 1897556 | -10486 | 535590 | 517815 | N/A | 3527 | 3731 | 22 | [minimal.md](artifacts/wasm-footprint/minimal.md) |
| `user_hub` | leaf-canister | 2045126 | 2046012 | -886 | 588298 | 562646 | N/A | 3830 | 4033 | 25 | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | 1999862 | 2001747 | -1885 | 575379 | 554163 | N/A | 3698 | 3901 | 26 | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | 1947795 | 1954537 | -6742 | 555729 | 535015 | N/A | 3631 | 3835 | 25 | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale` | leaf-canister | 1887070 | 1897556 | -10486 | 536101 | 517827 | N/A | 3527 | 3731 | 22 | [scale.md](artifacts/wasm-footprint/scale.md) |
| `shard_hub` | leaf-canister | 2052483 | 2052705 | -222 | 591179 | 566760 | N/A | 3847 | 4050 | 26 | [shard_hub.md](artifacts/wasm-footprint/shard_hub.md) |
| `shard` | leaf-canister | 1887070 | 1897556 | -10486 | 536019 | 517826 | N/A | 3527 | 3731 | 22 | [shard.md](artifacts/wasm-footprint/shard.md) |
| `test` | leaf-canister | 1966626 | 1971070 | -4444 | 563203 | 542527 | N/A | 3644 | 3848 | 25 | [test.md](artifacts/wasm-footprint/test.md) |
| `root` | bundle-canister | 3327471 | 3112916 | 214555 | 950289 | 854715 | N/A | 5618 | 5618 | 45 | [root.md](artifacts/wasm-footprint/root.md) |

## Risk Score

Risk Score: **4 / 10**

Interpretation: this is a wasm drift score, not a correctness score. The main drivers in Canic are the shared runtime floor across leaf canisters and the special-case bundle behavior in `root`.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo build --target wasm32-unknown-unknown ... && dfx build ...` | PASS | built and cached raw/shrunk artifacts for cargo/dfx release builds |
| `ic-wasm <artifact> info` | PASS | structure snapshots captured for built and shrunk artifacts |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | twiggy artifacts captured for each canister in scope |
| `baseline size-metrics.tsv comparison` | BLOCKED | first run of day; no baseline comparison available |

## Follow-up Actions

1. Owner boundary: `shared runtime baseline`
   Action: compare `minimal` retained hotspots against one feature canister in the next run and treat overlapping drivers as shared-cost reduction candidates.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-31/wasm-footprint.md`
2. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-31/wasm-footprint.md`

## Report Files

- [wasm-footprint.md](./wasm-footprint.md)
- [size-summary.md](artifacts/wasm-footprint/size-summary.md)
- [size-report.json](artifacts/wasm-footprint/size-report.json)
- [app.md](artifacts/wasm-footprint/app.md)
- [minimal.md](artifacts/wasm-footprint/minimal.md)
- [user_hub.md](artifacts/wasm-footprint/user_hub.md)
- [user_shard.md](artifacts/wasm-footprint/user_shard.md)
- [scale_hub.md](artifacts/wasm-footprint/scale_hub.md)
- [scale.md](artifacts/wasm-footprint/scale.md)
- [shard_hub.md](artifacts/wasm-footprint/shard_hub.md)
- [shard.md](artifacts/wasm-footprint/shard.md)
- [test.md](artifacts/wasm-footprint/test.md)
- [root.md](artifacts/wasm-footprint/root.md)
