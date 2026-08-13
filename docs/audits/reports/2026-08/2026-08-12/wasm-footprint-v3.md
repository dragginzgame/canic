# Wasm Footprint Audit v3 - 2026-08-12

## Verdict

- Run result: `pass`.
- Result validity: `valid`.
- Comparability: `first-v3-baseline`.
- Authoritative risk score: `5/10`.

V3 completed fresh release and debug builds for every frozen configured role
plus Fleet Coordinator and Wasm Store through Canic's authoritative host
artifact builder. It did not invoke direct Cargo
Wasm compilation, infer a target-directory artifact, or recreate a pre-shrink
metric. It admits the current roster and separate infrastructure coverage after
v2's frozen scope correctly detected product drift. Historical v2 evidence
remains valid but is not comparable with v3.

## Scope And Identity

- Definition: `docs/audits/recurring/system/wasm-footprint.md`.
- Compared predecessor: `N/A`.
- Original v3 baseline: `docs/audits/reports/2026-08/2026-08-12/wasm-footprint-v3.md`.
- Release anchor: `v0.101.53`.
- Source commit: `23c0328f78b215580d734ef01b52b35fa3e38ade`.
- Source tree: `4e9d2bd8da24ead06a237d349dcb3b42a6115bf0`.
- Product tree: `cf92f56fc7d889ef4d6066d3f733e0e485bd70ac2f834355b3b7b683828226f1`.
- Method: `CANIC-WASM-001/v3`; definition `9747666aeee64eff0af26b92f90bd1ccf12f318023780aa7148fdd55fc29d745`; executable
  composite `0eb95225272ca531e4104454d24c2e2fa26d4fc26cfefbe4d151383ad0c6b6a6`.
- Ordered roster: `app,test,user_hub,scale_hub,user_shard,scale_replica,root,fleet_coordinator,wasm_store`.
- Profiles: `release+debug`.
- Branch/worktree: `detached`; clean disposable linked worktree before the
  run, tracked-clean after the run, with only permitted `.icp/` build output.
- Environment: local, offline, isolated `CARGO_TARGET_DIR`; no replica,
  credentials, deployment, or authoritative external mutation.
- Auditor: Codex.
- Started/completed: `2026-08-12T18:07:25Z` / `2026-08-12T18:24:53Z`.

## Immutable Run Identity

```text
release_anchor: v0.101.53
source_commit_full: 23c0328f78b215580d734ef01b52b35fa3e38ade
source_tree_hash: 4e9d2bd8da24ead06a237d349dcb3b42a6115bf0
product_tree_hash: cf92f56fc7d889ef4d6066d3f733e0e485bd70ac2f834355b3b7b683828226f1
clean_worktree: true before; tracked-clean after; generated .icp only
cargo_lock_hash: b78a2423c2e06e4da22b55dd31f2b12ef69091d9de9a738bde861c28bd0165b3
rust_toolchain: rustc 1.97.1 (8bab26f4f 2026-07-14); cargo 1.97.1 (c980f4866 2026-06-30)
target_triple: wasm32-unknown-unknown
feature_set: apps/test frozen configured roles plus Coordinator and Store
audit_method_id: CANIC-WASM-001
audit_method_version: 3
audit_method_fingerprint: 0eb95225272ca531e4104454d24c2e2fa26d4fc26cfefbe4d151383ad0c6b6a6
audit_script_hashes: definition=9747666aeee64eff0af26b92f90bd1ccf12f318023780aa7148fdd55fc29d745; executable-composite=0eb95225272ca531e4104454d24c2e2fa26d4fc26cfefbe4d151383ad0c6b6a6
external_tool_versions: icp 1.3.0; ic-wasm 0.11.0; twiggy-opt 0.8.0
fixture_or_seed: apps/test/canic.toml@23c0328f78b215580d734ef01b52b35fa3e38ade; roster=app,test,user_hub,scale_hub,user_shard,scale_replica,root,fleet_coordinator,wasm_store
environment_class: isolated local linked-worktree execution_trace
execution_path_key: cc09a3319dc654dc277c542b01b8bc9812c0c9246bdb1fec475eee967410d884
started_at: 2026-08-12T18:07:25Z
completed_at: 2026-08-12T18:24:53Z
```

The execution path itself is not retained. Its hash is a comparison key because
the independently owned `CANIC-092-BUILD-001` path-dependence finding makes a
different checkout path non-comparable for gzip/byte continuity.

## Canonical Artifact Sizes

| Canister | Kind | Release Wasm | Release gzip | Debug Wasm | Debug gzip | Debug delta | Predecessor delta |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `app` | component | 3006400 | 980885 | 6433987 | 1585714 | +3427587 (114.01%) | N/A (N/A) |
| `test` | component | 3037685 | 991291 | 6497057 | 1602718 | +3459372 (113.88%) | N/A (N/A) |
| `user_hub` | component | 3135440 | 1026694 | 6724477 | 1663661 | +3589037 (114.47%) | N/A (N/A) |
| `scale_hub` | component | 3047098 | 994075 | 6517862 | 1605304 | +3470764 (113.90%) | N/A (N/A) |
| `user_shard` | component | 3140027 | 1028056 | 6720789 | 1663219 | +3580762 (114.04%) | N/A (N/A) |
| `scale_replica` | component | 3016265 | 985294 | 6454190 | 1592950 | +3437925 (113.98%) | N/A (N/A) |
| `root` | fleet-subnet-root | 7539746 | 2430627 | 15666083 | 3777667 | +8126337 (107.78%) | N/A (N/A) |
| `fleet_coordinator` | fleet-coordinator | 3439803 | 1075598 | 7247791 | 1698688 | +3807988 (110.70%) | N/A (N/A) |
| `wasm_store` | wasm-store | 2597251 | 855667 | 5553475 | 1377059 | +2956224 (113.82%) | N/A (N/A) |

There is no dedicated minimal role in scope. Component release spread is
`1.0444`; `root` is interpreted separately as Fleet Subnet
Root infrastructure and is `2.4012` times the largest
Component. Coordinator and Store are independently reported infrastructure.
No v1 raw/shrunk delta is
reported because that obsolete duplicate artifact model was removed.

## Structure And Retained-Size Evidence

| Canister | Functions | Data sections | Data bytes | Exports | Largest shallow item | Largest retained item |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `app` | 5449 | 3 | 236516 | 26 | `data[0]` (236102) | `table[0]` (1377137) |
| `test` | 5502 | 3 | 238124 | 29 | `data[0]` (237702) | `table[0]` (1400367) |
| `user_hub` | 5743 | 3 | 241440 | 32 | `data[0]` (240982) | `table[0]` (1486455) |
| `scale_hub` | 5531 | 3 | 238624 | 29 | `data[0]` (238206) | `table[0]` (1407085) |
| `user_shard` | 5728 | 3 | 242296 | 33 | `data[0]` (241854) | `table[0]` (1492867) |
| `scale_replica` | 5480 | 3 | 236656 | 27 | `data[0]` (236238) | `table[0]` (1385466) |
| `root` | 10977 | 3 | 446252 | 126 | `data[0]` (445254) | `table[0]` (5217079) |
| `fleet_coordinator` | 5136 | 3 | 242940 | 28 | `data[0]` (242522) | `table[0]` (1566403) |
| `wasm_store` | 5046 | 3 | 216224 | 31 | `data[0]` (215710) | `table[0]` (1203715) |

All canonical release artifacts were accepted by `ic-wasm info`, `twiggy
top`, retained `top`, `dominators`, and `monos`. The builder's shrink step
removes source-level names, so current attribution is structural rather than a
claim about a particular crate. Repeated `table[0]`/element retention across
Components is a runtime fan-in signal; it is not sufficient by itself to assign a
dependency owner. Bounded dominator and monomorphization evidence is retained
in each role detail file.

The largest retained item occupies `69.1944%` of its canonical
release Wasm. Largest compatible predecessor growth is
`0.00%`; `0.00%` means either no positive growth or no
compatible predecessor.

## Risk Score

Risk score: **5 / 10**.

- no compatible v3 predecessor: +2.
- root/max-Component release ratio 2.0-2.9999: +1.
- largest retained item >= 25% of release Wasm: +2.

This is size-pressure evidence, not a correctness verdict. Root build-path
reproducibility remains owned by `CANIC-092-BUILD-001` and is neither cleared
nor duplicated here.

## Findings

- Method revision: v3 admits the current configured roster and separate
  Coordinator/Store infrastructure; valid v2 history cannot baseline v3.
- New product findings: none. The first v3 measurement is a baseline, and no
  comparable regression exists to attribute.

## Required Checklist

| Requirement | Result | Evidence |
| --- | --- | --- |
| clean isolated product snapshot | PASS | linked worktree clean before; tracked-clean after |
| canonical release artifacts | PASS | complete nine-role roster built through host `build_artifact` |
| canonical debug artifacts | PASS | same nine roles and authority |
| builder gzip integrity | PASS | every gzip decodes to its paired canonical Wasm |
| machine-readable sizes | PASS | `size-metrics.tsv` |
| `ic-wasm info` | PASS | nine release artifacts parsed |
| `twiggy top` and retained `top` | PASS | compact hotspot columns retained |
| `twiggy dominators` | PASS | bounded role excerpts retained |
| `twiggy monos` | PASS | bounded role excerpts retained |
| compatible predecessor selection | PASS | exact method/roster/profile/path/tool keys; `N/A` |
| direct Cargo/pre-shrink fallback absent | PASS | v3 invokes only the host artifact authority |
| source mutation | PASS | no tracked mutation or unexpected untracked path |

## Verification Readout

| Command/check | Result | Notes |
| --- | --- | --- |
| `cargo run --offline --locked -p canic-host --example build_artifact -- <role> release ...` | PASS | nine ordered roles |
| same authoritative command with `debug` | PASS | nine ordered roles |
| `gzip -t` plus decoded SHA-256 equality | PASS | release and debug artifacts |
| `ic-wasm <release.wasm> info` | PASS | all roles |
| `twiggy top\|dominators\|monos <release.wasm>` | PASS | all roles |
| method composite | PASS | root-independent `0eb95225272ca531e4104454d24c2e2fa26d4fc26cfefbe4d151383ad0c6b6a6` |
| product-tree identity | PASS | `cf92f56fc7d889ef4d6066d3f733e0e485bd70ac2f834355b3b7b683828226f1` |
| retained evidence hashes | PASS | manifest binds the report and compact artifacts |

## Retained Evidence

- [size summary](artifacts/wasm-footprint-v3/size-summary.md)
- [machine-readable metrics](artifacts/wasm-footprint-v3/size-metrics.tsv)
- [method identity](artifacts/wasm-footprint-v3/method.json)
- [evidence manifest](artifacts/wasm-footprint-v3/evidence-manifest.yml)
- [app detail](artifacts/wasm-footprint-v3/app.md)
- [test detail](artifacts/wasm-footprint-v3/test.md)
- [user_hub detail](artifacts/wasm-footprint-v3/user_hub.md)
- [scale_hub detail](artifacts/wasm-footprint-v3/scale_hub.md)
- [user_shard detail](artifacts/wasm-footprint-v3/user_shard.md)
- [scale_replica detail](artifacts/wasm-footprint-v3/scale_replica.md)
- [root detail](artifacts/wasm-footprint-v3/root.md)
- [fleet_coordinator detail](artifacts/wasm-footprint-v3/fleet_coordinator.md)
- [wasm_store detail](artifacts/wasm-footprint-v3/wasm_store.md)
