# Wasm Footprint Audit - 2026-04-03

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `9920eb13`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-03T17:06:55Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `release`
- Target canisters in scope: `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `test` `root` 
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `dfx`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/release/raw/` and shrunk artifacts under `artifacts/wasm-size/release/shrunk/` were recorded for `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `test` `root` . |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint/size-report.json) plus per-canister `*.size-report.json` files. |
| Twiggy top captured | PASS | `*.twiggy-top.txt` and `*.twiggy-top.csv` emitted for each canister when `twiggy` is available. |
| Twiggy dominators captured | PASS | `*.twiggy-dominators.txt` emitted for each canister when `twiggy` is available. |
| Twiggy monos captured | PASS | `*.twiggy-monos.txt` emitted for each canister when `twiggy` is available. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint`; baseline path resolves to `N/A`. |
| Budget surface recorded for the current reference roles | PASS | Checked-in budget table loaded from `docs/audits/recurring/system/wasm-budgets.tsv`. |
| Size deltas versus baseline recorded when baseline exists | PARTIAL | First run of day; baseline deltas are \`N/A\`. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- First run of day for `wasm-footprint`; this report establishes the daily baseline.
- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 1040642 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint/app.md) |
| `minimal` | leaf-canister | `table[0]` | 1040642 | shared-runtime floor; use this to judge workspace baseline pressure | [minimal.md](artifacts/wasm-footprint/minimal.md) |
| `user_hub` | leaf-canister | `table[0]` | 1198316 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1140797 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1096637 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale` | leaf-canister | `table[0]` | 1040642 | largest retained symbol from raw-built twiggy analysis | [scale.md](artifacts/wasm-footprint/scale.md) |
| `test` | leaf-canister | `table[0]` | 1110308 | largest retained symbol from raw-built twiggy analysis | [test.md](artifacts/wasm-footprint/test.md) |
| `root` | bundle-canister | `table[0]` | 2528642 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint/root.md) |

## Budget Snapshot

| Canister | Shrunk wasm | Budget | Budget delta | Status | Note |
| --- | ---: | ---: | ---: | --- | --- |
| `app` | 1767346 | 1950000 | -182654 | OK | reference application leaf |
| `minimal` | 1767349 | 1950000 | -182651 | OK | shared runtime floor |
| `user_hub` | 1924204 | 2100000 | -175796 | OK | user coordinator leaf |
| `user_shard` | 1871639 | 2050000 | -178361 | OK | user shard leaf |
| `scale_hub` | 1825842 | 2000000 | -174158 | OK | scaling coordinator leaf |
| `scale` | 1767349 | 1950000 | -182651 | OK | scaling worker leaf |
| `test` | 1840757 | 2000000 | -159243 | OK | test helper leaf |
| `root` | 3816745 | 3200000 | +616745 | OVER | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `minimal` remains the shared-runtime floor. If `minimal` stays close to feature canisters, size pressure is coming from shared crates rather than role-specific logic.
- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Minimal floor close to feature canisters | WARN | `minimal` shrunk wasm = 1767349, `app` shrunk wasm = 1767346. |
| Root control-plane outlier | WARN | `root` shrunk wasm = 3816745. |
| Shrink delta unexpectedly low | OK | `minimal` shrink delta = 122431 bytes. |
| Budget overruns in current scope | WARN | 1 canister(s) currently exceed the checked-in shrunk-wasm budget table. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Budget | Budget delta | Budget status | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 1889780 | 1767346 | 122434 | 1950000 | -182654 | OK | 538255 | 499430 | N/A | 3525 | 3525 | 23 | [app.md](artifacts/wasm-footprint/app.md) |
| `minimal` | leaf-canister | 1889780 | 1767349 | 122431 | 1950000 | -182651 | OK | 537927 | 499418 | N/A | 3525 | 3525 | 23 | [minimal.md](artifacts/wasm-footprint/minimal.md) |
| `user_hub` | leaf-canister | 2057089 | 1924204 | 132885 | 2100000 | -175796 | OK | 594525 | 547980 | N/A | 3848 | 3848 | 27 | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | 2002454 | 1871639 | 130815 | 2050000 | -178361 | OK | 578496 | 536231 | N/A | 3695 | 3695 | 27 | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | 1952270 | 1825842 | 126428 | 2000000 | -174158 | OK | 559689 | 517081 | N/A | 3634 | 3634 | 26 | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale` | leaf-canister | 1889780 | 1767349 | 122431 | 1950000 | -182651 | OK | 537876 | 499433 | N/A | 3525 | 3525 | 23 | [scale.md](artifacts/wasm-footprint/scale.md) |
| `test` | leaf-canister | 1969211 | 1840757 | 128454 | 2000000 | -159243 | OK | 566339 | 524442 | N/A | 3641 | 3641 | 26 | [test.md](artifacts/wasm-footprint/test.md) |
| `root` | bundle-canister | 3459869 | 3816745 | -356876 | 3200000 | +616745 | OVER | 990644 | 1467212 | N/A | 5840 | 5852 | 48 | [root.md](artifacts/wasm-footprint/root.md) |

## Risk Score

Risk Score: **4 / 10**

Interpretation: this is a wasm drift score, not a correctness score. The main drivers in Canic are the shared runtime floor across leaf canisters and the special-case bundle behavior in `root`.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `checked-in wasm budget table` | PASS | loaded shrunk-wasm budgets from `docs/audits/recurring/system/wasm-budgets.tsv` |
| `cargo build --target wasm32-unknown-unknown ... && dfx build ...` | PASS | built and cached raw/shrunk artifacts for cargo/dfx release builds |
| `ic-wasm <artifact> info` | PASS | structure snapshots captured for built and shrunk artifacts |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | twiggy artifacts captured for each canister in scope |
| `baseline size-metrics.tsv comparison` | BLOCKED | first run of day; no baseline comparison available |

## Follow-up Actions

1. Owner boundary: `wasm budget discipline`
   Action: investigate the current budget overruns first and decide whether the bytes should come down or the checked-in budget table should move with an explicit reason.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-03/wasm-footprint.md`
2. Owner boundary: `shared runtime baseline`
   Action: compare `minimal` retained hotspots against one feature canister in the next run and treat overlapping drivers as shared-cost reduction candidates.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-03/wasm-footprint.md`
3. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-03/wasm-footprint.md`

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
- [test.md](artifacts/wasm-footprint/test.md)
- [root.md](artifacts/wasm-footprint/root.md)
