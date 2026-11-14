# One-child Root

The experimental Root creates and installs one approved Hello World canister:

```text
Root
└─ Application
```

Root's controller is the external operator identity. The application's sole
controller is Root. External identities appear as controller metadata, never as
nodes above the canister tree.

## Try the installed local journey

From the Canic checkout, choose an unused directory for a new project:

```sh
canic app init starter --fleet mainnet --network ic \
  --source ./starter --canic-source .
cd starter
cargo generate-lockfile
canic artifact build --package starter-app --output artifacts/app.wasm
canic artifact build --manifest ../Cargo.toml --package canic-root --output artifacts/root.wasm
canic app canister list
canic fleet canister list
```

Initialization writes only local files. `--source` owns a new config inside that
directory and cannot be combined with `--config`. The parent directory must exist;
existing destinations are never replaced. A complete source/config tree publishes
together; errors retain any uncertain staging path for inspection. Generation
neither fetches dependencies nor prepares tools. Keep the generated Cargo lockfile
with your project after preparation.

The generated app uses the selected local Canic checkout, with its supplier
selections, compiler and lint policy. Root remains a supplied Canic package.
Canic source aliases resolve to their physical path. If that checkout moves,
update the dependency and Root build paths; the generated README records the
exact build command. No legacy crates.io Canic dependency is selected.

`--output` is a replaceable build input, separate from the immutable artifact
bytes retained by deployment plans. Relative artifact paths resolve beside the
generated config, and its sole app/fleet can be inferred by listing commands.

Back in the Canic checkout, qualify the installed graph with the same generator:

Prepare the simulator once with `make m0-setup`, then run:

```sh
make root-test
```

The test generates a standalone app using the CLI, prepares its lockfile offline,
builds its application and supplied Root using the CLI artifact builder, installs
Root in PocketIC, admits
one child, upgrades Root before dispatch, and observes creation and readiness
without another operator mutation. It checks the physical controller sets,
unauthorized admission, a greeting response, exact replay and refusal of changed
authority. Cycles are synthetic; no browser or terminal window opens. The local
simulator stops when qualification finishes.

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

From the generated project directory, use the identity that owns
`OPERATOR_PRINCIPAL`. Prepare the qualified ICP signer and fund its cycles-ledger
account manually. Replace the uppercase operation/hash placeholders with the
values printed by planning and artifact inspection. These apply commands spend
cycles and install code on the selected network:

```sh
canic fleet plan --canister root --controller OPERATOR_PRINCIPAL --cycles 3000000000000
canic fleet status ROOT_OPERATION
canic fleet apply ROOT_OPERATION --identity canic-mainnet
canic artifact inspect artifacts/root.wasm
canic fleet plan --canister app --controller OPERATOR_PRINCIPAL --root-module ROOT_SHA256
canic fleet status CHILD_OPERATION
canic fleet apply CHILD_OPERATION --identity canic-mainnet
```

The completed named Root receipt supplies its identity; no duplicate ID binding
is needed. An explicitly configured existing Root is also supported. The child
plan pins Root's code and sole operator controller, the child's bytes and 1T
attachment. Root must retain at least a 0.5T liquid reserve. Its single child slot
cannot be repurposed, even after refusal or expiry.

New child plans schedule dispatch ten minutes after planning. Submit before that
time; `--execute-at UNIX_SECONDS` can select a different future time. Effects
expire ten minutes after dispatch. Replanning returns the original operation
and schedule. A successful pending apply means Root has custody, not that the
child is ready. Root continues after the operator leaves.

After dispatch, repeat apply to retain a replicated observation and complete the
receipt once Root reports readiness and certified child metadata agrees:

```sh
canic fleet apply CHILD_OPERATION --identity canic-mainnet
canic fleet show
canic fleet call CHILD_OPERATION --name team
```

While Root works, `fleet show` automatically follows the Root in the retained
child plan. No `--manager` flag or repeated apply is required for a progress view:

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
canic fleet status CHILD_OPERATION
canic fleet status CHILD_OPERATION --json
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
slot stays consumed; adding cycles does not reset it. Owner maintenance and
controller reconciliation still require the qualified Wire path, with no CLI
recovery command yet. Status never performs those actions automatically.

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
Deploy Root, then its child using the sequence above with `--fleet second` added
to each `fleet plan`, `status`, `apply`, `show` and `call` command. Each fleet has
its own Root, child and default journal. Mainnet apply spends cycles for this
additional deployment; initializing the fleet does not copy the first fleet's IDs
or reuse its approved operations.

## Upgrade the existing child

The first upgrade uses the same named slot, its completed creation receipt and
the same Root. Root must already run the upgrade-capable executable. After
editing the generated greeting, rebuild its app-defined artifact. Copy the child
principal and previous module hash from the retained creation receipt; use the
reviewed hash of the deployed Root. These commands plan locally, then admit an
in-place upgrade on the selected network:

```sh
canic artifact build --package starter-app --output artifacts/app.wasm
canic fleet plan --canister app --target CHILD_PRINCIPAL --previous-module PREVIOUS_SHA256 --root-module ROOT_SHA256 --controller OPERATOR_PRINCIPAL
canic fleet status UPGRADE_OPERATION
canic fleet apply UPGRADE_OPERATION --identity canic-mainnet
```

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

This experimental owner has **one permanent upgrade slot**, separate from the
creation slot. Expiry, failure and completion never reset it. An uncertain install
is never issued again. A lost host admission reply can be recovered through
`fleet apply`'s replicated read. Engine callback failures may require owner
maintenance and controller `reconcile_upgrade`; that recovery is qualified at
the Wire boundary but has no CLI maintenance command yet. Preserve blocked
evidence; do not delete the journal or reinstall to obtain another allowance.

`fleet status UPGRADE_OPERATION` shows retained upgrade evidence. `fleet show`
and `--watch` query creation and the upgrade selected by the retained plan:

```sh
canic fleet show
canic fleet show --watch
```

The upgrade is shown as not admitted before staging, then its reported phase.
These reports are uncertified and must match the complete approval and network;
an unavailable or mismatched report is UNKNOWN. They share the existing five-second
owner observation budget. Fleets without an upgrade plan receive no upgrade query.
Live Ready does not retain a receipt; repeat the original apply for authenticated
completion. Watching never reconciles or authorizes work, even when a report is
blocked. The localhost view opens no browser and stops with Ctrl-C.
The installed CLI fixture upgrades
one generated greeting while preserving the second fleet and both creation
receipts. `make growth-test` separately proves profile data/ownership across a
changed executable, rejected upgrade, callback loss and owner maintenance.

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
It validates the original payloads before writing the new format during lifecycle
execution. Downgrading to a binary that only reads the old payload is unsupported.

The engine retains one creation approval and at most 512 KiB of application Wasm, attaches
1T cycles on creation and requires a 0.5T liquid reserve. Each creation, upload
and installation effect is issued at most once. An uncertain creation with no
retained principal stays blocked; retry does not obtain another allowance.
Module/controller observations precede application-reported readiness. These
bounds and the separate-message commit boundaries come from the existing
[growth qualification](growth.md).
The separate upgrade approval retains at most another 512 KiB, uses existing
canister cycles and checks the same 0.5T liquid reserve. Application data remains
owned by the application's lifecycle; management evidence alone cannot prove
that an arbitrary application's migration preserves its data.

## Remaining work

This qualifies a generated app and one-child Root implementation. It does not yet
create a complete fleet through one CLI plan, admit repeated child upgrades, or
provide cascading discovery and subtree operations. Root maintenance preserves
its authority and child identity; the stateless greeting makes no application
data-preservation claim. Existing mainnet profile canisters are unchanged.

The generated app and supplied Root now use the reviewed managed-child CLI path.
Whole-fleet reviewed convergence remains separate work.
