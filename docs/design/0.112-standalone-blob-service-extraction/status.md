# Upstream Integration Implementation Status

Current execution follows the [consolidated push-readiness checklist](../../audits/working/0.112-push-readiness/plan.md). Dated sections below retain historical evidence; they do not override that checklist or establish current qualification.

## Selected release scope — 2026-10-26

The [current delivery decision](0.112-design.md#accepted-release-scope--blob-and-wire--2026-10-26)
limits remaining push-readiness work to Blob storage and Wire. Production Auth and
live Backup completion are deferred; their unavailable paths and activation gates
stay closed. Historical Auth/Backup acceptance obligations below are future work,
not blockers for this selected release. Existing pending changes are preserved.

## Backup convergence implementation boundary — 2026-10-24

Current-source inspection confirms published Backup 0.40.0 coordinator/attempt
APIs are available. Canic's active restore runner still owns scheduling and
mutable apply journals. Its ICP configuration does not supply the upstream exact
network/caller/release/request authority. Root checkpoint storage, authenticated
bounded Wire dispatch and the shared catalogue mutation guard are now connected. Installed Root/two-participant
qualification and paid snapshot composition remain pending.
See the [implementation contract](0.112-design.md#backup-execution-convergence--2026-10-24).

| Batch | Outcome and owner | Required qualification | Current state |
| --- | --- | --- | --- |
| B5 | Production Root checkpoint ownership: existing model, control-plane storage, authenticated commands, complete participant discovery and topology exclusion | Native exact-unit/replay tests plus actual Root/two-participant interruption and release | Storage/commands/guard connected; installed Root qualification pending |
| B1 | Backup original host workflow composed with Root-paid effects; one owner per effect class | Exact identity, finite attempts, changed/missing original refusal, unknown-result recovery and cold replay | Custody foundations integrated; stage/attempt engine adoption pending |
| B5 | Complete capture and same-release restore with application acceptance and terminal release | Complete artifact bytes, stopped restore, outstanding external obligations, accepted start and effect-free terminal replay | Participant fixture evidence only; production path pending |

The dormant capture mutation cleanup is complete (248 Backup and 52 CLI backup
library tests, strict affected Clippy). It does not qualify these pending outcomes.
The previous Auth 0.6 failure below is historical evidence, not the current Backup
implementation blocker. Live creation remains unavailable; the complete 0.112
batch remains not push-ready. Current checkpoint evidence is retained under `/tmp/canic-root-checkpoint/`;
no production snapshot effects or deployments were run.

## Capture executor retirement — 2026-10-18

The unused injected Canic capture runner is removed in favor of the IC Backup
execution/recovery owner. This is a hard cut of an unconnected public executor,
not completion of live backup. Dry-run planning and retained evidence models remain,
as do active restore execution and its interruption proofs. Integration of Root
checkpoint authority with upstream capture is still tracked in #513 and #550.
The older package-test counts below describe their original validation snapshots.

## Live Backup work resumed — 2026-10-15

The maintainer requested getting Backup working, superseding the implementation
deferral below. The [resumed plan](0.112-design.md#live-backup-implementation-resumed--2026-10-15)
retains application-coordinated checkpoints and framework-owned acceptance.

The Root journal model now binds the complete selected participant catalogue to
one Root installation, operation, digest and monotonic sequence. Exact begin
replay retains progress; stale issuance and changed units refuse. Preparation
requires every acknowledgement, and release intent includes unanswered prepares.
Membership remains fenced until every release acknowledgement is retained.
Deserialization validates the retained unit and phase. Eighteen participant/Root
native cases and strict Core library/test lint pass on the selected graph.

This is a serialized model, not installed Root orchestration. Stable control-plane
storage, authoritative topology guards, authenticated command dispatch, paid
snapshot intent/reconciliation and host capture/restore wiring remain incomplete.
Live creation remains unavailable. Whether completion of this resumed workflow
is a 0.112 release requirement is being clarified separately from implementation.
Published Blob alignment is qualified. The current integration blocker is the
Auth 0.6 session-store contract described below.

## Accepted 0.112 scope — 2026-10-15

The maintainer selected **ship integrations; keep live backup unavailable**.
The [delivery boundary](0.112-design.md#accepted-delivery-boundary--2026-10-15)
supersedes the earlier live-readiness requirements and B5 release obligation.
B1 ships the integrated custody machinery; B5 ships only its participant SDK.
Root-paid capture, membership exclusion, coordinated restore/release and pending
unpublished upstream extractions remain backlog. Auth redesign stays deferred.
Live creation continues to refuse before effects; dry-run records are not backups.

Current selections include Blob 0.27.1, Memory 0.41, Metrics 0.11, Timers 0.24,
Build 0.6, Host 0.19, Testkit 0.46 and Auth/protocol-types 0.6. Both private Blob
locks match. Adapter/fixture compilation, strict fixture lint and embedded-peer
provenance verification pass. Four selected installed cases pass on fixed inputs:
checkpoint freeze/restore, dedicated Blob permissions/recovery, embedded
application-state/recovery and production initialization target binding/lost
replies. The governed run takes 491.32s. The live-create refusal regression also
passes. Evidence: `/tmp/canic-0271-ready/`; Blob qualification is recorded on
[Blob #77](https://github.com/dragginzgame/ic-blob-storage/issues/77#issuecomment-6192039897).

The maintainer selected Auth 0.6 during qualification. Its default graph compiles,
but Core's native Auth tests expose the unconverted session-store boundary:
atomic login revocation changes, caller-based replay custody, session-only
removal and explicit restoration/verification features. This needs the actual
durable contract, not a signature-only adaptation. Scope clarification is pending
because Auth redesign was deferred. [#491](https://github.com/dragginzgame/canic/issues/491#issuecomment-6192040229)
owns the concrete failure and acceptance. The whole 0.112 batch is not push-ready
on this graph; the open changelog is updated, but publication readiness remains
blocked by that integration. No broad gate or own-version/Git effect ran.

## Current adoption and cleanup — 2026-10-14

Continue the existing B1–B5 release batch; own versions stay 0.111.0 and the draft
stays 0.112.0. Latest inputs and retained evidence are in the
[current handoff](../../status/current.md). Earlier rows below retain their
qualification graphs rather than claiming current-source installed acceptance.

| Outcome | Owner | Included evidence and focused validation | Status |
| --- | --- | --- | --- |
| Shared/Host adoption and installer repair | Shared 0.13.3; Host 0.18.2; Canic #563 | Canonical snapshot/governance, platform pins, failure propagation, Bash 3.2 and changed native fixtures | Local checks pass; no Git/release effects |
| Memory callback hard cut and composed graph | Memory 0.38; Canic #559 | Pool/bootstrap/accounting native cases; both Blob locks aligned to 0.25/0.38 and current Metrics/Timers; installed composition | Native cases and all three installed composition/production-initialization cases pass |
| Bounded Backup record acquisition | Host read owner; Canic #562 | Complete record decoding, 64 MiB read/write boundary, FIFO and aliases, durable refusal, terminal custody and CLI restore | Local checks and strict affected lint pass; broader B1 engine remains active |
| Selected build-script include exports | Canic #553; Cargo producer identity; Host read and IC Build evidence | Exact package/manifest/recipe admission, foreign same-name producers, malformed/nonregular records, absent depfiles and late/changed external inputs | Local 44 reuse, 10 artifact and 18 bootstrap cases, real-Cargo cold/warm matrix and strict Host lint pass on root Timers 0.22.6; installed receipts/private graphs remain on 0.22.5; delivery/native acceptance remains open |
| Fleet progress escaping | Host Text 0.18.2 | Real provisioning failure fields, visible controls/bidi escapes, existing heartbeat and screen lifecycle cases | Local owning cases and lint pass; interactive clipping/layout remain Canic-owned |


## Maintainer-requested live-readiness completion — 2026-10-14

The maintainer requested completing Build, Blob, Auth, Backup, IC tools and
Shared Tooling before live delivery. This continues the accepted 0.112 batch;
Git/release/publication, broad gates and live effects retain separate authority.

| Owner | Current completion work | Required acceptance | Status |
| --- | --- | --- | --- |
| Build / Blob | Select depfiles belonging to the current Cargo graph and generated Fleet targets; exclude unrelated shared-target Apps without dropping selected external inputs | Cold/reuse/mutation refusal and the dedicated, embedded and production-initialization managed cases | Published Build 0.3.0 integration passes 41 reuse cases; generated-input preflight is included. Strict affected Host/Backup/CLI lint passes. All three dedicated/embedded/production Blob 0.23.4 installed journeys pass |
| Auth | Measure the actual shared session-admission boundary; adopt published shape validation and the accepted Auth 0.6 storage/admission contract when available | Native-agent establishment plus exact replay, maximum-state budget and SDK denial cases | All eight native-agent cases pass on 0.5.10, including unchanged maximum-state limits; all eight cases also pass on the frozen graph; the maintainer deferred the coming Auth redesign on 2026-10-14; retain this qualified 0.5.10 baseline |
| Query receipt evidence | Use published 0.55.6 bounded certificate/tip/block APIs in the real registry graph | Existing receipt Candid, network/account/memo/amount, malformed evidence and budget cases | Real registry 0.55.6 adoption qualified: all 30 receipt cases and strict Host library/test lint pass |
| Backup | Complete original command/layout custody convergence before composing capture/restore execution | Original intent-before-dispatch, descendant exclusion, terminal receipt validation, lost responses and real same-release capture/restore | Original command/layout custody is integrated through published Backup 0.24.3/Host 0.17.12. All 309 package cases, 42 CLI restore cases, the Host bounded-spawn regression and strict affected lint pass. Live create still refuses; the maintainer-selected application-coordinated checkpoint and actual Fleet capture/restore remain pending |
| IC tools / Shared Tooling | Qualify selected Host 0.17.12 graph and snapshot 0.12.5; retain the independently selected Testkit 0.39 update | Exact tool receipts, consumer checks and native Linux/macOS acceptance | Reviewed Shared 0.12.5 snapshot (112 payloads), governance links, release adoption, formatter and IC installer fixtures pass on Linux. Host 0.17.12 is published and selected; exact prepared host/IC/Rust/Testkit tool receipts pass; native acceptance remains required |

The previous complete test attempt failed Store fixture compilation, Blob artifact
preparation and a stale native-agent metric assertion. The repaired Store
grant/revocation interruption case now passes on PocketIC; All three earlier Blob journeys and eight native-agent cases pass on their recorded
selected graphs. The updated graph has passing native Build/Query/Backup/CLI
evidence; all three current installed Blob journeys and all six Testkit payload-limit cases pass. Snapshot 0.12 integrity (111 payloads), IC tool
receipts and the selected Testkit 0.39 runner pass their focused Linux checks.
No area is declared fully qualified for live delivery across all required hosts.
Evidence for this continuation is retained under `/tmp/canic-112-completion/`.

## Duplication-removal batches — 2026-10-13

The maintainer requested the [duplication-removal plan](0.112-design.md#duplication-removal-plan--2026-10-13)
and its in-repository implementation. Published dependencies and product authority
remain the boundary. The maintainer separately authorized scoped IC Build and
IC Query implementation; publication remains separate.

| Batch | Outcome/owner | Included evidence | Targeted validation | Status |
| --- | --- | --- | --- | --- |
| B1 | IC Backup custody; Canic retains original/terminal authority (#513) | Exact command custody, descendant exclusion, retained references and terminal replay | Backup library/test lint, owning custody/restore cases and affected CLI/Host callers | Journal/command/layout guards now use published owners; original records, complete terminal provider and bounded Host capture are integrated. 309 owning cases, affected CLI cases, bounded Host caller and strict lint pass; remaining capture workflow is active |
| B2 | IC Build input evidence (#530, Build #12) | External/new/removed inputs, cold/reuse, mutation refusal and finalized release admission | Host owning reuse tests/lint; native host qualification | Published 0.3.0 adoption passes 41 reuse cases and the actual generated preflight; private scanner/snapshot/parser removed. Strict affected lint passes. Native acceptance remains open |
| B3 | Agent local query codec; IC Query bounded ledger evidence | Actual selected-origin HTTP, rejection, timeout and receipt budget/authority boundaries | Host replica-query and receipt tests/library lint | Local query codec removed; 13 HTTP/projection cases and strict Host library/test lint pass. Published Query 0.55.6 now owns certificate/tip and immutable bounded block-chain validation in the real registry graph; all 30 Canic receipt cases and strict Host library/test lint pass. Product mint binding and Candid projection are preserved |
| B4 | IC Auth protocol and validation (#491, Auth #50) | Canonical structure/hash, Fleet binding and non-issuing refusal | Contracts/Core owning auth tests/lint | The maintainer deferred the pending Auth redesign on 2026-10-14. Retain the qualified 0.5.10 baseline; future shape/storage/admission changes are separate |
| B5 | Application-coordinated checkpoint / Canic Root and application participant; IC Backup retains host plan, attempt and artifact custody | Exact Root installation/membership, application freeze, same-ID same-release capture/restore, interruption, delayed replies and complete release | Owning model/SDK checks and actual Root/application PocketIC journey | Maintainer selected coordinated checkpoints. Participant/SDK record contract shares one canonical phase, adds exact installed-Root endpoint admission and passes 15 native/Candid/authority cases and strict lint. Root capture/restore integration and installed qualification remain open under [#550 — Coordinated Fleet backup checkpoints](https://github.com/dragginzgame/canic/issues/550); direct host management permission cannot be inferred from Root control. Auth redesign is deferred and does not gate this work |

The complete 0.112 integration batch remains open. No full-suite, native macOS,
deployment or performance improvement is implied by these source changes.

B3 logs: `/tmp/canic-112-replica-tests-final-pass.log`,
`/tmp/canic-112-host-clippy.log`, `/tmp/canic-112-query-certificate-typed.log`
and `/tmp/canic-112-query-clippy-final.log`. Earlier failed attempts remain
retained; the frozen actual receipt consumer proof has a separate source graph.

The full ledger candidate is `/tmp/canic-112-certificate-consumer-02/`.
It archives the earlier certificate-only source, retains a source override only
inside that disposable checkout, and replaces the local generic value budget,
hash-chain and certificate decoders with IC Query. Canic keeps deposit schema,
exact network/account/memo/fee/amount binding and original mint authority. The
actual ledger-block Candid projection remains equal. The candidate was preliminary evidence; current registry qualification is now
recorded above and in `/tmp/canic-112-completion/query-receipts-final.log`.
Neither proof qualifies live payment effects. Receipts:
`/tmp/canic-112-full-ledger-consumer.log`,
`/tmp/canic-112-ledger-consumer-clippy-final.log`,
`/tmp/canic-112-query-chain-tests.log` and
`/tmp/canic-112-query-report-regressions.log`.

## Current planning identity — 2026-10-10

The maintainer moved this roadmap position to 0.112 so 0.111 is available for
the current hard cut. Already implemented adapter work remains part of the
coordinated delivery under [#444](https://github.com/dragginzgame/canic/issues/444);
the rename neither publishes it nor moves its existing evidence to a new graph.
The [current handoff](../../status/current.md) records Blob 0.21.5 qualification
on Memory 0.33 and the subsequent unqualified root move to Memory 0.34.
The following October 7 record retains its original versions and evidence.

## Retained implementation record

Date: 2026-10-07

The current Canic-owned adapter selects published Blob Storage 0.17.1. Its
independent workspace and both consumer lockfiles select Memory 0.31.1,
Timers 0.14.12 and arithmetic-only Metrics 0.2.9, matching Canic's runtime stack.
Timers directly re-exports Metrics' summary type. There are no cross-version
conversion adapters, application overrides or copied storage implementations.
[Canic#444](https://github.com/dragginzgame/canic/issues/444) owns the downstream
acceptance record; the [design](0.112-design.md) and
[current handoff](../../status/current.md) describe implementation and evidence.

Canic remains the sole lifecycle and default memory/timer runtime owner.
`mount!` contributes endpoints and application-selected memory requests;
`lifecycle::install` and `restore` run synchronously after Canic restoration.
The embedded application composes its durable counter and service rows into its
sole application sampler. `canister!` assembles those same parts for the dedicated
shell. DTO imports remain passive. The independent service owns its storage,
authority, provider, accounting and recovery contracts. Neither the adapter nor
the temporary service proof enters Canic's default release/test dependency graph.
The generic production initialization probe remains framework-owned coverage.

Root now supports production application initialization. A Component Spec with
`application_init_required = true` holds an allocated Component before installing
it. The controller binds 1–16,384 opaque bytes to the exact allocated Principal
and retained member operation. Root atomically retains and charges that binding;
its hash participates in the install intent. Ordinary and Component Group
installation deliver those exact inner bytes inside Canic's protected envelope.
Changed targets or bytes are refused; identical replay is effect-free, including
after completion. Host owns the bounded command encoder without knowing the
application schema. The application remains responsible for creating its input
after observing the real target. A fixture callback is test support only.

Complete release-bound dedicated and embedded Fast builds pass. Single-role
compilation does not embed the required release-build identity and therefore does
not qualify managed installation. Four native initializer boundary/recovery cases,
one install-intent reservation/replay case and two Host encoding cases pass.
Current Core native all-target/all-feature
and Wasm library Clippy, plus adapter Wasm library Clippy, pass with warnings
denied. Both consumer shells pass native scoped lint; authoritative Wasm qualification
belongs to the managed builds. Direct consumer Wasm Clippy correctly refuses
without validated build authority; no admission flag or guard was bypassed.
Both current-stack consumer forms pass PocketIC installation, typed caller and
tenant refusals, discarded mutation-response reconciliation, aggregate metrics,
same-release restoration and repeated current-instance recovery. Their complete
blob endpoint schemas match structurally. Eight complete normal Wasm graphs
(two application roles and their six infrastructure roles) each contain exactly
one Memory, Timers and Metrics identity. Evidence is retained in
`target/review-validation/blob-memory-qualification-20261007/consumer-pocketic.log`,
its per-package tree files and `runtime-identities.json`. That inventory derives
its graphs from the built application/infrastructure manifests and verifies
their exact artifact hashes. All three independent locks are refreshed; the
primary manifest/lock and earlier dirty work are unchanged.

The governed production Root case passes on this current graph in 192.98s.
It verifies held
allocation, wrong-target and caller refusals, controller allocation inspection,
Root interruption before binding, a discarded binding-command ingress reply,
exact full-bound 16,384-byte delivery, effect-free binding replay and synchronous
same-release application restoration. Its retained log is
`target/review-validation/blob-memory-qualification-20261007/production-pocketic.log`;
the exact governed runner finishes in 328s with normal server/scratch cleanup.
Root upgrade during installation quiesces pending IC management callbacks; this
case does not drop an install callback. Native tests separately qualify retained
install-intent bytes/hash and renewal after interruption. An actual uncertain
management install-response interruption is therefore not claimed as managed
execution evidence. These are local PocketIC results, not live deployment proof.

Read-only Toko source inspection still finds a consumer-owned shell using the
reusable adapter. Its retained lock includes older Blob Storage 0.15.2 and
Memory 0.28.4 alongside Memory 0.31.1; no current complete Toko managed graph is
qualified by the Canic proofs above. Its blob
Component Spec still lacks the required-init flag, and no application-owned
initializer producer or production binding caller was found in the inspected
source. Downstream acceptance needs those application changes and its own managed
execution. No Toko files were changed, and no Toko build or installation ran.
Independent preparer updates do not update the managed adapter or supply Root
arguments. Live uploads, reads and provider readiness remain unqualified.

The extraction removes the former embedded service, command group, billing Medic
option, stable allocations and blob-specific default test/CI lane. Published
historical evidence retains its recorded source/graph scope in earlier handoffs;
it does not qualify this graph. The temporary consumer execution proof does not
restore an engine suite or a permanent blob runner. Decoder-budget feedback stays
in [ic-blob-storage#7](https://github.com/dragginzgame/ic-blob-storage/issues/7).

These configuration, wire and stable-record changes are a pre-1.0 hard cut,
requiring clean reinstall across releases while retaining same-release recovery.
The maintainer-selected 0.110.53 draft remains unchanged; its existing human-owned
minor/release boundaries remain in
[#459](https://github.com/dragginzgame/canic/issues/459). No commit, version bump,
publication, deployment, broad gate or sibling mutation ran. Source removal does
not delete provider data, end billing or settle existing cycle/storage liabilities.
