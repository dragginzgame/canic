# CANIC-165 fixture provisioning: Canic integration design

Design work selected by the maintainer's 2026-09-11 upstream-feedback request.
This tracks the selected Canic-owned implementation. No release number is
allocated. The .15 CANIC-160/164 corrections and CANIC-166 diagnostic
remain independent of this product extension. No new minor begins here.

The maintainer selected implementation of CANIC-165 on 2026-09-11 and explicitly
continued issue fixes without a minor closeout audit. Release allocation remains
undecided; closeout is not a prerequisite to this selected implementation work. The
[application commit proof](../../reports/2026-09/2026-09-11/canic165-import-commit.md)
now qualifies published IcyDB plus an application-owned stable checkpoint,
including traps, returned errors, restart, bounded validation and same-Principal
reinstall fencing. The subsequent [transport proof](../../reports/2026-09/2026-09-11/canic165-transport.md)
qualifies held inter-canister replies, concurrent-pull rejection, post-await
trap cleanup, lost-response recovery and reinstall during a fetch. These use a
test source. The subsequent [retained Store proof](../../reports/2026-09/2026-09-11/canic165-store.md)
qualifies the actual Store's opaque source namespace, bounded upload recovery,
exact target grants, revocation and shared capacity. Subsequent checkpoints below
qualify automated delivery, receipt readiness and later Shards. Root now revokes
source grants before recycling and retains replacement revisions in installation
intent. Direct fixture-bearing Root retirement is qualified; interrupted effects,
reviewed funding and generated apply keep FP2 open.

## Outcome and current evidence

A reviewed release supplies immutable fixture data outside its application
Wasm. Initial targets and later-created Shards obtain their role's selected
data after the operator exits. Fleet completion and application placement wait
for the exact target's durable data-completion receipt. Canic owns delivery,
installation binding and readiness; the application owns row interpretation,
validation, writes and committed import progress.

The downstream CANIC-165 proposal dated 2026-09-11 is directionally sound.
Its current Translation loader advances one read page or write batch per
invocation; the earlier all-batches-in-one-callback observation is historical.
Embedding remains in `translation/design/src/fixture/mod.rs`, and heap progress
does not establish interrupted upload recovery. Removing embedded payloads
reduces executable data; it does not remove the database runtime or loaded
stable memory. Instruction and Wasm-size improvements require measurements.

Current Canic owners inspected:

| Owner | Existing behavior | Required extension |
| --- | --- | --- |
| `canic-host` release/build and Fleet Ensure | Exact build/desired identity, reviewed effects and resumable Store publication | Bind fixture content into release inputs and reviewed storage/funding; publish complete fixture objects before target delivery |
| `canic-control-plane` Store template storage/publication | Chunked Wasm manifests and publication-caller access | Explicit opaque fixture object namespace, retention and exact target read grants; fixture bytes cannot masquerade as Wasm |
| Core runtime activation and `canic::start!` | Prepared fences ordinary application endpoints; activation schedules application hooks | Keep runtime activation independent; schedule bounded target delivery through the existing lifecycle/timer owner |
| Control-plane Component and child allocation | Separately tracks runtime activation and directory membership | Retain pending allocation until exact data completion, for initial Components and later children |
| Application database participant | Owns rows and application invariants | One durable import/validation cursor and receipt with a proved local write/checkpoint boundary |

Source entrypoints include
[`nonroot.rs`](../../../../crates/canic-core/src/workflow/runtime/nonroot.rs),
[`start.rs`](../../../../crates/canic/src/macros/start.rs),
[`activation policy`](../../../../crates/canic-core/src/domain/policy/pure/fleet_activation.rs),
[`Store API`](../../../../crates/canic-control-plane/src/api/template/mod.rs), and
[`allocation driver`](../../../../crates/canic-control-plane/src/workflow/component_registry/lifecycle_drivers/mod.rs).

## Selected integration

Use one retained Store source and sequential target-pull delivery after runtime
activation. Reuse the existing Store owner, journalled release publication,
Component provisioning and timer owners. Do not add an operator daemon,
arbitrary URL source, generic shell hook, Root payload relay, database importer
inside Canic, or a second lifecycle. Keep the current Prepared endpoint fence.

The existing Wasm Store is not already this API. Its retained Root/installation
controller mutation predicate does not grant managed application canisters
fixture access. Introduce explicit fixture operations under the existing
role-owned command/status contract and an authenticated bounded data lane.
Root grants reads to the exact installed target for the selected fixture;
ordinary catalogue access and general Component membership are insufficient.
Target-source access must work before application placement eligibility, or
the readiness dependency would deadlock delivery.

Sequence:

1. Deterministically validate and chunk application-authored fixture data at
   build time. Include its content descriptor in governed build inputs.
2. Review and commit the complete source object through bounded, resumable
   Store updates. Incomplete objects cannot be selected for target delivery.
3. Retain the allocation/install intent, verify the installed target and grant
   source reads for that exact binding. Activate its runtime without asserting
   application data readiness.
4. The target fetches at most one chunk at a time and delegates one synchronous
   bounded step to its registered application importer. Re-read the binding
   after every await; a late callback cannot write into a newer installation.
5. The application incrementally validates stored rows and required derived
   projections, then commits its completion receipt. Canic observes that exact
   receipt before reporting deployment completion or admitting application use.

## Identity and authority

Separate content identity from installation authority. A content descriptor
contains format/schema fingerprint, encoded length, ordered chunk digests and
expected completion summary. Its digest must not include the release ID whose
build-input hash includes that descriptor; this avoids a circular identity.
The release manifest binds the resulting content digest to release and role.
Host-only paths and filenames never grant runtime authority.

Root derives each target grant from its existing installation/allocation
authority: network, Fleet, Root, parent, role, canister Principal, install ID,
release build and selected content descriptor. Source registration is allowed
only to that Store's exact Root; source reads require the registered target
caller and matching object/installation binding. A later Shard receives a new
grant under the selected active release. Future target Principals do not belong
in the original reusable content manifest.

The application receipt binds that same installation and fixture identity plus
its validated content summary. Canic verifies the expected summary and exact
authenticated target; application code proves schema/relationship semantics.
Canic must not interpret arbitrary table names or expose IcyDB's trusted
structural insertion as a public arbitrary-write interface.

No independently editable ready flag or operator skip switch exists. A role
without fixture data has no prerequisite by its compiled release contract.
Reinstall invalidates previous receipt authority even at the same Principal.
New product schema/protocol discriminators remain at 1. Release boundaries
reinstall disposable data; they do not migrate or accept predecessor state.

## Persistence and bounded work

Source ingestion and application import have different receipts: the first
proves retained bytes; the second proves validated database contents. Keep one
authoritative application progress cursor. Canic schedules and projects it;
Canic must not maintain a competing row index.

Use independently decodable, relationship-ordered, single-entity chunks and
one fetch in flight per target. Exact replay acknowledges already committed
content; conflicting or skipped content rejects. Retained cursor/accumulator
size is bounded, rather than an ever-growing request history. Pin the source
descriptor so a prior acknowledgement can be reconstructed without rewriting.

The application's rows and progress checkpoint need one proved consistency
boundary. Same-entity IcyDB atomic insertion alone is insufficient evidence for
a separate checkpoint. No await may split that commit. A typed error after
partial mutation must not be treated as rollback. The adapter must demonstrate
either atomic commit or exact reconciliation of that partial write before it
can advance. Qualify traps, returned errors and restart with actual PocketIC
execution. If the existing composition cannot provide this, document the
precise missing seam before adding an upstream dependency or new journal.

The managed probe now demonstrates this composition with IcyDB 0.257.4 and
ic-memory 0.13.2: a trap rolls back both independently stored rows and checkpoint;
a returned error after row insertion commits the rows without the checkpoint.
The candidate adapter therefore traps on failure between successful insertion
and checkpoint completion. It rechecks Canic's actual installed Fleet identity
at each commit/validation, and qualifies persisted progress and receipt replay.
No new database dependency seam was needed for this bounded neutral proof.

Validation scans actual stored records incrementally, checks unexpected rows,
counts/content identity and indexed relationships, and fences competing
writers. Final receipt creation compares already accumulated bounded summaries;
it cannot parse, sort, hash or cross-join the entire dataset. Any read projection
required for service must also be constructed or restored with bounded work.

Qualify encoded/decoded bytes, rows, field sizes, relationship lookups, descriptor
metadata, storage, concurrent target imports, instruction cost and retry/cycle
budgets. Numeric defaults come from that evidence; neither 128 rows nor the
historical 20M sampling reference is a fixture-loading safety proof. Reject an
individual record that cannot fit a bounded step at build preflight.

## Readiness, retention and interruption

Runtime Active permits infrastructure work; it does not assert fixture data
completion. Placement and the application target both enforce the receipt gate.
Health and protected progress remain observable. Initial Hub/Shard bootstrap
must not require a parent's data-ready membership to obtain its source grant
or initial child, otherwise the initial-child readiness dependency can cycle.
Qualification must cover that existing bootstrap dependency explicitly.

Root Fleet phase and target Component runtime phase are separate: a Prepared
Root can provision an Active child runtime. Exact target grants must work in
that state, before parent membership/data completion. The combined initial-Shard
PocketIC baseline now qualifies the fixture receipt gate: the child can finish
while the parent remains data-pending and Root remains Prepared. Allocation
completion, sharding pool Active state, assignment and membership admission
require exact completion evidence.

Keep the allocated target pending while it loads. Repeated observations cannot
create another Shard, assign a User twice or count loading progress as business
readiness. Recover a lost final response by observing the same durable receipt,
without repeating the complete import or validation.

Active release eligibility and issued target imports retain source references.
Completing the initial Fleet cannot collect data needed by future Shards.
Source outages leave targets pending and use bounded backoff. Permanent
authority/content/schema conflicts stop with typed diagnostics. Funding follows
the existing reviewed allowance path; fixture deployment must not silently
raise spend limits. Supersession fences late callbacks and reconciles issued
effects before references can be released. Cycle conservation remains required.

## Downstream acceptance refresh — 2026-09-12

Toko Miner's refreshed CANIC-165 handoff accepts the selected provisioning owner
and distinguishes unpublished partial evidence from a complete released seam.
Its funding/backoff and retirement list predates the newer local checkpoints
below: those checks remain covered within their recorded scope. Exact Root-await
and held actual-Store reply interruption, plus generated Fleet apply/funding,
remain Canic acceptance. CANIC-166 remains a separate fresh reviewed staging
recovery boundary, with no new local recovery framework requested.

Downstream adoption must cover both Translation and Game Shard against their
current build-time compacted embedding baseline. Retain matched source/toolchain,
profile and feature inputs for the embedded/external subjects, their distinct
exact release identities, and normal Canic finalization. Measure code, data,
custom, raw and gzip bytes separately; payload removal does not establish a code
reduction. Include new importer/decoder/validation code in the comparison.

The downstream receipt must prove decoded row equality and existing reads, import
and final-validation instruction/heap/stable-memory costs, and later Game Shard
delivery after the operator exits. Data unavailability must fence gameplay with
bounded retries. Reinstall discards prior rows/checkpoints; same-operation reply
loss remains recoverable. Remove embedding and its compaction output step together
when each role adopts the external path. This records downstream work only;
Canic agents do not modify either Toko role or claim those measurements here.

## Sequenced work and acceptance

| Batch | Concrete outcome | Required evidence and cleanup | Status |
| --- | --- | --- | --- |
| FP0 | Select Canic integration and identify unavailable seams | Owner/source review, identity/readiness dependency analysis, explicit persistence gate | Design checkpoint complete; no runtime API claim |
| FP1 | Prove one application-neutral importer and receipt boundary | PocketIC traps/errors/restarts around rows/checkpoint, bounded validation, no duplicate commit, stale-install callback refusal | Qualified against the composed probe and controllable source; includes callback traps, supersession, in-flight reinstall and durable receipt recovery |
| FP2 | Complete retained Store delivery through existing provisioning | Source upload retry, wrong caller/binding/conflicting chunks, initial parent/child receipt/grant ordering, pending placement, later Shard after operator exit, retention/outage/funding bounds | Store delivery, receipt readiness, later-Shard recovery, grant reuse, durable backoff, direct fixture-bearing Root retirement, grouped-removal rejection and automatic pending-target funding and reviewed publication retry accounting qualified; pending real fetch at reinstall submission qualified; exact held-reply effects and generated apply/funding remain; release position not assigned |
| FP3 | Propagate complete contract and cut over downstream roles | Shipped facade/config/Candid/docs/fixtures; Toko-owned Translation and Shard conversion, measured Wasm and per-message costs | Canic propagation belongs to FP2; downstream repository changes remain outside this workspace |

Initial parent/child readiness ordering is mandatory FP2 integration evidence:
it must exercise the actual retained Store grant and allocation gate introduced
there. Keeping it as a prerequisite to implementing those paths would make the
sequence circular. Neither the existing initial-child baseline nor a test-only
receipt observation substitutes for that proof. FP2 is complete only with source
and target interruption, final lost reply, exact replay, no duplicate allocation,
complete source retention and bounded worst-case finalization. Native decoder
tests alone cannot qualify it. No release/minor allocation is inferred here.

### Retained Store implementation checkpoint

The Store owns fixture allocation 68, disjoint from executable templates. Its
schema-1 descriptor hashes content independently of release identity. Admission
checks the complete serialized command against the existing command envelope;
chunk payloads reuse the existing 1 MiB transport bound. Neither introduces an
instruction-budget threshold or qualifies production fixture ceilings.

Grants reuse `ManagedCanisterBinding`, including Component children, rather than
inventing a second Registry identity. Exact Root intents compare a retained
revision; revoked revisions fence delayed grant/revoke messages. Endpoints
authenticate the exact target and full current grant before payload access.
The Store verifies bound authority; the subsequent Root checkpoints qualify
installed-target verification and release-selected source orchestration.

Fixture keys, descriptors, chunks and grant records count toward the existing
Store byte allowance in both fixture and template admission. Retained fixture
state blocks ordinary Store retirement. Reviewed reference release, terminal
retirement and funding/backoff remain implementation work;
the current retention guard is conservative, not the finished lifecycle.

The following host, grant and receipt checkpoints connect this Store primitive
to the existing provisioning owners. The Store-side PocketIC sender fixture
alone does not prove Root's autonomous issuance or initial child ordering.

### Host fixture artifact implementation checkpoint

The host source compiler now accepts role-scoped, application-authored chunk
paths, preserving their independently decodable order. It reads and verifies
one bounded payload at a time and retains exact bytes beneath the selected
release. A canonical child manifest binds release, Component topology, role,
content descriptor and digest; local paths and future target Principals are
absent. Payload files commit before the manifest. Interrupted copies resume
without overwriting exact artifacts, and finalized builds cannot acquire or
repair missing fixture artifacts.

`FixtureContentApi` owns the shared validation boundary used by host compilation
and Store admission. Publication planning verifies the observed source identity,
chunk count, byte prefix and completion flag before selecting one retained
chunk. Only an explicit authenticated `NotFound` requests preparation; other
Store failures propagate. It performs no remote effect and grants no authority.

[Host source qualification](../../reports/2026-09/2026-09-11/canic165-host-sources.md)
covers seven native host cases, six Store regressions, scoped Clippy and the
isolated Root feature compile. That initial checkpoint stopped at the host API;
the complete-build integration below now binds and consumes its child manifest.
Neither checkpoint qualifies automatic publication or application readiness.

Remote publication must preserve Root-only descriptor registration and target
grant authority while providing an explicit, content-scoped upload path for
the reviewed host publisher. The current primitive still admits only Root
uploads. Resolve this at the existing Store publication owner when wiring the
reviewed effects; do not route fixture payloads through Root or introduce a
second background publisher. No Store endpoint authority changed in this slice.

### Complete-build integration checkpoint

Attached application packages now select a source JSON document through their
existing `package.metadata.canic.fixture` key. Its format/summary hashes and
ordered chunk paths stay host-owned. Source paths do not enter runtime config.
A complete build captures package/config/document/payload fingerprints, retains
exact fixture artifacts before Cargo and verifies both live inputs and retained
descriptors before finalization. A transient change followed by restoration
cannot substitute copied payloads. Explicit fixture inputs participate in cache
fingerprints even beneath otherwise excluded source directories.

`CurrentReleaseSetManifest` now requires `fixture_artifact_manifest_sha256` and
binds an empty child manifest when there are no fixture prerequisites. The v1
schema changes through the current pre-1.0 hard cut. Exact build reuse validates
that child digest, retained payloads and configured source selection. It still
requires the same release identity. Fleet generation and installation argument
compilation reject fixture-bearing releases until reviewed delivery exists;
there is no path that silently ignores a declared provisioning prerequisite.

[Build-binding qualification](../../reports/2026-09/2026-09-11/canic165-build-binding.md)
records 42 focused host and 25 CLI cases plus scoped host/CLI/internal-fixture
Clippy. The [build contract](../../../features/build-and-evidence/fixture-artifacts.md)
documents the maintained metadata/source surfaces. The following checkpoints
qualify reviewed publication, grants and receipt-gated placement/dispatch.
The full FP2 batch remains open.


### Root source-authority checkpoint

The [qualified Root source path](../../reports/2026-09/2026-09-11/canic165-root-sources.md)
extends the canonical Root release manifest with required, admission-projected
fixture bindings. Build authority verifies the complete fixture child before
projection; Root independently checks admitted roles, ordered descriptors,
content IDs and combined payload allowance. Empty closures remain explicit.

`PrepareStoreFixture` takes a selected role and the staged manifest reference,
not caller-authored content authority. Root verifies its installed release digest,
checks its adopted Store, rechecks authority after the manifest read and registers
one descriptor. Store retains the only source upload cursor. The existing exact
Root/retained-installation-controller predicate now admits verified fixture
uploads; only Root may register descriptors or grants. Actual controller removal
fences subsequent uploads. Bootstrap and status verify complete source metadata,
without Root relaying or re-reading fixture bytes.

The actual Prepared-Root proof covers source selection, malformed manifest
reference, replayed registration, incomplete bootstrap refusal, direct publication
and completed replay. It does not qualify reviewed host journal execution or
target readiness. The Store composition proof additionally covers controller
upload/restart/removal and a 1 MiB chunk. Replay inventories include the new
source/grant operations; no command is excluded from exact variant coverage.

The host publication checkpoint below connects the existing journal and
content-addressed plan objects to these runtime owners. Fixture-bearing generation
stays gated until the full retention/funding lifecycle is complete. No release
allocation, external publication or downstream edit is part of this checkpoint.


### Reviewed host publication checkpoint

The Store-sequence compiler now schedules exact Root source preparation and
one bounded direct Store upload per chunk before bootstrap. Each action uses
the existing Fleet intent journal, observation/retry path and cycle accounting.
The preparation action pins the selected descriptor and Store; upload actions
retain constant-size expected prefix status instead of copying the full
descriptor into every chunk action. Reopening content-addressed plans checks
that expected prefix against the exact descriptor and chunk bytes. Reports
externalize payloads through the same object directory as Wasm publication.

The separate protected `canic_root_fixture_status` composite query permits
preparation reconciliation while Prepared. Ordinary Root status remains usable
by existing update callers; its query body does not make a Store call. The
[IC query contract](https://docs.internetcomputer.org/references/ic-interface-spec/https-interface/)
requires a composite query for the same-subnet Store read and excludes composite
queries from update calls.
Store cursor observations reconcile uploads, including a lost reply and a later
completed cursor. The earlier standalone publication selector is removed: the
Fleet protocol owner now owns action selection and reconciliation. No second
mutable source or row cursor was introduced.

The fixture-bearing generation gate remains until interrupted grant effects,
funding/backoff and source-reference retirement are complete. This checkpoint does not allocate a
release or make the combined worktree push-ready.


[Publication qualification](../../reports/2026-09/2026-09-11/canic165-publication.md)
retains 55 focused native checks, scoped warning-denied Clippy and the actual
two-chunk Root/Store journey (201.20s; 240s runner including builds). The exact
Prepared-Root composite-query admission is covered; wrong kinds and non-Root
roles remain fenced. The final 1,615-input snapshot remained unchanged during
runtime qualification. This checkpoint composes native journal recovery with
real canister commands; subsequent checkpoints qualify target receipt delivery.
Full generated apply remains gated on the complete lifecycle.


### Initial target-grant implementation checkpoint

Root now carries selected fixture descriptors through its protected bootstrap
receipt and derives each initial target grant from the existing installation
plan. It verifies the installed target before issuance and rechecks retained
allocation, installation and Root authority after Store awaits. Child verification
also checks the runtime installation identity. Exact enabled grants reconcile
replay; revoked or conflicting grants do not become new initial grants.

The [initial-grant qualification](../../reports/2026-09/2026-09-11/canic165-grants.md)
now includes protected target assignment. Its 47 focused native checks, isolated
role compiles, scoped Clippy and actual initial-Shard journey pass. Hub/Shard
runtime status retains the exact Store, descriptor and grant; unselected roles
receive no grant. Terminal replay preserves grants and pool observations. The
case takes 440.06s (525s runner including compilation and uncached Wasm builds);
all 1,622 source inputs and their inventory remain unchanged.

This checkpoint qualified fresh target access. The following consumer and readiness
checkpoints complete its import/receipt integration. Interrupted grants, reviewed
funding, revocation before reuse and source retirement remain; the combined batch
is not push-ready.


### Protected target assignment

The selected Store, descriptor and exact revision-1 grant now travel in the
non-root init payload and persist inside the existing Component runtime record.
Root compares installed assignment with selected intent before issuing access;
Directory preparation, activation and synchronization projections retain it.
Source hashing and chunk verification have one core implementation, with the
complete Store command-envelope bound still owned by publication admission.

Protected-state admission and status validate descriptor/content identity against
the exact target, installation and release. A target cannot initialize another
Principal's assignment. Grant replay accepts the original revision only. The
record adds no import cursor, mutable readiness flag or grant replacement path.
This supplies the registered importer with durable source authority; it does not
complete importer scheduling, application receipt readiness or source retirement.


### Registered automatic consumer

The core consumer accepts one synchronous application participant from the existing
lifecycle participant. Application-owned progress and receipts remain the only
data authority. The facade exposes registration and observation; the bounded
step is internal to Canic's automatic owner.

The [consumer qualification](../../reports/2026-09/2026-09-11/canic165-consumer.md)
connects one retained native watchdog, durable attempt fencing and permanent
failure retention. Lifecycle schedules work after restoration; it never invokes
application callbacks. The watchdog pre-arms its successor in a separate message,
so pre-await and post-await callback traps cannot strand recovery. The initial
ordinary-timer attempt failed that actual proof and was replaced. Successful
steps request immediate work; transient failures honor provisioning backoff at
Canic's existing recovery cadence.

Twenty native checks, scoped Clippy and all five composition cases pass. Actual
Store-to-IcyDB delivery qualifies outage recovery, returned-error rollback,
invalid checkpoint/receipt refusal, same-heap continuation, fresh-heap restart,
completed replay after revocation and persistent authority failure. Runtime cases
take 121.29s (164s runner); all 1,631 source hashes and their inventory remain
unchanged. The subsequent custody-guard cleanup passes all eighteen timer/memory guard
cases and scoped Clippy.

The 751-byte recovery record stores a fixture attempt fence and permanent returned
failure, not an application cursor or provider state. A mutating callback error
still traps to preserve rollback; its diagnostic code is not durably persisted.
Exact automatic held-response/reinstall races, reviewed funding and source
retirement remain acceptance work; parent/child ordering and later delivery are
qualified in the following checkpoints. Fixture-bearing Fleet generation stays disabled; the combined batch is not
push-ready.

### Receipt-gated readiness

The [combined readiness proof](../../reports/2026-09/2026-09-11/canic165-readiness.md)
connects the real retained Store and initial Hub/Shard bootstrap to Root membership
and application dispatch. Protected readiness carries the exact fixture result;
Root checks its own retained installation selection before admitting a receipt.
Application dispatch and direct Sharding assignment require data completion,
while infrastructure configuration, observation and initial child bootstrap remain
available. No second persisted readiness owner is added.

The actual Prepared-Root journey holds both imports, creates the initial child,
completes the child while the parent remains held, then reaches terminal
membership after releasing the parent. Pending observations retain the same
selected targets; repeated account assignment returns the existing Shard. The
separate Store/IcyDB journey proves gate behavior through outage, invalid progress,
receipt validation, permanent failure and restart. The later-delivery checkpoint
extends the same journey with an autonomous initial Shard while the parent stays
held, followed by another Shard after publication authority leaves. Interrupted
grants and the funding/reference-retirement lifecycle remain.


### Later-Shard delivery and codec failure recovery

The [delivery qualification](../../reports/2026-09/2026-09-11/canic165-delivery.md)
extends the same Fleet journey beyond initial membership. A later Shard receives
its exact grant and imports automatically from retained Store data after the
publication controller is removed. Store outage and account retries yield one
additional Shard; terminal replay preserves its assignment and pool observation.
The disposable Shard has no manual import-release endpoint. The initial Hub's
test hold still proves autonomous child progress while Root remains Prepared.

Encoding and decoding failures now stop with a permanent `Codec` diagnostic,
while transport outages remain retryable. The malformed-peer proof records one
read, no imported rows and the same retained failure after target restart.
Seventeen native regressions, all six targeted PocketIC cases and scoped Clippy
pass on unchanged recorded inputs. The recovery record remains bounded at 751
bytes. These checks do not replace the exact interrupted-grant, automatic
in-flight reinstall, pool-reuse and funding/reference-retirement acceptance.
Fixture-bearing generation and complete generated apply stay gated until that
lifecycle is finished.

### Revocation before recycling

Root now revokes the exact allocation's Store grant before physical reset. Its
existing pending pool claim fences both Store calls and late reset callbacks;
revocation failure leaves that claim pending. Observing an already revoked exact
grant reconciles the response without another mutation. No new stable-state or
reset journal owner is introduced.

The extended [delivery proof](../../reports/2026-09/2026-09-11/canic165-delivery.md)
passes removal after a Store admission outage, exact disabled grants, empty Ready
pool return, terminal replay and rejection of stale grant issuance. Sixteen native
checks, scoped Clippy and the 52.34s Fleet case (67s runner) pass. Background
maintenance may progress independently; the proof compares the affected target
and grant. Interruption inside the revocation effect still needs runtime evidence.
Replacement installation must retain its next selected grant revision before
effect; ordinary retries cannot infer a new revision from changing Store state.


### Retained replacement grant selection

A fresh allocation selects the successor of an exact disabled grant before
installation. Root, release and physical target must match; the installation
operation must differ. Parent and child install effects now retain the selected
revision, and all plan/effect equality and size accounting include it. Nullable
selection is explicit for roles without fixtures; missing fields do not become
an implicit default. This changes the maintained current stable schema through
the pre-1.0 reinstall-only cut.

Installed targets accept the positive selected revision, and Root compares their
assignment with its retained intent. Issuance rechecks the workload claim and
Root authority across Store calls. A stale installation cannot grant access after
recycling begins, and retries cannot select a newer revision from Store. The
replacement fixture uses the public application RPC to create a new child after
recycling; it does not add another production provisioning endpoint or journal.
The real-canister replacement case passes in 436.53s (498s runner including
uncached builds), with 1,634 unchanged source inputs and inventory. The same
physical Shard receives a new installation, successor grant and completed import;
exact RPC replay and stale prior revocation preserve the replacement grant.
The Installed owner now starts stopped recycled targets after checking their module,
Root controller and workload claim. Already-running retries skip the start effect;
the existing operation retains responsibility through runtime verification.
All 39 focused control-plane and four core native checks pass, alongside scoped
warning-denied Clippy. The delivery evidence topic records commands, log hashes
and the unchanged source snapshot; the complete FP2 batch remains open.

### Partial-import reinstall and Store receipt loss (2026-09-12)

The automatic Store/IcyDB journey now pauses a disposable importer after its first
committed chunk. Reinstalling the same physical target under a new installation
clears that row and checkpoint. While the replacement remains Prepared, the test
revokes the old grant and issues revision 3, discarding both command receipts at
the test caller. Store restart and exact replay preserve the selected grant;
stale issuance, revocation and reads reject. The replacement then activates,
starts empty during a source outage, completes automatically after Store returns
and retains its exact receipt across target restart.

All five composition integration tests pass in 109.13s (155s runner), alongside
scoped Clippy with warnings denied. The final 1,635-input snapshot includes the
concurrently updated lockfile and stayed unchanged through qualification.
Evidence is consolidated in the existing delivery topic. The pause is a
disposable application callback control; production scheduling and Store
endpoints are unchanged.

This proves an interrupted durable import and real Store receipt reconciliation.
It does not hold a real Store fetch response across reinstall or interrupt Root
inside its grant/revocation await. Those exact inter-canister boundaries remain
open alongside reviewed funding, reference release, terminal Store retirement
and fixture-bearing generated apply.


### Durable outage backoff — 2026-09-12

The automatic fixture consumer previously retained its retry streak and deadline
only in heap memory. Same-release restoration therefore reset outage pressure.
Both values now belong to the existing async-job recovery record. Exact attempt
completion commits the new deadline and releases the lease in one local message;
claim admission enforces the deadline before entering importer or Store work.
Late or foreign completions cannot change it. Successful progress resets it.
No application cursor, second journal, memory allocation or funding owner is added.

This uses the existing provisioning delay policy (one-second initial delay,
doubling to a 60-second maximum), including saturation rather than wrapping into
an immediately eligible deadline. The delay maximum controls retry frequency;
it does not bound lifetime cycle spending or complete reviewed retry funding.
Source-reference release, terminal Store retirement, exact interrupted-effect
qualification and generated Fleet apply remain part of the accepted batch.


### Source retirement through the existing Root operation — 2026-09-12

Source references derive from existing authority rather than a second reference
journal. The release remains eligible while Root can provision workloads. Only
its published draining and exact final-inventory intent, with terminal Component
history, no remaining non-Store pool assets and no pending lifecycle effects,
closes those references. Root revalidates this authority around Store preparation.
Completing initial imports alone never qualifies a Store for preparation.

Store `RunGc` now carries an explicit `Prepared` or `Complete` target alongside
its operation identity. This is a current-contract hard cut. Repeating preparation
only acknowledges the retained write fence and inventory; it never escalates to
collection. Collection is issued by the existing Root reclamation owner after
logical removal. The exact same operation can resume `Clearing` after restart,
and terminal requests are effect-free. A different operation cannot take over.

The Prepared fence rejects fixture payload reads as well as writes. Bytes remain
retained for final inventory until explicit collection. Each collection pass
removes at most one admitted fixture entry and its exact byte charge, retaining
the accounting key until the final entry is gone. This bounds local cleanup by
the existing admitted entry envelope without imposing a total source-size cap.
The Root owner revisits pending clearing; no fixture cursor or new timer owner is
introduced. Existing template cleanup, empty-Store checks, cycle reclamation and
physical deletion follow only after fixture bytes reach zero.

The Store sequence and full Root-owned direct Component retirement are now
qualified together with a real fixture-importing Shard. The retirement driver
uses the durable subtree completion result, including its exact Registry head,
when the last descendant membership disappears. Final Root accounting counts
settled child records; the installation reservation has already been released.
The complete journey covers retained source bytes, all workload/pool handoffs,
Store deletion, Root deletion and exact Ledger-transfer recovery/replay.

Coordinator group/service references intentionally fence standalone Root removal.
The grouped initial/later-Shard regression asserts that rejection and unchanged
Registry/source retention. This acceptance does not enable grouped application
retirement or weaken its separate lifecycle authority. Fixture-bearing generated
apply and its reviewed lifecycle must preserve that boundary. Reviewed funding
and the two exact interrupted-effect proofs also remain open. Detailed failing
and passing evidence is retained in the existing delivery report.

### Pending initial Component funding checkpoint

The real initial-Hub fixture exposed an admission deadlock: a registered Prepared
Component could allocate its initial child, but a cycles request was rejected
before reaching the configured funding policy. The exact Prepared Root now admits
that cycles request through the existing authority, role limits, cost guard and
replay owner. No allowance, timer, journal, schema or identity is introduced.

The initial/later-Shard PocketIC journey qualifies request clamping, cooldown,
exact replay, cumulative allowance exhaustion and the retained readiness gate.
Unregistered callers and Prepared recycling remain rejected. Only the recipient
is stopped during exact balance assertions. The disposable policy exercises a
full-sized grant and smaller final grant in two transfers, with a short cooldown.
Root executes actual management transfers. Restarting the recipient and releasing
the application hold completes the same import and provisioning operation. This does not qualify the target's
automatic funding timer or generated publication/retry cost review. Those remain
open with the two exact interrupted effects and generated apply. See the existing
delivery report for the initial rejection and final source-bound evidence.

### Automatic pending-target funding checkpoint

A separate actual PocketIC case now keeps the importing Hub running. Its normal
automatic timer requests the configured amount, receives a full grant and a
smaller final grant, then stops issuing requests after its cumulative allowance
is exhausted. An ordinary request independently confirms typed
`ChildBudgetExhausted`. Fixture readiness and Root membership remain pending until
the application hold is released; the same import reaches Ready while retaining
selected target identities and Store sources. The disposable configuration uses
a high demand threshold to avoid burning existing cycles. Production defaults
and all funding owners are unchanged. Both this case and the earlier
initial/later-Shard recovery regression pass with scoped Clippy; the existing
delivery evidence retains the final 1,638-input snapshot.

The next reviewed-funding correction remains within FP2: fixture preparation
and upload bind the existing per-action burn allowance, but failed calls can
remain at Intent without a durable paid-attempt counter. Pool maintenance already
persists its reviewed attempt consumption before issuing an update. A terminal
burn check and a stalled-observation count do not substitute for that boundary.
Use the existing Fleet journal and reviewed authority to account for publication
attempts before effects, retain consumption across process interruption, reconcile
successful lost replies without another upload and stop before unreviewed retries.
Do not silently increase funding limits or introduce a separate payment owner.
The exact interrupted grant/revoke, Store-fetch/reinstall and complete generated
apply proofs also remain; the combined batch is not push-ready.

### Durable publication retry allowance checkpoint

The existing Fleet owner now binds `maximum_attempts` into each fixture preparation
and upload, using the configured base `maximum_stalled_observations` when compiling
the reviewed plan. It reserves that many per-update burn allowances plus repeated
observations; zero attempts and arithmetic overflow fail closed. The existing
journal carries a required `publication_attempts` counter and persists consumption
before the call. Failures, lost replies and process interruption consume authority;
progress never resets it. Reconciliation precedes the attempt gate, so an already
committed prefix still completes at the bound. A missing commit at exhaustion
returns typed `FixturePublicationBound` and issues no further update.

The current action/journal schema hard-cuts without another journal, state owner,
compatibility default or funding mechanism. Per-call funding defaults are unchanged;
the larger worst-case total is explicit in the review. Editing current desired
input does not extend an issued action. Host regressions qualify restart, exact
persistence before issue, exhaustion and final lost-reply recovery, budget/hash
binding, overflow and invalid counters. The actual Root/Store publication/replay
case and scoped Clippy also pass; the delivery report retains exact source-bound
logs. Full generated Fleet funding/apply and the two exact suspended-effect cases
remain. This closes the previously identified local publication retry accounting
gap, not the complete FP2 batch.


### Pending real Store fetch at reinstall submission checkpoint

The Store/IcyDB integration now observes an actual fetch lease before submitting
consumer reinstall. Store runs on a separate disposable application subnet to
make the pending request visible between rounds. The lease observation is
read-only and feature-gated to internal fixtures; the application probe query
requires its test controller. No transport interception or alternate Store is
introduced.

After reinstall, the replacement remains NotBegun with no rows, application
admission or active fetch, including after ten further rounds and fifty seconds
of simulated time. Revoking the old grant and granting the new installation
allows that replacement to complete with its exact binding and summary. A read
using the old grant returns the typed authority rejection.

All six Store/IcyDB integration cases pass in 131.54s (163s runner), and scoped
core/probe/internal-testing/integration Clippy passes. The existing delivery
bundle retains the logs and `pending-real-fetch-reinstall-source.sha256`; all
1,639 source inputs and inventory remain unchanged.

The observation is at reinstall submission, not a deterministic hold of the
Store reply until after execution of reinstall. That stricter acceptance remains
open, as does interruption of Root inside its actual Store grant/revoke await.
The cross-subnet fixture does not qualify generated Fleet topology or complete
generated apply/funding. FP2 remains open and fixture-bearing generation remains
disabled; this checkpoint does not waive either exact interrupted-effect proof.
