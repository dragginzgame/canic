# Wasm Footprint Audit v6 - 2026-09-03

## Verdict

- Run result: `fail`.
- Result validity: `valid`.
- Comparability: `first-v6-baseline`.
- Authoritative risk score: `7/10`.

V6 completed two isolated clean release builds and one debug build for every frozen configured role
plus Fleet Coordinator and Wasm Store through Canic's authoritative host
artifact builder. It did not invoke direct Cargo
Wasm compilation, infer a target-directory artifact, or retain a competing
pre-optimization Wasm. The governed release transform supplies its own exact
before/after measurements. Historical v5 evidence remains valid but is not
comparable with v6 because it predates the indexed Hub and child roles.

## Scope And Identity

- Definition: `docs/audits/recurring/system/wasm-footprint.md`.
- Compared predecessor: `N/A`.
- Original v6 baseline: `docs/audits/reports/2026-09/2026-09-03/wasm-footprint-v6.md`.
- Release anchor: `v0.110.5`.
- Source commit: `50f40171d6177c3d1e490b1fdb5f6163323b2cd5`.
- Source tree: `5a66988735c707b188d9d1fe03a3ed3b4ff7a273`.
- Product tree: `624e09f252a87b6a222d2da68d3fbfb3fcf3959948c2ad1f4f2c07d582db3b2e`.
- Method: `CANIC-WASM-001/v6`; definition `0c8487a989dadaae03cba0437545a2ea236274607622ff3d74623696c1ebe797`; executable
  composite `726e629bd67983acd2a7e4d42c275c282953b1ffef0e3236435c3c0817ef0d01`.
- Ordered roster: `app,index_hub,test,user_hub,scale_hub,index_child,user_shard,scale_replica,root,fleet_coordinator,wasm_store`.
- Profiles: `release-clean-a+release-clean-b+debug`.
- Branch/worktree: `detached`; clean disposable linked worktree before the
  run, tracked-clean after the run, with only permitted `.icp/` build output.
- Environment: local, offline, isolated `CARGO_TARGET_DIR`; no replica,
  credentials, deployment, or authoritative external mutation.
- Auditor: Codex.
- Started/completed: `2026-09-03T08:34:36Z` / `2026-09-03T09:23:02Z`.

## Immutable Run Identity

```text
release_anchor: v0.110.5
source_commit_full: 50f40171d6177c3d1e490b1fdb5f6163323b2cd5
source_tree_hash: 5a66988735c707b188d9d1fe03a3ed3b4ff7a273
product_tree_hash: 624e09f252a87b6a222d2da68d3fbfb3fcf3959948c2ad1f4f2c07d582db3b2e
clean_worktree: true before; tracked-clean after; generated .icp only
cargo_lock_hash: 0fd6c7897d08e6a0f4e436caaf319ba24ebc32236434010b5c6ae3507f663147
rust_toolchain: rustc 1.97.1 (8bab26f4f 2026-07-14); cargo 1.97.1 (c980f4866 2026-06-30)
target_triple: wasm32-unknown-unknown
feature_set: apps/test frozen configured roles plus Coordinator and Store
audit_method_id: CANIC-WASM-001
audit_method_version: 6
audit_method_fingerprint: 726e629bd67983acd2a7e4d42c275c282953b1ffef0e3236435c3c0817ef0d01
audit_script_hashes: definition=0c8487a989dadaae03cba0437545a2ea236274607622ff3d74623696c1ebe797; executable-composite=726e629bd67983acd2a7e4d42c275c282953b1ffef0e3236435c3c0817ef0d01
external_tool_versions: icp 1.4.0; ic-wasm 0.11.1; twiggy-opt 0.8.0; wasm-opt version 132 (version_132)
fixture_or_seed: apps/test/canic.toml@50f40171d6177c3d1e490b1fdb5f6163323b2cd5; roster=app,index_hub,test,user_hub,scale_hub,index_child,user_shard,scale_replica,root,fleet_coordinator,wasm_store
environment_class: isolated local linked-worktree execution_trace
execution_path_key: 18a061f6f71b52c15db9c293a66318610cc94213d7594f47b19220d95eba572f
started_at: 2026-09-03T08:34:36Z
completed_at: 2026-09-03T09:23:02Z
```

The execution path itself is not retained. Its hash is a comparison key because
the independently owned `CANIC-092-BUILD-001` path-dependence finding makes a
different checkout path non-comparable for gzip/byte continuity.

## Canonical Artifact Sizes

| Canister | Kind | Release Wasm | Release gzip | Debug Wasm | Debug gzip | Debug delta | Predecessor delta |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `app` | component | 2887478 | 1039059 | 6503664 | 1620397 | +3616186 (125.24%) | N/A (N/A) |
| `index_hub` | component | 2672270 | 960608 | 6111310 | 1519597 | +3439040 (128.69%) | N/A (N/A) |
| `test` | component | 3202037 | 1139866 | 7271644 | 1818010 | +4069607 (127.09%) | N/A (N/A) |
| `user_hub` | component | 3330758 | 1188786 | 7589098 | 1903067 | +4258340 (127.85%) | N/A (N/A) |
| `scale_hub` | component | 3252022 | 1157869 | 7410414 | 1858900 | +4158392 (127.87%) | N/A (N/A) |
| `index_child` | component | 2443547 | 885997 | 5591079 | 1396431 | +3147532 (128.81%) | N/A (N/A) |
| `user_shard` | component | 3259113 | 1161338 | 7410085 | 1858964 | +4150972 (127.37%) | N/A (N/A) |
| `scale_replica` | component | 2898207 | 1044134 | 6525480 | 1628006 | +3627273 (125.16%) | N/A (N/A) |
| `root` | fleet-subnet-root | 7097112 | 2523593 | 15943177 | 3952520 | +8846065 (124.64%) | N/A (N/A) |
| `fleet_coordinator` | fleet-coordinator | 3497977 | 1215581 | 7864111 | 1871070 | +4366134 (124.82%) | N/A (N/A) |
| `wasm_store` | wasm-store | 2452131 | 890086 | 5566994 | 1395876 | +3114863 (127.03%) | N/A (N/A) |

## Governed Release Optimization

| Canister | Raw before → after | Gzip before → after | Code before → after | Data section before → after | Functions before → after |
| --- | ---: | ---: | ---: | ---: | ---: |
| `app` | 3067317 → 2887478 | 1020273 → 1039059 | 2827346 → 2650724 | 202577 → 200022 | 5369 → 4672 |
| `index_hub` | 2843118 → 2672270 | 943256 → 960608 | 2612651 → 2444607 | 193761 → 191602 | 5056 → 4346 |
| `test` | 3403236 → 3202037 | 1119669 → 1139866 | 3142734 → 2944773 | 216629 → 214090 | 5940 → 5189 |
| `user_hub` | 3538804 → 3330758 | 1171883 → 1188786 | 3276982 → 3072247 | 220849 → 218254 | 6256 → 5473 |
| `scale_hub` | 3457034 → 3252022 | 1141493 → 1157869 | 3198807 → 2997048 | 218405 → 215853 | 6062 → 5295 |
| `index_child` | 2599776 → 2443547 | 868576 → 885997 | 2378085 → 2224620 | 186873 → 184710 | 4653 → 3996 |
| `user_shard` | 3464190 → 3259113 | 1140899 → 1161338 | 3203202 → 3001399 | 217277 → 214712 | 6103 → 5341 |
| `scale_replica` | 3078355 → 2898207 | 1025182 → 1044134 | 2837328 → 2660428 | 202969 → 200385 | 5399 → 4700 |
| `root` | 7542918 → 7097112 | 2454567 → 2523593 | 7100961 → 6659744 | 336997 → 333748 | 10983 → 9596 |
| `fleet_coordinator` | 3721861 → 3497977 | 1182054 → 1215581 | 3468047 → 3247309 | 201465 → 199123 | 5324 → 4463 |
| `wasm_store` | 2608460 → 2452131 | 870170 → 890086 | 2408151 → 2254620 | 179229 → 177039 | 4922 → 4244 |

There is no dedicated minimal role in scope. Component release spread is
`1.3631`; `root` is interpreted separately as Fleet Subnet
Root infrastructure and is `2.1308` times the largest
Component. Coordinator and Store are independently reported infrastructure.
No v1 raw/shrunk delta is
reported because that obsolete duplicate artifact model was removed.

## Structure And Retained-Size Evidence

| Canister | Functions | Data sections | Data bytes | Exports | Largest shallow item | Largest retained item |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `app` | 4709 | 180 | 198528 | 9 | `code[926]` (126498) | `table[0]` (1188945) |
| `index_hub` | 4385 | 164 | 190241 | 9 | `code[857]` (126497) | `table[0]` (1041436) |
| `test` | 5228 | 195 | 212473 | 11 | `code[1023]` (126498) | `table[0]` (1467065) |
| `user_hub` | 5512 | 194 | 216646 | 14 | `code[1097]` (126498) | `table[0]` (1585863) |
| `scale_hub` | 5334 | 194 | 214245 | 11 | `code[1050]` (126498) | `table[0]` (1514356) |
| `index_child` | 4035 | 154 | 183429 | 9 | `code[778]` (126497) | `table[0]` (841415) |
| `user_shard` | 5384 | 195 | 213096 | 13 | `code[1053]` (126498) | `table[0]` (1453247) |
| `scale_replica` | 4737 | 180 | 198891 | 10 | `code[932]` (126498) | `table[0]` (1197769) |
| `root` | 9641 | 280 | 331442 | 11 | `code[7662]` (241913) | `table[0]` (4658570) |
| `fleet_coordinator` | 4497 | 233 | 197210 | 6 | `code[147]` (255910) | `table[0]` (1240111) |
| `wasm_store` | 4283 | 163 | 175684 | 10 | `code[820]` (126542) | `table[0]` (999590) |

All canonical release artifacts were accepted by `ic-wasm info`, `twiggy
top`, retained `top`, `dominators`, and `monos`. The builder's shrink step
removes source-level names, so current attribution is structural rather than a
claim about a particular crate. Repeated `table[0]`/element retention across
Components is a runtime fan-in signal; it is not sufficient by itself to assign a
dependency owner. Bounded dominator and monomorphization evidence is retained
in each role detail file.

The largest retained item occupies `65.6404%` of its canonical
release Wasm. Largest compatible predecessor growth is
`0.00%`; `0.00%` means either no positive growth or no
compatible predecessor.

## Risk Score

Risk score: **7 / 10**.

- no compatible v6 predecessor: +2.
- Component release spread >= 1.25: +2.
- root/max-Component release ratio 2.0-2.9999: +1.
- largest retained item >= 25% of release Wasm: +2.

This is size-pressure evidence, not a correctness verdict. Root build-path
reproducibility remains owned by `CANIC-092-BUILD-001` and is neither cleared
nor duplicated here.

## Findings

- Method revision: v6 retains path-confined role-local optimizer records and
  two-clean-build determinism while adding the configured indexed Hub and
  child roles; valid v5 history cannot baseline v6.
- New product findings: none. The first v6 measurement is a baseline, and no
  comparable regression exists to attribute.

## Required Checklist

| Requirement | Result | Evidence |
| --- | --- | --- |
| clean isolated product snapshot | PASS | linked worktree clean before; tracked-clean after |
| canonical release artifacts | PASS | complete 11-role roster built through host `build_artifact` |
| deterministic clean release builds | PASS | exact Wasm, gzip, Candid, and transform metrics match across isolated targets |
| canonical debug artifacts | PASS | same 11 roles and authority |
| governed Binaryen optimization | PASS | pinned tool plus before/after raw, gzip, code, data, and function metrics for every role |
| Candid/export/feature parity | PASS | release transform rejects before replacing an artifact if any governed invariant changes |
| builder gzip integrity | PASS | every gzip decodes to its paired canonical Wasm |
| machine-readable sizes | PASS | `size-metrics.tsv` |
| `ic-wasm info` | PASS | 11 release artifacts parsed |
| `twiggy top` and retained `top` | PASS | compact hotspot columns retained |
| `twiggy dominators` | PASS | bounded role excerpts retained |
| `twiggy monos` | PASS | bounded role excerpts retained |
| compatible predecessor selection | PASS | exact method/roster/profile/path/tool keys; `N/A` |
| direct Cargo/pre-optimization fallback absent | PASS | v6 invokes only the host artifact authority |
| source mutation | PASS | no tracked mutation or unexpected untracked path |

## Verification Readout

| Command/check | Result | Notes |
| --- | --- | --- |
| `cargo run --offline --locked -p canic-host --example build_artifact -- <role> release ...` | PASS | 11 ordered roles, repeated from isolated clean targets |
| same authoritative command with `debug` | PASS | 11 ordered roles |
| `gzip -t` plus decoded SHA-256 equality | PASS | release and debug artifacts |
| `cmp` plus SHA-256 identity | PASS | two clean canonical release builds, all roles |
| `ic-wasm <release.wasm> info` | PASS | all roles |
| `twiggy top\|dominators\|monos <release.wasm>` | PASS | all roles |
| method composite | PASS | root-independent `726e629bd67983acd2a7e4d42c275c282953b1ffef0e3236435c3c0817ef0d01` |
| product-tree identity | PASS | `624e09f252a87b6a222d2da68d3fbfb3fcf3959948c2ad1f4f2c07d582db3b2e` |
| retained evidence hashes | PASS | manifest binds the report and compact artifacts |

## Retained Evidence

- [size summary](artifacts/wasm-footprint-v6/size-summary.md)
- [machine-readable metrics](artifacts/wasm-footprint-v6/size-metrics.tsv)
- [optimizer before/after metrics](artifacts/wasm-footprint-v6/optimization-metrics.tsv)
- [clean-build determinism](artifacts/wasm-footprint-v6/determinism.tsv)
- [method identity](artifacts/wasm-footprint-v6/method.json)
- [evidence manifest](artifacts/wasm-footprint-v6/evidence-manifest.yml)
- [app detail](artifacts/wasm-footprint-v6/app.md)
- [index_hub detail](artifacts/wasm-footprint-v6/index_hub.md)
- [test detail](artifacts/wasm-footprint-v6/test.md)
- [user_hub detail](artifacts/wasm-footprint-v6/user_hub.md)
- [scale_hub detail](artifacts/wasm-footprint-v6/scale_hub.md)
- [index_child detail](artifacts/wasm-footprint-v6/index_child.md)
- [user_shard detail](artifacts/wasm-footprint-v6/user_shard.md)
- [scale_replica detail](artifacts/wasm-footprint-v6/scale_replica.md)
- [root detail](artifacts/wasm-footprint-v6/root.md)
- [fleet_coordinator detail](artifacts/wasm-footprint-v6/fleet_coordinator.md)
- [wasm_store detail](artifacts/wasm-footprint-v6/wasm_store.md)
