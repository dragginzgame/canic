# B1 endpoint reply-serialization measurement

Reviewed: 2026-09-22. Experiment: `b1-10-candid-serialization-newtypes`.
Verdict: complete controlled footprint measurement; material endpoint-reply
reachability, with intentionally invalid replies and no production deletion.

## Source and verification

The retained run completes successfully under
`.tmp/b1-continuation-20260922/row10/`. It builds all fourteen selected artifacts
twice for both the unchanged baseline and the audit variant. Both conditions
repeat exactly; the product checkout restores before the queued metrics run
starts. No build was repeated solely for this review.

Source is frozen `v0.110.5`, commit
`50f40171d6177c3d1e490b1fdb5f6163323b2cd5`, with unchanged product lock
`0fd6c7897d08e6a0f4e436caaf319ba24ebc32236434010b5c6ae3507f663147`.
The exact patch is
`d3f9497af1bf9e23db5cf45881bb6ca3c59ee2676900d1063d8389f165d2a813`.
Rust/Cargo are 1.97.1, ic-wasm 0.11.1, didc 0.5.4 and WABT 1.0.34. The
[run metadata](b1-row10-measurement/run-metadata.tsv) retains method, harness
lock, counter and tool identities.

Independent [verification](b1-row10-measurement/verification.json) checks the
complete selector/condition/repetition matrix, every Wasm/gzip/Candid digest and
length, gzip roundtrips, all paired metric fields and determinism records,
method-source hashes and the harness lock. A separate direct Wasm-section read
confirms local function counts against both recorded counters. Candid bytes and
export name/kind identities match across the baseline and variant for every
artifact. All 56 artifact vectors are retained in the
[metrics](b1-row10-measurement/artifact-metrics.tsv), with complete
[determinism records](b1-row10-measurement/determinism.tsv).

## Matched footprint

Deltas are variant minus this run's own baseline, after optimization. The
replica-limited and optimizer-defined function deltas agree. Table minimum and
element-entry deltas also agree for every artifact.

| Artifact | Code bytes | Defined functions | Table entries | Gzip bytes |
| --- | ---: | ---: | ---: | ---: |
| App | -75,046 | -155 | -106 | -24,328 |
| Index Hub | -71,069 | -144 | -95 | -24,393 |
| Test | -82,146 | -165 | -115 | -27,251 |
| User Hub | -81,631 | -163 | -113 | -27,762 |
| Scale Hub | -77,726 | -156 | -108 | -26,558 |
| Index Child | -68,001 | -142 | -94 | -22,154 |
| User Shard | -80,187 | -162 | -112 | -26,032 |
| Scale Replica | -76,363 | -156 | -107 | -25,005 |
| Root | -169,788 | -331 | -195 | -50,946 |
| Fleet Coordinator | -58,705 | -126 | -74 | -19,720 |
| Wasm Store | -37,218 | -78 | -52 | -13,105 |
| Runtime probe | -58,050 | -120 | -82 | -22,277 |
| Payload-limit probe | -46,480 | -102 | -65 | -16,551 |
| Blob-storage probe | -69,653 | -132 | -87 | -24,292 |

The eleven separately deployed canonical artifacts total 877,880 fewer code
bytes, 999,416 fewer raw Wasm bytes, 287,254 fewer gzip bytes and 1,778 fewer
defined functions. These cross-artifact sums are not one module's headroom and
must not be added to overlapping ablations. Root alone accounts for 169,788 code
bytes and 331 defined functions. Complete absolute vectors, including data
bytes and exports, remain in the metrics above.

## Interpretation and remaining work

The switch replaces endpoint reply encoding with an opaque one-byte response
while keeping source-level signatures, request decoding, endpoint dispatch,
explicit authorization, payload checks and declaration registration. It does
not produce valid Candid replies. Direct inter-canister encoders and request
deserialization remain outside the switch.

This is inclusive attribution at the endpoint reply boundary. Removing the use
of a typed result may also make result-producing computation unreachable to the
optimizer; the delta is not an isolated cost for the Candid library or every
transparent newtype. No instruction, wire, lifecycle or authorization parity is
claimed. Named optimized-body evidence and safe role/provider cuts are needed
before choosing a production change. Source duplication alone does not justify
removing required serialization.

Row 10 is now measured. Row 12's independent matched measurement has started;
row 13's matched run is queued after it. Provider qualification, the prepared
generic cohort, remaining switches, named mapping, allowances and predecessor
comparison remain open. B1 completion and B2/B3 still require human acceptance.
No production runtime change, broad validation, Git publication or deployment
is part of this experiment.
