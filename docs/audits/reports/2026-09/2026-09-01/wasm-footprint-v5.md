# Invalidated Wasm Footprint v5 Method Attempt - 2026-09-01

> **INVALID — do not use as baseline evidence.** The executable method identity
> was v5, but the report generator retained contradictory v4 presentation
> labels. The corrected, independently rerun baseline is
> [wasm-footprint-v5-2.md](wasm-footprint-v5-2.md). The compact measurements
> below are retained only to preserve the failed method attempt.

# Wasm Footprint Audit v4 - 2026-09-01

## Verdict

- Run result: `pass`.
- Result validity: `valid`.
- Comparability: `first-v4-baseline`.
- Authoritative risk score: `6/10`.

V4 completed two isolated clean release builds and one debug build for every frozen configured role
plus Fleet Coordinator and Wasm Store through Canic's authoritative host
artifact builder. It did not invoke direct Cargo
Wasm compilation, infer a target-directory artifact, or retain a competing
pre-optimization Wasm. The governed release transform supplies its own exact
before/after measurements. Historical v3 evidence remains valid but is not
comparable with v4.

## Scope And Identity

- Definition: `docs/audits/recurring/system/wasm-footprint.md`.
- Compared predecessor: `N/A`.
- Original v4 baseline: `docs/audits/reports/2026-09/2026-09-01/wasm-footprint-v5.md`.
- Release anchor: `v0.109.35`.
- Source commit: `3185dc45beba1a909f451efd72edef1c311fedc9`.
- Source tree: `13ab98875a9305c812c6ea3f3ed6f1adcc9b1db5`.
- Product tree: `c1fc28c7d730d274b91a2c0bcf0250d498c27181bf71c1ff38e8201a114edd23`.
- Method: `CANIC-WASM-001/v5`; definition `a51d17f9e744bf6452005e543ba6f6e77a3c2257d385aa2ad58f1acad2439c19`; executable
  composite `7728ecc60f5618883bb0e8182e4c3fa9ab7ee9dd610363e37588c8f5497360ab`.
- Ordered roster: `app,test,user_hub,scale_hub,user_shard,scale_replica,root,fleet_coordinator,wasm_store`.
- Profiles: `release-clean-a+release-clean-b+debug`.
- Branch/worktree: `detached`; clean disposable linked worktree before the
  run, tracked-clean after the run, with only permitted `.icp/` build output.
- Environment: local, offline, isolated `CARGO_TARGET_DIR`; no replica,
  credentials, deployment, or authoritative external mutation.
- Auditor: Codex.
- Started/completed: `2026-09-01T15:56:53Z` / `2026-09-01T16:40:45Z`.

## Immutable Run Identity

```text
release_anchor: v0.109.35
source_commit_full: 3185dc45beba1a909f451efd72edef1c311fedc9
source_tree_hash: 13ab98875a9305c812c6ea3f3ed6f1adcc9b1db5
product_tree_hash: c1fc28c7d730d274b91a2c0bcf0250d498c27181bf71c1ff38e8201a114edd23
clean_worktree: true before; tracked-clean after; generated .icp only
cargo_lock_hash: 649c3450d0f793cab45b6ec196fa82419ca716336903c14df4a7c2c7b04e8b16
rust_toolchain: rustc 1.97.1 (8bab26f4f 2026-07-14); cargo 1.97.1 (c980f4866 2026-06-30)
target_triple: wasm32-unknown-unknown
feature_set: apps/test frozen configured roles plus Coordinator and Store
audit_method_id: CANIC-WASM-001
audit_method_version: 5
audit_method_fingerprint: 7728ecc60f5618883bb0e8182e4c3fa9ab7ee9dd610363e37588c8f5497360ab
audit_script_hashes: definition=a51d17f9e744bf6452005e543ba6f6e77a3c2257d385aa2ad58f1acad2439c19; executable-composite=7728ecc60f5618883bb0e8182e4c3fa9ab7ee9dd610363e37588c8f5497360ab
external_tool_versions: icp 1.3.0; ic-wasm 0.11.1; twiggy-opt 0.8.0; wasm-opt version 132 (version_132)
fixture_or_seed: apps/test/canic.toml@3185dc45beba1a909f451efd72edef1c311fedc9; roster=app,test,user_hub,scale_hub,user_shard,scale_replica,root,fleet_coordinator,wasm_store
environment_class: isolated local linked-worktree execution_trace
execution_path_key: 18a061f6f71b52c15db9c293a66318610cc94213d7594f47b19220d95eba572f
started_at: 2026-09-01T15:56:53Z
completed_at: 2026-09-01T16:40:45Z
```

The execution path itself is not retained. Its hash is a comparison key because
the independently owned `CANIC-092-BUILD-001` path-dependence finding makes a
different checkout path non-comparable for gzip/byte continuity.

## Canonical Artifact Sizes

| Canister | Kind | Release Wasm | Release gzip | Debug Wasm | Debug gzip | Debug delta | Predecessor delta |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `app` | component | 2885999 | 1038424 | 6494693 | 1619528 | +3608694 (125.04%) | N/A (N/A) |
| `test` | component | 3200574 | 1139640 | 7268887 | 1817132 | +4068313 (127.11%) | N/A (N/A) |
| `user_hub` | component | 3326921 | 1188287 | 7618236 | 1914980 | +4291315 (128.99%) | N/A (N/A) |
| `scale_hub` | component | 3250522 | 1157419 | 7401719 | 1856121 | +4151197 (127.71%) | N/A (N/A) |
| `user_shard` | component | 3254719 | 1160530 | 7412893 | 1860589 | +4158174 (127.76%) | N/A (N/A) |
| `scale_replica` | component | 2896719 | 1043642 | 6514713 | 1626483 | +3617994 (124.90%) | N/A (N/A) |
| `root` | fleet-subnet-root | 7149541 | 2539590 | 16032374 | 3983835 | +8882833 (124.24%) | N/A (N/A) |
| `fleet_coordinator` | fleet-coordinator | 3496090 | 1214593 | 7849875 | 1868539 | +4353785 (124.53%) | N/A (N/A) |
| `wasm_store` | wasm-store | 2452234 | 889967 | 5563466 | 1394929 | +3111232 (126.87%) | N/A (N/A) |

## Governed Release Optimization

| Canister | Raw before → after | Gzip before → after | Code before → after | Data section before → after | Functions before → after |
| --- | ---: | ---: | ---: | ---: | ---: |
| `app` | 3064368 → 2885999 | 1019692 → 1038424 | 2824674 → 2649529 | 202317 → 199763 | 5353 → 4657 |
| `test` | 3400312 → 3200574 | 1119420 → 1139640 | 3140087 → 2943588 | 216369 → 213837 | 5924 → 5174 |
| `user_hub` | 3533534 → 3326921 | 1170636 → 1188287 | 3272186 → 3068953 | 220589 → 217932 | 6231 → 5450 |
| `scale_hub` | 3454082 → 3250522 | 1140913 → 1157419 | 3196131 → 2995832 | 218145 → 215593 | 6046 → 5280 |
| `user_shard` | 3458268 → 3254719 | 1139895 → 1160530 | 3197929 → 2997689 | 216889 → 214296 | 6080 → 5320 |
| `scale_replica` | 3075406 → 2896719 | 1024747 → 1043642 | 2834656 → 2659233 | 202709 → 200117 | 5383 → 4685 |
| `root` | 7593756 → 7149541 | 2470734 → 2539590 | 7149166 → 6709592 | 337873 → 334578 | 11026 → 9633 |
| `fleet_coordinator` | 3720063 → 3496090 | 1180746 → 1214593 | 3466506 → 3245678 | 201209 → 198867 | 5323 → 4463 |
| `wasm_store` | 2607459 → 2452234 | 870403 → 889967 | 2407282 → 2254883 | 179101 → 176894 | 4919 → 4239 |

There is no dedicated minimal role in scope. Component release spread is
`1.1528`; `root` is interpreted separately as Fleet Subnet
Root infrastructure and is `2.1490` times the largest
Component. Coordinator and Store are independently reported infrastructure.
No v1 raw/shrunk delta is
reported because that obsolete duplicate artifact model was removed.

## Structure And Retained-Size Evidence

| Canister | Functions | Data sections | Data bytes | Exports | Largest shallow item | Largest retained item |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `app` | 4694 | 179 | 198277 | 9 | `code[923]` (126498) | `table[0]` (1190384) |
| `test` | 5213 | 194 | 212228 | 11 | `code[1020]` (126498) | `table[0]` (1468514) |
| `user_hub` | 5489 | 195 | 216316 | 13 | `code[1091]` (126498) | `table[0]` (1586486) |
| `scale_hub` | 5319 | 194 | 213985 | 11 | `code[1047]` (126498) | `table[0]` (1515773) |
| `user_shard` | 5363 | 195 | 212680 | 12 | `code[1048]` (126498) | `table[0]` (1453073) |
| `scale_replica` | 4722 | 180 | 198623 | 10 | `code[929]` (126498) | `table[0]` (1199208) |
| `root` | 9678 | 283 | 332247 | 11 | `code[7684]` (244339) | `table[0]` (4660365) |
| `fleet_coordinator` | 4497 | 233 | 196954 | 6 | `code[147]` (255287) | `table[0]` (1240111) |
| `wasm_store` | 4278 | 164 | 175531 | 10 | `code[817]` (126542) | `table[0]` (1001371) |

All canonical release artifacts were accepted by `ic-wasm info`, `twiggy
top`, retained `top`, `dominators`, and `monos`. The builder's shrink step
removes source-level names, so current attribution is structural rather than a
claim about a particular crate. Repeated `table[0]`/element retention across
Components is a runtime fan-in signal; it is not sufficient by itself to assign a
dependency owner. Bounded dominator and monomorphization evidence is retained
in each role detail file.

The largest retained item occupies `65.1841%` of its canonical
release Wasm. Largest compatible predecessor growth is
`0.00%`; `0.00%` means either no positive growth or no
compatible predecessor.

## Risk Score

Risk score: **6 / 10**.

- no compatible v4 predecessor: +2.
- Component release spread 1.10-1.2499: +1.
- root/max-Component release ratio 2.0-2.9999: +1.
- largest retained item >= 25% of release Wasm: +2.

This is size-pressure evidence, not a correctness verdict. Root build-path
reproducibility remains owned by `CANIC-092-BUILD-001` and is neither cleared
nor duplicated here.

## Findings

- Method revision: v4 qualifies the canonical Binaryen transform and two-clean-build
  determinism across the full roster; valid v3 history cannot baseline v4.
- New product findings: none. The first v4 measurement is a baseline, and no
  comparable regression exists to attribute.

## Required Checklist

| Requirement | Result | Evidence |
| --- | --- | --- |
| clean isolated product snapshot | PASS | linked worktree clean before; tracked-clean after |
| canonical release artifacts | PASS | complete nine-role roster built through host `build_artifact` |
| deterministic clean release builds | PASS | exact Wasm, gzip, Candid, and transform metrics match across isolated targets |
| canonical debug artifacts | PASS | same nine roles and authority |
| governed Binaryen optimization | PASS | pinned tool plus before/after raw, gzip, code, data, and function metrics for every role |
| Candid/export/feature parity | PASS | release transform rejects before replacing an artifact if any governed invariant changes |
| builder gzip integrity | PASS | every gzip decodes to its paired canonical Wasm |
| machine-readable sizes | PASS | `size-metrics.tsv` |
| `ic-wasm info` | PASS | nine release artifacts parsed |
| `twiggy top` and retained `top` | PASS | compact hotspot columns retained |
| `twiggy dominators` | PASS | bounded role excerpts retained |
| `twiggy monos` | PASS | bounded role excerpts retained |
| compatible predecessor selection | PASS | exact method/roster/profile/path/tool keys; `N/A` |
| direct Cargo/pre-optimization fallback absent | PASS | v4 invokes only the host artifact authority |
| source mutation | PASS | no tracked mutation or unexpected untracked path |

## Verification Readout

| Command/check | Result | Notes |
| --- | --- | --- |
| `cargo run --offline --locked -p canic-host --example build_artifact -- <role> release ...` | PASS | nine ordered roles, repeated from isolated clean targets |
| same authoritative command with `debug` | PASS | nine ordered roles |
| `gzip -t` plus decoded SHA-256 equality | PASS | release and debug artifacts |
| `cmp` plus SHA-256 identity | PASS | two clean canonical release builds, all roles |
| `ic-wasm <release.wasm> info` | PASS | all roles |
| `twiggy top\|dominators\|monos <release.wasm>` | PASS | all roles |
| method composite | PASS | root-independent `7728ecc60f5618883bb0e8182e4c3fa9ab7ee9dd610363e37588c8f5497360ab` |
| product-tree identity | PASS | `c1fc28c7d730d274b91a2c0bcf0250d498c27181bf71c1ff38e8201a114edd23` |
| retained evidence hashes | PASS | manifest binds the report and compact artifacts |

## Retained Evidence

- [size summary](artifacts/wasm-footprint-v5/size-summary.md)
- [machine-readable metrics](artifacts/wasm-footprint-v5/size-metrics.tsv)
- [optimizer before/after metrics](artifacts/wasm-footprint-v5/optimization-metrics.tsv)
- [clean-build determinism](artifacts/wasm-footprint-v5/determinism.tsv)
- [method identity](artifacts/wasm-footprint-v5/method.json)
- [evidence manifest](artifacts/wasm-footprint-v5/evidence-manifest.yml)
- [app detail](artifacts/wasm-footprint-v5/app.md)
- [test detail](artifacts/wasm-footprint-v5/test.md)
- [user_hub detail](artifacts/wasm-footprint-v5/user_hub.md)
- [scale_hub detail](artifacts/wasm-footprint-v5/scale_hub.md)
- [user_shard detail](artifacts/wasm-footprint-v5/user_shard.md)
- [scale_replica detail](artifacts/wasm-footprint-v5/scale_replica.md)
- [root detail](artifacts/wasm-footprint-v5/root.md)
- [fleet_coordinator detail](artifacts/wasm-footprint-v5/fleet_coordinator.md)
- [wasm_store detail](artifacts/wasm-footprint-v5/wasm_store.md)
