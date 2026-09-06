# Auth feedback: controlled Toko Miner Root measurement

The auth-free Toko Miner Root loses 333,288 code-section bytes (5.15%) when
Root delegation becomes capability-owned. Its ordinary Root control-plane
surface remains present. The checked delegation commands and state families
are absent from the pruned build.

| Canonical artifact measure | Delegation retained | Capability-owned | Reduction |
| --- | ---: | ---: | ---: |
| Code section, bytes | 6,466,908 | 6,133,620 | 333,288 (5.15%) |
| Raw Wasm, bytes | 6,900,605 | 6,549,390 | 351,215 (5.09%) |
| Gzip, bytes | 2,444,329 | 2,316,213 | 128,116 (5.24%) |
| Defined functions | 9,223 | 8,813 | 410 (4.45%) |

## Controlled inputs

Both builds use the same isolated Canic source snapshot, copied Toko Root Rust
source, topology, auth configuration, dependency lockfile, local build network
and `release` profile. The copied Rust source matches the read-only downstream
byte-for-byte. The configuration changes only the Root package locator to the
isolated package; the harness package name and build-script path are local
measurement details. Neither build edits Toko Miner.

The retained-delegation counterfactual differs in exactly two runtime inputs:
Root capability resolution always selects Root delegation, and the
`root-control-plane` feature selects its stable state. Restoring the candidate
versions of those two files produces the capability-owned build. Their before
and after hashes, retained counterfactual source, and shared input hashes are
in [the evidence directory](artifacts/feedback-auth/). This comparison measures
the marginal capability change within the working candidate; it is not a
comparison against an immutable published release.

Tools: Rust 1.97.1, Binaryen/wasm-opt 132 and ic-wasm 0.11.1. The release
profile retains its LTO and single codegen unit. Both runs explicitly preserve
linked names with `CARGO_PROFILE_RELEASE_STRIP=none` before Canic's normal
canonical artifact finalization. Each gzip was independently decompressed and
compared with its exact canonical Wasm.

## Absence and limits

The retained-delegation optimized companion has 16 matching Root delegation
state symbols. The pruned companion and linked Wasm have none. The checked
families cover the Root delegation state operations, records, data and TLS
owner. The pruned Candid omits `GetOrCreateDelegationProof`,
`UpsertIssuerPolicy`, `UpsertIssuerRenewalTemplate` and the checked issuer
renewal status surface. The exact before/after Candid and structured metrics
are retained beside the input hashes.

The named optimized companions do **not** have byte-identical executable
sections to the canonical Wasms; their metrics explicitly record
`named_executable_sections_match: false`. Symbol evidence belongs to those
companions and the linked artifacts. Canonical sizes, function counts and
hashes are reported independently. This does not constitute complete B1
acceptance or finalized Fleet release certification.

Separate focused runtime evidence covers auth-free Root activation, selected
issuer restoration/corruption and an auth-enabled nineteen-Workload/five-Ready
activation with terminal replay. The latter passes in 49.69 seconds and is
retained as [the activation log](artifacts/feedback-activation/nineteen-workload-activation.log).
The generated production-host nineteen-Workload/five-Ready proof now passes
in 990.33s with terminal conservation and effect-free replay. The complete
correction batch remains blocked on the predecessor transition described in
[the readiness report](fleet-feedback-readiness.md).

## Selected issuer capability fixtures

Earlier focused capability fixtures independently retain the expected state
selection. Their Wasm, gzip and Candid hashes were rechecked against the
recorded metrics. State-symbol counts refer to the linked Wasm and named
optimized companion; each companion reports non-identical executable sections
relative to its canonical artifact. These are targeted capability checkpoints,
not a complete current-source B2/B3 matrix.

| Fixture | Issuer state, linked/optimized | Root delegation, linked/optimized | Local authorization, linked/optimized | Canonical Wasm SHA-256 |
| --- | --- | --- | --- | --- |
| verifier-only | 0/0 | 0/0 | 0/0 | `d3a42f1741254760c0a5a589c26ee1773abb16435616aa76ad180b3bb6174f47` |
| zero-auth | 0/0 | 0/0 | 0/0 | `956006f4c5680318f8c196cf0386c0a6cce7d1aeff38abf4633784bcb070719e` |
| local-only | 0/0 | 0/0 | 8/7 | `9dcd2ee95d8f52bb73cf22b90e58d81b350bb5bf460bb03a5616797c87afd233` |
| issuer-positive | 4/3 | 0/0 | 8/7 | `4a3777212135290e0620449b3e7f7405f9c640e9216ef6224b6a54b91365e300` |

The issuer-positive fixture also selects local application authorization.
The auth-free Root checkpoint that exposed the unselected credential access
is superseded by the corrected activation and controlled Toko Root evidence
above; its old metrics must not be used to claim current Root absence.

During the completion audit on 2026-09-06, all four gzip payloads and Candid
files were rechecked against their recorded hashes and byte sizes. Their
metrics and Wasm feature flags are now retained in the evidence
directory alongside the Toko comparison. The copied `canic-core`, `canic`,
`canic-control-plane` and `canic-macros` Rust source trees matched the workspace at that checkpoint in both paths and bytes. The two controlled-change after hashes and
all recorded measurement inputs also matched. The lockfile difference was confined
to the workspace-only custody/testing integration and measurement fixture;
dependency versions and sources are unchanged. This corroborates the focused
capability evidence without relabelling it as a new build or broad B2/B3 gate.

The subsequent Fleet retirement changes are outside that source comparison;
these measurements remain checkpoint evidence.
