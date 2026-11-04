# Current handoff — 2026-11-04

## Authority and delivery boundary

The maintainer requested finishing timer cleanup, adopting the new upstream
Backup/Blob/Host/Platform/Testkit/Shared Tooling family, then deploying Canic.
Delivery authority is now present for the scoped work; earlier no-push requests
are superseded. No staging, commits, version changes, push or publication occurred.
Other sessions must hold dependency inputs while installed qualification finishes;
the maintainer confirmed that pause. The final six manifest/lock hashes stayed fixed.

The maintainer explicitly authorized the 0.114 → 0.115 boundary exception and
requested making 0.115 live. Prepare the accumulated source candidate and run the
governed full release gate, then publish Framework before Host/adapters. All 38
members currently remain 0.114.0; four 0.115.0 drafts cover the breaking changes.
Do not revive cancelled 0.114.1. Retain the Auth acceptance limits below in release
reporting; publication does not turn controlled-local evidence into production proof.
Current HEAD: 0509ec870. Recorded published 0.114.0:
`e753785953e6805461ac337868c32163f65c3209`. Preserve unrelated dirty/staged work,
`.git/release-state/`, `.tmp/release-0114-20261031/`, and the candidate named by
`.tmp/release-01141-20261102/candidate-path`. Check active target users before
input edits or compilation. Do not create an alternate checkout without approval.

## Current dependency family

Root and both private consumers now select Blob/runtime-contracts 0.71.1,
Platform 0.22.0, Types 0.18.1 and Wire 0.16.2. Root also selects Backup/Agent
0.58.0, Host Tools 0.82.0, Testkit 0.85.1, host Artifacts/FS/Process 0.74.3 and
Text 0.74.2. Timers remains 0.40.3, Metrics 0.29.1 with Metrics Storage 0.68.0,
Storage 0.66.2 and Logging 0.24.6. The older Blob Metrics 0.28 edge is removed.
Query retains its independent native Platform 0.19/Host Tools 0.79 boundary;
other suppliers still use Platform 0.21. No facade/type bridges were added.
Locked admission confirms one normal-Wasm Timers, Storage, Runtime 0.19.1 and
matching guard in each selected Blob graph. All compiler declarations and actual
Rust/Cargo selections pass the 1.99.1 admission. Edition remains 2024.
Shared Tooling 0.29.4 at `628ccc75d813bfd9bae777bfbaef55838d25b1b2` is already
upstream's latest committed revision; all 164 snapshot files verify. Its stale
local adoption note is corrected. Testkit 0.85.1 CLI/PocketIC setup passes.
The supplied Auth helper remains IC Auth-owned, dev-only, version 0.33.1 and
outside every Canic release group. Its unchanged archive extraction now lives at
`integrations/auth-backend-conformance`; Cargo excludes it from workspace members,
retains Core's dev edge, and the stable-storage conformance case passes.

## Timer cleanup and qualification

Timers owns callback execution. Canic retains durable demand, paid-operation
custody, lifecycle reconstruction, bounded retry authority and snapshot fences.
Three source/caller/guidance passes removed the remaining demonstrated duplication:

- Non-root init/upgrade bootstrap now uses one owned native continuation, returning
  retry decisions instead of allocating a new lifecycle registration per attempt.
  The existing 64-attempt window and capped delay remain; lifecycle stays synchronous.
- Store GC uses the existing typed-completion adapter and reports collection errors
  as failure. Root retains exact-operation retry authority. The adapter now has a
  concrete caller; no additional generic scheduler or application timer facade exists.
- Installed testing exposed a Coordinator directory-confirmation race: an unchanged
  provisioned Root receipt without publication evidence was classified as conflict.
  It now waits with typed availability, preserving the original intent. Changed
  receipts still conflict; issued authority is not rewritten.

Fresh passing evidence: bootstrap retry-bound regression; directory wait/changed-
receipt regression; two Store GC tests; 10 lifecycle and 16 timer ownership guards;
strict Core and Control Plane owning lint; 12 Backup checkpoint tests; installed
registration-budget recovery; installed bounded Store GC restoration/replay; both
private Blob composition proofs with upgrade/recovery; all 14 timer-authority cases.
No further scheduler duplication or raw-provider bypass was found in these passes.
The timer-owned batch is locally qualified; full shared-release readiness is not claimed.

Evidence: `timer_sediment_cleanup_20261104` in the
[adoption receipt](../audits/working/0.112-push-readiness/upstream-adoption.json),
logs in `target/timer-final-cleanup/`. Preserve the failed E132 run and its canister
logs under `.tmp/test-runtime.P3ape4/`. A subsequent run was stopped by concurrent
Blob 0.71.1 lock adoption before execution; the aligned rerun passed. Older receipts
retain their actual graphs, not an implied refresh from these later checks.

Earlier issue-specific evidence remains in the same receipt: #348/#165/#158 bounded
continuations/reconstruction/reviewed resume; #94 framework bootstrap before user
hooks; #46 atomic reservation scheduling; #183 funding custody; #123 bounded fixture
import; timer-specific #346 private-dispatch refusal; #730 watchdog ownership;
#150 batched Store cleanup; #89 initial-child commit receipt/startup; #398 paid
recovery; #176 snapshot suspend/resume; #545 status; #590 framework-owned adapter
composition; #375 guidance. Broader #346 endpoint inventory and actual downstream
database semantics remain outside the timer slice. Historical #590 proofs add no
IcyDB dependency or consumer release obligation to Canic.

## Contracts and schema/build features

Passive contracts use Types directly with empty defaults; `external-candid` owns
Nat conversions and coupled declarations. Native/Wasm codec-only checks pass;
passive normal/build closures exclude Platform, Candid, Agent and Runtime. All 326
Contracts tests with external-candid/wire-schema pass. Types 0.18.1 is published.
The CyclesConversionError re-export now shares its defining external feature gate.
Wire schema is selected through explicit Contracts/Core features, not the root catalog.
Default Core excludes direct proc-macro2/quote; native rendering needs build-support,
forwarded by Canic and selected by scaffolds, Fleet generation and Host checks.
Feature/lint/Host/scaffold evidence: `schema_build_features_20261104` and
`target/feature-boundaries/`. Wire 0.16.2 still includes its supplier derive edge;
Canic does not claim the planned upstream optional-derive cut is adopted.

## Separate retained work and release limitations

The [Auth progress table](../features/authentication/README.md#progress) owns Auth
acceptance. PRs #701/#703 are locally incorporated, not merged. `auth_*` receipts
retain actual graph-bound stable-storage, issuer, snapshot and browser proofs.
Prepared Root sealing, retirement fences and controlled-local capacity pass;
production clock/restore bounds, production key custody/activation, full UI and
complete combined-state cost remain open. Browser transport, Chromium IndexedDB
reopen/logout and issuer lost-response recovery have controlled-local evidence;
those are not production trust or complete Auth release qualification.

Private test consolidation and controlled-local issuer reclamation are qualified.
Storage/API cuts, four release groups, flat adapter layout and framework-first
publication remain as recorded in the group ledgers and adapter release guide.
Query publication/repair/retry authority remains separately accepted.
The 19-area removal inventory remains; dependency adoption closes no issue by itself.
Live Backup/Restore #513/#394/#550, Blob production/provider acceptance #622/#494,
Jobs #502 (unmerged #722), durable Relay #631/#636, metrics/pagination cost and
macOS/hosted qualification remain separate. ICRC #706 stays explicitly mounted.

Release hooks use exact source/recipe/Cargo-bound Shared plans and Canic's roster.
Framework/shared/uncertain changes require the full governed release gate; the fast
lane cannot qualify this runtime/dependency batch. No fresh full release gate or
hosted CI pass is claimed yet. The maintainer has authorized the combined 0.115
release; its governed validation and publication are the active work.
