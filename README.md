# canic

Canic's first native CLI and deployable greeting application. The CLI lives in
`crates/canic-cli`, deployment logic in `crates/canic-host`, the small shared
Wire contract in `crates/canic`, and the demo in `apps/hello/canister`.

The [persistent profile demo](docs/demos/profile.md) adds an application-owned
worker, discovery entry, Wire contract and native client under `apps/profile/`.
Start with `make profile` and `make profile-test`; the same Canic CLI deploys both
canisters. The entry resolves the worker while private calls keep the user's identity.
The [bounded growth demo](docs/demos/growth.md) adds one autonomous manager:
approve a worker once, let the host leave, then inspect the retained outcome with
`canic fleet --manager MANAGER --open`. Run `make growth-test` locally first.

This is the fresh Canic rewrite. The predecessor is maintained separately as
[canic-legacy](https://github.com/dragginzgame/canic-legacy).

The rewrite starts at **0.200.0**, above the legacy crates.io release line.
Development is shared through GitHub. We will not publish this rewrite to
crates.io until it is mature; all workspace packages retain `publish = false`.
Build from this repository in the meantime.

The accepted [Canic system design](docs/design/canic-system-design.md) defines
the restart: product scope, architecture, capability ownership, deferred ideas,
qualification and delivery milestones. Work is in
[M0 decisions and experiments](docs/design/m0-first-journey.md). The first M1
deployment path is implemented. Mainnet creation, installation and greeting have
passed, followed by a profile entry/worker deployment and a worker upgrade that
preserved its private profile. A separate manager has now created and installed
one usable worker autonomously on mainnet. The manager has also survived a
mainnet upgrade with its full journal and spent
approval preserved. Use the existing
[profile pair](docs/demos/profile.md#current-mainnet-demo) or
[autonomous worker](docs/demos/growth.md#current-mainnet-demo).

Delivery starts with a hands-on Hello World deployment locally and on mainnet,
followed by stateful composition and bounded autonomous operation. Each increment
will include a runnable example and team-facing instructions so feedback can
shape the next step. Early mainnet experiments do not imply production lifecycle
support. Each deployment plan has one operator and one application canister;
the CLI can inspect multiple retained deployments without a Coordinator or Store.

The multi-canister profile demo includes a browser fleet view showing deployed
canisters, declared relationships and operation evidence, so the team can inspect
and give feedback on the pair. The initial [fleet snapshot](#inspect-the-fleet)
is available now; canister cycle-balance reporting remains future work. An optional
wallet panel shows public ICP and cycles-ledger balances. Explicitly selected
managers report their created worker for independent metadata observation. The profile demo supplies
an explicit direct readiness check and a declared entry-to-worker arrow through
`canic fleet --link ENTRY:WORKER --open`. See
the [visualization acceptance contract](docs/design/canic-system-design.md#demo-fleet-visualization-from-the-first-multi-canister-increment).

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

## Check the mainnet wallet

```sh
canic_owner=o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe
canic wallet "$canic_owner"
canic wallet "$canic_owner" --address-only
canic fleet --wallet "$canic_owner" --manager rk6n2-wyaaa-aaaad-qmoua-cai --open
```

`wallet` takes a public principal, not an identity name. It observes the default
(zero) subaccount on IC mainnet, without accessing keys or requesting a password.
`--address-only` derives the ICP receive address offline. The Canic wallet's address is:

```text
b38bdabcc30b47b6dda966a2a7c49e4540514ce6a36c78aa42424492a98c59d3
```

Balances use anonymous, explicitly uncertified
[`icrc1_balance_of` queries](https://docs.internetcomputer.org/guides/digital-assets/ledgers/)
to the fixed mainnet ICP and cycles ledgers. The cycles-ledger account excludes
cycles already held by canisters. Amounts retain exact integer precision; JSON
`amount` fields are decimal strings in e8s or cycles. A failed ledger observation
has null amounts and its own error, while the other result and receive address
remain available. Read those fields before relying on a partial report.

The two observations run concurrently with a five-second limit each, independent
timestamps, no automatic retries and no conversion or transfer. This is a saved
point-in-time observation; rerun the command to check an incoming deposit.
Nondefault subaccounts and local-replica wallets are not supported by this slice.

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

## Inspect the fleet

```sh
canic --json fleet
canic fleet --open
canic fleet --watch
canic fleet --serve
canic fleet --html target/fleet.html
canic view target/fleet.html
```

`--open` creates a fresh report under `target/fleet/`, serves it at
**http://localhost:8787/** and launches your default browser using `xdg-open` on
Linux or `open` on macOS. `--serve` prints the URL without launching a browser.
Keep the command running while reviewing; Ctrl-C stops the viewer. Use `--port`
to choose another port (`--port 0` chooses a free one). Browser launch failure
leaves the localhost viewer available at the printed URL.

`canic view PATH` serves a previously saved snapshot without contacting canisters.
It serves only that report, not the containing directory or deployment journal.
Use `--html PATH --open` to choose where a fresh report is also retained.
Without `--watch`, these are saved observations: browser refresh does not make
new network reads.

`canic fleet --watch` keeps the localhost page up to date and prints its URL
without opening a browser. Add `--open` to launch the normal browser. The host waits
30 seconds after each completed check before beginning the next one; the browser
reloads every 30 seconds. Observation rounds never overlap, and browser requests
do not trigger extra network checks. The current page stays available while the
next round runs. Ctrl-C stops both the viewer and observation loop.

Use the existing mainnet demo and wallet with:

```sh
canic fleet --watch --manager rk6n2-wyaaa-aaaad-qmoua-cai \
  --wallet o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe \
  --link bisqm-biaaa-aaaas-qhfeq-cai:4pkhu-oaaaa-aaaai-ra2ta-cai
```

Each successful round replaces the in-memory page. Unavailable individual
observations are marked unknown. If a complete refresh fails, the last completed
snapshot stays visible with a **Refresh failed** warning and its original
timestamps; a later successful refresh clears that warning. At most one current
report and its last completed source are retained in memory, with a 4 MiB page
limit. There is no disk history or automatic deployment/funding action.
If `--html PATH` is supplied, it saves only the initial static snapshot and is
never overwritten by the watcher. `canic view PATH` always remains a saved view.

The self-contained report shows known
canisters and certified controller relationships, current module hashes, original
deployment approvals, historical receipts and unresolved operations. It makes
anonymous metadata reads without signing or resuming any deployment. Different
local network trust roots remain separate even when canister principals match.

Each round observes the selected `--state-dir`, not a network-wide inventory.
Use `--watch` for repeated observations, or rerun `--open` with a new output
filename for another saved snapshot. Existing reports are preserved.
Each target gets at most five seconds within a
thirty-second network budget, and the inventory accepts at most 128 operations.
An explicit `--wallet PRINCIPAL` adds up to five seconds for its separate mainnet
balance observations, before fleet collection; each balance has its own timestamp.
Unreadable records and unavailable observations are explicit partial results.
Canister cycle balances and running/stopped status remain unobserved. Optional
manager reports are labelled uncertified and declared routes remain separate from
certified module/controller metadata. Reports contain no keys, signed requests
or artifact bytes.

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

`toko-tooling` 0.43.0 is pinned as a development dependency in both Cargo
workspaces. Its authenticated source snapshot supplies shared Make commands and
supporting scripts under `vendor/toko-tooling/`; `TOKO-TOOLING.md` remains at the
repository root. Read `AGENTS.md` and `TOKO-TOOLING.md` before making changes.
Use `toko-cloc` for line counts; `make cloc` was retired in 0.43.0.
The qualification workspace uses IC Testkit 0.104.0 for PocketIC lifecycle and
raw calls, with application encoding supplied directly by IC Wire.
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
