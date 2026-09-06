# Wasm Ablation B1-05 - Shared CBOR Helper Reachability

## Verdict

- Run result: `pass`.
- Result validity: `valid` for optimized build-only shared-CBOR-helper
  attribution.
- Production decision: retain the current codecs; use the measured repeated
  footprint to prioritize role-selected persistence and capability-owned
  records before deciding whether any remaining codec needs replacement. Do
  not apply the audit switch as production code.

Replacing the shared bounded CBOR helper with the experiment's opaque stub
removes 9,332,399 artifact-summed optimized code-section bytes, 9,432,649
total Wasm bytes, 3,091,392 gzip bytes and 5,344 replica-limited defined
functions across the eleven canonical roles. The `runtime_probe` and
`blob_storage_probe` capability fixtures lose another 1,186,996 optimized code
bytes and 702 defined functions. These sums are attribution totals across
separately deployed artifacts, not one module or aggregate network headroom.

The canonical optimized-code reduction ranges from 595,788 bytes in Index
Child to 1,804,522 bytes in Root. Fleet Coordinator loses 1,380,816 bytes;
ordinary application roles generally lose about 675--706 KiB. The fixtures
lose 583,559 and 603,437 bytes. This is strong evidence that shared stable-
record serialization is a dominant, broadly reachable family, but the switch
intentionally overlaps the activation and authorization experiments and does
not isolate Ciborium itself from every direct caller.

## Scope And Identity

- Experiment: `b1-05-relevant-cbor-stub`.
- Immutable source: `v0.110.5` at
  `50f40171d6177c3d1e490b1fdb5f6163323b2cd5`.
- Source tree: `5a66988735c707b188d9d1fe03a3ed3b4ff7a273`.
- Artifacts: all eleven canonical Canic roles plus `runtime_probe` and
  `blob_storage_probe`.
- Immediate baseline: `b1-01-current-baseline`.
- Switch: audit-only patch
  `scripts/ci/wasm-ablation-patches/b1-05-relevant-cbor-stub.patch`.
- Switch SHA-256:
  `1f9f218fb741940c477b4e5b4a457f3f3fd17269d9f0e215da7fc484c55e25b7`.
- Applied switch-diff SHA-256:
  `2e61881a6d9441162345360fca58db1ffe511823e096c368b11d57040214a343`.
- Runner SHA-256:
  `4d3b8c6ee837052f2c0f635e5757e16c4ef3c289465fecab7f195019fc15860f`.
- Build-harness source SHA-256:
  `58f2453cae5124246666f8b53eca37040ef3bd2c829e624d7dec0e3864056f91`.
- Build-harness lock SHA-256:
  `bc1c13d66e8fad878a3b3d443a8a5c2d2d64bb45b8f7dfb894e004b93f5d0b62`.
- Environment: local, offline Cargo, disabled incremental compilation, no
  compiler wrapper and one fixed absolute target path removed and recreated
  before every repetition.

The switch preserves the shared helper's generic signatures and Serde bounds,
but returns an opaque bounded byte from encoding and an opaque typed failure
from decoding. `core::hint::black_box` prevents constant-folding the
experiment result through its callers. The switch reaches every caller of
`canic_core::cdk::serialize`; direct Ciborium users remain unchanged. It
therefore provides shared-helper attribution only and proves no stable codec,
restore, recovery or runtime parity.

## Optimized Result

| Artifact | Wasm delta | Gzip delta | Code delta | Code delta % | Defined-function delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| App | -703,648 | -232,056 | -696,139 | -26.2622% | -406 |
| Index Hub | -683,167 | -225,518 | -675,676 | -27.6395% | -401 |
| Test | -704,287 | -233,853 | -696,852 | -23.6640% | -413 |
| User Hub | -713,332 | -238,168 | -705,828 | -22.9743% | -407 |
| Scale Hub | -706,854 | -234,975 | -699,335 | -23.3341% | -406 |
| Index Child | -601,582 | -202,318 | -595,788 | -26.7816% | -350 |
| User Shard | -704,329 | -232,652 | -696,832 | -23.2169% | -411 |
| Scale Replica | -703,633 | -232,218 | -696,122 | -26.1658% | -406 |
| Root | -1,821,605 | -584,131 | -1,804,522 | -27.0960% | -937 |
| Fleet Coordinator | -1,395,712 | -445,091 | -1,380,816 | -42.5219% | -759 |
| Wasm Store | -694,500 | -230,412 | -684,489 | -30.3594% | -448 |
| Runtime probe | -592,501 | -201,208 | -583,559 | -26.5409% | -357 |
| Blob-storage probe | -612,531 | -205,965 | -603,437 | -28.8382% | -345 |

Across the eleven canonical artifact vectors, the switch also removes 93,197
data-section bytes and 633 indirect-table and element entries. The two
fixtures separately lose 17,125 data-section bytes and 86 table and element
entries. It changes no Wasm export count, exported IC method count, Candid
size, Candid method count or Candid hash.

## Determinism And Structured Evidence

Both clean baseline builds are byte-identical across Wasm, gzip and Candid and
have identical complete metric vectors for all thirteen artifacts. Both clean
variant builds satisfy the same checks. The variant preserves every
artifact's baseline Candid hash.

All 52 method-owned transform records use schema version 1, name the exact
role and report typed `applied` outcomes for `shrink`, `candid_metadata` and
`optimize`. Each optimizer outcome carries its numeric before/after vector. No
pass decision or retained metric depends on explanatory build-log prose.

## Interpretation

The measurement confirms that stable-record encoding and decoding through the
shared helper is one of the largest repeated Wasm families in the retained
baseline. It deliberately does not say that 9.3 MiB can be removed from one
canister or that the helper can be deleted. The result overlaps row 3's
activation persistence and row 4's authorization persistence, and it includes
other stable domains selected by each role.

The production ordering remains: first prevent roles from reaching storage
they do not own, then split still-whole records by capability, and only then
remeasure residual codec families. A positional or manual stable codec is
justified only where that residual evidence remains material. Any production
cut must preserve bounded decoding, synchronous restore, corruption rejection,
same-release interruption recovery and exact current-schema behavior.

## Verification

| Check | Result |
| --- | --- |
| exact immutable source and product lockfile | PASS |
| separate hash-bound method lockfile | PASS |
| clean linked worktree before and after | PASS |
| authoritative `canic-host` release builder | PASS |
| typed transform schema, outcomes and numeric metrics | PASS |
| two clean baseline builds for thirteen artifacts | PASS |
| two clean variant builds for thirteen artifacts | PASS |
| Wasm, gzip, Candid and metric determinism | PASS |
| `wasm-validate`, `gzip -t` and `didc check` | PASS |
| independent replica-limited function counter | PASS |
| exact hash-bound one-switch path set | PASS |
| audit patch reversed | PASS |
| codec, persistence, restore and runtime parity | OPEN |

## Retained Evidence

- [artifact metrics](artifacts/wasm-ablation-b1-05/artifact-metrics.tsv)
- [determinism](artifacts/wasm-ablation-b1-05/determinism.tsv)
- [run metadata](artifacts/wasm-ablation-b1-05/run-metadata.tsv)
- [evidence manifest](artifacts/wasm-ablation-b1-05/evidence-manifest.yml)
