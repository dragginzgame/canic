# Wasm Footprint Audit - 2026-06-19

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `ef55e53c`
- Method tag/version: `Method V2`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-06-19T12:15:07Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `release`
- Target canisters in scope: `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root` 
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `icp`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/release/raw/` and shrunk artifacts under `artifacts/wasm-size/release/shrunk/` were recorded for `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root` . |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint/size-report.json) plus per-canister `*.size-report.json` files. |
| Twiggy top captured | PASS | `*.twiggy-top.txt` and `*.twiggy-top.csv` emitted for each canister when `twiggy` is available. |
| Twiggy dominators captured | PASS | `*.twiggy-dominators.txt` emitted for each canister when `twiggy` is available. |
| Twiggy monos captured | PASS | `*.twiggy-monos.txt` emitted for each canister when `twiggy` is available. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint`; baseline path resolves to `N/A`. |
| Size deltas versus baseline recorded when baseline exists | PARTIAL | First run of day; baseline deltas are `N/A`. |
| `wasm-debug` built artifacts captured | PASS | Debug raw artifacts under `artifacts/wasm-size/wasm-debug/raw/` were recorded for `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root` . |
| Debug-vs-audit size deltas recorded | PASS | Debug-vs-`release` built wasm deltas were recorded in the report and machine-readable artifacts. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- First run of day for `wasm-footprint`; this report establishes the daily baseline.
- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 1858419 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint/app.md) |
| `user_hub` | leaf-canister | `table[0]` | 2006767 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1951627 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1879168 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale_replica` | leaf-canister | `table[0]` | 1866922 | largest retained symbol from raw-built twiggy analysis | [scale_replica.md](artifacts/wasm-footprint/scale_replica.md) |
| `root` | bundle-canister | `table[0]` | 3049612 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint/root.md) |

## Current Size Snapshot

| Canister | Shrunk wasm | Shrink delta | Baseline delta | Note |
| --- | ---: | ---: | --- | --- |
| `app` | 2996944 | 200279 | N/A | role-specific leaf |
| `user_hub` | 3154906 | 210920 | N/A | role-specific leaf |
| `user_shard` | 3095441 | 205389 | N/A | role-specific leaf |
| `scale_hub` | 3028777 | 202306 | N/A | role-specific leaf |
| `scale_replica` | 3006586 | 200885 | N/A | role-specific leaf |
| `root` | 4914820 | 282135 | N/A | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.
- No dedicated `minimal` shared-runtime baseline is attached in the current audited scope; treat repeated hotspots across leaf canisters as shared fan-in pressure until an explicit audit baseline role is attached.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Root control-plane outlier | WARN | `root` shrunk wasm = 4914820. |
| Positive same-day baseline drift in current scope | N/A | First run of day; baseline drift is not comparable yet. |
| Dedicated minimal baseline present | N/A | No `minimal` baseline role is attached in the current audited scope. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 3197223 | 2996944 | 200279 | 833927 | 747698 | N/A | 5714 | 5714 | 18 | [app.md](artifacts/wasm-footprint/app.md) |
| `user_hub` | leaf-canister | 3365826 | 3154906 | 210920 | 886846 | 793838 | N/A | 6041 | 6041 | 22 | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | 3300830 | 3095441 | 205389 | 867577 | 781480 | N/A | 5928 | 5928 | 23 | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `scale_hub` | leaf-canister | 3231083 | 3028777 | 202306 | 842849 | 758221 | N/A | 5773 | 5773 | 21 | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale_replica` | leaf-canister | 3207471 | 3006586 | 200885 | 837291 | 752616 | N/A | 5742 | 5742 | 19 | [scale_replica.md](artifacts/wasm-footprint/scale_replica.md) |
| `root` | bundle-canister | 5196955 | 4914820 | 282135 | 1981001 | 1856021 | N/A | 7646 | 7646 | 39 | [root.md](artifacts/wasm-footprint/root.md) |

## Debug-vs-Audit Profile Snapshot

| Canister | wasm-debug built wasm | release built wasm | Delta | Delta percent | wasm-debug built gz |
| --- | ---: | ---: | ---: | --- | ---: |
| `app` | 90961523 | 3197223 | +87764300 | 2745.02% | 18530424 |
| `user_hub` | 94637195 | 3365826 | +91271369 | 2711.71% | 19274651 |
| `user_shard` | 92567033 | 3300830 | +89266203 | 2704.36% | 18901040 |
| `scale_hub` | 91414575 | 3231083 | +88183492 | 2729.22% | 18645735 |
| `scale_replica` | 91114361 | 3207471 | +87906890 | 2740.69% | 18572335 |
| `root` | 104617814 | 5196955 | +99420859 | 1913.06% | 22781014 |

## Risk Score

Risk Score: **4 / 10**

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
   Target report date/run: `docs/audits/reports/2026-06/2026-06-19/wasm-footprint.md`
2. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-06/2026-06-19/wasm-footprint.md`

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
