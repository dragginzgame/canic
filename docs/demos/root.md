# Root and its children

The optional `backup` build exposes authenticated snapshot execute/lookup for a
fresh Root. It uses one surviving original-result ledger alongside the existing
Storage records. After admitting snapshot work it fences ordinary private
management endpoints; it never starts or activates a restored application.
See the [executor contract](../design/canic-system-design.md#backup-signing-and-execution-ownership).
The installed local witness is `bash scripts/dev/test-root.sh snapshot::` after simulator
preparation. This is not yet a Backup CLI workflow or an upgrade for existing fleets.

The generated starter begins with a Wasm Store and one approved Hello World
application; a fresh or completed one-child fleet can include a second application:

```text
Root
├─ Wasm Store
└─ Application
```

Root's controller is the external operator identity. The application's sole
controller is Root. External identities appear as controller metadata, never as
nodes above the canister tree.

The development Root uses two named Storage 0.78 cells on one backing.
The Backup build adds a third cell for snapshot custody. Fresh installs and compatible new executables
are locally qualified. Root now uses journal schema 2 and accepts only an explicit
`Ready` response. Schema-1 greeting and existing 0.74 fleet images must keep their qualified old
Root; the new executable refuses those images. A failed Host maintenance install
retains its signed attempt and leaves the target stopped for explicit recovery.
Do not rebuild and upgrade the historical mainnet demo below with this executable.

The generated `wasm-store` child is separate from both application slots.
Whole-fleet planning currently admits one or two direct application children.
It reports an unsupported child count before reading artifacts or opening the
journal. This is the qualified implementation bound, not an operator capacity
setting. Adding a declaration cannot extend a retained approval; changing or
removing a declaration does not release an existing allocation.
Once initial creation completes, adding a second declaration selects a separate
child approval through `fleet plan`. It consumes the unused runtime identity;
configuration sorting cannot move the original child. Root supplies the fixed
1T creation attachment from its current balance, subject to its reserve check.
This does not renew the original operator debit or provide automatic funding.
Whole-fleet apply completes Store handover/readiness before
admitting either app; the approval includes its separate 1T debit. See the
[Store bootstrap workflow](wasm-store.md). Adding Store to an already retained
fleet remains an unqualified adoption path; the existing mainnet demo below is
unchanged. Fresh fleets with Store publish each application by content hash before
admission; existing inline fleets keep their original delivery contract. A changed
Store-backed child with unpublished bytes first needs publication plan/apply, then a separate upgrade
plan/apply. See [whole-fleet artifacts](wasm-store.md#whole-fleet-artifacts).

## Add a child to a completed fleet

For a generated starter with only its original application, declare a second
instance of the same package, then build and plan:

```sh
canic app canister add worker --app starter --parent root --role worker \
  --wasm artifacts/worker.wasm --manifest Cargo.toml --package starter-app
canic app build --app starter --canister worker
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
```

Review and apply the returned operation with `canic fleet apply OPERATION --app
starter --fleet mainnet --identity canic-mainnet`. Store-backed fleets first return
publication; after applying it, replan for the separate child creation approval.
Repeat apply on that exact creation operation to retain its completion. The
original fleet operation continues to describe its original approved children.
`canic fleet show --app starter --fleet mainnet` includes the additional child.

Finish outstanding work before adding children. Change existing artifacts and
add a child in separate reviewed windows. Replanning an unfinished addition
returns its original schedule and digest, including after expiry; it cannot
authorize a replacement. Later whole-fleet planning includes the added child in
serial upgrades and complete Root maintenance. This is bounded growth to two
children, not general allocation or spare-canister reuse.

The named `fleet plan --canister` path can also select the added child's next
upgrade. It verifies the explicit target and predecessor module against retained
history, and the explicit Root module against completed maintenance. Store-backed
named plans retain their bound-Store and completed `--release` prerequisites.
Both planners validate the same custody history; switching to whole-fleet planning
returns the identical pending upgrade. Each successor needs the previous upgrade's
retained completion and exact predecessor hash; both paths keep the eight-upgrade
bound and immutable historical approvals.

From 0.218, upgrades stop the target and drain callbacks before installation,
then restart it before readiness checks. Expect application downtime during this
window. Stop/start attempts retain the original deadline and are never replaced
after an uncertain outcome. Expiry or a refused installation can leave the target
stopped; retain the operation and its evidence for explicit operator recovery.
An unfinished legacy Host installation without retained stop evidence is blocked.
Blob admission remains closed after restart until its separately approved activation.

## Current mainnet demo

The maintainer's generated `starter/` project now has a live Root and greeting app.
Root `epxmv-tiaaa-aaaah-awe2a-cai` has completed the qualified predecessor
transition; app `eiwkb-6qaaa-aaaah-awe2q-cai` has subsequently upgraded in place.
From the Canic checkout:

```sh
canic fleet show --app starter
```

The greeting is `Hello from upgraded Canic, team!`. Keep `starter/.canic/` for
recovery. The [journey record](../qualification/history.md#mainnet-predecessor-transition-and-subsequent-app-upgrade)
contains the exact operations, artifact hashes and funding evidence. The creation
walkthrough below is for a fresh project; the existing demo needs no new creation.

## Try the installed local journey

From the Canic checkout, build the CLI and choose an unused directory for a new project.
`make canic` installs the CLI on PATH; all examples use the `canic` command:

```sh
make canic
canic app init starter --source starter
canic fleet init mainnet --app starter --network ic
canic app build --app starter
cd starter
canic app canister list --app starter
canic fleet canister list --app starter --fleet mainnet
```

Initialization writes only local files. `--source` owns a new config inside that
directory and cannot be combined with `--config`. The parent directory must exist;
existing destinations are never replaced. A complete source/config tree publishes
together; errors retain any uncertain staging path for inspection. Generation
neither fetches dependencies nor prepares tools. Keep the generated Cargo lockfile
with your project after the first `app build`; later builds preserve it.

The generated app uses the selected local Canic checkout, with its supplier
selections, compiler and lint policy. Root and Store remain supplied Canic packages.
Canic source aliases resolve to their physical path. If that checkout moves,
update the dependency and Root/Store build paths in `canic.toml`.
No legacy crates.io Canic dependency is selected.

`--output` is a replaceable build input, separate from the immutable artifact
bytes retained by deployment plans. Relative artifact paths resolve beside the
generated config, and its sole app/fleet can be inferred by listing commands.

Back in the Canic checkout, qualify the installed graph with the same generator:

Prepare the simulator once with `make qualification-setup`, then run:

```sh
make root-test
```

This suite explicitly retains inline-custody fixtures for existing fleets. It
generates a standalone app using the CLI and builds its application
and supplied Root through `app build` and the locked artifact builder, then installs
Root in PocketIC, admits
one child, upgrades Root before dispatch, and observes creation and readiness
without another operator mutation. It checks the physical controller sets,
unauthorized admission, a greeting response, exact replay and refusal of changed
authority. Cycles are synthetic; no browser or terminal window opens. The local
simulator stops when qualification finishes.

The same suite also qualifies two children through the runtime and installed CLI: separate
creation and upgrade histories, sibling progress after callback loss, full-budget
artifact custody, shared cycle reservations and preservation on fresh records.
Old allocator images are tested as refused upgrades, not migrations. The CLI fixture covers creation, lost admission replies, separate child
upgrades, maintenance with both full journals, and terminal/HTML views. Untracked
work in either child blocks Root maintenance before installation.

The generated Root/Store/app default has a separate installed CLI witness:

```sh
bash scripts/dev/test-publication.sh fleet_store_upgrade_retains_authority_recovers_and_feeds_root_and_child_plans
```

It initializes and builds all three declarations, deploys the normal Root and
Store, publishes the app and retains completion. It then qualifies interrupted
Store maintenance and subsequent Root/child upgrades using test-only fault code.
It edits and rebuilds the generated greeting source, publishes that new artifact
and verifies the changed greeting directly at the same principal.
An unchanged rebuild preserves artifact identities and the project lockfile.

## A fresh two-child fleet

From the Canic checkout, choose an unused project name. This adds a second
instance of the generated greeting package with its own artifact output:

```sh
canic app init siblings --source siblings
canic fleet init mainnet --app siblings --network ic
canic app canister add worker --app siblings --parent root --role worker --wasm artifacts/worker.wasm --manifest Cargo.toml --package starter-app
canic app build --app siblings
canic fleet plan --app siblings --identity canic-mainnet
```

The manifest and Wasm paths are relative to the generated config. `--manifest`
and `--package` attach the existing build recipe; both require a Wasm output.
Planning retains an approval but has no network effects. Prepare the signer and
fund its cycles-ledger account before applying the reviewed plan. The default
two-child budget is 5T cycles, including Store's separate 1T and 1T attached to
each application creation from Root's allocation:

```sh
canic fleet apply OPERATION --app siblings --identity canic-mainnet
canic fleet show --app siblings
```

Apply again to retain completed outcomes after autonomous work finishes. Two-child
greeting calls select the retained child name with `--canister`; the sole child
remains inferred for the starter. Fleet views remain read-only and open no browser.
Each child has up to eight serial upgrades. Change one child artifact at a time,
then use the same plan/apply commands; change Root separately after both complete.
Maintenance verifies complete custody and both children's code/controllers.
The current Root migrates historical positional child records to identified
collections in its original Storage allocations. Approval and pin identities
remain unchanged, but older executables refuse the new stored format. Finish
admitted work and use the reviewed maintenance plan; this transition does not
support downgrading to the pre-migration Root.

The initial approval fixes its names and permanent slots. A completed one-child
fleet can add its second child under a separate approval, as described above;
renaming or removing retained children is refused. This walkthrough does not
change the existing mainnet starter. Arbitrary tree depth, growth beyond two
children and subtree cascades remain future qualification work.

## Inspect the starter inputs

To inspect/build the inputs yourself:

```sh
canic artifact build --package canic-root
canic artifact build --package ex-template
canic artifact inspect target/root/root.wasm
```

The last command requires the artifacts retained by `make root-test`. Root and
application bytes are under `target/root/`; those are build outputs, not custody
for a deployed mainnet fleet.

## Deploy the generated application

For a new generated fleet, build all three declared artifacts first and prepare the
qualified signer and manually funded cycles-ledger account. From the generated
directory (`cd starter` from the Canic checkout), use your ICP identity name.
Planning reads its public principal; it does not sign. These commands select the generated app and fleet:

```sh
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
canic fleet status OPERATION --app starter --fleet mainnet
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
```

The preview binds all three artifacts, the network and a default 4T total debit:
3T for Root (including its 1T application attachment) and 1T for Store. It fixes
dispatch/deadline bounds. The child's parent is the typed output of Root creation.
Apply creates Root/Store, hands Store to Root, publishes the application and admits it without
asking for a second approval or editing the original plan. New plans dispatch once
approved prerequisites are ready, within ten minutes of planning. Apply promptly;
bootstrap and publication consume that same window. Explicit future `--execute-at`
schedules still require admission before their dispatch time. Retained plans keep
their original timing. Use the same operation to retain completion:

```sh
canic fleet show --app starter --fleet mainnet
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
```

Replanning unfinished work returns the original operation and deadlines; it never
renews expired authority. After completion, unchanged inputs report no artifact
changes against retained evidence, without signing or refreshing network state. Completed apply checks current code/controllers without signing or
creating again. Status is local evidence; watching never retains completion.
The two steps share one approval digest in fleet views. Original single-canister
journals remain readable. This first fleet path refuses pre-bound IDs, larger
trees and unrelated single-canister plans. Existing-fleet adoption is not
implemented; its own completed child can receive successive upgrades below. Preserve the complete journal and resume the same operation.

## Recovery and cleanup

An expired approval can be replaced only before effect preparation. Cancel it
explicitly, retaining the revocation, then review a new plan:

```sh
canic fleet cancel OPERATION --app starter --fleet mainnet
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
```

Once preparation begins, cancellation refuses. Preserve the original operation;
expiry or a failed read does not prove that no effect occurred.

Back up the config and its entire hidden .canic directory together. Git ignores
the journal; it contains irreplaceable attempts and outcomes. Keep source and
Cargo.lock for future builds, and retain the signer through ICP's identity storage.
target/ and artifacts/ are replaceable build outputs; each plan retains its own
approved Wasm bytes. A never-planned project can be deleted locally. After planning,
archive its evidence first. Local deletion never removes canisters or refunds cycles.

While Root works, `fleet show` automatically follows the Root in the retained
fleet or child plan. No `--manager` flag or repeated apply is required for a progress view:

```sh
canic fleet show
canic fleet show --watch
```

The watch serves `http://localhost:8787/` until Ctrl-C and opens no browser.
Both views query only, retain the plan's exact network binding and check the
reported approval before observing its child. A public report cannot bind the
child to its declared slot or create a completion receipt. These reads consume
none of the 32 retained signed observation attempts. Root reports and reported
child identity remain labelled separately from authenticated deployment evidence.

Each selected Root is queried for both supported child slots, including children
created through another operator's journal. A configured Root can be selected
explicitly with `canic fleet show --manager ROOT_PRINCIPAL`; its identity must
already have an exact configured network binding. Confirmed empty additional
slots are omitted, while unavailable endpoints produce an inventory issue.
Discovery neither names an undeclared child nor grants custody, and currently
stops at these two slots rather than recursively traversing descendants.
Terminal and browser reports label each physical slot (`slot 1` or `slot 2`),
alongside an unambiguous local child name or its reported principal. The slot
label is diagnostic context, not a new app definition or an inferred tree edge.

The read-only replicated observation can recover a lost staging response without
re-signing admission. Its fresh signing authority cannot invoke management
effects or extend Root's effect deadline. Up to 32 observation attempts are
retained; an unresolved signed observation blocks rather than being replaced.
Public status queries remain observational and cannot establish child custody.
`fleet status` reads retained evidence without network calls or signatures.
Root code/controller drift blocks apply; keep the journal and original attempts.

For a blocked operation, inspect retained evidence before attempting recovery:

```sh
canic fleet status OPERATION
canic fleet status OPERATION --json
```

Use UPGRADE_OPERATION instead for an upgrade, and keep the same config, fleet
and journal selection. The human view shows Root dispatch counts, request-hash
custody and the retained failure. An unknown creation refund stays unknown;
it is not displayed as zero. Upgrade versions describe the retained management
observations, not a fresh code check. Live observations remain in `fleet show`.

The suggested next step depends on that evidence. Pending work calls for
observation; Ready without a receipt calls for applying the original operation
to verify completion. Issued effects without retained outcomes require review
if fresh observations remain unresolved. An expired or reserve-blocked Root
slot stays consumed; adding cycles does not reset it. Explicit `fleet reconcile OPERATION --identity canic-mainnet` requests recovery
under the original approval on a compatible owner; it cannot renew deadlines or
reset slots. Owner maintenance still requires the qualified controller path.
Status never performs those actions automatically.

The installed CLI fixture creates and installs Root through the public plan/apply
commands, then resolves the child's parent from that completed receipt without
binding an ID into config. It covers admission, lost staging reply recovery,
sole Root control, exact replay, fleet projection and receipt recovery without
another signature. It repeats deployment in a second fleet of the same generated
app, checks separate journals and identities, and refuses cross-fleet apply before
signing. The fixture uses synthetic cycles; this increment has not deployed a new
mainnet Root or application.

## Reuse the app in another fleet

From an existing generated project, choose an unused fleet name. These commands
only add and inspect declarations; the second fleet shares the app's role, parent
and Wasm definitions but starts with no canister identities or journal:

```sh
canic fleet init second --network ic
canic fleet list
canic fleet canister list --fleet second
```

With two fleets, select `--fleet NAME` on each deployment or observation command.
Use the whole-fleet sequence above with `--fleet second` added
to each `fleet plan`, `status`, `apply`, `show` and `call` command. Each fleet has
its own Root, child and default journal. Mainnet apply spends cycles for this
additional deployment; initializing the fleet does not copy the first fleet's IDs
or reuse its approved operations.

## Upgrade the existing child

For a fleet created through the whole-fleet path above, first retain its creation
completion with apply. Edit the generated application's greeting source, then
rebuild and plan from its project directory:

```sh
canic app build
canic fleet plan --identity canic-mainnet
canic fleet apply PUBLICATION_OPERATION --identity canic-mainnet
canic fleet plan --identity canic-mainnet
canic fleet status UPGRADE_OPERATION
canic fleet apply UPGRADE_OPERATION --identity canic-mainnet
```

New Store-backed bytes first need publication; copy PUBLICATION_OPERATION from
that plan, apply it, then plan again for the upgrade. Already published bytes and
existing inline fleets skip that publication step.

The planner resolves the existing child, previous module and exact Root module
from retained completion; no principal or hash needs copying. Review the upgrade
preview before applying. It has no creation debit and preserves the same child
identity. Change Root and child in separate reviewed windows; changing both
artifacts together is refused. Unfinished creation must complete with its original inputs before upgrading.
An already retained upgrade keeps its original bytes and schedule; a different
input cannot replace it, including after expiry or failure.

The named `--canister` / `--root-module` path can also plan a child's next upgrade
from a whole-fleet deployment. It resolves the original runtime slot from retained
creation custody, including the second child, rather than configuration order.
Both paths use the same immutable managed plan, serial history and runtime owner.
Named successors require the latest completed predecessor hash. Whole-fleet
planning resolves it automatically.

Add `--fleet NAME` when the app has multiple fleets. Copy UPGRADE_OPERATION from
the new plan. New upgrades dispatch when ready within the original ten-minute
effect window. Replanning returns that original authority. After dispatch,
repeat the same apply to retain authenticated completion:

```sh
canic fleet apply UPGRADE_OPERATION --identity canic-mainnet
canic fleet status UPGRADE_OPERATION
```

Root checks the old code, sole-controller relationship and canister version
before installation. It retains upgrade intent before dispatch, uses upgrade
mode, and requires replacement code, exactly one version increment and the
application's readiness response before reporting completion. The child keeps
its principal and Root controller. Creation evidence and its receipt remain
historical records; they do not describe the replacement executable.

This experimental owner retains **at most eight serial upgrades**, separate from
creation. After verified successful completion, edit/build/plan/apply again for
the next change. Each approval is new authority linked to the last completed code,
not a reset of earlier work. Expired, failed and uncertain operations cannot be
overtaken; an uncertain install is never issued again. History exhaustion blocks
new work without erasing any approval or outcome. A lost host admission reply can be recovered through
`fleet apply`'s replicated read. Use explicit `fleet reconcile UPGRADE_OPERATION
--identity canic-mainnet` for supported engine recovery on a compatible Root.
Owner maintenance still requires the qualified controller path. Preserve blocked
evidence; do not delete the journal or reinstall to obtain another allowance.

`fleet status UPGRADE_OPERATION` shows retained upgrade evidence. `fleet show`
and `--watch` query creation and the upgrade selected by the retained plan:

```sh
canic fleet show
canic fleet show --watch
```

With several retained upgrades, the view selects the tip of the completed
predecessor chain and keeps historical operation rows.

The upgrade is shown as not admitted before staging, then its reported phase.
These reports are uncertified and must match the complete approval and network;
an unavailable or mismatched report is UNKNOWN. They share the existing five-second
owner observation budget. Fleets without an upgrade plan receive no upgrade query.
Live Ready does not retain a receipt; repeat the original apply for authenticated
completion. Watching never reconciles or authorizes work, even when a report is
blocked. The localhost view opens no browser and stops with Ctrl-C.
The installed CLI fixture upgrades
one generated greeting while preserving the second fleet and both creation
receipts. It also drives the same whole-fleet flow with a profile manager/worker,
checking private data and owner access after upgrade. `make growth-test` additionally
proves profile data/ownership across a
changed executable, rejected upgrade, callback loss and owner maintenance.

## Root maintenance

For Store-backed children, the same workflow retains every artifact reference,
pin, cache deadline and historical upgrade source. Both executables must support
the complete `observe_stored_children` contract. The preview identifies the Store
and pin count; `--json` exposes the full approved custody. Store code, sole Root
control and subnet placement are checked before and after installation alongside
the child checks. An interrupted completion check resumes the original installation
attempt; it cannot declare success with missing source evidence.

Whole-fleet planning also accepts a Root-only artifact change after creation and
every admitted child upgrade have completed. It preserves the Root/child principals,
controllers, original creation and full upgrade journal. Root and child changes
need separate approvals; pending, failed or uncertain child work blocks maintenance.

For a generated `starter` project directly under this checkout, with a completed
fleet created using the current custody-capable Root, build a reviewed replacement
from the Canic checkout. This example selects the qualified maintenance marker;
it does not enable fault injection:

```sh
canic artifact build --package canic-root --features maintenance-candidate --output starter/artifacts/root.wasm
canic fleet plan --app starter --identity canic-mainnet
canic fleet status MAINTENANCE_OPERATION --app starter
```

After reviewing the returned plan, use its digest. Add `--fleet NAME` if needed:

```sh
canic fleet apply MAINTENANCE_OPERATION --app starter --identity canic-mainnet
canic fleet show --app starter
canic fleet plan --app starter --identity canic-mainnet
```

Keep this operator window exclusive: no out-of-band staging, installation or
controller changes. Apply verifies certified Root and child bindings and obtains
a replicated full-journal observation before installation. It retains the exact
signed install, verifies journal preservation afterward, then writes completion.
Resume the same digest after interruption. An expired uncertain install stays
blocked; a new signature, reinstall or automatic rollback is not a recovery path.
Future child plans bind the new Root hash; historical approvals remain unchanged.
The original creation approval may no longer apply against the new Root code;
inspect its historical receipt with `fleet status`.

The replacement must export `observe_custody` (ID 30). The predecessor normally
uses that contract too. One retained single-upgrade Root, SHA-256
`e4a4127abd525f59378503dcdb98fcb13f8eb9895673d4d8a4dea41d85e24296`, instead uses
its controller-only creation and unit upgrade observations before installation.
Both replies must match the approved journal, including an empty upgrade slot
when no upgrade is recorded. The same full-custody postflight then proves the
transition. The module hash selects this contract before dispatch; there is no
endpoint probing, error fallback or change to old approvals. Other old binaries,
maintenance during active/uncertain work, and Root replacement need separate
qualification. Existing mainnet demos are unchanged.

## Responsibility and bounds

`canic-root` mounts its lifecycle and management endpoints explicitly. It uses
the same `canic-management` engine as the profile fixture manager, with an explicit
`readiness` request that accepts only `Ready`. Each app owns its prerequisite and
fence checks. The independent greeting/probe endpoints remain available to callers;
Root does not use them to infer readiness.

`canic-management-protocol` contains only authority/evidence messages, hashing
and decoding limits. Host tooling and application protocol consumers do not link
the management runtime. Existing `manager` and `worker` wire fields, RPC numbers,
stable keys and enum ordering remain unchanged, preserving retained profile-demo
authority. Upgrade messages have their own method numbers. The versioned stable
payload migration packs the separate creation/upgrade journals and artifact bytes
inside the original two checked allocations, under the existing 64-page bound.
The v3 journal retains creation, the current upgrade and up to seven completed
predecessors within the existing 8 KiB cell. It reads original creation and v2
single-upgrade payloads before validation and migration. Historical artifact
bytes remain in the host journal; Root retains their hashes and outcomes.
It validates the original payloads before writing the new format during lifecycle
execution. Downgrading to a binary that only reads the old payload is unsupported.

For inline delivery, the engine retains one creation approval and at most 512 KiB of application Wasm, attaches
1T cycles on creation and requires a 0.5T liquid reserve. Each creation, upload
and installation effect is issued at most once. An uncertain creation with no
retained principal stays blocked; retry does not obtain another allowance.
Module/controller observations precede application-reported readiness. These
bounds and the separate-message commit boundaries come from the existing
[growth qualification](growth.md).
An inline upgrade approval retains at most another 512 KiB, uses existing
canister cycles and checks the same 0.5T liquid reserve. Application data remains
owned by the application's lifecycle; management evidence alone cannot prove
that an arbitrary application's migration preserves its data.
Store-backed children instead retain immutable references and pins; the Store
owns up to 8 MiB per artifact and the derived management cache.

## Remaining work

The generated starter qualifies Root, a separate Store and one application
through one CLI approval, with a separate two-application witness. It does
not yet admit larger trees, active-work Root maintenance, unbounded upgrade history,
cascading discovery or subtree operations. Root maintenance preserves
its authority and child identity; the stateless greeting makes no application
data-preservation claim. Existing mainnet profile canisters are unchanged.
