# canic

Canic's first native CLI and deployable greeting application. The CLI lives in
`crates/canic-cli`, deployment logic in `crates/canic-host`, the small shared
Wire contract in `crates/canic`, and the demo in `apps/hello/canister`.

This is the fresh Canic rewrite. The predecessor is maintained separately as
[canic-legacy](https://github.com/dragginzgame/canic-legacy).

The accepted [Canic system design](docs/design/canic-system-design.md) defines
the restart: product scope, architecture, capability ownership, deferred ideas,
qualification and delivery milestones. Work is in
[M0 decisions and experiments](docs/design/m0-first-journey.md). The first M1
deployment path is implemented. Mainnet creation, installation and greeting have
passed; the same-canister mainnet upgrade remains pending.

Delivery starts with a hands-on Hello World deployment locally and on mainnet,
followed by stateful composition and bounded autonomous operation. Each increment
will include a runnable example and team-facing instructions so feedback can
shape the next step. Early mainnet experiments do not imply production lifecycle
support. This increment has one operator, one canister and no Coordinator or Store.

The first multi-canister demo will include a browser fleet view showing deployed
canisters, relationships, status and operation progress. It is required with that
increment, no later than M2, so the team can inspect and give feedback on a real
fleet. See the [visualization acceptance contract](docs/design/canic-system-design.md#demo-fleet-visualization-from-the-first-multi-canister-increment).

The runnable [M0 experiments](qualification/README.md) exercise creation,
direct-cycle funding and owner maintenance on local PocketIC instances. Run
`make m0-setup`, then `make m0-test` after preparing the documented Rust target.
`make m0-artifact-test` separately proves retained-artifact installation while
the operator is absent, including maintenance and callback-loss cases.

Build the CLI from the repository root:

```sh
make canic
./target/debug/canic --help
```

To build the demo application after preparing the Wasm target, run
`./target/debug/canic build`. Building does not deploy or spend cycles.

Rust 1.100.0 is pinned in `rust-toolchain.toml`. Install it with Rustup, then run:

```sh
make install-tools
make tools-check
make toko-tooling-check
make check
```

Prepare the Wasm target and the local simulator explicitly:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.100.0
make m0-setup
make m1-test
```

`make m1-test` builds both demo executables and exercises the actual CLI against
PocketIC: create, install, greet, recover retained requests and upgrade the same
canister. It uses synthetic cycles only. The simulator invokes the compiled CLI
as a process, keeping its older pinned dependency graph separate from Host.

## First mainnet deployment

Use the repo-pinned ICP CLI 1.7.0 for offline signing. It can use an existing
encrypted identity such as `canic-mainnet`; Canic never imports or exports its key.
Build the CLI and app, then create a plan:

```sh
make canic
export PATH="$PWD/target/debug:$PWD/.tools/ic/bin:$PATH"
canic --json build > target/hello-build.json
canic --json plan \
  --wasm "$(jq -r .wasm target/hello-build.json)" \
  --network ic \
  --controller "$(icp identity principal --identity canic-mainnet)" \
  --cycles 1000000000000 > target/hello-plan.json
cat target/hello-plan.json
```

Review the controller, network, artifact and budget. The example selects **1T
cycles from the identity's default cycles-ledger account**, including the current
100M ledger fee; the provider's creation fee comes out of the attachment. It
does not convert ICP. The current ledger fee is checked before initial dispatch;
this experimental path assumes that fee remains unchanged during submission.
Plans expire after one hour. Applying the exact digest is the spending approval:

```sh
canic --json apply "$(jq -r .operation target/hello-plan.json)" \
  --identity canic-mainnet > target/hello-receipt.json
canic call "$(jq -r .operation target/hello-plan.json)" --name team
canic status "$(jq -r .operation target/hello-plan.json)"
```

Enter the identity password in ICP CLI's terminal prompt when requested. Signing
progress goes to stderr; JSON stdout stays machine-readable. The deployment
receipt verifies the certified module hash and sole controller. The greeting
is explicitly an **uncertified query**. `status` reports retained local evidence.

Preserve `.canic/`: it contains approved artifacts, signed requests and receipts,
with owner-only file permissions. It is ignored by Git and must survive process
restarts. Protect it as authority-bearing state. Rerun **the same `apply` command**
to resume. Within the original signed submission window, Canic can resend only
the exact retained request. An expired attempt without a retained result stays
blocked; do not delete its directory or create another plan to guess recovery.
The CLI does not currently renew read-state authority after that window.

## Upgrade the demo

The second executable changes the greeting. It preserves the canister ID and
controller; this stateless demo promises no application-data migration.

```sh
canic --json build --features greeting-v2 > target/hello-build-v2.json
canic --json plan \
  --wasm "$(jq -r .wasm target/hello-build-v2.json)" \
  --network ic \
  --controller "$(icp identity principal --identity canic-mainnet)" \
  --target "$(jq -r .canister target/hello-receipt.json)" \
  --previous-module "$(jq -r .module_hash target/hello-receipt.json)" \
  > target/hello-upgrade-plan.json
cat target/hello-upgrade-plan.json
canic --json apply "$(jq -r .operation target/hello-upgrade-plan.json)" \
  --identity canic-mainnet > target/hello-upgrade-receipt.json
canic call "$(jq -r .operation target/hello-upgrade-plan.json)" --name team
```

An upgrade requires the exact prior module hash and sole controller before its
first submission. It creates no canister and debits no cycles-ledger funds;
execution is charged to the existing canister. Coordinate operations through one
retained state directory. Cross-host fencing and concurrent external changes to
the same canister are not qualified by this first path.

Local plans use `--network local --url http://127.0.0.1:PORT/ --root-key HEX
--routing-canister PRINCIPAL`, with the selected replica's DER trust root and an
existing local canister for subnet routing. The test harness supplies these;
local creation uses the replica's provisional API, whereas mainnet uses the
[cycles ledger's creation API](https://github.com/dfinity/cycles-ledger/blob/main/README.md).
The local rehearsal does not prove the live ledger/CMC integration.

## Development

Run `make help` for development commands. `make ci` is the aggregate local check;
the individual targets select narrower checks. `make install-hooks` optionally
enables the shared formatting hook.

Run `make lint` to check both product and qualification workspaces against the
adopted [Toko Tooling lint policy](TOKO-TOOLING.md#common-rust-lint-policy).
It runs Clippy with warnings denied, Rustdoc with warnings denied, formatting
and tooling agreement checks, including the deployed Wasm targets. All packages
inherit their workspace lint table; `unsafe_code` is forbidden and lint
exceptions use scoped `expect` attributes with reasons. This command runs no
application or simulator tests. These are local checks; a hosted GitHub Actions
workflow has not yet been configured.

`toko-tooling` 0.42.2 is pinned as a development dependency in both Cargo
workspaces. Its authenticated source snapshot supplies shared Make commands and
supporting scripts under `vendor/toko-tooling/`; `TOKO-TOOLING.md` remains at the
repository root. Read `AGENTS.md` and `TOKO-TOOLING.md` before making changes.
Focused M1 checks are `make m1-test`, `make m1-clippy` and `make m1-doc`.
Linux execution is qualified locally; native macOS execution remains to be checked.

`make deps-list` lists local product and qualification package versions.
`make deps-update` updates dependencies in both workspaces. Select a single
supplier with `DEPENDENCY_PACKAGE=name`, optionally with
`DEPENDENCY_PRECISE=X.Y.Z`; use `DEPENDENCY_MANIFEST=path/Cargo.toml`
to include another maintained workspace.

To update tooling, deliberately change its exact pin in both workspace catalogs,
run `make deps-update DEPENDENCY_PACKAGE=toko-tooling`, then
`make toko-tooling-preview` and `make toko-tooling-update`.
Verify with `make toko-tooling-check`.
