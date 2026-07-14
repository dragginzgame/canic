# Wasm Footprint Audit - 2026-07-01

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `docs/audits/reports/2026-07/2026-07-01/wasm-footprint.md`
- Code snapshot identifier: `db1fd994`
- Method tag/version: `Method V2`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-07-01T11:05:40Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `release`
- Target canisters in scope: `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root`
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `icp`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/release/raw/` and shrunk artifacts under `artifacts/wasm-size/release/shrunk/` were recorded for `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root`. |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint-2/size-report.json) and [size-metrics.tsv](artifacts/wasm-footprint-2/size-metrics.tsv) retain aggregate and baseline data. |
| Twiggy top analyzed | PASS | Offender and retained-size summaries remain in the per-canister detail reports; raw output was later pruned. |
| Twiggy dominators analyzed | PASS | Retained-size ownership was summarized in the report; raw output was later pruned. |
| Twiggy monos analyzed | PASS | Generic-bloat evidence was summarized in the report; raw output was later pruned. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint-2`; baseline path resolves to `docs/audits/reports/2026-07/2026-07-01/wasm-footprint.md`. |
| Size deltas versus baseline recorded when baseline exists | PASS | Baseline deltas were calculated from `docs/audits/reports/2026-07/2026-07-01/wasm-footprint.md`. |
| `wasm-debug` built artifacts captured | PASS | Debug raw artifacts under `artifacts/wasm-size/wasm-debug/raw/` were recorded for `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root`. |
| Debug-vs-audit size deltas recorded | PASS | Debug-vs-`release` built wasm deltas were recorded in the report and machine-readable artifacts. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- Same-day rerun against baseline `docs/audits/reports/2026-07/2026-07-01/wasm-footprint.md`.
- Per-canister baseline deltas in the snapshot table compare current shrunk wasm bytes to the baseline run.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 2410413 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint-2/app.md) |
| `user_hub` | leaf-canister | `table[0]` | 2558736 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint-2/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 2520675 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint-2/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 2431162 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint-2/scale_hub.md) |
| `scale_replica` | leaf-canister | `table[0]` | 2418916 | largest retained symbol from raw-built twiggy analysis | [scale_replica.md](artifacts/wasm-footprint-2/scale_replica.md) |
| `root` | bundle-canister | `table[0]` | 2815367 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint-2/root.md) |

## Current Size Snapshot

| Canister | Shrunk wasm | Shrink delta | Baseline delta | Note |
| --- | ---: | ---: | --- | --- |
| `app` | 3554375 | 231012 | -1983 | role-specific leaf |
| `user_hub` | 3712170 | 241667 | -2131 | role-specific leaf |
| `user_shard` | 3671154 | 235764 | -2131 | role-specific leaf |
| `scale_hub` | 3586172 | 233075 | -2039 | role-specific leaf |
| `scale_replica` | 3563881 | 231626 | -2136 | role-specific leaf |
| `root` | 5638747 | 321806 | -19315 | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister detail reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.
- No dedicated `minimal` shared-runtime baseline is attached in the current audited scope; treat repeated hotspots across leaf canisters as shared fan-in pressure until an explicit audit baseline role is attached.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Root control-plane outlier | WARN | `root` shrunk wasm = 5638747. |
| Positive same-day baseline drift in current scope | OK | 0 canister(s) grew versus the selected same-day baseline. |
| Dedicated minimal baseline present | N/A | No `minimal` baseline role is attached in the current audited scope. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 3785387 | 3554375 | 231012 | 960457 | 864792 | -1983 | 6428 | 6428 | 18 | [app.md](artifacts/wasm-footprint-2/app.md) |
| `user_hub` | leaf-canister | 3953837 | 3712170 | 241667 | 1010821 | 912301 | -2131 | 6754 | 6754 | 22 | [user_hub.md](artifacts/wasm-footprint-2/user_hub.md) |
| `user_shard` | leaf-canister | 3906918 | 3671154 | 235764 | 999791 | 908633 | -2131 | 6657 | 6657 | 23 | [user_shard.md](artifacts/wasm-footprint-2/user_shard.md) |
| `scale_hub` | leaf-canister | 3819247 | 3586172 | 233075 | 969041 | 876211 | -2039 | 6487 | 6487 | 21 | [scale_hub.md](artifacts/wasm-footprint-2/scale_hub.md) |
| `scale_replica` | leaf-canister | 3795507 | 3563881 | 231626 | 962808 | 869774 | -2136 | 6456 | 6456 | 19 | [scale_replica.md](artifacts/wasm-footprint-2/scale_replica.md) |
| `root` | bundle-canister | 5960553 | 5638747 | 321806 | 2248908 | 2097025 | -19315 | 8513 | 8513 | 40 | [root.md](artifacts/wasm-footprint-2/root.md) |

## Debug-vs-Audit Profile Snapshot

| Canister | wasm-debug built wasm | release built wasm | Delta | Delta percent | wasm-debug built gz |
| --- | ---: | ---: | ---: | --- | ---: |
| `app` | 99810306 | 3785387 | +96024919 | 2536.73% | 20417555 |
| `user_hub` | 103482652 | 3953837 | +99528815 | 2517.27% | 21177038 |
| `user_shard` | 101515067 | 3906918 | +97608149 | 2498.34% | 20825381 |
| `scale_hub` | 100284018 | 3819247 | +96464771 | 2525.75% | 20537028 |
| `scale_replica` | 99983483 | 3795507 | +96187976 | 2534.26% | 20465288 |
| `root` | 113281300 | 5960553 | +107320747 | 1800.52% | 24791177 |

## Risk Score

Risk Score: **3 / 10**

Interpretation: this is a wasm drift score, not a correctness score. The main drivers in Canic are the shared runtime floor across leaf canisters and the special-case bundle behavior in `root`.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo build --target wasm32-unknown-unknown ... && cargo run -p canic-host --example build_artifact ...` | PASS | built and cached raw/shrunk artifacts for cargo/icp release builds |
| `cargo build --target wasm32-unknown-unknown -p <package> --locked` | PASS | built and cached wasm-debug raw artifacts for profile comparison |
| `ic-wasm <artifact> info` | PASS | structure snapshots captured for built and shrunk artifacts |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | twiggy artifacts captured for each canister in scope |
| `baseline size-metrics.tsv comparison` | PASS | baseline deltas calculated from `docs/audits/reports/2026-07/2026-07-01/artifacts/wasm-footprint/size-metrics.tsv` |

## Follow-up Actions

1. Owner boundary: `shared runtime baseline`
   Action: decide whether a dedicated audit baseline role should be attached, or keep using repeated leaf hotspots as the shared-runtime signal.
   Target report date/run: `docs/audits/reports/2026-07/2026-07-01/wasm-footprint-2.md`
2. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-07/2026-07-01/wasm-footprint-2.md`

## Report Files

- [wasm-footprint-2.md](./wasm-footprint-2.md)
- [size-summary.md](artifacts/wasm-footprint-2/size-summary.md)
- [size-report.json](artifacts/wasm-footprint-2/size-report.json)
- [app.md](artifacts/wasm-footprint-2/app.md)
- [user_hub.md](artifacts/wasm-footprint-2/user_hub.md)
- [user_shard.md](artifacts/wasm-footprint-2/user_shard.md)
- [scale_hub.md](artifacts/wasm-footprint-2/scale_hub.md)
- [scale_replica.md](artifacts/wasm-footprint-2/scale_replica.md)
- [root.md](artifacts/wasm-footprint-2/root.md)
