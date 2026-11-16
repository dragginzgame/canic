# Canic qualification

This workspace owns installed effect-boundary and product regression tests.
Store-native tests and the standalone upload/reopen/version-reuse witness now live
in `dragginzgame/ic-wasm-store`. The two local workspaces pin its passive protocol;
the existing artifact builder selects the matching locked crates.io runtime and
builds it using its packaged lockfile.
This workspace retains Root integration, chunked installation and complete CLI
journeys. `make store-test` does not rerun the supplier's native suite.
`bash scripts/dev/test-store.sh preservation` selects three installed boundaries
against published 0.2.3: distinct removed/restored pages, an actual historical
scan retaining its digest/refusal across CUJ8→CUJ9, and a new Root scan retaining
deleted/cancelled labels and its removed-entry cursor across Root and Store
upgrades. Store activation remains explicit; a downgrade cannot discard the
scan contract. Historical pre-registry Store maintenance starts under the retained
old Root, preserving that scan through owner replacement.
`bash scripts/dev/test-store.sh removal` selects the actual Root forwarding cases:
full-manifest delete/cancel, pinned refusal, immutable tombstones/quota, malformed
and unauthorized requests, maintenance fencing, expiry before exact restoration,
and a lost reply that cannot become settlement evidence. The fixture is the caller
and retains the original request/result.
`bash scripts/dev/test-publication.sh cli_removal_restoration` separately qualifies
Host/CLI custody using installed Root/Store and the real ICP signer: shared content,
publication quiescence, one retained removal signature, reconcile without renewed
authority, deadline-gated explicit restoration, historical payload replay refusal,
and cancellation committed remotely with a lost reply that remains blocked.
Only independent publication sources and one canonical journal are selected; this
does not qualify nested removal selection or repair after a refusal/unknown outcome.
Its packages are named by responsibility, independently of design milestones:

| Package | Responsibility |
| --- | --- |
| `canic-qualification` | Native harnesses, retained predecessor binaries and explicit shared simulator preparation |
| `canic-effect-probe` | Small Wasm probe isolating creation/funding commit boundaries and owner rollback |
| `canic-asset-catalog` | Real embedded Blob/catalog composition with explicit lifecycle and qualification-only failure points |
| `canic-artifact-probe` | Small Wasm probe isolating retained-byte installation onto an existing target |

Each package lives in its identically named directory under this workspace.

M0/M1 package and Make names are retired without aliases. `make qualification-setup`
prepares the simulator for every suite. Select `make effect-test`,
`make artifact-custody-test` or `make journey-test` for the renamed workflows;
`root-test`, `growth-test`, `profile-test` and `endpoint-test` keep their names.
`make store-test` qualifies versioned Store custody and Root's Store-backed
creation/upgrade engine, after `make canic` and simulator preparation. It builds
Store and Root executables through the public artifact builder, checks bounded
Wire frames, resumes publication across Store maintenance and verifies byte reuse.
Root-mediated publication remains usable after controller handover: incomplete
uploads resume after Root maintenance without changing Root stable bytes.
Malformed/unauthorized/expired requests preserve state; lost Root replies retain
Store commits and exact replays recover through replicated observation.
`make store-readiness-test` selects the installed handover/readiness cases and
their bounded protocol test without building application artifacts. Actual Root
calls are refused before Store handover and succeed afterward, across repeated
reads, a reply trap and Root maintenance. Both application slots and both owners'
stable bytes remain unchanged; malformed, unauthorized and expired requests are
refused. The full `store-test` includes these cases. This observation alone does
not certify code/controllers or complete CLI bootstrap.
`bash scripts/dev/test-store.sh custody` selects the installed preservation-read
witness and Canic's management protocol tests. Supplier metadata budgets and reopen
tests live in ic-wasm-store. The installed witness preserves shared
release pins from two former owners and an incomplete upload across a Store
executable change, then reads through the real Root after sole-control handover.
It covers unauthorized, malformed, expired and missing-release requests, reply
fault 12, Root maintenance and unchanged stable bytes. Direct test-controller
Store maintenance here does not qualify a Host-driven Store upgrade workflow.
`bash scripts/dev/test-store.sh admission` selects the installed persistent mutation
gate and cache-continuation witnesses, plus Canic's management protocol tests.
Store-native invariants and metadata bounds are qualified in the supplier repository.
Actual Root calls close and activate admission; a lost Root reply preserves the
Store commit. Closed admission survives Store/Root replacement, stale or wrong-owner
activation is refused, and an in-flight cache repair cannot continue through a
close/reopen cycle. The fixture retains direct operator control for Store upgrade;
it does not claim the production Root-only installation flow. Prior CWS1 settings
migration is tested at the record codec boundary, not using a historical binary.
`bash scripts/dev/test-store.sh quiescence` selects four installed Root-owned
closure/stop cases and the maintenance Wire-budget test. Store has sole Root
control. Callback faults 13/14 preserve closure and stop intent; Root replacement
and post-deadline observation recover without reissuing stop. Unfinished children,
wrong Store control and expired dispatch refuse effects. The pinned predecessor
Root binaries migrate without gaining maintenance or preservation evidence.
This selection ends at Stopped. Full child-custody reads refuse any retained
Store-maintenance record, including completed operations.
The same selection now interrupts a closed-catalog scan with Root replacement
and concurrent continuations, retaining one cursor/count and including an
unfinished upload. Native Store cases cover aliases, former-owner pins,
reopen and stale/wrong-owner/open-gate refusal. Metadata page and retained Root
status fit their scoped Wire budgets. No payload scan is claimed; old CUJ5
attempts cannot gain preservation evidence during migration.

`bash scripts/dev/test-store.sh maintenance` selects the full local runtime
replacement witness. Root alone uploads/installs approved bytes, restarts with
admission closed, verifies metadata and explicitly activates. Callback faults
15–18 and pre-install dispatch fault 19 cover retained intent and observation-only
recovery across Root replacement. The fixture compares every Store stable byte
before/after upgrade, including chunks, shared-version pins and an unfinished
upload; publication resumes only after activation. The same checks also start
from the retained Store 0.1.0 executable in `canic-qualification/fixtures`, proving
the pre-registry transition without rebuilding the predecessor or copying its
implementation. Wrong bytes/callers, malformed frames, expiry, rejected modules
and incompatible replacement endpoints cannot
activate or reissue installation. Recovery after expiry can retain an installed
outcome but cannot start it with expired authority. The repeat fixture migrates
both an issued installation and a completed attempt from a retained 0.215 Root,
then qualifies bounded successors without losing prior authority. CLI qualification
is separate below; neither selection establishes mainnet readiness.
The explicit `recover_store_maintenance` endpoint qualifies observation without
fresh dispatch: it leaves Reserved, Closed, Stopped, Uploaded, Installed, Verified
and Ready unchanged, including with live authority. Lost callbacks recover through
the same journal across Root replacement, concurrent calls and expired authority;
post-install metadata verification may finish but activation remains explicit.
Wrong callers, altered attempts and malformed recovery frames preserve state.
`bash scripts/dev/test-store.sh root-maintenance` selects the complete Root custody
budget and an installed Host replacement while Store is stopped. It compares
children, pins and the exact Store operation, refuses altered operation evidence
before signing install, interrupts after installation and resumes the original
signed attempt before retaining completion. Root and Store stable bytes remain
unchanged, and the original Store operator can still recover the same operation.
The Host test executable is built in the product workspace and invoked by the
simulator harness; their incompatible supplier graphs remain separate, as with
CLI witnesses. The ignored native entrypoint is run explicitly by this selection
and `make store-test`. This qualifies explicit Host approvals, not automatic
whole-fleet planning of Store maintenance or mainnet use.
The automatic local path is owned by the installed CLI publication runner:
`bash scripts/dev/test-publication.sh fleet_store_upgrade_retains_authority_recovers_and_feeds_root_and_child_plans`.
It creates an isolated signer, generates the default starter through `app init
--source`, builds all three recipes and bootstraps the normal Root/Store/app with
the default 4T budget. It installs a qualification-only fault Root through reviewed
maintenance, selects a changed Store
artifact, refuses reconcile before admission, pauses before install and activation,
and recovers upload callback fault 15 without new installation authority. It compares
Store stable bytes, rejects missing/substituted completion custody, preserves the
first operation through a second Store replacement and later Root replacement,
then publishes and upgrades the child using the new Store binding and explicit
complete-history read. Replanning retains the same pending approval and derives
the next sequence only after completion. The runtime maintenance selection also
qualifies all eight slots, ninth-attempt refusal, a retained 0.215 Root transition,
historical replay and older-code refusal without dropping custody. Fixture credentials
stay under `target`; it never reads the operator wallet. This uses synthetic cycles
and is not mainnet qualification. No new runtime workflow or dependency is added.
Public Root diagnostics are checked against the private retained observation across
the same installed phases, including a stopped Store and Root replacement. Malformed
queries do not alter state and the replicated custody endpoint remains controller-only.

The same witness calls the generated greeting before and after editing its actual
source and rebuilding through the public artifact builder. The changed executable
keeps the original application principal. An unchanged `app build` preserves all
three artifact identities and Cargo.lock and produces no new deployment approval.
The later `app build --canister app` selects the declared recipe and returns only
that canister, preserving the exact Root/Store artifact bytes and Cargo.lock used
for maintenance fault qualification. Unknown selection and invalid recipes refuse
before lock/output creation in the native checks. No alternate greeting artifact stands in
for the generated application upgrade.
Actual Root calls acquire pins, install a child larger than the former inline
limit, and upgrade it using the Store cache. Lost pin/cache callbacks recover
through observation after Root maintenance. Missing cache, unpublished releases,
expired authority, retained upgrade history and mixed inline/Store custody have
explicit cases. A separate provider fixture reconstructs a cleared cache and
performs real chunked installation/upgrade. Named CLI deployment can now select
a completed publication operation. `make publication-test` focuses on the actual CLI, ICP signer
and installed Root/Store bootstrap and publication path: named creation, controller
handover, lost handover reply recovery, missing readiness evidence refusal,
unchanged application custody, interrupted upload, read-only
reconciliation, exact resumption, immutable version reuse, certified-code refusal
and signed observation custody. It needs the prepared simulator and ICP signer,
uses synthetic cycles, and leaves Root stable memory unchanged during publication.
The publication/resumption case also checks `fleet status` delivery evidence:
admission alone stays unknown; completed creation and upgrade expose their own
retained pin, exact manifest and original cache deadline without changing the
earlier operation. Native cases refuse substituted source evidence, backward pin
transitions and changed or removed cache deadlines across observation history.
Partial recovery replies preserve earlier delivery; explicit loss in a later full
observation refuses. Inline JSON shapes remain unchanged; terminal tests distinguish
pin retention from live cache readiness and replay permission.
The same named journey creates a child from a release larger than the inline
limit, refuses changed Store control, recovers missing admission evidence, upgrades
the same child after cache clearing, and calls the upgraded greeting application.
Run only that journey with `bash scripts/dev/test-publication.sh bootstrap_store_then_publish_resume_and_retain_versions`.
After a successful run it writes `target/publication/run-PID/publication-measurements.json`.
The report separates monotonic host wall time for cold planning, interrupted
prepare, observation-only reconciliation, resumed upload, warm planning and warm
publication. It records the actual artifact hashes and operations, and decodes
retained completed Put requests to verify the cold unique-chunk payload total and zero warm
payload requests. These are request payload bytes, not network retransmissions
or management-call counts. Build, replica setup, Store bootstrap, child scheduling,
deployment and cycles are outside this measurement. There is no timing threshold
or mainnet latency claim; use this as a local diagnostic baseline.
Publication does
not create a canister or reserve a deployment slot; see [Store scope](../docs/demos/wasm-store.md).

`bash scripts/dev/test-store.sh dispatch` qualifies Root's earliest-dispatch
contract independently of Host's current scheduling policy. Actual Store-backed
creation and upgrade admit elapsed dispatch timestamps before their original
expiry, complete without operator continuations and preserve exact custody across
Root replacement. Expired admission preserves state, future work still waits,
and a timer first run after expiry creates no child. Replays remain observational,
including after expiry. Native checks cover exact time boundaries and bounded
intervals. The publication runner's Store-upgrade journey additionally qualifies
the CLI default: initial fleet creation and the later child upgrade omit
`--execute-at`, bind earliest dispatch to plan creation and finish without simulated
clock jumps. It prints local wall timings separately for bootstrap and child upgrade
(publication already retained for the latter). This does not qualify older Root
binaries or mainnet timings; original code selection and expiry remain binding.

The same runner also qualifies a fresh whole fleet with Root, Store and two
application children above 2 MB each. Store readiness and complete publication
evidence block both application admissions;
reconciliation recovers a lost handover reply without admitting either child,
then apply resumes the original approval. Both children finish, replay creates
nothing extra, shared bytes are not uploaded twice, and publication/admission
replies can be lost without replacing signed attempts. Four successive child
upgrades exercise both slots and cache reconstruction, with independent publication
and installation approvals. Four-canister inventory is complete; missing Store
readiness proof refuses convergence. A second Root executable preserves complete
Store custody after both children upgrade. Changed Store authority refuses
installation, interrupted postflight and a lost install reply recover the original
signature, and subsequent child upgrades use the maintained Root, including reuse
of a release published before maintenance. Missing publication completion refuses
that reuse without granting renewed mutation authority. Native
maintenance tests reject altered pins, sources, deadlines and unexpected slots,
without admitting legacy inline observation methods for Store custody.
The whole-fleet journey also reads the public maintenance report before admission
and after Root stops Store. Repeated actual `fleet show` calls preserve both
canisters' stable bytes and keep all four inventory identities. The report does
not invoke signing or continue maintenance.
Native fleet tests admit both full 8 MiB manifests and
preserve existing inline fleets with or without Store and their retained digests.
`make blob-test` builds two embedded catalog executables using the optional
Canic adapter and the real Blob Service at its pinned Git revision. Five installed cases cover user
isolation, shared references, catalog/Blob rollback, malformed frames, capacity,
non-resurrecting replay, drain/reopen/exact activation and reopen refusal.
Supplier source/files and both consumer locks are fingerprinted before/after;
no supplier checkout files are changed. The immutable input is explicit
in [AGENTS](../AGENTS.md#local-blob-composition). See [scope and evidence](../docs/demos/blob.md).
Tests never install the simulator themselves. Preparation and tests use the same
`IC_TESTKIT_SERVER_DIRECTORY` override, defaulting to `target/qualification/server`.
Existing files under `target/m0/` are historical outputs and are not deleted or
silently used as a fallback; run setup once for the new location.

## Coverage retained from the initial experiments

The first retirement review keeps every effect scenario because the product
fixtures do not yet replace all of their obligations:

| Coverage | Why it remains |
| --- | --- |
| Creation dispatch, callback-write and later-message traps | Isolates each IC commit boundary; product growth covers a narrower set of interruption points |
| Direct-cycle funding, rejected recipient and refunds | Product runtime has no equivalent funding journey yet |
| Failed/successful owner upgrade with empty, successful, rejected and uncertain journals | Preserves the combined creation/funding rollback matrix |
| Retained artifact, occupied target, lost upload/install replies and maintenance | Proves install-only behavior on a pre-existing target; product growth creates its own target |

The separate native M0 harness has been removed; all tests now share the native
qualification package. The two probe implementations remain intentionally
isolated. Delete one only after product regressions cover its remaining cases.
The current Root-plus-two-child scope is described in the [Root guide](../docs/demos/root.md);
dated decisions and measured results live in [qualification history](../docs/qualification/history.md).

## Management-effect boundaries

These installed PocketIC experiments ask whether the management owner can retain
creation, funding and artifact evidence across traps and owner upgrades. They use
the published Platform runtime and real local management calls. These are
qualification fixtures, not a deployable Canic runtime or the Hello World CLI.

From the repository root, after the normal development tool preparation:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.100.0
cargo fetch --locked --manifest-path qualification/Cargo.toml
make qualification-setup
make effect-test
```

`qualification-setup` explicitly provisions Testkit's authenticated PocketIC bundle
under `target/qualification/server`. Tests only check that prepared bundle; they never
download a server. An already prepared bundle can be selected with
`IC_TESTKIT_SERVER_DIRECTORY=/absolute/path make effect-test`. Tests need permission
to bind a local loopback port. They create isolated local instances, use synthetic
cycles, and close each instance after its case. No mainnet endpoint or signer is
configured.

The runner builds the probe and selects its Wasm from Cargo artifact messages.
It first checks that maximum retained evidence fits the Wire status contract,
then runs five installed creation scenarios:

| Scenario | Observation required |
| --- | --- |
| Trap after enqueue, before dispatch commit | No new canister; intent and reservation roll back |
| Trap before recording the returned principal | One new canister; committed intent remains without a retained principal; another attempt is blocked |
| Write principal, then trap in the same callback | Same uncertainty as the preceding case: the write was not a commit |
| Commit principal, then trap in a separate self-call | Principal and original outcome survive; replay returns that evidence without creating another child |
| No injected fault | Creation evidence and controller assignment are retained; replay has no creation effect |

Each executor admits at most one creation and attaches 2 trillion synthetic
cycles. The harness bootstraps it with PocketIC's default 100 trillion cycles.
The reported executor balance decrease includes transferred cycles and execution
charges; it is not a measurement of consumed cost alone or a mainnet quote.
Uncertain attempts retain their full attachment as exposure and cannot reset it.

PocketIC subnet counts and controller inspection are independent test oracles.
They are not supplied to the executor to reconstruct a lost result. A child can
exist while its principal is absent from the executor's committed evidence.
The fixture deliberately demonstrates that blocked window rather than guessing
the principal or automatically retrying.

The funding experiment repeats the five interruption windows for both an empty
live recipient and a deleted recipient on the same subnet. The latter produces
a management rejection and refunds the attachment. Each fresh executor admits
one deposit of at most 1 trillion synthetic cycles, retaining the destination,
amount and exact management request. Successful callbacks retain completion with
zero refund; rejecting callbacks retain their raw reject code and matching refund.
Any unrefunded rejection exposure stays reserved. A retained result is returned
on identical replay; an uncertain attempt stays blocked. Changed destinations
or amounts cannot overwrite either slot state, including a fully refunded rejection.

Independent recipient balances prove whether a second credit occurred. Executor
balance changes help check physical refunds, but neither balance is accepted by
the probe as operation evidence. A lost callback stays blocked even when the
harness can see the transfer or refund. Separate admission cases check zero and
excessive amounts, malformed principals and unauthorized callers, followed by a
valid deposit to prove invalid commands did not consume the slot. Creation and
funding use independent test slots; this is not a shared product allowance policy.

The maintenance rehearsal uses two different executables with the same journal
format. It captures both journals outside the executor, stops it, injects a trap
after the candidate restores and writes stable memory, and restarts the surviving
executable. It then performs a successful upgrade and repeats the failure with
populated stable memory. Module identity, controller graph and exact journals
must survive each failed upgrade; the successful upgrade must select the changed
executable without resetting outstanding exposure. Replaying commands must create
no extra child or deposit. Cases cover an empty slot, retained successes, a
retained rejection/refund, lost callback evidence and failure in later work.
An unauthorized principal must be unable to upgrade the executor.

Run just that rehearsal with `make effect-maintenance-test`. `make effect-test` includes
it alongside the creation/funding boundary cases. The runner builds both Wasms
and saves their separate inputs under `target/effects/` before executing the tests.
The checkpoint stores each journal in its existing bounded Wire frame, using
one 64 KiB stable page through Platform Runtime. This isolated test owns that
raw backing; it does not qualify a production `ic-storage` integration. Existing
heap-only probes are not upgrade inputs: the harness installs fresh executors.

These cases qualify a maintenance window after message executions have settled,
including uncertain effects left by failed callbacks. They do not qualify
upgrading with callbacks in flight, schema migration, unconditionally trapping
pre-upgrade hooks, host crash recovery, governance, ledger-based funding, artifact
availability during operator absence or mainnet executor maintenance. The host
harness retains observations in memory, not a durable operator journal. Those
remain separate proofs. The continuation is an actual self-call; an ordinary
Rust-only `await` is not a commit boundary.

## Artifact custody without the operator

Run `make artifact-custody-test` after the same preparation. This separate fixture
keeps the preceding creation/funding probe unchanged. It builds the actual Hello
World worker and two executor executables from Cargo artifact records. The
controller approves one exact target, uncompressed Wasm hash, retained module,
dispatch time and interruption mode; invalid hashes, oversized uploads and
unauthorized callers cannot consume the slot. An admitted approval cannot be
replaced, even after a rejection or unknown outcome.

The executor survives a failed upgrade and a successful upgrade before dispatch.
With no subsequent operator command, a test-only heartbeat uploads the retained
bytes to its own IC-managed chunk store and calls `install_chunked_code` on an
existing empty worker. This fixture qualifies the on-chain chunked API selected
during the initial experiment; current supplier versions are pinned by Cargo.lock. It uses
one chunk and adds no Store canister. Direct host installation remains the CLI
baseline; this is not a second product deployment engine.

Four installed cases cover success, a lost upload callback, a lost installation
callback and refusal to install over existing code. A later executor upgrade
must preserve the original request commitments and attempt counts. A lost upload
callback remains blocked, even when the test oracle can see the stored chunk.
A lost install callback can be reconciled through the management API's module
hash and sole-controller evidence, without reinstalling or claiming that the
original callback was retained. Worker canister versions independently check
that rejected installs and repeated approvals did not replace code again.

The fixture limits artifacts to 64 KiB. Its checkpoint owns raw stable backing
only within this experiment; the provider chunk store is separately measured.
Neither proves production Storage integration, a production scheduling policy,
multi-chunk handling, chunk cleanup, callback-in-flight upgrades, application
readiness or a mainnet cost estimate. The worker is pre-created by the harness.
Product Root/growth fixtures separately qualify combined creation, installation,
readiness, maintenance and fleet views; this probe does not replace those tests.
The CLI's initial JSON/HTML inventory is separately exercised by `make journey-test`
against two greeting deployments, an uncertain upgrade and a stopped provider.

Focused checks are `make artifact-custody-clippy` and `make artifact-custody-doc`.
The aggregate `make ci` includes the artifact experiment separately from
`make effect-test`.

## Checks and evidence

The Root runner accepts test-name filters after artifact preparation. For the
generated starter's existing inline callers and predecessor transitions:

```sh
bash scripts/dev/test-root.sh whole_fleet managed_child_cli multi_child_cli retained_predecessor
```

These fixtures explicitly select inline custody. The publication runner above
owns the generated Root/Store/app default; neither fixture migrates retained fleets.
`bash scripts/dev/test-root.sh identified_storage_migrates_pending_children_and_refuses_downgrade`
checks migration from the retained positional-storage Root to CUJ9/CUA4 identified
collections. Both pending child approvals survive replacement unchanged and
complete independently. Compatible executables reopen the collection; the old
executable refuses without changing state. The two-child publication journey
starts from the same retained binary and compares complete custody across
Host-driven migration with both Store-backed children and their upgrade histories.
Stable payload bytes change once; original Storage allocations and backing size
are retained. Neither transition increases the two-child bound.
The Root runner also runs `cargo test --locked -p canic-management --lib`, exercising
the actual persistent codecs natively: malformed identities, oversized history,
truncated records and the full retained inline artifact budget. A mixed inline/Store
record fills the original artifact lease, migrates and reopens through governed
Cells without growing either allocation; excess data refuses before writing. These checks use
test-only native dependencies; Host still consumes the passive protocol alone.
Host's `root_custody::`, `maintenance::` and `managed::` native tests cover the
shared identity-keyed projection of historical complete Root replies: empty and
uncertain children, paired Store sources, retained maintenance history, truncated
frames and refusal of the wrong method or direction. The publication fixture
`fleet_store_upgrade_retains_authority_recovers_and_feeds_root_and_child_plans`
exercises both consumers through installed Root/Store and the real signer.
The multi-child journey plans the second child's first upgrade through the named
`--canister` path, verifies its retained slot and lost-reply recovery, then includes
it in whole-fleet convergence and Root maintenance. General dynamic allocation
and new-child admission are not qualified by this named-upgrade path.
The publication runner's `existing_fleet_adds_child_preserving_identity_and_maintenance`
case separately covers whole-fleet growth from one to two children, both inline
and Store-backed, after prior Root and Store maintenance. It checks separately
approved publication/creation, lexical name reordering, removal/artifact-change
refusal, lost admission replies, exact signed-attempt reuse, four named serial upgrades
of the added child and complete Root maintenance. Wrong targets, stale Root code
and missing creation completion refuse; whole-fleet planning returns the same
pending upgrade, including its completed Store-maintenance binding. Missing
predecessor completion and stale predecessor code refuse the second upgrade;
lost stage replies reuse exact signed bytes. Completed replay retains the latest
approval. The B → C → B → C sequence repeats a code transition with a fresh
approval against the latest predecessor, reusing the completed Store publications.
Original signed attempts stay unchanged; later Root maintenance preserves all
four upgrade records. The greeting variants differ by valid custom sections;
this qualifies artifact/authority reuse, not application schema downgrade or
state rollback.
Original fleet bytes and admission signatures remain unchanged; the installed
canister count excludes duplicate creation. No runtime capacity increase or
reusable-pool behavior is claimed. Both planners share ordered custody validation
and upgrade-count, artifact-capacity and dispatch-order checks. Serial named
planning requires a retained whole-fleet creation approval.
Management continuation timers and in-flight guards are keyed by the same child
identities as durable custody. The installed `children::identity`,
`children::lost_creation` and `children::two_children` cases cover positional
predecessor preservation, reverse admission order, independent sibling progress,
lost callbacks and bounded upgrade history. The profile witness below covers
same-instance timer recovery and later Root/Store maintenance. These checks do
not increase the current capacity or change serialized child identities.
Protocol checks compare numeric `ChildId` encoding against the frozen deployed
enum, including complete request frames and Store pin digests. Host checks retain
the original JSON spellings and approval digests. Unsupported constructor values,
numeric JSON and unknown Wire discriminants refuse. Request compatibility is
independent of storage compatibility: installed migration preserves approval
bytes, while old executables refuse the new persistent collection formats.
The multi-child journey also inserts a third declaration while both original child admissions
are retained. Planning reports the two-child bound, preserves the original approval,
signed admissions and Root stable bytes, and resumes the original plan after the
configuration is restored. Native checks cover zero/three children with and without
Store, refusing before artifact reads or journal creation.

Host plan tests check reservation conflicts independently of directory order,
including duplicate approvals for the same inputs, legacy/explicit First-slot
aliases, conflicting publication owners, malformed later records and the existing
128-operation planning bound. A matching approval cannot bypass the remaining
scan; valid retries still return their original digest and deadline.

`bash scripts/dev/test-root.sh discovers_both_children_without_local_deployment_records`
checks the shipped CLI against an installed Root with no local deployment journal.
It discovers children admitted in reverse slot order, verifies their controllers,
omits an empty additional slot and reports an older executable's missing endpoint
as unknown. Actual terminal and saved HTML output identify both child slots and
their reported principals. Repeated observations preserve Root stable bytes and
create no journal.
Discovery is bounded to the selected owners' two slots; it is not recursive tree
discovery or authenticated completion evidence.

`bash scripts/dev/test-root.sh profile_store` qualifies the profile manager as a
Store-backed management owner. It preserves two users' private state and Store
custody through worker, Store and owner upgrades, checks fleet observations,
and refuses an installed greeting as profile readiness without replaying effects.
After infrastructure maintenance, both users can still write; retries and refused
writes preserve state. A subsequent approved change to the compatible earlier
worker executable uses the current Root/Store bindings and retains the latest data.
It then adds a second worker whose name sorts before the first, writes distinct
profiles for the same two users, upgrades that worker, and replaces Root again.
The original allocation stays fixed; both workers preserve private records and
stable bytes, and Root retains both creation histories and their upgrades. Store
stable bytes remain unchanged across that final Root replacement. Cross-user and
Root reads remain forbidden in both workers. This qualifies explicit second-worker
growth, not automatic user routing, shared identities or sharding.
The same witness selects an explicitly qualified fault manager through maintenance,
loses a worker-install callback after remote commit, and recovers through the real
CLI's `fleet reconcile`. It checks original approval/signed-admission custody,
effect fingerprints, one installation/version increment and unchanged private
state before proceeding through normal Store/owner maintenance.
The interrupted upgrade remains unchanged across further timer opportunities
before reconciliation. IC Timers owns callback recovery tokens; Canic clears the
current token only through authenticated domain reconciliation after inspecting
durable operation evidence, then derives demand from that evidence.
The profile and publication witnesses share isolated signer preparation; generated
credentials stay in their fixture directories and never use the operator wallet.

`bash scripts/dev/test-root.sh metrics:: profile_store` qualifies public timer
metrics on installed Root and profile manager executables. Anonymous/non-controller
queries and malformed requests preserve counters and durable state. Root upgrades
reset the volatile counters with a new initialization window while preserving
operation evidence. The profile witness observes an abandoned callback and an
expected readiness failure, and consumes the aggregate report through the real
CLI fleet JSON. This proves local aggregate diagnostics, not historical metrics,
fleet-wide rates, persistence or mainnet deployment. Passive codec tests preserve
maximum counter totals/overflow quality; CLI tests retain unknown/clipped states
and keep metrics out of the compact terminal view.
The CLI's focused `fleet::` tests also cover the saved/live HTML projection:
the diagnostics section starts collapsed, exact decimal totals and overflow
quality match terminal details, unknown readings never become zero, and owner,
network and error text is escaped. Rendering preserves the source snapshot and
uses the existing bounded localhost viewer; it adds no browser RPC client.

`make profile-test` runs the [Canic-owned profile demo](../docs/demos/profile.md)
through the existing deployment CLI, with private ownership, bounded capacity,
failed-upgrade rollback and preservation across two executables. It also deploys
the discovery entry, checks immutable binding and upgrade preservation, exercises
direct worker observation and unavailable-worker recovery, and renders the pair.
Its entry, worker and client are products under `apps/profile/`; only its installed harness belongs
to this qualification workspace. `make profile-clippy` and `make profile-doc`
cover its native and Wasm graphs. These targets are included in the corresponding
repository aggregates.

Focused checks are `make effect-clippy`, `make effect-doc`, `make fmt-check` and
`make toko-tooling-check`. The aggregate `make ci` includes the fixture and
requires prior `qualification-setup`; it does not install tools. Qualification packages have
their own catalog and lockfile, with no dependency from the product workspace.
Build output lives in `qualification/target`; `target/effects` and
`target/artifact-custody` retain probe inputs and Cargo artifact records.

The [historical record](../docs/qualification/history.md) retains dated observations
and decisions. Passing these experiments does not establish
general effect recovery. Local dispatch-resource failures, cross-subnet funding
and production approval of blocked exposure remain unqualified.

## Application-owned growth journey

`make growth-test` composes the real profile worker with a one-slot manager,
IC Storage and IC Timers. It uses the Canic deployment CLI and native profile
approval client, then creates and installs one worker without another mutating
operator command. It exercises authenticated application use, five interruption
boundaries, a distinct manager executable, exact replay, network binding, expiry
and insufficient reserve. Subnet counts independently detect duplicate creation.
Lost creation replies remain blocked; upload/install outcomes can be reconstructed
by observation, including callback-trap recovery without replacing the manager.
Abandoned upload/install callbacks remain blocked across further timer
opportunities before explicit reconciliation. All cycles are synthetic.

The test also qualifies maximum artifact/evidence codec budgets and emits a fleet
snapshot under `target/growth/journey-N/fleet.html`. Serve the selected snapshot
with `canic fleet view PATH --port 8788 --open`, then use **http://localhost:8788/** in
the normal desktop browser. Ctrl-C stops the viewer; the replica has already
closed. See the [growth guide](../docs/demos/growth.md) for the selected topology,
limits, fresh mainnet instructions and recovery limitations. The focused lint and
documentation checks are `make profile-clippy` and `make profile-doc`.
