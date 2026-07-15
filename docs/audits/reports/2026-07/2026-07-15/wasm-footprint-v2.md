# Wasm Footprint Audit v2 - 2026-07-15

## Verdict

- Run result: `pass`.
- Result validity: `valid`.
- Comparability: `first-v2-baseline`.
- Authoritative risk score: `4/10`.

V2 completed fresh release and debug builds for all six attached roles through
Canic's authoritative host artifact builder. It did not invoke direct Cargo
Wasm compilation, infer a target-directory artifact, or recreate a pre-shrink
metric. This closes the executable-method defect in
`CANIC-092-AUDIT-016`; the measured result creates no new product finding.

## Scope And Identity

- Definition: `docs/audits/recurring/system/wasm-footprint.md`.
- Compared predecessor: `N/A`.
- Original v2 baseline: `docs/audits/reports/2026-07/2026-07-15/wasm-footprint-v2.md`.
- Release anchor: `v0.92.0`.
- Source commit: `91736337fc1cfeb891f17d7d62affb5e671348e2`.
- Source tree: `fd31bb8289365a38f2bea7f8ebd6973908ee959f`.
- Product tree: `c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e`.
- Method: `CANIC-WASM-001/v2`; definition `e33fc36ee904fa6a9af8c7aa399a94b98c441e25fe6590ac1548c548ba2f3ffb`; executable
  composite `c88cdf0ef4e79b3c1702ef9c00777f5a7940c4771ab51dbf8442fabd71396bc5`.
- Ordered roster: `app,user_hub,user_shard,scale_hub,scale_replica,root`.
- Profiles: `release+debug`.
- Branch/worktree: `detached`; clean disposable linked worktree before the
  run, tracked-clean after the run, with only permitted `.icp/` build output.
- Environment: local, offline, isolated `CARGO_TARGET_DIR`; no replica,
  credentials, deployment, or authoritative external mutation.
- Auditor: Codex.
- Started/completed: `2026-07-15T09:16:20Z` / `2026-07-15T09:25:08Z`.

## Immutable Run Identity

```text
release_anchor: v0.92.0
source_commit_full: 91736337fc1cfeb891f17d7d62affb5e671348e2
source_tree_hash: fd31bb8289365a38f2bea7f8ebd6973908ee959f
product_tree_hash: c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e
clean_worktree: true before; tracked-clean after; generated .icp only
cargo_lock_hash: 6cd75f146077bbf3f254fda608f1265531d1065ce0cd9c1bb56d67118f3de5cc
rust_toolchain: rustc 1.97.0 (2d8144b78 2026-07-07); cargo 1.97.0 (c980f4866 2026-06-30)
target_triple: wasm32-unknown-unknown
feature_set: fleets/test attached six-role roster
audit_method_id: CANIC-WASM-001
audit_method_version: 2
audit_method_fingerprint: c88cdf0ef4e79b3c1702ef9c00777f5a7940c4771ab51dbf8442fabd71396bc5
audit_script_hashes: definition=e33fc36ee904fa6a9af8c7aa399a94b98c441e25fe6590ac1548c548ba2f3ffb; executable-composite=c88cdf0ef4e79b3c1702ef9c00777f5a7940c4771ab51dbf8442fabd71396bc5
external_tool_versions: icp 1.0.2; ic-wasm 0.9.11; twiggy-opt 0.8.0
fixture_or_seed: fleets/test/canic.toml@91736337fc1cfeb891f17d7d62affb5e671348e2; roster=app,user_hub,user_shard,scale_hub,scale_replica,root
environment_class: isolated local linked-worktree execution_trace
execution_path_key: 42df7e27405a64065384e3968ff61e27177a042ccf6be5b09b58b330355dba5c
started_at: 2026-07-15T09:16:20Z
completed_at: 2026-07-15T09:25:08Z
```

The execution path itself is not retained. Its hash is a comparison key because
the independently owned `CANIC-092-BUILD-001` path-dependence finding makes a
different checkout path non-comparable for gzip/byte continuity.

## Canonical Artifact Sizes

| Canister | Kind | Release Wasm | Release gzip | Debug Wasm | Debug gzip | Debug delta | Predecessor delta |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `app` | leaf-canister | 2251324 | 747279 | 4816015 | 1219145 | +2564691 (113.92%) | N/A (N/A) |
| `user_hub` | leaf-canister | 2369688 | 789228 | 5094882 | 1291598 | +2725194 (115.00%) | N/A (N/A) |
| `user_shard` | leaf-canister | 2368860 | 789006 | 5079760 | 1287649 | +2710900 (114.44%) | N/A (N/A) |
| `scale_hub` | leaf-canister | 2290404 | 759760 | 4902642 | 1237989 | +2612238 (114.05%) | N/A (N/A) |
| `scale_replica` | leaf-canister | 2260688 | 752095 | 4835444 | 1225414 | +2574756 (113.89%) | N/A (N/A) |
| `root` | bundle-canister | 3845263 | 1742162 | 7858899 | 2772940 | +4013636 (104.38%) | N/A (N/A) |

There is no dedicated minimal role in scope. Leaf release spread is
`1.0526`; `root` is interpreted separately as a bundle canister
and is `1.6227` times the largest leaf. No v1 raw/shrunk delta is
reported because that obsolete duplicate artifact model was removed.

## Structure And Retained-Size Evidence

| Canister | Functions | Data sections | Data bytes | Exports | Largest shallow item | Largest retained item |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `app` | 4411 | 3 | 189240 | 21 | `data[0]` (188666) | `table[0]` (1381904) |
| `user_hub` | 4674 | 3 | 194456 | 25 | `data[0]` (193858) | `table[0]` (1481564) |
| `user_shard` | 4657 | 3 | 194136 | 26 | `data[0]` (193554) | `table[0]` (1484243) |
| `scale_hub` | 4486 | 3 | 191796 | 24 | `data[0]` (191218) | `table[0]` (1408624) |
| `scale_replica` | 4438 | 3 | 189372 | 22 | `data[0]` (188794) | `table[0]` (1389740) |
| `root` | 6030 | 3 | 890540 | 44 | `data[0]` (889806) | `table[0]` (2129488) |

All canonical release artifacts were accepted by `ic-wasm info`, `twiggy
top`, retained `top`, `dominators`, and `monos`. The builder's shrink step
removes source-level names, so current attribution is structural rather than a
claim about a particular crate. Repeated `table[0]`/element retention across
leaves is a runtime fan-in signal; it is not sufficient by itself to assign a
dependency owner. Bounded dominator and monomorphization evidence is retained
in each role detail file.

The largest retained item occupies `62.6564%` of its canonical
release Wasm. Largest compatible predecessor growth is
`0.00%`; `0.00%` means either no positive growth or no
compatible predecessor.

## Risk Score

Risk score: **4 / 10**.

- no compatible v2 predecessor: +2.
- largest retained item >= 25% of release Wasm: +2.

This is size-pressure evidence, not a correctness verdict. Root build-path
reproducibility remains owned by `CANIC-092-BUILD-001` and is neither cleared
nor duplicated here.

## Findings

- `CANIC-092-AUDIT-016`: fixed by v2's root-independent executable identity
  and sole authoritative artifact path.
- New product findings: none. The first v2 measurement is a baseline, and no
  comparable regression exists to attribute.

## Required Checklist

| Requirement | Result | Evidence |
| --- | --- | --- |
| clean isolated product snapshot | PASS | linked worktree clean before; tracked-clean after |
| canonical release artifacts | PASS | six roles built through host `build_artifact` |
| canonical debug artifacts | PASS | same six roles and authority |
| builder gzip integrity | PASS | every gzip decodes to its paired canonical Wasm |
| machine-readable sizes | PASS | `size-metrics.tsv` |
| `ic-wasm info` | PASS | six release artifacts parsed |
| `twiggy top` and retained `top` | PASS | compact hotspot columns retained |
| `twiggy dominators` | PASS | bounded role excerpts retained |
| `twiggy monos` | PASS | bounded role excerpts retained |
| compatible predecessor selection | PASS | exact method/roster/profile/path/tool keys; `N/A` |
| direct Cargo/pre-shrink fallback absent | PASS | v2 invokes only the host artifact authority |
| source mutation | PASS | no tracked mutation or unexpected untracked path |

## Verification Readout

| Command/check | Result | Notes |
| --- | --- | --- |
| `cargo run --offline --locked -p canic-host --example build_artifact -- <role> release ...` | PASS | six ordered roles |
| same authoritative command with `debug` | PASS | six ordered roles |
| `gzip -t` plus decoded SHA-256 equality | PASS | release and debug artifacts |
| `ic-wasm <release.wasm> info` | PASS | all roles |
| `twiggy top\|dominators\|monos <release.wasm>` | PASS | all roles |
| method composite | PASS | root-independent `c88cdf0ef4e79b3c1702ef9c00777f5a7940c4771ab51dbf8442fabd71396bc5` |
| product-tree identity | PASS | `c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e` |
| retained evidence hashes | PASS | manifest binds the report and compact artifacts |

## Retained Evidence

- [size summary](artifacts/wasm-footprint-v2/size-summary.md)
- [machine-readable metrics](artifacts/wasm-footprint-v2/size-metrics.tsv)
- [method identity](artifacts/wasm-footprint-v2/method.json)
- [evidence manifest](artifacts/wasm-footprint-v2/evidence-manifest.yml)
- [app detail](artifacts/wasm-footprint-v2/app.md)
- [user_hub detail](artifacts/wasm-footprint-v2/user_hub.md)
- [user_shard detail](artifacts/wasm-footprint-v2/user_shard.md)
- [scale_hub detail](artifacts/wasm-footprint-v2/scale_hub.md)
- [scale_replica detail](artifacts/wasm-footprint-v2/scale_replica.md)
- [root detail](artifacts/wasm-footprint-v2/root.md)
