<!-- Historical evidence only: superseded instructions, blockers and package selections below are not current authority. -->

# Current handoff — 2026-10-24

## Backup capture API cleanup — 2026-10-24

Removed dormant capture preflight application, journal transitions/receipt writes,
failed-operation retry and download-state mutation methods. Dry-run initialization,
passive records, evidence validation and status readers remain. Persistence and CLI
fixtures now construct retained records directly. The active restore engine is
unchanged; #513 retains coordinator convergence, and #550/#394 retain checkpoint
and live-capture requirements. README ownership and availability claims are corrected.

Pass: 248 canic-backup library tests, 52 CLI backup library tests, and strict
Clippy for both packages with all targets/features. The initial CLI binary filter
selected zero tests; the final library run is the coverage result. Corrected an
existing restore traversal fixture to exercise dry-run plan admission and assert
its current typed error. Evidence/before-images: /tmp/canic-backup-cleanup/.

This cleanup is ready within the open 0.112 batch. The complete release batch is
not push-ready; changelog drafts are updated but publication remains pending the
other accepted work. No full-suite gate, version change or Git publication ran.

## Public History Wire cutover — 2026-10-24

`canic_public_history` returns a direct native snapshot across history-capable
Root, Store, managed and local emitters. Contracts own the explicit method ID,
request and response budgets. Requests use native Principal; ops convert at the
unchanged cache boundary. Observatory shares bounded native reply decoding with
Metrics. Timer-authority callers use the same framed method. Removed public
History selectors, four projection types and obsolete Candid derives; protected
observability, configuration names, sampling and storage are unchanged.

Pass: two native History contract tests, 42 Observatory tests, 234 retained
projection tests and 47 activation tests. Final strict Clippy passes for
Contracts/Core/Host/facade tests and separately for the timer target. The complete governed
timer-authority PocketIC target passes all 10 cases (274s outer runner), including
bounded history, autonomous sampling, authority and restoration. Root and Store
governed Wasm builds, Store's canonical DID refresh and both exact declaration
checks pass. Scoped formatting and task-delta whitespace checks pass. Evidence:
`/tmp/canic-history-cut/`.

No full workspace suite, installed-IC, macOS, MSRV or TEE qualification is claimed.
This target does not qualify the embedded allocation peer. The complete batch
is not push-ready: remaining owned endpoint cutovers, production Wire schema
identity propagation and generated browser delivery still need completion.
Changelogs are updated; the batch is not ready to publish. No Git publication.

## Public metrics Wire cutover — 2026-10-23

`canic_public_metrics` now returns the typed native `PublicMetricsSnapshot` directly
across Root, Store and metrics-capable managed/local emitters. Contracts own its
method ID and both budgets. Public metric dimensions and sampler outputs use
native Platform Principal; ops convert at the unchanged internal model boundary.
Removed public Metrics union variants and metrics-only fixture projections.
Migrated Observatory, Blob composition, timer-authority and Fleet baseline callers.
The later History cutover is recorded above. Configuration names, collection,
cache limits, publication policy and storage remain unchanged.

Pass: two native codec/schema tests, including maximum-size 256-row payloads;
33 public-metrics tests, 47 activation tests, 238 projection tests, 41 Observatory
tests and exact canonical History/Root/Store checks. Scoped strict Clippy passes
for Contracts, Core, Host, facade, governed internal tests, timer-authority,
runtime probe, Blob adapter and the independent embedded consumer. Store's
governed Wasm build and canonical declaration refresh pass. Both complete
installed Blob runners pass: dedicated 347s, embedded 206s (all five recovery
scenarios). Worker successes alone are not the outer installed result.

Retained managed builds:

- Dedicated: `8518175fc5fb9c8f033b543d80e85840608234d35d43859a0323639414dcb64c`.
- Embedded: `edd41966c1e40837dcb6e88a3c0d1d98e597186c13b359adee28925e530957e2`.

Evidence and task before-images: `/tmp/canic-metrics-cut/`. All three lockfiles
are unchanged. No fresh standalone-local installed, combined native producer,
MSRV/macOS, installed-IC or full-suite result is claimed. Targeted PocketIC cases
do not run the embedded-peer preflight. ICP command/response handling has native
coverage; the installed proof uses PocketIC's Wire transport, not the ICP binary.

Next: migrate History and the remaining status operations with their callers.
Complete native producer coverage, normal build invocation, exact Wire schema
identity through release/binding records and generated browser delivery. Host
metrics still admits the existing retained release/profile binding; this is not
completed Wire release identity propagation. The maintainer is handling Tooling
#239; no private input-scanner workaround was added. The complete 0.112 batch is
not push-ready; the open changelog is updated but not ready for publication.

## Public health cutover and schema delivery — 2026-10-23

`canic_public_health` is now the shared typed Wire responsiveness query for all
role emitters. It returns `PublicHealth` directly with native Principal,
`Responding`, optional role and observation time. Contracts own its fixed method
ID and both budgets. Removed public `Health` union selectors, Candid derives and
projection callers; the protected diagnostic Health operation is independent.
Updated startup admission/read-only replay policy and local emission when no
public-status selectors remain. History and the remaining status operations still use Candid.

Pass: native payload/projection tests; strict Contracts/Core/internal governed
and timer-authority Clippy; governed Coordinator/Store Wasm builds and generated
canonical DID refresh; installed Coordinator case (63s runner) and dedicated
Blob composition (232s runner), including anonymous/non-controller health. Blob
build aa7e9625038a21ec13dd8a74cd3ef0b0bd1b811206035c4bd1a9e02afd6926f1
rebuilt Root, Coordinator, Store and Blob. The governed server tool was prepared
through the prescribed installer; sandbox socket binding required loopback
permission. Targeted cases do not run the embedded-peer preflight. No fresh
embedded Blob or standalone-local runtime, native schema producer, MSRV/macOS,
installed-IC or full-suite qualification is claimed. Six startup policy tests and all three locked dependency
admissions pass. Coordinator/Store/Root remaining Candid contracts match Rust;
strict declaration-checker Clippy passes. The embedded consumer lock gained only
the normal `ic-wire` dependency edge, without package-version changes. Root lock
is unchanged. Evidence/before-images: `/tmp/canic-public-health/`.


Schema acquisition now returns generated bindings and SHA-256 of the exact
admitted JSON bytes alongside its retained IC Build ownership record. Cached
results read the retained immutable output and re-admit it against final Wasm;
mutable materialization paths are not the identity source. This is the producer
result boundary, not completed release-manifest or frontend integration.
Evidence/before-images for this continuation: `/tmp/canic-wire-identity/`.
Pass: five focused schema admission/identity cases, the real retained-producer
regression (fresh/reused identity, output restoration and changed-Wasm refusal),
and strict Host library/test Clippy. Root/private lockfiles are unchanged. No
new installed or full-suite qualification; release/frontend propagation remains.

The maintainer selected separate typed endpoints per operation for the status
cutover. Emit capabilities at method boundaries, migrate all callers, then remove
the old union endpoints/projections; no fixed full-union or compatibility lane.
The design records this selection. Public health and metrics are migrated; the remaining
status operations and their callers are still pending.

Host `build_workspace_wire_schema` now resolves a configured role's package,
builds its `wire_schema` native example through the existing Cargo executor and
retains its admitted sidecar. It preserves runtime profile/default-feature policy
and selected features, adding `wire-schema` only to the native invocation. The
selected compiler identifies the host target; Cargo's exact manifest/example
artifact message selects the executable. The workspace build lock spans build
and acquisition. Do not nest this entrypoint under an already-held build lock.

IC Build owns the native Cargo input snapshot and transaction commit guard. Its
generic snapshot API supports executable examples despite the Wasm acquisition
restriction to cdylibs. The target-directory field owns snapshot target authority;
it must not also be passed as an environment override. The published Canic API
still does not run automatically from normal artifact finalization or seal a
release. Root/infrastructure producer entrypoints are not yet supplied.

The dedicated Blob native example links its actual mounted registry. Host admits
bounded output against final Wasm names, IDs, modes and all eight budgets before
retaining exact bytes. Wire owns type/reference admission and TypeScript output;
Blob owns payload projections/codecs/scalar checks. No inferred unit schemas,
Candid reconstruction, local scanner or cache implementation was added.

Pass: four native-build cases, including the real configured `fast` producer and
source-change refusal before publication (499s). Fast/debug producer schema bytes
match exactly (68,893 bytes; SHA-256
848c3e57670bfc1309338c86d5f01c1887429c17fa606ec647815563334ddf34).
The final Wasm is retained managed build
bb080f5a233e484af6ba1a395fdae0cd5242f422c1a320b5a921f2d4d562febd,
whose dedicated installed proof passed in the preceding slice (188s runner).

The maintainer updated the root lock during qualification. The complete native
build used Build/Host 0.66.0, Platform 0.6.11 and Auth 0.15.0. The final root graph
uses Build/Host 0.66.2, Platform 0.6.11 and Auth 0.15.1: strict Host Clippy, three
selection/source-custody tests and the actual producer retention/tamper regression
pass there. All three locked dependency admissions pass. The independent Blob
consumer lock is unchanged (Tooling 0.66.0, Platform 0.6.10, Blob 0.43.4). No
package selection was changed by this implementation. Current evidence/before
images: `/tmp/canic-wire-native-build/`; earlier producer/admission evidence:
`/tmp/canic-wire-producer/`, `/tmp/canic-wire-admission/`, `/tmp/canic-wire-supplier/`.

[Tooling #239](https://github.com/dragginzgame/ic-tooling/issues/239) records observed
hashing of a nested independent workspace's 45 GB generated target tree. Its
source-boundary fix belongs upstream; no Canic-specific exclusion bypass was
added. [Tooling #225](https://github.com/dragginzgame/ic-tooling/issues/225) and
[Canic #613](https://github.com/dragginzgame/canic/issues/613) retain composition.

Remaining: invocation from normal artifact builds, producer coverage for other
artifacts, exact release/frontend schema-hash binding, owned endpoint/caller
migration and remaining installed/payload/recovery acceptance. Native metadata
identity parity is not target-specific payload-layout proof. No fresh installed,
MSRV/macOS, installed-IC, TEE or full-suite qualification ran in this slice.

Continuation inspection confirmed a concrete endpoint-cutover obligation:
`dto::wire::managed` capability attributes prune public-status variants, while
Host readiness decodes `dto::wire::projection::overview` with only `Overview`.
Published Wire 0.6.10 encodes Serde variant indices, so simply replacing Candid
hooks would not give those types matching bytes. Resolve the payload layout and
all Rust projection consumers together before switching these endpoints. The
hard-cut design records this finding; no endpoint behavior changed in this
inspection. This is Canic work, not a demonstrated upstream defect. Several
substantial implementation/qualification batches remain; no completion percentage
or elapsed-time estimate is supported by the current evidence.

The complete Blob/Wire batch is not push-ready; the open 0.112 changelogs are
updated, not ready to publish. No Git publication, release or deployment.

## Supplier browser-runtime composition — 2026-10-23

Canic's canonical network schema owner now supplies the client adapter formerly
kept in the conformance example. It preserves supplier checks, adds the Canic
network validator, refuses replacing that check, and reexports the same WireError
identity for supplier adapters. Blob's published decimal adapter delegates to it;
no Blob validators or DTO schemas are copied. Fixed direct network validation
accepting a trailing newline through JavaScript's end-anchor behavior.

The new native `wire_composition` example combines the full upstream Blob method
catalog with an actual Canic network/Blob relationship payload. Strict example
Clippy, TypeScript 7.0.2 and Node 24.21.0 pass for exact Rust/TS frames, invalid
values from both owners and validator replacement refusal. The existing Canic
conformance example also passes, including the direct newline regression. The
initial JS fixture used a bare optional value; corrected it to Wire's generated
`{ some: value }` representation. Three-graph dependency admission and normal
Wasm feature checks pass; metadata/TypeScript generation stays out of deployed
graphs. Only an internal native dev dependency edge was added to the root lock.
Evidence and before-images: `/tmp/canic-wire-composition/`.

The compiler remains read-only sibling qualification tooling. Managed combined
schema production and release/frontend binding remain unfinished; this is not
browser-to-installed-IC, fresh MSRV/macOS or full-suite evidence. No Git publication
or deployment. The complete Blob/Wire batch remains not push-ready.

## Published Blob 0.43.4 descriptor adoption — 2026-10-23

Root and both private locks now select published Blob/contracts 0.43.4. Only those
two packages changed in each lock; no sibling override is used. The mounting
callback supplies canonical constants to Canic's raw Wire declarations. Managed
qualification recovers final Wasm records and compares exact IDs, query/update
modes and all eight request/response allowance fields with the supplying catalog
and native schema. Provider Candid has no Wire record; embedded application Wire
methods can coexist, with duplicate names/IDs refused. Payload codecs, storage
and lifecycle are unchanged; this does not establish target-specific type parity.

Pass: all three locked dependency/runtime-identity admissions; strict governed
internal and both private-consumer Clippy; dedicated installed proof (202s) and
embedded five-profile installed proof (187s), including rebuilt managed Wasm and
final-record checks; focused formatting. All three normal Wasm graphs exclude
wire-schema and TypeScript generation. Evidence, before-images and qualified lock
hashes: `/tmp/canic-blob434/`. The earlier prepared patch is now applied.

Blob #101's supplying API is adopted and locally qualified. Work remains local,
so this is not default-branch delivery or issue-closure evidence. Combined native
schema production, release/frontend identities, owned endpoint/caller migration
and remaining acceptance still block the complete batch. No broad suite, fresh
macOS/MSRV, installed-IC, Git publication or release is claimed. Not push-ready.

## Wire Result and access semantics — 2026-10-23

Wire no longer requires transport-level access rejection. Its declaration directly
selects native IO, allowing existing Result-based endpoints to preserve typed
access failures. Canic's existing diagnostic error carries host-only Wire
metadata; its fields and Serde encoding are unchanged. Explicit rejection still
uses the existing path. Proof-bearing guards consume the already-decoded Wire
argument, using the same guard as bounded Candid endpoints rather than reading a
second ingress encoding. This is adapter plumbing, not a new auth flow.

Pass: 52 macro tests, strict changed-target Clippy, eleven installed endpoint
cases (43s tests, 178s runner), and native metadata against the rebuilt managed
Wasm. The new installed Result case checks exact typed denial, guard short-circuit,
no handler mutation and valid retry. The later decoded-proof selection change
has focused macro coverage; it is not installed production Auth qualification.
The existing Contracts conformance example now also generates Result/error codecs.
Rust and generated TypeScript agree on exact frames for diagnostic codes 0, 1 and
65535 and Ok(u64::MAX); truncated/trailing frames and out-of-range codes reject.
Strict example Clippy, TypeScript 7.0.2 compilation and Node 24.21.0 checks pass.
The compiler remains read-only sibling test tooling, not an admitted production
frontend compiler. This is standalone byte parity, not browser-to-installed-IC
qualification. Explicit allocation budgets now accompany the example's methods.
Evidence and before-images: `/tmp/canic-wire-result/` and `/tmp/canic-wire-errors/`. No full suite, publication,
installed-IC or TEE claim. The full Blob/Wire batch remains unfinished and is not
ready to push/publish; the four remaining delivery areas below still apply.

## Protocol-owned Wire declarations — 2026-10-23

Wire attributes now accept a literal u64 or a path to the protocol owner's
constant, including macro-forwarded paths. Calls, arithmetic, noninteger literals
and literal overflow reject. The same expression feeds framing, static Wasm
declarations and native schemas. This removes the need to repeat protocol ID
numbers in consumer attributes; the probe's existing IDs/bytes are unchanged.
Raw endpoints can now select that same Wire contract instead of a separate
payload limit. They emit full compiled identity/budgets, while the owning library
retains decoding, reply encoding and payload projections. No unit schema is
inferred from a raw handler signature.

Pass: strict changed-target Clippy, 51 macro cases and ten installed endpoint
cases (latest run: 12s tests, 27s runner). Raw owner-codec cases cover ingress
and actual inter-canister calls, which bypass inspect_message. Persistent log
observations distinguish rejection before owner buffer access from decoder traps
and rolled-back state: the exact byte ceiling reaches the owner, one byte over
does not. Failed reply encoding rolls back state; a subsequent valid call works.
An async Wire composite query now also calls a second installed receiver and
encodes its reply after the callback. A downstream empty-canister rejection is
followed by a successful call; final Wasm and native schema retain composite mode.
Compiled metadata checks include exact ID/mode/budgets; the explicit native
payload projection also matches the built Wasm. Evidence and before-images:
`/tmp/canic-raw-wire/`, `/tmp/canic-raw-wire-relay/` and
`/tmp/canic-wire-composite/`. No broad suite, Git
publication, release, sibling edit or installed-IC claim.

[Blob #101](https://github.com/dragginzgame/ic-blob-storage/issues/101) is now
supplied by published 0.43.4 and adopted locally as recorded above. No copied
protocol IDs/budgets are needed. This is separate from #100's native projection
and the unfinished combined production schema/release binding.

Four substantive pieces remain: managed combined schema production/release hash
binding; owned production endpoints and callers; generated frontend replacement;
and complete target-specific/installed/tamper/recovery acceptance. This change
closes a prerequisite, not that complete batch. It is not ready to push/publish.


## Blob 0.43.3 published schema adoption — 2026-10-23

Root and both private Blob consumer locks now select published Blob/contracts
0.43.3 and Wire/derive 0.6.10; no sibling override is used. Contracts' native
wire-schema API supplies the method projections, generator and decimal-aware
runtime adapter requested in Blob #100. That missing published API is resolved.

The existing dedicated and embedded managed proofs now consume the supplying
catalog directly. They compare exact names/modes against each manifest-verified
final Wasm, cross-check IDs and all budget fields against Blob's runtime catalog,
and retain external-provider Candid classification. IDs/budgets are not independently
recovered from Blob's raw Wasm declarations; export checks are not type-layout
proofs. No copied DTO catalog, decimal validator or production endpoint change.

Pass: three-workspace locked dependency/runtime-identity admission, strict
governed internal and both private-consumer Clippy, dedicated installed proof
(193s) and embedded five-profile installed proof (167s), including new managed
Wasm builds and schema checks. Three native schema cases and the compiled registry
case also pass on Wire 0.6.10. All three normal deployed feature graphs exclude
wire-schema and TypeScript generation. Evidence and before-images: `/tmp/canic-blob433/`.
The source and lock changes remain local; no broad suite, fresh macOS/MSRV,
installed-IC, Git publication or release is claimed.

Next: compose these supplying schemas into production managed sidecars and bind
their exact bytes through release/frontend identities; complete #613's owned
endpoint/caller migration and target-specific payload qualification. Blob #100
remains open for that delivery boundary, not because its host schema API is missing.
Independent positive provider-credit evidence under #494 remains separate.
The complete Blob/Wire batch is not ready to push or publish.


## Native Wire schema bound to compiled identities — 2026-10-23

The native schema snapshot now requires the declarations read from final Wasm.
It refuses missing/extra/duplicate inventory and exact ID, export-kind or any
request/reply budget mismatch before returning metadata. There is no unchecked
snapshot entrypoint. This checks compiled identity, not target-specific type
layout or sidecar custody.

Pass: three schema unit cases, compiled query/update/composite registry test,
native payload-probe schema compared to the previously qualified managed Wasm,
and strict all-target Clippy for Canic and that probe with wire-schema enabled.
The probe checks Platform's canonical native Principal projection and a mutated-ID
refusal. Its explicit ignored test requires CANIC_WIRE_PROBE_WASM; ordinary test
execution alone does not run that cross-target proof. Normal Wasm graphs remain
free of JSON and Host Artifacts, including with wire-schema selected. Runtime
endpoint behavior is unchanged; the preceding seven-case installed proof was
not rerun. Evidence/before-images: `/tmp/canic-wire-binding/`.

Next remains target-specific type parity, managed sidecar/release hash propagation,
production endpoint/caller migration and generated frontend acceptance. Blob #100
owns the supplying schema gap; #494's independent positive-credit evidence remains
separate. The complete Blob/Wire batch is still not ready to push or publish.
No Git publication, release, live deployment or sibling edit.

## Compiled Wire artifact identity — 2026-10-23

Typed Wire adapters now emit compact private Wasm declarations with exact
name/ID, export kind and both codec budgets. Contracts owns their fixed framing;
Host checks actual exports, duplicate identity and codec-owner overlap. Shrink
and Binaryen must preserve those records before output publication. Native
type-schema generation stays host-only. No production endpoint was migrated.

Focused Host artifact tests (27), declaration framing tests (2), native compiled
registry test, 49 macro tests and strict Host/probe/macro/schema Clippy pass. All seven owning installed
payload cases pass, including assertions on the actual managed Wasm declarations,
frame refusals and rollback (42s test time, 181s runner). Evidence and task-start
before-images are retained in
`/tmp/canic-wire-artifact/`. No broad gate or installed-IC claim.

This still does not compare native type projections to the final target or bind
release schema hashes. Those checks, sidecar propagation, production callers
and generated frontend bundles remain required for #613. Blob #100 owns the
published Blob schema export gap; independent positive provider credit remains
separately unqualified under #494. The complete Blob/Wire batch is not ready to
push or publish. Existing dirty work is preserved; no Git publication or release.


## Compiled Wire schema registry qualified — 2026-10-23

Canic now has optional native-only wire-schema registration emitted by the same
macros as typed Wire endpoints. The deterministic v1 registry retains exact method
name/ID, actual query/composite/update kind, browser transport mode, named native
Serde request/reply projections and both full codec budgets. Wire owns projection
and generator validation; Canic rejects empty/duplicate inventories. The feature
is classified as non-role support by the existing closed Cargo/role catalog.
No production endpoint or storage format was migrated in this slice.

Pass: compiled query/update/composite registration, two registry refusal/order
cases, all 48 endpoint macro regressions, Cargo/catalog parity, strict scoped
Clippy and seven installed endpoint/payload tests, including Wire frame refusal
and synchronous rollback. A missing host-only catalog classification was caught
and corrected by the installed test. Normal Wasm includes runtime Wire but no
serde_json/TypeScript metadata, even with wire-schema selected.

The incoming root lock moved from Wire 0.6.8 to 0.6.9 during the first installed
run. Preserved that update; reran registry tests, strict lint, graph inspection and
all seven installed cases against a recorded unchanged 0.6.9 lock (13s PocketIC
lane). Evidence and task before-images: `/tmp/canic-wire-registry/`;
`qualified-lock.sha256` binds the final run. No broad gate, macOS or installed-IC
claim; no Git publication, release or sibling edit.

The Blob/Wire intersection has a verified supplier gap: Blob 0.43.2 has codec and
method-name owners but no host method-schema/DTO projection. Filed
[Blob #100](https://github.com/dragginzgame/ic-blob-storage/issues/100), after
searching existing issues. Do not copy Blob schemas downstream. Installed Blob
behavior remains qualified on its recorded graph; independent positive provider
credit remains separately unqualified under #494.

Next Canic work: compare compiled registry to final artifact inventory, carry an
explicit wire_schema_sha256 through Build/release/frontend artifacts, migrate
owned production endpoints and their callers, and qualify generated clients.
Native registry tests alone do not prove target/capability parity or full #613.
The complete Blob/Wire batch is not 100% or ready to push; open changelogs and the
migration contract describe the actual progress and remaining boundaries.

## Published upstream family adopted and qualified — 2026-10-23

Selected from crates.io: Host/Build/Testkit 0.66.0, Query 0.70.0, Backup 0.40.0,
Auth/protocol-types 0.15.0, Blob/contracts 0.43.2, Platform 0.6.10, Wire/derive
0.6.8, Memory/Storage 0.50.13, Metrics 0.15.7 and Timers 0.30.7. Root and both
private Blob consumer manifests/locks align; no sibling source override is used.
Shared Tooling snapshot is committed revision 0b07c2a (version 0.26.8).

Migrated seven Testkit call sites to explicit ownership APIs. Connect callers
retain external server custody; standalone/baseline capture uses the named
borrowed-server constructor. Build's release-editor pin advances from 0.22 to
0.66 with its required cli feature. Both tools are prepared through Shared
Tooling; the real-Cargo/fake-Git release transaction and rollback fixture passes.

Pass: strict governed internal and both private-consumer Clippy; all three
locked dependency/normal-Wasm runtime-identity admissions; 32 focused shared
Auth session-store tests; embedded five-profile PocketIC case (360s proof lane)
and dedicated case (63s), including managed Wasm builds. Scoped rustfmt,
shellcheck and snapshot verification pass. Logs and task-start snapshots:
`/tmp/canic-upstream066/`. No broad suite, native macOS, fresh MSRV execution or
installed-IC/provider qualification is claimed.

Important current source boundary: authorize_local_application remains denial-only
and the shared application-session adapter is cfg(test). Auth 0.15 adoption does
not implement production portable admission or login. The coordinated #613 Wire
cut and independent positive provider-credit evidence under #494 remain separate.
#620/#621 were closed after checking their existing implementation on remote main;
newer adoption/funding work is still local. Complete 0.112 is not ready to push;
changelogs describe this adoption without claiming release readiness. No staging,
commit, push, release, publication, deployment or sibling edit was performed.

## Mounted paid Blob outcomes qualified — 2026-10-23

The remaining inspection gap was per-operation paid output, not the aggregate
journal already covered. Added exact assertions for acknowledged callbacks
(ProviderAcknowledged 75, original request, refund 25, reported balance and exact
renewal), accepted malformed/provider errors (CreditRequired 75 with distinct
response classes), and callback traps (TransferUnknown 100, Uncertain phase and
no retained response). Fences and all fields remain exact through recovery.
Repeated reads, anonymous callers and substituted offers leave stable memory
unchanged; refusals are typed Denied/Conflict. Budget payments also retain zero
renewal while older paid operations retain their immutable grants.

Strict governed internal Clippy and the exact five-profile embedded PocketIC
case pass, including managed Wasm builds (165s runtime lane). Evidence:
`/tmp/canic-blob-paid-outcomes/pocketic.log`. The audit's current mounted table and
#494/#622 descriptions are reconciled with this evidence; older dated records are
historical. No production behavior changed and no broad suite was run.

Positive independent-credit/immutable-receipt acceptance remains unqualified.
The shared API exists, but genuine provider/account evidence is application-owned
under Blob #34; successful callback/balance data is insufficient. No missing API
blocks ordinary acknowledged funding. Default-branch delivery and #613 remain
separate, so no 100% delivered or complete-release-ready claim. Changelogs updated;
no commits, pushes, release, publication or deployment performed.

## Blob ceiling and lifetime capacity qualified — 2026-10-23

The embedded managed budget profile reaches the 10,150 cumulative renewal ceiling
using the two existing original grants, then completes operations 5 and 6 as
acknowledged payments. Even a one-cycle renewal returns RenewalCeiling. Operation
7 receives the sole JournalFull blocker despite sufficient spendable allocation
and acknowledged prior payments. Exact replay of grants for operations 3 and 4
adds nothing and frees no history slots; operations 5 and 6 cannot be resent.
Mounted totals retain 300 accepted/acknowledged/uncredited, 300 refunded, 9,850
available allocation, the installed ceiling and full lifetime capacity. These
facts and refusals survive authenticated same-release restoration/recovery.

Pass: strict embedded-consumer and governed internal Clippy; exact installed
embedded case covering all five independent profiles, including managed Wasm
builds (171s runtime lane). Evidence: `/tmp/canic-blob-budget/pocketic-final.log`.
An initial fixture catalog refusal was corrected by budgeting the four explicit
method-name slots; production limits did not change. No broad gate or real
provider qualification is claimed.

Positive independent-credit evidence/receipt replay and default-branch delivery
remain open in #494; current acknowledgments need no new provider receipt API.
Do not fabricate stronger evidence or call the complete batch deployment-ready:
#613 Wire work remains separate. Changelogs are updated. No commits, pushes,
release, publication or deployment occurred.

## Blob credit-refusal boundary qualified — 2026-10-23

The installed embedded fixture now checks independent-credit refusals against an
accepted unresolved payment: a foreign operation returns Binding, partial amount
returns AmountMismatch, and an empty digest returns EmptyReceipt. Each callback
observes the original exact intent, accepted 75 cycles, no credit and no provider
acknowledgment. Existing None-evidence, renewal and payment refusals continue;
restoration retains the same facts. No valid receipt hash was fabricated.

Artifact-worker preparation now runs once at each owning test entrypoint rather
than once per failure scenario. All four embedded installations still execute.
Strict embedded/internal Clippy, embedded managed case (144s) and dedicated case
(56s) pass, including managed Wasm builds. Evidence:
`/tmp/canic-blob-credit-refusals/{pocketic,dedicated}.log`. This is local Linux/
PocketIC qualification; no native macOS or real provider evidence is claimed.

Positive independent credit remains unqualified. The shared confirmation API is
available; its contract requires uniquely attributable, independently authenticated
provider evidence retained by the application owner, whose actual bytes are hashed.
Current provider requests carry no local operation ID, and a balance or successful
reply cannot supply that attribution. Blob #34 owns remaining provider facts;
ordinary acknowledged funding does not require this stronger evidence or a new
API. #494 remains open for that positive acceptance and delivery. Do not fabricate
a receipt protocol or call this 100% delivered. Full release work also retains
#613. Changelogs are updated; no commits, pushes, releases or deployment occurred.

## Blob full-offer liquidity refusals qualified — 2026-10-23

The installed embedded case adds a fresh liquidity profile. Three original
operations each offer the entire 9,900-cycle journal allowance: operating reserve
consumes observed liquidity; other liabilities are u128::MAX; or apparent room
covers the attachment without platform call costs/final journal-write charges.
Each reaches shared platform admission, records NotDispatched with a positive
call-cost bound, and refuses later redispatch. Mounted totals retain 29,700 unsent,
zero newly accepted/uncertain, full 10,000 available allocation and 9,900 attachment
allowance. The existing 200 refunded cycles stay unchanged. Restoration/recovery
preserve these records and replay refusals. Production funding policy is unchanged.

Pass: strict embedded-consumer and governed internal Clippy; installed embedded
PocketIC case covering all four independent profiles, including managed Wasm
builds (253s runtime lane). Evidence: `/tmp/canic-blob-liquidity/pocketic.log`.
Linux/PocketIC evidence is not native macOS or provider deployment qualification.
This validates shared call-cost admission and supplied holds, not a separate
measurement of every replica callback-cost component.

Remaining #494 work: positive independent-credit/immutable-receipt acceptance and
default-branch delivery. Current upstream #34 confirms that stronger provider
linkage/activity/reconciliation facts remain unqualified, but no new receipt API
blocks the already-released acknowledged-payment path. Do not fabricate receipt
hashes or treat balances as independent proof. No missing upstream library API
was identified. Full release readiness still includes #613; Blob is not claimed
100% delivered. Changelogs updated; no commits, pushes, release or deployment.

## Blob settlement callback trap qualified — 2026-10-23

The embedded managed case now runs three typed independent scenarios: malformed
reply, held provider error and held provider error followed by an intentional
consumer settlement-callback trap. The provider commits acceptance before its
barrier. After release, `CanisterCalledTrap` is observed; the shared callback's
journal writes roll back. Original operation 5 and its full 100-cycle uncertain
reservation remain, recorded accepted/refunded totals stay unchanged and the
recovery fence activates. Authenticated same-release recovery clears only that
fence. Original redispatch and preparation of operation 6 remain blocked.

Strict embedded-consumer and governed internal Clippy pass. The exact installed
embedded PocketIC case passes all three profiles, including managed Wasm builds
(162s runtime lane). Evidence: `/tmp/canic-blob-callback-trap/pocketic-final.log`.
The initial test expected no fence; its failure correctly revealed the upstream
safety fence, and the maintained assertion now requires it. Production semantics
were unchanged. These are Linux/PocketIC results, not live-provider qualification.

Remaining Blob acceptance is positive independent-credit/immutable-receipt replay
and maximal-liquidity reserve/callback/other-liability cases. The trap proof does
not establish every platform outage mode. #494 remains open for these gaps and
default-branch delivery; the full batch still includes #613 Wire work. Changelogs
are updated, but the batch is not deployment-ready. No commits, pushes, release,
publication or deployment were performed.

## Held accepted Blob error qualification — 2026-10-23

The embedded managed case now runs separate fresh installations for malformed
and structured provider-error outcomes. Published Testkit's accepting held fixture
commits 75 of the original 100 attached cycles before its response barrier. Two
concurrent ingresses cannot repeat operation 5 or prepare operation 6; the original
pending request, five retained intents and 100 reserved/uncertain cycles remain.
Release the provider's AccountBalanceOverflow reply without collecting the original
ingress result: mounted observations recover exact accepted/refunded totals,
acknowledgment remains unchanged, and renewal/retry/new payment remain blocked.
Same-release restoration and authenticated recovery preserve those facts.

Strict embedded-consumer and governed internal Clippy pass. The exact managed
embedded PocketIC case passes both profiles, including managed Wasm builds (123s
runtime lane). Evidence: `/tmp/canic-blob-paid-errors/`. The existing root selection
of Wire 0.6.7 was preserved. No production source, journal semantics, fabricated
credit evidence or payment scheduler was added.

Remaining #494 acceptance: a trapped/lost settlement callback (distinct from an
uncollected ingress reply), independently evidenced positive credit/receipt replay,
and maximal-liquidity reserve/callback/other-liability cases. Deployment evidence
remains application-owned. This narrows the gap but does not establish 100% Blob
qualification or release readiness; #613 and pending delivery remain separate.
Changelogs are updated. No Git publication, release or deployment was performed.

## Installed paid Blob acceptance — 2026-10-23

Both managed Blob compositions pass on 0.43.1. The root lock had advanced outside
this slice; both private locks were aligned without reversing that selection.
Published Testkit 0.65's bounded acceptance fixture replaces the obsolete #235
publication hold. The installed embedded case now proves two distinct acknowledged
payments, each offering 100 with exact callback acceptance 75/refund 25; accepted
amounts remain uncredited. Each original permits one 75-cycle budget increase,
rejects renewal above acceptance and replays without another grant or payment.
The mounted totals and replay survive same-release restoration/recovery.

A third partially accepted malformed reply retains 75 unresolved cycles. Absent
independent credit evidence changes nothing; renewal, original redispatch and the
next intent are refused. Restoration retains those facts. The recovery fixture
reacquires only typed stale `Binding` proof refusals, with three total attempts;
all other errors remain fatal. This follows the shared read-only proof contract.
No production funding policy or provider evidence source was introduced.

Pass: strict internal/governed and embedded-consumer Clippy, all three dependency
admissions, managed embedded and dedicated PocketIC cases (including Wasm builds).
Evidence: `/tmp/canic-blob-paid/`, especially `pocketic-recovery.log` and
`dedicated.log`. Initial lock-mismatch and stale-proof failures are retained.
These are Linux/PocketIC results, not native macOS or live-provider qualification.

Blob is not yet 100%: #494 still needs accepted provider-error/lost-callback,
positive independent-credit/receipt replay and maximal-liquidity reserve proofs.
Real provider evidence acquisition remains application-owned. Qualified local
work also needs delivery before tracker closure. The full batch retains #613's
Wire migration; changelogs are updated but the batch is not deployment-ready.
No commits, pushes, release or deployment actions were taken.

## Query 0.69 / coherent Host and Backup adoption — 2026-10-23

Published Query 0.69.0 resolves the upstream type-alignment blocker in
[Query #101](https://github.com/dragginzgame/ic-query/issues/101). Its closure
records a published Host 0.65 dependency change and registry-consumer proof;
it does not claim that Canic was already migrated. Canic now selects Query
0.69.0, Backup 0.39.0 and Host Artifacts/FS/Tools 0.65.0 alongside the already
selected Build, Process/Text and Testkit 0.65.0. Host 0.64 identities are removed
from the root lock. No adapters or source behavior changes were needed.

Strict Host/Backup/CLI library Clippy passes. Focused tests pass: 10 Backup
artifact cases, 17 Host artifact cases and 15 certificate/receipt cases.
Locked metadata and dependency-boundary admission pass in all three workspaces;
the two independent Blob locks require no changes. Evidence is retained under
`/tmp/canic-query069-adoption/`. Shared Tooling stays at revision 35bc5a9.

The previous Query/Backup dependency hold is superseded. The complete release
batch still has the recorded owned-Wire endpoint/schema/release-identity work
(#613) and paid-Blob acceptance (#494); it is not deployment-ready. No full
suite, native macOS, installed-IC or TEE qualification was run in this slice.
Changelogs describe this adoption in the existing open 0.112.0 entry; no version,
Git publication or deployment action was taken. Earlier entries below retain
historical evidence and do not override this current dependency selection.

## Latest published dependency recheck — 2026-10-23

Shared Tooling is refreshed through the canonical exporter to committed upstream
revision 35bc5a90e528ea2f60312997ea99a5969296248c. Its VERSION remains 0.26.7;
the admitted-owner lock-withdrawal fix is documented under unreleased 0.26.8.
Snapshot verification, the canonical wait/race fixture and offline admission of
the selected Testkit helper pass. Native macOS qualification was not performed.

Registry recheck: Host latest is 0.65.0, Blob/contracts latest is 0.43.0 (already
adopted), Query remains 0.68.1, and Backup 0.39.0 is now published. Backup 0.39
with Host Artifacts/FS 0.65 passes all ten Canic artifact tests, resolving the
published conversion gap in Backup #67. However Host Tools 0.64 (retained for
Query's LedgerTip) also requires Artifacts 0.64 WasmFacts. The full graph therefore
cannot yet take Backup 0.39/Artifacts/FS 0.65 without Query #101's owner alignment.
No cross-version Wasm or error adapters were added.

After qualification, the incoming compatible manifest/lock selections were
retained: Backup 0.38, Artifacts/FS/Tools 0.64.2, other adopted Tooling 0.65 and
Blob 0.43.0. Strict Host/Backup/CLI Clippy passes for that retained graph. Evidence:
`/tmp/canic-host-refresh/`. No dependency selection remains changed by this
recheck; the Shared snapshot and documentation changed. No Git publication or
release. The complete batch still needs Query alignment and the recorded Wire/
paid-Blob work; it is not ready for deployment.


## Tooling 0.65 and Blob 0.43 adoption — 2026-10-23

Shared Tooling 0.26.7 is adopted at upstream main revision
3e8d7cd3e7e9b7b79a5e800c30daf0be2710dab0 through the canonical exporter.
Explicit setup gains bounded slot waiting; offline checks stay immediate.
Build, Host Process/Text and Testkit require published 0.65.0. The Wire endpoint
acceptance test now uses Testkit's native WireCallSpec query/update helpers.

Host Artifacts/FS and Tools retain 0.64.2: published Backup 0.38.0 exposes Host
0.64 DirectoryChecksumError conversions, and Query 0.68.1 returns Host 0.64
LedgerTip. Confirmed E0277/E0308 failures are tracked in Backup #67 and Query #101.
No cross-version shims or sibling source edits were introduced. Other transitive
Host 0.64 packages remain through those owners; the retained graph compiles.

Blob/contracts 0.43.0 is adopted in all three workspaces. Its tenant native Wire
hard cut is consumed through upstream helpers/catalog; no local codec change.
Wire 0.5.22/derive are removed from all locks, selecting Wire 0.6.5 throughout.

Pass: snapshot verification; canonical bounded-wait fixture; Canic's isolated
release-adoption transaction fixture; explicit Testkit 0.65 helper setup; strict
Host/Backup/CLI, Wire acceptance-target, Blob and governed internal Clippy; all
three locked dependency/runtime-identity admissions. The owning payload PocketIC
target passes seven tests; dedicated and embedded Blob cases both pass, including
managed Wasm builds, tenant refusals, authorization and same-release recovery.
Evidence: `/tmp/canic-tooling065/`. These are Linux/PocketIC results, not native
macOS, production provider or installed-IC qualification.

Remaining work: publish owner alignment for full Host 0.65 adoption; complete
#613's role-wide Wire/schema/frontend cut and #494's accepted-payment proof.
The complete release batch is not ready. Both changelogs are updated; no commits,
pushes, releases or deployments were performed. Shared snapshot copies are
producer-owned and were not edited by hand.


## #613 guarded native Wire adapter qualified — 2026-10-23

Typed Canic endpoint macros accept explicit wire(id, request, response) alongside
on_access_denied="reject". The existing raw adapter owns executor/guard/metrics/
reply flow; Wire owns bounded frame bytes. Request size is checked before copy,
unit requests also decode, and response failures trap within the handler's IC
transaction. Selected endpoints emit raw export declarations and no Candid
registration. One request value or unit is supported; multi-argument handlers
must use an explicit request type. Lifecycle encoding is unchanged.

The installed payload probe uses native Principal query and guarded synchronous
update. All seven tests in the owning governed PocketIC target pass, including
wrong-method/direction/trailing/oversized refusals, access rejection, reply-budget
rollback and valid calls afterward. Strict macro/probe and acceptance-test Clippy
pass; 49 macro tests, formatting and all three dependency boundaries pass. Both
independent Blob consumer locks now include the facade runtime Wire dependency.
Evidence: `/tmp/canic-wire-endpoints/`.
The current selected Wire is 0.6.5; no broader dependency upgrade was requested.
These are Linux native/managed Wasm/PocketIC results, not macOS or installed IC.

No production role endpoint or caller has yet changed codec. The endpoint-derived
schema registry (including duplicate IDs), schema/release hashes, full role/client
cut and frontend artifact activation remain unfinished. #613 stays open and the
complete release batch is not ready. Both changelogs document this slice; no Git
publication, release or sibling source edits.


## Platform 0.6.9 native Wire metadata adopted — 2026-10-23

Framework and both independent Blob consumers require published Platform 0.6.9.
Contracts' wire-schema feature now selects Platform's host-only native Principal
metadata. The existing conformance generator covers Page<Principal>, with exact
Rust/TypeScript frames for empty, anonymous and 29-byte identities and rejection
of 30-byte identities. Platform #48's missing API no longer blocks Canic.

Pass: three pagination/native-Principal tests; strict Contracts library/test/
example Clippy; Rust 1.91 example check; Contracts Wasm check with wire-schema;
all three dependency-boundary checks; strict TS 7.0.2 and Node 24.21.0 conformance.
Normal Contracts Wasm dependencies exclude Wire metadata and serde_json.
Evidence: `/tmp/canic-platform069/`. TS compiler remains read-only local sibling
qualification, not an admitted release tool. No native macOS or installed-IC
qualification. Cargo normalized existing Windows dependency edges while resolving
metadata; no package-wide upgrade was requested.

#613 remains open: owned endpoint/caller conversion, generated method registry,
Wire release identities and frontend artifact activation are not delivered.
The complete release batch is not ready; changelogs reflect this qualified slice.
No commit, push, deployment, release or sibling source edit.


## Internal PocketIC feature simplification — 2026-10-23

Removed the default-enabled `pocketic-fixtures` switch. Reusable internal fixture
support is now unconditional for library consumers; the existing
`governed-pocketic-tests` boundary still excludes the stateful catalogue from
ordinary unit-test builds. No fixture or test case was deleted.

Pass: strict package-scoped Clippy for library/tests with no default features,
both without and with `governed-pocketic-tests`; package formatting and diff
checks. Evidence: `/tmp/canic-remove-fixture-feature/`. No PocketIC runtime run
was needed for this cfg-only simplification. README and both open changelogs
updated. The complete release remains blocked by the recorded Wire/Blob work;
this slice does not change those blockers. No Git publication or release.


## #557 bootstrap manifest timestamps fixed locally — 2026-10-23

Store preparation stamps zero creation/approval inputs with the supplied IC
seconds before projected-size checks and writes. Exact retries use the retained
times; changed content/binding still refuses without writes. Host completion now
accepts positive Store stamps for zero-time requests while retaining exact checks
for other fields and explicit nonzero times. The installed fixture shares Host's
checker rather than duplicating its previous zero-time comparison.

Pass: 11 focused Store tests, 26 Host current-protocol tests (one existing ignored),
strict Control Plane and internal Clippy, and the owning installed Store bootstrap/
catalog/zero-effect replay case. The latter asserts nonzero equal created/approved
times no later than publication. Evidence: `/tmp/canic-557/`, final runtime log
`pocketic-qualified.log`. Initial missing Testkit helper, non-convergence, private
module wiring and lint failures remain recorded. Testkit 0.64.2 helper setup used
the prescribed installer; no dependency selection was changed by this fix.

These are native/PocketIC checks, not a live sample Fleet or a universal two-second
publication timing guarantee. #557 remains open pending default-branch delivery.
Changelogs are updated; complete release remains blocked by the recorded Wire/Blob
integration work. No commit, push, release, deployment or sibling edit.


## #613 native Principal boundary correction — 2026-10-23

Owned Wire DTOs must use ic_platform::Principal; ExternalPrincipal is only for
required external Candid contracts. Earlier blocker reports used the wrong type.
Published Wire 0.6.4 resolves the Serde deny_unknown_fields/serde_bytes gaps.

Isolated native-Principal tests on Platform 0.6.8/Wire 0.6.4 pass exact byte-buffer
frame equality and round trips for empty, anonymous and 29-byte principals, plus
30-byte rejection. Metadata derivation still fails Principal: Wire. Platform #48
owns the missing host-only integration; Wire #17 and Canic #613 are corrected.
Evidence: `/tmp/canic-native-principal/{runtime,schema}.log`. This is native codec
qualification, not endpoint or generated-principal-client acceptance.

No production DTO imports changed independently of their codecs/callers. The
coordinated endpoint/client/artifact cut remains incomplete; storage and external
Candid contracts remain unchanged. Complete release is still blocked; changelog
and migration instructions are updated. No commit, push, release or sibling edit.


## #613 network scalar and generated-client conformance — 2026-10-22

Adopted published Wire/derive 0.6.2 without changing unrelated selections. Added
host-only metadata for actual CanonicalNetworkId text serialization and its paired
TS validator. The Contracts wire_conformance example uses actual pagination DTOs,
Wire's generated client/runtime and Rust frame vectors; it defines no endpoint.

Five network tests, strict Contracts lib/test/example Clippy, actual Rust 1.91
example compilation, Wasm compilation, dependency boundaries and graph exclusion
of Wire metadata/serde_json pass. Strict TypeScript 7.0.2 (ES2022/declarations) and
Node 24.21.0 pass exact frames, invalid-domain/missing-validator and frame refusals.
Evidence: `/tmp/canic-613-next/`. The TS compiler was used read-only from sibling
tooling; this is local qualification, not release-tool, browser or installed-IC
admission. The initial Node comparison was corrected for null-prototype records.

#613 remains open. Published Wire cannot derive Platform ExternalPrincipal fields
and rejects existing serde(deny_unknown_fields); byte-field projection also needs
supplier support. Minimal published-version evidence and acceptance are on Wire
#17, comment 6348937524. No identity wrappers, duplicate DTO catalog or weakened
serialization were introduced. Endpoint registration/codec conversion, schema
release identities, governed TS compiler adoption, frontend replacement and
installed acceptance remain. Complete batch is not ready to push or publish;
changelogs are updated. No commit, push, release, deployment or sibling edit.


## Blob held-funding concurrency qualified — 2026-10-22

The embedded managed proof holds the first provider callback, invokes concurrent
funding ingress twice and verifies that neither the original dispatch nor a new
reservation proceeds. The exact held request and mounted funding summary remain
unchanged. Releasing malformed bytes refunds the whole first offer; exhausted
provider slots reject the second offer with a whole refund. Their distinct typed
outcomes remain observable with zero accepted cycles before and after restoration.
No production funding policy, provider evidence, scheduler or journal was added.

Strict embedded-consumer and internal Clippy pass. The installed managed case
passes in `/tmp/canic-blob-concurrency/embedded-retry.log`. The first build was
correctly rejected after Cargo.lock changed from Wire 0.6.0 to 0.6.1 during the
build; that incoming selection was preserved. No mixed-input result is admitted.
This is PocketIC/framework evidence, not native macOS or real-provider acceptance.

Published Blob remains 0.42.0 and Testkit 0.64.1. Tooling #235's bounded
cycle-acceptance fixture is implemented upstream but not published. Accepted
payments/partial refunds, accepted malformed/error/lost callbacks, positive credit
and budget renewal remain unqualified under #494. Real-provider payer linkage is
application/deployment evidence. #494 stays open for these gaps and default-branch
delivery; the complete batch is not ready to push or publish. Changelogs are updated.
No commit, push, release, deployment or sibling edit.


## Published Wire 0.6.0 adopted for native metadata — 2026-10-22

Contracts now owns optional host-only `wire-schema` metadata on actual Page<T>
and PageRequest DTOs. The published metadata-only derive removes the earlier
missing-native-derive blocker; existing endpoint codecs and storage are unchanged.
Blob 0.42.0 still requires Wire 0.5.22 and must migrate at its upstream owner.
The two codec versions are not bridged or substituted. Independent consumer
lockfiles remain valid without changes because the metadata dependency is optional.

Two DTO schema/frame tests, strict Contracts library/test Clippy, native Rust 1.91
MSRV check, Wasm check with wire-schema selected and the dependency-boundary gate
pass. The normal Contracts Wasm graph excludes ic-wire and serde_json. Evidence:
`/tmp/canic-wire060/`. Only Wire 0.6.0/derive entries were added to the incoming
root lockfile; unrelated package selections and pending fixes were preserved.

This is package/DTO adoption, not completion of #613. Custom domain-scalar schema
projections, complete endpoint registration/conversion, release schema identities,
generated browser clients and installed acceptance remain. No generated-client
cross-language, native macOS, or live IC qualification is claimed. Changelog
surfaces are updated, but the complete release batch remains blocked. No commit,
push, publication or deployment.


## #435 Pool funding Root authority fixed locally — 2026-10-22

Pool funding now uses the configured parent Root's live observed principal,
independent of recovery-controller ordering. Missing parent names or observations
refuse planning with a typed topology error. Bootstrap and startup-continuation
callers pass their existing observation through the shared funding helper.
Non-pool funding keeps no pool authority; no paid-effect or journal format changed.

All 91 Host policy tests and strict Host library/test Clippy pass against the
locked graph. Targeted formatting and whitespace checks pass. Evidence:
`/tmp/canic-435/`. These are native policy checks, not installed-IC qualification.
Root and detailed changelogs are updated; #435 stays open pending default-branch
delivery. Existing Wire/Blob work and release blockers remain unchanged. The
complete batch is not ready to push/publish. No commit, push, release or deployment.


## #416 Store startup-fence bypass fixed locally — 2026-10-22

Removed the separate privileged Store preflight selected by Rust function name.
Typed and raw endpoints share the canonical runtime activation workflow. Its
policy requires the actual Store role, exact data export and Update kind before
admitting data operations while Prepared. Ordinary application methods with the
same names retain activation and fixture-readiness fences; caller authorization,
payload limits, authority restore and standalone-local semantics are unchanged.

All 13 endpoint-expansion tests, 47 Core activation tests, strict Core/Macros
library/test Clippy and the installed Store bootstrap/catalog/zero-effect-replay
PocketIC case pass. Evidence: `/tmp/canic-416/`. The locked current graph selects
Platform 0.6.8; the first offline Core run refused its uncached macros package,
and the locked online retry downloaded it without changing selections. Native
macOS and live IC were not run. Removed-path inventory and whitespace checks pass.

Root and detailed 0.112 changelogs are updated. #416 remains open for default-branch
delivery. Existing Wire/Blob work is preserved; complete release readiness remains
blocked by their recorded integration gaps. No commit, push, release or deployment.


## #613 Wire hard cut accepted — 2026-10-22

The maintainer explicitly selected the coordinated Wire migration now, superseding
0.112's earlier owned-Candid exception. AGENTS.md and the
[migration contract](../../design/0.112-standalone-blob-service-extraction/wire-hard-cut.md)
record that scope. This does not authorize a next minor, Git publication or release.

Registry IC Wire 0.5.22 provides native Serde frames and TS registry generation,
but no native-Serde type metadata derivation. The selected route retains existing
Rust shapes and storage encodings, with explicit method budgets. A registry-only
scratch reproducer rejects generic Page<T> with the finite Wire derive. Supplier
[Wire #17](https://github.com/dragginzgame/ic-wire/issues/17) owns the required
metadata projection; evidence is retained under `/tmp/canic-613/`. No handwritten
schema roster or DID-to-Wire conversion was introduced.
A separate scratch native-codec probe passes generic pagination roundtrip and
wrong-method, wrong-direction, oversized and trailing-frame refusals. This is
native supplier evidence, not generated-client or installed-endpoint qualification.

Production endpoint/client codecs, release records and frontend generation remain
unchanged. #613 is not fixed: canonical metadata production, coordinated endpoint/
caller conversion, release/artifact identity propagation, generated TS/JS clients
and installed acceptance remain pending. Build #225 follows that concrete producer.
The complete current batch is not ready for release. No broad gate or deployment.


## Blob funding evidence isolation — 2026-10-22

The embedded managed fixture now exercises each independent dispatch refusal
through the installed opt-in Canic API: missing spendability/recovery/activity,
unqualified provider, fenced recovery, outstanding/uncertain activity, insufficient
funds and independently missing liability holds. Every intent field is separately
substituted in the trusted callback evidence and refused with typed Binding.
Both original reservations subsequently settle with exact refunds and unchanged
retained-operation totals, then pass the existing restoration and inspection proof.
No production API, dependency selection or shared funding semantics changed.

Independent embedded-consumer strict Clippy and its targeted managed PocketIC
case pass; evidence is in `/tmp/canic-blob-evidence/{clippy,embedded}.log`.
The managed proof uses the development compiler, not native macOS or live IC.
Published Testkit is still 0.64.1 and Tooling #235 remains open without a bounded
cycle-acceptance fixture. Accepted payments, positive credit/renewal and uncertain
paid callbacks remain unqualified under #494. No 100% Blob or complete-release
readiness claim; current changelog surfaces are updated. No commit, push or deploy.


## Published Host/Build/Testkit 0.64.1 and #622 callback races — 2026-10-22

Adopted registry Host/Build/Testkit 0.64.1 in the root catalog and lock. Artifacts
was already locked at 0.64.1; its declared minimum is now aligned. Six packages
advanced from 0.64.0; native held-reply support adds wat/wast. Both independent
Blob consumer locks contain none of these host packages and remain unchanged.
The governed 0.64.1 Testkit server helper was installed through the scoped shared
installer after the first targeted run refused its missing tool-cache source.
No PocketIC version change or live provider request was made.

Native `held-reply-fixtures` is enabled only for governed PocketIC tests. Both
managed Blob shapes now prove that cancellation/revocation commits while the
original gateway reply remains held, then release it and assert Conflict with
unchanged committed membership, sequence and pending state. Cancellation's stale
reply proposes a different gateway; revocation's proposes the removed gateway.
A separately authorized fresh refresh succeeds afterward. No timing assumptions,
local fixture generator or Blob policy implementation was added.

Strict internal Clippy, Rust 1.91 native lib/test compilation, all three locked
dependency admissions and dedicated/embedded managed cases pass. The adapter's
normal Wasm graph excludes Testkit, Host and wat/wast. Managed artifacts use the
configured development compiler; no native macOS or installed-IC qualification
is claimed. Evidence: `/tmp/canic-host0641/`, with the initial setup refusal kept
separately in `dedicated.log` and passing retry in `dedicated-final.log`.

#622's held-callback gap is locally qualified; default-branch delivery remains.
#494/#235 accepted-payment evidence is separate and still incomplete. Complete
0.112 release readiness is not established. Changelogs and coverage notes are
updated; no commit, push, release, deployment or sibling edit.


## #622 populated funding inspection — 2026-10-22

The embedded installed fixture's two real rejected/full-refund operations now
qualify descending funding history, exclusive cursor boundaries, exact retained
outcomes, changed amount/target conflicts, caller refusal, unknown-operation
absence and repeated observation. Queries preserve stable memory. The same
checks run before restoration, while fenced and after continuity recovery.
This proves populated refunded records, not accepted/credited payment outcomes.

Strict internal Clippy, the targeted embedded managed PocketIC case and focused
format/diff checks pass; `/tmp/canic-622-populated/{clippy,embedded}.log`.
No production, dependency or deployed fixture source changed in this slice.
The dedicated fixture still qualifies empty funding views; populated records
are exercised in the application that opts into the funding API.

Published Testkit remains 0.64.0. #234's held-reply support is implemented locally
upstream but unpublished, so cancellation/revocation-before-stale-callback tests
remain open. #235 and #494 own accepted-payment qualification. #622 remains open
for delayed callbacks and default-branch delivery. No full-release readiness,
live-provider/native macOS qualification, commit, push or deployment is claimed.


## Opt-in Blob funding composition — 2026-10-22

The maintainer approved application-owned trusted evidence for #494. The new
`canic_blob_service::funding` API binds shared preparation, dispatch, credit and
renewal workflows to the installed journal and actual IC caller/service. Every
journal access observes platform version; no journal borrow crosses dispatch
awaits. Evidence acquisition occurs only after authentication. No new journal,
endpoint, scheduler, evidence source or independent funding policy was added.

Strict adapter/internal and independent embedded-consumer Clippy pass. The
embedded managed PocketIC case passes missing evidence, caller/scope refusal,
two distinct replica-rejected fully refunded dispatches, exact refund/zero
acceptance, repeat-attempt refusal, credit/renewal refusal without acceptance,
uncollected ingress reply inspection and same-release restoration/recovery.
The dedicated managed regression also passes. Rust 1.91 adapter-library Wasm
and embedded-consumer native checks pass, as do all three locked dependency
admissions and focused formatting/diff checks. Logs: `/tmp/canic-blob-funding/`.
The incoming root lock selected Wire 0.5.22; it was fetched without changing the
selection. The embedded consumer adds its already-resolved Blob package as a
direct dependency to use upstream funding types; no sibling files were edited.

[Tooling #235](https://github.com/dragginzgame/ic-tooling/issues/235) tracks bounded
cycle acceptance in provider fixtures. Successful acknowledgments, accepted
errors, uncertain paid callbacks and positive credit/renewal are not qualified
by fully refunded calls. #234's held-reply work exists locally upstream but is
not the selected published Testkit. #494/#622 remain open; full Blob integration
and complete release readiness are not established. No native macOS or live
provider qualification, commit, push, release, payment outside PocketIC or deployment.


## Blob acknowledgment rejection atomicity — 2026-10-22

Both managed compositions now reject malformed acknowledgment batches with an
eligible leading root and invalid later roots, over-count batches and over-byte
requests while retaining the eligible deletion entry and non-live status. A
maximum-size valid repeated-root batch succeeds; replay and physical/liability
accounting remain unchanged. No production API or dependency changes.

Strict internal Clippy and both targeted managed PocketIC cases pass, with
focused formatting and diff checks. Evidence: `/tmp/canic-blob-gc-atomic/`.
No full-suite, live-provider or native macOS qualification is claimed.

IC Tooling #234 is still open without a held-reply implementation; #622 retains
the delayed callback race gap. #494 still awaits the explicit funding evidence
owner decision. Complete Blob integration/release readiness is not established.
No commit, push, payment or deployment.


## Blob GC bounds and remaining race/funding gaps — 2026-10-22

Both managed compositions now qualify the mounted external GC liveness endpoint
at the shared 10,000-root bound, reject over-count and over-byte requests, and
refuse gateway listing/acknowledgment after explicit revocation while retaining
public conservative liveness. Strict internal Clippy and both targeted managed
PocketIC cases pass: `/tmp/canic-blob-races/{clippy-gc,dedicated-gc,embedded-gc}.log`.
No production API or dependency changes in this slice.

The attempted delayed gateway callback test did not establish cancellation-first
ordering: immediate provider replies completed during Canic's asynchronous
continuity admission and cancellation correctly returned `Conflict`. Both failed
experiments are retained in `/tmp/canic-blob-races/{dedicated,embedded}.log`; the
timing-dependent test was removed. This is not evidence of a Blob defect.
[IC Tooling #234](https://github.com/dragginzgame/ic-tooling/issues/234) requests a
bounded held-response fixture so [#622](https://github.com/dragginzgame/canic/issues/622)
can prove cancellation/revocation before the original callback.

[#494](https://github.com/dragginzgame/canic/issues/494) remains pending a decision
between an opt-in application-supplied trusted-evidence host API and inspection-only
until a concrete evidence source exists. Shared funding dispatch/journal APIs
are available; provider qualification, payer linkage, complete account activity
and other liabilities cannot be inferred from local journal/balance alone.
No paid path, fabricated evidence or automatic schedule was introduced.

No 100% integration or complete 0.112 readiness claim; delayed races, guarded
funding and real-provider acceptance remain unqualified. Native macOS and live
provider execution were not performed. No commit, push, release or deployment.


## Published Blob 0.42 adoption and mounted GC — 2026-10-22

Root and both independent consumer locks select published Blob/core contracts
0.42.0; only those two packages changed in each adoption update. Catalog provider
query/update emitters delegate bounded Candid GC decoding and policy to Blob,
with Canic raw endpoint admission/observability and native Candid declarations.
The owned Wire/schema hard cut requires reinstall; same-release restore remains.

The actual consumer build exposed grouped macro-forwarded export-name literals;
Canic's parser now unwraps invisible groups while rejecting nonliteral expressions.
The focused parser suite, final strict macro/adapter/internal Clippy, both isolated
consumer Clippy checks, all three dependency admissions and dedicated/embedded
managed PocketIC cases pass. GC tests cover live/unknown roots, gateway authority,
malformed roots, all-or-nothing batch refusal, release/listing, acknowledgment
replay and restore fences. Acknowledgment preserves physical/billing liabilities.
Logs: `/tmp/canic-blob042/`; original failed macro build is retained separately.

Rust 1.91 adapter-library Wasm and both consumer native checks pass. Direct
consumer MSRV Wasm checks correctly refused the unsupported Cargo route; managed
consumer Wasm/runtime proof uses the configured development compiler, not an MSRV
claim. Native macOS and real-provider evidence remain unavailable. Normal adapter
Wasm graph excludes Testkit, fixture encoding and host parsing dependencies.

#622 stays open for delayed gateway callback races and default-branch delivery.
#494 is updated: 0.42 provider acknowledgment is a supported distinct success
profile, not independent credit proof; guarded Canic paid-dispatch composition
still remains. Provider deletion, billing cessation and payer linkage need their
own evidence. No 100% or full 0.112 readiness claim, commit, push or deployment.

## Blob #622 service-wide pagination — 2026-10-22

Operator history now traverses multiple pages and matches the complete union of
two independently authorized tenant histories, including full entry identities
and lifecycle states. Duplicate identities/repeated cursors refuse the test;
page limits and cursor service/namespace/scope/filter are checked. Tenant callers
cannot read service pages or reuse service cursors under tenant scope. The
surrounding query batch preserves stable memory.

Strict Clippy, formatting and both dedicated/embedded PocketIC cases pass:
`/tmp/canic-blob-service-pages/{clippy,dedicated,embedded}.log`. No production or
dependency change. #622 remains open for delayed gateway callback races and
default-branch delivery; populated paid funding and live-provider acceptance
remain separate. Complete 0.112 readiness and native macOS/MSRV qualification
are not established. Changelogs updated; no commit, push or deployment.

## Blob #622 populated tenant pagination — 2026-10-22

Both managed compositions now admit 65 extra uploads through the mounted endpoint
and qualify tenant history across the 32-result and 64-scan limits. Ordered
continuation recovers every expected ID without duplicates; empty filtered pages
still advance. Changed tenant/filter cursors refuse and queries preserve stable
memory. Only fixture installation capacity grows; exact reservation accounting
and production page limits remain checked.

Final strict Clippy and both dedicated/embedded PocketIC cases pass:
`/tmp/canic-blob-pagination/{clippy,dedicated,embedded}.log`. No production or
dependency change. #622 remains open: delayed callback races, service-wide
cross-tenant pagination and default-branch delivery are not established here.
Populated paid-funding history/live-provider acceptance remains separate. No
native macOS/MSRV or full 0.112 readiness claim. No commit, push or deployment.

## Blob #622 payment-relationship observations — 2026-10-22

The mounted PaymentRelationship branch now passes both managed compositions:
exact service/payer bindings, operator/scope refusals, negative signed figures,
arbitrary-width counters, absent relationship, all structured provider errors,
malformed bytes and oversized replies. Responses echo the original request and
leave local status/funding unchanged. No second provider engine or production
change; Testkit's existing fixture adds the published relationship method.

Strict Clippy and both exact dedicated/embedded PocketIC cases pass:
`/tmp/canic-blob-relationship/{clippy,dedicated,embedded}.log`. #622 remains open
for delayed callback races and populated pagination; paid funding/live-provider
acceptance remains separate. No exhaustive branch coverage, native macOS/MSRV
or complete 0.112 readiness claim. Docs/changelogs updated; no Git publication.

## Blob #622 provider reply bounds — 2026-10-22

Added mounted account/gateway responses one byte above their configured limits.
Both return typed ReplyTooLarge; account status remains unchanged and gateway
sync retains its original attempt, refuses redispatch, and requires exact
operator cancellation. Strict Clippy and both managed dedicated/embedded cases
pass: `/tmp/canic-blob-bounds/{clippy,dedicated,embedded}.log`.

No production or dependency changes. #622 remains open for payment-relationship
responses, delayed callback races and populated pagination. Live-provider and
paid-funding evidence remains separate. Updated the guide's stale missing-method
claim: all methods have direct calls, without exhaustive behavioral coverage.
Complete 0.112 release readiness is not established; no Git publication.

## Blob #622 controlled provider slice — 2026-10-22

All current catalog methods now have direct mounted test calls. Added gateway
sync/cancel/revoke and account-balance inspection using published Testkit 0.64's
native `reply-fixtures`. Covers successful membership, actual operator/scope
refusals, malformed-reply pending identity, exact cancellation, discarded reply
recovery through the original sequence, revocation and restoration fences.
Account observations do not credit the funding journal. No provider payment ran.

Strict Clippy, both final managed dedicated/embedded cases and all three locked
dependency admissions pass: `/tmp/canic-blob622-provider/`. Test-only fixture
encoding adds wasm-encoder/leb128fmt to the root lock without upgrading existing
versions; the deployed normal Wasm graph excludes both and Testkit.

Keep #622 open: method invocation coverage is not exhaustive behavioral coverage.
Next: delayed callback versus cancellation/revocation, populated pagination,
payment-relationship responses and additional provider refusal/bound cases.
Populated paid funding and live-provider acceptance retain their separate owners.
Changelogs updated; no commit, push, deployment or sibling edit. Full 0.112
readiness, native macOS/MSRV and live-provider qualification are not established.

## Blob #622 operator inspection slice — 2026-10-22

Added direct mounted local status, funding history, outcome and preparation
assessment checks. Strict governed internal Clippy and both exact managed
dedicated/embedded cases pass on Blob/contracts 0.41.1; logs:
`/tmp/canic-blob622-operator/{clippy,dedicated,embedded}.log`. Checks cover operator
authority, independent service/namespace/Cashier/payer binding, empty history,
absent outcomes, cursor/intent refusals, allocation-reserve and provider-evidence
blockers, restored fencing, and unchanged stable memory/local journal observations.

Four methods still lack direct invocation: gateway sync/cancel/revoke and account
inspection. Testkit 0.64 publishes opt-in `reply-fixtures` raw-reply canisters for
controlled provider responses; that feature is not yet enabled or qualified here.
Populated funding/paid reconciliation and complete pagination remain separate
acceptance gaps; ten added method invocations do not establish exhaustive coverage.
Keep #622 open. No runtime/dependency change, payment, sibling edit or publication.
Changelogs reflect this slice; complete 0.112 push readiness is not established.

## Blob #622 inspection coverage slice — 2026-10-22

Added direct mounted coverage for six methods: upload admission, manifest, content
lookup, reference capacity, upload history and reference receipt. Both exact
managed dedicated/embedded cases and strict governed internal Clippy pass on
Blob/contracts 0.41.1. Evidence: `/tmp/canic-blob622/`. Original permissions,
declarations and receipts remain observable through same-release restoration;
changed authority/receipt inputs and foreign observers refuse. Absent results
are distinct from denied requests; queries preserve stable state.

Eight methods still lack direct coverage: local status, funding history/outcome/
assessment, account inspection and gateway sync/cancel/revoke. #622 stays open;
the six added methods do not establish exhaustive branch or pagination coverage.
Upstream provider issues remain maintainer-owned. No production behavior, version,
dependency, Git publication or paid effect changed. Changelogs updated for this
slice; complete 0.112 push readiness remains unestablished by the partial audit.

## Blob ownership and coverage audit — 2026-10-22

Both exact managed dedicated/embedded cases pass again on published Blob/contracts
0.41.1: `/tmp/canic-blob-audit/{dedicated,embedded}.log`. The audit found no second
Blob engine or retired verifier flow in active Canic Rust. However, the earlier
framework-completion statement was too broad: 14 mounted gateway/account and
inspection methods lack direct Canic test invocations. New Canic #622 owns that
coverage, distinct from the passing #620/#621 journey and metrics subset.

See [the audit](../../audits/working/blob-0411-adoption.md) for owners, exact methods,
artifacts and evidence limits. Updated #620's retired verifier requirements and
commented on canonical upstream GC issue #32. Provider #32/#34 and application
#31/#5/#4 remain separate gaps; no new upstream algorithm defect was established.
No issue closure, commit, publication, paid effect or deployment. Full coverage
and complete 0.112 readiness are not claimed; native macOS/MSRV were not rerun.

## Blob 0.41.1 patch adoption — 2026-10-22

Verified published Blob/contracts 0.41.1. The root lock already selected this
pair; raised catalog minimums and aligned both independent consumer locks without
changing other selections. Upstream changes no Rust source or adapter API.
Strict adapter and both consumer Clippy checks and all three locked dependency
admissions pass. Evidence: `/tmp/canic-blob0411/`. The preceding 0.41.0 managed
PocketIC evidence remains historical; those cases were not rerun for this patch.
No new macOS, MSRV or live provider qualification. Provider acceptance gaps and
pending default-branch delivery remain; this patch does not make the complete
0.112 batch ready to publish. Changelog surfaces are updated; no Git publication.


## Blob 0.41 / Testkit 0.64 framework acceptance — 2026-10-22

Adopted published Blob/contracts 0.41.0 and Testkit 0.64.0, preserving incoming
manifest selections and unrelated work. Removed verifier installation fields and
attestation fixtures. Certificate issuance now registers retention intent and the
first reference through Blob's shared engine; it does not prove provider bytes.
Replaced Testkit's removed PocketIC forwards with Platform imports/features and
Fake with fake_principal. No compatibility lane or storage migration was added.

Passing: strict all-target/all-feature Clippy for canic, canic-host, canic-tests,
canic-testing-internal and canic-blob-service; isolated internal no-default-feature
Clippy; both independent Blob consumer Clippy checks; two exact installation
codec regressions; both managed dedicated/embedded PocketIC cases; dependency
admission for all three workspaces and targeted formatting. The managed cases
build actual Wasm and qualify endpoint parity, authority, registration, lost
certificate replies, original first-reference recovery, replay/release, fences,
metrics and embedded application state. Evidence: `/tmp/canic-blob041/`.

#620/#621 are locally qualified, not closed before default-branch delivery.
Blob's obsolete verifier requirement #33 is retired by the published contract;
provider GC/deletion/billing #32 and real application/provider acceptance #31
remain independent. Linux local framework acceptance is complete for this scope;
end-to-end Blob is not claimed 100%. Native macOS, MSRV and installed-IC/provider
qualification were not rerun. No commit, push, release, deployment or paid effect.
The complete 0.112 batch retains its other product/provider acceptance gaps.


## Coordinated Backup/Query/Host adoption qualified — 2026-10-22

Verified registry releases and adopted Backup 0.38.0, Query 0.68.0 and
Build/Host Artifacts/FS/Process/Text/Tools 0.64.0 together. Query #100 and Backup
#66 no longer block this selected graph: their published public Host identities
align, with no source adapters, aliases or sibling overrides. Preserve the
incoming Auth/Timers and unrelated lock selections. Testkit remains 0.63.3 and
retains transitive Host Artifacts/FS/Process 0.63.3; its API migration is separate.

Passing: all three locked dependency admissions; strict all-target/all-feature
Clippy for canic-host, canic-backup and canic-cli; all 10 Backup artifact tests;
all 15 CLI restore-runner tests. Evidence: `/tmp/canic-query068/`. Checksum typed
errors, no-follow/staging behavior and runner receipts/retries remain covered.
No full workspace, managed deployment, MSRV or native macOS qualification ran.
Changelogs updated; pending #589 and other local work preserved. No commit,
push, release or deployment. These dependency blockers are resolved; existing
product/provider acceptance gaps still prevent claiming complete release readiness.


## Backup 0.38 publication verified; Query prerequisite pending — 2026-10-22

Published Backup 0.38.0 resolves its Host 0.64 dependency alignment (#66).
Verified crates.io API and sparse index: Backup requires Host Artifacts/FS/Process
^0.64; Query remains published at 0.67.6. Query's sibling 0.68.0 changes are
prepared source, not a registry dependency. Query #100 remains the prerequisite
for combined Backup/Host adoption because its public LedgerTip and Host Tools'
WasmFacts boundary must share the selected Host identity.

Keep the compilable incoming Backup 0.37.4 and Host/Build 0.63.3 selections until
Query 0.68 is published. No alias, duplicate record conversion, local patch or
unpublished dependency was added. Once published, adopt Backup 0.38, Query 0.68
and Host/Build 0.64 together; run Backup artifact/runner regressions and scoped
Host/Backup strict Clippy. No compilation was repeated for this registry-only
check; the preceding failed adoption logs already establish the type conflict.


## Shared Tooling adopted; Host 0.64 held — 2026-10-22

Shared Tooling 0.26.6 is adopted at committed upstream revision
`4cb9bc907154225a3f7e4c33fd8d75fbeaa125c8`, verified against remote HEAD.
Canonical export includes 133 files and the required CI/release integrity audit.
Snapshot/governance checks, changed production-shell ShellCheck, all three locked
dependency admissions and diff whitespace checks pass. Clippy remains first in
release validation. Upstream exporter/regression suites were not duplicated here.

Published Host/Build 0.64 was attempted, then restored to the incoming 0.63.3
selection: Query 0.67.6 returns Host Tools 0.63 LedgerTip; retaining old Tools
alongside new Artifacts also fails its WasmFacts boundary. Backup 0.37.4's public
artifact-error conversion likewise accepts only Host Artifacts 0.63 errors.
Trackers: https://github.com/dragginzgame/ic-query/issues/100 and
https://github.com/dragginzgame/ic-backup/issues/66. Temporary conversion code was
removed; no compatibility bridge, Cargo override or sibling edit remains.
Evidence: `/tmp/canic-host-shared-oct22/`. Failed Clippy attempts are retained as
blocker evidence, not passing qualification. Prior pending source/lock work is
preserved. No commit, push, release or deployment; complete batch readiness is
still blocked by its existing acceptance gaps and this dependency alignment.


## Published IC dependency alignment — 2026-10-22

Paused #589 implementation at the maintainer's request; its pending changes remain.
Verified published IC versions through crates.io and raised catalog minimums to
Auth/protocol 0.14.3, Backup 0.37.4, Build/Host/Testkit 0.63.3, Memory/Storage
0.50.10, Metrics 0.15.5, Platform 0.6.7, Query 0.67.6 and Timers 0.30.2.
Blob/contracts remain 0.40.4. The incoming root lock already selected these;
preserved it and aligned both independent Blob locks, including Wire/derive 0.5.20.
Shared Tooling snapshot and unrelated dependencies were not updated in this batch.

Passing: all three locked dependency admissions/boundaries and runtime identity
checks; native strict library Clippy for Core, Host, Backup, Blob adapter and both
Blob consumers; Core Wasm check with auth-local-application-authorization enabled.
Evidence: `/tmp/canic-deps-oct22/`. A direct consumer Wasm check was rejected by
the expected managed-build guard, before artifact qualification; no bypass added.
Managed consumer builds/install/recovery, MSRV and native macOS were not rerun.
No new production readiness, TEE or clock-skew qualification is claimed. Existing
release acceptance gaps remain; changelogs are updated, with no Git publication.


## Replica shutdown verification — 2026-10-22

#589's false-success defect is locally fixed: successful ICP stop and the
already-stopped fallback both check the selected IPv4 loopback gateway. An open
TCP listener refuses completion even without healthy HTTP; only connection
refusal establishes absence. No process is terminated or descriptor deleted.
All 17 focused replica tests pass with local socket access; strict CLI lib/tests
Clippy passes. Evidence: `/tmp/canic-589/{tests-local,clippy-final}.log`.
The sandboxed test run failed at socket bind, not a shutdown assertion.

Keep #589 open: safe orphan ownership/reclamation, PID diagnostics and selected
stale-descriptor cleanup remain unfinished, as does native macOS qualification.
The inspected ICP wrapper does not expose retained orphan process custody; port
reachability or executable names cannot authorize termination. Both changelogs
and CLI documentation are updated. No commit, push or release was performed.
The complete 0.112 batch retains its existing integration/provider acceptance
blockers; this slice does not establish complete push/publish readiness.



## Coordinator Health role — 2026-10-22

#534 is locally fixed. Public Health now uses the existing dedicated Coordinator
runtime-kind marker rather than requiring an App environment role. Non-Coordinator
lookup and unavailable-role semantics remain unchanged; no authority is populated
or widened for reporting. The canonical Coordinator PocketIC case passes real
Health/Overview identity checks after installation and same-release snapshot
restoration, alongside its registry replay and restored mutation-fence checks.
This is snapshot evidence, not a new Coordinator upgrade contract or installed-IC
qualification. Strict feature-enabled internal Clippy and formatting pass.
Evidence: `/tmp/canic-issue534/{pocketic,clippy}.log`.

Both changelogs are updated. Keep #534 open pending delivery; #536's separate
standards work is preserved and its existing endpoint assertion also passed.
No dependencies, sibling files, Git publication, deployment or broad gates changed.
The complete 0.112 batch still has its prior integration/provider acceptance gaps;
this slice alone does not establish push/publish readiness.


## Replica log readability and issue triage — 2026-10-22

#554 is locally corrected: runtime replica rendering no longer generates ANSI
color escapes, while stable append and event selection remain unchanged. The
all-severity/optional-topic native regression and scoped Core lib/tests strict
Clippy pass. Evidence: `/tmp/canic-issue554/{test,clippy}.log`. Installed ICP output
has not been requalified. Both changelogs are updated; keep the issue open until
default-branch delivery and installed acceptance.

#552 and #555 are already fixed in local commit `de4f36d4e`; their trackers now
record that evidence. Remote main remains `4caeec25db7d81f6121db1c95ba7a2d91d319c60`
and still contains both defects, so neither was closed. #566 already records its
local fix. No Git publication, dependency changes or broad validation ran.
This focused slice does not establish complete 0.112 push/publish readiness;
Auth/Backup integration and Blob provider acceptance retain their existing gates.


## Blob mounted recovery acceptance — 2026-10-21

#620's Canic-owned mounted acceptance is locally qualified, alongside #621's
metrics fix. Both final dedicated/embedded PocketIC cases and strict internal
feature-enabled Clippy pass. Added original-operation inspection after an
uncollected certificate reply; restoration after preparation/exposure; complete
attestation receipt equality; crossed service/namespace and caller refusals;
expired admission/preparation; pre-exposure and confirmed-upload revocation;
and malformed versus oversized certificate rejection. No production contract,
dependency, sibling, publication or deployment changed in this slice.

Evidence: `/tmp/canic-blob-recovery/`, final `*-qualified.log` and
`clippy-complete.log`; earlier failed decoder expectation and sandbox socket
failure are retained separately. Keep #620/#621 open pending default-branch delivery.
Published Blob/contracts remain 0.40.4. The guide and both changelogs are updated.

End-to-end Blob is still not 100%: upstream #32 needs authoritative provider GC
caller/failure and deletion/billing evidence and shared handlers; #33 needs the
operated verifier/provider acceptance. Serving/multi-user evidence remains #5/#6/#31.
No live-provider, installed-IC or native macOS qualification is claimed. The complete
0.112 batch is not push/publish-ready based on these local checks alone.

## Blob mounted journey and fence metrics — 2026-10-21

Fixed a reproduced stale public metric: Blob capacity inspection reported a recovery
fence after background platform-version changes while `blob.fenced` still reported
active. Sampling now observes the version through Blob's shared installation owner;
it never clears fences or adds a continuity/timer owner.

Pass: strict feature-enabled internal Clippy and both exact managed PocketIC cases.
They now check public accounting/cache/timestamp/visibility and embedded metrics,
plus two successful mounted preparation/certificate/verifier-attestation/reference
journeys, reference replay, released-reference download refusal and retained
attestation statements across restoration. Evidence: `/tmp/canic-blob-metrics/`.
The verifier and bytes are controlled fixtures, not an external provider read.

#621 is locally resolved pending delivery. #620 remains open for crossed bindings,
revoked/expired permissions, lost-certificate-reply reconciliation and interruption
at intermediate operation stages. Upstream provider/GC/billing acceptance remains
separate. Blob is not 100% qualified; complete 0.112 is not push/publish-ready.
Both changelogs and the guide are updated. No dependencies, sibling files, Git
publication, deployment or paid provider effects changed.

## Blob framework qualification checkpoint — 2026-10-21

Adopted published Blob Storage/contracts 0.40.4 locally; qualification is incomplete. Both private consumer locks align with Platform 0.6.6, Memory/Storage
0.50.10 and Wire 0.5.19. Fixed the build macro's conflicting Store App check;
canonical Store identity and selected-App Root identity are both enforced.
Added malformed/wrong-method/trailing/oversized tenant-ingress refusal coverage.

Pass: three-workspace dependency admission, dedicated and embedded managed
PocketIC compositions, production Root target-binding/recovery, the build-identity
regression, strict scoped Clippy, formatting and both complete managed Wasm builds
on Rust 1.91. MSRV evidence is build-only; endpoint acceptance used the normal
local toolchain. Evidence/before-images: `/tmp/canic-blob-closeout/`.

No remaining Blob/Cashier engine was found in Core. Adapter composition remains
Canic-owned. No live provider, native macOS or installed-IC acceptance is claimed;
optional funding #494, dynamic provisioning #493 and downstream Apps are separate.
Both changelogs and the composition guide are updated. Keep #444 open for delivery.
No Git publication or deployment ran. The prior closeout claim was too broad:
Canic #620 still needs the successful mounted upload/certificate/attestation/reference
journey, and #621 needs public metrics composition/restoration acceptance. Passing
admission/recovery tests do not prove those paths. Upstream provider acceptance
remains open under Blob #5/#6/#31/#32/#33; funding is #34 / Canic #494.
Do not describe Blob as 100% qualified. The overall 0.112 batch also remains blocked
by unfinished Auth production integration.

## Retired token diagnostic producers — 2026-10-21

Removed unused delegated-token errors and the token-expiry constructor after
checking current and sibling source consumers. E159 is retired in the canonical
ledger/host catalogue, with no runtime producer constant or numeric reassignment.
Preserved active proof/attestation errors and corrected generated retired-row
formatting so the existing catalogue equality guard remains exact.

Pass: all four diagnostic-register tests, strict all-feature Core/Host library
and test Clippy, consuming Canic receiver-feature Wasm compilation and scoped
formatting/whitespace. Evidence/before-images: `/tmp/canic-auth-error-cut/`.
Both changelogs are updated. No Auth crate change, dependency update, sibling edit
or Git publication ran. This cleanup does not add production portable endpoints.
Root enrollment, retained receiver runtime and endpoint integration still block
the complete 0.112 batch. Activation remains closed pending qualified skew inputs;
TEE hardware and installed-IC acceptance remain unavailable. Not push/publish-ready.

## Retired subject-validator removal — 2026-10-21

Removed the unused topology-based application account validator and its unused
denial surface. Source searches found no production or sibling consumers; only
its own tests depended on it. The selected portable flow authenticates the account
at the issuer and binds the separate browser caller through shared Auth. Active
transport/topology guards and the closed application facade remain intact.

Pass: five focused access-auth tests, strict all-feature Core library/test Clippy,
the consuming Canic receiver-feature Wasm build and scoped formatting. Evidence
and before-images: `/tmp/canic-auth-subject-cut/`. Both changelogs and active Auth
documentation are updated. No upstream Auth change is needed for this removal.
Production enrollment, retained receiver runtime and portable endpoints under
#491/#507 still block the complete 0.112 batch; it is not push/publish-ready.
Skew activation remains closed and TEE qualification unavailable. No dependencies,
sibling source, Git publication, deployment or installed-IC qualification changed.

## Maintenance clock refusal — 2026-10-21

Session maintenance now closes derived indexes before checking the retained clock,
matching startup's fail-closed behavior. A native shared-engine admission case
proves subsequent authorization fails after rollback, preserves the complete
checkpoint and login occupancy, and succeeds after valid restoration with the
original lease. No Auth 0.14.1 API change is needed for this correction.

Pass: 34 focused session-storage tests, strict all-feature Core library/test Clippy,
receiver-feature Wasm compilation and scoped formatting. Evidence/before-images:
`/tmp/canic-auth-maintenance-clock/`. No installed-IC or TEE qualification ran.
Root enrollment, retained receiver runtime and portable production endpoints remain
unfinished under #491/#507; the unqualified skew activation gate remains closed.
The complete 0.112 batch is not push/publish-ready. Both changelogs are updated;
no dependencies, sibling source, Git publication or deployment changed.

## Bounded standalone manifest validation — 2026-10-21

Locally repaired the remaining reader in [#562](https://github.com/dragginzgame/canic/issues/562).
`backup manifest validate` now uses Backup's canonical 64 MiB regular-descriptor
reader before JSON decoding. Native I/O, JSON and schema failures retain their
categories; explicit path aliases remain supported. Snapshot streaming is separate.

Pass: all seven manifest-command tests, including exact/over-limit acquisition,
writerless FIFO refusal, aliases and typed failures; strict CLI library/test Clippy
and scoped formatting/whitespace. Evidence: `/tmp/canic-bounded-manifest-20261021/`.
No dependency, version, Git publication, broad gate or live restore effects ran.

The fix and both changelog entries are ready for integration. Keep #562 open for
delivery and owning acceptance. The complete 0.112 batch remains unready to
push/publish while Auth production work under #491/#507 is unfinished.

## Renewal completion clock — 2026-10-21

Renewal now reads time after its asynchronous sweep when computing success and
failure schedules. An already-expired proof, including exact expiry, no longer
constrains retry to one nanosecond; normal bounded backoff applies. Signed proof
deadlines remain unchanged. This fixes active Root renewal, not portable issuance.

Pass: five focused renewal tests (including before/at/after expiry), strict Core
all-feature library/test Clippy, Root-signing-feature Wasm compilation and scoped
formatting. Evidence and before-images: `/tmp/canic-auth-renewal-completion/`.
No installed-IC or TEE qualification, dependency update or Git publication ran.
Production enrollment, retained receiver runtime and portable endpoints remain
unfinished under #491/#507. The unqualified skew activation gate stays closed;
the complete 0.112 batch is not push/publish-ready. Both changelogs are updated.

## App workspace membership and backup path admission — 2026-10-21

Locally repaired [#295](https://github.com/dragginzgame/canic/issues/295) and
[#317](https://github.com/dragginzgame/canic/issues/317). App creation validates and
adds its initial workspace member with existing scaffold rollback. Deletion previews
member/default-member removals, checks the reviewed manifest and publishes before
removing files. A failed publication preserves the App; partial removal reports the
remaining path and keeps membership updated. Other Apps/shared globs are preserved.

Backup manifests, download journals, saved restore plans and persistence now use one
relative-path rule, with typed refusals before file access. Nested/Unicode identities
remain supported; descriptor confinement stays at use.

Pass: 55 app/scaffold and 58 manifest/journal/restore-plan native cases, strict Backup
and CLI library/test Clippy, scoped formatting and whitespace. Evidence and task
before-images: `/tmp/canic-input-fixes-20261021/`. Checks used the incoming published
Host/Build 0.63.2, Platform 0.6.6 and Memory/Storage 0.50.10 graph. No dependency,
version, Git publication, broad gate, live restore or deployment ran in this task.

These fixes and both changelog entries are ready for integration; the complete
0.112 batch remains unready to push/publish while Auth production work under
#491/#507 is unfinished. Keep these issues open until delivery and owning acceptance.


## Restored receiver authority qualification — 2026-10-21

Added native Ed25519 acceptance through shared-engine admission into Canic's
stable backend, index reconstruction and login-ledger restoration. Matching
current authority succeeds. Changed issuer/key, expired enrollment, a newer login
reset floor and authority shorter than the original lease refuse access with
StaleAuthority. All cases preserve the complete checkpoint and login occupancy.
The fixture's protected authority is independently controlled, not credential-led.

Pass: 31 shared session tests; the final admission-based regression also passed
separately, along with strict all-feature Core library/test Clippy and formatting.
Evidence: `/tmp/canic-auth-restored-authority/`. Only native test code changed;
this supplies no production Root-refresh, endpoint, installed-IC or TEE evidence.
Production enrollment/runtime/endpoints remain incomplete under #491/#507 and the
skew activation gate stays closed. The complete 0.112 batch is not push/publish-ready.
Both changelogs are updated. No dependencies, sibling files or Git publication changed.


## Independent receiver authority in Auth qualification — 2026-10-21

Corrected the durable admission fixture: submitted claims no longer supply the
protected issuer/key, audience, mechanism or grants. Authority comes from an
independent approved set; removed the credential-carried verification key.
Signed unapproved-issuer, excessive-grant, foreign-audience and substituted-key
cases return typed refusals without session/login writes, followed by a successful
approved admission. This strengthens evidence; it does not add production enrollment.

Pass: 30 shared session-store tests, strict all-feature Core library/test Clippy,
scoped formatting and whitespace. Evidence: `/tmp/canic-auth-protected-fixture/`.
Checks used Auth 0.14.1 and incoming Platform 0.6.6/Memory-Storage 0.50.10 selections;
this slice did not change dependencies. No installed-IC or TEE qualification ran.
Production enrollment/runtime/endpoints remain unfinished under #491/#507 and the
skew activation gate stays closed. The complete 0.112 batch remains unready to
push/publish; both changelogs are updated. No Git publication or sibling edits ran.


## Published Auth 0.14.1 and signing completion — 2026-10-21

Adopted crates.io Auth/protocol types 0.14.1 in the catalog and all three maintained
locks; no unrelated package selections changed. Published source revision is
`5d7da698d3e4d297f32445fb88adbc66a79c5520`. Both Rust source trees match 0.14.0;
upstream SDK/receiver qualification and guides changed. Current contract links
and both changelogs now select this release. Upstream fixture acceptance does not
complete Canic production integration.

Signing now timestamps successful results and computes bounded retry backoff
using the completion clock, supplied by production IcOps. Results arriving at or
after expiry are discarded. Root retrieval/renewal also recheck completion time;
both successful and failed callbacks respect current batch identity/status.
Original signed bytes/deadlines and the existing retry cap remain unchanged.

Pass: 29 batch tests, 29 shared session-store tests, seven provisioning tests,
strict all-feature Core library/test Clippy, receiver-feature Core Wasm compilation
and dependency admission for all three workspaces. Receiver-feature Wasm also
passes on Rust 1.91.0. Evidence: `/tmp/canic-auth-signing-deadlines/`. These are
local checks, not native macOS, installed-IC or TEE evidence.

Production Root enrollment, retained receiver runtime/budgets, portable issuer,
admission/logout endpoints and successful authorization remain unfinished under
#491/#507. Production skew activation stays closed; #506 also remains open pending
delivery. The complete 0.112 batch is not push/publish-ready. No sibling edits,
Git publication, release or deployment ran.

## Local application Root binding — 2026-10-21

Startup now requires the configured local application proof Root to equal the
installed Root before retaining authority. Missing and mismatched Roots return
typed refusals without changing the prior binding or authority generation.
The global proof configuration remains unchanged for other consumers.

Pass: both local-authority tests, strict all-feature Core library/test Clippy,
and receiver-feature Core Wasm compilation. Evidence:
`/tmp/canic-auth-root-binding/`. This qualifies the production startup path with
native state fixtures and cross-compilation, not installed endpoint admission.

Root enrollment delivery, retained receiver runtime/budgets and production
issuer/admission/logout are still unfinished under #491/#507. The facade remains
closed; production skew is unqualified and TEE hardware evidence unavailable.
The complete 0.112 batch remains unready to push/publish. Both changelogs are
updated; no dependency, Git publication, release or sibling changes were made.


## Unused token-scope policy removal — 2026-10-21

Removed `auth.allowed_token_scopes` from the maintained schema, parser/validation,
generated runtime configuration, constructors and five Canic fixtures. It had no
authorization consumer. Consuming TOML must remove that setting before rebuilding;
there is no compatibility alias. Local application scope/TTL validation and
retained Root/issuer proof support remain. CONFIG.md and both changelogs now
state the current ownership boundary and required consumer action.

Pass: 49 schema tests, both renderer tests, the Auth allocation regression and
12 runtime Auth tests; the existing chain-key fixture check now also includes
both alternate Root configurations. Strict all-feature Core library/test Clippy,
receiver-feature Core Wasm compilation and native compilation of the actual
Root, issuer and local-authorization probes passed, as did scoped formatting.
Evidence: `/tmp/canic-auth-token-policy-cut/`. Checks used the incoming Auth 0.14.0,
Platform 0.6.5 and Memory/Storage 0.50.9 graph; only missing locked packages were
fetched, with no dependency changes in this slice.

This removes configuration residue; it does not implement protected enrollment
or open the denial-only facade. Root enrollment delivery, retained receiver
runtime/budgets and production issuer/admission/logout remain next under #491/#507.
Production skew activation stays closed; no installed-IC or TEE evidence is claimed.
The complete 0.112 batch remains unready to push/publish. No Git publication,
release, deployment or sibling edits ran.

## Family history truncation — 2026-10-21

Locally repaired [#105](https://github.com/dragginzgame/canic/issues/105): public
history truncation now records rejection in each family's latest admission pass.
Sampling another family preserves its result; a pass without history rejection
clears its own flag, including after expiry releases capacity. Failed snapshot
validation preserves the prior result. An exhaustive allocation-free mask keeps
the flags bounded. Direct `PublicHistoryCache::truncated` callers supply a family.
Reply shapes, stable schemas, retention, gap semantics and query purity are retained.

Pass: 35 targeted history/public-metrics tests with all features, strict Core
library/test Clippy with all features, a no-default-feature Core Wasm check,
scoped formatting and whitespace. Evidence:
`/tmp/canic-history-truncation-20261021/`. Regressions cover all five families,
rejection, independent recovery, invalid replacement, empty passes, expiry and
selected/disabled public response projection. This is native backend and
cross-compilation evidence, not installed-IC or instruction-budget qualification.

This slice and both changelog surfaces are ready for integration. Keep #105 open
until delivery on default main. The complete 0.112 batch remains unfinished on
Auth and is not push/publish-ready. No dependency/version change, commit or push ran.


## Published Auth 0.14 adoption — 2026-10-21

Selected crates.io `ic-auth` and `ic-auth-protocol-types` 0.14.0 after inspecting
the published sources (revision `39a572404374388117b01eb1518321526fe8cee5`).
Updated the framework catalog and all three maintained lockfiles; unrelated
package selections are unchanged. This supersedes the earlier unadopted-draft note.

Canic's real shared-engine admission regressions now project typed future-start,
expiry and caller-substitution refusals, retaining exact original credential retries
and deadlines. Auth's Wire reply and explicit account/browser coordinator changed
upstream; Canic still has no production endpoint/client using those surfaces.
No local codec, text-error fallback or replacement auth flow was introduced.

Pass: 31 session-storage tests, 49 selected batch/delegation tests and seven
provisioning tests (including #506), strict all-feature Core library/test Clippy,
Core receiver-feature Wasm checks on the active compiler and Rust 1.91.0,
scoped formatting/whitespace and dependency admission for the framework and both
independent Blob consumers. Evidence: `/tmp/canic-auth-014-adoption/`.
The receiver's normal dependency graph selects both Auth verifiers and restoration
without Auth signature preparation. Platform verification still shares its
`canister-sig-creation` feature; no whole-artifact signing-code exclusion is claimed.

No new upstream adoption blocker was found. #418/#491/#507 remain open: protected
Root enrollment, retained receiver state/budgets, production issuer/admission/logout,
successful scoped authorization and installed acceptance are still unfinished.
Production clock-skew activation remains closed; no native macOS, installed-IC or
TEE hardware qualification ran. Both changelogs are updated for this adoption;
the complete 0.112 batch is not push/publish-ready. No Git publication or deployment.

## CLI text escaping — 2026-10-21

Locally repaired [#301](https://github.com/dragginzgame/canic/issues/301): runtime
and management inspection, quick-status headers and admission-status lines use
the shared terminal escaping owner before report separators are joined. List
headings and missing-role hints are escaped alongside existing table-cell handling.
Control characters, line separators and bidirectional controls become visible
text; printable Unicode, trusted layout/colours and original JSON values remain.
No transport, endpoint or report schema changed.

Pass: 54 tests in the affected CLI renderer/output modules (74 selected tests
including incidental substring matches), strict CLI library/test Clippy with all
features, scoped formatting and whitespace. Evidence:
`/tmp/canic-terminal-text-20261021/`. Regressions inject OSC/ANSI sequences,
newlines, C1 and Unicode controls into runtime fields, management data, phases,
versions and headings; they verify line boundaries and raw decoded JSON values.
This is native Linux rendering evidence; no live Fleet or macOS run is claimed.

This slice and both changelog surfaces are ready for integration. Keep #301 open
until delivery on default main. The complete 0.112 batch remains unfinished on
Auth and is not push/publish-ready. No dependency/version change, commit or push ran.


## Independent local application Auth policy — 2026-10-21

Removed the local-session configuration dependency on the token-verifier flag
and duplicate token-scope ceiling. Local scopes remain sorted, unique and bounded;
TTL validation and protected Root/registry generation tracking remain intact.
The runtime projection now resolves configured local policy and managed Fleet/role
identity without silently treating it as disabled when that flag is absent.

Pass: seven focused policy/projection/role-allocation tests, strict Core library
and test Clippy, production-feature Core Wasm compilation, scoped formatting and
whitespace. Evidence: `/tmp/canic-auth-policy-decoupling/`. The normal/build Wasm
feature graph was inspected separately from native dev-dependency unification.
It selects both Auth verifiers and restoration without Auth signature preparation.
Platform's `signature-verification` still selects its `canister-sig-creation`
feature (shared public-key/verification primitives); this inspection does not
prove artifact-level signing-code exclusion.

This is configuration/runtime-projection cleanup, not completed Auth integration.
The shared session adapter remains test-gated; promoting it alone would leave
unused code without the protected receiver runtime owner. Root enrollment is
still DTO-only. Next: connect protected enrollment delivery, retained ledger and
budgets, admission/logout and successful scoped authorization. Production skew
activation remains closed; TEE and installed endpoint qualification remain absent.
The Auth 0.14 sibling draft remains read-only and unadopted. #491/#507 stay open.
The complete 0.112 batch is not push/publish-ready; both changelogs reflect this
slice. No dependencies, versions, Git refs or deployed state changed.

## Positive bounded top-up configuration — 2026-10-21

Corrected the validation slice of [#72](https://github.com/dragginzgame/canic/issues/72):
Component and child top-up amounts must be positive and at most half the threshold.
Division avoids the former saturating-multiplication hole at `u128::MAX`.
Omit `topup` to disable it. Configuration docs explain that default initial cycles
start below the default threshold, give the examples explicit initial headroom,
and document the lifetime child budget and existing typed exhaustion warning/stop.
No initial-balance restriction, new runtime warning or funding-cap reset was added.

Pass: both targeted validation regressions, strict Core library/test Clippy with
all features, scoped formatting and whitespace. Evidence:
`/tmp/canic-topup-validation-20261021/`. Cases cover omitted/default policies,
zero amounts, exact/odd thresholds, full-width bounds and both TOML ownership
locations. Validation is host/test-only; no new installed-IC evidence is claimed.

This slice and both changelog surfaces are ready for integration. Keep #72 open
for default-main delivery and final disposition of its startup-warning recommendation.
The complete 0.112 batch remains unfinished on Auth and is not push/publish-ready.
No dependency/version change, commit or push ran.


## Root config diagnostic path removal — 2026-10-21

Locally repaired [#330](https://github.com/dragginzgame/canic/issues/330): the build
macro no longer emits the absolute source config path as a runtime string.
Root startup and both lifecycle forwarding layers no longer carry its redundant
argument. Configuration traps identify embedded config and retain the cause.
Direct helper callers must remove the final path argument; generated sources,
runtime authority, typed configuration and diagnostic TOML are otherwise retained.

Pass: all four build-config surface tests, strict facade/Core/Control Plane
library/test Clippy with all features, a Wasm check of the actual `root_probe`
startup macro, scoped formatting and whitespace. Evidence:
`/tmp/canic-root-config-path-20261021/`. The new subprocess regression compares
current compiler values and generated file contents across two checkout locations.
This is generated-source and cross-compilation evidence, not whole-Wasm
reproducibility or installed-IC lifecycle qualification.

This slice and both changelog surfaces are ready for integration. Keep #330 open
until delivery on default main. The complete 0.112 batch remains unfinished on
Auth and is not push/publish-ready. No dependency/version change, commit or push ran.


## Independent account/browser caller qualification — 2026-10-21

The local read-only Auth 0.14 draft now passes explicit `{ browser, account }`
identities to the private coordinator: account signing is confined to issuer
issue/lookup, while browser signing serves receiver admission. The owner reports
separate SDK qualification; Canic has not adopted or rerun that unpublished code.
No new upstream blocker or alternate flow was introduced.

Canic's credential fixture now takes the observed receiver caller independently
of claims instead of deriving a necessarily matching caller. The new regression
tries the authenticated account principal and another device against the same
credential, requires typed caller mismatch with unchanged storage/ledger, then
admits only the bound browser and checks separate issuer/subject identity.
Pass: 31 focused storage tests, strict Core library/test Clippy and scoped
formatting/whitespace checks. Evidence: `/tmp/canic-auth-caller/`. The incoming
lock selects Metrics 0.15.4; no dependency version was edited by this task.

This is native storage/shared-engine qualification, not production caller capture
or endpoint acceptance. No production code or activation policy changed. Runtime
ownership, protected enrollment, successful receiver/issuer endpoints and browser
integration remain unfinished. Production skew and TEE evidence remain unqualified;
the full Auth/0.112 batch is not push/publish-ready. Changelogs are updated; no
sibling edit, commit, publication, deployment or state reset occurred.


## CLI metrics coverage — 2026-10-21

Locally repaired [#284](https://github.com/dragginzgame/canic/issues/284): metrics
queries follow short pages within the selected per-canister row cap. JSON retains
the latest server `total`, pre-filter `fetched` count and `truncated` flag; text
warns about omitted rows even when `--nonzero` leaves no displayed entries.
Empty pages stop pagination; typed query failures retain error/unavailable status
with an unknown total. Runtime endpoint contracts and dependencies are unchanged.
Strict JSON consumers must accept the three new report fields. Metrics pages
remain live observations, not an atomic snapshot across calls.

Pass: 21 targeted CLI metrics tests, strict CLI library/test Clippy with all
features, the recursive command-help ordering/size regression, scoped formatting
and whitespace. Evidence: `/tmp/canic-metrics-pagination-20261021/`. Regressions
cover short pages, exact caps, later nonzero rows, empty pages, typed failures and
JSON/text coverage. This is native CLI evidence, not installed-Fleet qualification.

This slice and its changelog surfaces are ready for integration. Keep #284 open
until delivery on default main. The complete 0.112 batch remains unfinished on
Auth and is not push/publish-ready; no version change, commit or push ran.


## Production Auth restoration clock boundary — 2026-10-21

The production synchronous startup owner now supplies `IcOps::now_nanos()` to
application-session restoration. Restoration invalidates derived indexes before
checking the persisted clock; rollback refuses with indexes unavailable and no
record changes. Successful restoration advances the protected observation,
rebuilds byte accounting, and leaves session leases/replay/login records unchanged.
Cleanup and login maintenance reuse the same clock/snapshot validation owner.
No activation, enrollment substitution, timer or compatibility path was added.

Pass: 30 targeted session-storage tests, strict Core library/test Clippy and
normal Auth-enabled Wasm checking. Evidence: `/tmp/canic-auth-boot-clock/`.
The regression directly tests the storage boundary with a retained session,
backward time and later successful restoration. This is native backend evidence;
production startup wiring is inspected and cross-compiled, not installed-IC
lifecycle qualification. Existing #506 renewal work is preserved.

Production runtime ledger/budgets, protected Root enrollment/reset metadata,
successful issuer/reconciliation and receiver/browser endpoints remain unfinished.
The 0.14 refusal draft remains separate; no dependency selection changed here.
Production skew and TEE evidence remain unqualified; activation stays closed.
The complete Auth/0.112 batch is not push/publish-ready. Changelogs are updated;
no sibling edit, commit, release, publication, deployment or state reset ran.


## Local Auth 0.14 review and immutable retry fixture — 2026-10-21

Inspected the read-only sibling's dirty 0.14 draft atop delivered commit
`7a1183ba8556ab7c6e7880332a8695af2b86c44f`; its package version remains 0.13.3.
The draft replaces admission error text with the bounded `AdmissionRefusal`,
projects core errors via `SessionError::admission_refusal()` and preserves the tag
in the browser adapter. Receiver encoders/generated clients must move together;
no fallback or downstream copy is needed. Requests, successful replies, credential
bytes, stored records and read/logout contracts are unchanged. No new upstream
blocker was established. Canic remains on published Auth 0.13.3 with no path override.

Refactored the native credential fixture to retain one signed issuance separately
from admission attempts. The regression checks typed future-start refusal without
writes, later creation, exact recovery of the original live session after the
admission window closes, and expired refusal at fresh receiver state. No retry
reissues or changes bytes/deadlines. These are Canic storage/shared-engine cases,
not real receiver endpoints, an issuer service or installed browser qualification.
Pass: 29 storage tests and strict Core library/test Clippy; evidence:
`/tmp/canic-auth-014-review/`. No production source changed; no fresh Wasm or
installed endpoint qualification is claimed.

Pending 0.14's refusal projection does not implement Canic's missing runtime,
enrollment, issuer/lookup or receiver/browser endpoints. Adopt matching published
Auth packages and generated Wire clients together when available; do not reproduce
the draft enum locally. Production skew and TEE evidence remain unqualified and
activation stays closed. The complete 0.112 batch is not push/publish-ready. No
sibling edit, dependency update, Git publication or deployment occurred.


## Store staging-status allocation and hashing — 2026-10-21

Locally repaired [#149](https://github.com/dragginzgame/canic/issues/149): staging
status reads each chunk once, retains observed progress hashes and streams the
payload SHA-256. It no longer concatenates payload bytes or allocates from the
declared size. Inconsistent lengths, hashes or chunk counts report incomplete.
Preparation rejects sizes above the chunk-count ceiling or Store byte limit
before any manifest, metadata or combined-first-chunk write. Stable records,
endpoint shapes, chunk size and digest identities are unchanged.

Pass: all 25 targeted Store template tests with only `wasm-store-canister`, strict
Control Plane library/test Clippy with all features, Store-only Wasm checking,
scoped formatting and whitespace. Regressions cover full-sized chunks, partial
progress, inconsistent retained evidence, huge declared sizes, typed atomic
refusal, exact ceilings and existing same-operation replay. Evidence:
`/tmp/canic-store-staging-fix-20261021/`. This is native Linux and cross-compilation
evidence; installed query instruction budgets, PocketIC and native macOS were
not measured. Both changelog views and the publication guide are updated.
The code repair is ready for integration and remains uncommitted; #149 stays open
pending default-main delivery. The complete 0.112 batch still has outstanding Auth
work and is not push/publish-ready. No release or Git publication ran.

## Shared Auth login pruning adapter — 2026-10-21

Implemented bounded login maintenance in the existing test-gated session adapter.
Auth 0.13.3 `prune_with_commit` selects expired callers and resulting pool counts;
Canic commits the complete candidate before heap deletion. The adapter checks the
ledger's exact records/counts and Fleet binding, preserves trusted time on empty
or failed work, and refuses removal while a retained session requires the login.
No parallel retention algorithm or new timer was added. Cleanup's existing clock
and snapshot validation are shared by both maintenance paths.

Pass: 28 storage tests, followed by all three pruning cases with the final
foreign-Fleet assertion, strict Core library/test Clippy and Auth-enabled Wasm.
Evidence: `/tmp/canic-auth-prune/`. Cases exercise 256 unknown logouts in two bounded
batches, pre-expiry retention, pool reuse, exact stable accounting, rollback,
atomic commit refusal, subsequent successful cleanup and incomplete-ledger refusal.

The adapter remains test-gated; this is backend qualification, not production
pruning or endpoint acceptance. Production wiring requires the retained runtime
ledger/budgets and protected skew/enrollment metadata. Production activation stays
closed under the maintainer's unqualified-skew decision; TEE hardware qualification
is unavailable and remains separate from non-TEE custody. The complete Auth/0.112
batch is not push/publish-ready. Both changelog views are updated. No sibling edits,
commits, publication, deployment or state reset occurred.


## Build configuration reporting and App binding — 2026-10-21

Locally repaired [#75](https://github.com/dragginzgame/canic/issues/75): Root's
diagnostic TOML serializes the validated, host-resolved model, preserving multiline
values and reporting defaults and derived public fields. Cycle values retain the
model's decimal-count representation; runtime-injected trust data is excluded.
Removed the line-trimming helper and raw-source argument; no TOML parser was added
to canister Wasm.
Locally repaired [#329](https://github.com/dragginzgame/canic/issues/329): Root/Store
metadata must name the selected App before output generation. Removed unused
build-helper exports and role-only adapters; maintained metadata and role checks
remain. Both hard cuts are described in the existing 0.112 notes and CONFIG.md.

Pass: 16 default-feature build-support tests, all three real build-macro surface
tests, strict facade/Core library/test Clippy with all features, control-plane
facade Wasm checking and scoped formatting/whitespace. Evidence:
`/tmp/canic-build-config-fixes-20261021/`. This is Linux/native and cross-compile
evidence, not installed-IC, PocketIC or native macOS qualification.
These fixes are ready for integration and remain uncommitted; both issues stay
open pending default-main delivery. The complete 0.112 batch still has outstanding
Auth work and is not ready to push or publish. No release or Git publication ran.

## Auth protected cleanup clock — 2026-10-21

Continue software integration while TEE hardware qualification is unavailable.
The maintainer's no-qualified-production-skew decision still keeps activation
closed; TEE evidence and ordinary non-TEE custody remain independent choices.

The existing stable Auth header and atomic state projection now retain
`observed_at_ns`. Cleanup persists accepted time before validation, refuses
backward time, retains observations on empty/failed work, invalidates indexes on
corrupt state, and rebuilds encoded-byte accounting after header changes. Rows
remain unchanged on refusal. This is the current 0.112 reinstall-only contract;
there is no migration, compatibility reader or new state generation.

Pass: 25 session-storage tests, eight runtime/intent tests and the stable record
footprint check. Formatting, whitespace and locked input stability also pass.
Coverage includes empty cleanup, restore rollback, failure clock retention,
accounting, and the actual shared ledger rejecting an older restore clock without
extending login retention. Strict Core library/test Clippy and normal Auth-enabled Wasm checking pass; evidence:
`/tmp/canic-auth-clock/`. The incoming lock selected Metrics 0.15.3; fetched that
exact release after offline cache refusal without updating dependency selections.

This does not complete protected runtime metadata: production admission/logout
must retain observations too, and retained ledger/budgets, Root enrollment/reset
floors, endpoint success paths and issuer/browser integration remain unfinished.
No login/tombstone deletion or admission activation was added. Existing #506
renewal behavior and unrelated pending work remain intact. The complete Auth and
0.112 batch remains not push/publish-ready; notes are updated, no Git publication,
deployment, sibling edit, installed-IC or TEE qualification occurred.


## Store and Observatory reporting fixes — 2026-10-21

Locally repaired [#538](https://github.com/dragginzgame/canic/issues/538): Store
creation now stores epoch seconds, matching publication, GC and retirement.
The separately named adoption receipt retains exact nanoseconds and retry identity.
This is a 0.112 hard cut; no predecessor-state conversion was added.
Locally repaired [#541](https://github.com/dragginzgame/canic/issues/541): the exact
Coordinator row resolves its Observatory subnet from the validated Fleet binding;
Root descendants retain parent-chain placement and unresolved rows remain null.

Pass: 39 Observatory tests, the Root-only overview timestamp regression, eight
owning Store state tests, strict Host/Control Plane/Contracts library/test Clippy
with all features, Root-only Wasm checking and scoped formatting/whitespace.
Evidence: `/tmp/canic-reporting-fixes-20261021/`. This is native Linux and Wasm
cross-compilation evidence; no PocketIC, installed-IC or native macOS run occurred.
Both changelog views and the operator timestamp/placement guidance are updated.
These two fixes are ready for integration; the complete 0.112 batch still has
outstanding Auth work and is not push/publish-ready. Changes remain uncommitted;
both issues stay open pending default-main delivery. No release or Git publication ran.

## Auth activation decision and cleanup validation — 2026-10-21

The maintainer confirmed no maximum issuer/receiver clock-skew bound is qualified
for the intended live Fleet: keep production activation closed. Controlled-clock
fixtures remain allowed; their values are not production evidence. The existing
closed authorization facade remains unchanged. No production configuration flag
or activation mechanism is implied by this decision.

Fixed cleanup accepting inconsistent session/login state after deleting its
expired session: validate the original complete snapshot before candidate expiry
removal. The focused regression preserves mismatched account records on typed
refusal and still cleans a valid expired session without deleting its reservation.
The planned login/tombstone deletion was not implemented: shared ledger pruning
also requires persisted trusted clock observations, including empty/failed work;
Canic's production metadata/runtime ownership is still missing. Do not add a
standalone expiry loop that bypasses that contract.

Pass: 23 focused storage tests, strict Core library/test Clippy with Auth enabled,
and normal Auth-enabled wasm32 checking. Evidence:
`/tmp/canic-auth-cleanup-validation/`. These are backend/native and cross-compile
checks, not real endpoint acceptance. Tracker updates: [#491](https://github.com/dragginzgame/canic/issues/491#issuecomment-6317804220)
and [#507](https://github.com/dragginzgame/canic/issues/507#issuecomment-6317804874). No sibling edits or Git publication occurred. Both
changelog views are updated. #491/#507 and the complete Auth/0.112 batch remain
unfinished and not push-ready: Root enrollment, runtime metadata, portable
issuer/reconciliation, successful receiver endpoints, browser logout transport and
installed acceptance still need implementation and qualification. The missing
production clock bound is an additional deployment requirement, not the reason
those implementation tasks are unfinished.


## Current Auth contract reconciliation — 2026-10-21

Read all five requested IC Auth contracts and inspected source at
`7a1183ba8556ab7c6e7880332a8695af2b86c44f` (0.13.3; sibling lockfile dirty,
left untouched). Canic uses published Auth 0.13.3. Active authentication docs and
#491/#507 now distinguish current supported shared APIs, historical token/client
flows and missing Canic production integration. No sibling source was changed.

Removed Canic's competing per-account session quota and its derived account
index. Session/replay occupancy follows browser callers; `(issuer, subject)`
remains account identity. The maximum-capacity regression now uses independent
browser devices for one account, and the two-issuer identity/logout regression
retains separate accounts after restoration. No stable encoding changed.

Pass: 22 focused session-storage tests, strict Core library/test Clippy with
local Auth enabled, normal Auth-enabled Wasm checking, formatting and whitespace.
Evidence: `/tmp/canic-auth-production/`. These are backend/native and
cross-compilation checks, not successful production endpoint acceptance. The
incoming #506 renewal fix was preserved; this pass did not re-run its tests.

The requested production integration is NOT finished. The facade remains denial
only and the transaction adapter remains test-gated. Root registration/enrollment,
retained runtime metadata/budgets, issue/lookup, admission/read/logout exports,
volatile reset profile, complete persistent restore and the browser adapter all
remain implementation work. No missing shared session-engine API was established.
Do not describe those Canic gaps as an upstream block. The private/unpublished
browser package needs a real consumer/distribution boundary; no production client
was added. TEE evidence and a qualified production volatile-reset clock bound
remain external deployment requirements, separate from local software testing.

The complete 0.112/Auth batch is not push-ready or publish-ready. Trackers remain
open; no commits, pushes, publication, deployment or state reset occurred.


## Log retention and backlog disposition — 2026-10-21

Locally repaired [#73](https://github.com/dragginzgame/canic/issues/73) and
[#102](https://github.com/dragginzgame/canic/issues/102): configuration rejects
log messages above 64 KiB, and age expiry beyond the IC clock's range schedules
no deadline instead of failing runtime startup. Count retention and the exact
requested age remain unchanged; no stable encoding or lifecycle owner changed.
The 0.112 notes mark the new configuration bound as breaking; CONFIG.md agrees.

Pass: all 15 targeted log tests, strict Core library/test Clippy with all
features, scoped formatting and whitespace checks. Evidence:
`/tmp/canic-issue-log-fixes-20261021/`. This is native Linux evidence, not
PocketIC startup or installed-IC qualification. Both fixes remain uncommitted
and their issues stay open pending default-main delivery.

Closed [#130](https://github.com/dragginzgame/canic/issues/130) as not planned
after reviewing default main's byte-based replay identity and canonical display
against the current input-normalization policy. The existing Auth integration
acceptance below still prevents the complete 0.112 batch from being push-ready.
No version, Git publication, release or deployment action ran.

## Auth issues #491, #506 and #507 — 2026-10-21

Local #506 repair preserves the typed issuer response through installation,
validates exact proof/hash/issuer/Root and effective deadlines, reads fresh Root
completion time, and stores effective expiry/refresh through the existing renewal
writer. Newer installed retained proof epochs fence late older responses. No
stable record/schema change or second renewal writer was introduced.

Pass: 27 chain-key batch tests, seven provisioning tests, strict Core library/test
Clippy and normal Auth-enabled wasm32 checking. Logs: `/tmp/canic-auth-complete/`.
The production local-authorization feature now explicitly selects the complete
certified verifier, addressing the dev-feature masking gap under #491.

#491 remains unfinished: protected Root enrollment, retained clock/ledger/budget
runtime ownership and successful establish/authorize/logout endpoints still need
implementation and installed qualification. #507 has no portable issuer/lookup
endpoint yet; do not restore the retired delegated-token endpoints. The maintainer
selected the authenticated IC caller as the account subject. Standard IC identity
delegation preserves that caller identity; the portable browser key is a separate
transport principal and must never replace the verified account subject. An
intermediary issuing for another subject requires separately explicit authority.
The maintainer requested a detailed explanation of these options after the fix.

Both changelog views are updated. Changes remain uncommitted and issues stay open
pending delivery/remaining acceptance. The complete Auth/0.112 batch is not ready
to push or publish. No live IC, macOS or full-suite qualification is claimed.


## Upstream adoption and Auth configuration fixes — 2026-10-21

Verified published Auth/protocol-types 0.13.3, Blob/contracts 0.40.3, Query 0.67.3
and Wire 0.5.16. Preserved the root's incoming updates and aligned both private
Blob consumer locks, including Platform 0.6.4, Memory/Storage 0.50.8 and Metrics
0.15.2. The Blob catalog floor is 0.40.3. IC Tooling 0.63.2 and Shared Tooling
0.26.2 were already current at inspection. No sibling source or Git refs changed.

Fixed #63 locally: attestation epoch floors reject noncanonical and undeclared
application roles while accepting declared and built-in infrastructure roles.
Fixed three #70 gaps: signing version below its acceptance floor, explicit
paths longer than 255 components and delegation/attestation TTL overflow.
Network key-ID policy and host curve-validation coverage remain separate #70
questions. No endpoint bytes, stable schema or runtime lifecycle changed.

Pass on the final graph: 50 configuration tests, 21 durable Auth session tests,
strict Core and both private Blob consumer Clippy, Auth-enabled Wasm compilation,
three-workspace dependency admission and both exact managed Blob composition
cases. Lock hashes remained fixed during the final qualification. Evidence:
`/tmp/canic-upstream-next/latest-*`. Earlier proofs use older selections; an
intermediate managed run correctly refused a changed root lock and is not a
runtime failure or final-graph acceptance. Its logs and failed scratch are retained.

This is Linux/native and local PocketIC evidence, not native macOS, fresh MSRV,
installed-IC Auth or external Blob provider qualification. Both changelog views
and CONFIG.md are updated. #63/#70/#444 stay open pending default-main delivery
and remaining acceptance. Auth's production admission is still fail-closed:
protected Root enrollment, retained clock/ledger/budget ownership, complete
verifier selection and endpoint success/logout wiring remain under #491.
The complete 0.112 batch is not push-ready; no commit, push or release ran.

## Auth integration and tooling adoption — 2026-10-21

Adopted published IC Tooling 0.63.2 for all seven existing root dependencies;
the lock update changes only that package family. Shared Tooling 0.26.2 is pinned
to remote-main revision `1959ef75d0b86ed7725d41ae5056303d35eb633d` through its
canonical exporter (no matching GitHub Release/tag was found).
Host receipt verification now uses the published bounded ICP decoder. Certificate
and exact-payment authority stay local; the protobuf projection and direct Prost
edge are test-only. Testkit's 0.63.2 server is prepared and passes offline admission
with `cli`. Both private consumers have no IC Tooling dependencies to update.

Pass: snapshot verification, three-workspace dependency admission, shared IC-tool
installer fixtures, ShellCheck with sourced companions, 15 authenticated ICP
receipt cases, ten Build cache cases, strict Host library/test Clippy, and
CLI/Backup/internal-testing/runtime-test library and test compilation. Scoped
manifest/Rust formatting and diff hygiene pass. Evidence: `/tmp/canic-tooling-adopt/`.
This is native Linux and synthetic host-fixture evidence, not native macOS,
PocketIC execution or installed-IC qualification. No publication, release or sibling
edit ran. Both 0.112 changelog drafts include the adoption; Auth remains incomplete.

Verified published Auth/protocol-types 0.13.2; the root already selects it and both
private Blob fixture locks now match. Scope is Canic only, with no downstream
application work. Preserve the other pending dependency and source changes.

Removed unused test-only session-policy/error adapters and their self-tests.
The authority snapshot no longer copies an unused generation; the tested atomic
backend reads storage generation at transaction and commit time. Its generation
read helpers are compiled with their current test-only backend consumers. Retain
Root enrollment/proof authority, certification composition, canonical conversions,
stable records and the real session backend. No stable schema or wire change.

Current Auth follow-through preserves issuer-scoped account identity. The public
authorization result now includes issuer, and derived capacity indexes, logout,
restoration and heap estimates key accounts by issuer plus subject. A signed
regression fills one issuer's account capacity, admits an equal subject under
another issuer, restores both and logs out the second without changing the first.
The prior Fleet-binding restoration check remains: logout-only rows must match
enabled Fleet authority; disabled authority retains denials but rejects mixed
audiences. Failed restoration preserves records and installs no indexes.
No stable schema or canonical session encoding changed.

Pass: 23 targeted session-storage tests, strict Core library/test Clippy and the
auth-local-application-authorization Wasm graph on Auth 0.13.2, Memory/Storage
0.50.7 and the maintainer-selected Platform 0.6.4. Evidence:
/tmp/canic-auth-account/; prior Fleet restoration evidence remains in
/tmp/canic-auth-fleet-restore/. Public guidance and both 0.112 notes describe
the issuer field's source-level hard cut. Admission remains closed; these native
and cross-compilation checks do not establish an installed successful login.

Prior direct signed admission, exact retry with exhausted credit, restore/access/
logout and real commit-refusal evidence remains in `/tmp/canic-auth-admission/`.
Earlier private-consumer/dependency-admission checks in `/tmp/canic-auth0132/`
were not rerun for Memory/Storage 0.50.7. This is native Linux and Wasm
cross-compilation evidence, not installed login, PocketIC admission or native macOS.

Auth adoption is not complete: application authorization still deliberately denies
access, and the production shared session store/admission/logout wiring remains
unfinished under #491. Published APIs exist. Next integrate the protected Root
approval/caller/clock with the canonical engine and prove successful admission,
scoped access, logout, authority changes and failed atomic commits through the
actual endpoint path. Do not treat backend tests or this cleanup as a live login
service. The wider 0.112 batch is not push-ready from this slice alone. Both
changelog drafts are updated; no commit, push, release or sibling edit ran.

## Focused Core fixes — 2026-10-21

Implemented locally: #108 propagates reached internal and unavailable-network
access evaluation errors through negation/alternatives; #95 seeks the final
cycle-event key within its timestamp range; #87 removes the ignored ops settings
sender-version field and all five callers while retaining the management request's
current-execution version binding. No storage encoding or lifecycle change.

Pass: eight access-expression tests, five cycle-storage tests, strict Core and
Control Plane library/test Clippy, scoped Rust formatting and diff whitespace.
Checks use the maintainer's current locked Auth 0.13.1/Metrics 0.15.2 selections;
missing locked packages were fetched without updating dependency selections.
Evidence: `/tmp/canic-followup-fixes/`. No broad suite, native macOS or installed-IC
qualification was run. These three issues stay open until default-main integration.
Both 0.112 changelog drafts include this slice. The complete integration batch
is still pending; this result does not resolve the PR/release blockers below.
No commits, pushes, merges, release or deployment ran; unrelated work is preserved.

Continued #419 locally: session cleanup now runs despite an intent-cleanup error,
records both owners' failures, and retains a one-second retry. Timers deliberately
forces invariant completions to Stop, so the coordinator reports retryable failure
while preserving the original storage diagnostic. The local-intent failure test
checks expired replay removal, live replay retention, intent preservation and the
retry decision. Full-capacity, receipt/session-failure injection and same-release
PocketIC recovery acceptance remain outstanding; do not close #419 yet.

#463 locally replaces GNU find formatting with portable basename execution.
Its independently runnable inventory regression covers empty/populated topics,
spaces, hidden directories, advisory mismatch and genuine find failure. Linux
execution and targeted ShellCheck pass; native macOS remains unqualified. The
existing combined document tests stop on a stale capability-surface audit
fingerprint before their other cases; this change does not refresh audit authority.
Eight focused intent-workflow tests and strict Core library/test Clippy pass.
Evidence for both repairs: `/tmp/canic-portable-guard/`. Neither issue is integrated
into default main. Next: complete #419's owner-failure and capacity/restart proofs,
then reconcile the stale audit fingerprint with its actual changed authority.

## Verified issue closeout

Blob follow-through: closed #524 as completed and PR #603 as superseded after
comparing implementation/tests against remote main `4caeec25db7d81f6121db1c95ba7a2d91d319c60`.
Main already owns shared Blob dispatch, async raw admission and exact installation
refusals; current Blob 0.40 delivery remains pending under #444. No PR was merged.
Upstream Blob PR #97 changes CI compiler selection only and has pending checks;
it supplies no missing Canic consumer API. Keep #123 (fixture-import trap recovery),
#493 (future parent authority), #494 and provider/application acceptance open.

Locally repaired #410's endpoint inventory: recursive source discovery, raw
entrypoints, exact query/update membership, both-manifest uniqueness and exact
command-enum selection. Existing standards queries now have read-only rows.
All 33 replay-policy tests, strict Core library/test Clippy, scoped formatting
and diff hygiene pass with the maintainer's newly selected Auth 0.13.2;
this does not requalify Blob PocketIC against that new Auth selection. Keep #410
open until default-main integration. Evidence: `/tmp/canic-blob-triage/`.
Both changelog drafts include this focused guard repair; no broad suite or release
ran and the complete 0.112 batch remains pending.

Closed #512 and #71 as completed on default main; closed #27, #114–#122,
#124 and #125 as superseded after preserving unique requirements on #444/#494.
Evidence remains on those issues and in `/tmp/canic-closeout-20261021-evidence/`;
the exact main export is `/tmp/canic-closeout-20261021/`. Open count was 438.
This was a selected-candidate review, not a complete backlog audit. Keep #123,
#419, #463, #499 and outstanding native/integration acceptance open.

## Blob 0.40.1 adapter qualification — 2026-10-21

The maintainer scoped this work to Canic's published Blob adoption and repository
cleanup. Toko Miner and other downstream application integration are excluded.
Dynamic parent provisioning (#493), provider qualification and optional guarded
funding (#494) remain separate work; they are not completion criteria for this
Canic adapter cleanup.

Verified published Blob/contracts 0.40.1 and aligned both private consumer locks
with the root's selected 0.40.1. Root retains Platform 0.6.3, Memory/Storage 0.50.6,
Auth 0.13.2 and Wire 0.5.11; private consumers retain their independent Platform
0.6.1, Memory/Storage 0.50.4, Auth 0.13.0 and Wire 0.5.9 selections. No runtime
handle bridge or sibling source dependency was introduced.

Removed the adapter's duplicated category-based request-limit table. Published
Blob's per-method ceiling now controls ingress registration, bounded raw argument
reads and Wire decoding. Numeric limits, provider Candid replies, caller binding,
shared memory ownership and lifecycle behavior are preserved. The previous
CONFIGURATION_LIMITS removal leaves application-envelope ownership with the
consumer. Active usage guidance is application-neutral. No Canic-owned Blob
runtime or Cashier implementation remains to extract.

Pass: three-workspace dependency admission; strict native Clippy for dedicated
and embedded consumers; both exact managed PocketIC composition cases, including
fresh Wasm builds, endpoint parity, malformed installation refusal, tenant/uploader
permissions, fencing and same-release restoration. The embedded application state
is retained. Evidence: `/tmp/canic-blob0401/` (dedicated 132s, embedded 50s, excluding
runner preflight). An initial competing lint was stopped after detecting an
independent workspace check; subsequent validation ran after the target was idle.
Earlier Root target-binding proof remains recorded on Blob 0.40.0 in
`/tmp/canic-blob-followthrough/`; it was not rerun in this endpoint-limit slice.

This Canic-only adoption/cleanup slice is ready for integration and both 0.112
changelog drafts are updated. #444 remains open for default-main delivery and its
separately tracked acceptance. The complete 0.112 batch still contains other
pending work; no full suite, native macOS, installed IC, commit, push or release
was performed. Historical blocker statements below do not supersede this scope.

## Backlog closeout and portable Backup fixture — 2026-10-21

Verified remote default main remains `4caeec25db7d81f6121db1c95ba7a2d91d319c60`;
local 0.112 integration is ahead and dirty. Closed #533 and #482 as not planned,
explicitly superseded by the reinstall-only pre-1.0 policy already on main. This
is not implementation of data-preserving upgrades or permission to erase data.
Same-release recovery, backup/restore and cycle conservation remain required.

Corrected #515's remaining writerless-FIFO fixture in `persistence/json.rs` to
use native `mkfifo -m 600`, avoiding a Rustix export excluded on Apple targets.
The exact Linux regression and strict Backup library/test Clippy pass on the
current lock selecting Platform 0.6.2, retaining typed `NotRegularFile` refusal.
The concurrent dependency update was preserved; it is not attributed to this fix.
Native macOS and default-branch integration remain outstanding; keep #515 open.
Evidence: `/tmp/canic-issue-closeout/`. Preserve prior pending integration work.

PR #617 has passed preflight/security but its combined native/build acceptance
was still running at review; #604 awaits that stacked repair. #605 and #606
have conflicts and failed checks. None was merged or closed merely for age or
because a similar local change exists. The broader Blob Platform blocker remains.

## Focused integration follow-up — 2026-10-21

The governed ingress target exposed two local defects after dependency adoption:
PocketIC selected ambient ICP 1.6.0 instead of the verified repository-local 1.7.0,
and the bare payload probe endpoint was absent from its generated Candid service.
The runner now uses the owning `ic-tools-check` selection before choosing the
prepared executable, preserving explicit overrides, native/version checks and
invocation-only PATH changes. The probe explicitly registers its existing Candid
contract; endpoint bytes and admission limits are unchanged.

Pass: native ICP selector regressions (PATH shadowing, explicit override, refusal
without scratch/PATH effects, and plan-only operation); scoped ShellCheck;
warning-denied payload probe Clippy; governed `pic_ingress_payload_limits`
(all six cases), `pic_receipt_backed_intent` conformance (one composite case),
and `pic_root_funding_recovery` (all four cases, including lost responses, replay
and measured recovery floors); and `timer_authority` (all ten cases covering
registration, lifecycle, restoration, observation authority and bounded history).
These execute through Testkit 0.63's prepared PocketIC 16.1 server. This is local
Linux PocketIC evidence, not native macOS or installed-IC qualification.
Logs: `/tmp/canic-followup/`. Preserve all preceding pending dependency work.

Blob/contracts 0.39.0 remains the latest published selection and still retains
Platform 0.5.1; Blob integration and governed internal compilation remain blocked
by [Blob #95](https://github.com/dragginzgame/ic-blob-storage/issues/95). The full
release batch remains unqualified; no Git or release operations were performed.

## Dependency qualification — 2026-10-21

Root and both independent Blob fixture locks select Platform/macros 0.6.1,
Timers 0.30.1, Memory/Storage 0.50.4, Metrics 0.15.1 and Auth/protocol types
0.13.0. All IC Tooling libraries now select published 0.63.0: Build, Host
Artifacts/FS/Process/Text/Tools and Testkit. Backup 0.37.0 and Query 0.67.0
align their public Host types with that family. No sibling patches or unpublished
overrides were used. The release-metadata executable recipe remains unchanged.

The published testing facade and 327 internal/runtime fixture calls now use
explicit Platform Candid encoding, the existing bounded decoders and raw Testkit
transport. Removed Testkit Candid helper/error consumers and facade reexports;
no replacement generic codec helper. Existing caller identities, methods and
argument/result tuple shapes remain. Baseline recovery recognizes typed transport
failures without classifying decoder/rejection text as dead transport. Exact
relay/sampling/publication scenarios share their repeated protocol calls locally.
Track integration at [Canic #615](https://github.com/dragginzgame/canic/issues/615);
these changes are pending locally, so the issue is not closed.

Testkit's governed server installer now selects the required `cli` feature.
The recipe regression passes with installer effects substituted; ShellCheck and
focused formatting pass. Explicit `testkit-server.sh` setup installed the published
0.63 executable;
its receipt check passes with the `cli` feature selected. No managed runtime
scenario was executed.

Pass on Linux/Rust 1.99.1: strict Core library/test Clippy with all features on
Auth 0.13; strict canic-tests library/integration-test Clippy on the aligned
Tooling/Query/Auth graph; strict internal fixture library Clippy; Backup library/test
Clippy on Backup 0.37/Host 0.63;
all three dependency-boundary checks and locked fixture metadata; 16 Auth session
transaction/replay/logout/restoration regressions; Core Wasm compilation with local
application authorization. Governed internal test compilation is blocked by two
Blob adapter Principal mismatches (Platform 0.5.1 versus 0.6.1). Earlier focused
Core/Host/facade regressions remain documented below; they are not newly rerun
runtime evidence. Current logs: `/tmp/canic-tooling-complete/`.

Release admission still fails on Blob/contracts 0.39.0 retaining Platform 0.5.1
alongside 0.6.1. Auth 0.13.0 resolves its part of the split. Remaining published
alignment: [Blob #95](https://github.com/dragginzgame/ic-blob-storage/issues/95).
Do not bridge runtime handles or weaken the gate. No embedded-fixture refresh,
managed runtime execution, full suite, MSRV build, native macOS qualification,
commit, release or deployment ran in this continuation. The complete dependency
adoption batch is not ready to push as a qualified release.

Shared Tooling remains reviewed 0.26.1 (67d4f25d1a613a27d7dfd7b5a4ae119e0e529fc7).
Its 132 snapshot files, selected ic-wasm/icp/wasm-opt tools and the automatic
adoption-before-Clippy / fixture-freshness release barriers were qualified in the
prior slice. Evidence remains in `/tmp/canic-shared0261-*.log`.

## Focused dependency adoption admission — 2026-10-21

Added `bash scripts/dev/check-dependency-adoption.sh PACKAGE...`: locked/offline
resolution and existing boundary checks across all three workspaces, normal-Wasm
Blob Memory/Platform/Storage singleton diagnostics, explicit affected-package and
fixture Clippy, then read-only embedded-peer verification. No implicit updates or
artifact refresh; no new release pipeline. Usage and limitations are in TESTING.md.
Command-recording regression tests join the existing tooling gate.

Pass: failure/ordering fixtures, ShellCheck, whitespace, and the real command with
`canic-blob-service` selected (including both independent fixture lint checks).
Its first real run detected stale embedded Wasm after the incoming Platform 0.5.1
lock selection. Explicitly refreshed the artifact/provenance and reran successfully.
The incoming Cargo.lock edit is preserved. Evidence: `/tmp/canic-adoption-check.log`,
`/tmp/canic-adoption-refresh.log`, `/tmp/canic-adoption-recheck.log`.
Shared reusable ownership request: [Shared Tooling #295](https://github.com/dragginzgame/shared-tooling/issues/295).
No broad suite, managed installation proof, native macOS qualification, commit or
release ran. This check is ready to use; complete release readiness remains subject
to the owning release validation, including installed behavior beyond this check.

## Embedded allocation peer refresh — 2026-10-20

Refreshed the checked-in peer Wasm and `scripts/dev/managed-root-fixture.json`
through the owning locked/offline refresh example after dependency changes made
its executable stale. The refresh reconfirmed identical bytes after publication;
the separate locked/offline `verify_embedded_root` example now passes.
Logs: `/tmp/canic-peer-refresh.log` and `/tmp/canic-peer-verify.log`.

No runtime source or dependency selections changed. Changelogs reflect the new
fixture evidence. This clears the reported test preflight failure; subsequent
ordinary and PocketIC suites were not run, so complete release readiness remains
unverified. No commit, release, push or publication ran.

## Blob 0.39 dependency alignment — 2026-10-20

The maintainer's root manifest/lock edits select published Blob/contracts 0.39.0.
Its Memory 0.50, Storage 0.50 and Platform 0.5 requirements resolve the six prior
adapter type errors. Targeted `canic-blob-service` Clippy passes with all targets,
all features and warnings denied. Preserve those incoming root dependency edits.

Aligned both independent Blob fixture catalogs and lockfiles with Platform 0.5;
the embedded fixture also selects Memory/Storage 0.50. Both now select the root's
Timers 0.29.7, eliminating Platform 0.4. Locked/offline native checks pass for both.
Logs: `/tmp/canic-blob039-{clippy,consumer-native,embedded-native}.log`.
Direct Wasm checking was refused by the managed-build guard; it is not Wasm
qualification. Managed builds and installed dedicated/embedded recovery proofs
remain outstanding. This dependency blocker is cleared, but complete release
readiness is not yet established. No broad gate, release or publication ran.

## Aligned Host type cleanup — 2026-10-20

Removed the redundant ledger-tip reconstruction after Query 0.66 aligned with
Host 0.62. Backup 0.36 now shares that Host graph too: use its canonical directory
error conversion and delete Canic's duplicate helper, preserving typed error tests.
The incoming Cargo.lock selection of Memory 0.50.1 is preserved byte-for-byte.

Pass: targeted Host/Backup Clippy with all targets/features and warnings denied,
10 artifact tests and 15 authenticated-ledger receipt tests, formatting and diff
whitespace checks. Logs: `/tmp/canic-aligned-{host-clippy,backup-tests,receipts-tests}.log`.
Registry lookup still reports Blob 0.38.0, whose published manifest requires
Memory 0.49 / Storage 0.5 / Platform 0.4. The incompatible public runtime/principal
types remain the release blocker tracked in [Blob #93](https://github.com/dragginzgame/ic-blob-storage/issues/93).
Open 0.112 changelogs reflect direct shared-type use. The complete batch remains
blocked; no full suite, release, push or publication ran in this cleanup.

## Shared Tooling 0.25.9 adoption — 2026-10-20

Refreshed through the canonical exporter from published main `85aded191d85ac4a8c0e5deab207f6be18895862`
using a clean isolated source checkout; preserved the sibling's unpublished edits.
All 132 snapshot files verify. The changed release runner refuses another version
increment from committed preparation until exact annotated-tag and destination
branch evidence confirms delivery. Tool/dependency selections are unchanged.

Canic already runs Clippy immediately after tool admission. Executing its actual
Make recipes with recording stubs under `make -j4` proved both success order and
that Clippy failure prevents invariant/check/feature/test stages. Publication and
fast-patch refusal fixtures, isolated release-runner tests, focused ShellCheck
and whitespace checks pass. Logs: `/tmp/canic-shared0259-*.log` and
`/tmp/canic-shared0259-order/`. No full Clippy/build/PocketIC suite or real release,
publication or push ran. Open 0.112 changelogs are updated; overall release
readiness still depends on the Blob runtime alignment recorded below.

## Lockfile preparation repair — 2026-10-20

Commit `3b0710189` changed Backup ^0.35 to ^0.36 and Query ^0.65 to ^0.66
without updating Cargo.lock, which still selected 0.35.1 / 0.65.2. Canic's local
release preflight correctly refused `cargo fetch --locked`; the shared runner
had not mutated versions or locks. Published Shared Tooling 0.25.9 at `85aded19`
explicitly separates dependency reconciliation from locked release fetching.
Reviewed that guidance; the adopted snapshot remains 0.25.8.

Narrowly resolved root Backup 0.36.0 / Query 0.66.0 and reconciled both independent
Blob consumer locks through offline metadata, preserving their declarations.
Pass: root `cargo fetch --locked` and locked/offline metadata for all three graphs.
Metadata evidence is `/tmp/canic-lock-*-verified.json`; no compile/full suite ran.
Graph consistency does not fix Blob's Memory/Platform identity mismatch described
below. Release remains blocked on a compatible published Blob graph. Commit
manifests and every affected lockfile together before source-bound validation.

## Authorized 0.112 release retries — 2026-10-20

Maintainer authorized `make release-minor && make publish` with fixes/retries.
Five attempts stopped before version preparation: uncommitted dependency inputs,
Backup/Host error-type mismatch, newly selected unfetched Auth, Platform/Host/Blob
type boundaries, then only Blob composition errors. Source fixes and incoming
dependency selections are committed through `0521f7925`; version remains 0.111.0.
No release tag, push or package upload occurred.

Fixed Backup error conversion across Host versions, explicitly enabled Platform
0.5 Candid service generation for existing contracts, and projected authenticated
ledger index/hash into the selected Host type. Pass: 10 backup artifact tests,
one generated-codec regression and 15 authenticated-ledger regressions.
Latest full Clippy attempt reports only `canic-blob-service` failures; subsequent
release gates/tests have not run. Evidence: `target/validation-runs/validation.Iz5tgA`
and `/tmp/canic-release-{backup-artifacts,codecs,ledger-receipts}.log`.

Blocked on published Blob: latest registry release 0.38.0 still uses Memory
0.49.4 / Storage 0.5.8 / Platform 0.4.1. Canic selects Memory/Storage 0.50.0 and
Platform 0.5.0. Public requests, runtime handles and principal types do not compose.
Do not bridge memory owners or remove the single-runtime gate. Approved evidence
is posted on [Blob issue #93](https://github.com/dragginzgame/ic-blob-storage/issues/93#issuecomment-6297546427).
Next: adopt a compatible published Blob graph, qualify dedicated/embedded adapters,
then resume the authorized release/publish cycle. Open 0.112 changelogs include
the local fixes; the complete batch is not ready to publish. Keep dependency
inputs fixed during validation; one concurrent Auth lock update required a restart.

## Cache executable fixture race — 2026-10-20

Replaced both direct executable-script writes in Host cache tests with published
Host Process 0.31.2's isolated-writer helper. This prevents parallel forks from
inheriting writable script descriptors, the documented ETXTBSY failure mechanism.
Production execution, failure propagation and no-retry assertions are unchanged.

Pass: four focused cache tests, 20 repeated four-thread runs (80 further passes),
Host library/test Clippy with warnings denied, scoped formatting and whitespace.
Logs: `/tmp/canic-cache-script-{tests,stress,clippy}.log`. Open 0.112 notes updated.
The reported test-fixture blocker is addressed; full-suite and serial PocketIC
qualification remain unrun here, so complete release readiness is unconfirmed.
No dependency changes, commit, push, release or deployment.

## Ordinary endpoint and trap gates — 2026-10-20

Resolved both failures in the 14:04 workspace ordinary-test log. Reviewed raw
fixture exports now require exact explicit decoder/encoder options as well as
their guard, kind and identity. Lifecycle, funding, provisioning and generated
endpoint traps delegate to the existing `IcOps::trap`; messages and rollback
semantics are retained. Removed the hidden direct CDK trap reexport and migrated
its generated consumers and compile inventory.

Pass: 15 endpoint/export/trap integration tests, 13 endpoint-expansion unit tests,
warning-denied Clippy for affected runtime/macro libraries and the runtime probe
with all features, focused integration-test Clippy, scoped formatting/whitespace,
and embedded-peer verification against a rebuilt Wasm fixture (no artifact change).
Logs: `/tmp/canic-ordinary-*.log`. Open 0.112 notes are updated. No full workspace
or serial PocketIC suite was run; complete release-batch readiness remains
unconfirmed. No commit, push, release or deployment was performed.

## Root fixture feature gates — 2026-10-20

Aligned Root baseline/topology orchestration, exports and orchestration-only
imports/constants with Fleet Registry fixture availability. Ordinary native tests
no longer compile references to the excluded Fleet Registry module. Moved pure
snapshot-target selection and its controller/uniqueness test into a separate
Root snapshot module, retaining ordinary coverage and updating runner classification.

Pass: targeted internal-testing Clippy with warnings denied in default and
all-feature configurations; exact native snapshot-selection and governed worker
partition regressions; scoped formatting and whitespace checks.
Logs: `/tmp/canic-fixture-gates-{default,governed,native,inventory}.log`.
Open 0.112 changelogs are updated. This clears the reported compilation blocker;
whole-batch release readiness remains unconfirmed. No full suite, PocketIC run,
commit, push, release or deployment was performed.

## Shared Tooling 0.25.8 adoption — 2026-10-20

Refreshed through the canonical exporter to upstream published main commit
`0a2d2ee3b5e0ae77755c2915ac28836d4d5ba359` (version 0.25.8; no current GitHub
release entry). Verified all 132 snapshot files. Catalog/tool pins are unchanged.
Validation now admits tools then runs existing full Clippy before invariant
fixtures/check/tests. Packaging, publication and fast-patch validation also gate
expensive work on Clippy after admission. Sequential recipes retain ordering
under parallel Make; the fast lane still avoids PocketIC.

Adopted retained Cargo metadata diagnostics and numbered `- Unreleased` support;
Canic's detailed-note test accepts both pending heading forms. Pass: snapshot
integrity, shared release-runner fixtures, canonical metadata fixtures with the
selected IC Build editor, parallel-Make ordering, publication/fast-lane refusal
fixtures, focused changelog test/Clippy, scoped ShellCheck/rustfmt/whitespace and
three workspace dependency-boundary checks plus the framework Wasm graph check.
Logs: `/tmp/canic-shared0258-*.log`. Automatic artifact-bound dependency-role
enforcement remains incomplete; existing explicit graph checks remain in place.

Preserved pending fixture, documentation and dependency work. Memory now resolves
0.49.3 rather than the earlier fixture-refresh lock's 0.49.2; fixture freshness on
that selection was not requalified here. Tooling changes and open 0.112 notes are
ready for review, not whole-release sign-off. No broad validation, real release,
publication, deployment or native macOS qualification was performed.

## Embedded allocation-peer refresh — 2026-10-20

Regenerated the checked-in allocation peer and `scripts/dev/managed-root-fixture.json`
through `refresh_embedded_root`, then passed the read-only `verify_embedded_root`
preflight. The artifact now binds current source, Rust 1.99.1 and the maintainer's
selected dependencies, including Auth 0.11.1, Metrics 0.14.11 and Wire 0.5.1.
Fetched missing locked packages; preserved the incoming Cargo.lock byte-for-byte.
The refresh's second build confirmed the artifact does not change its own build.

Logs: `/tmp/canic-peer-refresh.log` and `/tmp/canic-peer-verify.log`.
The reported stale-fixture blocker is resolved and the open 0.112 notes updated.
No full test suite, PocketIC suite, release or deployment was run. Whole-batch
release readiness remains subject to the remaining maintainer validation.

## Full workspace Clippy — 2026-10-20

Maintainer explicitly requested the full Clippy target. `RUSTC_WRAPPER='' make
clippy` now passes for the maintained workspace with all targets/features and
`-D warnings`. Fixed the redundant reference-memory clone and adapted Host
metadata/tree capture, build grouping and internal fixture discovery to Build
0.23 command recipes. Host retains existing budgets, successful/complete-output
admission, environment selection and error mapping; Build still owns rendering
and evidence decoding. No dependencies or lint allowances changed in this pass.

Pass: full Clippy, scoped rustfmt/whitespace, and the Host isolated-role Cargo
resolver regression. The regression's first run failed on sandbox registry DNS;
rerunning with `CARGO_NET_OFFLINE=true` passed using cached dependencies.
Logs: `/tmp/canic-full-clippy-pass3.log` and
`/tmp/canic-full-clippy-host-test-offline.log`. Earlier failed passes are retained.
Open 0.112 notes cover the Build integration. Existing contract-test and handoff
edits were preserved. The Clippy blocker is cleared; complete release validation
and deployment readiness are not established by this run. No Git/release effects.

## Component registry test lint — 2026-10-20

Shortened repeated Candid paths through a scoped import in the component registry
round-trip test, preserving all four assertions and clearing `too_many_lines`.
Pass: `canic-contracts` all-target/all-feature Clippy, the exact round-trip test,
scoped rustfmt and whitespace checks. Logs: `/tmp/canic-contract-roundtrip-*.log`.
No production change or lint suppression; routine test cleanup does not add a
changelog entry. This blocker is fixed; whole-batch release readiness remains
unqualified by these focused checks. No Git or release operations performed.

## Explicit fixture call codecs — 2026-10-20

Fixed all reported fixture errors from the latest check/Clippy log:
`root_funding_probe` and `icp_refill_stub` now encode their single Candid
arguments before Platform raw calls; funding responses use the same upstream
`decode_one` behavior as the removed response convenience method. Unit arguments,
wait modes, attached cycles and callback error branches remain unchanged.
The Root audit balance endpoint now supplies scalar Candid hooks, retaining
its local-controller guard and skipping quota of 10,000.

Pass: all-target/all-feature Clippy for all three affected packages, the funding
codec regression, scoped formatting/whitespace checks and standalone funding/refill
Wasm checks. Direct Root audit Wasm checking correctly refused the unmanaged
build path; its managed artifact and PocketIC effects were not rerun.
Logs: `/tmp/canic-probe-codec-*.log`. Native checks preceded the maintainer's
concurrent Build 0.23.0/Backup 0.35.1 update; standalone Wasm checks followed it.
Those manifest/lock edits are preserved and outside this fix's attribution.

The reported compile errors are fixed and open 0.112 notes updated. This slice
is ready for review, not whole-batch publication sign-off. No broad validation,
Git publication, version change or deployment was performed.

## Crypto-closure gate correction — 2026-10-20

The latest invariant failure required standalone `ic-canister-sig-creation` and
`ic-signature-verification` packages that published Platform 0.4.1 now packages
internally. Verified its registry source notice, feature wiring and Auth 0.11's
Platform calls before updating the gate's exact expected sets and Cargo fixtures.
The detection inventory still rejects unexpected separate signature providers;
ECDSA/BLS, SHA-256, duplicate-version and zero-auth checks remain enforced.

Pass: `make wasm-crypto-closure-gate` (fixtures and all 12 real canonical role
graphs, including nine zero-auth roles), focused ShellCheck and whitespace checks.
No runtime or dependency change; no broad validation or external deployment.
This reported blocker is fixed. The checkout was clean at starting commit
`e2f08b0cd47ba3155d1236a6a0917de80fffccb9`, superseding the prior dirty-work
prerequisite below. This fix and its open 0.112 notes are ready for review;
whole-batch release qualification is not established by this targeted gate.

## Blob 0.38 deployment qualification — 2026-10-20

Retained the maintainer's published Blob/contracts 0.38.0 and Platform 0.4.1
root selection. Aligned both independent Blob consumer locks on those versions
and Timers 0.29.5; each now resolves only Platform 0.4.1. Fixed the adapter's
expanded endpoint and installation macros to use Blob's owned Platform
principals. Updated managed and native decoding fixtures at their existing
external-principal boundaries. No protocol, storage or memory migration.
The Root fixture reuses its topology projection, clearing a function-length lint.

Prepared the governed Testkit 0.59 runner. The native ICP on the global PATH is
1.6.0; qualification explicitly selected the prepared repository ICP 1.7.0 with
`CANIC_TEST_ICP_BIN=/home/adam/projects/canic/.tools/ic/bin/icp`.

Pass: dedicated and embedded managed Blob PocketIC installation/permission/recovery
cases, including retained embedded application state; runtime observer-authority
PocketIC case; both native Blob installation decoding/limit cases; adapter and
internal-test Clippy; Rust 1.91 adapter Wasm cross-compilation; all three workspace
dependency-boundary checks; framework and both consumer Wasm normal/build graph
checks; scoped formatting, whitespace and unchanged-lock checks. Managed consumer
Wasms were built with the development toolchain, separately from the adapter's
MSRV library check. Logs: `/tmp/canic-blob38-*.log`.

The selected consumer graph checks require one Platform, executor, Memory and
Timers provider and exclude the checked tooling packages. General automatic
artifact-bound enforcement of dependency-role labels remains missing. The earlier
missing Testkit runner and old transitive Platform blockers are resolved. Historical
ablation evidence/tooling debts below remain outside release qualification.

This Blob slice and the open 0.112 changelogs are ready for review. The complete
pending release batch is not signed off for publication: the governed release
preflight rejects the checkout's uncommitted non-release work. Preserve that work;
commit authority remains with the maintainer. External release/publication versus
a named Fleet/environment is awaiting clarification. No live installed-IC,
native macOS or broad release qualification was run, and no Git publication,
version change or deployment was performed. An unrun broad suite alone is not a
new blocker; the pending source/target decisions are concrete prerequisites.

## Platform 0.4 / Testkit 0.59 cleanup — 2026-10-20

Retained published Platform 0.4.0 and Testkit 0.59.0. Removed the
`ic-testkit-platform` catalog/member alias and routed both reply-limit constants
through Platform 0.4 without changing any of their five budgets. Both independent
Blob locks now select Auth 0.11. Principal conversions preserve Canic's existing
DTO/storage identities; no wire, storage or memory migration was introduced.
Removed Backup's temporary error converter in favor of Backup 0.35's upstream
Host conversion. The maintainer advanced Query from 0.64 to 0.65 during this pass;
its ledger-evidence consumers now use Host Tools directly, while Query still
authenticates certificates and Canic retains spending authority.

Fixed Core's default/all-feature lint failures and matched `canister_test`'s
features to its configured delegated verifier. Three Host examples and the
historical ablation helper now use Clap; the helper's generated manifest includes
Clap. Materialized 151 historical evidence symlinks and verified identical resolved
bytes. Shared Tooling remains the previously adopted 0.25.4 snapshot.

The framework Wasm normal/build graph gate now explicitly excludes Testkit and
its catalog's tooling-only package identities. Raw stable structures remains a
fixture-only direct dependency, but its catalog identity is production because
IC Storage uses it transitively. General automatic admission from catalog labels
for every deployment target/feature selection and artifact is still missing;
the explicit framework graph check does not establish that broader claim.

Pass: Core all-target/all-feature and Host-example Clippy; 43 focused tests
(18 Auth sessions, 10 Backup artifacts, 15 ledger evidence); Host examples and
governed internal native test compilation; Rust 1.91 Core all-feature Wasm cross-compilation;
managed Wasm builds of `canister_test`, `intent_authority` and `runtime_probe`;
three workspace dependency-boundary checks; framework graph exclusions; scoped
Rust/manifest formatting, ShellCheck and whitespace checks. Logs:
`/tmp/canic-04-cleanup-*.log`. Earlier adoption logs are under
`/tmp/canic-platform04-*`; they precede the final Query selection.

Runtime qualification remains incomplete: the selected PocketIC case built all
three Wasms, then refused the absent governed server URL. The prepared Testkit
0.59 server runner is missing (`bash scripts/ci/testkit-server.sh --check`).
The historical ablation regression also lacks `wasm-validate`; its separate
manifest check finds a pre-existing missing baseline/sequence mismatch. Its
helper intentionally targets frozen product APIs, so a current-tree helper
compile is not historical qualification. No compatibility shim was added.
Blob 0.37 still brings Platform 0.2.2 transitively; no direct Testkit alias remains.

The cleanup and changelog are ready for review, not a whole-0.112 deployment
sign-off. No installed-IC, native macOS or broad release qualification was run.
No staging, commits, pushes, version changes, publication or deployment.

## Platform 0.3 and Shared Tooling 0.25.3 adoption — 2026-10-20

Verified Platform 0.3.0 in the official registry index and retained the maintainer's
root selection. Updated both Blob consumer catalogs/locks and application/role
scaffolds. Existing protocols and stored principal values use Platform's
`principal::ExternalPrincipal`, the same original registry type; CDK boundaries
convert explicitly to/from Platform's new owned principal. Generated authentication
and Blob endpoints participate. Existing Candid/Wire contracts, memory IDs and
storage encodings remain unchanged. AGENTS records the accepted 0.112 Wire deferral.

Bare funding, ledger, refill, sharding, intent, payload and performance fixture
endpoints now supply explicit Candid codecs. Removed the unused ICP stub Candid
export. Two lifecycle fixture imports use the existing storage facade; its cell
delegates to the same underlying cell and allocation, without a data migration.

Shared snapshot: 0.25.3 at `c55c1065c0061455dab7f0e2be8ddd04fdeb9910`, verified
against remote HEAD and exported canonically (131 payload files). Manifest and
lock candidate rendering now use the published IC Build 0.22.0 editor; normal
tool setup prepares it and release checks select it offline. Removed retired
Perl renderer/test references and the product release path's `cargo set-version`
requirement. Canic retains rollback, receipts, exact ownership and source checks.
A real manifest candidate was rendered only into `/tmp`; repository version
remains 0.111.0. No staging, commits, pushes or release execution.

Testkit 0.57 still exposes Platform 0.2's `DecodeLimits`. Retain
`ic-testkit-platform` only behind native `canic/testing` and in Host dev dependencies,
with all five budgets unchanged. The governed internal and runtime test consumers
now compile. Wasm-filtered resolution with testing enabled excludes Testkit and
this alias. Remove the retained dependency after
[Testkit #133](https://github.com/dragginzgame/ic-testkit/issues/133#issuecomment-6286884282)
aligns the public API. No upstream decoder or Testkit transport was copied.

Pass: production CLI/control plane; eleven direct-CDK fixture consumers; both
independent Blob consumers; governed internal and runtime test compilation;
Rust 1.91 Wasm Blob/funding/ledger/refill fixtures and all Core/control-plane features; strict
fixture Clippy; 88 focused Rust tests (macros, funding codec, endpoint codec,
scaffolds and release guards); release rendering/rollback/saved-source/preflight
fixtures; tool recipe admission/failure cases; snapshot/governance, dependency
boundaries in all three workspaces, layering and dependency-risk checks.
Logs: `/tmp/canic-platform-*.log`, `/tmp/canic-adoption-*.log`,
`/tmp/canic-shared-adapter-test.log`.

Remaining adoption gaps: 151 existing historical audit symlinks conflict with
Shared's new maintained-file rule; four maintained Rust utilities still parse
arguments manually (`wasm-ablation-build-artifact.rs`, Host's `build_artifact`,
`check_wire_contracts` and `local_fleet` examples). Preserved these for a scoped
cleanup. Dependency-role deployment exclusions still lack a general automated
gate; the concrete Testkit exclusion above was checked separately. Transitive
Candid and Platform 0.2 remain through published upstream crates; independent
consumer locks also retain older upstream selections. One executor 2.0.0 is
selected in the checked Canic Wasm normal/build graph.

This adoption is ready for review, not a whole-0.112 closeout or full baseline
compliance claim. Earlier PocketIC evidence below predates this graph. No new
PocketIC, installed-IC, native macOS or broad workspace qualification was run.

## Funding probe explicit codecs — 2026-10-20

All 20 Root funding probe RPC endpoints and its initializer now provide explicit
Platform codec hooks. They retain Candid argument/reply shapes and the existing
10,000 skipping quota for decoded arguments. Controller/role checks, attached
cycles, callback logic and intentional traps are unchanged.

Pass: focused native codec regression, package Clippy including tests with warnings
denied, Wasm cross-compilation check, scoped formatting and diff whitespace checks.
No PocketIC or installed-IC rerun; no broad validation. This fixes the reported
probe compile blocker, not a complete 0.112 release closeout. Changelog surfaces
are updated. A concurrent unrelated Cargo.lock change was preserved.

## Authority-recovery layering fix — 2026-10-20

The reported layering failure came from two direct Platform trap calls in
`workflow/runtime/authority_restore`. Both now use the existing `IcOps::trap`,
which delegates to the same Platform trap with the same message. Atomic rollback
semantics and the layering guard remain unchanged.

Pass: complete layering script, scoped Rust formatting and diff whitespace checks.
No compilation or broad validation rerun for this two-call substitution. The
reported layering blocker is resolved locally; this is not a complete 0.112
release closeout. Both changelog surfaces include the fix.

## Blob and Platform integration unblock — 2026-10-20

Adapted Blob 0.37 owned requests/replies and installation bytes to method-bound
Wire frames in the adapter, dedicated/embedded consumers and qualification callers.
External provider Candid remains unchanged. Ordinary Canic endpoints now generate
explicit Candid decoder/encoder hooks for Platform 0.2.2, retaining Candid method
registration, executor ownership and generic-value shapes. Lifecycle entries use
the existing synchronous raw adapter. Hub/Shard fixture stores use governed
Storage adapters; the controller-only fixture release retains its direct guarded
endpoint so it can release the application startup hold.

Raised Storage's minimum to 0.5.7: the older accepted minimum allowed independent
locks to retain incompatible Memory types. Updated both independent consumer locks
and aligned embedded Memory to 0.49. No sibling source edits or Git publication.

Pass: 46 macro unit tests; generated Candid byte-shape regression; strict codec
Clippy; two Blob installation tests including identity/direction, trailing and
byte-limit refusal; native Hub/Shard/adapter and both consumer checks; feature-enabled
qualification caller compilation; Rust 1.91 Wasm Blob-library check; three dependency
boundary checks; scoped formatting/whitespace. The new codec test is registered in
the workspace inventory. Direct fixture Wasm checks are refused by the managed-build
guard and are not qualification. Both dedicated and embedded Blob governed PocketIC
cases pass, including managed artifact builds, permissions, malformed installation
refusal and same-release
recovery; embedded qualification also preserves application-owned state. These
were two exact cases, not a full suite or installed-IC/native macOS qualification.
Prepared the pinned Testkit 0.57 CLI/PocketIC bundle with `make install-testkit-server`.
The runner uses `RUSTC_WRAPPER=''` and
`CANIC_TEST_ICP_BIN=/home/adam/projects/canic/.tools/ic/bin/icp` (verified ICP 1.7.0),
plus the exact `CANIC_TARGET_GOVERNED_CASE` name. The default PATH had ICP 1.6.0;
no global executable was replaced. Successful runs cleaned their private scratch.
Evidence: `/tmp/canic-oct20-*.log`, especially `blob-governed.log` and
`blob-embedded-governed.log`. These integration blockers are cleared; this batch
and its changelog are ready for review. Unrelated pending work has not received a
whole-0.112 closeout audit, so no full-release push/publish sign-off is claimed.


## Query certificate API unblock — 2026-10-19

Aligned mint receipt authentication with published Query 0.64 and Host 0.30.
Removed the certificate-depth override and redundant authenticate forwarding
function. Certificate CBOR now follows IC Agent codec recursion; certificate byte
budgets and generic ledger-value depth remain independently enforced. Added
excessive CBOR nesting coverage to the existing bounded ICP receipt regression.

Real Host target passes all 30 focused operator-mint receipt tests and all selected
ICP configuration tests, clearing the earlier YAML slice's Host compilation gap.
The current run selects Platform 0.2.2 and Testkit 0.57; no dependency edits were
made in this slice. Formatting and scoped whitespace checks pass. Logs:
`/tmp/canic-query064-receipts.log`, `/tmp/canic-query064-icp-config.log`.
Both open changelogs updated. No broad gate, installed proof, commits or release.
Complete 0.112 remains not push/publish-ready: Backup still selects Host FS 0.29
against direct 0.30, Blob still retains Memory 0.48 against direct 0.49, and the
other previously recorded fixture/installed qualification gaps need current checks.


## Host-aligned YAML parser — 2026-10-19

Replaced serde-saphyr with Host Tools' serde_yaml_ng 0.10.0 in raw ICP manifest
inspection and CI authority tests. One parsed document rejects duplicate keys,
unsupported tags and reserved << mapping keys before typed projection. Literal
quoted << keys are now also rejected (documented hard cut); quoted names and
ordinary aliases retain their behavior. The 1 MiB pre-parse bound and Host FS
bounded read remain. Host's resolved-project API cannot replace raw membership,
network and gateway checks without changing input ownership, so no wrapper or
unnecessary icp-project feature was added. Removed eight old parser packages;
added three replacement packages. Preserved incoming dependency updates.

Exact-source isolated qualification passes four parser regressions and both CI
authority tests, including current repository workflow files, on development Rust
and Rust 1.91. Strict isolated Clippy uses the repository's doc-comment lint rule.
Formatting, whitespace, direct boundaries and normal Wasm graph exclusion pass.
The real Host unit target remains blocked before tests by two Query 0.64 errors:
removed CertificateEvidenceLimits.depth and authenticate's removed depth argument.
Incoming Testkit 0.56 cleared the prior decoder identity errors. No full Host or
installed/macOS qualification claimed. Evidence: `/tmp/canic-yaml-adoption/`.
Both changelogs and ICP integration docs updated; no commits or release. This
slice is reviewable, but the complete 0.112 batch is not push/publish-ready.

## Platform 0.2.1 partial adoption — 2026-10-19

Verified published 0.2.1. Root already selected 0.2/0.2.1; raised the minimum to
0.2.1 and aligned both independent Blob catalogs and locks. Their required path
closure also selects Blob 0.36.1, Memory 0.49 and Wire 0.4.9. Preserved unrelated
root lock updates. Removed only the unused facade CDK export_candid re-export and
its reference inventory entry; canonical Candid export_service remains.
Contracts/Core and facade reference-surface native checks pass. Contracts also
passes Rust 1.91 Wasm checking. All three direct dependency-boundary checks pass.
Generated user_hub check fails: Platform 0.2 requires explicit codecs for plain
Candid endpoints; two fixture imports also use removed Memory substrate exports.
No protocol conversion or locally invented compatibility runtime was added.
Migration guidance tracked at https://github.com/dragginzgame/ic-platform/issues/36;
PocketIC export issue #27 is still open and 0.2.1 still lacks RawMessageId.
Root retains Platform 0.1.18 via Auth, Backup, Blob, Host Tools, Query, Testkit and
Timers; independent locks retain 0.1.17 transitively. These retain upstream CDK
and Candid; direct declarations being clean is not a Candid-free Wasm proof.
Root's externally updated Storage 0.5.7 cleared the previous Core Memory mismatch;
Blob/fixture graph qualification and Backup Host 0.30 mismatch remain separate.
Evidence: `/tmp/canic-platform021/`. No installed-IC/native macOS qualification,
broad suite, commits or release. Complete 0.112 is not push/publish-ready.

## IC Host 0.30 adoption review — 2026-10-19

Verified all five published Host packages latest at 0.30.0; the maintainer's
catalog and root lock already select them directly. No dependency edits needed.
Published source comparison shows only FS capability/TemporaryFile feature gating;
Canic uses neither, so no opt-in added. Backup's resolved FS 0.30 feature tree
selects neither backend. Both independent Blob locks contain no Host packages.
Focused `cargo check --locked --offline -p canic-backup` fails with two checksum
error conversion E0277s: Backup 0.33 still exposes Host 0.29 types. Extended the
existing https://github.com/dragginzgame/ic-backup/issues/62 with exact evidence.
Build 0.21, Query 0.63 and Testkit 0.55.2 also retain Host 0.29 transitively; those
boundaries were inventoried, not claimed to fail. No local compatibility adapter.
Evidence: `/tmp/canic-host030-check.log`, `/tmp/canic-host030-features.log`.
Updated both open changelogs. Full adoption/0.112 readiness remains blocked by
Backup alignment plus the separately recorded Memory and installed proof gaps.
No broad suite, native macOS qualification, commits, release or deployment.

## Completed-source reset issue closeout — 2026-10-19

Closed #208 and #204 against remote main
`4caeec25db7d81f6121db1c95ba7a2d91d319c60`. Obsolete completed-preparation/reset,
completed-source generation and terminal-retirement paths are deleted. Operation
locking and retirement no longer read or reject the old preparation journal.
Remaining activation recovery and paid-effect admission are distinct maintained
owners. Posted immutable source evidence; no claim of full installed reset proof.
Eleven verified closures across recent passes. Reviewed #56, #94 and #322 remain
open: no expected-role verifier parameter, optional-init continuation still used
by Store/local lifecycle, and standalone fallback still lacks Component Spec.
No source/dependency changes or runtime tests. Evidence: `/tmp/canic-issue-closeout/`.

## Published Memory/Blob alignment blocker — 2026-10-19

Registry confirms Memory 0.49.0 and Blob Storage/Contracts 0.36.1 are latest.
The maintainer's root catalog/lock already selects these; preserved selections.
Published Storage 0.5.6 and Blob 0.36.1 both require Memory ^0.48, retaining a
second Memory 0.48.2. Focused `cargo check --locked --offline -p canic-blob-service`
fails with 29 Core E0308 errors passing 0.49 RuntimeMemory to 0.48 Storage adapters.
Evidence: `/tmp/canic-new-memory-blob-check.log`. Owning tracking:
https://github.com/dragginzgame/ic-storage/issues/17 and
https://github.com/dragginzgame/ic-blob-storage/issues/93.
Blob 0.36.1 includes the exact-path provider catalogue dispatch fix, but installed
acceptance is not established. Both independent consumer locks remain on Blob
0.35/Memory 0.48; align after published Storage/Blob share Memory 0.49. No source,
manifest or lock edits this pass; no local overrides or second runtime added.
Complete 0.112 remains blocked. No broad validation, release or deployment ran.

## Anonymous-token and scope closeout — 2026-10-19

Closed #107 and #128 against remote main
`4caeec25db7d81f6121db1c95ba7a2d91d319c60`: public delegated-token issuance,
self-issuable application-scope policy and authenticated token guard are removed.
The cited user_shard token consumer is gone. Current local application admission
explicitly denies anonymous callers and otherwise fails closed pending portable
Auth integration. Closures establish removal, not complete Auth qualification.
Source-bound comments retain this limit. Nine verified closures across recent
passes. #71 remains open: its max(explicit, configured) epoch-floor correction and
regression are committed, but its requested passing focused check remains
unverified in this pass. No source/dependency edits or runtime tests. Evidence:
`/tmp/canic-issue-closeout/107.md` and `128.md`.

## Additional token-path closeout — 2026-10-19

Closed #55 and #58 against remote main
`4caeec25db7d81f6121db1c95ba7a2d91d319c60`. The delegated-token issuance/lazy-repair
predicate and proof-skipping positive-cache verifier are removed. Maintained
preparation is role attestation; the remaining token verification helper is
native-test-only Root proof qualification. Posted immutable source evidence and
confirmed both CLOSED. These are removal-based resolutions, not claims that Root
renewal liveness (#27) or portable-session qualification is complete. #57 remains
open because Root proof infrastructure remains and its binding concern needs
separate verification. No source/dependency changes or runtime tests this pass.
Seven issues closed across the recent closeout passes; this is not a full backlog
audit. Evidence: `/tmp/canic-issue-closeout/55.md` and `58.md`.

## Test-discovery closeout — 2026-10-19

Closed #332 against remote main `4caeec25db7d81f6121db1c95ba7a2d91d319c60`.
The missing admission proof is registered; the ordinary runner enables the
feature-gated native selectors; actual compiled --list/--ignored identities are
reconciled with exactly one native/governed owner and explicit opt-in handling.
Relevant source/runner files match HEAD. Posted immutable source evidence and
confirmed CLOSED status. This does not establish a passing current suite while
Platform #27 blocks internal compilation.

Other reviewed findings remain open: #342 still lacks a Wasm MSRV CI step;
#383 still has the Root JSON manifest parser; #351 retains fixed-size metadata
on variable-length records; #374 lacks caller-owned cycle-request identity;
#358 still defaults network classification to Local; #334 still uses a textual
trap guard. #380 has improved keyed logout code but its maximum-state/installed
acceptance was not established here. No new source/lock/test or release effects.
Evidence: `/tmp/canic-issue-closeout/332.md`. This remains a focused backlog pass.

## Historical auth issue closeout — 2026-10-19

Closed #345 and #420 after inspecting committed command unions, auth DTOs,
workflows and maintained docs on remote main
`4caeec25db7d81f6121db1c95ba7a2d91d319c60`. The public delegated-token preparation,
caller-ext issuance contract and token-based session establishment through
canic_command are removed. Root delegation proof infrastructure remains.
Both issues now have source-bound disposition comments and confirmed CLOSED
state. These closures certify removal of the reported surfaces, not complete
portable-auth or downstream deployment qualification.

Kept #418 open: current portable Claims/Signature/Certified failures still use
the general security-invalid projection, preserving its diagnostic follow-up.
Reviewed tooling-adoption candidates with pending native acceptance remained open;
no native evidence was inferred from shared upstream passes. No code, dependency,
changelog or Git-ref changes and no test runs. Evidence: `/tmp/canic-issue-closeout/`.

## Issue closeout — 2026-10-19

Verified remote main equals local HEAD `4caeec25db7d81f6121db1c95ba7a2d91d319c60`.
Closed #501: public/internal fixtures consume Testkit's canonical URL with bounded
explicit Connect; earlier managed installation evidence is recorded on the issue.
Closed #355: the dead approved-manifest resolver/registrations are deleted on main;
maintained component installation uses exact Root release-set evidence. General
support-export scope remains #366. Posted immutable source links and limits on
both closures, then confirmed their CLOSED state.

Kept #562 open and documented its missed standalone backup manifest validator:
`backup/manifest/mod.rs::validate_manifest` still reads unbounded text. Kept #515
open: although the old file-lock module is gone, the current JSON FIFO test still
calls Darwin-unavailable rustix::fs::mkfifoat under cfg(unix). These are the next
two small, concrete corrections; native macOS qualification remains separate.
No source/lock changes or test runs for this closeout. Reviewed candidates with
outstanding native/installed acceptance stayed open. This was a focused review,
not a complete backlog audit. Evidence/comments: `/tmp/canic-issue-closeout/`.

## Operator and build-cache qualification — 2026-10-19

Confirmed published Platform remains 0.1.18, Blob remains 0.35.0 and Platform
PR #28 remains open. No unpublished overrides or boundary bypasses were added.

Additional focused checks pass on the current published integration:
- 52 CLI backup tests: effect-free live-create refusal, dry-run persistence,
  execution/layout locking, listing, verification and protected pruning.
- 42 CLI restore tests: preparation, journal/receipt validation, dry-run command
  generation, completion/failure reporting and retry handling. The real upload
  process-death test remains ignored because it requires an explicitly disposable
  ICP environment and prepared restore journal; no installed-IC evidence claimed.
- 10 Host build-cache tests: explicit target paths, declaration/runtime output
  separation, exact failure propagation, wrapper retention, locking and stable
  tool observations.
- One Host artifact-reuse environment regression: credential and shell-depth
  changes preserve reuse while actual build-input changes invalidate it.

Testkit changed externally to 0.55.2 before the Host checks. Its published
changelog covers Host tooling adoption; it still forwards Platform's incomplete
PocketIC exports. Preserve this selection. The exact Testkit server tool will
need setup for 0.55.2 before the next governed PocketIC attempt.

Evidence: `/tmp/canic-deploy-next/`. No new source defect found in these checks,
no production/test code or changelog change, and no broad suite or deployment ran.
The next useful acceptance remains published Platform #27 exports followed by
Root restoration; published Blob #88/#89 fix followed by dedicated/embedded Blob
and production Root-binding installation journeys. Complete 0.112 is still not
qualified for push/release. Existing open changelog entries remain unchanged.

## Independent deployment preparation — 2026-10-19

Backup 0.33.0 now passes all 260 Canic Backup library tests with Host 0.29.0.
The former Backup checksum identity mismatch is cleared by actual consumer
qualification; this does not enable or qualify coordinated live backup.

Completed the Build 0.21 fixture environment correction beyond the Root path:
embedded-peer verification, bootstrap Store, release-artifact identity and
host-input guard specifications now use the same internal workspace-owned
capture helper. Only ambient CARGO_TARGET_DIR is omitted; the typed build target,
other captured environment and explicit fixture overrides remain unchanged.
Moved the helper out of the PocketIC-only artifact module so embedded fixtures
share it without depending on that implementation. Updated the equivalent Host
infrastructure-input test specification.

Focused validation: Host's infrastructure-input preparation regression passes
both normally and with CARGO_TARGET_DIR explicitly set to the repository target.
Warning-denied Host library/test Clippy passes with Build 0.21, Host 0.29,
Testkit 0.55.1 and Platform 0.1.18. Timers changed externally to 0.29.2 during the
pass; preserved the lockfile and downloaded that selected package for Host checks.
Formatting and whitespace checks pass. Internal fixture compilation and installed
Root restoration remain blocked by Platform #27 / PR #28, as recorded below;
these Host checks do not qualify those fixture paths. Embedded artifact bytes
were not refreshed. Evidence: `/tmp/canic-deploy-local/`.

No broad gate, deployment, release, Git publication or dependency downgrade ran.
Complete 0.112 remains unready pending the missing Platform exports and the
Root/Blob installed journeys. Both existing open changelogs are updated for this
slice; the complete release batch is not yet qualified.

## Root fixture cleanup and current qualification blocker — 2026-10-19

Removed the unused `activate_managed_fleet` driver and the redundant
`PreparedManagedRoot.wasm_store` / `installation_controller` fields. Shared
managed Fleet bootstrap remains the sole activation path; required install and
controller values remain local to preparation. With Build 0.21.0, focused
internal-library and `pic_root_baseline_restore` integration-target checks
passed, clearing the previous dead-code and Build/Host compilation blockers.

The governed restoration run then exposed Build 0.21's refusal of ambient
`CARGO_TARGET_DIR` in WasmBuildSpec inputs. Updated shared and Root fixture
specification construction to exclude that output-only variable; explicit
specification targets and actual build-command target selection remain intact.
All other environment inputs and fixture overrides remain captured.

The checkout changed to Backup 0.33.0 and Testkit 0.55.1 during qualification;
preserved those selections. Prepared the exact published Testkit runner through
`make install-testkit-server`. Use `CANIC_TEST_ICP_BIN="$PWD/.tools/ic/bin/icp"`
for the governed proof: the repository tool is the pinned ICP 1.7.0, while the
current global PATH selects 1.6.0.

The current locked graph cannot compile the restoration proof or the focused
internal-library Clippy target: Testkit 0.55.1 forwards Platform 0.1.18's explicit
PocketIC surface, which omits `common::rest::RawMessageId`. Canic's
`managed_component_group::settle_submitted_call` requires this exact ingress
handle to preserve terminal-call settlement. Added current consumer evidence to
[Platform #27](https://github.com/dragginzgame/ic-platform/issues/27#issuecomment-6267002030);
[PR #28](https://github.com/dragginzgame/ic-platform/pull/28) remains open.
No local DTO replacement, direct PocketIC bypass or dependency downgrade added.

Next: adopt the published export fix and rerun the exact governed Root
restoration proof, then its focused Clippy check. The environment correction
has formatting/whitespace validation but no completed installed proof yet.
Earlier successful compile evidence predates the Testkit update. Backup 0.33
selects Host 0.29 identities, but Backup compilation was not requalified in this
Root-only pass. Do not repeat the historical Backup 0.32 mismatch as current
without checking it. Blob installed journeys remain separate outstanding work.
Both open changelogs updated. Complete 0.112 is not ready to push; no broad gate,
native macOS, installed-IC qualification, Git publication or release ran.
Evidence: `/tmp/canic-root-dead-code/`.

## Host 0.29 adoption boundary — 2026-10-19

Verified all five published Host packages at 0.29.0 and inspected shipped APIs.
The checkout was externally updated to that family and Auth 0.10.4 during this
pass; preserved those selections. The attempted old-version targeted update
found no Host Text 0.28.1 selection and made no adoption claim. Current manifests
and locks already select 0.29 directly; no downgrade or sibling override added.

Focused library checks confirm two Backup checksum-conversion errors and nine
Host/Build output-limit, child-spawn and Cargo-observation type errors. Published
Build 0.20.0 and Backup 0.32.0 still expose Host 0.28 identities. Recorded the
current evidence on Build #9 and opened Backup #62. Producer releases aligning
these public identities are required for full 0.29 integration. Tools also
exchanges Artifacts WasmFacts, so it is not wholly independent of that boundary.

Moved internal managed fixture gzip encoding/decoding to Host Artifacts and
removed canic-testing-internal's direct flate2 dependency. Preserved level 6 and
existing trusted-fixture size allowances; decoding now rejects trailing data or
additional members, matching retained artifact framing. A disposable registry-only
probe confirms old/new encoded byte equality and decoded payload equality for
four inputs, plus trailing-data refusal. Changed Rust formatting, dependency
boundaries and internal Machete checks pass. Full internal compilation and
installed restoration remain blocked; the probe does not replace those checks.

Retain Host tests' independent flate2 fixture construction, Ciborium's Canic
record/hash codecs and Prost's strict ICP transfer projection. Host's Registry
Protobuf projections and ICRC evidence helpers do not replace that ICP codec.
SignalActions could replace signal-hook only after reviewing process-global
handler ownership, chaining and default-action semantics; no blind replacement
was made. No generic Host API currently replaces all CBOR callers.

Remaining release work: aligned Build/Backup publication; removal/qualification
of the previously reported Root fixture dead code and restored-baseline proof;
published Blob provider fix (latest observed Blob 0.35.0, PR #89 merged) and the
pending dedicated/embedded/Root installed journeys. Full 0.112 remains blocked.
Both changelogs updated. No broad suite, native macOS, installed-IC or Canic Git
publication/release ran. Evidence: /tmp/canic-host29/.

## Complete root dependency-catalog review — 2026-10-19

Audited all 58 workspace dependency entries against Cargo metadata, member
normal/build/dev/target declarations, source references and macro expansion.
Every catalog entry has active consumers; no root entry can be deleted outright.
Removed five unused canic-tests declarations: canic-control-plane, ciborium,
ic-build, sha2 and tokio. Shared internal test support already selects the exact
Control Plane features. The lockfile changes only those five dependency edges;
no package versions or transitive packages changed.

Retained Platform declarations used by finish!'s generated Candid export and the
dedicated Blob shell's macro-expanded Canic/Platform references. Retained
serde_bytes used in Serde attributes. Runtime sha2 0.10 and host sha2-host 0.11
serve separate active consumers; HMAC uses the latter's digest identity.
serde_path_to_error remains used by native config diagnostics. A repository-wide
Machete scan also encountered an archived report manifest with obsolete workspace
inheritance; a second scan restricted to active package directories completed.
Its remaining warnings were verified attribute/macro consumers, not deletions.

The focused canic-tests test-target check exposed Host Tools 0.28's missing
explicit candid feature. Enabled it alongside ic-interface in canic-host; the
retry compiles Host and then stops in pending Root fixture work on unused
PreparedManagedRoot.wasm_store/installation_controller fields and the unused
activate_managed_fleet helper. Do not claim the integration-test targets passed.
Machete passes for both changed packages; dependency boundaries and whitespace
checks pass. Both open changelogs updated. Complete 0.112 remains unqualified;
no broad suite, installed acceptance, Git publication or release ran.

Full per-entry evidence/dispositions: /tmp/canic-dependency-audit/catalog.json.
Compiler logs and before-images are beside it. Continue by resolving the pending
Root fixture dead code and rerunning its targeted compile/restoration proof;
keep Blob installation qualification separate.

## Platform ECDSA adoption — 2026-10-19

Verified published Platform 0.1.17 from crates.io and inspected its shipped ECDSA
exports/migration guidance. Raised the minimum in all three workspace catalogs,
aligned both independent Blob consumer locks, and replaced every direct K256
import with ic_platform::ecdsa. Removed the root catalog/Core dev declaration;
Core's Platform ecdsa feature is dev-only. Existing prehash construction, high-S
rejection/normalization and signature types remain unchanged. All three adopted
dependency-boundary policies now forbid direct k256, including aliased and
normal/build/dev/target-specific declarations. No missing Platform API was found.

Qualification: 124 focused Core auth tests pass on Auth 0.10.3/Platform 0.1.17;
Core's auth-chain-key-ecdsa library cross-compiles for wasm32-unknown-unknown on
Rust 1.91.0. All three boundary checks, changed-Rust formatting and whitespace
checks pass. Production feature inspection excludes the new Platform ecdsa and
ed25519 dev features. K256 0.13.4 remains transitively owned by Auth, IC Agent,
IC secp256k1 and Platform; direct dependency removal is not Wasm absence.
Consumer lock refresh also aligns their Timers with the framework's existing
0.29 requirement (resolved 0.29.1). Root lock regeneration normalized existing
Windows dependency edges; no unrelated package version upgrade was requested.

Preserved incoming Build 0.20, Backup 0.32, Auth and Host updates and all pending
Root/PR work. This slice does not requalify Host/Build integration or clear the
pending Root restoration and Blob installed acceptance. Complete 0.112 remains
unqualified for publication; both open changelogs include this slice. No broad
suite, native macOS, installed-IC, Git publication or release ran. Evidence and
before-images: /tmp/canic-platform-crypto/.

## Older review findings reconciled — 2026-10-19

Closed #436, #126 and #285 after verifying their implementations on remote main
4caeec25db7d81f6121db1c95ba7a2d91d319c60. #436 uses one Registry result envelope
and has success/typed-rejection query regressions; its historical three-test
qualification remains scoped to the October 13 dependency graph. #126's invalid
ComponentInstanceId Storable bound was removed with contract extraction; current
registry keys retain raw 32-byte identities. #285 now selects an explicit built
release or exact retained installed declaration, propagates selection errors and
rejects ambiguous roles. Its committed regressions cover target selection and
protocol-binding mismatches. Closure comments link immutable implementation and
regression evidence; no new installed or whole-release acceptance is claimed.

Read 12 further candidate bodies and discussions after inventorying all 466 open
issue titles at the start of this pass; the backlog is not exhaustively assessed.
#71's epoch-floor correction is on main, but the attempted focused offline Core
regression stopped before compilation: selected ic-auth 0.10.2 is not cached.
Keep #71 open pending its requested validation. #418 retains a consumer diagnostic
acceptance requirement; #362/#364/#367 still have relevant source/doc paths.
#447/#463 require additional qualification review. #420/#426 were inspected but
not fully adjudicated. Preserve the existing pending implementation batch.

Next: qualify #71 when dependencies are available, finish the prepared #566 and
PR #605/#606/#597 changes, then review #604 against the current Shared snapshot.
Evidence is in /tmp/canic-close-pass2/. This pass changed no code, lockfile, Git
ref or changelog; only this handoff and three GitHub issue dispositions changed.

## Verified issue closures and memory guide correction — 2026-10-19

Inspected the latest 100 open issue titles and read 33 candidate descriptions and
all returned comments. Closed #559, #576 and #577 after verifying delivery on
remote default branch 4caeec25db7d81f6121db1c95ba7a2d91d319c60. #559's obsolete
reservation callback is absent; all three committed locks each have one Memory
identity, and its earlier 27-case/three-installed-journey evidence remains scoped
to the graph recorded in that issue. #576/#577 have committed typed idle/missing
review errors before external calls or journal publication, with four plus one
passing regressions retained in /tmp/canic-issue-triage/. No new runtime acceptance
or whole-release readiness is implied by these closures.

#566 remains open: corrected both memory guides locally to use current stable
keys and explain ledger-assigned manager IDs. Removed false fixed-ID assignments
for activation, restore/admission, provisioning and async-job recovery; preserved
historical fixture observations. Both changelogs updated. Exact keys match the
Core/Control Plane catalogs; changed-file links and whitespace pass. This is
source/documentation qualification, not a new installed memory report.

Do not close #538/#541/#543/#554: source inspection still finds their reported
paths. #509/#515/#528/#529/#532/#540 and shared-tooling adoption candidates retain
explicit native or current-graph acceptance gaps; producer publication or code
presence alone is insufficient. #482/#533 request a policy change and are not
closed merely because today's policy remains reinstall-only. Follow #566 with
focused review/delivery, then seek matching hosted evidence for already-applied
adoptions. Evidence: /tmp/canic-issue-closure/. No code, locks, Git refs or release
actions changed in this pass; only issue closures and local documentation edits.

## Independent backlog work — 2026-10-19

Reviewed all 18 open PR titles/statuses and the newest 50 open issues, then read
#605, #606, #588 and #597 descriptions/discussions/reviews and diffs. This is not
an exhaustive review of the remaining issue backlog. Closed issue #547 and PR
#588 only after fetching their implementation from GitHub default branch
4caeec25db7d81f6121db1c95ba7a2d91d319c60: diagnostic range classification and
explicit hook-install consumer selection are already delivered there.

Applied #605's timer guide corrections and #606's IEC chunk-upload logging locally,
retaining their intended scope and updating both 0.112 changelogs. The current
locked Canic Wasm graph selects one Timers 0.29.0 provider. The focused Core
formatter regression passes, as do scoped Rustfmt, local documentation links and
whitespace checks. This does not qualify installed log output or native macOS.

#597's extractor installation/PATH work is already present, but its final lint
correction was missing: local Make now includes tool-versions.env in ShellCheck,
and the catalog has its Bash directive. Focused source-following ShellCheck and
the development-recipe/extractor/CI-PATH fixture pass (installer effects substituted).
No real tools were installed. PRs #605/#606/#597 and issues #555/#552 stay open
until their remaining changes land on the default branch. No commits, pushes,
merges, branch deletions or release actions were performed.

Next: finish delivery/review of these prepared fixes; inspect #604 against the
current Shared snapshot without reverting its revision; qualify Root once the
Host/Build/Backup graph aligns. #545 needs current installed timer evidence, not
closure based on its older report. Evidence/before-images: /tmp/canic-queue-605-606/.

## Root bootstrap consolidation prepared — 2026-10-19

The Root topology fixture now calls the managed Fleet bootstrap helpers with its
profile config and retained artifacts. It stages the exact release manifest,
installs/synchronizes a Coordinator, prepares the Component Registry, provisions
configured Components and activates Root. Removed its duplicate ordinary-template
publisher and unused chunk/version settings. The full topology profile includes
its previously omitted index/test roles. Coordinator snapshots join the baseline;
blank pool canisters use the Fleet owner's shared custody reset rather than
attempting snapshots of canisters without code. The restoration proof now
uninstalls both Root and Coordinator before reacquisition.

This implementation is NOT qualified or a resolved blocker yet. The current
external Host 0.28 selection blocks both attempted focused commands before the
changed fixture compiles. The internal native test fails in canic-backup with two
DirectoryChecksumError conversions: Backup 0.31.4 uses Host 0.27 identities.
The internal library check reports ten Host errors: removed ic_host_tools::candid
and Build 0.19's Host 0.27 process/input types conflicting with Host 0.28.
No dependency rollback, shim or upstream override was introduced. Formatting and
whitespace checks pass; native and PocketIC acceptance remain outstanding.
Evidence and before-images: /tmp/canic-root-bootstrap/ (native.log, check.log).
Once the selected Host/Build/Backup APIs align, compile this fixture, run the
snapshot-target unit and `make test-pocketic-case CASE=pic_root_baseline_restore`,
then address any installed bootstrap failure. The earlier activation failure is
historical evidence, not proof that this replacement now passes. Blob #89 remains
an independent upstream blocker. No broad gates or Git/release actions ran.

## Focused restore and Build 0.19 qualification — 2026-10-19

Continue 0.112 with the framework Candid protocol unchanged and live coordinated
backup unavailable. Preserve the external Build 0.19 and Timers 0.29 selections.
The Build-only lock refresh preceded a separate external dependency refresh;
qualification below selects Build 0.19.0, Timers 0.29.0, Metrics 0.14.4,
Platform 0.1.15, Auth 0.10.0, Testkit 0.54.1 and Blob 0.35.0.

Host now uses typed CargoInvocation/execute_cargo for configured roles and all
infrastructure builds. Graph arguments have one renderer; explicit environments,
features, profiles, declaration flags, wrapper selection and primary failed exits
remain intact. Successful output requires stable tool observations. Removed the
unused raw profile-argument helper. Timers' wide counters project exactly to the
existing u64 contracts: unrepresentable runtime/public process reports refuse,
and unified metric rows omit values that cannot be represented. No wire or
storage schema change was introduced.

Added destructive-state repeated-acquisition proofs for both baseline pools.
Fleet restoration passed on the earlier Build 0.18 graph (111s): same instance,
restored installed code/controllers, non-regressed time and validated bindings.
Root preparation exposed two corrected fixture defects: role package paths must
resolve to Cargo names before cache construction, and the capability test App
requires Root-signature and configured chain-key verification. The issuer-only signature-store feature
also now imports Principal independently of Root signature creation.
After all artifacts built, the importer exposed a stale `.icp` output lookup.
It now uses the effective qualified release identity; duplicate fixture path
settings are removed and the shell size check uses the builder-returned path.

Current focused evidence in /tmp/canic-pooled-proof/: 102 Host build-module cases
pass across the main run and focused retries; three existing opt-in cases remain
ignored. The first run's canonical-path assertion was corrected and its registry
access failure passed with offline policy propagated to subprocesses. Three Core
counter/health cases pass, including exact u64 cap, narrowing, aggregate overflow
and flagged upstream overflow. Host library/tests with local-fleet, the governed internal library/test harness,
and the selected Root integration target pass strict Clippy. Core cross-compiles
to Wasm on Rust 1.91.0. ShellCheck with sourced helpers, scoped formatting/diff
checks and the derived test inventory pass.
Root's latest retry (root-8.log) builds/imports artifacts and installs Root/Store,
but rejects PrepareFleetActivation with typed STATE_UNAVAILABLE (140). Its
legacy fixture stages ordinary TemplateManifest entries only, omitting the
current RootStoreReleaseSetManifest/BootstrapStore and Component Registry
publication flow used by fleet_registry::baseline. Root bootstrap consequently
still reports missing staged release roles. Do not weaken activation checks or
claim Root pool restoration passed: it never reached snapshot capture. Reuse the
working managed Fleet bootstrap owner for the Root topology recipe, preserving
its selected roles, feature/config authority and child metadata. This is local
harness work, not a demonstrated missing upstream API. Fleet's final-graph
repeated-acquisition proof passes (114.59s; fleet-final.log), including actual
Coordinator, Root, Store and Component setup, same-instance snapshot restoration,
installed code/controllers, current time and Fleet bindings.

Blob PR #89 remains open/unpublished: Linux and MSRV passed; ARM macOS also passed and Intel macOS
was still running at the latest observation. Canic retains published sources,
without an override. Dedicated/embedded Blob installation and production Root
binding acceptance remain behind that publication. The complete 0.112 batch is
not push/publish-ready. A precise continuation update for existing issue #548 is
prepared at /tmp/canic-pooled-proof/issue-548-comment.md. Automatic approval
review rejected posting uncommitted fixture details without payload-specific
approval; no comment was published. Open changelogs cover these changes; no broad suite,
Canic Git publication, release, deployment or installed-IC qualification ran.

## 0.112 blocker qualification — 2026-10-19

Keep Canic's framework protocol on Candid for this release; the framework-wide
IC Wire migration is deferred. Live coordinated backup remains unavailable.

The Build 0.18 staging integration, Storage 0.5.3 minimum/consumer 0.5.4 locks,
embedded Memory 0.48.2/Storage counter and archived audit-manifest repairs remain
applied. Earlier evidence is retained in /tmp/canic-112-unblock/.

Reconciled Blob-specific #603 and raw endpoint prerequisite #591 into the current
Platform tracking-executor graph. The adapter now uses upstream ServiceWorkflow,
Wire tenant codecs and CBOR Blob requests/installation bytes. Removed the local
workflow dispatcher and account/gateway adapters. Framework and external provider
certificate contracts remain Candid. Updated both consumer installation producers,
Root binding fixture and bounded decoding regressions; no storage migration.

The dedicated managed journey compiles coordinator, store, Blob and Root, but
refuses the artifact before installation: the provider certificate is absent from
its generated Candid, raw declarations were stripped, and the new Timers internal
dispatch was unrecognized. Local fixes retain raw declarations in private IC
metadata, compare them across optimization and recognize the exact Timers update.
A test fixture's section length now uses LEB128 beyond 127 bytes.

The provider omission reproduces independently with published Blob Contracts
0.35.0 and Platform 0.1.15. Filed [Blob #88](https://github.com/dragginzgame/ic-blob-storage/issues/88)
and submitted [Blob PR #89](https://github.com/dragginzgame/ic-blob-storage/pull/89)
from an isolated checkout. Exact-path catalogue dispatch fixes expansion order;
the compiler-backed exact provider contract regression and strict Clippy pass.
Canic retains published dependencies without an override. Adoption requires the
corrected upstream package to be published; the PR is not a published prerequisite.

Local evidence in /tmp/canic-blob-integration/: 46 macro unit tests, 25 artifact
regressions and two bounded installation decoder cases pass. Macro, Blob adapter
and Host libraries pass strict Clippy. The pinned ic-wasm preserves exact private
raw metadata in a focused executable probe. The failed managed attempt remains
separate from these passing checks; dedicated/embedded installed recovery,
production Root binding and changed pooled restore still need acceptance on the
final graph. Pooled restore needs a focused repeated-acquisition proof; a first
lease alone does not exercise Testkit's reuse branch. Do not substitute the broad
instruction audit for that check. No installed qualification follows from compilation.

The complete 0.112 batch is not push/publish-ready. Both open changelog views and
Blob composition docs describe the pending integration. No broad suite, native
macOS, installed-IC, Canic commit/push, release or deployment ran. Only the scoped
upstream PR branch was committed and pushed; sibling working trees are untouched.


## Build/Testkit harness and Blob integration blockers cleared — 2026-10-18

Migrated internal fixture builds to Build 0.17.6 typed Cargo selection,
explicit complete process environments, bounded cache retention and explicit
Host child custody. Batch reports retain failure phases and partial timings.
Testkit 0.54.1 now owns pooled snapshot restoration; removed duplicate recipe
restore callbacks and obsolete recipe/receipt identities. Preserve top-up
minimums, current-time reset policy, readiness, authority checks and allocation
cleanup. Installation, diagnostics and retry fixtures use current passive data.

Blob 0.34.0 owns the platform-version checkpoint; removed the local Host wrapper
and duplicate counter, preserving initial observation and mutation fencing.
Installation memory types distinguish backing memory from runtime grants.
Raw stable-memory corruption is confined to canic-tests development dependencies;
allocation inspection uses Memory's read-only doctor report. No storage keys,
record encodings or allocation identities changed.

Validation (evidence and before-images: /tmp/canic-harness-current/):
- Internal ordinary/governed test targets and all canic-tests targets compile.
- Internal governed library/tests and canic-tests tests pass scoped Clippy with
  warnings denied. Migration-related redundant borrows are removed; ordered
  scenario length exceptions are local, explained and checked with expect.
- Seven cache/input/snapshot-target native regressions pass; the real-tool
  generated-artifact comparison remains ignored.
- The native allocation lookup regression passes and preserves exact stable bytes.
- Blob adapter Wasm compilation passes on Rust 1.91.0.
- Root dependency boundaries, scoped formatting and scoped diff checks pass.

The complete 0.112 batch is not yet push/publish-ready: changed pooled restore
and Blob continuity behavior still needs its targeted PocketIC acceptance, and
the historical audit-manifest dependency-pin failure recorded below is unresolved.
No new missing upstream API was confirmed. Both open 0.112 changelog views cover
this slice; versions remain unchanged. No broad suite, installed-IC or native
macOS qualification, commit, push, release or deployment ran.

## Platform native secp256k1 adoption — 2026-10-18

Verified published Platform 0.1.13 and its native-only secp256k1 module, which
reexports the existing exact upstream 0.3.0 public-key types. Core offline key
derivation now imports those types through Platform. Removed the direct catalog
and member dependency plus its pin exception; added direct-dependency refusals
to all three maintained workspace boundary policies. Catalog minimums and
scaffold templates select Platform 0.1.13. No key/path/network policy, wire
format or crypto implementation changed.

All five offline derivation tests pass. Core Wasm compilation passes on Rust
1.91.0. The selected normal Wasm graph excludes ic-secp256k1; native resolution
retains it only through Platform. Three boundary checks and scoped formatting
and diff checks pass. The two independent consumer lockfiles also resolve Auth
0.10.0 and Blob 0.34.0 to satisfy already-present shared path requirements.
An external update selected Query 0.62.2 during this work; it was preserved.
Evidence and before-images: /tmp/canic-platform-secp/.

The dependency-pin check cannot finish: the historical
docs/audits/reports/2026-09/2026-09-24/release-binding-feasibility/Cargo.toml
inherits ic-platform from a catalog without that declaration. That unrelated
historical artifact remains untouched. The earlier internal harness cache/reset
API blockers remain; the complete 0.112 batch is not push/publish-ready.
Changelog drafts include this adoption. No broad suite, installed-IC/native
macOS qualification, commit, push, release or deployment ran.

## Host build integration qualified; harness cache/reset APIs remain — 2026-10-18

Host now uses IC Build's public Cargo selection types, explicit default-feature
grouping, and the validated InputEvidence returned by post-build validation.
Input inventory construction is shared by initial capture and post-build
validation; rejection recapture remains advisory only. Corrected the Candid
resolver's byte return type and Host digest formatting. PocketIC startup uses
explicit Spawn/Connect configurations, preserving timeout and server ownership.
351 remaining old Candid method calls now use Testkit reply-argument functions
with single-reply tuple decoding, existing callers and decoder budgets; removed
the obsolete call-trait imports. Seven scoped Host Clippy findings are corrected.

Evidence in /tmp/canic-local-integration/:
- Host local-fleet library check passes.
- 19 reuse tests passed initially; the remaining active test failed only on
  sandbox registry access and passed with network access. One opt-in real-Cargo
  matrix remains ignored.
- Host library/test Clippy with local-fleet and warnings denied passes.
- The generated-infrastructure input stability test passes.
- Dependency boundaries, scoped Rustfmt and scoped diff checks pass.

Dependencies changed externally during qualification. The 19-test run used Build
0.17.6, Host 0.27.6, Testkit 0.54.1, Platform 0.1.12 and Auth 0.9.1.
The successful reuse retry and Clippy used Auth 0.10.0. The final infrastructure
test rebuilt Core, Control Plane and Host with Platform 0.1.13 and Auth 0.10.0.
No dependency selections or versions were changed by this slice.

The complete 0.112 batch is still not ready to push/publish. A focused
canic-tests compilation fails in canic-testing-internal: its artifact builders
still use removed Build cache/pruning/progress/profile APIs, and its two pooled
baseline recipes still use removed Testkit reset/receipt APIs. Some baseline
imports are incorrectly test-gated despite use by its ordinary library.
These are confirmed local integration blockers, not verified missing upstream
functionality. The migrated Candid calls parse and Host funding callers pass
Clippy, but the complete internal harness remains unqualified behind those
errors. Continue with artifacts.rs, embedded_root, and the Root/Fleet Registry
baseline recipes. Detailed compiler evidence: tests-check.log.

Open 0.112 changelogs include this slice; full batch publication readiness is
not established. No broad suite, PocketIC execution, new MSRV/Wasm proof,
installed-IC or native-macOS qualification, commit, push or release ran here.

## Memory/Storage compilation cleared; Testkit harness migration remains — 2026-10-18

Core and Control Plane now use published Storage 0.5.3 adapters with Memory
0.48.2 exclusive leases. Maps/cells/vectors retain their record encoding and
allocation keys. Receipt reservation retains the owned map on success/refusal;
chunk resets consume and restore the owned vector. The duplicate local bounded
cell and obsolete raw collection exports are removed. Tests use governed fixture
runtimes; explicit unmanaged images retain the custody/diagnostic cases. The only
lock change in this slice adds Control Plane's Memory test-support dependency
edge; no package selections or release versions change.

The facade's `testing` feature now uses Testkit 0.54 reply-argument functions and
explicit borrowed-server configuration. Caller identities, single-reply shapes,
decoder budgets and startup timeout are preserved. Its old call-trait reexport is
removed. Minimal-feature auth dead code and scoped compiler/Clippy errors are
corrected without changing admission policy.

Passing evidence: 3 receipt-capacity/bounded-cell tests, 44 focused Core memory,
recovery, metrics, replay and reopen tests, and 39 Control Plane storage/directory
cases. Core/Control Plane library Clippy passes with warnings denied; minimal
Control Plane and facade testing/sharding/application-auth/control-plane-role
compilation pass. Final Control Plane Wasm cross-compilation passes on Rust 1.91.
Root dependency boundaries, scoped Rustfmt and diff checks pass. Evidence and
before-images: `/tmp/canic-storage-adapters/`.

The previous 17-error Memory/Storage blocker is cleared for these targets. The
complete 0.112 batch is not ready: 18 remaining source files in `canic-tests`,
`canic-testing-internal` and Host funding tests still import the removed Testkit
`CandidCallExt`; many associated method calls need migration to explicit reply
argument functions. This is verified source/API drift, not an upstream missing-API
blocker. Continue with that test-harness migration and its focused build checks.
No broad gate, PocketIC execution, installed-IC/native-macOS qualification, Git
publication or release ran in this slice. Changelogs remain in the open 0.112.0
batch; they do not imply complete batch qualification.


## Platform 0.1.12 completes direct CDK routing — 2026-10-18

Verified non-yanked registry 0.1.12 and archive checksum, then adopted it in all
three catalogs/locks and scaffold templates. Remaining system, signing-cost,
cycle, logging and typed call-error references now use Platform's original CDK
exports. Direct CDK declarations are removed from Core, facade, Blob adapter and
all remaining probes; all three policies forbid them. Source guards retain their
coverage of raw Platform entrypoints and policy/DTO effects. No storage schema,
call policy, decoder limits or executor identity changed.

All boundary checks pass. Actual Root funding and ICP refill probes pass Wasm
compilation on Rust 1.91 without direct CDK. Focused Core compilation still fails
at the same 17 Memory/Storage errors; no new Platform import failure is reported.
This does not establish full runtime, installed-IC or native macOS acceptance.
The complete release remains blocked by Memory/Storage integration. Evidence and
before-images: `/tmp/canic-platform-0112/`. Platform #22's missing-export gap is
resolved for Canic; producer closure remains with the upstream owner.


## Platform 0.1.11 counters and incoming cycles — 2026-10-18

Verified published non-yanked 0.1.11 and its archive checksum. All three catalogs
and locks select it. Core perf now calls Platform's exact CDK call-context counter;
Core incoming-cycle reads/acceptance and the Root funding probe use its new
exports, without changing policy, limits or arithmetic. Scaffold templates select
0.1.11. Remaining missing CDK APIs stay explicit under Platform #22.

All three direct dependency-boundary checks and scoped formatting/diff checks
pass. The actual `root_funding_probe` cross-compiles for Wasm on Rust 1.91.
This is not installed-cycle execution or native macOS acceptance. No full runtime
build was rerun; the last confirmed blocker remains the 17 Core Memory/Storage
errors. Concurrent lockfile changes, including Windows selections, were preserved
and are not attributed to Platform. Evidence: `/tmp/canic-platform-0111/`.


## Platform 0.1.10 runtime boundary — 2026-10-18

Adopted registry 0.1.10 in all three workspaces. Platform's maintained CDK macros
replace upstream macro calls in fixtures and Canic's hidden macro bridge. Supported
system, call and protected-spawn APIs route through Platform; Host BLS fixture
keys select its independent `bls` capability. All maintained direct Candid and
BLS declarations are removed. Fleet wrapper and CLI scaffold generators emit
Platform instead of direct Candid/CDK; obsolete wrapper version fields are gone.
Direct CDK remains only for missing runtime exports, tracked in
[Platform #22](https://github.com/dragginzgame/ic-platform/issues/22), with related
stable/certificate gaps #19–21. No local replacement APIs, storage migration,
new decoder budgets or altered executor/lifecycle ownership were introduced.

All three dependency-boundary checks pass, including forbidden direct Candid/BLS.
All 43 Canic macro tests pass. Actual `icp_refill_stub` and `cycles_ledger_stub`
Wasm checks pass on Rust 1.91 using only Platform declarations. The facade testing
check still fails at the same 17 Core Memory/Storage errors; full facade/generated
Fleet/Host and installed-IC qualification therefore remain unproven. Native macOS
was not run. The complete release batch is not ready. Evidence and before-images:
`/tmp/canic-platform-0110/`. No Git publication or release effects ran.


## Testkit decoder owner — 2026-10-18

Removed the facade/Host direct `ic-testkit-candid` declarations and the facade
feature edge. Testkit 0.54 uses Platform's exact `DecodeLimits`; both maintained
budget constants now name that owner with every limit unchanged. The root lock
adopts the already selected Testkit 0.54 requirement. All three direct boundary
policies forbid the retired package. Query 0.62.1 still retains it transitively;
removing Canic's declarations does not remove that upstream dependency.

All three dependency-boundary checks, scoped Rustfmt and diff checks pass. The
focused facade testing build stops at the existing 17 Core Memory/Storage errors;
Testkit 0.54 itself checks successfully. Log: `/tmp/canic-testkit054/canic-check.log`.
The complete release batch remains blocked by runtime compilation.


## Platform 0.1.9 adoption — 2026-10-18

Verified the non-yanked registry release and archive checksum; all three catalogs
and locks now select 0.1.9. The new canonical `DeleteCanisterSnapshotArgs` replaces
Canic's final snapshot-delete request mirror. Target, method, unbounded wait,
IDs and authority/recovery flow are unchanged. Intentional partial replies remain.
The root lock changes only Platform; both independent consumer locks also reconcile
existing Timer requirements to 0.28.0 and drop the superseded Metrics 0.13.3 node.

An isolated Rust 1.91 proof passes type equality, exact Candid byte equality and
bidirectional decoding for empty/nonempty snapshot IDs; the selected dependency
graph also passes Wasm cross-compilation on Rust 1.91. All three direct boundary
checks pass. This is not installed-IC or native macOS qualification; full runtime
compilation and release acceptance remain separate. Evidence and before-images:
`/tmp/canic-platform-019/`. CDK frontend/macro ownership remains Platform #1;
this release does not eliminate the transitive Candid/CDK graph.


## Shared graph observation adoption

`runtime-dependency-graph-gate` now runs during local invariants and CI preflight.
The one observation requires Memory/Timers singletons and no raw CDK timer.
The remaining Rust source/lifecycle guards are preserved.

The snapshot selects committed Shared topic source `6eb8c41` from PR #285
(version annotation 0.24.0; unmerged, not a new release). Local package roots,
feature matrices and policy remain consumer-owned. Graph qualification runs
locked/offline against the copied/current source; it does not qualify deployment
artifacts, native macOS or installed-IC behavior.


**Canic 0.111.0 is live; the integration batch remains uncommitted.** The maintainer
explicitly waived the 0.111 closeout audit for completing Auth, Backup and Blob
integration, then a cleanup slice to reduce Canic's surface. That decision is
recorded on [#517](https://github.com/dragginzgame/canic/issues/517#issuecomment-6119790876).
It authorizes this in-repository batch, not release/version/Git effects or a
blanket exception for future minors. The pending incompatible public family is
classified under the existing **0.112.0** changelog entry. Own package versions
remain **0.111.0**. The objective is to delete duplicate mechanisms in favor of
upstream owners; moving declarations alone does not demonstrate faster builds.


## Auth capability and grant ownership — 2026-10-18

The approved Canic-only cuts use existing published Auth 0.9.1 APIs. ECDSA-only
Root verification selects `chain-key-verification`; certified receivers retain
canister-signature verification explicitly. The crypto guard now separates the
Root-signing and delegated-verification profiles. Root still prepares signatures;
it no longer selects the unused BLS verifier. Configuration and certificate
preparation use Auth's grant rules, with Canic retaining bounded DTO conversion,
configuration sorting/empty handling and typed diagnostic projection. A malformed
later role does not mask an earlier grant failure. Signed bytes and storage
formats are unchanged; no additional Auth release is required.

The actual offline crypto guard passes all 12 canonical roles, with 9 zero-auth
roles. Failure fixtures reject both an accidental Root BLS dependency and missing
Root ECDSA. Shell lint and focused formatting pass. Exact current/before source
copies, real Canic Contracts and published Auth pass 27 isolated cases on Rust
1.91, including a typed 1,767-input parity matrix and exact capacity boundaries.
The copied grant/canonical/projection modules also cross-compile for Wasm on 1.91.
Isolated Clippy passes with only unused-neighbor dead-code diagnostics allowed;
no production lint allowance was added. All probe external identities are selected
by Canic's current lockfile. These are source/graph checks, not installed behavior.

The full Core ECDSA check still fails at 17 pre-existing Memory/Storage errors;
its focused test configuration stops at 13 obsolete Memory API references.
No broad gate, installed-canister or native macOS qualification ran. The complete
0.112 batch remains blocked by those runtime/storage integrations. Pending notes
are updated; own versions, Git index/refs and other work are preserved. Concurrent
Host 0.27.3 and Metrics 0.14.1 lock updates and the Cargo-execution handoff were
retained, not attributed to this cut. The crypto graph check passes again after
the later Metrics update; no lockfile mutation was performed by this task. Evidence: `target/auth-capability-grants-20261018/`; existing adoption
tracking remains [#491](https://github.com/dragginzgame/canic/issues/491) and
[#517](https://github.com/dragginzgame/canic/issues/517).

## Shared Cargo execution — 2026-10-18

Integrated the prepared Canic contribution from the IC Build convergence work
against selected registry IC Build 0.17.2. App batches and all generated Fleet
roles, including Root, use shared communication and exact artifact selection.
One typed message stream feeds selection and Canic’s include-export admission.
The local heartbeat thread, raw selector and its private DTO/error types are gone.
A named output record carries execution and typed messages. The Canic-specific
custom-target capture and library/binary collision regressions remain.

Three isolated tests of the extracted actual execution code pass on Rust 1.91:
failed-exit precedence versus malformed JSON, typed build-script evidence and real
Wasm artifact retention across target collisions. Strict harness Clippy passes.
The full Host check still stops at the same 17 Core Memory/Storage errors. The
harness substitutes only the canonical declaration marker and fixture plumbing;
it does not qualify full Host export/release admission or installed behavior.
Formatting and diff checks pass. Evidence: `/tmp/canic-build-convergence/`.

This completes the Canic execution slice, not the full fleet convergence proposal.
Provenance still independently probes tools after custom builds: carrying exact
resolved recipe facts through actual executions and complete-release cache hits
remains tracked by Build #71. The published facts API does not by itself bind a
custom `cargo rustc` invocation; do not attach an unrelated recipe’s observations.
Query/Testkit prepared consumer changes remain in the existing upstream work;
no sibling edits, commits, pushes, releases or deployment ran. The complete batch
remains blocked by runtime compilation and unfinished provenance integration.

## Platform 0.1.8 adoption — 2026-10-18

Published 0.1.8 is selected in all three catalogs and lockfiles. Core’s management
boundary uses Platform’s CanisterIdRecord, CanisterInfoArgs, Snapshot and ChunkHash;
four exact DTO mirrors are removed. Call bodies are unchanged except for type
substitutions. Partial status/history response projections remain unchanged.
The missing DeleteCanisterSnapshotArgs export is documented with a reproducible
E0432 consumer and acceptance checks in [Platform #13](https://github.com/dragginzgame/ic-platform/issues/13#issuecomment-6248729948).
The necessary local delete-request DTO remains pending that API.

An isolated Rust 1.91 test passes exact Candid type/byte parity and bidirectional
decoding for the four replaced shapes; Wasm cross-compilation passes. Formatting,
locked metadata, all three dependency-boundary checks and diff checks pass. Core
still stops at the same 17 Memory/Storage errors. Installed runtime and native
macOS qualification remain unavailable; the complete release batch is blocked.
Evidence and before-images: `/tmp/canic-platform-018/`. No commit or release ran.

## Platform 0.1.7 adoption — 2026-10-18

All three catalogs and locks select published 0.1.7. Default decoders and structural
subtyping now route through Platform with upstream limits/error behavior intact.
Direct Candid is removed from Contracts, Core, Host, CLI and both internal test
packages; the hidden facade Candid reexport is deleted. CDK macro consumers retain
their direct declarations. Generated Fleet packages resolve their required Candid
version through Platform rather than Core. No new wrapper or decoder policy added.

All 331 Contracts tests pass. An isolated default-decoder identity/diagnostics
comparison passes on Rust 1.91 and cross-compiles to Wasm. Import audit, locked
metadata (including generated dependency selection), formatting and all three
boundary checks pass. The selected runtime graph contains no parser and retains
executor 2.0.0. Host still fails at the same 17 Core Memory/Storage errors; full
Host, installed-IC and native macOS qualification remain unavailable.

Platform issue #5 is updated with adoption evidence. The remaining upstream gaps
are already tracked by #1 (CDK runtime/macros) and #2 (Testkit identity); no duplicate
issue was filed. Evidence and task before-images: `/tmp/canic-platform-017/`.
The complete release batch remains blocked. No commit, push or release ran.

## Platform 0.1.6 follow-up — 2026-10-18

Published 0.1.6 is selected in all three catalogs and locks. The root lock already
selected it at task entry; both Blob consumer locks advanced from 0.1.5. Host
ledger types and facade raw argument APIs now route through Platform. Direct
ledger/IC0 dependencies are removed and prohibited by all three policies. Existing
upstream package selections are preserved; unrelated Windows lock edges were
retained from the starting worktree and locked metadata admission passes.

The isolated ledger identity/wire test passes on Rust 1.91; its raw argument
function signatures and ledger feature cross-check on Wasm. Mapped imports,
formatting and all three boundary checks pass. Host still stops at the same 17
Core Memory/Storage errors, so no full Host, installed-IC or native macOS
qualification is claimed. Remaining Candid/CDK API exceptions are unchanged.
No commit, release or deployment ran; the complete batch remains blocked.
Evidence and task before-images: `/tmp/canic-platform-016/`.

## Published Platform update — 2026-10-18

Registry recheck still selects 0.1.5. Host Agent APIs and native delegation tests
now route through Platform `agent`; runtime probe HTTP types use
`management-canister-types`. Direct Agent/management-types declarations are gone
and forbidden by all three boundary policies. No resolved package versions or
checksums changed. The mapped-import identity audit, two isolated native type
identity tests (including Rust 1.91), the Rust 1.91 Wasm cross-check, formatting
and boundary checks pass.
Host compilation still stops in Core at the same 17 Memory/Storage errors, before
qualifying Host or installed runtime behavior. Evidence: `/tmp/canic-platform-routing/`.

Selected published **ic-platform 0.1.5** in all three catalogs and lockfiles.
New supported paths cover configured/staged decoding, Principal errors, service
registration/export, reflection/equality and native parsing/binding generation.
Direct parser dependencies are removed and forbidden by all three boundary
policies. Existing Principal/contract/certification/executor adoption remains.
CDK runtime/spawn, default Candid decode helpers, structural subtyping and
Testkit limits identity remain explicit exceptions; see
[the boundary](../../governance/platform-boundary.md) and Platform issues
[#1](https://github.com/dragginzgame/ic-platform/issues/1),
[#2](https://github.com/dragginzgame/ic-platform/issues/2),
[#5](https://github.com/dragginzgame/ic-platform/issues/5).

Contracts 331 tests and macros 43 tests pass on 0.1.5; strict all-target Clippy for
those packages passes. An isolated two-test probe qualifies explicit service
export, native parser admission, Encode/Decode and configured quota refusal;
it passes Rust 1.91 and Wasm cross-compilation without direct Candid. Earlier
Backup qualification remains recorded in the prior adoption evidence.
The runtime recheck exposed two log imports dropped in the earlier import grouping;
both are restored and the parent/child import audit reports no further omissions.
Core now fails only at the same 17 Memory/Storage errors. Runtime-dependent Host,
generated-role and installed-IC checks remain blocked; native macOS is unqualified.

Lock resolution also reconciled the maintainer's pending Build 0.17, Blob 0.33,
Query 0.62 and newly changed Metrics 0.14 constraints. Those API migrations remain
separate work. Evidence/before-images: `/tmp/canic-platform-015/` (earlier adoption:
`/tmp/canic-platform-adoption/`). The open 0.112 changelog is updated; package
versions remain 0.111.0. No broad gate, commit, push, release or deployment ran.
The complete release batch is still not ready to push.


## Shared Tooling 0.24.0 adoption — 2026-10-18

Upstream main was rechecked at the same 0.24.0 revision; the canonical exporter
preview reports no snapshot changes. Corrected member labels so internal test
crates are explicitly tests/tooling and Blob resumes the production group.
All three manifests retain identical TOML values and lockfiles are unchanged.

The canonical snapshot now pins `cab12cd3dfa4fdb34846b25397f0d72881bb9ae0`.
PR #597’s extractor setup is integrated locally; IC executables are explicit local
setup/check selections. No PR merge, release or package-version change ran.
Catalog/member role annotations and grouped write/check/release ordering are active.
Focused formatter, extractor/Make/workflow, current candidate, selected commit,
release transaction and validation-lane fixtures pass. Snapshot integrity passes.
Real `tools-check` passes after canonical preparation of the narrowed formatter
bundle, ICP 1.7.0 IC bundle and locked Testkit server; extractor remains 0.1.6.
Eighteen locked production graph inspections exclude tests/tooling-only identities;
this is not automated deployment enforcement or successful runtime compilation.
The missing transitive normal/build exclusion gate and artifact-evidence binding
are tracked by #602 and documented in `docs/governance/dependency-roles.md`. Evidence and adoption
before-images: `/tmp/canic-shared-024/`. The complete 0.112 batch remains unready
because the separate runtime integration blockers below remain unresolved.


## Canonical owner convergence — 2026-10-18

Release reply decoding now uses the published `ic-platform::candid` owner in six
Host paths; caller quotas, aggregate debits and typed domain errors stay local.
Host PR #189 and issue #188 were closed because they duplicated that owner.
Five isolated tests of the extracted actual funding decoder pass. The Host release
regressions remain blocked before compilation by 17 existing Core/Memory errors.

The unused public backup capture executor and its exclusive recovery fixtures are
removed. Dry-run plans, retained journal reading/validation, filesystem verification,
restore execution and restore crash proofs remain. CLI directly reports journal
lock errors. Public Host checksum owners replace two private Backup calls.
All 260 Backup package tests and strict all-target Clippy pass, including restore
and retained persistence crashes;
see `/tmp/canic-owner-convergence/` for exact final logs and change inventory.

IC Build 0.17.2 is now selected and the shared execution slice is integrated;
see the current handoff above. Complete environment/fact binding and provenance
projection remain consumer work under Build #71. Keep complete-release admission and
the special declaration `cargo rustc` path local until equivalence is qualified.
No dependency version, Git index, commit, release or deployment was changed here.
The complete 0.112 batch is still unready; changelog drafts include this slice.

## Canonical Auth session ownership — 2026-10-18

The Canic-only cleanup retains canonical Auth session bytes instead of persisting
and reconstructing a second identity/scope/lifetime tuple. The parallel local
session/replay model and common authorization policy are removed. Both runtime
and const scope validation use Auth's published detailed parser; Canic still owns
set bounds, protected fleet authority, generation transitions and stable commits.
The compact replay row is a single host persistence projection, not a second
stored copy of Auth replay bytes. The custom transactional adapter remains because
it owns Canic's per-subject quota and combined stable/index accounting. No new
shared backend or compatibility contract is introduced.

Real Auth-issued sessions exercise the adapter's authorization, replacement,
logout, corruption and rollback-on-returned-error paths. Missing login reservations
refuse restoration, and per-subject capacity refuses before persistence. Retired
fake session constructors and test-only lookup/page/removal helpers are gone. The
Wasm denial probe now covers only the three implemented closed caller outcomes.
The production facade remains closed pending protected Root enrollment; this is
not a claim of installed application admission.

All 25 isolated actual-source cases pass, including maximum occupancy of 2,048
sessions and 4,096 replay records. The probe uses published Auth 0.9.1, with external
lock selections admitted against Canic's unchanged lockfile. Strict Clippy and the
actual Rust 1.91 test-source compiler check pass. Only the blocked stable accessor
layer is substituted with native memory, so this is not stable-memory, upgrade or
installed-canister qualification. Snapshot-only lint expectations are relaxed in
the probe; production source has no new lint allowances.

The focused Core check still fails on 17 pre-existing Memory/Storage library
errors; its test configuration also reaches 13 obsolete Memory API errors.
The removed-import regression caught during this cleanup is corrected. Full batch
push readiness is still blocked by storage integration and protected enrollment,
not by an unrun broad suite. No broad gate ran. Evidence and exact before-images:
`target/auth-session-owner-20261018/`. Existing Auth/Storage integration tracking
stays with [#491](https://github.com/dragginzgame/canic/issues/491) and
[#517](https://github.com/dragginzgame/canic/issues/517).

The existing 0.112 notes and stable-layout guide describe the private session-row
hard cut under Canic's reinstall-only policy. Own versions, dependency selections,
Git index/refs, Toko and IC Auth source are unchanged by this Canic task.


## Backup Principal dependency — 2026-10-17

Backup now imports all seven Principal uses directly from `ic_principal`, with
only its Serde/conversion features. The lockfile changes only Backup's direct
edge; no package versions change and Candid remains upstream's transitive input.
Locked metadata passes. Four actual-source isolated topology/hash tests and strict
Clippy pass. Package-scoped check and all-target/all-feature Clippy still stop at
the two existing private Backup checksum calls. The manifest/lock mismatch is
cleared, but the complete release batch remains unready. Open 0.112 notes are
updated. Evidence: `/tmp/canic-backup-principal/`; no Git/release effects ran.


## Endpoint inspection and upstream extraction — 2026-10-17

Endpoint inspection now selects the deployed Fleet's retained release and checks
its exact protocol binding. Host owns resolution; CLI retains explicit offline
build selection and rendering. The metadata/role-file fallback is deleted. Target
selection rejects ambiguous roles and accepts exact member Principals. The open
0.112 notes and operator guide describe the `installed` JSON source contract.

Four isolated actual-source cases pass for target selection, exact declaration
binding and canonical protocol-profile hashing. Strict Clippy passes with a
harness-only missing-panic-documentation exception on the unchanged Core hash
module. The resolver verifies the same bytes returned to the caller instead of
reopening the sidecar. This does not qualify release loading or the CLI as a whole. The focused `cargo check --locked --offline -p canic-cli
--tests` stops before compilation because the current root manifest and lock are
inconsistent; this task leaves those dependency selections unchanged. No broad gate
ran. The complete integration batch remains unready for push/publication.

[Host PR #189](https://github.com/dragginzgame/ic-host-tooling/pull/189), tracked by
[Host #188](https://github.com/dragginzgame/ic-host-tooling/issues/188), adds bounded
single-value Candid decoding with explicit caller-owned limits and aggregate
accounting. Four tests pass on Rust 1.99 and the declared 1.88 minimum; strict
library/test Clippy passes. The prepared `/tmp/canic-cleanup-seven/decoder-adoption.patch`
passes apply-check and preserves five Canic release observers' exact quotas and
typed budget/decode failures. It is not applied until the API is published.

[Build PR #73](https://github.com/dragginzgame/ic-build/pull/73), tracked by
[Build #71](https://github.com/dragginzgame/ic-build/issues/71), retains tool probe
bytes, typed Cargo selection, target and selected effective environment with input
snapshots. Standalone/source-change, unchanged fingerprint-vector and batched
resolution proofs pass; the standalone case also passes on Rust 1.91. Strict
library/test Clippy passes. Facts describe inputs, not successful compilation.
Canic provenance still needs to carry the published facts through actual app and
infrastructure execution and retained cache-hit outputs before deleting its own
probes/fingerprinting. Do not rediscover a snapshot after compilation and label it
executed evidence. This producer prerequisite does not complete consumer adoption.

Evidence and before-images: `/tmp/canic-cleanup-seven/`. Only isolated upstream
PR topic branches were committed/pushed; no merges, releases, Canic Git effects,
package versions or dependency overrides were performed. Linux evidence does not
qualify native macOS. Canic #517 remains the integration tracker.


## Timing, provenance and ICP decoder extraction — 2026-10-17

Identity accounting now belongs to the request timer; disabled diagnostics,
clones and handler replacement retain totals. Provenance path rendering no
longer rereads/hashes the manifest. Seventeen isolated actual-source timing,
path and hash tests plus strict Clippy pass. The focused Host check still stops
at the existing 17 Core storage errors; the complete batch is not push-ready.

[Query PR #80](https://github.com/dragginzgame/ic-query/pull/80) implements bounded
ICP protobuf transfer decoding under the ledger-evidence owner; four decoder
cases pass on Rust 1.99 and 1.91, and selected-feature strict Clippy passes.
[Query #79](https://github.com/dragginzgame/ic-query/issues/79) owns the producer
work; [Canic #517](https://github.com/dragginzgame/canic/issues/517) tracks adoption.
The prepared `/tmp/canic-cleanup-six/icp-adoption.patch` passes apply-check and
removes Canic's production decoder/direct prost dependency while retaining exact
certificate, read and reviewed-intent checks. It is not applied: publication and
consumer qualification remain required. No Canic versions, dependency selections,
Git index or refs changed; only the isolated Query PR branch was published.


## Admission and validation cleanup — 2026-10-17

Admission plan/status/apply ownership now lives in Host. CLI retains parsing,
plan-file I/O and rendering; Medic calls Host directly. Pure admission policy
remains Core-owned. Configuration and provisioning share canonical encoding
primitives, and frontend prepared/disk verification shares one validator while
retaining prepublication checks and postpublication readback.

The canonical byte-vector test passes. An isolated actual-source proof passes
13 cases, including CLI parsing/help, three admission authority/retry cases and four frontend
publication/tampering cases. Its transport scaffold is never exercised and its
frontend fixture bypasses release discovery; this is not installed qualification.
The changed source also passes isolated Clippy with the workspace lint policy;
only unused harness helpers and transport stubs carry harness-only exceptions.
The targeted CLI check stops at two private Backup 0.30 checksum APIs; Host stops
at the existing 17 Core storage errors. The complete 0.112 batch is not ready to
push or publish. No versions, dependency selections or Git refs changed here.


## Auth proof and runtime dependency fixes — 2026-10-17

Auth 0.9's original sixteen Contracts CandidType failures are cleared. Canic now
owns explicit Candid proof DTOs and ops projects all fields into upstream protocol
types. Root verification and typed failures use `ic_auth::delegation`; no old token
API alias or application-session bypass is added. Nine transport types preserve
the upstream Serde representation; canonical hashes and proof material stay owned
by Auth. Application admission remains closed pending Root-managed enrollment.

Canic now derives timer-health labels once from current snapshot state, mode and
outcomes, shared by status and metrics. Memory 0.48 responses retain permanent
key/slot bindings and measurements and remove the obsolete allocation lifecycle
state type/fields. The protocol-surface assertion follows the current shape.

The owning Auth transport test and two Memory contract tests pass; Contracts
library/test Clippy passes with warnings denied. An isolated actual-source probe
passes 38 cases: canonical golden hashes, real chain-key proof verification,
installation/expiry/enrollment refusals, timer classification and Memory projection.
It supplies passive host config/bounds and omits runtime error mapping plus one
runtime facade case. This is not installed admission or complete Core qualification.
The selected-feature Core check reaches 17 storage-family errors. The final
all-feature library check reaches 24, including sharding fallout, raw collection/
manager imports and a cloned lease; Auth and timer API errors are gone.

[Storage #11](https://github.com/dragginzgame/ic-storage/issues/11) records the newly
confirmed map reservation gap: receipt pre-admission capacity must stay fallible
and collection-owned without cloning allocation authority. Existing Storage #3
(map clear) and #10 (bounded cells) also await integration/publication. Do not remove
capacity admission or introduce raw backing access to bypass those prerequisites.
Blob still selects Memory 0.45 while Canic selects 0.48. Root-managed portable
session delivery/admission and installed qualification remain incomplete.

Before-images and logs: `/tmp/canic-live-fixes/`. Dependency selections, Canic Git
index/refs and deployed canisters were not mutated by this slice. The open 0.112 notes
cover the changes; the complete batch is still not ready for push/publication.
No broad gate or deployment ran.


## Version readers, environment and discovery cleanup — 2026-10-17

Unused root/workspace version-reader APIs and reexports are deleted. Build tools
and fingerprints now share one sanitized base environment captured at first use
in the host process. Command creation disables ambient inheritance; context and
Cargo execution no longer reapply environment policy over explicit overrides.
Two focused regressions of the actual environment module pass, including a real
child with explicit override/removal. The standalone harness substitutes only the
ICP password-variable constant; this is not whole Host qualification.

[IC Build PR #63](https://github.com/dragginzgame/ic-build/pull/63) supplies nearest
explicit-workspace lookup with
file-start canonicalization, nested declarations and typed malformed/non-file/
dangling-marker failures. Three cases pass on actual Rust 1.91 and strict
library/test Clippy passes. The prepared Canic adapter retains ICP discovery and
fallback policy; its three tests pass on Canic's Rust 1.99 in an isolated dependency
probe. The probe's initial 1.91 run failed on Canic's existing `std::assert_matches`;
it is not a Canic MSRV claim. Native macOS qualification remains separate.

Consumer discovery deletion waits for a published API. The production dependency
graph is unchanged; the exact rehearsal patch and before-images are retained in
`/tmp/canic-cleanup-four/`. No Canic Git or release effects and no broad gate ran.
The complete integration batch remains unready on the previously recorded Auth,
Storage/Blob and installed-proof gaps; those were not requalified in this slice.
The open 0.112 notes cover the completed local cuts.


## Published Storage boundary adoption — 2026-10-17

Core now selects published IC Storage 0.4 for `Storable`/`storable` and Memory
0.47's root backing-memory exports. Registry Cargo metadata confirms Storage 0.4
uses Memory 0.47 despite its changelog's older 0.46 wording. The lockfile adds only
Storage and the Core dependency edge; existing selections are unchanged. Blob and
Auth are maintainer-owned during this slice and were not edited.

This is codec-boundary adoption, not a completed collection migration. The raw
collection and manager imports still need removal. Storage PRs #2 (narrow exports),
#3 (in-place map clear) and #10 (bounded cells) remain open. Existing raw fixture
setup and retained/cloned memory patterns also need conversion to governed leases.
Host PR #180 remains open, so durable symlink selection is not yet adoptable.
Do not merge or release these proposals without maintainer authority.

Two isolated tests pass using Canic's actual storage macros/CBOR helpers and
published Storage 0.4/Memory 0.47: unchanged record bytes and size admission, plus
exclusive-map lease and record retention across reopen. This does not qualify
Core's collections. The focused Core check stops at the existing 16 Contracts
Auth CandidType errors. No broad gates ran. Evidence and before-images live in
`/tmp/canic-storage-adoption/`; probe compilation used its own target directory.
The complete integration batch remains unready; finish collection/fixture adoption
when its APIs land, alongside the maintainer's Auth/Blob fixes and installed proofs.


## Auth 0.9 feature resolution — 2026-10-17

Auth and protocol-types now select 0.9. Core's chain-key capability and native
development dependency select `certified-proof-verification` in place of the
removed token feature, retaining both chain-key and canister-signature checks.
The targeted root update changes only the two Auth package selections. Both Blob
consumer locks also absorb the existing catalog's Memory 0.47, Storage 0.4,
Metrics 0.13 and Timers 0.27 edges required by their Canic path dependencies.

Locked offline metadata passes for all three graphs; root metadata confirms the
complete certified verifier features. Core's focused chain-key library check
passes resolution, then stops at the existing 16 Contracts CandidType errors.
Auth 0.9 also retires the token Rust module still used by Canic; that adoption
and the transport boundary remain unfinished. Evidence and starting snapshots:
`/tmp/canic-auth-feature-fix/`. No broad gate, Git or release effects ran.
The complete integration batch remains unready for push or publication; the
open 0.112 notes cover this dependency-resolution correction.

## Infrastructure, diagnostics and JSON cleanup — 2026-10-17

Coordinator, Root and Store now share `canister_build::infrastructure` for build
commands, declaration/profile derivation, runtime capture and finalization. Root
retains configured package/capability admission and generated declarations;
Coordinator and Store retain canonical Candid refresh rules. Application builds
keep their existing batching, with Root routed separately through infrastructure.
Runtime paths come from Cargo artifact messages bound to the admitted manifest.
The two old bootstrap modules are removed and their tests move to the shared owner.

Build context printing now belongs to the CLI/example invocation, including the
ablation helper. Ancestor probing and persistent diagnostic marker state are
removed. Fleet's recursive decimal serializer is removed: explicit field/map
adapters and `Cycles` own human-readable decimal encoding. Binary Serde and Candid
remain numeric; JSON/TOML consumers outside Fleet now see cycle strings too.

Eight tests of the actual Cycles source pass in an isolated dependency probe,
including exact CBOR/Candid bytes at zero, one, above u64 and u128 maximum. A
scratch parity case compares the former serializer with explicit scalar, optional,
map and nested-cycle encodings and passes exact byte/value comparisons. An
isolated extraction of the command constructor and environment/profile helpers
passes all 18 role/profile/declaration combinations; its process launcher and
path/network inputs are supplied by the probe. This does not qualify package
resolution or installed Wasm. New owning tests cover Root command/source admission,
nested Fleet action bytes and projected balances; they are not yet runnable.
`cargo check --locked --offline -p canic-host --tests` still fails on the existing
16 Auth 0.8 CandidType errors in Contracts before Host. Focused formatting and
whitespace checks pass. Evidence and before-images: `/tmp/canic-cleanup-three/`.

No full gate, publication or Git effects ran. The open 0.112 notes include these
hard cuts; the complete integration batch remains unready for push until Auth
adaptation, Storage/Blob convergence and installed qualification are completed.


## Table, checksum and read-batch cleanup — 2026-10-17

The local table engine is deleted. CLI callers select Host Text directly; build
reports use underlined columns and module-drift colour is applied after layout.
The checksum wrapper and duplicate validation/error variants are removed in favor
of IC Backup's validated record. Host-backed descriptor traversal and streaming
replace the retired Backup 0.28 filesystem calls, preserving private create-new
staging and no-follow admission. The previously reported five Backup API errors
are resolved. Independent Fleet reads and Candid extraction share one bounded
collector with caller-selected panic handling and unchanged four-worker limits.

All 313 Backup library tests pass, including filesystem rejection, persistence
crash/restart and restore proofs. Strict Backup all-target Clippy passes. Four
standalone tests of the actual shared collector pass. Eleven list-rendering tests
pass in an isolated probe using the exact renderer and registry-tree source plus
published Host Text 0.25.1; the unused protocol-binding payload and enclosing error
carrier are substituted. This is rendering evidence, not full CLI qualification.
Host/CLI all-target/all-feature Clippy stops at the existing 16 Auth 0.8 CandidType
errors in Contracts. No broad validation, native macOS qualification, Git or
release effects ran. Evidence and before-images: `/tmp/canic-cleanup-two/`.

The open 0.112 changelogs describe the hard cuts. Full integration is still not
push-ready: Contracts Auth adaptation and the existing Storage/Blob and installed
qualification work remain. Pending upstream PR adoption from the preceding slice
is separate; those local implementations remain until publication.


## Three ownership extractions — 2026-10-17

Backup and CLI restore tests now call published Host Process crash-barrier APIs
with their existing five-second and thirty-second allowances. Eight duplicate
handshake/wait functions are deleted; persistence assertions remain local.
Only the Backup package dependency edge was added to Cargo.lock, with no selected
package-version change from this slice. Scoped Backup tests and Backup/CLI lint
checks stop at existing Backup 0.28 removed checksum/staging functions and Auth
0.8 CandidType failures; these are not passing consumer qualification.

[Storage PR #10](https://github.com/dragginzgame/ic-storage/pull/10) supplies the
governed bounded-cell adapter. Two Memory 0.47 regression cases and strict
library/adapter Clippy pass, covering unchanged bytes on rejected initialization
or replacement, exclusive leases and cold reopen. Adopt it after publication
alongside the governed collection migration, then remove Canic's bounded wrapper.

[Host PR #180](https://github.com/dragginzgame/ic-host-tooling/pull/180) supplies
admitted durable symlink selection. Two focused tests and library/test Clippy
pass for relative-library execution, rejection, occupied staging and failed
rename. The original installer destination was corrected: Host Tools forbids
filesystem/process dependencies. Generic publication belongs in Host FS; Canic
retains download/archive selection, pins, layout, admission and retry policy.
After publication replace the inline bundle selection sequence, retaining bundles
on after-publication failure. Neither pending implementation is removed prematurely.

Evidence: `/tmp/canic-three-extractions/`; isolated PR checkouts are
`/tmp/canic-bounded-cell-pr` and `/tmp/canic-tool-publication-pr`. No Canic Git,
release or broad-validation effects ran. Upstream PRs are submitted, not merged
or released. Native macOS and full Canic consumer qualification remain pending.
The complete 0.112 batch remains unready for push; existing Auth, Backup and
Storage/Blob integration blockers are not closed by these extractions.


## Memory 0.47 diagnostic projection — 2026-10-17

Allocation responses now project current owners, ledger identity and physical
pool eligibility from the committed runtime bindings/pool, keeping unknown IDs
unlabelled. Numeric measurements remain upstream. One runtime borrow covers the
report and authority views. Ledger snapshots likewise borrow the runtime's pool;
the duplicate Canic pool cache and its separate unavailable error are removed.

The exact projection functions and copied passive DTO shapes pass an isolated
regression with real Memory 0.47 bootstrap: committed labels, governance identity,
unknown/excluded ID and pool eligibility. The probe substitutes the error carrier
and omits DTO serialization derives; it is not full Core/endpoint qualification.
The owning existing projection case is updated for explicit pool context.
Evidence and before-images: `/tmp/canic-memory47-projection/`.

The selected graph now includes Auth/protocol-types 0.8. Core's focused locked
check stops in Contracts with 16 CandidType failures on upstream proof/header/
witness types. Those Canic boundary DTOs need adaptation before package tests can
run. Storage/Blob migration and portable Auth admission remain incomplete. The
0.112 notes cover this slice; the complete batch is not ready to push.

## Host 0.25 resolution — 2026-10-17

Canic Host now selects only `ic-interface`: Candid service inspection is always
available and `ic-limits` is retired. Replica structural comparison is now private
Canic policy beside Wasm enforcement, retaining 12 MiB code bodies, 50,000 local
functions and 1,000 imported plus local globals at the previously selected replica
revision. The exact policy source passes its inclusive/exceeded boundary regression
in an isolated package against published Host Artifacts 0.25.

Resolution remains blocked independently: Host 0.25 requires thiserror ^2.0.21,
while latest PocketIC 16.1.0 pins =2.0.18. A two-dependency isolated registry repro
confirms this; [Host #178](https://github.com/dragginzgame/ic-host-tooling/issues/178)
owns compatible publication. Root Cargo.lock is unchanged by this work. Automatic
approval review rejected the root online update attempt, so verification used a
disposable manifest instead. Logs/before-images: `/tmp/canic-host25/`. No full
package qualification, broad gate or release ran; existing Memory/Storage, Blob,
Auth and installed qualification work remains before complete push readiness.

## IC Storage map-clear prerequisite — 2026-10-17

Canic's existing in-place stable-map clearing requires a missing adapter operation.
[Storage PR #3](https://github.com/dragginzgame/ic-storage/pull/3) exposes the exact
upstream `clear_new` operation while retaining the governed allocation. The isolated
branch passes a public populated/empty-clear and cold-reopen regression plus strict
library/adapter Clippy. It retains released Storage 0.3.1's Memory 0.45 graph;
Memory 0.46 adoption remains with Storage #1 and narrow exports with #2. This is
not Memory 0.46 lease, installed-canister or native macOS qualification.

The PR is submitted for review, not merged or released. Canic source and dependency
selections are unchanged by this slice. The Storage owner must integrate and publish
the compatible facade, including this operation; Blob also needs Memory 0.46
alignment before full collection adoption. Evidence: `/tmp/canic-storage-clear/`
and `/tmp/canic-storage-clear-{test,clippy}.log`. Existing Auth and installed
acceptance blockers remain; the complete 0.112 batch is not ready to push.

## Memory 0.46 policy-token slice — 2026-10-17

Canic now derives the infallible bootstrap token from its policy name/version and
an explicit optional admission token. Fixture callers supply 32-byte identities;
backing-memory setup uses Memory's root `DefaultMemoryImpl` export. The exact
policy/admission sources pass an isolated regression against registry Memory 0.46,
covering repeatability, absent versus zero-token admission and changes to every
participant-token byte. The probe substitutes only the registry error type; it is
not full Core qualification. Evidence: `/tmp/canic-memory46-policy/`.

The focused Core library check now reports two removed raw-collection reexports
in `cdk::structures`. The next work is the governed IC Storage collection migration,
including exclusive-lease lifetimes and fixture callers, then package regressions.
IC Storage 0.3.1 is published. Blob remains published at 0.31 and depends on Memory
0.45, so its matching 0.46 allocation integration is also required. No direct raw
collection dependency, compatibility wrapper or dependency rollback was added.
Both 0.112 notes cover this slice. Whole-batch build, Auth and installed acceptance
remain incomplete; this is not push-ready and no Git/release effect ran.

## Issue #579 toolchain bundle correction — 2026-10-17

Toolchain installation now reports the configured public destination rather than
requiring admitted executable parents to match. This permits Binaryen's macOS
bundle layout while retaining both installers' executable/library admission.
The obsolete `SplitInstallDirectories` error is removed; both 0.112 notes explain
the behavior and public error change.

An isolated harness runs the exact orchestration function with admitted-tool
stubs: bundle acceptance, public destination reporting and rejection propagation
pass. This is not real download, library-loading or macOS qualification. The
owning Host regression stops before tests at the existing Memory 0.46 import
errors. Evidence and task before-images: `/tmp/canic-579/`. Native macOS evidence
and package regression remain open; no issue closure or Git/release effect ran.
Whole-batch Auth and installed-qualification requirements are unchanged.

## PR #588 and issue #583 — 2026-10-17

Developer setup now passes the explicit Canic destination to the shared hook
installer. Bash syntax, ShellCheck and focused exact-argument probes pass,
including spaces, a different working directory, installer failure and absent hooks.
The missing-feature diagnostic shows `canic = { workspace = true, features = [] }`;
the managed-Fleet guide now uses a workspace catalog and explicit member tables.
Both manifest examples parse as TOML. The open 0.112 notes include these changes.

The selected graph changed to Memory 0.46, Build 0.12.1 and Query 0.60 during this
slice. The focused Host role-manifest regression stops before tests because Core
still imports Memory's removed `ic_stable_structures` and `PolicyIdentityError`
exports. Log and task before-images: `/tmp/canic-pr588-issue583/`. No dependency
rollback, broad gate, Git publication, PR merge or issue closure was performed.
The preceding Blob checks belong to the earlier Memory 0.45 graph. Current build
readiness requires the Memory 0.46 caller adoption as well as the existing Auth
and installed-qualification work. Issue #579's split-tool-directory check remains
a confirmed follow-up for bundled macOS Binaryen installations.

## Blob 0.31 adoption — 2026-10-17

The maintainer selected published Blob/contracts 0.31 in the root graph. Canic's
account and gateway adapters now use `CaffeineReplyLimits` with unchanged budgets;
both private consumer locks select 0.31 and remove the obsolete Memory 0.44 edge.
Adapter all-target strict Clippy, internal fixture all-target/all-feature compilation,
and both locked native consumer compilations pass. Logs and task before-images:
`/tmp/canic-blob31/`. No broad suite, installed journey or Git/release effect ran.

This supersedes the Blob compilation blocker below. Whole-batch push readiness
still requires portable Root-managed Auth admission (the public application
facade currently returns `AuthorityUnavailable`), final embedded-fixture verification
and selected installed Blob/checkpoint recovery qualification. Live Backup remains
unavailable under the accepted delivery scope. The 0.112 notes cover this adoption,
but do not claim the incomplete batch is ready for publication.

## Issue triage and Fleet diagnostics — 2026-10-17

Issues #576 and #577 now return specific typed errors for idle attempt recovery
and absent import reviews, before live transport creation or journal publication.
All four focused attempt-recovery tests and the import admission regression pass;
evidence is retained in `/tmp/canic-issue-triage/`. Both 0.112 changelog views agree.

Open Testkit preparation PRs overlap the local 0.50 adoption and need reconciliation,
not blind application. PR #588 still supplies the explicit hook destination fix.
Blob's pending 0.31 notes incorporate its PRs #73/#78; Backup 0.27.1's notes include
its still-open Host-adoption PR #57's outcomes. No PR was merged or closed.
The Blob build mismatch and whole-batch Auth/installed qualification work remain;
this diagnostic slice does not make the complete 0.112 batch ready to push.

## Current build blocker and private consumer alignment — 2026-10-17

The embedded Blob application now uses Memory 0.45 and key-only requests. Both
private consumer lockfiles were regenerated against their current manifests and
pass locked/offline metadata resolution. Evidence: `/tmp/canic-consumer-align/`;
task input is retained in `before/`. Root dependencies and package versions were
not changed. The pending changelog includes the consumer propagation.

A fresh locked adapter compile still fails on one request-type mismatch: published
Blob 0.30 returns Memory 0.44 requests, whereas Canic selects Memory 0.45.
`cargo info ic-blob-storage` reports 0.30.0 as latest, and an explicit 0.31.0 lookup
fails. Upstream's pending 0.31 changelog already names the required Memory 0.45
alignment. Publish the matching Blob/contracts family, then adopt it and qualify
the adapter, private consumers and internal dev/test targets. No local shim or
unpublished dependency override was introduced. Consumer code compilation and
installed qualification are not claimed; the overall batch is not push-ready.


## App fixture Memory callers — 2026-10-17

The user-hub and user-shard reinstall fixtures now use Memory 0.45's two-argument
request constructor. Their locked strict library/test Clippy checks pass with the
selected Auth 0.7.8 (`/tmp/canic-app-memory45/clippy.log`). The remaining old schema
constructor is confined to the independent embedded Blob consumer's Memory 0.41
graph. No dependency selection or release changed; the whole-batch blockers below
remain. Both 0.112 changelog views include the fixture propagation.


## Testkit 0.50 caller adoption — 2026-10-16

Typed Candid calls in the published qualification helpers, Host funding fixtures,
internal fixture library and runtime tests now pass explicit finite decode limits.
Framework fixture replies use a 4 MiB wire ceiling with bounded work/type/header
budgets; small Host ledger replies use 64 KiB. HTTP fixtures use Host Process
capture with their existing header/body/time limits. Baseline outcomes use current
fields, and rejection tests match the typed canister-rejection variant. Repeated
metrics queries and bounded-query admission assertions were extracted for clarity.
The two direct Candid dependency edges are locked; package versions did not change.

Passing evidence in `/tmp/canic-testkit50/`: Host/published-helper compilation and
strict lint, internal fixture library compilation/lint with all features, runtime
test-target compilation/lint, and 17 native HTTP regressions. No PocketIC suite or
broad gate ran. Incoming source is retained in `before/`; open 0.112 changelogs are
updated. Full internal dev/test compilation remains blocked by Blob 0.30 returning
Memory 0.44 requests into Canic's Memory 0.45 boundary (`fixtures.log`). That path
still needs qualification after dependency alignment. Portable Auth admission and
installed qualification remain incomplete, so the whole batch is not push-ready.

## Memory 0.45 adoption — 2026-10-16

Current requests and allocation declarations omit removed schema metadata;
bootstrap policy validates key/slot together. The host pool and macro accept
explicit excluded IDs. Ledger diagnostics expose `excluded_ids`, remove allocation
`schema_version`, and acquire commit evidence through the runtime diagnostic API.
Core/control-plane requests, public composition, and root-workspace fixtures use
the current contract. Independent Blob consumer workspaces retain their own graphs.
The memory guide and open 0.112 changelogs document the public hard cut.

Passing evidence: 33 Core Memory tests, two public pool-provider tests, the Candid
ledger-shape regression, strict Core library/test lint, and native lifecycle-probe
compilation. Evidence is in `/tmp/canic-memory45/`; incoming source is retained in
its `before/` directory. No dependency selections, versions or Git refs changed.

The full batch remains blocked. The Testkit caller failure recorded in `native-agent.log` is addressed above.
Blob 0.30 still returns Memory 0.44 requests while Canic
selects Memory 0.45, so the adapter fails at its request boundary (`blob.log`).
Portable Auth admission and installed qualification also remain unfinished.
No broad validation or release ran.

## Auth status capability emission — 2026-10-16

Managed Auth status types and their endpoint now require delegation-proof issuer
capability. Local authorization alone previously emitted two empty Candid enums,
whose derives failed strict unreachable-code lint. The redundant local-auth macro
attribute parameter is removed; no placeholder variant or lint suppression remains.
Independent locked/offline strict library lint passes for `local_authorization_probe`
and `delegation_root_stub`, avoiding cross-fixture feature unification. Evidence:
`/tmp/canic-auth-empty-local-clippy.log` and `/tmp/canic-auth-empty-root-clippy.log`.
Incoming source: `/tmp/canic-auth-empty-before/`. The 0.112 changelogs are updated;
portable Auth admission and installed qualification still block the complete batch.


## Auth storage visibility lint — 2026-10-16

The two conditional Auth record reexports now use `pub` inside the existing
crate-private storage module. The layering guard explicitly requires that storage
boundary and exempts only its internal reexports; `ops/storage` remains checked.
Core library/test strict lint with `auth-local-application-authorization`, the
layering guard and repository-configured shell lint pass. Evidence:
`/tmp/canic-auth-visibility-clippy.log`; incoming content:
`/tmp/canic-auth-visibility-before/`. This clears the reported visibility failure;
the complete 0.112 Auth/installed-qualification blockers below remain.


## Host 0.23 caller alignment — 2026-10-16

Host/Backup/CLI compile with Host 0.23.1. Missing-path resolution explicitly
preserves its existing depth policy; regular-file opening and exclusive acquisition
are separate, with locks acquired before protected work. Candid normalization now
uses a direct Host Text dependency. The only task-owned lockfile change adds that
Host dependency edge; selected package versions were retained.

All 36 focused native regressions pass across JSON persistence, Candid extraction,
output roots, build/CLI locks, component operations, bootstrap inspection and
reinstall handoff. Strict all-target lint for Host, Backup and CLI passes. Two
existing overlong functions were split along inspection spending and interrupted
handoff boundaries. Evidence: `/tmp/canic-host023-*.log`; incoming content:
`/tmp/canic-host023-before/`. A concurrent Cargo update interrupted the check sequence;
checks resumed after it ended with the selected dependency graph unchanged.

The open 0.112 changelogs include this adoption. The complete batch is not ready
to push: portable Root-managed Auth admission and installed recovery/fixture
qualification remain unfinished. Own versions remain 0.111.0. No broad validation,
Git publication or release ran.


## Invariant gate repairs — 2026-10-16

The layering guard now passes: checkpoint DTO round-trip qualification lives in
ops tests, and the two Auth record reexports are crate-visible. The release
validation-lane fixture now copies the shared version-input validator required by
`next-release-version.sh`; its own rejection/reuse/failure cases pass. The relocated
checkpoint round-trip regression passes. Evidence: `/tmp/canic-release-lane-fix.log`
and `/tmp/canic-checkpoint-wire-fix.log`.

Auth 0.7.5 now clears the crypto closure check: all 12 canonical roles pass,
with no duplicate crypto package versions and no signature stack in the nine
zero-auth roles. This resolves the local dependency evidence for
[IC Auth #65](https://github.com/dragginzgame/ic-auth/issues/65).
Evidence: `/tmp/canic-auth075-crypto.log`. The gate was not weakened.
Complete 0.112 readiness still requires portable Auth admission and installed
qualification. No broad gate, commit, version change or release ran.

## Memory 0.44 integration — 2026-10-16

Canic now owns one explicit thread-local Memory runtime. Lifecycle bootstrap
constructs it; framework/application stores and the Blob adapter receive handles
from that owner. Readiness and diagnostics do not initialize memory. Construction,
bootstrap and reentrant access retain typed errors. Allocation projections derive
bytes from upstream page/bucket measurements and constants, while checking primary
conservation inputs before subtraction. Current Canic diagnostic DTO values remain
unchanged. Removed upstream singleton and fixture transition APIs have no callers.

Focused qualification passes: 33 Core Memory tests, two public composition tests,
strict Core library/test lint, Host/CLI/Backup all-target compilation, Core Wasm
compilation and the managed lifecycle probe's native compilation. The Wasm probe
check exposed and corrected two stale Auth feature gates, then reached the required
`canic build` artifact guard; no governed installed artifact or PocketIC qualification
is claimed. Evidence: `/tmp/canic-memory44-*.log`; incoming source:
`/tmp/canic-memory44-before/`. Tests began with Timers 0.25.1; subsequent lint and
compilation used the maintainer-selected 0.25.2. No dependency files were edited.

Build 0.10.0 aligns with Host 0.22.1. Blob/contracts 0.30.0 now select the
same Memory 0.44.0 as Canic, clearing the known dependency mismatches. The Blob
adapter's locked/offline all-target compile and strict library lint pass; evidence:
`/tmp/canic-blob030-check.log` and `/tmp/canic-blob030-clippy.log`. No source or
dependency changes were needed for this recheck. The complete 0.112 batch remains
not push-ready: portable Root-managed Auth admission and installed recovery/fixture
qualification are unfinished. The open changelog documents the implemented slices;
no commit, version change, release or broad gate ran.

## Shared Tooling and Host adoption — 2026-10-16

Shared Tooling 0.15.7 is adopted at `05548771d95c4f27f23f29e28a6db8f4c0cac44d`;
the 119-file snapshot integrity check passes. The maintainer-selected Host 0.22.1
callers now specify durable publication mode and permissions explicitly, preserving
replace/create-only behavior and owner-only environment-key creation. No compatibility
wrappers or package-version changes were added.

The focused Host/CLI/Backup all-target compile exposes five Host errors: published
Build 0.9.0 takes Host Process 0.21 `OutputLimits`, while Canic selects 0.22.1.
Build's local checkout already selects Host 0.22; a published aligned Build is
required before the consumer compile and focused publication tests can complete.
Evidence: `/tmp/canic-host022-check.log`; incoming source is retained under
`/tmp/canic-host022-before/`. No broad gate or Git publication ran.

Auth 0.7.2 includes the required session selectors; Blob 0.29.0 aligns with Memory
0.43. Their prior focused compile passed, as did eight portable session-storage
tests. Those upstream blockers are cleared. The complete 0.112 batch remains
unready: portable Root-managed enrollment/admission and installed recovery/fixture
qualification are unfinished. Application admission still refuses. Live Backup
remains unavailable under the accepted release scope.

## Application Auth hard cut — 2026-10-16

The maintainer explicitly selected removal of both delegated-token issuance/
per-call guards and delegated-token session establishment. The DTOs, command and
status variants, access guard, issuer retention and replay helpers, session
workflow and obsolete installed journeys are removed, including sample-app
endpoints. Root proof provisioning/renewal, role attestations and portable
Root enrollment/storage infrastructure remain. Public application admission
refuses while the portable workflow is unfinished; this is not a working login
replacement.

Focused evidence under `/tmp/canic-auth-hardcut/`: 321 Auth-enabled Core tests,
32 replay-policy tests, 39 runtime metric tests, 42 endpoint macro tests and 38
public endpoint/protocol tests and 283 current DTO tests pass. The instruction-audit native tests pass
38 cases with report generation ignored. Default Core/Contracts and Auth-enabled
Core strict lint pass; final Core lint includes the serialization benchmark.
The affected native-agent/instruction targets and four changed canister packages
pass strict lint. No installed PocketIC proof or broad gate ran for this cut.

During qualification the maintainer selected Build 0.9. Its consumers now derive
output directories from existing Cargo profile arguments. Testkit 0.48 baseline
callers return the successful restore receipt directly. The agent did not change
manifest or lock selections. Both changes remain within the existing integration
batch; no publication, staging, commit or own-version change ran.

Active guides and 0.112 notes describe the removal. The reduced instruction audit
uses method v4, with a separate comparison identity. Four changed audit method
fingerprints match; the catalog-wide check still fails on the unchanged incoming
`capability-surface.md` fingerprint mismatch. Historical reports retain their
original scope. The selected Blob 0.28/Memory 0.43 alignment and portable Auth
workflow/enrollment/clock-floor/installed qualification remain unfinished. The
complete 0.112 batch and changelog are not ready to publish; this cut is local
source preparation. Retained fixture artifacts still need final release-graph
qualification. Live Backup remains unavailable under the accepted release scope.

## Auth 0.7 package recheck — 2026-10-16

The maintainer selected published Auth/protocol-types 0.7.0 and Metrics 0.11.4.
The session-storage fixture now consumes Auth's decoded claims in the two-argument
verifier callback, avoiding a second decode. All 18 owning storage tests and strict
Core library/test lint pass. Auth-enabled compilation still fails at the two
unfinished establish/authorize workflow calls. Auth PR #64 remains open and the
published Session still lacks mechanism/key-binding selectors.

Registry inspection confirms Blob/contracts remain 0.28.0 with Memory 0.41;
Canic still selects Memory 0.43. Build 0.8.0, Host Process 0.21.0, Testkit 0.48.0
and Backup 0.25.1 remain current. Blob publication/alignment and the unfinished
Root-managed Auth integration still block the complete 0.112 batch and publication
readiness. No dependency selection, own version, Git index or release changed.
Evidence and incoming copies: `/tmp/canic-auth07-check/`. No broad gate ran.

## Build 0.8 and Memory 0.43 consumer qualification — 2026-10-16

Published Build 0.8 clears the Host Process type mismatch. Memory 0.43 adoption
now uses the consolidated RuntimeBootstrapPolicy, typed resolution errors and
canonical ledger identity constants. Diagnostic fixture constructors use Memory's
test-support feature through a development dependency; production admission and
report fields retain their existing behavior.

All 33 memory-filtered tests, two public memory-provider tests, six CLI diagnostic
tests and 17 Host artifact tests pass. The diagnostic and artifact fixes previously
blocked below are now exercised. During qualification the maintainer selected
Testkit 0.48; a scoped lock update selects 0.48.0 and removes Host Process 0.20.1.
Strict Core/Host/CLI library-and-test lint and public-provider lint pass on that
final graph. Behavioral runs preceded the Testkit update and retain their original
0.47 input scope. Evidence and incoming copies: `/tmp/canic-build08-check/`.

The Blob adapter check fails at its request return boundary: published Blob 0.28
exposes Memory 0.41 requests while Canic selects 0.43. The sibling Blob checkout
already prepares this alignment for 0.29; inspection was read-only. Blob #77 and
Canic #517 retain the publication/adoption dependency. No bridge or downgrade was
added. Root/private-consumer adoption and installed qualification follow publication.

Auth PR #64 remains open at `19ea243`; selected Auth 0.6.4 still lacks its session
mechanism/key selectors. Root enrollment, portable endpoint wiring, persisted
clocks/floors, cleanup and installed recovery qualification remain unfinished.
The complete 0.112 batch and changelog are not ready to publish. Live Backup remains
unavailable under the accepted release scope. No broad gate, own version change,
staging, commit, sibling edit or release ran. Older entries retain historical scope.

## Host transform write-policy adoption — 2026-10-16

Both Wasm shrink and Binaryen optimization now pass explicit Host WriteOptions:
Replace publication and 0666 staging permissions reduced by umask, matching the
previous Host 0.19 named-writer defaults. Qualification-before-publication and
producer failure handling are unchanged. The focused locked/offline Host library
check confirms both missing-argument errors are gone; five Build/Host OutputLimits
identity errors still prevent successful compilation and test execution.

Published Build remains 0.7.0 with Host Process 0.20. The sibling Build checkout,
inspected read-only, already prepares Host 0.21 alignment for pending 0.8.0 and
records targeted producer qualification. Do not duplicate that in-progress work
or introduce a consumer bridge. Adoption remains tracked by Canic #517 and
Build #9. Evidence and incoming copies: `/tmp/canic-host021/`. The #547 tests
remain unexecuted. The complete 0.112 batch and notes remain unready to publish;
no broad gate, dependency/version change, sibling edit, commit or release ran.

## Diagnostic range reporting prepared — 2026-10-16

Issue #547 is corrected locally: decimal codes outside u16 receive a distinct
CodeOutOfRange error; malformed syntax retains InvalidCode. Focused regressions
cover overflow, in-range unknown values, leading zeros and current catalogue
identities. Tests have not executed: the selected graph stops in Host compilation
with two missing WriteOptions arguments after Host FS 0.21 adoption and five
OutputLimits identity mismatches between Build 0.7's Host Process 0.20.1 and
Canic's 0.21. Source and failure evidence are under `/tmp/canic-547/`.

This is a dirty local fix, not delivered or qualified behavior. The existing
integration owner #517 retains the new dependency blockers. Auth integration and
the complete 0.112 release batch remain unfinished; notes are not ready for
publication. No dependency downgrade, broad gate, commit or release ran.

## Auth restoration association check — 2026-10-15

Restoration now checks each retained login with a corresponding session against
IC Auth's canonical session/login association API. A different account binding
or a revoked login with a retained session rejects before indexes are published.
Logout state without a session remains valid and preserves replay evidence.
The cross-record lookup is allocated only after existing state size/count limits.
This does not complete missing-row, clock/floor or live enrollment restoration.

The new regression failed before the correction. All 18 targeted session-storage
tests and strict Core library/test lint now pass on Auth 0.6.2, Metrics 0.11.3
and Timers 0.25. Evidence and incoming copies: `/tmp/canic-auth06-restore-binding/`.
No dependency selection changed. Upstream Auth PR #64 remains open; published
Auth is still 0.6.2. Root enrollment, the portable workflow, clocks/floors,
cleanup and installed qualification remain unfinished. The complete 0.112 batch
and its changelog are not ready to publish. No broad gate, staging, commit,
version transaction or release ran.

## Auth logout binding preservation — 2026-10-15

A focused regression reproduced a stable-adapter defect: after session removal,
a logout proposed from another account's login for the same transport caller
could replace the retained login binding. The adapter now requires binding
equality for logout as well as admission. The failed commit changes neither
stable state nor the proposing heap ledger; matching logout retries remain
idempotent and preserve replay custody through restoration.

All 17 targeted session-storage tests and strict Core library/test lint pass.
The current lock selects Auth/protocol-types 0.6.2 and Timers 0.25; no dependency
selection was changed by this slice. Inspection of published Auth 0.6.2 shows
that the session mechanism/key selectors are still absent. Evidence, the failing
regression before the fix, and incoming source copies are retained under
`/tmp/canic-auth06-logout-binding/`.

Root enrollment, clocks/floors, cleanup and the portable endpoint workflow remain
unfinished. These native checks do not qualify installed Auth or the complete
0.112 batch. The batch and changelog are not ready for publication. No broad
gate, staging, commit, version transaction or release ran.

## Auth policy and verifier diagnostics — 2026-10-15

The local session policy now uses Auth 0.6.1's maintained fields. The adapter
handles its current typed failures and preserves budget exhaustion and clock
rollback through both portable credential verifier envelopes. The production
session capability selects upstream portable signature verification. Revoked
credentials remain security refusals; storage failures remain unavailable.

Both focused policy/diagnostic tests and strict Core library/test lint pass.
Feature-enabled compilation now has two remaining errors, both in the old
establish/authorize workflow calls; the three policy/error-projection errors
recorded below are resolved. This run selects Metrics 0.11.2, so it does not
relabel earlier graph evidence. Evidence and incoming copies are retained at
`/tmp/canic-auth06-policy/`. No upstream or dependency-version edit was made.

Continue with the portable credential workflow and protected Root enrollment,
persisted clocks/floors, bounded cleanup and installed recovery qualification.
The complete 0.112 batch and its changelog remain unready for publication. No
broad gate, staging, commit, version transaction or release ran.

## Auth durable admission/logout continuation — 2026-10-15

The stable adapter now commits canonical login rows and protected pool totals
with session/replay admission, and commits logout denial with session-only removal.
Replay custody survives logout. Restoration validates row identities and exact
pool occupancy; failed staging, mismatched bindings and exhausted capacity leave
storage unchanged. Native fixtures now use signed portable credentials instead
of the old retained codec files. Runtime login clocks/floors, cleanup, Root
refresh and endpoint wiring remain unfinished; this is storage qualification,
not a working Auth-enabled deployment.

The selected Core graph now uses Memory 0.42. Its snapshot diagnostic is mapped
directly; the existing optional payload measurement remains unavailable because
allocation metadata does not measure payload occupancy. No dependency downgrade
or local override was introduced. Auth remains published 0.6.1; upstream
[PR #64](https://github.com/dragginzgame/ic-auth/pull/64) is still open.

On this graph, 369 Auth-filtered native tests, 30 role-contract tests, 11
state-contract tests and 15 memory-ops tests pass; strict Core library/test lint
passes. Feature-enabled compilation still reports five errors in the old
session policy/error projection and delegated-token workflow. The durable
transaction/test compile errors recorded below are cleared. Evidence and the
incoming source snapshot are under `/tmp/canic-auth06-transactions/`.

The complete 0.112 batch and its changelog are not ready to publish. Continue
with portable credential admission, persisted login clocks/floors and bounded
cleanup, Root issuance/refresh and installed recovery proofs. Earlier installed
fixture evidence does not qualify these changed sources or dependency graph.
No broad gate, staging, commit, version change or release ran.

## Auth 0.6.1 and caller-bound replay continuation — 2026-10-15

The maintainer's root lock now selects Auth/protocol-types 0.6.1. Both private
Blob consumer locks are aligned to that patch, and the root catalog requires
0.6.1 for its public session/login association check. No unrelated dependency
selection was changed by this continuation. Earlier installed qualification
below remains evidence for its original 0.6.0 graph.

Canic's session projection now permits distinct transport and account principals.
Replay records, pair checks and capacity accounting use the transport caller;
the obsolete account field is removed from replay storage. This is part of the
pending 0.112 reinstall-only cut. The local model regression now retains distinct
caller/account/issuer identities, and a new capacity case checks separate caller
budgets. These cases have not executed: Core native test compilation still fails
with eight errors in the old transaction adapter and its old codec fixtures.
Ordinary Core compilation and strict library lint pass on 0.6.1; this is not
Auth-enabled qualification.

The explicitly authorized isolated upstream proposal is
[Auth PR #64](https://github.com/dragginzgame/ic-auth/pull/64), head `19ea243`.
It exposes retained session mechanism/key selectors so Root enrollment lookup
does not parse private records or guess among an issuer's keys. All 20 upstream
session tests, strict affected library/test lint and the Rust 1.88 session-restore
library check pass. Merge/publication remain separate; Canic has no path override.

Evidence and incoming source copies: `/tmp/canic-auth06-durable/`. The durable
login/session/replay transaction, portable workflow, Root issuance/refresh and
installed restoration proofs remain unfinished. The complete 0.112 batch is not
push-ready; the notes are maintained but not ready to publish. No Canic staging,
commit, own version change, broad gate or release ran.

## Blob alignment qualified; Auth 0.6 boundary remains — 2026-10-15

Published Blob/contracts 0.27.1 and both private composition locks now share
Memory 0.41. The selected graph also includes Metrics 0.11, Timers 0.24, Build
0.6, Host 0.19, Testkit 0.46 and Auth/protocol-types 0.6. No compatibility bridge
or override was added. The embedded allocation peer and its structured evidence
are refreshed; provenance verification, adapter/internal compilation and strict
internal/integration fixture lint pass.

All four selected installed cases pass on the final fixed graph: checkpoint
freeze/restore (35.58s), dedicated Blob permissions/recovery (3m10s), embedded
application-state/recovery (3m09s) and production initialization target binding/
lost replies (16.44s). The governed PocketIC run takes 491.32s. The live-create
refusal regression passes and retains existing output unchanged. Earlier metrics
(86) and timer (12) cases passed before the Auth update; their input identity
remains separate. Evidence: `/tmp/canic-0271-ready/`, with final graph hashes in
`graph-auth06.sha256`. The first installed attempt overlapped the maintainer's
Auth update and failed on mixed protocol identities; it is retained separately.

Core's targeted native Auth check fails against 0.6: the session store still
implements the old transaction and subject-based replay contract. The new owner
requires atomic login revocation persistence, caller-based replay custody,
session-only removal and explicit restoration/verification features. Do not
ignore the proposed login change merely to compile. The existing
[Auth issue](https://github.com/dragginzgame/canic/issues/491#issuecomment-6192040229)
records the failure and required safety evidence. The maintainer has now resumed
Auth integration and selected Fleet Root management of issuer enrollment. The Blob consumer
blocker is resolved; its separate publication-classification concern remains on
[Blob #77](https://github.com/dragginzgame/ic-blob-storage/issues/77#issuecomment-6192039897).

The complete 0.112 batch is not push-ready on Auth 0.6. Open changelog notes are
updated; own versions remain 0.111.0 and publication readiness is blocked by the
Auth integration. Retain the accepted release scope with live Backup unavailable;
its resumed production Root workflow remains incomplete separate work. No broad
gate, staging, commit, version transaction or release ran. Older sections below
retain their historical dependency selections and limitations.

### Root-managed Auth continuation — 2026-10-15

The approved design is recorded at the top of the 0.112 extraction plan. Passive
enrollment contracts now bind the Fleet, exact Root/receiver installations, role,
fresh challenge, revision/generation, validity windows, approved issuer keys and
scopes, and terminal key revocations. They are not yet a callable endpoint or an
installed enrollment protocol. The local session capability selects IC Auth's
checked restoration feature; native verification tests select the verifier feature.

The unconnected enrollment validator proposal is retained outside production at
`/tmp/canic-auth06-integration/enrollment-proposal/`; it must be integrated with
the actual stable transaction owner rather than added as unused runtime code.
Remaining work is Root issuance and receiver refresh/startup closure, portable
credential admission, atomic login/session/replay persistence through shared commit
hooks, bounded cleanup/logout, complete restoration and installed recovery proofs.
Do not treat the passive contract checks as qualification of those workflows.
The existing Auth-enabled compile failures remain in scope; no additional
maintainer choice is needed to proceed.

Both enrollment Candid round-trip tests, strict Contracts library/test lint and
ordinary Core library compilation pass. Evidence is under
`/tmp/canic-auth06-integration/`. Auth-enabled compilation and installed admission
remain unqualified; the passing ordinary build does not cover that feature.

## Memory 0.41 adoption and Blob alignment — 2026-10-15

The maintainer updated the root catalog/lock to Memory 0.41.0. Registry checks
still report Build 0.6.0, Host 0.19.0, Backup 0.24.7, Blob 0.27.0, Query 0.56.0
and Testkit 0.45.0. Core compiles; the Blob adapter fails at its request return
boundary because published Blob exposes Memory 0.40 types. Keep the maintainer's
0.41 selection; no type bridge, downgrade or source override was introduced.

Three Memory test constructors now omit the removed declaration label. All 27
Memory-filtered cases and 21 checkpoint-model cases pass on 0.41; strict Core
library/test lint also passes. The numeric-slot
encoding and constructor hard cuts are documented in the open 0.112 notes;
pre-1.0 retained installations require the existing clean reinstall procedure.
Earlier installed Blob/checkpoint proofs remain evidence for their Memory 0.40
graph and do not qualify 0.41.

[Blob #77](https://github.com/dragginzgame/ic-blob-storage/issues/77) records the
exact consumer failure and public version boundary. The maintainer's local Blob
commit `b92ccb8` already selects 0.41; published main was `cc1a720` when checked.
No duplicate implementation PR or sibling edit was made. The pending 0.27.1
notes claiming unchanged contracts need to reflect the exposed identity change
before publication. Adoption in both private consumer locks and installed
composition qualification follow a published coherent Blob family.

Evidence and incoming copies: `/tmp/canic-memory041-adoption/`; initial compile
failure: `/tmp/canic-memory041-check.log`. Live Backup's production Root workflow
remains unfinished, independently of this dependency mismatch. The complete 0.112
batch is not ready to push or publish. Own versions, Canic Git index and release
records remain untouched.

## Published Build 0.6 qualification — 2026-10-15

The maintainer's graph now selects published Build 0.6.0. Its Host 0.19 boundary
removes the previous producer mismatch. Fixture workspace discovery uses the new
Cargo observation API through one Canic-owned policy: the internal fixture
manifest, 64 KiB stdout, 256 KiB stderr and no elapsed-time deadline. Cargo still
owns workspace membership; no path-layout guessing or compatibility import remains.
Default and governed fixture caller compilation and strict internal/integration
fixture lint pass on the actual registry graph.

The exact Testkit 0.45 runner is prepared through the governed setup target.
The single installed checkpoint case passes using that owner, PocketIC 16.1.0,
Build 0.6, Host 0.19, Blob 0.27 and Memory 0.40. It covers durable business freeze,
upgrade retention, snapshot capture/restore, fresh restore authority, delayed
requests and release replay. Its selected case takes 29.14s; the governed run,
including managed artifact preparation, takes 108.14s. This replaces the previous
candidate/old-driver qualification limitation for the participant proof.

Both managed Blob composition cases also pass on the published graph: dedicated
permissions/recovery and embedded application-state retention/recovery. All four
artifacts were rebuilt for each consumer. The selected run takes 336.91s; the
root catalog and all three lockfile digests remain unchanged. This supersedes
the pending installed Blob 0.27 qualification noted below.

Evidence and incoming source copies: `/tmp/canic-build06-adoption/`. Live Backup
still requires the production Root checkpoint store, authenticated complete-unit
orchestration, paid snapshot reconciliation and host restore integration. The
complete 0.112 batch remains unready; no own package versions, Git index or release
records changed. Build and fixture notes now identify the maintained 0.6 contract.

## Upstream recheck and Host proposal repairs — 2026-10-15

No newer registry versions are available than the preceding refresh: Build
0.5.0, Backup 0.24.7, Blob 0.27.0, Host 0.19.0, Query 0.56.0 and Testkit 0.45.0.
Build #44 is open and mergeable with successful Linux, macOS Apple Silicon,
macOS Intel and lint/security CI. Its publication still blocks the normal Canic
Host/fixture graph; candidate rehearsal evidence remains separate.

Host PRs #120 and #127 now incorporate released 0.19.0 (`804a78b`) through
normal isolated topic merges, at `415f86a` and `c89b1b3` respectively. Their
implementations are unchanged; compatible pending notes now select 0.19.1.
Two duration cases and 69 durable filesystem cases pass on Rust 1.88, alongside
strict affected-package lint, Rustdoc, formatting and diff checks. Hosted checks
must qualify these new heads. No integration branch merge or publication ran.
Evidence is retained in `/tmp/canic-refresh-followup-20261015/`.

The Root checkpoint storage/topology trace confirms these must be coupled to
complete authenticated issuance/release and participant discovery. No unused
store, standalone freeze gate or live-create bypass was added. Coordinated Backup
implementation remains unfinished; the full 0.112 batch is not push-ready.
Canic source, dependencies, package versions and Git index remain unchanged by
this follow-up; the handoff records the upstream contribution progress.

## Published dependency refresh — 2026-10-15

Registry inspection confirms Build 0.5.0, Host 0.19.0, Testkit 0.45.0,
Query 0.56.0 and Backup 0.24.7. Blob/contracts 0.27.0 became available during
this work; the root and both private consumer locks now select it with Memory
0.40.0, without changing unrelated pins. The registry-backed Blob adapter check
passes. Installed composition qualification on this exact graph remains pending.

Testkit 0.45 callers now use fallible installation/Candid APIs, nonzero retries
and a fallible baseline builder. Canic owns the fixture target paths removed
by Build 0.5. The source changes from draft Canic PR #572 were reconciled with
current local fixtures rather than replacing their incoming changes.

Build 0.5 still exposes Host 0.18 output-limit types while Canic selects 0.19.
The isolated [Build #44](https://github.com/dragginzgame/ic-build/pull/44)
aligns its three Host dependencies and classifies the exposed identity change
for pending 0.6. Five observation and six real Cargo example cases pass on Rust
1.91; strict library lint passes on the default toolchain. Publication is needed
before normal Canic Host/fixture qualification can pass. No shipping override,
compatibility shim, dependency downgrade or maintainer sibling edit was added.

A frozen consumer rehearsal with only that Build candidate substituted passes
internal/test-package caller compilation and strict internal fixture lint,
including the governed feature; strict integration-test lint also passes.
Checkpoint fixture lint now separates restore
assertions and removes redundant Copy-error clones. This is candidate evidence, not
published adoption. Evidence and incoming copies are retained under
`/tmp/canic-upstream-refresh-20261015/`. The accepted coordinated live Backup
work remains incomplete; its Root stable store, topology gate, paid snapshot
reconciliation and production capture/restore path still need implementation.
The full 0.112 batch is not push-ready. Changelog notes are updated; Canic's own
versions, Git index and release records remain untouched.

## Live Backup implementation resumed — 2026-10-15

The maintainer now requests getting Backup working. This supersedes the temporary
implementation deferral; the accepted consistency contract remains application
coordinated, with framework-owned fixtures. The design/status tracker records
Root state, topology exclusion, paid capture/reconciliation, host custody and
same-release restore as one complete outcome. Live create must keep refusing
until the actual production path and its interruption evidence pass.

The Root checkpoint model now retains a bounded complete unit and Root-issued
sequence, replays an exact original begin without issuing a second sequence,
rejects stale issuance and changed installation/membership, and retains release
intent even for unanswered prepare calls. Every participant must acknowledge
before the aggregate reaches Prepared or Released. Membership admission remains
closed until Released. CBOR decode validates the retained identity and phase.
All 18 participant/Root native cases and strict Core library/test lint pass.
Evidence/before-images: `/tmp/canic-live-backup-20261015/`.

This slice exposes the canonical model through Core's shared control-plane
support; it does not install a Root store or dispatch a snapshot. Next implement
that stable owner and topology gates, then authenticated Root command/RPC steps
with existing paid-effect custody, followed by host capture/restore integration.
The installed participant proof is already retained from the previous batch;
current installed qualification is blocked by Blob 0.26 / Memory 0.40. Do not
replace the graph with overrides or infer that native journal tests enable live
backup. The complete batch remains unready, with no own version/Git/release effects.

## Upstream integration PR repairs — 2026-10-15

Four existing PRs were reconciled with their latest released upstream bases in
isolated checkouts and pushed without integration merges or publication:
[Build #38](https://github.com/dragginzgame/ic-build/pull/38) at `aaff825`
and [Build #37](https://github.com/dragginzgame/ic-build/pull/37) at `9b3b67c`
now build on released 0.5.0; [Host #120](https://github.com/dragginzgame/ic-host-tooling/pull/120)
at `b3d267e` and [Host #127](https://github.com/dragginzgame/ic-host-tooling/pull/127)
at `7d06ae5` now build on released 0.18.6. GitHub reports all four conflict-free;
hosted checks must qualify these new heads before upstream integration.
Published notes and metadata are preserved, with compatible pending notes updated.
No maintainer sibling checkout or Canic dependency graph was changed.

Focused evidence passes: Build configuration has three public cases, four input
mutation/include/alias cases and one compiler-identity case on Rust 1.91; flags
has its real-Cargo harness with five scenarios on Rust 1.91. Host duration has
two cases and filesystem primitives have 69 durable cases on Rust 1.88. Strict
affected package lint, warning-denied Rustdoc, formatting and diff checks pass.
Retained checkouts/logs are under `/tmp/canic-upstream-prs-20261015/`.

Backup custody/process integration is delivered. Further generic engine adoption
and live Root coordination are distinct: publishing Host filesystem helpers does
not implement Root-issued checkpoint sequences, complete-unit exclusion, paid
snapshot reconciliation or restore/release authority. The accepted live-backup
deferral and installed participant proof are recorded on
[#550](https://github.com/dragginzgame/canic/issues/550#issuecomment-6189432882).
The registry now has Build 0.5.0; normal Canic adoption still needs removal of its
remaining fixture-only test-target helper callers. Blob remains published at
0.26, so the Memory 0.40 mismatch still blocks the narrowed release batch.

## Shared Tooling 0.15 adoption — 2026-10-15

Refreshed the existing snapshot selection from committed upstream HEAD
`3bef3f2daec7f64f875397ed4d53d5b9e2e68a08` (version annotation 0.15.0),
verified against the remote HEAD. The exporter now records 115 payloads, including
the required pure version/date validator. This is exact-commit adoption, not a
claim that every pending feature listed in upstream changelogs has shipped.
Canic's isolated release fixture now includes that required helper. Current
reporting rules count genuinely created/deleted symbols only; changelogs describe
outcomes and consumer actions without structural symbol inventories.

Snapshot integrity, governance closure (227 references across 47 documents) and
Canic's real-Cargo/fake-Git transaction/rollback fixture and shared release
command simulation pass. Focused ShellCheck with sourced companions and Perl
syntax admission also pass. Repository description
matches the maintained framework scope. Evidence and incoming dirty-file copies
are retained in `/tmp/canic-shared015-adoption/`. The 0.112 notes and adoption
guide identify the new revision. No application dependencies, own package versions,
Git index/refs or release records changed. The previously recorded Memory/Blob
integration blocker is independent of this tooling adoption.

## Accepted release scope and current blocker — 2026-10-15

The maintainer selected **ship integrations; keep live backup unavailable**.
The 0.112 draft now explicitly defers Root-paid capture, membership exclusion and
coordinated restore/release under #550. Pending unpublished Host/Build extractions
and the Auth redesign do not gate this narrowed release. Existing Backup custody
and checkpoint participant SDK remain included. Live create still returns its
typed unavailable error before resolving or creating any output; dry-run is only
planning. This decision supersedes earlier handoff statements requiring complete
live Backup or every pending upstream extraction for 0.112.

The dependency graph changed during checkpoint qualification: the root requests
Memory 0.40 and selects Metrics 0.10.1, while Blob 0.26 uses Memory 0.39.
Offline metadata reconciled the root lock without updating other selections.
The participant fixture checks successfully on Memory 0.40, but affected internal
lint cannot compile the Blob adapter because its public memory declaration types
come from different crate versions. Registry inspection confirms Blob 0.26.0 is
the newest published release; sibling 0.27 is a draft targeting Memory 0.40.
Do not bridge these types or silently revert the maintainer's Memory selection.
A published matching Blob family and aligned private consumer locks, followed by
owning adapter/composition checks, remain required before declaring push readiness.

The new installed participant fixture covers business freeze, lifecycle recovery,
delayed requests and snapshot restoration; it is not Root orchestration. The SDK
now exposes its canonical checkpoint binding. Qualification evidence is retained
under `/tmp/canic-112-checkpoint-20261015/`. The installed participant case now
passes with a Memory 0.40 Wasm; its native driver was built before that graph
change. This is a scoped participant proof, not complete current-graph validation.
The participant fixture also passes warning-denied lint, and the registered-case
partition regression passes using the retained native driver. Internal lint is
blocked before reaching the new test module by the Blob memory mismatch.
The CLI live-create refusal regression also passes on the current root graph:
new output is not created and retained output is unchanged.
The initial fixture omitted activation and was corrected; the next attempt was
stopped by the changed locked graph before execution. No broad gate, package
version change, commit, push or release has run.

## Remaining fixture Cargo observation — 2026-10-15

The registry index still selects IC Build 0.4.0 as the newest published release;
0.4.1 remains an upstream documentation draft. Canic already meets its Rust 1.91
floor and has no callers of the removed Wasm-path convenience functions.
The embedded-peer source builder now uses IC Build's canonical metadata
observation with 16 MiB stdout and 1 MiB stderr limits. Its locked/offline recipe,
local-version normalization and external-lock preservation remain Canic-owned.
The internal testing package explicitly selects the existing Host process
workspace dependency for capture limits; no dependency version changed here.

Four fixture regressions, strict internal-library lint and current-source
embedded-peer verification pass on Build 0.4.0, Host 0.18.5 and the maintainer's
Timers 0.23.1 selection. The explicit relocation/version real-Wasm test remains
ignored; it was not run. Evidence and task before-images are retained under
`/tmp/canic-build04-continuation-20261015/`. Duration PR #120 is still open;
upstream scratch and Cargo configuration/flag owners remain pending. This
completion does not make the full 0.112 batch push-ready: coordinated Backup and
remaining upstream delivery are still open. Changelog notes are updated; package
versions, Git index and publication remain untouched.

## Blob 0.26 adoption — 2026-10-15

The adapter now delegates continuity preparation, version observation, resume
orchestration and certificate issuance to published Blob 0.26.0. Both private
consumer locks select Memory 0.39.0, Metrics 0.10.0 and Timers 0.23.0 alongside
Blob 0.26.0, matching the framework without updating unrelated transitive pins.
Memory diagnostics now expose the current snapshot; Metrics projections use the
current aggregate API. Testkit 0.44 callers use current capture/restore, error
fields and native PocketIC time access. Remaining nested nullable-decoder callers
now use Contracts directly.

The adapter check, 15 Memory cases, 17 Metrics cases and the Memory Candid
roundtrip pass. Strict Blob/Core/Host/CLI and changed integration-test lint pass;
both Host nullable-owner and capacity-import decoding regressions pass.
The embedded allocation peer was refreshed and its verifier
passes. Dedicated and embedded managed composition cases pass on IC Build 0.4.0 and
Host 0.18.5. The root catalog changed during the first embedded build, invalidating
its follow-up cache admission; both cases were rerun on the final stable graph.
Adoption evidence is recorded on [Blob #72](https://github.com/dragginzgame/ic-blob-storage/issues/72#issuecomment-6187960855).
Retained evidence and source before-images are under
`/tmp/canic-blob026-20261015/`. This is framework qualification; application/provider
acceptance and coordinated live Backup remain separate unfinished obligations.
The complete 0.112 batch is not yet ready to push or publish; package versions
and the Git index remain untouched.

## JSON and timestamp consolidation — 2026-10-15

Fleet and build-lock records now call Contracts' required-nullable decoder;
Backup validation, runner and CLI report consumers share a pure timestamp parser.
Missing/null semantics and caller-specific malformed-timestamp policies remain.
Five timestamp-filtered, three Contracts serialization and 18 Backup preflight
cases pass. Host's owning nullable-field regression cannot compile because the
current Core dependency adoption has unrelated Memory diagnostic and Metrics
summary API errors; CLI qualification shares that dependency boundary. No broad
gate or unrelated dependency repair ran. Source before-images and logs are in
`/tmp/canic-json-time-20261015/`.

[Host #122](https://github.com/dragginzgame/ic-host-tooling/issues/122) owns the
prepared invocation scratch API at `/tmp/canic-host-scratch-20261015/`. Four
focused Linux cases, strict lint, Rustdoc and Rust 1.88 checks pass. The Canic
adoption patch passes applicability checking but waits for registry publication;
current Candid argument-file handling remains intact. This slice does not make
the complete 0.112 batch ready to push or publish. Package versions and the
current checkout's Git index remain untouched.

## Staging and text extraction — 2026-10-15

Download and restore staging now delegate private directory creation to published
Host FS. Native I/O kinds and before/after-creation evidence remain available;
Canic owns journal sequencing, namespace custody and stale-stage cleanup.
Twelve staging cases, 36 restore-runner cases and strict Backup library/test
Clippy pass in the isolated package workspace
`/tmp/canic-backup-staging-20261015/`. The main locked invocation refuses the
concurrent Testkit catalog 0.44 / lock 0.43 mismatch before compiling. Its lock
was not changed; isolated evidence is not full current-workspace qualification.
Before-images, checks and the pending duration adoption patch are retained under
`/tmp/canic-extractions-20261015/`.

[Host PR #120](https://github.com/dragginzgame/ic-host-tooling/pull/120) supplies
compact duration formatting, with focused Linux tests, strict lint and Rust 1.88
checks. Canic's direct CLI adoption waits for publication; no live path override
or compatibility reexport is added. Native macOS remains upstream qualification.
The proposed complete Wasm-tool relocation was narrowed after reading Host's
ownership contract: pins, installation policy, archive recipes and differing
admission requirements stay local. The remaining generic bundle-publication gap
is tracked by [Host #121](https://github.com/dragginzgame/ic-host-tooling/issues/121).
Existing Host directory PRs own deeper staging primitives; no competing owner
was added. These scoped changes do not complete the accepted 0.112 batch or make
its changelog/version surfaces ready for publication. Canic remains uncommitted.

## Published Cargo observation owner — 2026-10-15

Discovery, role-package admission and complete-build export inventories now use
published IC Build 0.3.3's bounded observations and canonical Cargo metadata types.
The private schema, feature argument builder and unbounded capture are removed.
Canic retains commands, 16 MiB stdout/1 MiB stderr limits, sanitized diagnostics
and package/recipe authority; the existing #553 export repair is preserved.
The catalog minimum is 0.3.3; no source override or own version change is added.

The 41 role, 44 reuse/export, seven Fleet, 18 bootstrap, ten artifact and four
canonical-selection checks pass. Strict Host library/test Clippy passes; actual
Rust 1.91 library checking passes in a frozen copy on the unchanged incoming lock
`1fb060c8029f246f1841f67d2d6c61c8e8bda51cc3c07ef03fa10d35db64372f`:
Build 0.3.3, Host 0.18.3, Testkit 0.43, Memory 0.38, Metrics 0.10, Timers 0.22.6.
The opt-in real-Cargo cold/warm/relocation/source-change matrix also passes
in that copy; its synthetic release sealing is not installed Fleet acceptance.

Another task raised the live Memory requirement to 0.39 during qualification;
the live locked-check refusal is retained separately. The older graph's passing
checks do not qualify the independent adoption or current installed artifacts.
Build #33 and Canic #530 retain delivery, native and current-graph/application
acceptance. Source and pending 0.112 notes stay uncommitted; no version, release,
publication or live effect ran. Before-images, failed attempts and frozen-source
evidence: `/tmp/ic-build-canic-adopt033-20261015/`.

## Selected build-script inputs — 2026-10-14

The local [#553](https://github.com/dragginzgame/canic/issues/553) correction
retains Canic include exports from current Cargo JSON messages, associated with
exact compiler-artifact package IDs and canonical manifests. Config, profile,
network, target and build environment bind each observation recipe. The selected
App and generated Fleet catalogs admit producer records through Host's bounded
descriptor reader; IC Build still owns input evidence and before/after refusal.
Same-name foreign workspaces, malformed/nonregular foreign records and replaced
selected records have owning regressions. Selected external includes remain
observed before depfiles exist; late discovery and byte changes still refuse.

All 44 reuse, 10 artifact and 18 bootstrap cases, the opt-in real-Cargo cold/warm/
relocation matrix and strict Host library/test lint pass on the final root graph.
The matrix seals synthetic release records; it is not an installed Fleet proof.
The root lock changed externally during qualification to Timers 0.22.6 at SHA-256
`1b7633d5a6ad5aa7fceffc864d4aa03a82b5e5f2dcdea6327a95436d12444283`.
Catalog and Shared snapshot remain unchanged. Earlier installed composition and
embedded-peer receipts, and both private Blob locks, retain Timers 0.22.5; they
are not current 0.22.6 installed acceptance. Evidence/before-images are retained
under `/tmp/canic-553-20261014/`. One intermediate real-Cargo fixture failure
exposed the already-diagnosed shared-target build-script clobbering hazard;
foreign input acquisition is qualified independently from subsequent recompilation.
The full 0.112 batch remains open for coordinated Backup, upstream ownership,
delivery/native-host acceptance and new-graph alignment/installed qualification.
No version, Git publication, release or live effect ran.

## Previous adoption and first sediment removals — 2026-10-14

The maintainer requested latest Shared Tooling/Host adoption and implementation
of the audit findings. That adoption selected Host 0.18.2 (including CLI-only Text),
Build 0.3.2, Backup 0.24.5, Blob/contracts 0.25, Query 0.55.8, Testkit 0.42,
Memory 0.38, Metrics 0.9.6 and Timers 0.22.5. Both private Blob locks now select
Blob 0.25/Memory 0.38 and the same Metrics/Timers family. Root lock SHA-256:
`1a9464b6c5ce5fc86c877947c341d24672b8fbcfd622c33991d7962ff444951c`.
Shared's clean main snapshot is 0.13.3 at
`d963db4b85b4fa65560fd87e8398f228955a2b71`, with 114 verified payloads.
Its selected payloads match 0.13.2; the future version/date/prose helpers in its
notes remain pending PRs and are not adopted as implemented APIs.

The obsolete Memory reservation callback/direct-call fixture is removed (#559).
Bootstrap admission, policy identity and explicit namespace grants remain.
CI/dev callers use the maintained shared CI installer and common lint pins;
additional platform digests and Canic tool policies remain local (#563).
Native query fixtures use bounded Testkit HTTP acquisition; ordinary executable
fixtures use Host's fork-safe writer. Fleet progress delegates text escaping to
Host, while interactive clipping, ANSI layout and screen custody remain local.
Backup layout/restore/CLI record acquisition shares Host's bounded descriptor
reader, with a matching 64 MiB publication limit (#562). Snapshot artifacts have
independent limits; schema/terminal receipt validation and path custody remain.

The 27 memory cases, 47 Host fixture cases, 11 JSON/durability cases, 36 restore
runner cases, 42 CLI restore cases (one ignored), 33 progress cases and strict
affected Core/Host/Backup/CLI library/test lint pass on their recorded inputs.
Host 0.18.2 reruns pass; source/lock drift checks retain their exact scope.
Snapshot/governance/link checks and the four-platform installer/Make fixtures,
including Bash 3.2 admission, pass. Testkit 0.42/PocketIC 16.1 are prepared and
admitted. The allocation peer is refreshed and read-only verification passes
against the selected lock. The subsequent Timers 0.22.5 update is fetched and
both private locks aligned; its 27 memory and six timer cases and refreshed-peer
verification pass. All three installed cases pass on that unchanged final graph:
dedicated composition, embedded composition and production Root initialization/
recovery. A declaration build first ran out of space; `target/` was then removed
externally. The retry rebuilds and passes without agent cleanup. Original failed
and earlier-graph receipts remain distinct. Final four-package strict lint also
passes on the unchanged Timers 0.22.5 graph.
Evidence and incoming dirty before-images: `/tmp/canic-removals-20261014/`.

The complete 0.112 batch remains **not ready to push or publish**. Coordinated
live capture/restore (#550/#394), generic Backup engine convergence (#513),
selected generated-export evidence (#553), Build metadata ownership (Build #33)
and shared Blob continuity/certificate orchestration (Blob #72) remain open.
Auth redesign is deferred. No broad gate, Git publication, package version or
live effect ran. Earlier receipts below retain their original graphs.

## Shared Tooling and recent changelog layout — 2026-10-14

The maintainer requested the latest Shared Tooling adoption, local PR inspection
and revision of recent release notes. The canonical exporter selects clean source
0.12.8 at `eafa07cac1fdba7d6303d061ed8af458aacf32b7`, verified against remote main,
with 114 payloads. New derived guides describe Rust preparation and CI change
selection; this refresh changes no tool pins or Canic workflow selections.

The new shared readable-layout policy replaces the local one-bullet restriction.
Root and detailed notes for pending 0.112, published 0.111, 0.110.54 and 0.110.53
now separate outcomes and upgrade actions, keep descriptive issue links at their
owners and distinguish live-backup/graph limitations from implemented mechanisms.
Intermediate 0.112 adoption narratives are consolidated; all existing issue
destinations remain represented. Older historical sections, published version
identities/dates and the recorded 0.110.53 minor-boundary conflict are retained.
The pending version remains 0.112.0 because public Rust, access, memory and journal
contracts change; contributor-only Shared Tooling adoption is not its breaking cause.

Both open local source PRs are inspected without modifying their branches:
#546 at `f76ea5bf3835462920f8f4ff383cb6b979aecd95` and #519 at
`a25f4fd22eab23e1a2a5fa9947a9bbfa064e25a0`. Their exact hosted Linux checks,
ordinary tests and MSRV pass; both PocketIC jobs fail the embedded allocation-peer
preflight before server/test startup. Both macOS lanes are queued at inspection.
Existing #551 owns the repair, which already has active isolated-checkout work;
no duplicate implementation or integration is performed.

Snapshot integrity, source-owner refresh fixtures, documentation links and the
existing compiled changelog identity test pass. Evidence/before-images are under
`/tmp/canic-shared-changelogs-20261014/`; the identity test reads the current notes,
and is not a current-graph Cargo qualification. This scoped documentation/tooling
task is complete; the full integration batch remains not ready to push/publish
for the previously recorded runtime graph and live-backup work. No broad gate,
Cargo/version/index/ref mutation, PR merge or live effect ran.

## Maintainer Cargo update recheck — 2026-10-14

The maintainer updated the root catalog and lock. It now selects Build 0.3.0,
Backup 0.24.3, Blob/contracts 0.23.4, Host 0.17.13, Query 0.55.7, Testkit 0.40.0,
Memory 0.37.0, Metrics 0.9.4 and Timers 0.22.3. Auth redesign remains deferred.
Root lock SHA-256 is
`a03c6dd8306baabd8d738ea4305b0875256f468cfbb0cc8eb0530ede16a90a45`;
both private Blob locks and Shared Tooling 0.12.5 remain unchanged.

The focused initial compile exposed Memory 0.37's allocation-summary hard cut.
The Canic projection and direct report assertions are corrected; all 15 owning
allocation/bootstrap/conservation cases and eight memory-metrics cases pass.
The prepared Testkit 0.40 caller migration is applied: borrowed-server construction
uses `build_instance`, and standalone/root/registry baseline captures transfer an
explicit instance owner. Governed internal library compilation and strict lint
pass. Strict all-feature Core/Host library/test lint passes after correcting the
proposal's map/unwrap lint. Original failed attempts are retained.

The remaining compile blocker is published Blob 0.23.4's Memory 0.36.4 dependency.
The root graph contains both Memory versions; the adapter cannot return Blob's
0.36 `MemoryRequest` as Canic's 0.37 request. Crates.io still reports 0.23.4 as
the latest Blob release at this inspection. Its owner already records the pending
0.24.0 adoption under [Blob #68](https://github.com/dragginzgame/ic-blob-storage/issues/68).
No duplicate PR, local override or conversion between runtimes is introduced.
Both private consumers still select Memory 0.36.4 and Timers 0.22.2; align all
three graphs with the published compatible Blob family before installed checks.
[Canic #548](https://github.com/dragginzgame/canic/issues/548) and
[#549](https://github.com/dragginzgame/canic/issues/549) retain their delivery and
installed qualification requirements; the owning issues have current evidence.

Earlier Blob/PocketIC and embedded-fixture receipts bind the preceding graph.
The embedded fixture's recorded lock differs from this selection; current-source
verification/refresh remains required after the Blob compile blocker clears.
No updated-graph installed journey or broad gate is claimed. Evidence and source
before-images are in `/tmp/canic-update-check-20261014/`. The complete 0.112 batch
remains **not ready to push or publish**: graph convergence and installed checks
remain, as does coordinated live backup under #550/#394. Changelog surfaces retain
the open 0.112 draft; own package versions, Git refs and live effects are unchanged.

## Previous registry graph and checkpoint participant — 2026-10-14

The maintainer resumed implementation and requested adoption plus owning issues/PRs.
The root now requires published Build 0.3, Backup 0.24.3, Blob 0.23.4, Host 0.17.12,
Query 0.55.6 and Testkit 0.39. Auth redesign is explicitly deferred. The root lock
selects those releases, Memory 0.36.4, Metrics 0.9.4 and Timers 0.22.2. Both private
Blob consumers select Blob/contracts 0.23.4 and those same Memory/Metrics/Timers
versions; their unchanged Auth selection remains separate.

Shared Tooling 0.12.5 is adopted from clean committed source
`4256171b5b7ec4c4cc4078548c1c1617ee150699`, with 112 verified payloads and unchanged
common tool versions/checksums. Snapshot, governance links, IC installer fixtures,
Canic formatter/tool-recipe fixtures and actual prepared tool checks pass.
The selected Testkit 0.39 runner and PocketIC 16.1.0 are prepared explicitly.

On this graph, 41 Build reuse cases (one optional performance case ignored),
30 Query receipt cases, 309 Backup native cases and 42 CLI restore cases (one
ignored) pass. Strict affected Host/Backup/CLI library/test lint passes. Generated
Cargo metadata requires the explicitly inherited offline setting for offline
qualification; the initial network-resolution failure remains recorded.

The interrupted checkpoint draft now has an application-facing SDK record API,
exact original restore-source retention and durable release high-water behavior.
Wire and stored records share one canonical phase enum; the duplicate is removed.
The SDK endpoint predicate authenticates the issuing Root against Canic's
independently installed receiver authority. Root recovery authentication remains
independent from ordinary application admission; it confers no freeze/effect claim.
All 15 participant ordering/replay/serialization/Candid and authority cases and strict Core/SDK
library/test lint pass. This qualifies local transitions, not application freeze
or IC effects. [#550 — Coordinated Fleet backup checkpoints](https://github.com/dragginzgame/canic/issues/550)
owns remaining installed-Root authority, membership exclusion, application hooks,
Root-paid capture and outside-snapshot same-release restore/release qualification.
Live creation remains unavailable under #394; generic engine adoption stays #513.

Exact producer hosted results are separate: at inspection Build 0.3 CI is queued; Host 0.17.12's
run is cancelled. Testkit 0.39 passes the other eleven hosted jobs but its Intel
checks fail in the synthetic async peer fixture. Its owner has a pending repair
under Testkit #68 and a separate unpublished 0.40 ownership cut; Canic #548 owns
that caller migration after publication. No local override or premature 0.40
selection is introduced. Earlier producer failures are historical.

Current evidence and source before-images: `/tmp/canic-adoption-20261014/`.
All three installed Blob 0.23.4 journeys and all six Testkit payload-limit cases
pass on the selected graph. The embedded fixture is refreshed and read-only
verification passes. The final participant/guard source is explicitly refreshed
and its read-only source/toolchain/artifact verification also passes. Blob/payload
receipts retain their original artifact/source scope before the unused SDK-only
refinements; their consuming behavior is unchanged and is not silently rebound.
The complete
0.112 batch remains open and not ready to push or publish. No broad Canic gate,
version, Git publication, live deployment or paid effect ran.

## Framework live-readiness completion — 2026-10-14

The maintainer requested completing Build, Blob, Auth, Backup, IC tools and
Shared Tooling, and selected **Canic framework only** for this qualification.
The existing 0.112 batch stays open; package versions stay 0.111.0.

Build 0.2.6 now owns bounded selected Cargo-depfile reads, parsing and input
evidence. Unrelated App records no longer contaminate selected recipes; malformed,
changed and newly observed selected inputs refuse. All 40 reuse cases and the
actual generated-input preflight pass. Published Query 0.55.5 owns certificate,
tip and bounded block-chain validation; 30 receipt cases preserve Canic mint
binding. The Store's installed held-reply/Root-restart recovery case and all eight
native-agent cases pass on their recorded graphs. All three Blob framework
journeys and all eight native-agent cases pass on the frozen selections.

The separately authorized Host child transfer API is published in 0.17.8 and
selected by Canic. Backup uses published 0.24.2 journal, command and layout guards.
Original command identities are persisted before dispatch; missing/replaced
custody refuses. Fresh complete terminal bytes and exact quiescence govern
reference release. All 309 Backup cases, 42 CLI restore cases and the one-spawn,
bounded-output Host regression pass. Strict Backup/Host/CLI library/test lint
passes with all features. The registry-only custody composition proves descendant
exclusion, cancellation and output-overflow cleanup. Local generic lock/reference
engines and raw descriptor handles are removed. Live create remains unavailable:
actual Fleet capture consistency/authority and a same-release capture/restore
journey remain required under #394/#513.

Shared Tooling 0.12.0 is adopted at f7fe31bf80a2b445bf37a470527f001210241aea,
with 111 verified payloads. Governance links, snapshot/release adoption, IC installer,
formatter and local tool adapters pass. Exact prepared host/IC/Rust/Testkit tools
pass offline checks. The selected manifest, lockfiles and snapshot are frozen;
qualification checks their hashes, with receipts under `/tmp/canic-112-completion/`.
The stale embedded allocation peer refusal was reproduced, explicitly refreshed
from these inputs and then passed read-only verification. Its old bytes and failed
receipt remain retained. Exact Host 0.17.8 CI passes Linux/MSRV but both macOS lanes fail; the Intel
log proves a nonportable FIFO fixture. Producer correction/adoption and exact
native acceptance remain required. The frozen selections remain unchanged.

Auth's accepted pending cut is now 0.6, including `session-restore`, explicit
storage/reset policy and receiver-wide `SessionAdmissionBudget` (#58/#59).
Published 0.5.10 does not provide those APIs; its current-source consumer proofs
cannot qualify the pending contract. Shape validation and coordinated Canic
framework adoption remain open. Downstream application/provider acceptance is
outside this maintainer-selected framework scope.

Evidence and before-images: `/tmp/canic-112-completion/`. Linux proofs do not
establish native macOS acceptance. The complete batch is **not ready to push or
publish**. No broad Canic gate, version/Git/publication or live effect ran.

## Testkit 0.38 payload-limit caller migration — 2026-10-14

The previously approved caller migration under
[#540](https://github.com/dragginzgame/canic/issues/540) is applied to the existing
0.112 batch. The suite uses Testkit's standalone recipe and baseline pool with
one slot, lazy construction, retained canister metadata and the original explicit
cycle top-up. Payload, dispatch, lifecycle and Candid-boundary assertions remain.
The already-selected Testkit 0.38 requirement and own package version are preserved.

Focused compilation and all six tests through `make test-pocketic-case
CASE=pic_ingress_payload_limits` pass; the live artifact observes one cold build
and five warm restores. Exact CLI preparation was explicit after offline admission
refused the missing selected 0.38 runner. An initial source-cache miss was prepared
separately with locked fetch, without dependency resolution.

Strict lint is incomplete: concurrent Host reuse changes trigger too_many_lines
in input_snapshot; the caller-only attempt then stops on missing IC Query
certificate-host/ledger-evidence-host features in concurrently changed receipts.
Those inputs remain intact. The tests qualify their compiled artifact, not the
whole concurrently edited batch. Logs: `/tmp/ic-testkit-issues-20261014/`, with
full live output in `target/test-runs/20261014T065121Z-3666646.8CJlEX/1.log`.
Delivery, complete current-source validation and native macOS acceptance remain
outstanding; Host 0.17.8 is still unpublished. No version, index, ref or release
mutation ran for this migration.

## 0.112 duplication-removal plan and Query extraction — 2026-10-13

The maintainer requested the existing 0.112 design's duplication-removal plan
and implementation, then separately authorized scoped IC Build/Query work.
The [design](../../design/0.112-standalone-blob-service-extraction/0.112-design.md)
and [batch status](../../design/0.112-standalone-blob-service-extraction/status.md)
record Backup custody, Build input evidence, read-only evidence and Auth owners.
The concurrent published Build adoption below is preserved rather than repeated.

Canic local replica queries now use IC Agent's envelope/response owner. Delete
the private CBOR codec and golden codec tests; retain environment/readiness
projection, anonymous selected-origin HTTP, no proxies/redirects, one request
on HTTP refusal, typed rejection and the 30-second overall timeout. All 13
owning HTTP/projection tests and strict all-feature Host library/test Clippy pass.
Logs: `/tmp/canic-112-replica-tests-final-pass.log` and
`/tmp/canic-112-host-clippy.log`. Earlier source-race, sandbox and fixture failures
remain retained and are superseded only for their corrected source/command.

IC Query's uncommitted 0.55.5 draft now owns bounded certificate/witness/tip
authentication and immutable bounded generic block-chain validation. Nine new
cases, 18 directly affected report/certificate regressions and scoped strict
library/test lint pass. The default library remains free of the optional host
engines. A frozen Canic candidate replacing its certificate/value-budget/chain
engines passes all 30 receipt cases and strict Host library/test lint, preserving
network/account/memo/fee/amount binding, Candid structure and original mint
authority. Its source override exists only under
`/tmp/canic-112-certificate-consumer-02/`; the real Canic graph remains registry-only.

Query publication and actual registry adoption are still required before deleting
those remaining Canic receipt engines. Backup's original-command/terminal custody
integration and Auth's published validation adoption remain open. The complete
0.112 batch is not ready to push; source counts establish no build-time or Wasm
improvement. Own package versions, Git refs, releases and live/paid effects were
not changed by this task; independent maintainer work retains its own authority.

## Published IC Build input-engine adoption — 2026-10-13

The maintainer authorized Canic integration after publishing IC Build 0.2.2.
The catalog now requires that registry API; the incoming lock already selected
0.2.2 and remains unchanged. Its archive checksum is
`d408281094f1ae6928f02102131a904b8edcf5693b6c8f01b105e6488d0c4d82`, with
VCS receipt `e9be97fa356c14cb5431b879e304a0f8cc5077c3`.

[#530](https://github.com/dragginzgame/canic/issues/530) now delegates per-path
input capture/digests and before/after mutation guards directly to IC Build's
`InputEvidence`; selected retained Cargo records use its strict discovery API.
The private scanner, snapshot module and parser are removed. Two remaining
generic regressions move to IC Build, whose complete Linux gates pass; Canic
keeps product recipes, required toolchain admission, generated export/fixture
selection, source custody, complete-release format/namespace, finalized release
bindings, output projection and diagnostic serialization. There is one generic
traversal owner and no compatibility reader or incidental cleanup.

The targeted Host reuse suite passes 40 cases, including real Cargo generation,
external/disappearing input evidence, cold/repeat release reuse, environment/tool
changes, tampered outputs, rejection diagnostics and cancellation. Its opt-in
synthetic-sealing performance case remains ignored. Evidence and task-owned
before-images are under `/tmp/ic-build-canic-022/`; `canic-final-reuse-tests.log`
owns this pass and `canic-final-clippy.log` owns the strict Host library/test
Clippy pass. The incoming graph selects Host 0.17.3/Query 0.55.4; locked cache
preparation is explicit, with lock SHA-256
`0b30060e6d39ff08ac2c54b2d07de74b21f10863e19426ebba89a5630d78bc10`
unchanged through final checks. The query owner resolves the earlier duration
lint; initial receipts remain retained. Required tool absence still yields the
original NotFound failure rather than optional source evidence. This is focused
Linux evidence, not broad workspace or native consumer acceptance.

IC Build 0.2.1's exact release CI passes Linux and both native macOS hosts;
the newer 0.2.2 Intel CI fails an inadmissible invalid-byte filename in its new
test fixture. IC Build #15 owns the local repair and exact native acceptance;
Apple Silicon remains queued. Complete Canic native/application acceptance remains
outstanding. The wider Auth/Backup/Blob integration batch stays open under 0.112.0.
No package version, staging, commit, push, release, publication or broad gate ran.

## Backup lock adoption and pending Auth/Build APIs — 2026-10-13

Canic Backup now uses the published IC Backup 0.24 journal lock/error directly.
The redundant wrapper and generic lock tests are removed; upstream retains
contention, unsafe-entry and process-death coverage. Canic's restore-reference
retention and completed-restore lost-response replay cases pass, with strict
package library/test Clippy. Evidence: `/tmp/canic-backup-shared-lock-*.log`.
This prepares guarded reference-release adoption but does not complete #513 or
enable live backup creation (#394). Product journal and command custody remain.

The selected Auth 0.5.10 and Build 0.2.1 lack the next shared APIs: scope/grant
validation (Auth #50) and standalone input evidence/depfile discovery (Build #12).
Their sibling checkouts contain uncommitted implementations for 0.5.11 and 0.2.2;
no sibling edits or local dependency overrides were made. Adopt published APIs
before retiring the corresponding Canic implementations. The release batch stays
open; package versions and Git refs are unchanged.

## Native-agent Memory diagnostics repair — 2026-10-13

The native-agent integration fixture no longer accesses Memory 0.36.2's private
commit store. It reopens the existing memory geometry and reads recovered ledger
bindings from the public doctor report, without bootstrapping or treating its
empty diagnostic inputs as admission authority. All four fixture lookups use the
raw stable-memory snapshot. The focused byte-preservation regression passes;
strict Clippy for the native-agent target passes. The initial regression used an
invalid key without `.v1`; that test input is corrected and its failure retained.
Evidence: `/tmp/canic-native-agent-memory-clippy.log` and
`/tmp/canic-native-agent-memory-test-final.log`. No broad or PocketIC suite ran.
This compile blocker is cleared; the complete integration batch remains open.

## Published Build 0.2 and Shared Tooling 0.10 adoption — 2026-10-13

The real Canic graph now selects registry IC Build 0.2.0, clearing the Host 0.17
`OutputLimits` mismatch. Strict Host library/test Clippy with all features passes
against the published package; no source override is present. The root requirement
was already 0.2 when this adoption began; the scoped lock update selected 0.2.0
and removed the obsolete Host process 0.16 entry. Other incoming updates remain.

Shared Tooling 0.10.0 is adopted at
`ed9c26f015691859f67b50b512a94709dcdc0c6c`, with 111 verified payloads.
Common host/IC/Rust versions and digests match the previous catalogs. Remove
redundant Canic Make defaults and use snapshot-owned catalogs; retain product
compiler and Testkit selections. Explicit tool setup and all offline tool checks
pass, including the selected Testkit runner. Snapshot governance/link checks,
dependency pins, formatting and both local tool/formatter adapter fixtures pass.
The tool fixture now expects the snapshot's absolute catalog paths; its original
failure is retained. The newly referenced reporting rule is an explicit entrypoint.

Evidence: `/tmp/canic-build020-host-clippy.log` and `/tmp/canic-shared010-*`.
These are focused Linux checks, not the full workspace or native macOS gate.
The reported compile blocker is resolved; the wider integration batch remains
open. The existing 0.112 draft reflects these selections; package versions and
Git refs were not changed by this task. Earlier blocked evidence below is historical.

## Validation fallout after Host 0.17 selection — 2026-10-13

The reported validation exposed missed bench/app paths and the newly selected
Host 0.17 feature split. Host now enables `candid-service`; the lock adds its
`candid_parser` edge without changing versions. Both disposable Hub/Shard apps
provide explicit Memory composition for their rows and fixture-import cells,
using the same named keys for requests and opens. The authorization probe now
declares its existing `probe:read` scope in its component token allowlist.
Remove the upstream crypto microbenchmark, `CanisterSignatureCbor`,
`root_canister_sig_verification_message`, `self_describing_cbor` and its private
domain constant; retain Canic token serialization benchmarks.

Warning-denied Clippy passes for the serialization bench with all Core features
and for the three repaired fixture libraries. The initial fixture visibility
lint was corrected; evidence is retained in `.tmp/memory-036/` under
`serialize-clippy.log` and `fixtures-fallout-clippy-final.log`. No managed runtime
proof was rerun for these fixture repairs, and no broad suite ran.

Before the published adoption above, the focused Host check reached
`canister_build/batch/mod.rs` and fails because IC Build 0.1.3 accepts Host 0.16
`OutputLimits` while Canic selects Host 0.17. Published Build 0.1.4 was also
inspected after the repeated validation failure: its registry manifest and public
grouping API still use Host 0.16, so updating to that patch cannot clear this
error. The maintainer subsequently authorized the sibling fix: IC Build now
selects Host 0.17, with a breaking 0.2.0 draft and unchanged 0.1.4 package
metadata. Its Linux delivery checks pass, including the corrected release
fixture. Canic Host compilation and strict all-feature library/test Clippy pass
against a frozen producer in `/tmp/canic-host017-proof.oEVphD`; the source
override exists only in that disposable copy. See `/tmp/ic-build-host017-*`
and `/tmp/canic-host017-consumer-*.log` for evidence. The published adoption above
supersedes this dependency blocker without relabeling the earlier proof.
Earlier Host 0.16 and Blob managed proofs below retain their exact graph scope.

## Artifact staging owner convergence — 2026-10-13

The authorized [#529](https://github.com/dragginzgame/canic/issues/529) replacement
is applied. Configured-role and infrastructure producers capture completed Cargo
outputs with registry IC Build 0.1.3; temporary allocation and cleanup use its
shared staging owner. Canic keeps product paths, transform/tool selections,
source/output admission policy, release binding, qualification and durable
publication under its build lock. Publication still replaces gzip, Candid and
Wasm in that order after qualifying the complete candidate set. Source capture
retains the existing unbounded read policy; staging is not release/cache admission.
Role-facing size diagnostics recognize the current temporary owner.

The published archive checksum is
`1527d2dfe308fc7c1236b6db42c8c6883635f0d60d1dbbabd28462648058caec`.
The qualified graph selects Host 0.16.0, Memory 0.36.1, Testkit 0.36.0,
Blob/contracts 0.23.0 and Timers 0.21.4. Its lockfile SHA-256 is
`62547e79b68f73a3e6eb3e83b7b92fdea4e58f8ad8ed4ee8fba2a5f4fed9bf94`.
Concurrent catalog updates were preserved; this migration did not rewrite them.

A frozen current-graph checkout at `/tmp/canic-529-registry-qualification/`
uses the registry package without an override and a separate target. All 45
focused checks pass: 23 artifact tests, two actual Cargo-output tests, three
release-binding tests and 17 declaration/extraction tests, including the real
extractor 0.1.6 case. The governed PocketIC binding case remains ignored.
Warning-denied Host all-target/all-feature Clippy and changed-file formatting
pass. Final source custody matches 1,592 owning-package Rust/manifest/config
inputs to the live checkout. Receipts are `/tmp/canic-529-registry-*.log`;
the snapshot and final custody log retain their exact inputs. The earlier
uncached-Timers refusal and its scratch remain retained.

Three production adapters shrink from 1,506 to 1,481 code lines (Cloc 2.10,
comments/blanks excluded). There is one staging/capture engine and no parallel
compatibility flow. The open 0.112 draft covers this change; own versions remain
0.111.0 and Git delivery is user-owned. Native macOS and application acceptance
remain on #529/#528; these scoped proofs do not close the wider integration batch.

## Funding decoder and explicit Memory adoption — 2026-10-13

The local #436 repair decodes the canonical Registry endpoint result once through
the shared query client. All three funding tests pass on published IC Build 0.1.3,
Testkit 0.36.0, Host 0.16 and Memory 0.36.1, including the actual query path,
canonical request decoding and typed remote rejection. This is native codec
evidence, not live funding acceptance. Build's required minimum is now 0.1.3;
the earlier Host public-type mismatch is cleared. The internal testing library
also checks successfully against Testkit 0.36.0. Locked source preparation ran
separately; no dependency downgrade or sibling override was introduced.

The local #527 implementation replaces global memory declarations with explicit
Core/Control Plane requests composed by the lifecycle owner. Store opens and
request factories share unchanged named keys and role/feature selection. Remove
`ic_memory_key!`, `memory_bootstrap_admission!`, the no-argument `MemoryRuntimeApi`
and declaration/pool/admission registration. The pool macro requires a named
pure provider; applications pass their composition to `start!`. Independent
ingress constructors now have a direct Canic-owned `ctor` dependency.

Earlier focused proofs on Memory 0.36.0 pass: 28 Core memory cases, the Control
Plane lazy-store allocation case, two composition cases, strict Core/Control
Plane library/test Clippy and Root/Coordinator/Wasm Store feature checks.
On the latest Memory 0.36.1 graph, both composition tests and all three metrics
facade tests also pass.

Published Blob/contracts 0.23.0 now clear the former Memory-type blocker. Root
and both isolated consumer locks select one Memory 0.36.1 each, without a source
override. Warning-denied adapter Clippy passes. The selected published Testkit
0.36 runner is prepared, and both focused managed Blob cases pass: dedicated
permissions/recovery and embedded application-state retention/recovery. Their
checks include exact installation decoding, endpoint parity and restore fencing.
Root selects Auth 0.5.8 and Timers 0.21.4 for these managed checks; consumer locks
retain their own compatible published selections. The root manifest and all
three locks stayed fixed during both runs.

Dedicated artifacts are retained at build
`39021f01483ac31d8be5ca364ca32605ce38d382c13ad0ee0ae7315d6a5069e8`;
embedded artifacts at
`b6bbcd1b93783b92f3f4325e00f416976495ce228c29f4e783e4551ea52bf6d0`.
Fast-profile code sections are 4.56 MiB (dedicated Blob), 4.57 MiB (embedded),
8.97 MiB (Root), 4.47 MiB (Coordinator) and 3.02 MiB (Wasm Store), all below
12 MiB. These are current fixture measurements, not a before/after reduction or
Toko release acceptance. The first runner took 424s including harness compilation;
the second reused those outputs and took 51s. Logs, input hashes and the former
failed check remain under `.tmp/memory-036/`.

The Blob/Memory dependency blocker is cleared and this scoped adoption is
qualified. The wider Auth/Backup release batch remains separately open; these
results alone do not qualify that whole batch as ready to push. The open 0.112
draft includes the changes; package versions remain 0.111.0. No broad gate,
commit, push, publication or sibling edit ran.

## Async instrumentation size reduction — 2026-10-13

The scoped local [#526](https://github.com/dragginzgame/canic/issues/526) change
shares async accounting across handler types, retaining safe pinned references,
per-poll context restoration and completed-handler destruction inside its scope.
Eleven Core accounting tests and warning-denied all-features library/test Clippy
pass. The exact held-HTTP overlap PocketIC case passes on both baseline and final
source, including both completion orders. Default-feature native tests remain
blocked by untouched Auth dead-code checks; this does not qualify that graph.

The runtime probe's `fast` code body falls from 3,349,506 to 3,338,038 bytes.
After identical `ic-wasm shrink` and Binaryen 133 `-Oz` processing it falls from
2,652,013 to 2,643,053 bytes (8,960 saved), with equal export names/kinds. These
thin-LTO probe measurements are not Toko release headroom. Source/artifact hashes,
commands and logs remain in `.tmp/perf-wasm-headroom/measurement.json` and adjacent
files. No dependencies, package versions, Git refs or sibling files changed.
The open 0.112 draft covers this slice; publication and exact Toko release
remeasurement remain outstanding. The wider integration batch is still open.

The follow-up [#531](https://github.com/dragginzgame/canic/issues/531) experiment
confirmed that baseline explicit access expressions forced async wrappers even for sync
handlers with local-only authentication. Six compiled query/update expansion
scenarios confirm the distinction. Separately making seven public or seven
guarded non-awaiting probe handlers sync saves 1,875 or 3,757 optimized code-body
bytes against the same baseline; the guarded wrappers remain async. Each variant
passes all ten owning timer/observability PocketIC tests. These are independent
`fast`-profile experiments, not a combined or Toko release measurement. Temporary
fixture and macro-test edits were restored byte-for-byte. Evidence and hashes
remain in `.tmp/endpoint-async-audit/measurement.json`; the #526 fix is unchanged.

## Declaration cache owner convergence — 2026-10-13

The maintainer authorized [#528](https://github.com/dragginzgame/canic/issues/528).
Configured-role and infrastructure extraction now use the existing IC Build
0.1.1 artifact transaction API through the same Canic recipe adapter. Generic
key/environment framing, manifests, verification, staging and retained output
ownership have one engine. The private JSON cache reader/writer is retired;
existing files remain untouched. The new store is beneath
`.canic/build-reuse/declarations/ic-build`, with independently materialized outputs
beside it. No dual reader, migration, implicit pruning or package-version change.

Canic retains native extractor admission and invocation custody, normalization,
its output budget, four-worker scheduling/order/draining, and optional-cache
fallback. Cached Candid reads/writes are capped at 4 MiB; larger valid extraction
results are returned without cache publication. Changed declarations or tools
remain fatal even when cache preparation/publication fails.

All 17 scoped extraction/cache tests pass on Linux, including the explicitly
selected real `candid-extractor` 0.1.6 cold/reuse/fresh-byte parity case. Strict
Host all-target/all-feature Clippy and changed-file formatting pass. Test receipts
are `/tmp/canic-528-candid-final.log` and `/tmp/canic-528-clippy-final.log`; earlier
focused receipts and the corrected Clippy finding are retained. These tiny-tool
fixtures do not establish a speed-up or application release acceptance. Root
manifest and lock bytes are unchanged. Source remains uncommitted; native macOS
Intel/Apple Silicon qualification remains on the issue. Other concurrent dirty
work is preserved, and the wider integration batch remains unfinished.

## Synchronous endpoint admission — 2026-10-13

The maintainer authorized implementing [#531](https://github.com/dragginzgame/canic/issues/531).
Builtin caller guards, role-attestation verification, composed access evaluation
and custom predicates are synchronous. The public custom contract is now
`AccessCheck`; remove `AsyncAccessPredicate`, its async-trait imports/attributes
and awaits on local helpers. There is no compatibility alias. Generated wrappers
follow handler synchrony; 25 non-awaiting framework handler definitions are now
sync. Proof issuance, provisioning and renewal keep actual external awaits.
Consuming applications still need to remove auth-only async from their handlers.

Validation passes: 51 endpoint macro tests, 28 Core access tests, two facade
tests, warning-denied Clippy for the four changed libraries, ten timer/authority
PocketIC cases, six payload/denial cases, the exact Registry role-attestation
case and the fresh-issuer token case including wrong-presenter rejection.
The token test had a stale `session` grant outside its configured scope ceiling;
it now requests `verify`, without widening production authorization.
Including Control Plane tests in Clippy still finds unrelated redundant clones
at `ops/component_provisioning/tests.rs:1923` and `:2024`.

Against the unchanged #526 probe baseline, raw code body falls from 3,338,038
to 3,318,649 bytes; identical shrink/Oz processing gives 2,643,053 to 2,630,180
bytes (12,873 saved). Export names/kinds, Cargo inputs and the #526 source hash
match. This is a fast-profile probe, not Toko release headroom. Evidence,
source/artifact hashes, original files and logs are retained under
`.tmp/auth-sync-implementation/`. The open 0.112 draft covers the hard cut;
this scoped change is validated, but the wider integration release batch remains
open. No package version, Git publication or sibling edit was performed.

## Published Testkit selection — current graph

The maintainer removed the Testkit sibling link. Root now selects published
Testkit 0.35.0, archive SHA-256
`3313fa4e49094266268ccc732512933d8f159946a7176cf7213a90f2cfa49228`.
The temporary path exception is removed, and the standard locked server selector
passes using the prepared published 0.35 runner. Root also selects Query 0.55.0,
Backup 0.21.1, Timers 0.21.1, Host 0.15.1, Metrics 0.8.1, Blob/contracts 0.22.14,
Memory 0.35.19, Auth/protocol-types 0.5.7 and IC Build 0.1.1.

Query's current catalog loader is asynchronous. Host calls its maintained name;
fixture callers now await it or drive it on their owning runtime. Ten catalog
regressions and four cache/timing tests pass, including cancellation, cache
preservation, assurance and use inside an existing runtime. Host local-Fleet
and internal-testing library checks, strict Host library/test Clippy, governed
test compilation, dependency inheritance/pinning and locked metadata pass.
Both standard published-runner managed cases pass: maximum-state authorization
uses 989,364 instructions (unchanged 1,000,000 ceiling), with bounded restoration,
cleanup and memory checks; two-target recovery/composed certification also passes.
The embedded preflight detected genuinely stale bytes. Canonical refresh and the
read-only verifier now pass; original artifact/evidence and failure remain retained.
Evidence and held inputs: `.tmp/testkit-registry-adoption-20261012/`.
Earlier source-path proofs below retain their historical graph identity; the
whole Auth/Backup release batch remains unfinished.

## Host 0.15 artifact-interface adoption

The maintainer authorized applying the prepared Host migration to Canic. Incoming
root Host 0.15 and Metrics 0.8 selections were preserved. Canic Host enables
`ic-interface`, uses the moved text API and inspects Candid methods without
building rich argument presentation trees. Borrowed canonical Wasm facts retain
name/kind and metadata checks around transforms; function renumbering remains
allowed. Artifact comparison now includes modes and maps oneway to update,
retaining Canic's internal-export policy and release/type authorities.

Linux checks pass: 24 artifact tests, 12 Candid/extraction/cache tests and strict
Host library/test Clippy. The existing real-extractor case remains intentionally
ignored. Locked metadata passes, all prior package identities/checksums and
Windows dependency selections remain, and the lock adds only the parser edge to
Host tools. Evidence is retained in `/tmp/host015-consumer-apply/canic/`.
[Canic #458](https://github.com/dragginzgame/canic/issues/458) and
[Host #71](https://github.com/dragginzgame/ic-host-tooling/issues/71) own acceptance.
Changes remain uncommitted; full gates and native macOS were not run. This scoped
result does not qualify the other ongoing integration work below.

## Blob exact installation decoding

The targeted [#524](https://github.com/dragginzgame/canic/issues/524) repair retains
zero CONFIGURATION_LIMITS skipping quota and unchanged ordinary endpoint limits.
Its two native dedicated/embedded regressions now pass after the old Auth blocker
cleared. The managed fixture retains exact original initialization bytes, enabling
installed cases to vary only nested application arguments under the real protected
envelope. This helper is host-side test support, not reset/recovery authority.

Both actual managed compositions pass in frozen temporary checkouts at
`/tmp/canic-524-isolated.v5md68w9/`, with a separate owning target and explicit
published Testkit 0.35 CLI / PocketIC 16.1 server. Source snapshots cover Canic and
its selected Testkit path dependency. The dedicated/embedded cases reject unknown
fields and extra arguments, preserve exact stable bytes/configuration and app
state, retain the required Blob fence, and recover through the existing operator
same-instance workflow. Existing permission/capacity and restoration journeys also
pass. Scoped denied-warning Canic/testing Clippy passes. Old attempts remain.

Dedicated build: `f665751a199eb49b43d11b0a38b7c59263ac44dc6d5900e27140a9f858d3f85b`.
Embedded build: `e0225bbcdcb30a50a24c3b28e246abde37fb7fbf3f25d25f2625acc763ccd752`.
The first installed assertion incorrectly expected an unfenced view after failed
reinstall; its original log/artifacts remain. The correction requires the fence,
then performs real recovery rather than directly clearing it.

Original root attempts remain in `.tmp/issue-524/recheck.*`: native successes,
stale fixture-lock refusal, interrupted check during owner build contention,
standard published-CLI admission refusal for source-path Testkit, and a later
16-error Timers 0.21 caller mismatch during that independent adoption. The timer
owner's repaired calls are frozen for the passing cases; fixture locks are aligned
only with the directly required published Blob/Auth/Timers APIs. The corrected
regression and locks are copied back only after original files match the frozen
baseline and its build directory is idle. The whole live Auth/Backup batch is not
qualified by this local proof. #524 remains open pending family delivery; no full
gate, source fallback, package version or Git release effect is introduced.

## Scoped metrics qualification repair

The Metrics maintainer explicitly authorized two tool-resolution assertion repairs,
embedded allocation-peer regeneration and the named held-HTTP PocketIC case.
Warning-denied Host Clippy and all four resolver tests pass on Linux. The canonical
refresh and verifier pass against root Metrics 0.7.3, Host 0.14.9, Memory 0.35.17
and Testkit 0.34.5. Exact Testkit/PocketIC setup and offline admission pass.
The case passes inventory/fixture preflight and starts PocketIC, then refuses the
required `canister_test` Wasm build because unfinished Auth integration leaves
`ISSUER_CANISTER_SIG_DOMAIN` and `IssuerProofPrepareFailed` unused under denied
dead-code warnings. No overlap/completion-order pass is claimed.
[#491](https://github.com/dragginzgame/canic/issues/491) owns that integration;
[#450](https://github.com/dragginzgame/canic/issues/450) retains native qualification,
and [#99](https://github.com/dragginzgame/canic/issues/99)/
[#447](https://github.com/dragginzgame/canic/issues/447) remain open.

Complete repair logs, exact selected inputs, original and regenerated fixture
identities, the missing-runner attempt and failed private scratch are retained in
`.tmp/metrics-qualification-20261012/` outside Cargo clean. Existing integration
work, own package versions and Git index/refs are preserved; no broad gate ran.

## Earlier qualified Blob graphs

Earlier Blob qualification held
root at Auth/protocol-types 0.5.3, Backup 0.20.1, Blob/contracts 0.22.11,
Memory 0.35.15, Timers 0.19.1, Metrics 0.7.2, Host 0.14.7, Query 0.54.13 and
Testkit 0.34.4 / PocketIC 16.1.0. Those selections are preserved. The root Blob
requirements now start at 0.22.11 because the adapter uses its delivered API.

- Cargo.toml SHA-256: `095be1d4f9b1f56d6cfbec6f0e60ec53971b8b8279ff733c524188819dbb9ac9`.
- Cargo.lock SHA-256: `c21bc626993d7845628c1e711d6fd05838ea0b4d22f464399217fa5cb3d39e41`.

Both private Blob locks independently select Auth 0.5.1, Blob/contracts 0.22.11,
Memory 0.35.14, Timers 0.19.0 and Metrics 0.7.1. They retain private package
versions 0.1.0; the adapter uses the main workspace version/catalog. Actual Blob
proofs below bind these fixture graphs and their retained artifacts, rather than
claiming the later root patches were installed in those canisters. Scoped final
Clippy compiles the newest root graph with warnings denied.

## Current integration

Auth's SDK session/backend adoption is implemented under
[#491](https://github.com/dragginzgame/canic/issues/491). The durable backend
stages a session/replay pair, checks all resource limits before mutation and
commits synchronously. Bounded stable vectors and derived indexes replace the
whole-store admission rewrite. The SDK owns replay/admission and live authority
checks; status/audit also recheck current Root policy. Core's generic canister
signature adapter delegates canonical DER, certificate freshness and cryptography
upstream, removing its direct signature-crate dependencies.

Current focused managed resource cases pass on Auth/protocol-types 0.5.7,
Timers 0.21, Host 0.15 and the explicit Testkit 0.35 source library with the
prepared published 0.35 server. Maximum state (2,048 sessions / 4,096 replays)
uses **989,121 authorization instructions**, below the unchanged 1,000,000
limit; one/median state uses 822,370 / 988,222. The maximum case also passes
bounded restoration/cleanup and no cleanup memory-growth checks. Previous
3.12M/2.66M/2.46M/1.17M failures remain historical measurements, not current
failures. Logs and held input hashes live in `.tmp/auth-session-20261012/`.
The current-graph two-target recovery/composed-certification case and real
Registry-bound Root-attestation verifier case pass (16.69s / 18.71s).
Eighty-eight selected native cases, warning-denied Core library/test Clippy and
default/session-enabled Wasm compilation also pass. The changed Timers probe's native API check passes. Direct fixture Wasm Cargo
compilation correctly refuses outside Canic's governed artifact builder; that
separate installed probe was not run. Actual SDK-facade single/combined denial acceptance
remains [#525](https://github.com/dragginzgame/canic/issues/525#issuecomment-6130059637), and publication /
downstream acceptance remain separate. Keep the whole integration batch open.

Auth now owns canonical request hashing, certificate issuance rules, enabled
SEC1/prehash checks and low-S signer normalization. Default request construction
requires no crypto feature. Canic retains configured scope/TTL diagnostics,
issuer/key policy, exact caller/operation custody and committed replay/session
transactions. Invalid raw Canic role projection precedes shared certificate
lifetime validation as a deliberate hard cut; the combined-invalid regression
observes zero proof callbacks. The certificate adapter contracts from 124 to 51
production lines. Token preparation and complete current-policy proof checks
already delegate upstream; the positive-proof cache is retired.
[Auth adoption](https://github.com/dragginzgame/ic-auth/issues/32#issuecomment-6120465762)
and [#491](https://github.com/dragginzgame/canic/issues/491) own remaining acceptance.

Timers 0.21 registration uses the typed initial start policy. Lifecycle one-shots
use atomic scheduled creation instead of separate scheduling/rollback. Retained
registrations remain inactive until their owner schedules. Metrics 0.8 owns the performance aggregate; totals
are `u128`; the public Rust PerfEntry field changes accordingly. Existing u64
views omit unrepresentable totals while retaining the exact boundary value.
No second memory/timer runtime or compatibility wrapper was added.

Backup owns checksums/staging, checked digest validation, journal locks and
private sidecar admission. Canic retains command/descendant custody, original
restore authority and terminal reference release. Root selects published 0.21.1;
the authorized sibling implementation now adds guarded reference release to the
existing 0.21.3 draft, with product terminal admission and exact existing command
custody. Six new native process/filesystem cases and strict Clippy pass on its
held latest graph. Qualification now covers every configured Backup gate target,
including native/PocketIC tests, docs, Rust 1.88 consumers and both archives.
Unchanged tooling/release/hook evidence was reused; all affected checks were rerun
after correcting directory-shaped generic lock admission. Exact Testkit 0.35.1
setup and the original failures remain retained. Publication and Canic
adoption remain separate under [Backup #29](https://github.com/dragginzgame/ic-backup/issues/29)
and [#513](https://github.com/dragginzgame/canic/issues/513);
[#394](https://github.com/dragginzgame/canic/issues/394) owns live Fleet backup.
Do not substitute unrestricted deletion or all-Applied accounting.

The current Canic runner now holds every command guard through reference
publication rather than dropping each before the write. Its existing retention,
lost-final-response replay and active-command regressions pass, together with
strict package library/test Clippy. Evidence remains in
`/tmp/ic-backup-guarded-release/`. This scoped correction does not adopt upstream
custody records or enable live creation; fresh Fleet routing, application
consistency and original paid-effect recovery remain required. The whole
integration batch remains unfinished; no versions or Git effects were performed.

Blob schemas, storage, policy, accounting and installation-memory grants already
belong upstream. The dedicated and embedded hosts retain one Canic memory and
lifecycle owner. Production Root-driven target-bound installation passes on the
delivered 0.22.11 dedicated graph, including controller/target refusals, restart,
lost-response recovery, conflicting rebinding and effect-free replay. The
complete four-artifact Fast build is
`f7aca7dd2fc40c90fcdc1a61d54eaa9c5824268e74551150f273669565830db5`;
the Root journey reuses its verified complete artifacts in 2.69s.
[Blob #61](https://github.com/dragginzgame/ic-blob-storage/issues/61)'s published
endpoint owner now replaces Canic's duplicate declaration block.
Browser/provider, completion verifier, GC, billing and actual application
acceptance remain separate under [#444](https://github.com/dragginzgame/canic/issues/444),
[Blob #31](https://github.com/dragginzgame/ic-blob-storage/issues/31),
[Blob #32](https://github.com/dragginzgame/ic-blob-storage/issues/32),
[Blob #33](https://github.com/dragginzgame/ic-blob-storage/issues/33),
[Blob #34](https://github.com/dragginzgame/ic-blob-storage/issues/34) and
[#494](https://github.com/dragginzgame/canic/issues/494). Framework proof does not
establish provider acceptance or authorize paid probes.

## Focused evidence

Current logs and held inputs: `target/review-validation/upstream-integration-20261012/`.
Session request/retry/conflict and endpoint policy, signer, certificate construction,
token verification/preparation, timer custody, wide totals and u64 projection cases
pass. Scoped Core/Blob warning-denied Clippy and enabled Wasm library checks pass.
Those initial native/Clippy proofs select Host 0.14.2; its later patch to 0.14.3
changed only the four Host selections. The managed and Backup runners were rebuilt
on 0.14.3. All 12 Backup artifact and three layout-custody cases pass on that latest
graph. The first managed timer/metrics attempt caught a missing watchdog initial
schedule argument; it was corrected after the owner finished. Its corrected rerun passes in 10.25s, rebuilding only the runtime probe in
4.62s while reusing both other fixtures. All 15 timer inventory cases pass.
The latest locked independent public client crosses Auth, Backup and Host
type boundaries successfully; the default Wasm library check passes and its
normal/build tree excludes Auth cryptography.
The first direct runtime-probe Wasm check refused the authoritative build-entrypoint
guard; actual canister proofs use the managed Host owner without bypassing it.
Failed/drifted attempts remain distinct; no full workspace or broad gate ran.

Both consumer shapes now have actual current-fixture-graph managed proofs.
The preceding detailed handoff and earlier target-based logs were removed by an
external Cargo clean; retained Wasm artifacts and failed scratch remain. New
completion evidence lives in `.tmp/blob-completion-20261012/` so another Cargo
clean does not erase it.

## Blob completion slice

The two formerly ignored composition journeys now belong to the governed
framework catalogue. A single isolated build worker uses the production CLI,
admits current-source complete cache reuse and supplies finalized manifest-bound
artifacts. Remove the adapter's testing-only dependency; it no longer carries
its own ignored PocketIC test runner. The dedicated, embedded and production
Root journeys pass in 18.05s, 68s and 15.42s. Permissions, two uploader grants,
lost tenant-mutation results, distinct capacity accounting, snapshot fencing,
same-release restoration and embedded counter retention are covered. Production
binding retains controller/target refusals and interruption/exact replay. Its
four-artifact cache reuse takes 2.77s. The final worker run totals 182.57s.

The initial parent-side cache recheck refused the test runner's compiler wrapper;
current-input admission remains in the correctly isolated build owner. Failed
scratch retains that attempt. The exposed timer instruction-total field now
carries `u128` after Timers 0.19; exact Candid/JSON wide-value roundtrip passes.
Existing u64 metric rows omit unrepresentable timer totals. No compatibility
reader or clipping was introduced.

The maintainer explicitly authorized [Blob #61](https://github.com/dragginzgame/ic-blob-storage/issues/61)
in `/home/adam/projects/ic-blob-storage`. Its opt-in contracts macro and standalone
adoption shipped in 0.22.11 at `ebb560f492b6b53925faae8890316fc66bfd6def`.
Canic consumes the actual registry packages through its root catalog and both
private locks, without a Blob source override. The adapter endpoint module is
113 lines rather than 376; the standalone module is 61. Canic retains endpoint
authentication, bounded decoding, actual caller context and the single lifecycle.

Published adoption evidence lives in `.tmp/blob-published-20261012/`. Dedicated,
embedded and production Root journeys pass in 23.80s, 2m03s and 15.12s; the
owning runner totals 177.26s. Both actual 32-method Blob Candid projections pass
strict structural comparison against the exact released standalone contract.
An initial runtime pass exposed missing Candid registrations from an imported
emitter alias. Its run was interrupted and retained; the final adapter uses a
lexical emitter. Both composition cases compare generated Candid against every
callable Wasm endpoint using the existing Host validator. Scoped warning-denied
Clippy passes on the newest root graph. The final source and catalog are
qualified; the earlier compile-only candidate was not delivery evidence. The
catalog and both fixture locks remained fixed throughout these checks. No broad
suite ran.
No Canic commit or publication was performed. Concurrent upstream commits belong
to their owner; this agent did not create them.

Provider/browser, verifier, GC, funding and application acceptance remain the
separate existing obligations. These framework proofs do not close those issues
or make the entire Auth/Backup/Blob release batch push-ready.

## Retained issue fixes and next work

The bounded config-reader fix for [#509](https://github.com/dragginzgame/canic/issues/509)
is applied locally. Host rejects oversized input before all three YAML consumers;
exact limit, missing file, UTF-8, followed/dangling symlink, directory and device
cases pass among all 23 scoped config tests. Host Clippy passes for all targets
and features with warnings denied. Its reported provisioning helper was shortened
without changing the assertions; its owning regression also passes. Both native
macOS jobs now run the config tests; their execution remains pending. Evidence
is retained in `.tmp/issue-509-20261012/`.

These latest Host checks select Auth/protocol-types 0.5.4, Backup 0.20.3,
Blob 0.22.12, Host 0.14.9, Memory 0.35.17, Metrics 0.7.3, Query 0.54.15,
Testkit 0.34.5 and Timers 0.19.3. The final check inputs stayed fixed. Earlier
Blob and Auth proofs retain their recorded graphs; this Host proof does not
relabel their installed artifacts or establish integration completion.

The macOS worker-fixture blocker in [#474](https://github.com/dragginzgame/canic/issues/474)
is corrected locally: the fixture deliberately creates redundant path separators,
then selects one physical path before passing scratch custody. Its previous
trailing-slash `TMPDIR` failure and corrected worker/workspace-runner fixture
passes are retained
under `target/review-validation/macos-scratch-20261012/`, using admitted Testkit
0.34.3 on Linux. Scoped [PR #519](https://github.com/dragginzgame/canic/pull/519)
contains only this fix and compatible draft notes against released main. Its
`baaf95619` source passes both fixtures with released Testkit 0.33.1; native
macOS results are pending in [CI](https://github.com/dragginzgame/canic/actions/runs/38233266774).

[#516](https://github.com/dragginzgame/canic/issues/516) retains the first concrete
provisioning dependency, bounded rejection evidence and exact approved resume
command. Permanent refusals suspend; transport failures back off. Its earlier
managed recovery/permanent-refusal proofs and aligned three-boundary client proof
retain their Auth 0.4.7 / Host 0.14.1 input identity. The installed Toko Root was
not modified, and these fixtures do not constitute application acceptance.
[#517](https://github.com/dragginzgame/canic/issues/517) requires rebuilding direct
consumers on the current exposed family without aliases.

Remaining Auth work includes qualifying the adopted shared signature store and
finishing the session engine's protected authority/transaction contract. Preserve exact
caller/operation records, original retrieval clocks and one certification root.
Complete Auth/Backup integration and their owning recovery proofs before the
requested cleanup slice; guarded reference release remains issue-owned. The
Canic-owned Blob endpoint/composition reduction is complete; application/provider
acceptance remains separate.
The whole integration/release batch is still open, so this slice alone is not a
push-readiness recommendation.

Shared Tooling 0.9.2 is adopted at
`c118062a070a2e238f1eb3989b6fd700f369ce1e`, with 110 verified payloads and
recorded entrypoints. Canic uses the public consumer Make include; paired
formatter commands retain all three workspaces and the custom hook. The unused
generic Cargo installer fixture is retired. Cargo-sort-derives 0.14.1 is prepared
and admitted offline. Focused adoption evidence lives in `.tmp/shared-tooling-091/`;
the directory retains the 0.9.1 preparation and subsequent 0.9.2 refresh separately.
The broader Auth/Backup integration batch remains unfinished. Shipping Rust floor is 1.91.
Sibling repositories remain read-only outside the explicit Blob #61 authority.
Preserve dirty work, archives, failed
scratch and `.canic/local-work/` / incident-repair material. No staging, commit,
version transaction, push, publication, deployment or paid effect ran for the
integration batch. The maintainer-authorized scoped PR above is isolated from it.

## IC Build registry adoption and audit

The maintainer authorized moving generic Testkit build/cache mechanics and
Canic feature-equivalent Cargo grouping to IC Build
([IC Build #1](https://github.com/dragginzgame/ic-build/issues/1)).
Canic now requires crates.io `ic-build 0.1.1`, archive SHA-256
`e6c3b1a1040d38c3f10e9f12fc456e7ba718ceaaa86dc4967ac719b5487e4aeb`,
from published source `dbb17982557174eed29e1a499a2b1a7bb278d5a7`.
The incoming lock already selected this release; raising the minimum requirement
left the lockfile and all unrelated selections unchanged.
The obsolete IC Build sibling-path exception was removed. This audit used the
then-selected Testkit incubation path; current shipping selection is published
as recorded above. No sibling sources were edited by this audit.
The engine preserves v1 cache/stamp/lock bytes and namespaces, returned artifact
lifetime, typed failures, input-change rejection and explicit maintenance.
PocketIC lifecycle stays in Testkit; Canic's complete-release admission, recipes,
credentials, generated inputs and release identities remain consumer policy.
The separate complete-release reuse/finalization path is not replaced by a
generic cache-hit predicate in this batch. IcyDB adoption and native macOS
qualification remain separately scoped. No package versions, refs, tags or
release artifacts are changed by the extraction.

The Host generated-infrastructure input-stability case and all six selected
internal artifact cases pass with registry IC Build 0.1.1 and Host 0.15.1.
Nine direct published-API cases pass for transactional reuse/retention, pruning,
changed-input and malformed-output refusals, nested cache exclusion, real Cargo
feature grouping and structured complete/truncated failure evidence. Duplicate
and empty package names refuse before command-factory effects; an empty overall
list remains an effect-free plan. The real Rust 1.99.0 Wasm fixture passes cold
build, exact reuse, source invalidation and custody of retained old bytes. These
fixtures do not qualify Canic canister deployment. The published artifact engine
is unchanged from 0.1.0; this patch changes grouping admission and diagnostics.

Current evidence lives in `.tmp/ic-build-011-adoption-20261012/`. The manifest
and lock stayed fixed through passing checks at SHA-256
`f291ddc878ced81eac46ddaa406c48409f266a650b3d22fdf2724975282eb343`
and `d1dd5b874b813ef4ce1c9dd17c0a7735b6db6b14b7cca74470860c78c8e10791`.
Dependency inheritance/pinning and scoped diff checks pass. A final optional
cache timing check was terminated while queued behind another build, before
compilation or test execution; it is not counted as passing evidence.

Earlier 0.1.0 evidence and failures remain in `.tmp/ic-build-audit-20261012/`.
The previous Timers 0.21 compilation blocker no longer appears on current source;
the passing 0.1.1 artifact cases supersede that blocked consumer qualification
without relabeling its old inputs. The completed adoption does not qualify the
whole unfinished Auth/Backup/Blob integration batch as ready to push.

Published and consumer-verified grouping findings
[IC Build #3](https://github.com/dragginzgame/ic-build/issues/3) and
[#4](https://github.com/dragginzgame/ic-build/issues/4), plus publication
documentation [#2](https://github.com/dragginzgame/ic-build/issues/2), are closed.
Native macOS qualification and production complete-release convergence remain
with [#1](https://github.com/dragginzgame/ic-build/issues/1); its published-source
[CI run](https://github.com/dragginzgame/ic-build/actions/runs/38288344214) was
queued when inspected. Complete-release admission remains Canic-owned. No broad
gate, PocketIC, commit, own package-version change or release ran.

## Superseded handoff before Blob 0.47 installed qualification

# Current handoff — 2026-10-24
## Scope and working state
- Continue 0.112 Blob/Wire hard cut with typed operations; the batch is **not push-ready**.
- Preserve pending/Backup work; manifests/locks own versions; Tooling snapshot is 0.26.10.
- Follow AGENTS: no Git publication/release or broad validation. Check shared target
  ownership before edits/builds; run focused checks only. Siblings remain read-only.
- [Historical handoff](2026-10-24-blob-wire-precompact.md): superseded evidence only.
## Blob ownership and integration
- `ic-blob-storage` owns the service/storage/funding engines; its contracts crate
  owns schemas/methods/budgets. Canic's [adapter](../../../integrations/blob-service/src/lib.rs) composes endpoints,
  memory grants, synchronous lifecycle, metrics and authenticated funding access.
  Keep those boundary adapters and dedicated/embedded consumer proofs.
- The native [schema adapter](../../../integrations/blob-service/src/schema/mod.rs)
  consumes actual supplier schemas and `SERVICE_METHODS`; no second Blob engine or DID reconstruction.
- Framework composition coverage and live provider acceptance are distinct.
  Keep [#622](https://github.com/dragginzgame/canic/issues/622) evidence with the
  [Blob guide](../../features/blob-storage/README.md). Independent credit/receipt replay remains
  separate under [#494](https://github.com/dragginzgame/canic/issues/494).
  Provisioning #493 and Toko Miner are outside this batch; fixture tests do not close issues.
## Wire: implemented and next work
- Health, Metrics and History now use typed native Wire methods with explicit
  identities and bounded payloads. Their public union selectors are removed.
  Root-to-Store chunk reads now use one bounded Wire contract and byte-buffer replies.
  Protected observability and remaining discovery/control operations still have
  their maintained contracts. Storage and lifecycle encodings are unchanged.
- Final-Wasm declaration admission, native registration and retained schema/TS
  production exist. Real producer retention/recovery/refusal and strict Host Clippy
  pass: `/tmp/canic-wire-bindings/`. The checkpoint compile failure is fixed.
- Generated native schema forwarding is admitted without runtime capability
  forwarding. All 38 package checks and the governed Root Wasm build pass;
  `/tmp/canic-account-628/`. Immutable release schema binding is implemented below.
- Normal builds now acquire schemas/bindings after runtime finalization under the
  existing lock, then materialize exact retained bytes beside Wasm. Provenance
  requires both outputs and the schema hash. Scaffolds and 21 maintained managed
  packages declare native producers. Release manifests now bind exact schema bytes.
- Continue against the [hard-cut contract](../../design/0.112-standalone-blob-service-extraction/wire-hard-cut.md):
  1. Migrate remaining owned endpoints and all callers together. Capability-pruned
     Serde variants and narrower projection enums cannot share Wire indices.
  2. Keep producer qualification bound to the selected release graph.
  3. Qualify normal delivery and interrupted-output recovery on the selected graph.
     Native acquisition stays under the existing lock; release materialization
     consumes immutable outputs after the producer input guard completes.
  4. Retain schema bytes/hash through release records, provenance, installed
     bindings and frontend delivery; test reuse, mismatch and interrupted output.
  5. Qualify generated browser delivery against installed endpoints. The earlier
     TypeScript/Node composition proof is not installed-browser acceptance.
- External IC/provider protocols retain their specified Candid encoding. Never
  reinterpret a Candid hash as a Wire hash or add fallback codecs. Tooling #225
  and Canic #613 track composition. Published Tooling 0.66.3 completes the dedicated
  configured producer; #239 retains evidence and the artifact-input scan concern.
## Latest relevant evidence
- Blob Storage/Contracts 0.47.0 is selected in all three workspace locks.
  Published runtime/contract source matches 0.46; provider JSON is now opt-in
  and unused by this adapter. Three-workspace admission, adapter and both
  consumer all-target/all-feature strict Clippy, and adapter Wasm compilation
  pass: `/tmp/canic-blob047/`. Normal adapter Wasm features exclude provider-json.
  Blob-specific installed proofs below predate this update; no live-provider or
  macOS evidence is implied.
- Store chunk Wire cut passes full 1 MiB round-trip, oversized-reply rejection,
  identity and byte-schema tests; all 33 protocol-surface tests and strict
  Contracts/control-plane/facade all-target/all-feature Clippy pass. The managed
  Store build regenerated canonical Candid and retained Wire schema/TypeScript.
  The installed Root/Store catalogue publication/recovery/effect-free replay
  case passes (177s) on Blob 0.47/Wire 0.7.5. The stale embedded allocation peer
  was refreshed through its owner and final-source verification passes.
  Evidence: `/tmp/canic-chunk-finish/`. Other owned endpoint cuts and live-provider
  acceptance remain incomplete; this does not qualify the full release batch.
- Published Wire 0.7.5 is selected in all three locks; direct declarations require
  0.7.5. Its `Cow<str>` schema support resolves
  [Wire #19](https://github.com/dragginzgame/ic-wire/issues/19) for Canic's actual
  TemplateId/TemplateVersion. Native schema/text-byte regressions, strict scoped
  Contracts/internal Clippy, Contracts Wasm compilation, Rust 1.91 schema-feature
  compilation and dependency admission pass. Host-only derives preserve storage
  representation. Evidence: `/tmp/canic-wire075/`. The coordinated Store
  chunk endpoint/caller cut is qualified as recorded above.
- Generated Blob-client rejection coverage passes in both installed compositions
  on Wire 0.7.4 (533s): wrong-service queries/updates, truncated/trailing mutation
  frames and unchanged stable memory after rejection. Existing successful/replay
  cases also pass. Strict scoped Clippy passed and lock hashes stayed unchanged.
  Evidence: `/tmp/canic-blob-rejections/installed-final.log`. This predates 0.7.5
  adoption and is not current-graph installed evidence or live-provider proof.
- No prior `sample-pool` Fleet or live Blob provider/payer configuration was found
  in the current checkout. Maintained Blob consumers use application-supplied
  installation inputs and controlled provider fixtures. Live provider acceptance
  and #494 evidence cannot be inferred from those fixtures or historical chat IDs;
  no paid/provider action was taken.
- Blob generated-client proof is implemented as the explicit ignored case
  `pic::blob_service::tests::managed_browser_compositions`. It compiles retained
  supplier bindings and exchanges tenant query/update frames with both installed
  consumer shapes, checking caller denial and unchanged mutation replay. It now
  passes in 253s on Blob 0.46, Memory/Storage 0.51 and Wire 0.7.3. All five
  embedded funding/recovery profiles also pass (251s), as does strict internal
  all-target/all-feature Clippy. All three lock hashes stayed unchanged.
  Corrected the JavaScript assertions to compare record fields independently of
  Wire's null object prototypes; production codecs and endpoint behavior are unchanged.
  Logs: `/tmp/canic-blob-client/{installed-corrected,embedded-full,clippy}.log`.
  Use prepared `CANIC_FRONTEND_NODE` and `CANIC_FRONTEND_TYPESCRIPT` to rerun
  the opt-in generated-client proof. This is codec/endpoint qualification,
  not browser UI, live-provider acceptance or completion of #494.
- Adopted published Memory/Storage 0.51.0 with Blob/Contracts 0.46.0 across the
  root and both private consumer graphs, preserving one runtime identity per graph.
  Three-workspace locked admission, nine-package all-target/all-feature strict
  Clippy, both private consumer Clippy checks and 28 memory/56 stable-storage
  native tests pass. Concurrent Wire 0.7.3 selections are preserved in all locks;
  final manifest/lock hashes stayed unchanged throughout these passing checks.
  Under the maintainer's blocker-fix request, the stale embedded allocation peer
  and structured evidence were refreshed through their owning tool. Final-source
  verification passes: `/tmp/canic-push-blockers/verify-final.log`.
  Current-graph Blob installed proofs now pass as recorded above. No package/release version bump
  or Git publication. Evidence: `/tmp/canic-memory051-now/`.
- Aligned published graph adopted: Platform 0.6.15, Wire 0.7.2, Tooling/Testkit
  0.68.1, Query 0.72, Backup 0.44.1 and Blob/Contracts 0.45. All three workspace
  locks select one Wire identity. Prepared Testkit server/release editor are 0.68.1.
  Earlier Platform #55 and Query #103 identity blockers no longer reproduce.
- Auth/Protocol Types 0.16, Metrics 0.16 and Memory/Storage 0.50.17 adopted across
  owning manifests/locks. Six-package all-target/all-feature strict Clippy, both
  private consumer Clippy checks and three-workspace admission pass. Focused native
  tests pass: 344 Auth, 34 Memory and 91 Metrics. No custody/storage redesign.
- Current Blob 0.45 graph now passes both complete installed managed proofs:
  dedicated 130s and all five embedded profiles 405s. Timers 0.31 is aligned in
  all three locks; Query is 0.72.1. Scoped adapter/internal all-target/all-feature
  strict Clippy and three-workspace admission pass. Lock hashes stayed unchanged.
  Evidence: `/tmp/canic-blob-final/`; prior interrupted attempts are superseded.
  Mounted catalog review finds invocation coverage for every current method;
  this is not exhaustive branch or live-provider evidence. #494 still needs
  positive independent-credit confirmation/receipt replay; #622 remains open
  pending default-branch delivery. No Git publication or release was performed.
- Earlier Backup/Blob qualification remains in `/tmp/canic-adopt-bbw/` and
  `/tmp/canic-adopt-bbw-followup/`; it does not qualify the current graph. The latter
  fixed the dedicated schema test's native-only artifact dependency and verified
  Host Artifacts is absent from normal Wasm dependencies.
- Exact Wire hashes now bind application/infrastructure/Root release records; one
  contained declaration resolver serves both codecs. Installed Wire selection
  requires the sealed module hash. 89 release-set, 13 binding and 7 Root tests pass;
  scoped five-package strict Clippy passes: `/tmp/canic-blob-release/`.
  Blob 0.44.2 dedicated and all embedded managed scenarios pass (122s/219s);
  exact retained schema/hash/module assertions cover both installed compositions.
- Producer schemas now retain exact Wire/Canic/Blob browser sources. Native schema
  and Host admission tests (5/6), scoped strict Clippy, both managed proofs and
  TypeScript 6.0.3/Node 24.21 codec checks pass: `/tmp/canic-wire-browser/`.
  Dedicated/embedded cases pass in 133s/243s.
- Normal frontend export now compiles sealed Wire sources with pinned Node/TS;
  owned DID factories are removed. Fifteen frontend tests, five release declaration
  tests and scoped strict Clippy pass: `/tmp/canic-frontend-wire/`. Installed Fleet
  acceptance passes (126s): SDK query/update admission and denials, artifact
  integrity, asset capacity refusal and Observatory handoff. The resolver keeps
  raw-Wasm schema identity separate from installed gzip payload identity. A native
  regression covers the Principal response allocation budget. That run preceded
  the current Blob 0.45 graph. Current-graph requalification now passes in 451s
  with all three lock hashes unchanged: `/tmp/canic-blob-browser-current/`.
  SDK query/update succeed for the admitted caller and deny the unadmitted caller.
  This generic application proof does not exercise Blob upload/download delivery.
- Remaining owned endpoint/caller cuts, Blob-specific browser delivery and #494
  independent-credit acceptance still block completion. The installed SDK proof
  is local PocketIC, not live IC
  or browser UI qualification. Shared Tooling #43 supplies compiler pinning policy.
- Tooling 0.67.0 and Shared snapshot `4c0f198e` adopted; configuration limits
  supplied at every build-spec call. Three-workspace dependency admission and
  boundary/format/release fixtures, scoped strict Clippy and 13 checksum/build
  tests pass: `/tmp/canic-tooling-067/`. Query 0.71 now selects Host 0.67.
- Account #628: Platform 0.6.12, native/TS vectors, strict Clippy, MSRV and Wasm
  compilation pass; explicit schema-feature JSON isolation remains open (Wire #17).
  Evidence: `/tmp/canic-account-628/`; external ledger accounts remain unchanged.
- Earlier producer/runtime evidence remains in `/tmp/canic-fleet-schema/` and
  `/tmp/canic-embedded-schema/`; it is not one current release-graph proof.
- No full-suite, installed-IC, macOS, TEE or embedded allocation-peer qualification is implied.
## Other retained boundaries
- Independent discovery repair (#266): nearest App config roots and Cargo
  boundaries now bound selection; generated trees are skipped and traversal is
  limited. All 37 focused tests and strict Host Clippy pass on Linux. Evidence:
  `/tmp/canic-config-discovery/`. Blob/Wire delivery blockers remain separate.
- Independent absence repair (#261): Fleet Ensure binds missing-canister evidence
  to the exact requested Principal; direct-agent status uses typed reject fields.
  Native regressions pass: `/tmp/canic-exact-absence/`; strict Host Clippy also
  passes in `/tmp/canic-config-discovery/`. This does not
  resolve the Blob/Wire batch's remaining delivery blockers.
- Independent auth diagnostic repair (#279/#280): Medic warns about unconfigured
  or unfinished renewal; issuer metadata failures remain distinct from registry
  absence. All 28 focused CLI tests and strict CLI Clippy pass on Linux:
  `/tmp/canic-auth-diagnostics/`. Blob/Wire delivery blockers remain unchanged.
- Independent replica transport repair (#260): anonymous status/query responses
  are capped at 8 MiB with existing deadlines retained. All 14 focused tests and
  strict scoped Host Clippy pass on Linux: `/tmp/canic-replica-bounds/`.
  This repair does not resolve the Blob/Wire batch's remaining delivery blockers.
- Auth: preserve the selected portable credential/session design and #506 renewal
  semantics. The maintainer has qualified no production clock-skew bound; keep
  the production activation gate closed. TEE hardware evidence is unavailable.
  Reconcile implementation gaps with [Auth's current guide](../../features/authentication/README.md)
  and #491/#507 before claiming completion. Inspect current source.
- Backup: dormant capture mutation APIs were removed; active restore is retained.
  Unsettled snapshot loads now refuse automatic replay and failed-command rearming;
  recorded precondition refusals remain retryable. Process-interruption evidence:
  `/tmp/canic-backup-load-settlement/`. The subprocess adapter cannot settle an
  uncertain load; retain its journal and source backup pending qualified recovery.
  Backup 0.42 owns graph validation; Canic still validates serial journal progress.
  Exact workflow/stage/attempt authority is still pending. Evidence: `/tmp/canic-backup-ordering/` (115 restore
  tests, 42 CLI restore tests and strict Backup Clippy; real-ICP test ignored).
  Live capture remains unavailable. Preserve #513 coordinator convergence and
  #550/#394 checkpoint/live-capture requirements. Latest cleanup evidence:
  `/tmp/canic-backup-cleanup/` (248 library tests, 52 CLI backup library tests,
  strict all-target/all-feature Clippy). Adopted 0.42 has the coordinator;
  Root checkpoint storage, controller-authenticated Wire dispatch and catalogue
  exclusion are connected. Installed Root/two-participant interruption proof and
  paid snapshot settlement remain prerequisites for the engine switch.
  The [convergence contract](../../design/0.112-standalone-blob-service-extraction/0.112-design.md#backup-execution-convergence--2026-10-24)
  records exact identity, original attempt and two-effect-owner acceptance.
  Checkpoint evidence: `/tmp/canic-root-checkpoint/` (22 checkpoint tests, 33 replay-policy
  tests, installed participant recovery and strict scoped Wire-schema Clippy pass).
  Governed Root artifact qualification now passes; see Wire evidence above.
  No engine replacement or production qualification is claimed; see the [Backup guide](../../features/backup-and-restore/README.md).
