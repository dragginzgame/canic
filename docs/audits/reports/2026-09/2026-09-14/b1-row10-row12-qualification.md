# B1 endpoint-reply and metrics-provider qualification

Date: 2026-09-14. Verdict: rows 10 and 12 are ready for matched measurement;
B1 remains open.

Both complete selectors passed using their original patches: row 10 built
fourteen artifacts (eleven canonical roles plus runtime, payload-limit and
blob-storage probes); row 12 built all eleven canonical roles. Each artifact
passed Wasm validation, gzip round-trip, Candid validation and structured metric
checks, including the independent defined-function count.

The runs were sequential in one disposable linked worktree. Each runner exited
successfully, reversed its patch and verified the original product source and
lockfile. The active Canic checkout, pending IcyDB update and CANIC-139 correction
were outside the product build. No experiment changed production behavior.

## Exact identities and evidence

Product source for both runs is immutable `v0.110.5`, commit
`50f40171d6177c3d1e490b1fdb5f6163323b2cd5`, tree
`5a66988735c707b188d9d1fe03a3ed3b4ff7a273`. Product Cargo.lock SHA-256 is
`0fd6c7897d08e6a0f4e436caaf319ba24ebc32236434010b5c6ae3507f663147`
before and after both runs. Tools are Rust/Cargo 1.97.1, ic-wasm 0.11.1,
didc 0.5.4 and WABT 1.0.34. Exact executed method hashes and the independent
harness lock identity are retained in each run's metadata.

| Experiment | Patch SHA-256 | Artifacts | Evidence |
| --- | --- | ---: | --- |
| Row 10 endpoint-reply serialization | `d3f9497af1bf9e23db5cf45881bb6ca3c59ee2676900d1063d8389f165d2a813` | 14 | [Metadata](b1-row10-qualification/run-metadata.tsv), [metrics/hashes](b1-row10-qualification/artifact-metrics.tsv), [executed catalog](b1-row10-qualification/experiments.tsv), [roster](b1-row10-qualification/artifacts.tsv) |
| Row 12 metrics providers | `03fa9ecaf96e7edc2506a34859a65dafbe02b3f064322044bfa6919e508a0c9e` | 11 | [Metadata](b1-row12-qualification/run-metadata.tsv), [metrics/hashes](b1-row12-qualification/artifact-metrics.tsv), [executed catalog](b1-row12-qualification/experiments.tsv), [roster](b1-row12-qualification/artifacts.tsv) |

The copied catalogs truthfully record both experiments as `specified` during
execution. The maintained catalog changes them to `ready` only after the runs
pass. Local binaries, complete method snapshots and logs remain under
`.tmp/b1-10-12-qualification-20260914/`; summary logs are
`/tmp/canic-b1-10-qualification-20260914.log` and
`/tmp/canic-b1-12-qualification-20260914.log`.

## Limits and next work

Both runs are single-variant development qualification with
`retention_eligible=no` and no determinism claim. They provide no matched
before/after delta, production size saving, runtime instruction result or
behavior-parity evidence. Row 10 deliberately substitutes opaque endpoint
replies; row 12 deliberately disconnects read-side metrics providers. Neither
is a proposed production deletion. Named attribution is not produced here.

Ready rows 6, 8, 10 and 12 still need their governed matched measurements. The
remaining switches, generated-surface absence, generic cohort, allowances and
compatible predecessor evidence also remain open. These passes do not accept
B1 or authorize the wider B2/B3 state cuts.
