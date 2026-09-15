# B1 recovery-dispatch measurement

Date: 2026-09-14. Experiment: `b1-06-unconditional-recovery-dispatch`.
Verdict: pass for controlled footprint attribution. All twelve artifacts pass
both baseline and variant determinism; no production deletion is recommended.

## Feedback refresh

The read-only Toko Miner ledger refresh still ends at CANIC-171. Its SHA-256
remains `d8eaf8c6dfae43757ff84095047b297300a0b705605b7909967163ce2e7026b4`,
identical to the [recovery follow-up review](toko-recovery-followups.md).
No new issue or changed acceptance request was found at that initial refresh.
The post-run refresh advances to CANIC-175 with ledger SHA-256
`05a202c11f25e8863556d68c313372a822985f71049f0003cd2adbcbdf3b1433`.
Those new recovery findings require separate source review; this experiment
does not qualify them or the existing .17 recovery changes.

## Scope and source

The product is frozen `v0.110.5`, commit
`50f40171d6177c3d1e490b1fdb5f6163323b2cd5`, tree
`5a66988735c707b188d9d1fe03a3ed3b4ff7a273`. The patch SHA-256 is
`37bcd14362830bf8cf6fd154309660c5dde951681a0c8cd318c2d1b14f58cf50`.
The complete selector is eleven canonical roles plus `runtime_probe`.

The product lock before and after is
`0fd6c7897d08e6a0f4e436caaf319ba24ebc32236434010b5c6ae3507f663147`.
Rust/Cargo are 1.97.1, ic-wasm 0.11.1, didc 0.5.4 and WABT 1.0.34. The
[run metadata](b1-row6-measurement/run-metadata.tsv) binds exact method,
counter, harness-lock, tool and patch identities. The
[artifact table](b1-row6-measurement/artifact-metrics.tsv) contains all raw,
gzip, code/data, function, interface, optimizer and hash vectors; the
[determinism table](b1-row6-measurement/determinism.tsv) records all repeats.

The audit patch disconnects watchdog takeover dispatch for auth renewal,
placement acknowledgement, automatic top-up and Root pool maintenance. Shared
timer identities, registration, custody and ordinary pool maintenance remain.
This deliberately changes recovery behavior in the isolated product worktree;
it is an attribution experiment, not a proposed production deletion.

The governed runner builds each baseline and variant twice, recreating one
fixed private Cargo target before each repetition. It checks Wasm/gzip/Candid
bytes, complete structured metrics and independent defined-function counts.
No named attribution, runtime instruction measurement or lifecycle parity is
supplied by this build experiment.

## Measured result

Deltas are candidate minus this run's matched baseline, after optimization.

| Artifact | Raw Wasm bytes | Gzip bytes | Code bytes | Code change | Defined functions |
| --- | ---: | ---: | ---: | ---: | ---: |
| App | -243,564 | -95,742 | -235,874 | -8.8985% | -359 |
| Index Hub | -107,914 | -44,244 | -103,840 | -4.2477% | -210 |
| Test | -118,073 | -49,086 | -112,296 | -3.8134% | -231 |
| User Hub | -115,256 | -48,478 | -109,532 | -3.5652% | -227 |
| Scale Hub | -116,082 | -48,632 | -110,387 | -3.6832% | -225 |
| Index Child | -113,553 | -46,758 | -109,400 | -4.9177% | -221 |
| User Shard | -110,189 | -45,020 | -104,639 | -3.4863% | -218 |
| Scale Replica | -243,587 | -96,060 | -235,889 | -8.8666% | -359 |
| Root | -14,749 | -7,077 | -14,307 | -0.2148% | -39 |
| Fleet Coordinator | 0 | +1 | 0 | 0.0000% | 0 |
| Wasm Store | -110,916 | -44,986 | -106,820 | -4.7378% | -208 |
| Runtime probe | -118,792 | -50,184 | -113,041 | -5.1412% | -234 |

Across the eleven separately deployed canonical artifacts, the switch removes
1,293,883 raw Wasm bytes, 526,082 gzip bytes, 1,242,984 code bytes and 2,297
defined functions. The runtime probe is separate. These are attribution sums,
not one deployable module's headroom or additive savings with other ablations.
Coordinator's unchanged raw/code sizes do not mean byte-identical output;
its gzip is one byte larger. Every role preserves its Candid hash, Wasm export
count and exported IC method count.

The result confirms substantial role-dependent reachability rooted by the
combined recovery dispatch. It does not split required recovery from
role-inapplicable paths or attribute each helper independently. Keep this
measurement; do not apply its destructive switch to production. A future
role-selected implementation must prove actual recovery and timer-custody
behavior before claiming a safe saving.

## Toolchain correction

An initial invocation from the active .16 checkout exposed a provenance defect:
the artifact subprocess selected frozen Rust 1.97.1, while harness preparation,
counter compilation and final version reporting resolved from the invocation
checkout, which selects 1.98.1. That run was stopped before a comparison and
is excluded. Its incomplete output is under `.tmp/b1-row6-measurement-20260914/`.

The runner now changes to the verified frozen product worktree before any of
those operations. A build-free regression launches from the current checkout,
intercepts the first Cargo metadata call and checks its actual working directory
and clean product state. Existing runner tests and scoped ShellCheck pass.
The listing test also checks nonempty, unique experiment identities instead of
requiring an incidental aggregate count.

The corrected run is under
`.tmp/b1-row6-measurement-toolchain-20260914/`; its summary log is
`.tmp/b1-row6-measurement-toolchain-20260914.log`. Product packages, dependency
lock and audit switch are unchanged. Earlier retained B1 reports record 1.97.1;
their historical evidence is not rewritten.

The two baseline repetitions pass exact Wasm/gzip/Candid and complete metric
determinism for all twelve selected artifacts. A separate sanity comparison
with row 5's historical baseline finds Root differs by one code byte and 48
data bytes, with unchanged function count and Candid hash; the other eleven
artifacts match their historical code/function quantities. The cross-run
difference is not attributed to a cause or used as a measurement delta. Root's
two builds in this run are byte-identical. All candidate deltas must use this
run's own baseline.

## Remaining boundary

All 48 builds pass artifact validation and all 24 condition/artifact pairs pass
exact byte and metric determinism. The runner restored the original clean
source and lock; its disposable worktree was removed. Local binaries, method
snapshots and logs remain in the corrected run directory; repository evidence
contains their metrics and hashes, not the Wasm binaries.

Row 6 is measured. Rows 8, 10 and 12 still need matched measurements, and the
remaining B1 switches, generic mapping, allowances and predecessor evidence
remain open alongside recovery follow-ups. This run does not implement B2/B3,
change production recovery, mutate Toko Miner, or supply deployment timing.
Package versions remain .16 and the existing .17 changelog covers the tooling
correction. No broad validation, version bump, commit, push or deployment ran.
