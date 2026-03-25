# Wasm Footprint Audit - 2026-03-25

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `5b27de05`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-25T11:48:12Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `wasm-release`
- Target canisters in scope: `app` `blank` `user_hub` `user_shard` `scale_hub` `scale` `shard_hub` `shard` `test` `root` 
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `dfx`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/wasm-release/raw/` and shrunk artifacts under `artifacts/wasm-size/wasm-release/shrunk/` were recorded for `app` `blank` `user_hub` `user_shard` `scale_hub` `scale` `shard_hub` `shard` `test` `root` . |
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
| `app` | leaf-canister | `table[0]` | 1728873 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint/app.md) |
| `blank` | leaf-canister | `table[0]` | 1742740 | shared-runtime floor; use this to judge workspace baseline pressure | [blank.md](artifacts/wasm-footprint/blank.md) |
| `user_hub` | leaf-canister | `table[0]` | 1805097 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1752170 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1747876 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale` | leaf-canister | `table[0]` | 1728873 | largest retained symbol from raw-built twiggy analysis | [scale.md](artifacts/wasm-footprint/scale.md) |
| `shard_hub` | leaf-canister | `table[0]` | 1807840 | largest retained symbol from raw-built twiggy analysis | [shard_hub.md](artifacts/wasm-footprint/shard_hub.md) |
| `shard` | leaf-canister | `table[0]` | 1728873 | largest retained symbol from raw-built twiggy analysis | [shard.md](artifacts/wasm-footprint/shard.md) |
| `test` | leaf-canister | `table[0]` | 1733351 | largest retained symbol from raw-built twiggy analysis | [test.md](artifacts/wasm-footprint/test.md) |
| `root` | bundle-canister | `data[0]` | 8637091 | bundle-canister outlier; embeds child .wasm.gz artifacts and must not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint/root.md) |

## Dependency Fan-In Pressure

- `blank` remains the shared-runtime floor. If `blank` stays close to feature canisters, size pressure is coming from shared crates rather than role-specific logic.
- `root` is always interpreted as a bundle canister because it embeds child `.wasm.gz` artifacts during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Blank floor close to feature canisters | WARN | `blank` shrunk wasm = 3231312, `app` shrunk wasm = 3214731. |
| Root bundle outlier | WARN | `root` shrunk wasm = 11267613. |
| Shrink delta unexpectedly low | WARN | `blank` shrink delta = 0 bytes. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 3214731 | 3214731 | 0 | 922237 | 922237 | N/A | 5451 | 5451 | 35 | [app.md](artifacts/wasm-footprint/app.md) |
| `blank` | leaf-canister | 3231312 | 3231312 | 0 | 929185 | 929185 | N/A | 5482 | 5482 | 36 | [blank.md](artifacts/wasm-footprint/blank.md) |
| `user_hub` | leaf-canister | 3295124 | 3295124 | 0 | 955472 | 955472 | N/A | 5618 | 5618 | 36 | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | 3244688 | 3244688 | 0 | 933616 | 933616 | N/A | 5502 | 5502 | 37 | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | 3239196 | 3239196 | 0 | 931578 | 931578 | N/A | 5494 | 5494 | 37 | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale` | leaf-canister | 3214731 | 3214731 | 0 | 921951 | 921951 | N/A | 5451 | 5451 | 35 | [scale.md](artifacts/wasm-footprint/scale.md) |
| `shard_hub` | leaf-canister | 3299910 | 3299910 | 0 | 955333 | 955333 | N/A | 5625 | 5625 | 37 | [shard_hub.md](artifacts/wasm-footprint/shard_hub.md) |
| `shard` | leaf-canister | 3214731 | 3214731 | 0 | 921652 | 921652 | N/A | 5451 | 5451 | 35 | [shard.md](artifacts/wasm-footprint/shard.md) |
| `test` | leaf-canister | 3224575 | 3224575 | 0 | 924153 | 924153 | N/A | 5478 | 5478 | 37 | [test.md](artifacts/wasm-footprint/test.md) |
| `root` | bundle-canister | 11893405 | 11267613 | 625792 | 9360373 | 8725891 | N/A | 5883 | 5883 | 45 | [root.md](artifacts/wasm-footprint/root.md) |

## Risk Score

Risk Score: **6 / 10**

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
   Action: compare `blank` retained hotspots against one feature canister in the next run and treat overlapping drivers as shared-cost reduction candidates.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/wasm-footprint.md`
2. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-03/2026-03-25/wasm-footprint.md`

## Report Files

- [wasm-footprint.md](./wasm-footprint.md)
- [size-summary.md](artifacts/wasm-footprint/size-summary.md)
- [size-report.json](artifacts/wasm-footprint/size-report.json)
- [app.md](artifacts/wasm-footprint/app.md)
- [blank.md](artifacts/wasm-footprint/blank.md)
- [user_hub.md](artifacts/wasm-footprint/user_hub.md)
- [user_shard.md](artifacts/wasm-footprint/user_shard.md)
- [scale_hub.md](artifacts/wasm-footprint/scale_hub.md)
- [scale.md](artifacts/wasm-footprint/scale.md)
- [shard_hub.md](artifacts/wasm-footprint/shard_hub.md)
- [shard.md](artifacts/wasm-footprint/shard.md)
- [test.md](artifacts/wasm-footprint/test.md)
- [root.md](artifacts/wasm-footprint/root.md)
