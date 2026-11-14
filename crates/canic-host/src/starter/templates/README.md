# Canic starter

Edit `app/src/lib.rs` to change the greeting. Root is supplied by Canic.

```text
Root
└─ Application
```

The operator is Root's external controller, outside this canister tree.

## Build

From this directory, prepare and retain a Cargo lockfile, then build:

```sh
cargo generate-lockfile
canic artifact build --package starter-app --output artifacts/app.wasm
canic artifact build --manifest {} --package canic-root --output artifacts/root.wasm
canic app canister list
canic fleet canister list
```

The dependency uses your selected local Canic checkout. If it moves, update
Cargo.toml and the Root build command. No crates.io Canic package is selected.

## Deploy through Root

Generation deploys nothing. For mainnet, prepare the qualified ICP signer and
manually fund its cycles-ledger account first. Replace OPERATOR_PRINCIPAL with
your signing principal; use the identity that owns it in the apply commands.
These apply commands spend cycles and install code on the configured network.

Bootstrap Root with 3T cycles, leaving room for its 1T child and 0.5T reserve:

```sh
canic fleet plan --canister root --controller OPERATOR_PRINCIPAL --cycles 3000000000000
canic fleet status ROOT_OPERATION
canic fleet apply ROOT_OPERATION --identity canic-mainnet
canic artifact inspect artifacts/root.wasm
```

Copy ROOT_OPERATION from the plan output. Copy ROOT_SHA256 from the artifact
inspection, then explicitly approve the child's Root owner:

```sh
canic fleet plan --canister app --controller OPERATOR_PRINCIPAL --root-module ROOT_SHA256
canic fleet status CHILD_OPERATION
canic fleet apply CHILD_OPERATION --identity canic-mainnet
```

Copy CHILD_OPERATION from this plan. Apply admits the retained artifact to Root;
Root dispatches ten minutes after planning, then continues without the host.
Review and submit promptly. `--execute-at UNIX_SECONDS` can choose another
future dispatch for a new plan. Replanning a slot returns its original authority
and schedule. Root's effect deadline is ten minutes after dispatch.

After dispatch, repeat the same apply to retain a replicated progress observation
and, once ready, a verified completion receipt:

```sh
canic fleet apply CHILD_OPERATION --identity canic-mainnet
canic fleet show
canic fleet call CHILD_OPERATION --name team
```

To follow progress before retaining completion, use `canic fleet show`; it
automatically queries the Root named in the child plan. `canic fleet show --watch`
serves the read-only live view at http://localhost:8787/ until Ctrl-C, without
opening a browser. Public reports do not bind the child slot or create receipts,
and watching consumes no signed observation attempts.

Root must retain the approved code and sole operator controller. Its child has
Root as its sole controller. Pending admission is not a completion receipt.
Keep the config and its .canic journal; do not delete attempts to retry.

## First application upgrade

After the child has a completion receipt, edit the greeting and rebuild its
app-defined artifact. Copy CHILD_PRINCIPAL and PREVIOUS_SHA256 from that receipt.
Root must already run the upgrade-capable executable inspected above. Planning
is local; apply admits an in-place upgrade on the selected network:

```sh
canic artifact build --package starter-app --output artifacts/app.wasm
canic fleet plan --canister app --target CHILD_PRINCIPAL --previous-module PREVIOUS_SHA256 --root-module ROOT_SHA256 --controller OPERATOR_PRINCIPAL
canic fleet status UPGRADE_OPERATION
canic fleet apply UPGRADE_OPERATION --identity canic-mainnet
```

After dispatch, repeat that apply to retain completion. The child keeps its
principal and Root controller; the original creation receipt remains historical
evidence. `fleet status UPGRADE_OPERATION` shows the retained upgrade phase.
`fleet show` and `fleet show --watch` display separate creation and upgrade reports,
matched to the planned authority and network. Public Ready cannot create a
receipt; repeat the original apply to retain authenticated completion. Watch
serves localhost without opening a browser and stops with Ctrl-C.

This experimental owner has one permanent upgrade slot, separate from creation.
It never retries an uncertain installation or replaces expired authority.
Application persistence requires its own qualified upgrade lifecycle; this
greeting app has no persistent data. Whole-fleet convergence, repeated upgrades
and subtree operations remain future work.
