# Current handoff — 2026-11-02

## Baseline and authority

Published source baseline v0.114.0 is e753785953e6805461ac337868c32163f65c3209; its full
local gate qualified b94a133339178a1fc8cc638d8f4fe40a18dfc416. Preserve
.git/release-state/ and .tmp/release-0114-20261031/ for recovery, all dirty work,
staging and retained artifacts. The maintainer requested release/publication of
0.114.1. Own versions remain 0.114.0 until the governed complete release gate and
bump. Freeze the reviewed batch, integrate current origin/main, run
make release-patch, then make publish. Recovery evidence is retained under
.tmp/release-01141-20261102/. No application/Fleet deployment or minor closeout
is selected; never compete for an active target/. Remote native-export/metrics
fixes through 2c2272b5e are integrated. Two complete attempts stopped in Clippy
before versioning: retired storage init() calls, a lifecycle lint, then an
unchecked-key test lookup. The four App callers now explicitly create fresh
stores or strictly load retained stores. All three fixture packages and every
canic-tests target pass scoped Clippy; the exact allocation-lookup byte-preservation
regression passes. The third attempt passed workspace Clippy and private consumers,
then stopped at the older crypto-closure profile. Platform now owns certification/
BLS internally and ECDSA uses newer digest/field APIs. The corrected guard admits
only reviewed role-specific cohorts, rejecting duplicate identities within a
cohort, unknown cohorts and extra signature providers. Negative fixtures, all 12
real role graphs and 132 focused Auth cases pass. Crypto source paths and logs are
retained in the release evidence directory. Commit the corrections and retry the
normal fresh release; no version bump or recoverable 0.114.1 plan exists yet.

## Active upstream adoption

All three maintained locks now select Blob/contracts 0.66, Platform 0.17,
Memory/Storage 0.62.1, Metrics 0.25.1, Metrics Stable 0.64, Timers 0.36,
Wire 0.14.2 and Auth/protocol 0.30. Root also selects Query 0.84, Host Tools 0.77,
Testkit 0.78 and Backup/Agent 0.53.6. Build/Host helpers remain 0.73.6 and Logging
is 0.23.7; private consumers select the relevant same releases. The exact metadata
editor remains pinned at 0.73.3. Retain concurrent CRC 1.5.4/Thiserror 2.0.21 lock
updates; their exact selected sources were fetched before final qualification.

All supplying releases are published and the three graphs resolve. Query 0.84
clears the Host certificate-tree/ledger-tip type mismatch; authentication and
trust-root validation stay upstream. Auth 0.30 aligns its canonical identities.
No source override, runtime bridge or weakened verifier is selected.

Exact registry observations, update/fetch logs and original catalogs/locks live in
target/upstream-adoption/20261102/. Dependency admission passes all three normal
Wasm graphs with one Memory, Storage, Platform runtime and guard owner. Affected
Core, Control Plane, Host, Blob adapter and internal qualification all-target,
all-feature Clippy passes. The sole compile correction passes the held-reply
fixture mutably to supersession qualification, as required by Testkit release().

The earlier adoption passed 106 focused native cases: 13 Auth,
53 memory/metrics/storage/allocation, 30 certified receipts and 10 Query catalog
cases. Logs are target/upstream-adoption/20261102/native-*.log; their exact locks
precede the later cc 1.11 lock selection and current release corrections. Older candidate
proof in target/upstream-adoption/current/ retains its exact earlier scope.

The refreshed source-bound allocation peer verifies and both private consumers
pass strict Clippy. Current-family Blob qualification passes: dedicated managed
installation/recovery (459s), all five embedded funding profiles (577s), and both
generated-client shapes (133s), using Node 24.21.0, TypeScript 6.0.3 and Testkit
0.78. The initial dedicated run exposed an undersized held-reply method-name
budget; deriving its aggregate bytes from the four-entry catalog fixes admission
and the focused Clippy/rerun pass. All three lock hashes remain unchanged.
Evidence: target/upstream-adoption/20261102/{dedicated-fixed,race-clippy,
fixtures-clippy-peer}.log and target/integrations/checks/blob66-20261102/.

The accepted dependency/Blob adoption retains its direct local proofs. The
maintainer-selected complete release gate now owns whole-batch qualification.
The open 0.114.1 notes are current; own versions stay 0.114.0 until that gate
passes and the governed transaction bumps them. No successful complete gate,
hosted CI or registry publication is newly claimed.

## Blob and provider boundary

The adapter opens checked StableKey grants through the single committed host
runtime. Nonzero buckets, explicit creation on empty allocations and strict
retained loading preserve lazy stores and reject malformed contents. Deliberate
chunk clearing releases its lease before reopening/resetting that exact key.
Installed gateway tests cover supersession, exact cancellation and reversed held
replies in both shapes. Generated tenant query/update codecs preserve caller and
service binding, reject malformed frames and replay without another mutation.
This is installed PocketIC evidence, not a deployed browser/provider journey.
Earlier 53 native cases, scoped Clippy and Rust 1.91 Wasm proof select only
Memory/Storage 0.61 and Metrics Stable 0.62: target/integrations/checks/blob65/.
Those older results retain their narrower graph; the current-family installed
evidence above supersedes the pending Blob qualification gap. Delivery and the
remaining provider/funding acceptance stay tracked in
[#622](https://github.com/dragginzgame/canic/issues/622) and
[#494](https://github.com/dragginzgame/canic/issues/494); see the
[acceptance audit](../audits/working/blob-0411-adoption.md).

Caffeine is the fixed provider; actual network/application/service binding is
unrecorded. Blob #5/#31/#6/#34 require deployed publication/login, serving,
deletion/billing and independently attributable payer credit. Acknowledged
payment is not independent credit. No deployment, paid effect or closure occurred.

## Retained qualification and safety boundaries

Timers owns scheduling/callbacks; Canic keeps domain demand, durable attempts,
exact snapshot custody and lifecycle deferral. Lease expiry never replaces live
native work. Root observes history, rechecks quiescence and suspends producers
with the seal; resume reconstructs before mutation, with effect-free final replay.
.tmp/timers-native-cut-20261101-vXo4qw/evidence/ keeps 0.35.3/0.35.4 native and
installed proof; .tmp/timers-completion-20261101/evidence/ keeps idle red/green.
Actual database startup remains downstream-owned (#590 and IcyDB #373).

Platform owns execution/callbacks; Canic's stateless adapter projects errors and
metrics. Refusal/decode failure/counters never authorize replacement paid work;
retain exact domain journals and observe matching refunds before another await.
.tmp/platform-call-20261101-UXFJTl/evidence/ keeps native/installed proof (#700,
#368). The optional mounted ICRC adapter retains application consent policy;
.tmp/icrc-qualification-20261101-erYsxR/evidence/ keeps its native/installed
proof (#706). Earlier Auth proof is
.tmp/auth-platform-adoption-20261101-7EfsMQ/evidence/. All retain exact old inputs,
not qualification of this latest adoption; temporary source copies were cleared.

## Tooling, outputs and delivery

Shared Tooling is reviewed 0.27.26, 1ac8c6d878598c3d32cbd91c234cabea4e416961.
Its 152 payloads are unchanged from the preceding reviewed snapshot and verify
against that exact committed source. The prior governance-link and pinned-editor
transaction proofs retain their scope. Do not export the dirty sibling checkout.
Evidence lives in target/upstream-adoption/; prior compacted handoff is retained
in current/handoff-before.md there. Successful admitted tool compiler scratch is
discarded; no selected tool, release artifact or another user's cache is swept.

Private source/locks stay under integrations/. The canonical preparation helper
synchronizes target/integrations/workspace/ and holds custody through its child;
native caches use target/integrations/native/. Root clean covers generated
outputs. Old caches were removed (~147 GiB); existing outputs were retained under
target/integrations/previous-outputs/ (~7.9 GiB). Governed PocketIC interruption
scratch remains the documented outside-target recovery exception.

Delivered CI 41243808680 still fails PocketIC and both macOS jobs. Local bounded
download, physical-path and server-lifetime repairs retain focused evidence in
.tmp/ci-repairs-20261101/evidence/; they do not establish hosted/macOS acceptance
(#450/#442). No broad gate, CI rerun, version mutation or publication is claimed.
