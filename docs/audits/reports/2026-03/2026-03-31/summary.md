# Audit Summary - 2026-03-31

## Run Contexts

- Audit run: `wasm-footprint`
  - Definition: `docs/audits/recurring/system/wasm-footprint.md`
  - Baseline: `N/A` (first run for this scope on 2026-03-31)
  - Branch: `main`
  - Commit: `7ad87779`
  - Worktree: `dirty`
  - Method: `Method V1`
  - Comparability: `comparable`
- Audit run: `instruction-footprint-3`
  - Definition: `docs/audits/recurring/system/instruction-footprint.md`
  - Baseline: `N/A` (first clean run for this scope on 2026-03-31)
  - Branch: `main`
  - Commit: `7ad87779`
  - Worktree: `dirty`
  - Method: `Method V1`
  - Comparability: `partial`

Audits generated in this run:

- `wasm-footprint`
- `instruction-footprint-3`

## Risk Index Summary

| Audit | Risk Score |
| --- | ---: |
| `wasm-footprint` | 4 / 10 |
| `instruction-footprint-3` | 6 / 10 |

Overall day posture: **the first `0.20` wasm baseline and the first clean instruction baseline are now recorded. The wasm side already exposes one concrete reduction target in the shrink-path inversion, while the instruction side shows that update-floor measurement is working but query/flow observability is still incomplete.**

## Key Findings by Severity

### High

- No correctness failure was found; this is a build-artifact audit. The main structural issue surfaced by the first `0.20` baseline is the current shrink-path inversion on ordinary leaf canisters, where `dfx`-shrunk artifacts are larger than the raw Cargo wasm inputs.
- The first clean instruction baseline shows a method gap: six successful query scenarios currently leave no persisted `MetricsKind::Perf` delta, so query lanes are not yet comparable through the current persisted perf transport.
- The current repo scan found no real `perf!` checkpoints under `crates/`, so the new instruction audit can rank update-visible entrypoints but cannot yet localize flow-stage cost or regressions.

### Medium

- The current shared shrunk leaf floor is `1897556` bytes for `minimal`, `scale`, and `shard`, with `app` effectively identical at `1897571`; that keeps the main pressure on shared runtime cost rather than role-specific feature code.
- `root` remains a separate bundle-canister outlier at `3112916` shrunk bytes and must continue to be tracked separately from leaf canisters.
- The build path still pays extra `.did` regeneration work during `dfx build` for several canisters, which is not a measurement blocker but is unnecessary audit noise and a likely cleanup target.
- The current chain-key-free update floor is `test::test` at `384` local instructions. That gives `0.20` one concrete instruction baseline even before chain-key-dependent update flows are measurable in PocketIC.
- Chain-key-dependent update surfaces such as `scale_hub::create_worker` and sharding assignment flows remain deferred from the baseline until the audit harness can provision the expected ECDSA key.

### Low

- `ic-wasm` and `twiggy` were both available, so the baseline includes structure snapshots and retained-size artifacts without tooling gaps.
- The current Twiggy retained-hotspot summary is still dominated by `table[0]`, which is not yet a useful explanatory symbol for the first `0.20` hotspot report and should be refined in a follow-up slice.

## Verification Readout Rollup

| Command | Status | Notes |
| --- | --- | --- |
| `bash scripts/ci/wasm-audit-report.sh` | PASS | first dated `0.20` wasm baseline recorded under `docs/audits/reports/2026-03/2026-03-31/` |
| `cargo build --target wasm32-unknown-unknown ... && dfx build ...` | PASS | raw and shrunk artifacts recorded for the default deployable scope |
| `ic-wasm <artifact> info` | PASS | built and shrunk structure snapshots captured |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | hotspot attribution artifacts captured for every canister in scope |
| `bash scripts/ci/instruction-audit-report.sh` | PASS | first clean `0.20` instruction baseline recorded under `docs/audits/reports/2026-03/2026-03-31/` |
| `cargo test -p canic --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | isolated PocketIC scenarios produced normalized perf artifacts for the current update-visible scope |
| `query perf visibility` | PARTIAL | successful query lanes currently leave no persisted `MetricsKind::Perf` delta under the existing method |
| `baseline size-metrics.tsv comparison` | BLOCKED | first run of day; no same-day baseline exists yet |

## Follow-up Actions

1. Investigate why the current `dfx` shrink path makes ordinary leaf canisters larger than the raw Cargo wasm artifacts, and treat that as the first concrete `0.20` reduction target.
2. Add first stable `perf!` checkpoints to scaling, sharding, and root-capability flows so the instruction audit can move from endpoint-only totals to real flow-stage attribution.
3. Decide whether query lanes should become measurable through widened persisted perf accounting or through a separate query-focused audit method.
4. Add a key-provisioned PocketIC audit mode so chain-key-dependent update flows can join the instruction baseline.
5. Refresh hotspot attribution so the next `0.20` wasm rerun produces a useful shared-runtime hotspot shortlist instead of `table[0]` retained summaries.
6. Keep `root` measured separately from leaf canisters while the shared-floor work focuses on `minimal` and its near-identical peers.

## Report Files

- `docs/audits/reports/2026-03/2026-03-31/wasm-footprint.md`
- `docs/audits/reports/2026-03/2026-03-31/instruction-footprint-3.md`
- `docs/audits/reports/2026-03/2026-03-31/artifacts/wasm-footprint/size-summary.md`
- `docs/audits/reports/2026-03/2026-03-31/artifacts/wasm-footprint/size-report.json`
- `docs/audits/reports/2026-03/2026-03-31/artifacts/instruction-footprint-3/verification-readout.md`
- `docs/audits/reports/2026-03/2026-03-31/summary.md`
