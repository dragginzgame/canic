# Wasm Footprint Audit - 2026-04-05

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`
- Code snapshot identifier: `590335d1`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T23:22:20Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `release`
- Target canisters in scope: `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `root` 
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `dfx`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/release/raw/` and shrunk artifacts under `artifacts/wasm-size/release/shrunk/` were recorded for `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `root` . |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint-4/size-report.json) plus per-canister `*.size-report.json` files. |
| Twiggy top captured | PASS | `*.twiggy-top.txt` and `*.twiggy-top.csv` emitted for each canister when `twiggy` is available. |
| Twiggy dominators captured | PASS | `*.twiggy-dominators.txt` emitted for each canister when `twiggy` is available. |
| Twiggy monos captured | PASS | `*.twiggy-monos.txt` emitted for each canister when `twiggy` is available. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint-4`; baseline path resolves to `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`. |
| Size deltas versus baseline recorded when baseline exists | PASS | Baseline deltas were calculated from \`docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md\`. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- Same-day rerun against baseline `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`.
- Per-canister baseline deltas in the snapshot table compare current shrunk wasm bytes to the baseline run.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 982911 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint-4/app.md) |
| `minimal` | leaf-canister | `table[0]` | 982911 | shared-runtime floor; use this to judge workspace baseline pressure | [minimal.md](artifacts/wasm-footprint-4/minimal.md) |
| `user_hub` | leaf-canister | `table[0]` | 1146897 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint-4/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1092857 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint-4/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1041382 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint-4/scale_hub.md) |
| `scale` | leaf-canister | `table[0]` | 999948 | largest retained symbol from raw-built twiggy analysis | [scale.md](artifacts/wasm-footprint-4/scale.md) |
| `root` | bundle-canister | `table[0]` | 2490148 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint-4/root.md) |

## Current Size Snapshot

| Canister | Shrunk wasm | Shrink delta | Baseline delta | Note |
| --- | ---: | ---: | --- | --- |
| `app` | 1637105 | 113304 | -31656 | role-specific leaf |
| `minimal` | 1637104 | 113305 | -31659 | shared runtime floor |
| `user_hub` | 1800300 | 124462 | -30864 | role-specific leaf |
| `user_shard` | 1750451 | 122315 | -29134 | role-specific leaf |
| `scale_hub` | 1697813 | 117438 | -31992 | role-specific leaf |
| `scale` | 1654981 | 114406 | -31612 | role-specific leaf |
| `root` | 3674984 | 250282 | -35294 | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `minimal` remains the shared-runtime floor. If `minimal` stays close to feature canisters, size pressure is coming from shared crates rather than role-specific logic.
- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Minimal floor close to feature canisters | WARN | `minimal` shrunk wasm = 1637104, `app` shrunk wasm = 1637105. |
| Root control-plane outlier | WARN | `root` shrunk wasm = 3674984. |
| Shrink delta unexpectedly low | OK | `minimal` shrink delta = 113305 bytes. |
| Positive same-day baseline drift in current scope | OK | 0 canister(s) grew versus the selected same-day baseline. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 1750409 | 1637105 | 113304 | 497145 | 462009 | -31656 | 3342 | 3342 | 16 | [app.md](artifacts/wasm-footprint-4/app.md) |
| `minimal` | leaf-canister | 1750409 | 1637104 | 113305 | 496888 | 461904 | -31659 | 3342 | 3342 | 16 | [minimal.md](artifacts/wasm-footprint-4/minimal.md) |
| `user_hub` | leaf-canister | 1924762 | 1800300 | 124462 | 557948 | 512565 | -30864 | 3671 | 3671 | 20 | [user_hub.md](artifacts/wasm-footprint-4/user_hub.md) |
| `user_shard` | leaf-canister | 1872766 | 1750451 | 122315 | 540796 | 501258 | -29134 | 3538 | 3538 | 20 | [user_shard.md](artifacts/wasm-footprint-4/user_shard.md) |
| `scale_hub` | leaf-canister | 1815251 | 1697813 | 117438 | 519201 | 480259 | -31992 | 3456 | 3456 | 19 | [scale_hub.md](artifacts/wasm-footprint-4/scale_hub.md) |
| `scale` | leaf-canister | 1769387 | 1654981 | 114406 | 504405 | 469760 | -31612 | 3382 | 3382 | 17 | [scale.md](artifacts/wasm-footprint-4/scale.md) |
| `root` | bundle-canister | 3925266 | 3674984 | 250282 | 1543013 | 1417249 | -35294 | 5726 | 5726 | 41 | [root.md](artifacts/wasm-footprint-4/root.md) |

## Risk Score

Risk Score: **4 / 10**

Interpretation: this is a wasm drift score, not a correctness score. The main drivers in Canic are the shared runtime floor across leaf canisters and the special-case bundle behavior in `root`.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo build --target wasm32-unknown-unknown ... && dfx build ...` | PASS | built and cached raw/shrunk artifacts for cargo/dfx release builds |
| `ic-wasm <artifact> info` | PASS | structure snapshots captured for built and shrunk artifacts |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | twiggy artifacts captured for each canister in scope |
| `baseline markdown comparison` | PASS | baseline deltas calculated from `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md` current size snapshot |

## Follow-up Actions

1. Owner boundary: `shared runtime baseline`
   Action: compare `minimal` retained hotspots against one feature canister in the next run and treat overlapping drivers as shared-cost reduction candidates.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`
2. Owner boundary: `bundle canister root`
   Action: keep tracking `root` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`

## Report Files

- [wasm-footprint-4.md](./wasm-footprint-4.md)
- [size-summary.md](artifacts/wasm-footprint-4/size-summary.md)
- [size-report.json](artifacts/wasm-footprint-4/size-report.json)
- [app.md](artifacts/wasm-footprint-4/app.md)
- [minimal.md](artifacts/wasm-footprint-4/minimal.md)
- [user_hub.md](artifacts/wasm-footprint-4/user_hub.md)
- [user_shard.md](artifacts/wasm-footprint-4/user_shard.md)
- [scale_hub.md](artifacts/wasm-footprint-4/scale_hub.md)
- [scale.md](artifacts/wasm-footprint-4/scale.md)
- [root.md](artifacts/wasm-footprint-4/root.md)
