# Wasm Ablation B1-02 - Global Stable-Storage Registration

## Verdict

- Run result: `pass`.
- Result validity: `valid` for optimized footprint attribution.
- Production decision: replace global registration only through a role-selected
  implementation that separately proves storage bootstrap, restore and
  lifecycle parity; do not apply the audit patch as production code.

Global constructor registration is a material runtime contributor. Across the
eleven separately deployed canonical artifacts, the build-only switch removes
273,554 optimized code-section bytes, 288,870 total Wasm bytes, 97,166 gzip
bytes and 662 replica-limited defined functions. Those sums are attribution
totals, not one deployable artifact or a claim of aggregate network headroom.

The result is asymmetric. Fleet Coordinator alone loses 192,340 optimized code
bytes and 166 defined functions, a 5.9231% code-section reduction. The other
roles lose 6,951 to 11,085 code bytes and 43 to 75 defined functions each.
That concentration proves that unconditional registration retains a much
larger storage-related graph in Coordinator than in the other roles; this
experiment does not determine which declarations remain behaviorally required.

## Scope And Identity

- Experiment: `b1-02-global-storage-registration`.
- Immutable source: `v0.110.5` at
  `50f40171d6177c3d1e490b1fdb5f6163323b2cd5`.
- Source tree: `5a66988735c707b188d9d1fe03a3ed3b4ff7a273`.
- Artifacts: all eleven canonical Canic roles.
- Immediate baseline: `b1-01-current-baseline`.
- Switch: audit-only patch
  `scripts/ci/wasm-ablation-patches/b1-02-global-storage-registration.patch`.
- Switch SHA-256:
  `9b66e1d344c4f73df993e50fba243dbf6f601fe876ead62b155868649cce7c13`.
- Runner SHA-256:
  `7ef5b737498a5a16571a26b3532b64b518cf1fcf4ff90275d9ef5f73e8ba1973`.
- Environment: local, offline Cargo, disabled incremental compilation, one
  fixed absolute target path removed and recreated before every repetition,
  with the frozen worktree's stable sccache wrapper selected explicitly.

The switch removes static memory-declaration and authority-range constructors
plus eager TLS initializer registration. It retains the stable-key open path,
the marker-type reference and the runtime bootstrap-ready assertion. This is a
destructive build-only attribution: by itself it does not populate the memory
manifest, reserve authority ranges, eagerly initialize stable stores or prove
install/reinstall and same-release recovery behavior.

## Optimized Result

| Artifact | Wasm delta | Gzip delta | Code delta | Code delta % | Defined-function delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| App | -8,527 | -2,884 | -7,681 | -0.2898% | -46 |
| Index Hub | -7,765 | -2,012 | -6,951 | -0.2843% | -43 |
| Test | -8,512 | -2,882 | -7,671 | -0.2605% | -45 |
| User Hub | -8,033 | -3,123 | -7,279 | -0.2369% | -46 |
| Scale Hub | -7,728 | -3,003 | -6,965 | -0.2324% | -44 |
| Index Child | -10,160 | -2,484 | -9,031 | -0.4060% | -46 |
| User Shard | -8,526 | -2,655 | -7,673 | -0.2556% | -45 |
| Scale Replica | -8,535 | -3,031 | -7,681 | -0.2887% | -46 |
| Root | -12,140 | -4,256 | -11,085 | -0.1664% | -75 |
| Fleet Coordinator | -198,400 | -67,334 | -192,340 | -5.9231% | -166 |
| Wasm Store | -10,544 | -3,502 | -9,197 | -0.4079% | -60 |

Every artifact also loses data bytes and indirect-table entries. No artifact
changes its Wasm exports, exported IC methods, Candid bytes or Candid service-
method count. Full baseline, variant and pre-optimizer vectors are retained in
the machine-readable artifact table.

## Determinism

Both clean baseline builds are byte-identical across Wasm, gzip and Candid and
have identical complete metric vectors for every canonical role. Both clean
variant builds satisfy the same checks. The variant preserves each role's
baseline Candid hash. The audit patch was reversed and the immutable product
worktree was clean after the run.

## Interpretation

The existing global constructor graph is architectural supporting machinery,
not a second mutable storage authority. The measurement nevertheless confirms
that it carries real optimized reachability, especially in Fleet Coordinator.
This strengthens the accepted B2 direction: emit direct role-selected storage
declarations, range reservations and initialization calls instead of scanning
or constructing a whole reachable registration graph.

The audit patch is not that implementation. Removing constructors and eager
TLS registration without a replacement would weaken the exact memory manifest,
authority-range collision checks and lifecycle initialization contract. The
next production slice must preserve those invariants with explicit role-local
wiring, then prove install, synchronous restore, same-release interruption
recovery and optimized absence before taking the measured savings.

## Verification

| Check | Result |
| --- | --- |
| exact immutable source and lockfile | PASS |
| clean linked worktree before and after | PASS |
| authoritative `canic-host` release builder | PASS |
| two clean baseline builds for eleven roles | PASS |
| two clean variant builds for eleven roles | PASS |
| Wasm, gzip, Candid and metric determinism | PASS |
| `wasm-validate`, `gzip -t` and `didc check` | PASS |
| independent replica-limited function counter | PASS |
| exact one-switch path set | PASS |
| audit patch reversed | PASS |
| storage bootstrap and lifecycle parity | OPEN |

## Retained Evidence

- [artifact metrics](artifacts/wasm-ablation-b1-02/artifact-metrics.tsv)
- [determinism](artifacts/wasm-ablation-b1-02/determinism.tsv)
- [run metadata](artifacts/wasm-ablation-b1-02/run-metadata.tsv)
- [evidence manifest](artifacts/wasm-ablation-b1-02/evidence-manifest.yml)
