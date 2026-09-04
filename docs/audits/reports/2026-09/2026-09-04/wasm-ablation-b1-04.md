# Wasm Ablation B1-04 - Authorization Persistence Integration

## Verdict

- Run result: `pass`.
- Result validity: `valid` for optimized build-only stable authorization-
  persistence integration attribution.
- Production decision: retain authorization behavior and stable persistence;
  use the measured repeated footprint to design role-selected ownership and a
  narrower current persistence boundary. Do not apply the audit switch as
  production code.

Replacing the stable authorization-state cell with the experiment's heap cell
removes 1,628,872 artifact-summed optimized code-section bytes, 1,649,309
total Wasm bytes, 562,521 gzip bytes and 897 replica-limited defined functions
across the eleven canonical roles. The independent runtime capability fixture
loses another 148,935 optimized code bytes and 88 defined functions. These
sums are attribution totals, not one deployable module or aggregate network
headroom.

Every measured artifact carries a large share. The canonical optimized-code
reduction ranges from 146,406 bytes in Wasm Store to 151,837 bytes in Fleet
Coordinator; the runtime fixture loses 148,935 bytes. That repeated footprint
is strong evidence that stable authorization persistence machinery is linked
more broadly than its owning capabilities require. The switch does not remove
the authorization record, policy, workflow or endpoint call graph, so it does
not attribute those domains individually.

## Scope And Identity

- Experiment: `b1-04-authorization-record-codecs`.
- Immutable source: `v0.110.5` at
  `50f40171d6177c3d1e490b1fdb5f6163323b2cd5`.
- Source tree: `5a66988735c707b188d9d1fe03a3ed3b4ff7a273`.
- Artifacts: all eleven canonical Canic roles plus `runtime_probe`.
- Immediate baseline: `b1-01-current-baseline`.
- Switch: audit-only patch
  `scripts/ci/wasm-ablation-patches/b1-04-authorization-record-codecs.patch`.
- Switch SHA-256:
  `0acbbfc0677aaab1e078e6ba8de203a2da0470d6e4a55e66e209e41c89f13f2b`.
- Applied switch-diff SHA-256:
  `42ed7228ef4fa9c09ffa6c67acde6a7e8dd766bc0c950b0299c0c5e04c434be9`.
- Runner SHA-256:
  `4d3b8c6ee837052f2c0f635e5757e16c4ef3c289465fecab7f195019fc15860f`.
- Build-harness source SHA-256:
  `58f2453cae5124246666f8b53eca37040ef3bd2c829e624d7dec0e3864056f91`.
- Build-harness lock SHA-256:
  `bc1c13d66e8fad878a3b3d443a8a5c2d2d64bb45b8f7dfb894e004b93f5d0b62`.
- Environment: local, offline Cargo, disabled incremental compilation, no
  compiler wrapper and one fixed absolute target path removed and recreated
  before every repetition.

The switch preserves `AuthStateRecord`, the authorization operations and the
runtime authorization call graph, but replaces the stable cell with an
in-memory cell for the duration of each build. It therefore disconnects stable
cell initialization and the record's stable serialization integration while
preserving same-execution heap access. This is deliberately a destructive
build-only attribution. It provides no persistence, restore, upgrade,
interruption-recovery or authorization-behavior parity.

## Optimized Result

| Artifact | Wasm delta | Gzip delta | Code delta | Code delta % | Defined-function delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| App | -149,615 | -51,190 | -147,812 | -5.5763% | -79 |
| Index Hub | -149,949 | -50,828 | -148,140 | -6.0599% | -81 |
| Test | -149,874 | -50,665 | -148,077 | -5.0285% | -84 |
| User Hub | -149,658 | -51,098 | -147,862 | -4.8128% | -78 |
| Scale Hub | -149,663 | -51,435 | -147,864 | -4.9337% | -78 |
| Index Child | -150,060 | -50,820 | -148,266 | -6.6648% | -81 |
| User Shard | -149,881 | -50,909 | -148,076 | -4.9336% | -83 |
| Scale Replica | -149,615 | -51,339 | -147,812 | -5.5559% | -79 |
| Root | -148,241 | -51,033 | -146,720 | -2.2031% | -80 |
| Fleet Coordinator | -154,527 | -53,394 | -151,837 | -4.6758% | -90 |
| Wasm Store | -148,226 | -49,810 | -146,406 | -6.4936% | -84 |
| Runtime probe | -150,744 | -51,485 | -148,935 | -6.7737% | -88 |

Across the eleven canonical artifact vectors, the switch also removes 19,245
data-section bytes and 143 indirect-table and element entries. It changes
neither Wasm export counts nor exported IC method counts, Candid sizes, Candid
method counts or any artifact's Candid hash. The runtime fixture separately
loses 1,694 data-section bytes and 13 table and element entries while
preserving those same public-surface metrics. Full baseline, variant and pre-
optimizer vectors are retained in the machine-readable artifact table.

## Determinism And Structured Evidence

Both clean baseline builds are byte-identical across Wasm, gzip and Candid and
have identical complete metric vectors for all twelve artifacts. Both clean
variant builds satisfy the same checks. The variant preserves every artifact's
baseline Candid hash.

All 48 method-owned transform records use schema version 1, name the exact role
and report typed `applied` outcomes for `shrink`, `candid_metadata` and
`optimize`. Each optimizer outcome carries its numeric before/after vector. No
pass decision or retained metric depends on explanatory build-log prose.

## Interpretation

The measurement confirms a large, nearly uniform stable authorization-
persistence footprint across roles with materially different authorization
responsibilities. It does not prove the authorization state is sediment.
Current authorization records support delegated sessions, issuer policy,
proofs, renewal, revocation and exact restart behavior; those current safety
properties remain required wherever the owning capability is selected.

The actionable finding is narrower. B2 should make the stable authorization
allocation and its codec reachability role-selected. Later contraction should
separate the minimum record families required by verifier, issuer, Root and
zero-auth roles, then remeasure the complete canonical and capability-fixture
matrix. Any production cut must preserve synchronous restore, exact authority
validation, same-release interruption recovery and effect-free replay. This
experiment alone authorizes no source deletion.

## Verification

| Check | Result |
| --- | --- |
| exact immutable source and product lockfile | PASS |
| separate hash-bound method lockfile | PASS |
| clean linked worktree before and after | PASS |
| authoritative `canic-host` release builder | PASS |
| typed transform schema, outcomes and numeric metrics | PASS |
| two clean baseline builds for twelve artifacts | PASS |
| two clean variant builds for twelve artifacts | PASS |
| Wasm, gzip, Candid and metric determinism | PASS |
| `wasm-validate`, `gzip -t` and `didc check` | PASS |
| independent replica-limited function counter | PASS |
| exact hash-bound one-switch path set | PASS |
| audit patch reversed | PASS |
| persistence, restore and authorization parity | OPEN |

## Retained Evidence

- [artifact metrics](artifacts/wasm-ablation-b1-04/artifact-metrics.tsv)
- [determinism](artifacts/wasm-ablation-b1-04/determinism.tsv)
- [run metadata](artifacts/wasm-ablation-b1-04/run-metadata.tsv)
- [evidence manifest](artifacts/wasm-ablation-b1-04/evidence-manifest.yml)
