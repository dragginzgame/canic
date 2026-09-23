# B1 provider measurements

Reviewed: 2026-09-23. Metrics, configuration/provisioning, command, timer and
status experiments complete their retained measurements against immutable `v0.110.5`.
These are deliberately incomplete
audit canisters; none of the results authorizes a production deletion.

## Verification

Each retained provider experiment independently builds all eleven canonical
artifacts twice for both baseline and variant: 44 artifact vectors per
experiment; the timer experiment additionally includes the runtime fixture for
48 vectors. Verification checks complete selector/condition/repetition coverage,
every payload hash and length, gzip roundtrips, exact repeated metric vectors,
all determinism records, captured method sources and harness lock. Direct Wasm
parsing checks code/data section sizes and defined-function counts against the
recorded metrics. Every artifact preserves exact Candid bytes and export
name/kind identities across conditions. The original product checkouts restore
cleanly. No completed measurement was rebuilt for this review.

Product commit is `50f40171d6177c3d1e490b1fdb5f6163323b2cd5`; product lock is
`0fd6c7897d08e6a0f4e436caaf319ba24ebc32236434010b5c6ae3507f663147`.
Each evidence bundle retains its own source, patch, tool, method and lock
identities. Rust/Cargo are 1.97.1, ic-wasm 0.11.1 and Binaryen 132 `-Oz`.

| Experiment | Complete vectors | Independent verification |
| --- | --- | --- |
| 12: metrics | [metrics](b1-row12-measurement/artifact-metrics.tsv) | [verification](b1-row12-measurement/verification.json) |
| 13: configuration/provisioning | [metrics](b1-row13-measurement/artifact-metrics.tsv) | [verification](b1-row13-measurement/verification.json) |
| 14: command | [metrics](b1-row14-measurement/artifact-metrics.tsv) | [verification](b1-row14-measurement/verification.json) |
| 15: timer callbacks | [metrics](b1-row15-measurement/artifact-metrics.tsv) | [verification](b1-row15-measurement/verification.json) |
| 16: status | [metrics](b1-row16-measurement/artifact-metrics.tsv) | [verification](b1-row16-measurement/verification.json) |

## Matched results

Deltas below are variant minus each experiment's own baseline, summed across
eleven separately deployed canonical artifacts. They are not one module's
headroom. The experiments overlap and their totals must not be added.

| Removed provider dispatch | Code bytes | Raw Wasm bytes | Gzip bytes | Defined functions |
| --- | ---: | ---: | ---: | ---: |
| Metrics | -280,558 | -319,961 | -95,716 | -499 |
| Configuration/provisioning | -364,092 | -371,368 | -147,015 | -441 |
| Command | -2,827,931 | -2,923,760 | -1,133,111 | -4,703 |
| Timer callbacks | -3,553,322 | -3,720,741 | -1,353,127 | -6,676 |
| Status | -1,024,106 | -1,114,278 | -415,818 | -1,825 |

The configuration/provisioning result is entirely in Root; every other role's
code and defined-function delta is zero. Metrics changes neither Coordinator
nor Store code/function counts. Command attribution is largest in Root:
1,594,423 code bytes and 2,402 defined functions. Coordinator contributes
427,380 code bytes and 615 functions; Store contributes 237,049 and 487.
Status attribution is spread across roles, with Root accounting for 196,810
code bytes and 296 functions. Per-artifact absolute and marginal vectors,
including data and table entries, are retained in the linked evidence.

Timer attribution is largest in App and Scale Replica (639,577 and 639,583 code
bytes); Root contributes 113,992 bytes and 304 functions. The separate runtime
fixture contributes 152,015 code bytes and 335 functions. Coordinator has
unchanged code/raw lengths and function counts, but its bytes are not identical:
function, element, code and data section payloads differ, and gzip shrinks by
seven bytes. No
Coordinator executable-size saving is claimed. The full timer matrix preserves
exact Candid/export identities, including the runtime fixture.

The [provider boundary review](../2026-09-22/b1-provider-switches.md) identifies
exact dispatch roots and retained endpoint checks. The metrics experiment
removes read-side snapshot/projection providers while retaining recording.
Command attribution includes provisioning; status attribution includes metrics.
Other endpoint, persistence and recovery paths can keep the same provider
machinery reachable. The measurements therefore attribute dispatch reachability,
not complete libraries or an independently removable implementation.

Identical declarations and exports do not establish runtime, authorization,
provisioning or recovery parity. Instruction evidence remains absent. These
results prioritize the later role-specific provider analysis; safe production
cuts require named optimized-body evidence and maintained behavior qualification.
B1 remains open and does not grant B2/B3 acceptance.

## Timer qualification and retained measurement

Row 15 qualifies all eleven canonical artifacts plus the runtime fixture.
Independent [verification](b1-row15-qualification/verification.json) checks its
[complete vectors](b1-row15-qualification/artifact-metrics.tsv), payload and
method hashes, direct section/function metrics, and exact Candid/export
identities against the retained frozen baseline. This is one variant pass;
it is not a repeated measurement or a size-saving claim.

The catalog marks row 15 `ready`. Its matched measurement completes in
the restored provider checkout using a private target and its own frozen
method copy. Both clean repetitions match every metric and payload digest.
The retained bundle above supersedes qualification for footprint attribution;
the original run is `.tmp/b1-provider-measurement-20260923/row15/`.
The method includes the independently qualified cohort preparation support;
ordinary patch measurement still compares unchanged baseline against one patch.
Timer work overlaps the previously measured watchdog experiment and must stay
separate in interpretation.
