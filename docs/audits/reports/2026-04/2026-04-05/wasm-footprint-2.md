# Wasm Footprint Audit - 2026-04-05

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`
- Code snapshot identifier: `0eaaf2c4`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T11:43:59Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `release`
- Target canisters in scope: `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `test` `root` 
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `dfx`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/release/raw/` and shrunk artifacts under `artifacts/wasm-size/release/shrunk/` were recorded for `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `test` `root` . |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint-2/size-report.json) plus per-canister `*.size-report.json` files. |
| Twiggy top captured | PASS | `*.twiggy-top.txt` and `*.twiggy-top.csv` emitted for each canister when `twiggy` is available. |
| Twiggy dominators captured | PASS | `*.twiggy-dominators.txt` emitted for each canister when `twiggy` is available. |
| Twiggy monos captured | PASS | `*.twiggy-monos.txt` emitted for each canister when `twiggy` is available. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint-2`; baseline path resolves to `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`. |
| Size deltas versus baseline recorded when baseline exists | PASS | Baseline deltas were calculated from \`docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md\`. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- Same-day rerun against baseline `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`.
- Per-canister baseline deltas in the snapshot table compare current shrunk wasm bytes to the baseline run.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 989979 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint-2/app.md) |
| `minimal` | leaf-canister | `table[0]` | 977429 | shared-runtime floor; use this to judge workspace baseline pressure | [minimal.md](artifacts/wasm-footprint-2/minimal.md) |
| `user_hub` | leaf-canister | `table[0]` | 1140639 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint-2/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1084706 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint-2/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1039165 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint-2/scale_hub.md) |
| `scale` | leaf-canister | `table[0]` | 994399 | largest retained symbol from raw-built twiggy analysis | [scale.md](artifacts/wasm-footprint-2/scale.md) |
| `test` | leaf-canister | `table[0]` | 1032445 | largest retained symbol from raw-built twiggy analysis | [test.md](artifacts/wasm-footprint-2/test.md) |
| `root` | bundle-canister | `table[0]` | 2485614 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint-2/root.md) |

## Current Size Snapshot

| Canister | Shrunk wasm | Shrink delta | Baseline delta | Note |
| --- | ---: | ---: | --- | --- |
| `app` | 1685591 | 117018 | +17901 | role-specific leaf |
| `minimal` | 1669669 | 115416 | +15919 | shared runtime floor |
| `user_hub` | 1832071 | 126464 | +15934 | role-specific leaf |
| `user_shard` | 1780494 | 124287 | -14924 | role-specific leaf |
| `scale_hub` | 1734598 | 119958 | +15913 | role-specific leaf |
| `scale` | 1687499 | 116501 | +15907 | role-specific leaf |
| `test` | 1728232 | 120161 | -34910 | role-specific leaf |
| `root` | 3719114 | 218720 | -689 | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `minimal` remains the shared-runtime floor. If `minimal` stays close to feature canisters, size pressure is coming from shared crates rather than role-specific logic.
- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Minimal floor close to feature canisters | WARN | `minimal` shrunk wasm = 1669669, `app` shrunk wasm = 1685591. |
| Root control-plane outlier | WARN | `root` shrunk wasm = 3719114. |
| Shrink delta unexpectedly low | OK | `minimal` shrink delta = 115416 bytes. |
| Positive same-day baseline drift in current scope | WARN | 5 canister(s) grew versus the selected same-day baseline. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 1802609 | 1685591 | 117018 | 512818 | 478445 | +17901 | 3434 | 3434 | 21 | [app.md](artifacts/wasm-footprint-2/app.md) |
| `minimal` | leaf-canister | 1785085 | 1669669 | 115416 | 508356 | 472283 | +15919 | 3401 | 3401 | 18 | [minimal.md](artifacts/wasm-footprint-2/minimal.md) |
| `user_hub` | leaf-canister | 1958535 | 1832071 | 126464 | 567151 | 521706 | +15934 | 3729 | 3729 | 22 | [user_hub.md](artifacts/wasm-footprint-2/user_hub.md) |
| `user_shard` | leaf-canister | 1904781 | 1780494 | 124287 | 551274 | 510577 | -14924 | 3595 | 3595 | 22 | [user_shard.md](artifacts/wasm-footprint-2/user_shard.md) |
| `scale_hub` | leaf-canister | 1854556 | 1734598 | 119958 | 531894 | 492578 | +15913 | 3523 | 3523 | 22 | [scale_hub.md](artifacts/wasm-footprint-2/scale_hub.md) |
| `scale` | leaf-canister | 1804000 | 1687499 | 116501 | 515311 | 479573 | +15907 | 3441 | 3441 | 19 | [scale.md](artifacts/wasm-footprint-2/scale.md) |
| `test` | leaf-canister | 1848393 | 1728232 | 120161 | 529614 | 492136 | -34910 | 3499 | 3499 | 21 | [test.md](artifacts/wasm-footprint-2/test.md) |
| `root` | bundle-canister | 3937834 | 3719114 | 218720 | 1527400 | 1435713 | -689 | 5789 | 5789 | 45 | [root.md](artifacts/wasm-footprint-2/root.md) |

## Risk Score

Risk Score: **4 / 10**

Interpretation: this is a wasm drift score, not a correctness score. The main drivers in Canic are the shared runtime floor across leaf canisters and the special-case bundle behavior in `root`.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo build --target wasm32-unknown-unknown ... && dfx build ...` | PASS | built and cached raw/shrunk artifacts for cargo/dfx release builds |
| `ic-wasm <artifact> info` | PASS | structure snapshots captured for built and shrunk artifacts |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | twiggy artifacts captured for each canister in scope |
| `baseline size-metrics.tsv comparison` | PASS | baseline deltas calculated from `docs/audits/reports/2026-04/2026-04-05/artifacts/wasm-footprint/size-metrics.tsv` |

## Follow-up Actions

1. Owner boundary: `wasm drift follow-through`
   Action: investigate the canisters with positive same-day baseline deltas first and decide whether the added bytes are intentional or should come back down in the next rerun.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`
2. Owner boundary: `shared runtime baseline`
   Action: compare `minimal` retained hotspots against one feature canister in the next run and treat overlapping drivers as shared-cost reduction candidates.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`
3. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`

## Report Files

- [wasm-footprint-2.md](./wasm-footprint-2.md)
- [size-summary.md](artifacts/wasm-footprint-2/size-summary.md)
- [size-report.json](artifacts/wasm-footprint-2/size-report.json)
- [app.md](artifacts/wasm-footprint-2/app.md)
- [minimal.md](artifacts/wasm-footprint-2/minimal.md)
- [user_hub.md](artifacts/wasm-footprint-2/user_hub.md)
- [user_shard.md](artifacts/wasm-footprint-2/user_shard.md)
- [scale_hub.md](artifacts/wasm-footprint-2/scale_hub.md)
- [scale.md](artifacts/wasm-footprint-2/scale.md)
- [test.md](artifacts/wasm-footprint-2/test.md)
- [root.md](artifacts/wasm-footprint-2/root.md)
