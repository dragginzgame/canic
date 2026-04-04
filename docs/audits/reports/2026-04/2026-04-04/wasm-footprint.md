# Wasm Footprint Audit - 2026-04-04

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `dd709c04`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-04T08:59:44Z`
- Branch: `main`
- Worktree: `clean`
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
| `app` | leaf-canister | `table[0]` | 1041670 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint/app.md) |
| `minimal` | leaf-canister | `table[0]` | 1041670 | shared-runtime floor; use this to judge workspace baseline pressure | [minimal.md](artifacts/wasm-footprint/minimal.md) |
| `user_hub` | leaf-canister | `table[0]` | 1199724 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1141820 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1097831 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale` | leaf-canister | `table[0]` | 1056082 | largest retained symbol from raw-built twiggy analysis | [scale.md](artifacts/wasm-footprint/scale.md) |
| `test` | leaf-canister | `table[0]` | 1111327 | largest retained symbol from raw-built twiggy analysis | [test.md](artifacts/wasm-footprint/test.md) |
| `root` | bundle-canister | `table[0]` | 2531167 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint/root.md) |

## Budget Snapshot

| Canister | Shrunk wasm | Budget | Budget delta | Status | Note |
| --- | ---: | ---: | ---: | --- | --- |
| `app` | 1764427 | 1950000 | -185573 | OK | reference application leaf |
| `minimal` | 1764427 | 1950000 | -185573 | OK | shared runtime floor |
| `user_hub` | 1921521 | 2100000 | -178479 | OK | user coordinator leaf |
| `user_shard` | 1868555 | 2050000 | -181445 | OK | user shard leaf |
| `scale_hub` | 1822952 | 2000000 | -177048 | OK | scaling coordinator leaf |
| `scale` | 1779874 | 1950000 | -170126 | OK | scaling worker leaf |
| `test` | 1837700 | 2000000 | -162300 | OK | test helper leaf |
| `root` | 3814358 | 3200000 | +614358 | OVER | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `minimal` remains the shared-runtime floor. If `minimal` stays close to feature canisters, size pressure is coming from shared crates rather than role-specific logic.
- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Minimal floor close to feature canisters | WARN | `minimal` shrunk wasm = 1764427, `app` shrunk wasm = 1764427. |
| Root control-plane outlier | WARN | `root` shrunk wasm = 3814358. |
| Shrink delta unexpectedly low | OK | `minimal` shrink delta = 122147 bytes. |
| Budget overruns in current scope | WARN | 1 canister(s) currently exceed the checked-in shrunk-wasm budget table. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Budget | Budget delta | Budget status | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 1886574 | 1764427 | 122147 | 1950000 | -185573 | OK | 537082 | 498803 | N/A | 3525 | 3525 | 22 | [app.md](artifacts/wasm-footprint/app.md) |
| `minimal` | leaf-canister | 1886574 | 1764427 | 122147 | 1950000 | -185573 | OK | 537587 | 498796 | N/A | 3525 | 3525 | 22 | [minimal.md](artifacts/wasm-footprint/minimal.md) |
| `user_hub` | leaf-canister | 2054135 | 1921521 | 132614 | 2100000 | -178479 | OK | 593905 | 546399 | N/A | 3848 | 3848 | 26 | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | 1999115 | 1868555 | 130560 | 2050000 | -181445 | OK | 576388 | 535646 | N/A | 3695 | 3695 | 26 | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | 1949102 | 1822952 | 126150 | 2000000 | -177048 | OK | 558750 | 515058 | N/A | 3634 | 3634 | 25 | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale` | leaf-canister | 1902939 | 1779874 | 123065 | 1950000 | -170126 | OK | 543673 | 503681 | N/A | 3559 | 3559 | 23 | [scale.md](artifacts/wasm-footprint/scale.md) |
| `test` | leaf-canister | 1965868 | 1837700 | 128168 | 2000000 | -162300 | OK | 565392 | 523349 | N/A | 3641 | 3641 | 25 | [test.md](artifacts/wasm-footprint/test.md) |
| `root` | bundle-canister | 3458288 | 3814358 | -356070 | 3200000 | +614358 | OVER | 991415 | 1466501 | N/A | 5845 | 5857 | 47 | [root.md](artifacts/wasm-footprint/root.md) |

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
   Target report date/run: `docs/audits/reports/2026-04/2026-04-04/wasm-footprint.md`
2. Owner boundary: `shared runtime baseline`
   Action: compare `minimal` retained hotspots against one feature canister in the next run and treat overlapping drivers as shared-cost reduction candidates.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-04/wasm-footprint.md`
3. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-04/wasm-footprint.md`

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
