# Wasm Footprint Audit - 2026-04-05

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`
- Code snapshot identifier: `590335d1`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T23:08:41Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `release`
- Target canisters in scope: `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `root` 
- Analysis artifact note: `twiggy` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and `dfx`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | PASS | Cached raw artifacts under `artifacts/wasm-size/release/raw/` and shrunk artifacts under `artifacts/wasm-size/release/shrunk/` were recorded for `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `root` . |
| Artifact sizes recorded in machine-readable artifact | PASS | [size-report.json](artifacts/wasm-footprint-3/size-report.json) plus per-canister `*.size-report.json` files. |
| Twiggy top captured | PASS | `*.twiggy-top.txt` and `*.twiggy-top.csv` emitted for each canister when `twiggy` is available. |
| Twiggy dominators captured | PASS | `*.twiggy-dominators.txt` emitted for each canister when `twiggy` is available. |
| Twiggy monos captured | PASS | `*.twiggy-monos.txt` emitted for each canister when `twiggy` is available. |
| Baseline path selected by daily baseline discipline | PASS | Current run stem is `wasm-footprint-3`; baseline path resolves to `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`. |
| Size deltas versus baseline recorded when baseline exists | PASS | Baseline deltas were calculated from \`docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md\`. |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

- Same-day rerun against baseline `docs/audits/reports/2026-04/2026-04-05/wasm-footprint.md`.
- Per-canister baseline deltas in the snapshot table compare current shrunk wasm bytes to the baseline run.

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
| `app` | leaf-canister | `table[0]` | 980305 | largest retained symbol from raw-built twiggy analysis | [app.md](artifacts/wasm-footprint-3/app.md) |
| `minimal` | leaf-canister | `table[0]` | 980305 | shared-runtime floor; use this to judge workspace baseline pressure | [minimal.md](artifacts/wasm-footprint-3/minimal.md) |
| `user_hub` | leaf-canister | `table[0]` | 1144291 | largest retained symbol from raw-built twiggy analysis | [user_hub.md](artifacts/wasm-footprint-3/user_hub.md) |
| `user_shard` | leaf-canister | `table[0]` | 1090251 | largest retained symbol from raw-built twiggy analysis | [user_shard.md](artifacts/wasm-footprint-3/user_shard.md) |
| `scale_hub` | leaf-canister | `table[0]` | 1038776 | largest retained symbol from raw-built twiggy analysis | [scale_hub.md](artifacts/wasm-footprint-3/scale_hub.md) |
| `scale` | leaf-canister | `table[0]` | 997342 | largest retained symbol from raw-built twiggy analysis | [scale.md](artifacts/wasm-footprint-3/scale.md) |
| `root` | bundle-canister | `table[0]` | 2487538 | control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers | [root.md](artifacts/wasm-footprint-3/root.md) |

## Current Size Snapshot

| Canister | Shrunk wasm | Shrink delta | Baseline delta | Note |
| --- | ---: | ---: | --- | --- |
| `app` | 1671916 | 115480 | +3155 | role-specific leaf |
| `minimal` | 1671915 | 115481 | +3152 | shared runtime floor |
| `user_hub` | 1835250 | 126626 | +4086 | role-specific leaf |
| `user_shard` | 1785409 | 124478 | +5824 | role-specific leaf |
| `scale_hub` | 1732643 | 119601 | +2838 | role-specific leaf |
| `scale` | 1689933 | 116571 | +3340 | role-specific leaf |
| `root` | 3720687 | 218526 | +10409 | control-plane bundle outlier |

## Dependency Fan-In Pressure

- `minimal` remains the shared-runtime floor. If `minimal` stays close to feature canisters, size pressure is coming from shared crates rather than role-specific logic.
- `root` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap `wasm_store.wasm.gz` artifact during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as `canic-core`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Minimal floor close to feature canisters | WARN | `minimal` shrunk wasm = 1671915, `app` shrunk wasm = 1671916. |
| Root control-plane outlier | WARN | `root` shrunk wasm = 3720687. |
| Shrink delta unexpectedly low | OK | `minimal` shrink delta = 115481 bytes. |
| Positive same-day baseline drift in current scope | WARN | 7 canister(s) grew versus the selected same-day baseline. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `app` | leaf-canister | 1787396 | 1671916 | 115480 | 509362 | 473102 | +3155 | 3403 | 3403 | 17 | [app.md](artifacts/wasm-footprint-3/app.md) |
| `minimal` | leaf-canister | 1787396 | 1671915 | 115481 | 509366 | 473157 | +3152 | 3403 | 3403 | 17 | [minimal.md](artifacts/wasm-footprint-3/minimal.md) |
| `user_hub` | leaf-canister | 1961876 | 1835250 | 126626 | 568876 | 522768 | +4086 | 3731 | 3731 | 21 | [user_hub.md](artifacts/wasm-footprint-3/user_hub.md) |
| `user_shard` | leaf-canister | 1909887 | 1785409 | 124478 | 553307 | 512672 | +5824 | 3599 | 3599 | 21 | [user_shard.md](artifacts/wasm-footprint-3/user_shard.md) |
| `scale_hub` | leaf-canister | 1852244 | 1732643 | 119601 | 531861 | 491638 | +2838 | 3517 | 3517 | 20 | [scale_hub.md](artifacts/wasm-footprint-3/scale_hub.md) |
| `scale` | leaf-canister | 1806504 | 1689933 | 116571 | 516453 | 481369 | +3340 | 3443 | 3443 | 18 | [scale.md](artifacts/wasm-footprint-3/scale.md) |
| `root` | bundle-canister | 3939213 | 3720687 | 218526 | 1531444 | 1439124 | +10409 | 5787 | 5787 | 42 | [root.md](artifacts/wasm-footprint-3/root.md) |

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

- [wasm-footprint-3.md](./wasm-footprint-3.md)
- [size-summary.md](artifacts/wasm-footprint-3/size-summary.md)
- [size-report.json](artifacts/wasm-footprint-3/size-report.json)
- [app.md](artifacts/wasm-footprint-3/app.md)
- [minimal.md](artifacts/wasm-footprint-3/minimal.md)
- [user_hub.md](artifacts/wasm-footprint-3/user_hub.md)
- [user_shard.md](artifacts/wasm-footprint-3/user_shard.md)
- [scale_hub.md](artifacts/wasm-footprint-3/scale_hub.md)
- [scale.md](artifacts/wasm-footprint-3/scale.md)
- [root.md](artifacts/wasm-footprint-3/root.md)
