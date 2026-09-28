# Toko feedback follow-through

Date: 2026-09-28. Published base: 0.110.44. Current batch: 0.110.45.

The maintainer requests the release-test correction and remaining Canic-owned
feedback work together before pushing. Toko's feedback and source were read
without changing its checkout, deployment records or live estate.

## Release failure and current reset proof

The earlier release failure is in the completed-estate CLI fixture. It invokes
`fleet ensure --reinstall` without a current desired document and expects the
superseded completed-preparation route. The current route correctly selects a
fresh reset; the fixture fails before reviewing it. The failure is not an IC
query trace or the deliberately invalid Candid used by an unrelated native test.

The fixture now generates current authority, then invokes the public CLI for
infrastructure review/apply, Root-owned clearing, workload review/apply and replay.
The operator reserve matches the existing repeated-reset fixture, so the proof
can reach its injected lost install reply. Payment assertions remain exact.
Infrastructure publication retains the actual execution report instead of
constructing a zero-effect report after successful work. Phase diagnostic events
carry current authority and separate phase completion from Fleet completion.

The exact PocketIC case passes in **196.80 seconds**; the complete targeted runner
takes **286 seconds**, including compilation and artifact setup. It proves:

- a lost real install response and same-digest CLI continuation;
- child code and stable-memory clearing through the new Root;
- exact retained canister identities and operator Ledger debit;
- byte-identical historical source documents in the archive;
- separate bounded diagnostic receipts for review, failure and continuation;
- plain and JSON terminal replay without an available IC executable or new effects;
- offline replay of an earlier infrastructure phase after Fleet completion, with
  its original plan digest and no repeated effects;
- later reviewed capacity import into the completed current Fleet.

Log: `target/test-runs/20260928T080312Z-1917.XqdmIY/1.log`.
Earlier focused attempts exposed the insufficient fixture reserve; those failures
are retained and are not counted as passes. The existing changed-build/same-build
proof retains its separate evidence for interrupted final publication and repeated
resets. No new full workspace pass is claimed.

All **32** selected CLI progress/receipt tests and **six** query transport tests
pass, including interactive terminal restoration, unknown/repeated component
instances, real HTTP 502 retries and typed permanent-failure rejection. Scoped
host/CLI/internal-test warning-denied Clippy passes after correcting one eager
`Option::or` in the fixture. Logs: `.tmp/upstream-feedback45-native.log` and
`.tmp/upstream-feedback45-clippy-final.log`.

## Subsequent mixed-topology gate correction

The maintainer's next full run failed at the mixed-topology fixture's obsolete
completed-preparation CLI review, before the replacement reset was applied:
`target/test-runs/20260928T081507Z-53814.lgXOwc/7.log`.
The fixture now generates current authority and uses the shared clean-reset
workflow. Its large, superseded preparation sequence is removed. No production
admission or cycle-conservation check was relaxed.

The maintained assertions retain real Hub/Shard user-row deletion and authored
system-row restoration, exact infrastructure/application Wasm hashes, physical
IDs, five workload canisters, one Ready reserve and memory diagnostics. A full-reset
Ledger reserve avoids incidental fixture shortfalls. The shared proof also checks
that operator-balance drift causes rejection before execution records or paid
effects, then restores the fixture balance and loses a real funding reply. That
payment and the subsequent lost install reply recover under the same reviewed
phase; final exact debit, bounded burn and offline replay still pass.

Both affected callers pass:

| Proof | Test | Complete runner | Retained log |
| --- | ---: | ---: | --- |
| Mixed topology and retained Ready reserve | 346.87s | 366s | `target/test-runs/20260928T085923Z-19625.6DToIb/1.log` |
| Consecutive changed-build and same-build resets | 197.08s | 198s | `target/test-runs/20260928T090528Z-47971.wGD7Bm/1.log` |

Scoped warning-denied Clippy passes (3.19s) in `.tmp/mixed-reset45-clippy.log`.
Removing an unused wire response changed the decoder's variant-size distribution;
the existing lint expectation now applies to both tested configurations. An
intermediate run caught the old funding helper's incorrect full-Fleet digest at
the infrastructure phase; the helper now uses the current reset owner and asserts
the typed balance-drift rejection. Intermediate failures are not counted as passes.
No new complete workspace result is claimed. The complete accepted .45 batch
remains ready for maintainer review and the selected release-gate retry.

## Current feedback disposition

| Feedback | Canic implementation/evidence | Remaining acceptance |
| --- | --- | --- |
| CANIC-184 | Completed envelopes are selected before old executable contracts; current inventory/controller authority drives clean reset. Current public CLI proof passes. | Publish/adopt .45 and qualify Toko's selected deployment. |
| CANIC-183 | Shared bounded receipts cover the current reset. Infrastructure execution counts/conservation survive publication; phase events and plan/import digest attribution are explicit. | Released CLI and naturally required live Toko reset. |
| CANIC-150 | Reviewed component occurrences retain names, placement/member identity, Root, unknown/current state and bounded interactive rendering. Focused presentation and terminal-restoration tests pass. | Live named-component/interactive acceptance. |
| CANIC-160 | Root status reads use bounded typed query retries; actual HTTP 502 and permanent failures are covered. Reset receipts restore the missing measurement boundary. | Matched live deployment/replay measurements; no live speedup is claimed. |
| CANIC-176 | Build-cache diagnostics are already adopted. Six frozen-input Release measurements pass; each unchanged pair reuses byte-identical output in about 2.2s. | Runtime compilation after changed inputs still takes minutes; broader compiler/link optimization and released-CLI acceptance remain open. |

The operator guide previously retained a contradictory procedure requiring old
completed plans and source contracts to decode. That procedure is removed. The
maintained unreadable-plan and deliberate-wipe sections now select completion
first, use current inventory/build authority for completed estates and retain
reconciliation ownership only for unfinished issued effects. Existing guide
anchors remain available to diagnostics and links.

## Older feedback is not automatically new implementation work

- **CANIC-156/174:** the recursive funding/quote RF3 implementation is recorded
  complete in the .34 batch and reaffirmed after .36 in the active design tracker.
  Older downstream entries retain acceptance wording; they do not establish that
  this implementation is still missing.
- **CANIC-161:** the cycle-exhaustion incident remains unattributed. The original
  configuration lacked opt-in replenishment; the current copied App configuration
  has exact-role top-up policies. Supported diagnostics/manual recovery were
  documented earlier. The historical installed policy and long-running live
  acceptance are not proven by today's source, and no new framework loop defect
  is established.
- **CANIC-170:** explicit identity selection/binding was delivered. The original
  encrypted-identity incident remains unconfirmed; do not claim it reproduced.
- **CANIC-179:** current completion-first recovery guidance is corrected here.
  An unreadable old executable payload does not block a demonstrably completed
  estate; unresolved paid work still requires its actual owner.
- **CANIC-180:** the delivered terminal-replay correction and current offline
  reset replay are separate from downstream live acceptance.
- **CANIC-166:** the released .43 live hard cut remains verified. CANIC-184 tracks
  the subsequent completed-authority selection defect; do not reopen the already
  successful .43 deployment as unfinished work.

No live wipe should be repeated merely to gather diagnostic acceptance. Publication,
Toko adoption and deployment are separate effects from this in-repository work.


## Current application Release measurement — CANIC-176

The [structured results](toko-build-measurement.json) retain exact binary/input
and log-inventory hashes. The [executed shell harness](toko-build-measurement.sh)
was run from `.tmp/toko-feedback45-20260928/`, beside its private `app/`,
`framework/`, `tools/` and `target/` directories. It requires that prepared
snapshot; it is evidence of this run, not a standalone application setup tool.
Raw logs, input manifests, lockfiles and release file hashes remain under that
local directory's `results/`.

| Input cohort | Release build | Immediate unchanged repeat |
| --- | ---: | ---: |
| Empty private Cargo/artifact cache | 1,027.38s (17m07s) | 2.16s |
| Application Rust source changed | 456.48s (7m36s) | 2.22s |
| Application path dependency changed | 483.64s (8m04s) | 2.19s |

All six commands succeeded and each unchanged pair retained identical complete
release-file hashes. All eight Candid interfaces remained identical across the
three cohorts. The 5,184 selected input files were verified against their hashes
after the harness restored the two changed files.

Controls and limits:

- Public command: `canic --environment staging build toko_miner --profile release --verbose`.
  The executable and framework sources were frozen before measurement. Package
  versions identify .44, with unreleased .45 source changes; this is not a
  published binary qualification or a measurement of the later receipt-only fix.
- The read-only Toko snapshot identifies application .3.15 and contains local
  work. Its manifest requested IcyDB .261.12 while its retained lock selected
  .261.11. Only the private copy's lock was resolved; local Canic path patches
  select the frozen candidate. This is not an exact published Toko tag build.
- Cargo ran offline, with four jobs, incremental compilation disabled and no
  compiler wrapper. Registry sources and operating-system caches were already
  warm. The private target began empty. The host was shared; these are single
  ordered samples, not statistically comparable release benchmarks.
- Changes were non-semantic Rust comments in `game_shard/src/lib.rs` and
  `toko-miner-contracts/src/lib.rs`, with baseline restoration between cohorts.
  They measure changed-input invalidation, not the cost of a gameplay change.
- Cold runtime Cargo/link took 541.29s. After the application change it took
  331.51s, while declaration work took 26.53s and cached Candid extraction 0.08s.
  Nested and overlapping phase timings must not be summed as wall time.

Exact complete-build reuse works. Changed inputs still trigger substantial
runtime compilation under the embedded release identity. No unsafe or missing
warm-cache reuse was demonstrated; this work does not claim a compiler/link
speedup. Five compatible application groups retain their resolved feature
boundaries. Broader runtime build optimization remains a separate open performance
item; the older .39/.43 samples use different inputs and do not establish a
regression or improvement against these numbers.

## Batch handoff

The confirmed release regression, current receipt defect, operator guidance and
requested frozen-input measurements are complete. The accepted .45 implementation
batch and changelog are ready for maintainer review and the selected release gate.
The final targeted reset proof and scoped Clippy pass; no full workspace gate was
rerun. Package versions remain .44, with .45 unreleased changes left uncommitted.
This is not a claim that every upstream acceptance item is closed: live adoption,
matched deployment measurements and further runtime compilation optimization
remain as listed above. No Toko records, sibling source or live canisters changed.
