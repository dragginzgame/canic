# Fixture build artifacts

Canic retains immutable fixture artifacts during a complete App build and delivers
them through reviewed Store publication and automatic target imports. Fleet
generation and initialization verify the exact selected fixture manifest and
payloads. Deployment completion requires each target's durable import receipt.

## Declaration

An attached application role selects its fixture source manifest through its
existing Cargo package metadata:

```toml
[package.metadata.canic]
app = "miner"
role = "translation"
fixture = "fixtures/source.json"
```

The `fixture` path is relative to that package's `Cargo.toml`. The source
manifest is a JSON object with exactly these fields:

| Field | Contract |
| --- | --- |
| `format_hash` | 64 lowercase hexadecimal characters identifying the application-owned encoding/schema |
| `completion_summary` | 64 lowercase hexadecimal characters identifying the expected application-validated completion summary |
| `chunk_paths` | Ordered relative paths to independently decodable chunks, resolved beside the source manifest |

Generate the document and chunk files before starting `canic build`. Canic
preserves their order and treats payloads as opaque bytes. It neither validates
database relationships nor invokes a generator or shell hook. The application
owns encoding, relationship order, import validation and receipt semantics.

Fixture paths cannot contain parent traversal or absolute paths, and observed
file/directory symlinks are refused. Fixture inputs must remain beneath the
selected workspace root. A symlink rejection names the exact linked component,
including a shared `.canic` parent; keep mutable state independent between
checkouts. Each chunk must be nonempty and fit the existing
1 MiB Store payload bound; the resulting descriptor must fit the existing Store
command envelope. These are transport bounds, not application instruction
budgets or production-load qualification.

## Build and reuse

For lock contention diagnostics and exact reuse guarantees, see
[build artifacts](../../architecture/build-artifacts.md#complete-build-reuse-and-compilation-phases).

A complete App build records the selected configuration, attached package
manifests, source documents and chunk hashes before compilation. This includes
explicitly declared inputs under directories such as `.canic` that ordinary
source scanning excludes. Packages without a fixture declaration are also
fingerprinted, so introducing a declaration during a build is detectable.

The host retains verified chunks under
`.canic/release-builds/<release-id>/fixture-content/` before compiling Wasms.
The canonical `fixture-artifact-manifest.json` binds release, Component topology,
role and content descriptor. Local paths and future target Principals are absent
from that artifact. The complete release manifest binds its digest through the
required `fixture_artifact_manifest_sha256` field. Builds without fixture
prerequisites bind an empty fixture manifest.

Before finalization, the host checks both current source fingerprints and the
retained descriptors against the initial observation. A temporarily changed
file cannot escape detection merely by being restored before the final check.
Exact completed-build reuse also checks the fixture manifest, retained payloads
and current configured source selection. It does not reuse finalized Wasms
across release identities.

Source changes, altered retained payloads, substituted manifest digests and
missing finalized artifacts reject. Exact local copying can resume before
finalization; finalized builds cannot acquire or repair fixture artifacts.
The current pre-1.0 schema changes through a hard cut: rebuild a complete release
to obtain the required fixture authority field.

Focused role builds remain artifact construction, without a complete release
manifest. Retained fixture artifacts are outside the application Wasm; selecting
a source alone does not import rows or make an application ready.

## Root source authority

The canonical Root release manifest includes a required `fixtures` list. It
binds each admitted role to its content ID and descriptor. Each Root receives
only the sources for its admitted Components and children; repeated references
to the same content count once within the fixture namespace. Root and host
preflight include fixture payload bytes in the Store allowance. Store admission
also accounts for retained metadata and grants, so payload preflight alone is
not a promise that all storage will fit.

The authenticated `PrepareStoreFixture` Root command selects a role from that
protected manifest. The caller cannot supply a replacement descriptor or source
URL. Root registers one bounded descriptor with its exact adopted Store and
returns the retained upload cursor. Repeating registration recovers that cursor.
The exact retained installation controller, while still an observed controller,
may upload hash-verified chunks directly to Store. It cannot register descriptors
or target grants. Uploads to unknown sources, unauthorized publishers and
conflicting chunks reject.

Root bootstrap and live status require exact complete source metadata. They do
not relay or re-read fixture payloads. This is the maintained pre-1.0 manifest
shape, with no predecessor fallback.

## Reviewed source publication

The host compiles one Root preparation action per distinct source, followed by
one direct Store upload per chunk, before Root bootstrap. Those actions use the
existing Fleet intent journal and reviewed cycle allowance. The selected role,
content descriptor, Store identity and expected byte progress remain bound to
the plan. Publication observes Root/Store status before deciding whether an
effect is already complete; a lost upload response can be reconciled from the
retained Store cursor. A later completed cursor also proves earlier chunks.
The protected `canic_root_fixture_status` composite query reads that Store
metadata; the ordinary Root status endpoint keeps its existing call behavior.

Each preparation/upload binds `maximum_attempts` from the desired Fleet's
`maximum_stalled_observations` setting when its plan is compiled. Its execution
reservation covers that many calls at the configured per-update burn bound;
retry observations also count toward conservation. Zero attempts and arithmetic
overflow reject planning. The allowance and exact content are included in the
action/plan hashes. Fresh-Fleet review includes one preparation and every chunk
per distinct content object in its per-Root successor bound. Its required
`fixture_publication_retry_attempts` records the additional permitted calls;
continuation funding reserves their configured update/observation costs separately
from the action count. The complete allowance participates in the review digest.
No default funding limit is raised.

The existing journal's required `publication_attempts` field consumes an attempt
before each call. Failed calls, lost replies and process interruption retain that
consumption; progress and restart cannot reset it. Observation runs before the
attempt gate, including at exhaustion, so a committed prefix can still reconcile
without another upload. An absent result at the limit returns typed
`FixturePublicationBound` and leaves the operation retained. Editing desired input
does not extend the issued plan's allowance. The current action/journal shape
changes through the pre-1.0 hard cut, without compatibility defaults or readers.

Plans and reports retain chunk hashes, byte sizes and local content references.
Reopening a plan verifies its exact preparation, destination, descriptor, chunk
bytes and expected progress. A file with a valid hash for different bytes cannot
replace the reviewed payload. Payloads are never embedded in report JSON.
Application rows and their import cursor remain outside this publication owner.

## Initial target grants

Root's bootstrap receipt and live Store status carry the protected manifest's
selected fixture descriptors. The existing Component/child installation owner
uses that selection and its durable installation operation to grant source
access after checking the target's module, controller, managed binding and
runtime installation identity. It rechecks the retained allocation and Root
identity after Store awaits. Grant issuance does not require globally Active
Root or application data readiness.

Initial issuance selects revision 1. Before installation, a new allocation may
select the successor to a disabled grant for the same physical target, Root and
release. The selected revision is stored in the existing installation intent and
protected target assignment. Retries reuse that revision without selecting again
from Store. An already enabled exact grant reconciles a lost response without
another update; a revoked or superseded selection rejects. Source calls also
recheck the active pool claim, so an old allocation cannot issue access after
recycling begins.
Before recycling a fixture-bearing allocation, Root revokes its exact Store
grant under the existing pending pool claim. It checks that claim and Root/Store
authority across calls; revocation failure leaves recycling pending before any
uninstall. An already revoked exact grant reconciles a lost response. Delayed
reset callbacks cannot overwrite a completed or replaced recycling claim.
Replacement requires a different installation operation. It cannot revive the
old installation's grant.

Recycled canisters may be stopped. The existing Installed phase checks the exact
module, Root controller and workload claim, then starts a stopped target before
runtime verification and grant issuance. Startup retries observe an already
running target without repeating the start call.

A grant proves source access, not imported application rows. The application
importer and its durable completion receipt remain the readiness authority.

The later-Shard qualification fills the initial Shard to the configured capacity,
removes the publication controller, interrupts Store access and retries the same
account. The existing Root installation owner issues the later target's grant;
the Shard imports automatically before assignment succeeds. Completed account
replay preserves the selected Shard and pool workload. This proves retained
source availability after initial deployment, not source-reference retirement.

## Installed source assignment

The non-root installation payload carries the selected Store, descriptor and
exact selected grant. That assignment persists inside the protected Component
runtime record and is exposed by runtime status. Root compares the installed
assignment with its selected payload before granting access. Directory and
activation status projections retain it unchanged.

Consumers and Store publication share the canonical content digest and chunk
verifier. Protected-state admission and status reject a changed installation,
release, managed binding, descriptor or content digest, and reject zero or disabled
grants. Root additionally compares the installed grant revision with its retained
selection.
The installed assignment owns no row cursor and does not assert data readiness.
Application import progress and the durable completion receipt remain separate.

## Registered application consumer

`canic::api::fixture_provisioning` exposes `FixtureImporter` and
`FixtureProvisioningApi`. Register one static application participant from the
existing synchronous lifecycle participant after restoring the database; register
again on a fresh heap after a same-release restart. Registration cannot replace
an existing participant within the same heap.

The application owns its durable checkpoint. Its `progress` callback is read-only
and returns the exact target binding, next chunk and optional completion receipt.
`begin` initializes an absent checkpoint without wiping existing data.
`apply_chunk` commits one verified chunk and advances that checkpoint together.
`validate_step` inspects a bounded portion of stored data and eventually commits
the receipt with the descriptor's validated summary. Canic validates the binding,
position and summary; application code remains responsible for database semantics.
If application infrastructure is still recovering, the read-only callback returns
`FixtureImportError::NotReady` before any mutation. Canic retries with the existing
provisioning backoff rather than invoking a mutating callback prematurely.

The internal delivery step performs one bounded unit of work. It holds one fetch
lease, reads only the installed source, verifies chunk length/digest, then rechecks current runtime,
assignment and durable progress after the await. No database request or borrowed
checkpoint survives the source call. Mutating callbacks must be synchronous;
Canic traps a returned error or invalid postcondition so rows and checkpoint
cannot commit partially. Application database execution scopes belong inside
those callbacks.

`status` reports an absent prerequisite, missing importer, pending progress or
an exact durable receipt. Receiving all bytes remains pending until validation
commits that receipt. A completed step replays by observing the receipt without
another Store read. Source, endpoint and transport failures return typed errors.
A mutating application callback's error becomes a trap to preserve rollback;
the trap diagnostic includes its application error code. Permanent failures
returned before mutation retain their exact diagnostic in protected recovery
state and report `Failed`, including after restart. A trapped callback retains
uncertain work for watchdog recovery; its application code is not a durable
failure record. Encoding or decoding failures report `Codec` with the original
diagnostic code and stop automatic retries. Transport outages remain retryable;
an incompatible response is not treated as an outage.

Canic schedules delivery automatically after runtime activation. Registration and
same-release restoration reconcile one retained native watchdog; lifecycle itself
never invokes application import callbacks. The watchdog pre-arms recovery in a
separate message before dispatching work, so a trapped callback cannot strand
its successor behind an ordinary running timer.
Source awaits also carry an exact durable attempt fence. Recovery reconstructs
pending demand after traps and restart, while completed receipts stop delivery.
Successful steps request an immediate successor. Transient failures honor the
existing provisioning backoff and are revisited on Canic's recovery cadence.
The existing durable attempt record retains the failure streak and retry deadline,
so a same-release restart cannot accelerate attempts during a source outage.
Only the current attempt may change that delay; successful progress resets it.
The delay doubles from one second to the existing 60-second provisioning maximum.
That maximum bounds the interval between eligible retries, not total attempts or
cycle expenditure; it is not a reviewed funding allowance.
The facade exposes registration and observation, not manual step scheduling.
Application code must not run a loop of steps within one message or use a callback
as a second lifecycle owner. Reviewed retry funding remains under the delivery boundary below.

## Funding while an initial import is pending

A registered initial Component may request cycles from its exact Prepared Root
before its fixture receipt admits application use. Funding uses the existing
role policy, per-request cap, cooldown, cumulative child allowance, parent reserve
and durable replay owner. Pending imports do not enlarge those limits. Exact
replay returns the recorded transfer without paying twice, including during
cooldown. Exhaustion returns the typed funding preflight rejection.

The permission does not admit unregistered callers or other Prepared capabilities
such as recycling. Funding neither completes the receipt nor admits application
placement. One focused PocketIC proof stops the recipient during exact balance
checks and qualifies actual transfers, cooldown and replay. A separate case keeps
the target running and holds its application import: the actual automatic timer
receives a full grant and a smaller final grant, reaches the configured child
allowance, and stops issuing requests after exhaustion. Releasing the hold completes
the same import with the selected targets and source bytes retained.

The disposable test raises its demand threshold to exercise funding without
burning the target's existing balance. Production defaults are unchanged.
Reviewed publication retry accounting and generated Fleet funding pass focused
qualification. The target funding allowance does not establish a lifetime limit
on execution from existing balances.

## Receipt-gated readiness

Protected `canic_observability(Readiness)` includes a structured `fixture` result
with pending progress, an exact completion receipt or a typed failure. Its overall
readiness remains `NotReady` until both runtime bootstrap and the required receipt
are complete. Public health and bootstrap observations remain infrastructure
observations; they do not assert that application data has finished loading.

Root verifies the exact receipt binding and completion summary before admitting
Component or child membership. The target also fences application queries and
updates before their handlers run. Only the exact Canic configuration/status
endpoint names and call modes remain available during loading, under their
existing authentication. An application endpoint with a `canic_` name receives
no exemption. Sharding assignment checks the parent's fixture prerequisite even
when called directly from application code.

Initial child bootstrap runs on infrastructure readiness. It can allocate,
configure and obtain source grants while the parent's data remains pending;
this prevents a parent/child readiness cycle. Import progress and receipt queries
do not advance loading or create another allocation. No additional durable
readiness flag or application cursor is introduced.

## Delivery boundary

A successful complete build supplies source authority; reviewed apply publishes
it and runtime receipts establish target readiness. A generated mixed-Fleet
PocketIC journey qualifies the ordinary sealed packages through publication,
actual upload reply loss, fresh-adapter recovery, exact source/receipt checks,
reviewed retry funding, cycle conservation and terminal replay. Its selected-build
and identical-build reinstall paths also converge and replay without effects.

The neutral test source is application-authored fixture data supplied to the host
manifest compiler. This proof does not measure Toko's Translation or Game Shard
payloads or qualify a live staging recovery. Those remain downstream work under
the [CANIC-165 design](../../audits/working/canic165-fixture-provisioning/design.md).
The
[combined readiness proof](../../audits/reports/2026-09/2026-09-11/canic165-readiness.md)
qualifies Prepared-Root initial-child ordering, receipt-gated membership and
application dispatch.

The [delivery and grant lifecycle proof](../../audits/reports/2026-09/2026-09-11/canic165-delivery.md)
qualifies autonomous later Shards after publication authority leaves, Store-outage
recovery, exact account replay and permanent codec failure across target restart.
It also proves revocation before recycling, stopped-target reuse with a new grant
and completed import, and rejection of a stale previous-installation revocation.
The Store/IcyDB journey covers reinstall after a partial automatic import and
reconciliation of discarded Store grant receipts across restart. The held-reply
extension below qualifies interrupted grant and revocation effects.

A further real Store/IcyDB case submits reinstall while the consumer's existing
fetch lease is active. It places Store and the consumer on separate disposable
subnets to observe the pending call between rounds. The replacement remains
empty after additional network progress and completes only under its new grant
and installation receipt; reads with the old grant reject. A controller-guarded
probe query observes the lease through the `internal-test-fixtures` feature,
without changing transport or scheduling.

This vanilla case establishes pending fetch at reinstall submission. A separate
case now holds the canonical Store response until after consumer reinstall,
then proves empty replacement state, exact replacement completion and rejection
of the old grant. Its response barrier is available only in internal test builds;
normal Store builds emit neither the barrier nor its controller-only endpoints.
The instrumented Store executes real authorization and storage operations, but
is not a production-finalized or byte-identical shipping artifact.

A Root journey holds replies after real grant and revocation mutations, starts
stop during each held call, drains the bounded caller response, and restarts the
same Root Wasm while Store still holds its reply. Grant revision, selected targets
and effect-free revocation replay remain exact. Root restore now reschedules the
retained provisioning owner while Prepared; the existing dispatcher preserves
retry deadlines and review-required failures. This uses the supported stop/drain
boundary before heap replacement, not a promise to interpret undrained callbacks
after a Rust heap replacement. The generated Fleet proof above separately uses
the ordinary Store package without these response controls.


## Source retirement

Source retention follows Root's existing draining and final-inventory operation.
A release that can create another workload keeps its sources, even after every
initial importer completes. Root closes source references only with its exact
terminal inventory intent, no retained workload assets and no pending lifecycle
effects. Its checks surround Store preparation so late allocation work cannot
race the retirement fence.

Store GC requests state their target explicitly. `Prepared` fences reads and
writes but retains bytes; repeating it never authorizes deletion. After logical
removal, the same Root operation requests `Complete`. Clearing reclaims one
bounded fixture entry per pass, keeps byte accounting exact across interruption,
and resumes after same-release restart. The existing Root owner retries pending
clearing before template cleanup, cycle reclamation and Store deletion. Terminal
replay does no work; conflicting operation identities are rejected.

The actual Store preparation/restart/clearing journey and a complete Root-owned
retirement after a real Shard import pass focused qualification. The Root driver
finishes the last subtree journal before final inventory, and retained-byte
accounting uses settled child records rather than their former installation
reservation. Asset handoff, Store deletion and exact Ledger-transfer replay are
qualified together.

Standalone Root removal remains unavailable while Coordinator group/service
references exist. That rejection retains the Registry and fixture sources; this
work does not introduce grouped application retirement. Generated publication and
funding qualification do not expand that retirement authority.
