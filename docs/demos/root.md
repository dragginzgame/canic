# Root and its children

The generated starter begins with one approved Hello World child; a fresh fleet
can include a second direct child:

```text
Root
└─ Application
```

Root's controller is the external operator identity. The application's sole
controller is Root. External identities appear as controller metadata, never as
nodes above the canister tree.

A fresh fleet can also declare one `wasm-store` child of Root, alongside both
application slots. Whole-fleet apply completes Store handover/readiness before
admitting either app; the approval includes its separate 1T debit. See the
[Store bootstrap workflow](wasm-store.md). Adding Store to an already retained
fleet remains an unqualified adoption path; the existing mainnet demo below is
unchanged. Applications still use inline deployment until Store release selection
is integrated.

## Current mainnet demo

The maintainer's generated `starter/` project now has a live Root and greeting app.
Root `epxmv-tiaaa-aaaah-awe2a-cai` has completed the qualified predecessor
transition; app `eiwkb-6qaaa-aaaah-awe2q-cai` has subsequently upgraded in place.
From the Canic checkout:

```sh
canic fleet show --app starter
canic fleet call 0a68a8de48d4ca642147087a893813359ca4b7eda6847cf554acfaaa91c757cc --app starter --name team
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
canic app init starter --fleet mainnet --network ic \
  --source starter
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
selections, compiler and lint policy. Root remains a supplied Canic package.
Canic source aliases resolve to their physical path. If that checkout moves,
update the dependency and Root build paths; the generated README records the
Root workspace manifest in `canic.json`. No legacy crates.io Canic dependency is selected.

`--output` is a replaceable build input, separate from the immutable artifact
bytes retained by deployment plans. Relative artifact paths resolve beside the
generated config, and its sole app/fleet can be inferred by listing commands.

Back in the Canic checkout, qualify the installed graph with the same generator:

Prepare the simulator once with `make qualification-setup`, then run:

```sh
make root-test
```

The test generates a standalone app using the CLI and builds its application
and supplied Root through `app build` and the locked artifact builder, then installs
Root in PocketIC, admits
one child, upgrades Root before dispatch, and observes creation and readiness
without another operator mutation. It checks the physical controller sets,
unauthorized admission, a greeting response, exact replay and refusal of changed
authority. Cycles are synthetic; no browser or terminal window opens. The local
simulator stops when qualification finishes.

The same suite also qualifies two children through the runtime and installed CLI: separate
creation and upgrade histories, sibling progress after callback loss, full-budget
artifact custody, shared cycle reservations and migration from a retained old
Root. The CLI fixture covers creation, lost admission replies, separate child
upgrades, maintenance with both full journals, and terminal/HTML views. Untracked
work in either child blocks Root maintenance before installation.

## A fresh two-child fleet

From the Canic checkout, choose an unused project name. This adds a second
instance of the generated greeting package with its own artifact output:

```sh
canic app init siblings --fleet mainnet --network ic --source siblings
canic app canister add worker --app siblings --parent root --role worker --wasm artifacts/worker.wasm --manifest Cargo.toml --package siblings-app
canic app build --app siblings
canic fleet plan --app siblings --identity canic-mainnet
```

The manifest and Wasm paths are relative to the generated config. `--manifest`
and `--package` attach the existing build recipe; both require a Wasm output.
Planning retains an approval but has no network effects. Prepare the signer and
fund its cycles-ledger account before applying the reviewed plan. The default
two-child budget is 4T cycles, including 1T attached to each child creation:

```sh
canic fleet apply OPERATION --app siblings --identity canic-mainnet
canic fleet show --app siblings
canic fleet call OPERATION --app siblings --canister app
canic fleet call OPERATION --app siblings --canister worker
```

Apply again to retain completed outcomes after autonomous work finishes. Two-child
greeting calls select the retained child name with `--canister`; the sole child
remains inferred for the starter. Fleet views remain read-only and open no browser.
Each child has up to eight serial upgrades. Change one child artifact at a time,
then use the same plan/apply commands; change Root separately after both complete.
Maintenance verifies complete custody and both children's code/controllers.

The initial approval fixes names and permanent slots. Renaming, adding or removing
children from a retained fleet is refused. To try two children, create a fresh
fleet; this does not expand the existing mainnet starter. Arbitrary tree depth,
dynamic growth and subtree cascades remain future qualification work.

## Inspect the starter inputs

To inspect/build the inputs yourself:

```sh
canic artifact build --package canic-root
canic artifact build --package canic-hello
canic artifact inspect target/root/root.wasm
```

The last command requires the artifacts retained by `make root-test`. Root and
application bytes are under `target/root/`; those are build outputs, not custody
for a deployed mainnet fleet.

## Deploy the generated application

For a new generated fleet, build both declared artifacts first and prepare the
qualified signer and manually funded cycles-ledger account. From the generated
directory (`cd starter` from the Canic checkout), use your ICP identity name.
Planning reads its public principal; it does not sign. These commands select the generated app and fleet:

```sh
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
canic fleet status OPERATION --app starter --fleet mainnet
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
```

The preview binds both artifacts, the network, a default 3T Root debit, its 1T
child attachment and a fixed dispatch/deadline. The child's parent is the typed
output of Root creation. Apply creates/installs Root and admits the child without
asking for a second approval or editing the original plan. Root dispatches ten
minutes after planning; the final effect deadline is ten minutes later. Apply
promptly. After dispatch, use the same operation to retain completion:

```sh
canic fleet show --app starter --fleet mainnet
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet call OPERATION --app starter --fleet mainnet --name team
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
canic fleet status UPGRADE_OPERATION
canic fleet apply UPGRADE_OPERATION --identity canic-mainnet
```

The planner resolves the existing child, previous module and exact Root module
from retained completion; no principal or hash needs copying. Review the upgrade
preview before applying. It has no creation debit and preserves the same child
identity. Change Root and child in separate reviewed windows; changing both
artifacts together is refused. Unfinished creation must complete with its original inputs before upgrading.
An already retained upgrade keeps its original bytes and schedule; a different
input cannot replace it, including after expiry or failure.

The named `--canister` / `--root-module` path remains for individually deployed
children; both paths use the same immutable managed plan and runtime owner.

Add `--fleet NAME` when the app has multiple fleets. Copy UPGRADE_OPERATION from
the new plan. The ten-minute default dispatch delay and following ten-minute
effect window apply. Replanning returns that original authority. After dispatch,
repeat the same apply to retain authenticated completion:

```sh
canic fleet apply UPGRADE_OPERATION --identity canic-mainnet
canic fleet status UPGRADE_OPERATION
canic fleet call UPGRADE_OPERATION --name team
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
the same `canic-management` engine as the profile manager, with its own greeting
readiness check. The greeting app's replicated `probe` method shares admission
and response behavior with its existing `greet` query.

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

The engine retains one creation approval and at most 512 KiB of application Wasm, attaches
1T cycles on creation and requires a 0.5T liquid reserve. Each creation, upload
and installation effect is issued at most once. An uncertain creation with no
retained principal stays blocked; retry does not obtain another allowance.
Module/controller observations precede application-reported readiness. These
bounds and the separate-message commit boundaries come from the existing
[growth qualification](growth.md).
The current upgrade approval retains at most another 512 KiB, uses existing
canister cycles and checks the same 0.5T liquid reserve. Application data remains
owned by the application's lifecycle; management evidence alone cannot prove
that an arbitrary application's migration preserves its data.

## Remaining work

This qualifies a generated two-canister fleet through one CLI approval. It does
not yet admit larger trees, active-work Root maintenance, unbounded upgrade history,
cascading discovery or subtree operations. Root maintenance preserves
its authority and child identity; the stateless greeting makes no application
data-preservation claim. Existing mainnet profile canisters are unchanged.
