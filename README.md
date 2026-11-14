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
`canic fleet show --app profile --fleet mainnet --manager MANAGER --open`. Run `make growth-test` locally first.

This is the fresh Canic rewrite. The predecessor is maintained separately as
[canic-legacy](https://github.com/dragginzgame/canic-legacy).

The rewrite uses the **0.200.x** line, above the legacy crates.io release line.
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
`canic fleet show --app profile --fleet mainnet --link ENTRY:WORKER --open`. See
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
`./target/debug/canic artifact build --package canic-hello`. Building does not deploy or spend cycles.

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

## Command contexts

Help lists commands alphabetically. Only `--json` is global; other options follow
the action they belong to. Earlier top-level verbs and plural listing commands
have no aliases.

| Command | Input and scope |
| --- | --- |
| `app init APP --fleet FLEET --network ic` | Add a new app and its first fleet; create the config if missing. `--config PATH` chooses the file. Refuses existing app names. No deployment. |
| `app list` | Read all apps from `--config PATH` (default `canic.json`). |
| `artifact build --package PACKAGE` | Build an explicit Cargo package, optionally with `--manifest PATH` and `--features LIST`. App names do not select packages. |
| `artifact inspect WASM_FILE` | Read a local uncompressed Wasm file and report its SHA-256 and size. No live-canister inspection or full Wasm validation. |
| `fleet list` | Read fleet declarations; optional `--app APP` filters them. |
| `fleet init FLEET --app APP --network ic` | Add an empty fleet to an existing app in `--config PATH`; refuses existing names and deploys nothing. |
| `fleet canister add SLOT --app APP --fleet FLEET` | Declare a slot in that existing fleet; optional `--id PRINCIPAL` records an existing canister. Refuses existing slot names and deploys nothing. |
| `fleet canister list --app APP --fleet FLEET` | List local slot declarations and explicitly configured IDs, without reading journals or contacting canisters. |
| `fleet show` | Observe the selected app/fleet; `--html PATH` saves a report, `--serve` serves it, and `--watch` refreshes it. |
| `fleet plan --wasm WASM_FILE --controller PRINCIPAL` | Prepare one canister's deployment within the selected fleet, using the creation or upgrade arguments below. |
| `fleet apply OPERATION --identity IDENTITY` | Approve/resume that exact retained operation, including its funding and network effects. |
| `fleet status OPERATION` | Read that operation's retained local evidence. |
| `fleet call OPERATION --name NAME` | Query the hello demo deployed by that completed operation. This is not a general canister-method caller. |
| `fleet view REPORT_HTML` | Serve an existing local HTML snapshot. No app/fleet selection, journal access or live observation. |
| `wallet PRINCIPAL` | Read a public mainnet wallet, or derive its receive address with `--address-only`. |

`fleet show`, `plan`, `apply`, `status` and `call` accept `--app APP --fleet FLEET`
and optional `--config PATH` and `--state-dir PATH`. `--no-config` selects the
original standalone journal flow. `fleet list` needs only the config and optional
app filter; `fleet view` needs only its saved file and viewer options. Artifact,
wallet and saved-view commands reject unrelated deployment selectors.

Planning creation requires `--canister SLOT` for an unbound configured slot.
Planning an upgrade requires `--target PRINCIPAL --previous-module HASH`; the
principal must belong to the selected fleet. These choices are mutually exclusive.
The network comes from the fleet config, or from explicit `--network` and trust
inputs for standalone use. An explicit network must match the selected fleet.
Plans preserve their exact artifact, controller, cycle ceiling and operation digest.

Viewers use `--port PORT` (default 8787); `--open` is always opt-in. For a live
view, use `fleet show --serve` or `fleet show --watch`. For a saved view, use
`fleet view REPORT_HTML`. Neither opens a browser unless `--open` is supplied.

## Initialize an app and fleet

The hierarchy is **App → Fleets → Canisters**. An app is the logical application;
its fleets are separate deployments, such as `profile/mainnet` and
`profile/staging`. Each fleet has its own network, canister identities and journal.

List the local declarations without contacting the network or opening a journal:

```sh
canic app list
canic fleet list
canic fleet list --app profile
canic fleet list --json
```

Listings use `canic.json` in the current directory, or `--config PATH`. Counts are
declared fleets/canister slots, not proof of deployment or health. Use
`canic fleet show --app profile --fleet mainnet` for current deployment observations.

In a project without `canic.json`, initialize an app and its first fleet:

```sh
canic app init hello --fleet mainnet --network ic
canic fleet show --app hello --fleet mainnet
```

Initialization creates **only local declarations**. It creates `canic.json` if
missing or adds the new app to an existing config, preserving other apps. It
refuses an existing app name, does not access an identity, and performs no network
calls or funding. Existing-file updates use the sidecar lock described below.
Canister layout is declared later in the selected fleet:

```json
{
  "version": 2,
  "apps": {
    "hello": {
      "fleets": {
        "mainnet": {
          "network": { "kind": "Mainnet" },
          "canisters": {}
        }
      }
    }
  }
}
```

Add another fleet to an existing app without editing JSON:

```sh
canic fleet init staging --app hello --network ic
canic fleet list --app hello
canic fleet show --app hello --fleet staging
```

This adds an empty fleet on the explicitly chosen network, preserving all other
declarations. It refuses unknown apps and existing fleet names. No canisters are
created and no journal is opened; its future default journal remains separate
from the first fleet. A fleet name such as `staging` does not imply a network.
Local fleets require the same explicit URL, root key and routing canister inputs
as `app init`.

Config updates use a persistent, Git-ignored `CONFIG.canic-lock` sidecar and an
atomic file replacement. The lock coordinates Canic commands, not external text
editors; avoid simultaneous manual edits. Do not delete the sidecar while an edit
is running. A filesystem sync failure can occur after the complete update becomes
visible; inspect `fleet list` before retrying.

Add another app with its own first fleet through the same command:

```sh
canic app init another-app --fleet mainnet --network ic
canic app list
canic fleet list --app another-app
```

Choose an unused app name. Repeating `app init` cannot reset an app or add fleets
to it; use `fleet init` for another fleet within an existing app. A malformed
existing config is refused, never replaced with an empty one. Concurrent creation
of the first config can refuse one initializer; inspect `app list` before retrying.

The CLI discovers `canic.json` in the current directory, or accepts
`--config PATH`. Select `--app NAME --fleet NAME`; either may be omitted only
when its choice is unique. Ambiguous selection fails with the available names.
The default journal is `.canic/apps/APP/fleets/FLEET/` beside the config. A fleet's
optional `journal` path selects an existing journal relative to the config (or an
absolute path); `--state-dir PATH` overrides it. Configured observations and
operation access still require membership in the selected app/fleet.

Names contain 1–64 ASCII letters, digits, hyphens or underscores. Configuration
admits at most 16 apps, 16 fleets per app, 32 canisters per fleet and 128 canisters
total, within a 32 KiB file. Local initialization requires `--network local --url
URL --root-key HEX --routing-canister PRINCIPAL`; it does not start a simulator.

Duplicate app, fleet or canister keys are errors. A canister ID under the same
exact network binding may appear in only one fleet in this config. An external
controller or shared service is not implicitly a fleet member. Selection provides
local mistake prevention; controller/signature checks still supply authority.
Editing a name is not a rename or transfer operation, and never rewrites existing
approved plans or moves their journals.

To prepare the first deployment in the app/fleet created above, declare an empty
slot. Both slot commands require `--app` and `--fleet`; they accept `--config PATH`
but no network override or journal options. The fleet already owns its network.

```sh
canic fleet canister add backend --app hello --fleet mainnet
canic fleet canister list --app hello --fleet mainnet
```

Adding an empty slot does not create a canister or bind a principal. Duplicate names are
refused even for empty slots; existing identities and retained operations cannot
be reset by repeating the command. The list reads only declarations: an unset ID
can still have a completed deployment in the journal. Use `fleet show` for that
evidence and live observations.

To include an already deployed canister, set `canic_existing_id` to its principal
and choose an unused slot name in the fleet for its actual network:

```sh
canic fleet canister add existing --app hello --fleet mainnet --id "$canic_existing_id"
canic fleet canister list --app hello --fleet mainnet
```

This records local membership only. It does not contact the canister, verify
existence or control, change controllers, fund it or modify its code. Invalid
principals and duplicate membership on the same network are refused before the
config is replaced. Existing slots cannot be rebound with `add`, including empty
slots. An ID blocks creation in that slot; upgrading still requires a separate
target/hash plan and approval. No journal is created or changed by this command.

Set `canic_owner` to the controller principal and prepare the Wasm target, then
build, inspect and review the plan (requires `jq`):

```sh
canic artifact build --json --package canic-hello > target/hello-build.json
canic artifact inspect "$(jq -r .wasm target/hello-build.json)"
canic fleet plan --app hello --fleet mainnet --json --canister backend \
  --wasm "$(jq -r .wasm target/hello-build.json)" \
  --controller "$canic_owner" > target/hello-plan.json
canic fleet status --app hello --fleet mainnet "$(jq -r .operation target/hello-plan.json)"
```

Review artifact, controller, network and cycle ceiling. Applying the reviewed
digest authorizes deployment and its funding:

```sh
canic fleet apply --app hello --fleet mainnet "$(jq -r .operation target/hello-plan.json)" --identity canic-mainnet
canic fleet show --app hello --fleet mainnet
```

Named plans retain app/fleet/canister names under the approval digest. Repeating
an identical plan returns the original operation and expiry. Conflicting inputs
cannot replace an occupied slot or renew expired/uncertain authority. Inspect
and resume the original operation; do not delete its journal to retry. Explicit
`--target`/`--previous-module` upgrade plans remain available without slot flags.
Configured upgrade targets must already be declared in that fleet or identified
by its retained named operations; unrelated targets are rejected before a plan
is written.

This checkout's [canic.json](canic.json) declares two applications:

- `hello-world/mainnet`: the greeting canister.
- `profile/mainnet`: entry, shared worker, manager and managed worker.

Both select the original `.canic/` journal in place. Existing slots use
`{"id": "CANISTER_PRINCIPAL"}`, which blocks new creation in that slot.
Names are local declarations; they do not grant ownership or rewrite signed
plans. `profile_manager: true` selects the manager's read-only status query.
Certified code/controller observations and uncertified manager reports remain
distinct. A clone without the private journal can observe the public canisters
but has no retained receipts.

```sh
canic fleet show --app hello-world --fleet mainnet
canic fleet show --app profile --fleet mainnet
canic fleet status --app profile --fleet mainnet 8b54397d04474aa916e93bcb89f46b8db7e96fccd05391d4eeb4224930f601d6
```

The status example needs the original private journal. Initialization does not
automatically adopt canisters. `canic fleet show --no-config` inspects all original
journal operations, including unassigned records. Observation retains the selected
configuration for its run; restart a watcher after edits. Never place seed words,
keys or signed attempts in the shareable config.

## Check the mainnet wallet

```sh
canic_owner=o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe
canic wallet "$canic_owner"
canic wallet "$canic_owner" --address-only
canic fleet show --app profile --fleet mainnet --wallet "$canic_owner" --manager rk6n2-wyaaa-aaaad-qmoua-cai --open
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
This older explicit-plan recipe uses `--no-config` and the original journal.
For a named deployment use the initialization guide above. Build the CLI and app, then create a plan:

```sh
make canic
export PATH="$PWD/target/debug:$PWD/.tools/ic/bin:$PATH"
canic artifact build --package canic-hello --json > target/hello-build.json
canic fleet plan --no-config --json \
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
canic fleet apply --no-config --json "$(jq -r .operation target/hello-plan.json)" \
  --identity canic-mainnet > target/hello-receipt.json
canic fleet call --no-config "$(jq -r .operation target/hello-plan.json)" --name team
canic fleet status --no-config "$(jq -r .operation target/hello-plan.json)"
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
canic fleet show --app profile --fleet mainnet
canic fleet show --app profile --fleet mainnet --json
canic fleet show --app profile --fleet mainnet --open
canic fleet show --app profile --fleet mainnet --watch
canic fleet show --app profile --fleet mainnet --serve
canic fleet show --app profile --fleet mainnet --html target/fleet.html
canic fleet view target/fleet.html
```

`canic fleet show --app profile --fleet mainnet` prints a terminal summary of known canisters, controller roles,
verified code/controller observations, and operations without a terminal receipt. Explicit
`--manager`, `--wallet` and `--link` selections add their reports to the summary.
Unknown observations and partial inventory stay visible; manager and wallet
queries are labelled uncertified. No browser opens for this command.
Use `canic fleet show --app profile --fleet mainnet --json` for scripts and complete structured evidence; plain
`canic fleet show --app profile --fleet mainnet` no longer prints JSON.

Controller badges are **magenta**, selected managers **cyan**, and declared entries
or reported/declared workers **blue**. Verified metadata is green; unknown metadata
is yellow. Role names remain readable without colour. A **CONTROLLER CANISTER**
controls another known canister on the same network, based on verified metadata.
An **EXTERNAL CONTROLLER** is a principal outside that network's known canister
inventory; it may be an identity or an unlisted canister. The current demo uses
an operator identity and a manager, with no separately deployed root canister.
Root roles are not inferred from an incomplete inventory.

Colours default to interactive terminals; redirected output stays plain. Auto mode
honours nonempty `NO_COLOR` and `TERM=dumb`. Override with `--color always` or
`--color never`; JSON stays colour-free. Network numbers distinguish separate
environments within the current snapshot. Use `--details` to see full module
hashes and diagnostic network identities:

```sh
canic fleet show --app profile --fleet mainnet --manager rk6n2-wyaaa-aaaad-qmoua-cai --color always
canic fleet show --app profile --fleet mainnet --details --color never
```

`--open` creates a fresh report under `target/fleet/`, serves it at
**http://localhost:8787/** and launches your default browser using `xdg-open` on
Linux or `open` on macOS. `--serve` prints the URL without launching a browser.
Keep the command running while reviewing; Ctrl-C stops the viewer. Use `--port`
to choose another port (`--port 0` chooses a free one). Browser launch failure
leaves the localhost viewer available at the printed URL.

`canic fleet view PATH` serves a previously saved snapshot without contacting canisters.
It serves only that report, not the containing directory or deployment journal.
Use `--html PATH --open` to choose where a fresh report is also retained.
Without `--watch`, these are saved observations: browser refresh does not make
new network reads.

`canic fleet show --app profile --fleet mainnet --watch` keeps the localhost page up to date and prints its URL
without opening a browser. Add `--open` to launch the normal browser. The host waits
30 seconds after each completed check before beginning the next one; the browser
reloads every 30 seconds. Observation rounds never overlap, and browser requests
do not trigger extra network checks. The current page stays available while the
next round runs. Ctrl-C stops both the viewer and observation loop.

Use the existing mainnet demo and wallet with:

```sh
canic fleet show --app profile --fleet mainnet --watch --manager rk6n2-wyaaa-aaaad-qmoua-cai \
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
never overwritten by the watcher. `canic fleet view PATH` always remains a saved view.

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
canic artifact build --package canic-hello --json --features greeting-v2 > target/hello-build-v2.json
canic fleet plan --no-config --json \
  --wasm "$(jq -r .wasm target/hello-build-v2.json)" \
  --network ic \
  --controller "$(icp identity principal --identity canic-mainnet)" \
  --target "$(jq -r .canister target/hello-receipt.json)" \
  --previous-module "$(jq -r .module_hash target/hello-receipt.json)" \
  > target/hello-upgrade-plan.json
cat target/hello-upgrade-plan.json
canic fleet apply --no-config --json "$(jq -r .operation target/hello-upgrade-plan.json)" \
  --identity canic-mainnet > target/hello-upgrade-receipt.json
canic fleet call --no-config "$(jq -r .operation target/hello-upgrade-plan.json)" --name team
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
