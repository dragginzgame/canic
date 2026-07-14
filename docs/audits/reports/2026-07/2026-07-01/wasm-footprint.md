# Wasm Footprint Audit - 2026-07-01

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `db1fd994`
- Method tag/version: `Method V2`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-07-01T10:40:44Z`
- Branch: `main`
- Worktree: `clean`
- Profile: `release`
- Target canisters in scope: `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root`
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `icp`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/release/raw/` and shrunk artifacts under `artifacts/wasm-size/release/shrunk/` were recorded for `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root`. |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint/size-report.json) and [size-metrics.tsv](artifacts/wasm-footprint/size-metrics.tsv) retain aggregate and baseline data. |
| Twiggy top analyzed | PASS | Offender and retained-size summaries remain in the per-canister detail reports; raw output was later pruned. |
| Twiggy dominators analyzed | PASS | Retained-size ownership was summarized in the report; raw output was later pruned. |
| Twiggy monos analyzed | PASS | Generic-bloat evidence was summarized in the report; raw output was later pruned. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint`; baseline path resolves to `N/A`. |
| Size deltas versus baseline recorded when baseline exists | PARTIAL | First run of day; baseline deltas are `N/A`. |
| `wasm-debug` built artifacts captured | PASS | Debug raw artifacts under `artifacts/wasm-size/wasm-debug/raw/` were recorded for `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root`. |
| Debug-vs-audit size deltas recorded | PASS | Debug-vs-`release` built wasm deltas were recorded in the report and machine-readable artifacts. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- First run of day for `wasm-footprint`; this report establishes the daily baseline.
- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 2409534 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint/app.md) |
| `user_hub` | leaf-canister | `table[0]` | 2557857 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 2519796 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 2430283 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale_replica` | leaf-canister | `table[0]` | 2418037 | largest retained symbol from raw-built twiggy analysis | [scale_replica.md](artifacts/wasm-footprint/scale_replica.md) |
| `root` | bundle-canister | `table[0]` | 2816805 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint/root.md) |

## Current Size Snapshot

| Canister | Shrunk wasm | Shrink delta | Baseline delta | Note |
| --- | ---: | ---: | --- | --- |
| `app` | 3556358 | 231222 | N/A | role-specific leaf |
| `user_hub` | 3714301 | 241857 | N/A | role-specific leaf |
| `user_shard` | 3673285 | 235954 | N/A | role-specific leaf |
| `scale_hub` | 3588211 | 233229 | N/A | role-specific leaf |
| `scale_replica` | 3566017 | 231811 | N/A | role-specific leaf |
| `root` | 5658062 | 312838 | N/A | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister detail reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.
- No dedicated `minimal` shared-runtime baseline is attached in the current audited scope; treat repeated hotspots across leaf canisters as shared fan-in pressure until an explicit audit baseline role is attached.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Root control-plane outlier | WARN | `root` shrunk wasm = 5658062. |
| Positive same-day baseline drift in current scope | N/A | First run of day; baseline drift is not comparable yet. |
| Dedicated minimal baseline present | N/A | No `minimal` baseline role is attached in the current audited scope. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 3787580 | 3556358 | 231222 | 961818 | 865823 | N/A | 6430 | 6430 | 18 | [app.md](artifacts/wasm-footprint/app.md) |
| `user_hub` | leaf-canister | 3956158 | 3714301 | 241857 | 1011953 | 913661 | N/A | 6756 | 6756 | 22 | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | 3909239 | 3673285 | 235954 | 1002770 | 909876 | N/A | 6659 | 6659 | 23 | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | 3821440 | 3588211 | 233229 | 971271 | 876895 | N/A | 6489 | 6489 | 21 | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale_replica` | leaf-canister | 3797828 | 3566017 | 231811 | 964731 | 870848 | N/A | 6458 | 6458 | 19 | [scale_replica.md](artifacts/wasm-footprint/scale_replica.md) |
| `root` | bundle-canister | 5970900 | 5658062 | 312838 | 2255934 | 2114310 | N/A | 8519 | 8519 | 40 | [root.md](artifacts/wasm-footprint/root.md) |

## Debug-vs-Audit Profile Snapshot

| Canister | wasm-debug built wasm | release built wasm | Delta | Delta percent | wasm-debug built gz |
| --- | ---: | ---: | ---: | --- | ---: |
| `app` | 99830840 | 3787580 | +96043260 | 2535.74% | 20413349 |
| `user_hub` | 103478615 | 3956158 | +99522457 | 2515.63% | 21170661 |
| `user_shard` | 101529818 | 3909239 | +97620579 | 2497.18% | 20828583 |
| `scale_hub` | 100301261 | 3821440 | +96479821 | 2524.70% | 20534211 |
| `scale_replica` | 100000289 | 3797828 | +96202461 | 2533.09% | 20464507 |
| `root` | 113352116 | 5970900 | +107381216 | 1798.41% | 24849898 |

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
| `baseline comparison` | BLOCKED | first run of day; no baseline comparison available |

## Follow-up Actions

1. Owner boundary: `shared runtime baseline`
   Action: decide whether a dedicated audit baseline role should be attached, or keep using repeated leaf hotspots as the shared-runtime signal.
   Target report date/run: `docs/audits/reports/2026-07/2026-07-01/wasm-footprint.md`
2. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-07/2026-07-01/wasm-footprint.md`

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
