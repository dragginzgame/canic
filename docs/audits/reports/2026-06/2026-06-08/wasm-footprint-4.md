# Wasm Footprint Audit - 2026-06-08

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `docs/audits/reports/2026-06/2026-06-08/wasm-footprint.md`
- Code snapshot identifier: `f1e9c161`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-06-08T20:30:14Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `release`
- Target canisters in scope: `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root`
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `icp`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/release/raw/` and shrunk artifacts under `artifacts/wasm-size/release/shrunk/` were recorded for `app` `user_hub` `user_shard` `scale_hub` `scale_replica` `root` . |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint-4/size-report.json) plus per-canister `*.size-report.json` files. |
| Twiggy top captured | PASS | `*.twiggy-top.txt` and `*.twiggy-top.csv` emitted for each canister when `twiggy` is available. |
| Twiggy dominators captured | PASS | `*.twiggy-dominators.txt` emitted for each canister when `twiggy` is available. |
| Twiggy monos captured | PASS | `*.twiggy-monos.txt` emitted for each canister when `twiggy` is available. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint-4`; baseline path resolves to `docs/audits/reports/2026-06/2026-06-08/wasm-footprint.md`. |
| Size deltas versus baseline recorded when baseline exists | PASS | Baseline deltas were calculated from \`docs/audits/reports/2026-06/2026-06-08/wasm-footprint.md\`. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- Same-day rerun against baseline `docs/audits/reports/2026-06/2026-06-08/wasm-footprint.md`.
- Per-canister baseline deltas in the snapshot table compare current shrunk wasm bytes to the baseline run.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 1549072 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint-4/app.md) |
| `user_hub` | leaf-canister | `table[0]` | 1705412 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint-4/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1639858 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint-4/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1576023 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint-4/scale_hub.md) |
| `scale_replica` | leaf-canister | `table[0]` | 1563742 | largest retained symbol from raw-built twiggy analysis | [scale_replica.md](artifacts/wasm-footprint-4/scale_replica.md) |
| `root` | bundle-canister | `table[0]` | 2943155 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint-4/root.md) |

## Current Size Snapshot

| Canister | Shrunk wasm | Shrink delta | Baseline delta | Note |
| --- | ---: | ---: | --- | --- |
| `app` | 2676206 | 181107 | -43736 | role-specific leaf |
| `user_hub` | 2841745 | 192181 | -43632 | role-specific leaf |
| `user_shard` | 2770022 | 187296 | -43593 | role-specific leaf |
| `scale_hub` | 2714203 | 183443 | -43635 | role-specific leaf |
| `scale_replica` | 2691984 | 182015 | -43627 | role-specific leaf |
| `root` | 4821382 | 277662 | -51648 | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.
- No dedicated `minimal` shared-runtime baseline is attached in the current audited scope; treat repeated hotspots across leaf canisters as shared fan-in pressure until an explicit audit baseline role is attached.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Root control-plane outlier | WARN | `root` shrunk wasm = 4821382. |
| Positive same-day baseline drift in current scope | OK | 0 canister(s) grew versus the selected same-day baseline. |
| Dedicated minimal baseline present | N/A | No `minimal` baseline role is attached in the current audited scope. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 2857313 | 2676206 | 181107 | 768661 | 692885 | -43736 | 5144 | 5144 | 18 | [app.md](artifacts/wasm-footprint-4/app.md) |
| `user_hub` | leaf-canister | 3033926 | 2841745 | 192181 | 822952 | 743483 | -43632 | 5487 | 5487 | 22 | [user_hub.md](artifacts/wasm-footprint-4/user_hub.md) |
| `user_shard` | leaf-canister | 2957318 | 2770022 | 187296 | 802785 | 724928 | -43593 | 5286 | 5286 | 20 | [user_shard.md](artifacts/wasm-footprint-4/user_shard.md) |
| `scale_hub` | leaf-canister | 2897646 | 2714203 | 183443 | 783011 | 704833 | -43635 | 5218 | 5218 | 21 | [scale_hub.md](artifacts/wasm-footprint-4/scale_hub.md) |
| `scale_replica` | leaf-canister | 2873999 | 2691984 | 182015 | 775514 | 698835 | -43627 | 5186 | 5186 | 19 | [scale_replica.md](artifacts/wasm-footprint-4/scale_replica.md) |
| `root` | bundle-canister | 5099044 | 4821382 | 277662 | 1999629 | 1869404 | -51648 | 7470 | 7470 | 37 | [root.md](artifacts/wasm-footprint-4/root.md) |

## Risk Score

Risk Score: **4 / 10**

Interpretation: this is a wasm drift score, not a correctness score. The main drivers in Canic are the shared runtime floor across leaf canisters and the special-case bundle behavior in `root`.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo build --target wasm32-unknown-unknown ... && cargo run -p canic-host --example build_artifact ...` | PASS | built and cached raw/shrunk artifacts for cargo/icp release builds |
| `ic-wasm <artifact> info` | PASS | structure snapshots captured for built and shrunk artifacts |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | twiggy artifacts captured for each canister in scope |
| `baseline size-metrics.tsv comparison` | PASS | baseline deltas calculated from `docs/audits/reports/2026-06/2026-06-08/artifacts/wasm-footprint/size-metrics.tsv` |

## Follow-up Actions

1. Owner boundary: `shared runtime baseline`
   Action: decide whether a dedicated audit baseline role should be attached, or keep using repeated leaf hotspots as the shared-runtime signal.
   Target report date/run: `docs/audits/reports/2026-06/2026-06-08/wasm-footprint.md`
2. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-06/2026-06-08/wasm-footprint.md`

## Report Files

- [wasm-footprint-4.md](./wasm-footprint-4.md)
- [size-summary.md](artifacts/wasm-footprint-4/size-summary.md)
- [size-report.json](artifacts/wasm-footprint-4/size-report.json)
- [app.md](artifacts/wasm-footprint-4/app.md)
- [user_hub.md](artifacts/wasm-footprint-4/user_hub.md)
- [user_shard.md](artifacts/wasm-footprint-4/user_shard.md)
- [scale_hub.md](artifacts/wasm-footprint-4/scale_hub.md)
- [scale_replica.md](artifacts/wasm-footprint-4/scale_replica.md)
- [root.md](artifacts/wasm-footprint-4/root.md)
