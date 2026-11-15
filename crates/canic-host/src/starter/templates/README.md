# Canic starter

Edit `app/src/lib.rs` to change the greeting. Root and Wasm Store are supplied by Canic.

```text
Root
├─ Wasm Store
└─ Application
```

The operator is Root's external controller, outside this canister tree.

## Build

From this directory, build and install the CLI from the selected Canic checkout,
then build all three canisters. The first build prepares a missing Cargo.lock; later builds preserve it. `make canic` makes the `canic`
command available on PATH:

```sh
make -C {source} canic
canic app build
canic app canister list
canic fleet canister list
```

The dependency uses your selected local Canic checkout. If it moves, update
Cargo.toml and the Root/Store build manifests in canic.json. No crates.io Canic package is selected.

## Deploy through Root

Generation deploys nothing. For mainnet, prepare the qualified ICP signer and
manually fund its cycles-ledger account first. Planning reads the public principal
of your ICP identity; it signs nothing.
These apply commands spend cycles and install code on the configured network.

Review one approval for Root, Store and the application. The default 4T total
creation debit allocates 1T to Store and 3T to Root. Root funds the 1T application
attachment from its allocation, leaving room for its 0.5T reserve:

```sh
canic fleet plan --identity canic-mainnet
canic fleet status OPERATION
canic fleet apply OPERATION --identity canic-mainnet
```

Copy OPERATION from planning. Apply creates Root and Store, hands Store to Root,
publishes the application bytes, then admits the application by immutable release
reference using the retained results. It does not invent a principal or
rewrite the approval. Root dispatches once approved prerequisites are ready and may
finish without the host; the effect deadline is ten minutes after planning.
Review and apply promptly. An explicit future `--execute-at` schedule requires
admission before dispatch. Replanning preserves original timing, including after expiry.

After dispatch, repeat the same apply to retain a replicated progress observation
and, once ready, a verified completion receipt:

```sh
canic fleet apply OPERATION --identity canic-mainnet
canic fleet show
canic fleet call OPERATION --name team
```

To follow progress before retaining completion, use `canic fleet show`; it
automatically queries the Root named in the child plan. `canic fleet show --watch`
serves the read-only live view at http://localhost:8787/ until Ctrl-C, without
opening a browser. Public reports do not bind the child slot or create receipts,
and watching consumes no signed observation attempts.

Root must retain the approved code and sole operator controller. Its child has
Root as its sole controller, as does Store. Pending admission is not a completion receipt.
Keep the config and its .canic journal; do not delete attempts to retry.

## Upgrade the application

After retaining creation completion, edit `app/src/lib.rs`, then rebuild and review
the next plan. Unpublished bytes first need a publication approval; after applying
it, plan again for installation. Canic resolves the existing child and previous
code from its journal:

```sh
canic app build
canic fleet plan --identity canic-mainnet
canic fleet apply PUBLICATION_OPERATION --identity canic-mainnet
canic fleet plan --identity canic-mainnet
canic fleet status UPGRADE_OPERATION
canic fleet apply UPGRADE_OPERATION --identity canic-mainnet
```

Copy each operation from its corresponding plan. Already published bytes can
skip publication. Apply UPGRADE_OPERATION again after dispatch to retain
completion. The child keeps its identity and Root controller. Unchanged artifacts
report no changes against retained completion; this is not a fresh network check.
Pending work retains its original schedule. Change Root and child artifacts in
separate reviewed windows. A Root-only change plans maintenance once all child
work completes, verifying the full journal before and after installation.

## Current boundary

This generated project declares one unbound Root, Store and one application.
The whole-fleet path also accepts a second direct application child; Store does
not consume an application slot. Existing inline fleets keep their original
delivery contract; adding Store to a retained fleet is not qualified.
Keep its original journal for resumption. It refuses larger trees, pre-existing
identity bindings and unrelated single-canister approvals. Root retains at most
eight serial child upgrades. A successor requires verified successful completion;
failed, expired or uncertain work stays blocked. Full history cannot be reset.
Rebuild and repeat the same plan/apply flow for the next change. Root maintenance
requires both executables to support the full custody observation contract;
older Roots are refused before installation. Adoption of previously deployed
fleets is not implemented. Root checks this application's greeting `probe`
contract; changing the greeting text is supported, replacing that protocol is not.
The greeting is stateless; application data preservation requires its own
lifecycle and qualification.

## Recovery and cleanup

If an approval expires before apply, cancel it explicitly, then plan again:

```sh
canic fleet cancel OPERATION
canic fleet plan --identity canic-mainnet
```

Cancellation is allowed only before effect preparation. It retains the old approval
and permanently prevents applying it. Once preparation starts, preserve and resume
the same operation; expiry is not proof that nothing happened.

Back up canic.json and the entire hidden .canic directory together. The journal is
ignored by Git and contains irreplaceable signed attempts and result evidence.
Keep the signer identity separately through ICP's identity storage. Source and
Cargo.lock are needed to reproduce and upgrade the app. target/ and artifacts/
are replaceable build outputs; planning retains its own artifact copies.

A never-planned starter can be deleted as an ordinary local project. After planning,
archive its evidence first. Deleting local files never deletes mainnet canisters
or refunds cycles.
