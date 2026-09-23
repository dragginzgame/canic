# B1 provider counterfactuals

Date: 2026-09-22. Frozen product: `v0.110.5`, commit
`50f40171d6177c3d1e490b1fdb5f6163323b2cd5`.

As of 2026-09-23, all provider switches (rows 13 through 16) are `ready` after
complete qualification. Rows 12, 13, 14 and 16 also have independent retained
[matched measurements](../2026-09-23/b1-provider-measurements.md). Row 15's
matched run is active. The qualification history below records the earlier
single-pass evidence; it does not substitute for retained repetitions.
No production contraction or runtime-parity result is claimed here.

## Exact causal boundaries

| Row | Removed call roots | Retained boundaries |
| --- | --- | --- |
| 13 | Root's `ProvisionChild`, `ProvisionComponent`, `ProvisionComponents`, `ProvisionPeer` and `SynchronizeComponentDirectories` dispatch arms, plus its `Config` query provider | Root command/status admission, typed requests/responses, the other dispatch arms, configuration restoration and background provisioning/recovery owners |
| 14 | Managed-role command providers and the final Root, Coordinator and Store command dispatch | Explicit endpoint admission checks, payload-limit selection and guards, request decoding, response types, endpoint declarations, status providers, startup and timers |
| 15 | Native acknowledgement, auth renewal, cycle top-up, intent cleanup and log retention callbacks, core recovery watchdog, Root Pool maintenance and recovery watchdog | Timer identities, registration custody, declared cadence, lifecycle deferrals, application timers, endpoint providers and the shared timer implementation |
| 16 | Managed and unmanaged role status dispatch, plus Root, Coordinator and Store status dispatch | Explicit query admission checks, typed requests/responses and declarations, command providers, startup, recording and timer registration |

The configuration/provisioning switch targets the actual generated Root call
sites. It includes child, single-component and peer provisioning, rather than
only the separate group-provisioning facade. It leaves configuration restoration
and parsing intact. Provisioning code still reachable through recovery,
operation-status or other callers is deliberately retained. This is marginal
entrypoint attribution, not proof that an entire provisioning module disappears.

The command switch preserves the Root, Coordinator and Store checks before
provider dispatch. In managed role commands, checks are embedded in individual
match arms; those explicit caller checks remain in their original arms. Store
and Coordinator payload-limit helpers are unchanged. Store chunk publication
and retrieval endpoints remain separately reachable and are outside this switch.

The status switch preserves checks before the final request dispatch in all
five generated bodies. It includes the Metrics status arm and therefore
overlaps row 12. The command switch overlaps the provisioning arms in row 13.
Shared persistence, codecs and recovery machinery may also overlap other rows.
These deltas must never be added into a savings forecast.

Each endpoint replacement returns an opaque typed `REQUEST_INVALID` result using
`black_box` around the complete `Result`, while consuming the typed request
where applicable. This prevents the source from substituting a smaller reply
schema or deliberately specializing serialization to a known error variant.
It does not guarantee a particular optimizer outcome. The original endpoint
signatures and attributes remain, and qualification must check actual Candid
and exports. Provider-internal checks and behavior disappear with their provider;
preserved endpoint checks do not establish authorization or lifecycle parity.

These are deliberately nonfunctional audit canisters. None of the patches is
a production recommendation, a Cargo feature or a runtime option.

## Timer callback boundary

Row 15 disconnects the eight native business callback registrations in seven
frozen-source owners. Its exact patch digest is
`281510984c274898798e02f450f96744c4347a14e561aaf027f0e5aa67a9077e`.
One-shot callbacks report no work and stop; Root maintenance reports no work
and retains its after-completion recurrence; both watchdogs report no work
and continue. These replacements preserve registration shape, not useful timer
behavior, recovery or lifecycle parity.

The shared lifecycle registration helper, lifecycle/start macros, application
hooks and application-owned timers are unchanged. Source business functions
remain type-checked through discarded function-item references. The core
watchdog discards its recovery function argument before constructing the
replacement closure, so the closure does not capture that function pointer.
The discarded references are not calls; actual optimized absence still needs
artifact evidence. Commands, queries and other callers may keep the same
business code reachable.

Row 6 disconnects watchdog takeover while retaining ordinary native jobs;
row 15 includes those jobs and the watchdog callbacks. The rows overlap and
their measurements cannot be added. Row 15 is marginal callback attribution,
not a measurement of removing the shared timer runtime. Its selector includes
all canonical artifacts and the runtime fixture, which retains lifecycle and
application timer surfaces.

## Qualification boundary

Manifest preflight checks each exact patch digest and applicability against the
frozen Git tree. The existing ablation-runner regression passes, including
rejection of incorrect digests and patches for another source tree. Frozen
Rust 1.97.1's formatter parses the edited sources; this is syntax evidence,
not a build or optimized-artifact result.

Root-only row-13 qualification passes in 419 seconds in the separate disposable
checkout `.tmp/b1-provider-qualification-20260922/product`, with private Cargo
scratch. Its [artifact vector](b1-row13-root-qualification/artifact-metrics.tsv)
and [metadata](b1-row13-root-qualification/run-metadata.tsv) retain the exact
source, switch and tool identities. The 6,725,803-byte module passes Wasm,
Candid, gzip and structured-metric validation. Independent review verifies every
payload digest/length and gzip contents, plus
[exact Candid and export identities](b1-row13-root-qualification/interface-comparison.json)
against row 8's retained frozen baseline. Candid is 88,042 bytes; the typed
schema is not replaced by the audit error result. The runner restores the
original clean source and lock. Log:
`.tmp/b1-provider-qualification-20260922/row13-root.log`.

The subsequent complete row-13 qualification succeeds across all eleven
canonical artifacts. Its [artifact vector](b1-row13-qualification/artifact-metrics.tsv),
[metadata](b1-row13-qualification/run-metadata.tsv) and
[independent review](b1-row13-qualification/interface-comparison.json) retain
complete selector coverage, verified payload hashes/lengths and gzip contents,
and exact Candid/export identities against the retained frozen baseline.
Captured method hashes and harness lock also verify. The runner restores the
checkout before the successful exit record permits row 14 to start.

Row 13 is promoted to `ready` on that complete qualification. This is still a
single variant result, not a matched size reduction or a determinism claim.
Its subsequent matched measurement is now retained in the report above;
qualification was not reused as a retained repetition.
Row 14's complete eleven-role qualification also succeeds. Its
[artifact vector](b1-row14-qualification/artifact-metrics.tsv),
[metadata](b1-row14-qualification/run-metadata.tsv) and
[independent review](b1-row14-qualification/interface-comparison.json) verify
all payloads, gzip contents, method identities and exact Candid/export identities
against the retained frozen baseline. Row 14 is promoted to `ready`; its matched
measurement subsequently completes with a separate frozen method copy and is
retained in the report above.

Row 16's complete eleven-role qualification succeeds. Its
[artifact vector](b1-row16-qualification/artifact-metrics.tsv),
[metadata](b1-row16-qualification/run-metadata.tsv) and
[independent review](b1-row16-qualification/interface-comparison.json) verify
every payload hash/length, gzip contents, captured method identities and exact
Candid/export identities against the frozen baseline. Independent Wasm parsing
also agrees with the recorded code/data section sizes and defined-function
counts. The experiment is promoted to `ready`, without a paired-size or
determinism claim. Its subsequent retained measurement completes and is
independently verified in the report above.

Row 15's twelve-artifact qualification also completes. Independent
[verification](../2026-09-23/b1-row15-qualification/verification.json) confirms
payloads, methods, section/function metrics and exact baseline interfaces.
The catalog is now `ready`; its matched run uses a frozen method copy and
private target in the restored provider checkout. The current handoff owns
its run paths. No matched timer delta is claimed before that run completes.

Review each endpoint experiment's complete eleven-artifact selector and row 15's
canonical-plus-runtime selector, and verify interface identities. Promote a row to
`ready` only after its complete qualification succeeds. Retained runs still
require two clean baseline and variant repetitions with complete determinism.
Neither qualification nor these switches closes B1 or opens B2/B3.
