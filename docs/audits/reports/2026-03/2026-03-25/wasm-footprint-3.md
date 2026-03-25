# Wasm Footprint Audit - 2026-03-25

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-25/wasm-footprint.md`
- Code snapshot identifier: `799d5a29`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-25T13:34:13Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `wasm-release`
- Target canisters in scope: `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `shard_hub` `shard` `test` `root` 
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `dfx`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/wasm-release/raw/` and shrunk artifacts under `artifacts/wasm-size/wasm-release/shrunk/` were recorded for `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `shard_hub` `shard` `test` `root` . |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint-3/size-report.json) plus per-canister `*.size-report.json` files. |
| Twiggy top captured | PASS | `*.twiggy-top.txt` and `*.twiggy-top.csv` emitted for each canister when `twiggy` is available. |
| Twiggy dominators captured | PASS | `*.twiggy-dominators.txt` emitted for each canister when `twiggy` is available. |
| Twiggy monos captured | PASS | `*.twiggy-monos.txt` emitted for each canister when `twiggy` is available. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint-3`; baseline path resolves to `docs/audits/reports/2026-03/2026-03-25/wasm-footprint.md`. |
| Size deltas versus baseline recorded when baseline exists | PASS | Baseline deltas were calculated from \`docs/audits/reports/2026-03/2026-03-25/wasm-footprint.md\`. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- Same-day rerun against baseline `docs/audits/reports/2026-03/2026-03-25/wasm-footprint.md`.
- Per-canister baseline deltas in the snapshot table compare current shrunk wasm bytes to the baseline run.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 1615295 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint-3/app.md) |
| `minimal` | leaf-canister | `table[0]` | 1615246 | shared-runtime floor; use this to judge workspace baseline pressure | [minimal.md](artifacts/wasm-footprint-3/minimal.md) |
| `user_hub` | leaf-canister | `table[0]` | 1759867 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint-3/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1656110 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint-3/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1662781 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint-3/scale_hub.md) |
| `scale` | leaf-canister | `table[0]` | 1615246 | largest retained symbol from raw-built twiggy analysis | [scale.md](artifacts/wasm-footprint-3/scale.md) |
| `shard_hub` | leaf-canister | `table[0]` | 1762667 | largest retained symbol from raw-built twiggy analysis | [shard_hub.md](artifacts/wasm-footprint-3/shard_hub.md) |
| `shard` | leaf-canister | `table[0]` | 1615295 | largest retained symbol from raw-built twiggy analysis | [shard.md](artifacts/wasm-footprint-3/shard.md) |
| `test` | leaf-canister | `table[0]` | 1634931 | largest retained symbol from raw-built twiggy analysis | [test.md](artifacts/wasm-footprint-3/test.md) |
| `root` | bundle-canister | `data[0]` | 8408483 | bundle-canister outlier; embeds child .wasm.gz artifacts and must not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint-3/root.md) |

## Dependency Fan-In Pressure

- `minimal` remains the shared-runtime floor. If `minimal` stays close to feature canisters, size pressure is coming from shared crates rather than role-specific logic.
- `root` is always interpreted as a bundle canister because it embeds child `.wasm.gz` artifacts during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Minimal floor close to feature canisters | WARN | `minimal` shrunk wasm = 3087652, `app` shrunk wasm = 3087701. |
| Root bundle outlier | WARN | `root` shrunk wasm = 10965892. |
| Shrink delta unexpectedly low | WARN | `minimal` shrink delta = 0 bytes. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 3087701 | 3087701 | 0 | 891594 | 891594 | -127030 | 5271 | 5271 | 30 | [app.md](artifacts/wasm-footprint-3/app.md) |
| `minimal` | leaf-canister | 3087652 | 3087652 | 0 | 891850 | 891850 | N/A | 5270 | 5270 | 30 | [minimal.md](artifacts/wasm-footprint-3/minimal.md) |
| `user_hub` | leaf-canister | 3245634 | 3245634 | 0 | 942227 | 942227 | -49490 | 5552 | 5552 | 33 | [user_hub.md](artifacts/wasm-footprint-3/user_hub.md) |
| `user_shard` | leaf-canister | 3140010 | 3140010 | 0 | 908151 | 908151 | -104678 | 5350 | 5350 | 34 | [user_shard.md](artifacts/wasm-footprint-3/user_shard.md) |
| `scale_hub` | leaf-canister | 3145726 | 3145726 | 0 | 908426 | 908426 | -93470 | 5360 | 5360 | 33 | [scale_hub.md](artifacts/wasm-footprint-3/scale_hub.md) |
| `scale` | leaf-canister | 3087652 | 3087652 | 0 | 891187 | 891187 | -127079 | 5270 | 5270 | 30 | [scale.md](artifacts/wasm-footprint-3/scale.md) |
| `shard_hub` | leaf-canister | 3250477 | 3250477 | 0 | 942280 | 942280 | -49433 | 5559 | 5559 | 34 | [shard_hub.md](artifacts/wasm-footprint-3/shard_hub.md) |
| `shard` | leaf-canister | 3087701 | 3087701 | 0 | 892067 | 892067 | -127030 | 5271 | 5271 | 30 | [shard.md](artifacts/wasm-footprint-3/shard.md) |
| `test` | leaf-canister | 3115949 | 3115949 | 0 | 900061 | 900061 | -108626 | 5316 | 5316 | 33 | [test.md](artifacts/wasm-footprint-3/test.md) |
| `root` | bundle-canister | 11566724 | 10965892 | 600832 | 9109144 | 8502052 | -301721 | 5736 | 5736 | 42 | [root.md](artifacts/wasm-footprint-3/root.md) |

## Risk Score

Risk Score: **6 / 10**

Interpretation: this is a wasm drift score, not a correctness score. The main drivers in Canic are the shared runtime floor across leaf canisters and the special-case bundle behavior in `root`.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo build --target wasm32-unknown-unknown ... && dfx build ...` | PASS | built and cached raw/shrunk artifacts for cargo/dfx release builds |
| `ic-wasm <artifact> info` | PASS | structure snapshots captured for built and shrunk artifacts |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | twiggy artifacts captured for each canister in scope |
| `baseline size-metrics.tsv comparison` | PASS | baseline deltas calculated from `docs/audits/reports/2026-03/2026-03-25/artifacts/wasm-footprint/size-metrics.tsv` |

## Follow-up Actions

1. Owner boundary: `shared runtime baseline`
   Action: compare `minimal` retained hotspots against one feature canister in the next run and treat overlapping drivers as shared-cost reduction candidates.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/wasm-footprint.md`
2. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/wasm-footprint.md`

## Report Files

- [wasm-footprint-3.md](./wasm-footprint-3.md)
- [size-summary.md](artifacts/wasm-footprint-3/size-summary.md)
- [size-report.json](artifacts/wasm-footprint-3/size-report.json)
- [app.md](artifacts/wasm-footprint-3/app.md)
- [minimal.md](artifacts/wasm-footprint-3/minimal.md)
- [user_hub.md](artifacts/wasm-footprint-3/user_hub.md)
- [user_shard.md](artifacts/wasm-footprint-3/user_shard.md)
- [scale_hub.md](artifacts/wasm-footprint-3/scale_hub.md)
- [scale.md](artifacts/wasm-footprint-3/scale.md)
- [shard_hub.md](artifacts/wasm-footprint-3/shard_hub.md)
- [shard.md](artifacts/wasm-footprint-3/shard.md)
- [test.md](artifacts/wasm-footprint-3/test.md)
- [root.md](artifacts/wasm-footprint-3/root.md)
