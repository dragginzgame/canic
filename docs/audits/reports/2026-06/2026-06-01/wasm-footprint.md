# Wasm Footprint Audit - 2026-06-01

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `8247730d`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-06-01T13:18:55Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `release`
- Target canisters in scope: `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root` 
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `icp`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/release/raw/` and shrunk artifacts under `artifacts/wasm-size/release/shrunk/` were recorded for `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root` . |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint/size-report.json) and [size-metrics.tsv](artifacts/wasm-footprint/size-metrics.tsv) retain aggregate and baseline data. |
| Twiggy top analyzed | PASS | Offender and retained-size summaries remain in the per-canister detail reports; raw output was later pruned. |
| Twiggy dominators analyzed | PASS | Retained-size ownership was summarized in the report; raw output was later pruned. |
| Twiggy monos analyzed | PASS | Generic-bloat evidence was summarized in the report; raw output was later pruned. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint`; baseline path resolves to `N/A`. |
| Size deltas versus baseline recorded when baseline exists | PARTIAL | First run of day; baseline deltas are \`N/A\`. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- First run of day for `wasm-footprint`; this report establishes the daily baseline.
- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 1041001 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint/app.md) |
| `user_hub` | leaf-canister | `table[0]` | 1198166 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1121426 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1067807 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale_replica` | leaf-canister | `table[0]` | 1055649 | largest retained symbol from raw-built twiggy analysis | [scale_replica.md](artifacts/wasm-footprint/scale_replica.md) |
| `root` | bundle-canister | `table[0]` | 2522897 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint/root.md) |

## Current Size Snapshot

| Canister | Shrunk wasm | Shrink delta | Baseline delta | Note |
| --- | ---: | ---: | --- | --- |
| `app` | 2096807 | 141191 | N/A | role-specific leaf |
| `user_hub` | 2263151 | 152251 | N/A | role-specific leaf |
| `user_shard` | 2178673 | 146545 | N/A | role-specific leaf |
| `scale_hub` | 2134711 | 143515 | N/A | role-specific leaf |
| `scale_replica` | 2112445 | 142082 | N/A | role-specific leaf |
| `root` | 4201747 | 235483 | N/A | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister detail reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.
- No dedicated `minimal` shared-runtime baseline is attached in the current audited scope; treat repeated hotspots across leaf canisters as shared fan-in pressure until an explicit audit baseline role is attached.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Root control-plane outlier | WARN | `root` shrunk wasm = 4201747. |
| Positive same-day baseline drift in current scope | N/A | First run of day; baseline drift is not comparable yet. |
| Dedicated minimal baseline present | N/A | No `minimal` baseline role is attached in the current audited scope. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 2237998 | 2096807 | 141191 | 623563 | 571491 | N/A | 4276 | 4276 | 19 | [app.md](artifacts/wasm-footprint/app.md) |
| `user_hub` | leaf-canister | 2415402 | 2263151 | 152251 | 681119 | 621205 | N/A | 4624 | 4624 | 23 | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | 2325218 | 2178673 | 146545 | 652287 | 599219 | N/A | 4403 | 4403 | 21 | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | 2278226 | 2134711 | 143515 | 635552 | 582392 | N/A | 4351 | 4351 | 22 | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale_replica` | leaf-canister | 2254527 | 2112445 | 142082 | 629477 | 576361 | N/A | 4319 | 4319 | 20 | [scale_replica.md](artifacts/wasm-footprint/scale_replica.md) |
| `root` | bundle-canister | 4437230 | 4201747 | 235483 | 1741610 | 1630812 | N/A | 6810 | 6810 | 38 | [root.md](artifacts/wasm-footprint/root.md) |

## Risk Score

Risk Score: **4 / 10**

Interpretation: this is a wasm drift score, not a correctness score. The main drivers in Canic are the shared runtime floor across leaf canisters and the special-case bundle behavior in `root`.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo build --target wasm32-unknown-unknown ... && cargo run -p canic-host --example build_artifact ...` | PASS | built and cached raw/shrunk artifacts for cargo/icp release builds |
| `ic-wasm <artifact> info` | PASS | structure snapshots captured for built and shrunk artifacts |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | twiggy artifacts captured for each canister in scope |
| `baseline comparison` | BLOCKED | first run of day; no baseline comparison available |

## Follow-up Actions

1. Owner boundary: `shared runtime baseline`
   Action: decide whether a dedicated audit baseline role should be attached, or keep using repeated leaf hotspots as the shared-runtime signal.
   Target report date/run: `docs/audits/reports/2026-06/2026-06-01/wasm-footprint.md`
2. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-06/2026-06-01/wasm-footprint.md`

## Report Files

- [wasm-footprint.md](./wasm-footprint.md)
- [size-summary.md](artifacts/wasm-footprint/size-summary.md)
- [size-report.json](artifacts/wasm-footprint/size-report.json)
- [app.md](artifacts/wasm-footprint/app.md)
- [user_hub.md](artifacts/wasm-footprint/user_hub.md)
- [user_shard.md](artifacts/wasm-footprint/user_shard.md)
- [scale_hub.md](artifacts/wasm-footprint/scale_hub.md)
- [scale_replica.md](artifacts/wasm-footprint/scale_replica.md)
- [root.md](artifacts/wasm-footprint/root.md)
