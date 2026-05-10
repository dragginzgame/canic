# Wasm Footprint Audit - 2026-05-09

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `ed6bfe9c`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-09T15:53:10Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `release`
- Target canisters in scope: `app` `user_hub` `user_shard` `minimal` `scale_hub` `scale` `root` 
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `icp`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/release/raw/` and shrunk artifacts under `artifacts/wasm-size/release/shrunk/` were recorded for `app` `user_hub` `user_shard` `minimal` `scale_hub` `scale` `root` . |
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
| `app` | leaf-canister | `table[0]` | 972039 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint/app.md) |
| `user_hub` | leaf-canister | `table[0]` | 1129543 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1054248 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `minimal` | leaf-canister | `table[0]` | 972039 | shared-runtime floor; use this to judge workspace baseline pressure | [minimal.md](artifacts/wasm-footprint/minimal.md) |
| `scale_hub` | leaf-canister | `table[0]` | 998878 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale` | leaf-canister | `table[0]` | 986070 | largest retained symbol from raw-built twiggy analysis | [scale.md](artifacts/wasm-footprint/scale.md) |
| `root` | bundle-canister | `table[0]` | 2279906 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint/root.md) |

## Current Size Snapshot

| Canister | Shrunk wasm | Shrink delta | Baseline delta | Note |
| --- | ---: | ---: | --- | --- |
| `app` | 1683465 | 122277 | N/A | role-specific leaf |
| `user_hub` | 1840534 | 133805 | N/A | role-specific leaf |
| `user_shard` | 1765279 | 129439 | N/A | role-specific leaf |
| `minimal` | 1683461 | 122278 | N/A | shared runtime floor |
| `scale_hub` | 1712596 | 124848 | N/A | role-specific leaf |
| `scale` | 1698254 | 123327 | N/A | role-specific leaf |
| `root` | 3588379 | 245836 | N/A | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `minimal` remains the shared-runtime floor. If `minimal` stays close to feature canisters, size pressure is coming from shared crates rather than role-specific logic.
- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Minimal floor close to feature canisters | WARN | `minimal` shrunk wasm = 1683461, `app` shrunk wasm = 1683465. |
| Root control-plane outlier | WARN | `root` shrunk wasm = 3588379. |
| Shrink delta unexpectedly low | OK | `minimal` shrink delta = 122278 bytes. |
| Positive same-day baseline drift in current scope | N/A | First run of day; baseline drift is not comparable yet. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 1805742 | 1683465 | 122277 | 533925 | 495861 | N/A | 3552 | 3552 | 18 | [app.md](artifacts/wasm-footprint/app.md) |
| `user_hub` | leaf-canister | 1974339 | 1840534 | 133805 | 588667 | 543659 | N/A | 3893 | 3893 | 22 | [user_hub.md](artifacts/wasm-footprint/user_hub.md) |
| `user_shard` | leaf-canister | 1894718 | 1765279 | 129439 | 562852 | 522937 | N/A | 3691 | 3691 | 20 | [user_shard.md](artifacts/wasm-footprint/user_shard.md) |
| `minimal` | leaf-canister | 1805739 | 1683461 | 122278 | 533958 | 495876 | N/A | 3552 | 3552 | 18 | [minimal.md](artifacts/wasm-footprint/minimal.md) |
| `scale_hub` | leaf-canister | 1837444 | 1712596 | 124848 | 544930 | 506743 | N/A | 3622 | 3622 | 21 | [scale_hub.md](artifacts/wasm-footprint/scale_hub.md) |
| `scale` | leaf-canister | 1821581 | 1698254 | 123327 | 539840 | 501381 | N/A | 3595 | 3595 | 19 | [scale.md](artifacts/wasm-footprint/scale.md) |
| `root` | bundle-canister | 3834215 | 3588379 | 245836 | 1538013 | 1421284 | N/A | 6001 | 6001 | 42 | [root.md](artifacts/wasm-footprint/root.md) |

## Risk Score

Risk Score: **4 / 10**

Interpretation: this is a wasm drift score, not a correctness score. The main drivers in Canic are the shared runtime floor across leaf canisters and the special-case bundle behavior in `root`.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo build --target wasm32-unknown-unknown ... && canic build ...` | PASS | built and cached raw/shrunk artifacts for cargo/icp release builds |
| `ic-wasm <artifact> info` | PASS | structure snapshots captured for built and shrunk artifacts |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | twiggy artifacts captured for each canister in scope |
| `baseline comparison` | BLOCKED | first run of day; no baseline comparison available |

## Follow-up Actions

1. Owner boundary: `shared runtime baseline`
   Action: compare `minimal` retained hotspots against one feature canister in the next run and treat overlapping drivers as shared-cost reduction candidates.
   Target report date/run: `docs/audits/reports/2026-05/2026-05-09/wasm-footprint.md`
2. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-05/2026-05-09/wasm-footprint.md`

## Report Files

- [wasm-footprint.md](./wasm-footprint.md)
- [size-summary.md](artifacts/wasm-footprint/size-summary.md)
- [size-report.json](artifacts/wasm-footprint/size-report.json)
- [app.md](artifacts/wasm-footprint/app.md)
- [user_hub.md](artifacts/wasm-footprint/user_hub.md)
- [user_shard.md](artifacts/wasm-footprint/user_shard.md)
- [minimal.md](artifacts/wasm-footprint/minimal.md)
- [scale_hub.md](artifacts/wasm-footprint/scale_hub.md)
- [scale.md](artifacts/wasm-footprint/scale.md)
- [root.md](artifacts/wasm-footprint/root.md)
