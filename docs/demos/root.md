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

The installed CLI fixture covers admission, lost staging reply recovery, sole
Root control, exact replay, fleet projection and receipt recovery without another
signature. The fixture uses synthetic cycles; this increment has not deployed
a new mainnet Root or application.

## Responsibility and bounds

`canic-root` mounts its lifecycle and management endpoints explicitly. It uses
the same `canic-management` engine as the profile manager, with its own greeting
readiness check. The greeting app's replicated `probe` method shares admission
and response behavior with its existing `greet` query.

`canic-management-protocol` contains only authority/evidence messages, hashing
and decoding limits. Host tooling and application protocol consumers do not link
the management runtime. Existing `manager` and `worker` wire fields, RPC numbers,
stable keys and enum ordering remain unchanged, preserving retained profile-demo
authority. No second protocol reader or compatibility alias is introduced.

The engine retains one approval and at most 512 KiB of application Wasm, attaches
1T cycles on creation and requires a 0.5T liquid reserve. Each creation, upload
and installation effect is issued at most once. An uncertain creation with no
retained principal stays blocked; retry does not obtain another allowance.
Module/controller observations precede application-reported readiness. These
bounds and the separate-message commit boundaries come from the existing
[growth qualification](growth.md).

## Remaining work

This qualifies a generated app and one-child Root implementation. It does not yet
create a complete fleet through one CLI plan, upgrade the child, or
provide cascading discovery and subtree operations. Root maintenance preserves
its authority and child identity; the stateless greeting makes no application
data-preservation claim. Existing mainnet profile canisters are unchanged.

The generated app and supplied Root now use the reviewed managed-child CLI path.
Whole-fleet reviewed convergence remains separate work.
