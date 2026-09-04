# Wasm Ablation B1-03 - Activation Persistence Family

## Verdict

- Run result: `pass`.
- Result validity: `valid` for inclusive optimized-footprint attribution.
- Production decision: retain activation behavior and use the measured pressure
  to design the smallest role-selected current activation record and codec
  family; do not apply the fail-closed audit switch as production code.

The combined activation record, stable codec, mapper and storage-operation
family is a material runtime contributor. Across the eleven separately
deployed canonical artifacts, the build-only switch removes 3,001,136
optimized code-section bytes, 3,057,014 total Wasm bytes, 1,036,468 gzip bytes
and 2,025 replica-limited defined functions. Those sums are attribution totals,
not one deployable module or aggregate network headroom.

Every role carries a large share. The optimized code reduction ranges from
187,635 bytes in Fleet Coordinator to 297,260 bytes in Wasm Store, and the
defined-function reduction ranges from 124 to 259. The eight ordinary App,
Hub, Child, Shard and Replica shapes each lose approximately 288 to 289 KiB of
optimized code despite their different endpoint surfaces. That repeated
footprint is strong evidence for capability-owned, role-selected activation
persistence, but this inclusive experiment cannot distinguish the record,
codec, mapper and operation contributions from one another.

## Scope And Identity

- Experiment: `b1-03-activation-record-codecs`.
- Immutable source: `v0.110.5` at
  `50f40171d6177c3d1e490b1fdb5f6163323b2cd5`.
- Source tree: `5a66988735c707b188d9d1fe03a3ed3b4ff7a273`.
- Artifacts: all eleven canonical Canic roles.
- Immediate baseline: `b1-01-current-baseline`.
- Switch: audit-only patch
  `scripts/ci/wasm-ablation-patches/b1-03-activation-record-codecs.patch`.
- Switch SHA-256:
  `a328aa466efe264d4edd497b105121e3c055235081fc3d63e227abb55199f79b`.
- Applied switch-diff SHA-256:
  `7f291a3b62319eea47bc14f59af84bf585f971774ff77ad6c6b27f5c16a3e350`.
- Runner SHA-256:
  `107185281a48aa6b325f2a3db831e6ba58cf23cb73685ffa90c7b24dd47d9d13`.
- Build-harness source SHA-256:
  `58f2453cae5124246666f8b53eca37040ef3bd2c829e624d7dec0e3864056f91`.
- Build-harness lock SHA-256:
  `bc1c13d66e8fad878a3b3d443a8a5c2d2d64bb45b8f7dfb894e004b93f5d0b62`.
- Environment: local, offline Cargo, disabled incremental compilation, no
  compiler wrapper and one fixed absolute target path removed and recreated
  before every repetition.

The switch disconnects the activation stable record, its CBOR codec and mapper,
and the storage-operation implementation. It preserves the existing caller
signatures, endpoint-mode model, state-manifest identity and fail-closed error
surface through opaque operation results. This is deliberately an inclusive,
destructive build-only attribution. It provides no activation, persistence,
install, restore, interruption-recovery or runtime parity.

## Optimized Result

| Artifact | Wasm delta | Gzip delta | Code delta | Code delta % | Defined-function delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| App | -293,098 | -99,008 | -288,120 | -10.8695% | -187 |
| Index Hub | -293,071 | -99,204 | -288,226 | -11.7903% | -188 |
| Test | -293,258 | -98,742 | -288,276 | -9.7894% | -189 |
| User Hub | -294,107 | -99,697 | -289,127 | -9.4109% | -186 |
| Scale Hub | -294,128 | -99,394 | -289,136 | -9.6474% | -186 |
| Index Child | -293,969 | -99,863 | -288,983 | -12.9902% | -192 |
| User Shard | -293,590 | -98,607 | -288,588 | -9.6151% | -188 |
| Scale Replica | -293,106 | -99,089 | -288,120 | -10.8298% | -187 |
| Root | -211,728 | -73,958 | -207,665 | -3.1182% | -139 |
| Fleet Coordinator | -192,609 | -66,497 | -187,635 | -5.7782% | -124 |
| Wasm Store | -304,350 | -102,409 | -297,260 | -13.1845% | -259 |

Across the artifact-summed vectors, the switch also removes 53,261 data-
section bytes and 244 indirect-table and element entries. It changes neither
Wasm export counts nor exported IC method counts, Candid sizes, Candid method
counts or any role's Candid hash. Full baseline, variant and pre-optimizer
vectors are retained in the machine-readable artifact table.

## Determinism And Structured Evidence

Both clean baseline builds are byte-identical across Wasm, gzip and Candid and
have identical complete metric vectors for every canonical role. Both clean
variant builds satisfy the same checks. The variant preserves each role's
baseline Candid hash.

All 44 method-owned transform records use schema version 1, name the exact
artifact role and report typed `applied` outcomes for `shrink`,
`candid_metadata` and `optimize`. Each optimizer outcome carries its numeric
before/after vector. No pass decision or retained metric depends on explanatory
build-log prose.

## Interpretation

The measurement confirms that the current activation persistence family has a
large repeated optimized footprint. It does not prove that activation itself
is sediment: activation binds release identity, Fleet authority, Component
runtime preparation, credential generation and the Prepared-to-Active fence.
Those are current safety behavior and remain required.

The actionable finding is narrower. B2 should first make storage reachability
role-selected. B3 should then use fresh-install-only reconstruction evidence to
separate the minimum current activation data each role owns, avoid retaining
unselected record and codec machinery, and remeasure the complete canonical
and capability-fixture matrix. Any production cut must preserve synchronous
restore, endpoint fencing, exact authority validation, same-release recovery
and effect-free replay. This experiment alone authorizes no source deletion.

## Verification

| Check | Result |
| --- | --- |
| exact immutable source and product lockfile | PASS |
| separate hash-bound method lockfile | PASS |
| clean linked worktree before and after | PASS |
| authoritative `canic-host` release builder | PASS |
| typed transform schema, outcomes and numeric metrics | PASS |
| two clean baseline builds for eleven roles | PASS |
| two clean variant builds for eleven roles | PASS |
| Wasm, gzip, Candid and metric determinism | PASS |
| `wasm-validate`, `gzip -t` and `didc check` | PASS |
| independent replica-limited function counter | PASS |
| exact hash-bound one-switch path set | PASS |
| audit patch reversed | PASS |
| activation, restore and recovery parity | OPEN |

## Retained Evidence

- [artifact metrics](artifacts/wasm-ablation-b1-03/artifact-metrics.tsv)
- [determinism](artifacts/wasm-ablation-b1-03/determinism.tsv)
- [run metadata](artifacts/wasm-ablation-b1-03/run-metadata.tsv)
- [evidence manifest](artifacts/wasm-ablation-b1-03/evidence-manifest.yml)
