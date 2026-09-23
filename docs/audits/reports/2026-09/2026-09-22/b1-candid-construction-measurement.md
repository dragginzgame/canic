# B1 endpoint declaration-construction measurement

Reviewed: 2026-09-22. Experiment: `b1-08-endpoint-candid-type-construction`.
Verdict: complete controlled footprint measurement; no executable code or
defined-function reduction. Do not apply this destructive audit switch to
production.

## Source and verification

The completed retained run was recovered from
`.tmp/b1-continuation-20260921/row08/`. Its final log reports success, and its
metadata records two clean repetitions for both baseline and variant across
all fourteen selected artifacts. The disposable product checkout was clean
before beginning the next experiment. No builds were repeated for this review.

Source is frozen `v0.110.5`, commit
`50f40171d6177c3d1e490b1fdb5f6163323b2cd5`, tree
`5a66988735c707b188d9d1fe03a3ed3b4ff7a273`. The product lock remains
`0fd6c7897d08e6a0f4e436caaf319ba24ebc32236434010b5c6ae3507f663147`.
Patch SHA-256 is
`e89c5a1488d7f87bc6bb1ea96913ea72307ed2769f9c59b47973fd12c91af39f`.
Rust/Cargo are 1.97.1, ic-wasm 0.11.1, didc 0.5.4 and WABT 1.0.34.
The [run metadata](b1-row8-measurement/run-metadata.tsv) binds the method,
harness lock, patch and replica function-counter identities.

Independent review verified the complete selected matrix, every retained
Wasm/gzip/Candid digest and length, decompressed gzip equality, complete paired
metric equality, all determinism records and recorded method-source digests.
The [artifact vectors](b1-row8-measurement/artifact-metrics.tsv) retain all
56 builds; [determinism](b1-row8-measurement/determinism.tsv) passes for each
condition/artifact pair. These counts follow this experiment's selector and
repetitions, not a fixed test-registry requirement.

## Matched result

Deltas are variant minus this run's own baseline, after optimization.
Every artifact has zero change in code-section bytes, data-section bytes and
replica-limited defined-function count. The executable code sections are also
byte-identical, verified directly from the retained Wasm files.

| Artifact | Raw Wasm bytes | Gzip bytes |
| --- | ---: | ---: |
| App | -20,317 | -5,298 |
| Index Hub | -20,099 | -4,698 |
| Test | -25,944 | -6,391 |
| User Hub | -22,561 | -5,816 |
| Scale Hub | -21,805 | -5,178 |
| Index Child | -18,692 | -4,864 |
| User Shard | -25,430 | -5,703 |
| Scale Replica | -20,900 | -5,372 |
| Root | -81,928 | -17,000 |
| Fleet Coordinator | 0 | 0 |
| Wasm Store | -11,020 | -2,952 |
| Runtime probe | -14,470 | -4,003 |
| Payload-limit probe | -9,199 | -2,763 |
| Blob-storage probe | -15,989 | -4,426 |

The eleven separately deployed canonical artifacts total 268,696 fewer raw
bytes and 63,272 fewer gzip bytes. These are cross-artifact attribution sums,
not one module's headroom or savings additive with another ablation.

The [section comparison](b1-row8-measurement/section-comparison.json) identifies
changed section payloads by Wasm section ID or custom-section name. Only the
`icp:public candid:service` custom section changes in the three fixtures.
Canonical roles other than Coordinator also change data-section payload bytes
(section `11`) without changing their length; the counterfactual changes their
profile metadata. Coordinator is byte-identical. Every artifact preserves its
Wasm export and exported-method counts. Candid hashes change for every artifact
except Coordinator, as the switch intends.

## Decision and remaining work

This result rules out this declaration-construction switch as a source of
optimized executable-code or function-headroom savings in the measured matrix.
Smaller declarations are not equivalent public interfaces. The switch retains
runtime endpoints and wire serialization but proves neither declaration nor
profile-metadata parity, and supplies no runtime instruction measurement.

Row 8 is now measured. The subsequent
[row-10 serialization measurement](b1-reply-serialization-measurement.md)
completes against the same frozen source; row 12 remains in progress. The
remaining B1 switches, generic mapping, optimized generated-surface
evidence, allowances and predecessor comparisons remain open. B2/B3 still
require human acceptance of complete B1. No production runtime change, broad
validation, Git publication or deployment is part of this measurement.
