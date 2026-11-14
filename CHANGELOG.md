# Changelog

## [0.200.0] - Unreleased

### ▶️ Try it

List declarations from this checkout's `canic.json`, without contacting the network:

```sh
canic app list
canic fleet list
canic fleet list --app profile
```

Use `--json` for structured listings or `--config PATH` for another config.
Listed canister counts are declared slots, not a live deployment/health check.
Terminal tables use the shared IC Host renderer (`ic-host-text::table`) for
escaping, aligned header separators and column alignment, with numeric counts
right-aligned.
Help lists commands alphabetically at every level. Commands are grouped under
`app`, `artifact` and `fleet`; `wallet` takes a public principal. Scope options
follow the action: `canic fleet show --app profile --fleet mainnet`. There are no
aliases for the earlier top-level verbs, `apps`, or `fleets`.

After preparing the Wasm target and running `make canic`, build an explicit Cargo
package and inspect the resulting local file (requires `jq`; no deployment):

```sh
canic artifact build --package canic-hello --json > target/hello-build.json
canic artifact inspect "$(jq -r .wasm target/hello-build.json)"
canic fleet plan --help
```

`fleet plan` prepares one canister creation or upgrade. `--canister SLOT` selects
an unbound configured slot; `--target PRINCIPAL --previous-module HASH` selects
an upgrade. They cannot be combined. `fleet view REPORT_HTML` serves an existing
saved report; `fleet show --serve` observes and serves the selected fleet.

The hierarchy is **App → Fleets → Canisters**. The version 2 `canic.json` declares
`hello-world/mainnet` (one canister) and `profile/mainnet` (four canisters).
The original `.canic/` journal stays in place; no canisters are redeployed or funded.
After `make canic`, select the application and its fleet explicitly:

```sh
canic fleet show --app hello-world --fleet mainnet
canic fleet show --app profile --fleet mainnet
canic fleet show --app profile --fleet mainnet --json
```

The profile manager and its worker are included automatically. Inspect the
original manager upgrade receipt using this checkout's retained private journal:

```sh
canic fleet status --app profile --fleet mainnet 8b54397d04474aa916e93bcb89f46b8db7e96fccd05391d4eeb4224930f601d6
```

Configuration rejects duplicate JSON names and a canister declared in two fleets
under the same network binding. Configured upgrade planning also rejects targets
outside the selected fleet before retaining a new operation.

In a project without `canic.json`, initialize an app and its first fleet locally:

```sh
canic app init hello --fleet mainnet --network ic
canic fleet show --app hello --fleet mainnet
canic app init --help
```

Add other fleets under the app's `fleets` object and canister slots under each
fleet's `canisters` object. New journals default to
`.canic/apps/APP/fleets/FLEET/`; a fleet's explicit `journal` path can retain an
existing journal in place. Names may be omitted only when selection is unique.
See the [named planning flow](README.md#initialize-an-app-and-fleet).

Controller roles are magenta, managers cyan, and worker roles blue in an
interactive terminal. Inspect full hashes or choose plain output with:

```sh
canic fleet show --app profile --fleet mainnet --details --color never
```

Watch the profile fleet and its declared entry-to-worker route. This starts a
localhost viewer and prints its URL; it does not open a browser. Ctrl-C stops it:

```sh
canic fleet show --app profile --fleet mainnet --watch \
  --link bisqm-biaaa-aaaas-qhfeq-cai:4pkhu-oaaaa-aaaai-ra2ta-cai
```

Save a snapshot to a new filename, then serve it without reconnecting to canisters:

```sh
canic fleet show --app profile --fleet mainnet --html target/profile-fleet.html
canic fleet view target/profile-fleet.html
```

### ➕ Added

- Group commands by their actual context and reject unrelated scope arguments.
  Artifact building requires an explicit package; saved report viewing takes a
  file and accepts no app/fleet/journal selectors. Fleet observation is `fleet show`.
- Add `canic app list` and `canic fleet list` for local configuration listings, including
  app filtering for fleets and structured JSON output. Neither reads a journal
  nor contacts canisters; `canic fleet show` remains the selected deployment observation.
- Bind existing canister identities to configured fleet slots without rewriting
  approved plans or receipts. Refuse new creation in bound slots, preserve explicit
  journal selection, and retain configured profile-manager observations across snapshots.
- Add `canic app init` for a bounded local app/fleet/canister configuration and explicit
  network. Named plans retain names under their approval digest, reuse an existing
  operation on repetition, and show retained canister IDs in terminal/HTML/JSON.
  Initialization performs no network effects and refuses to replace existing config.
- Add `canic fleet show --watch` for automatic read-only observations and a refreshing
  localhost page. Keep one observation round in flight, preserve the last snapshot
  with a visible warning on failure, and stop all monitoring with Ctrl-C.
  Print the localhost URL by default; launch a browser only with explicit `--open`.
- Add read-only `canic wallet PRINCIPAL`, offline receive-address derivation and
  an optional `fleet --wallet PRINCIPAL` panel. Keep exact ICP/cycles amounts,
  explicit uncertified observations and per-ledger unknown results.
- Add a one-slot profile manager with retained Wasm, IC Timers scheduling and
  a 1T worker-creation attachment. Add native growth approval/observation commands
  and `canic fleet show --manager` for the manager and its reported worker.
- Serve fleet reports at localhost URLs: `canic fleet show --open` starts the viewer
  and browser, `--serve` prints the link, and `canic fleet view PATH` serves a saved
  snapshot without reconnecting to canisters. Ctrl-C stops the viewer.
- Add a persistent profile discovery entry with controller-only immutable binding,
  certified client resolution and bounded direct worker checks. Profile calls keep
  the original user identity. Show the pair with `canic fleet show --link ENTRY:WORKER`.
- Add a Canic-owned persistent profile demo with a native client, owner-only
  reads/writes, immutable user mappings, bounded capacity and idempotent revisions.
  Deploy it through the existing CLI and qualify preservation across two executables.
- Add `canic fleet show` and optional HTML snapshots of locally known deployments,
  with certified module/controller observations, retained operation evidence,
  distinct network trust bindings and explicit partial/unavailable results.
  `canic fleet show --open` creates a fresh snapshot and launches the default browser.

### 🔧 Changed

- Correct the hierarchy to App → Fleets → Canisters. Scope selection, journals and
  observations by app and fleet; refuse ambiguous selection and wrong-fleet operation
  access. Preserve original plan digests, receipts and unresolved-operation evidence.

- Highlight observed controller roles and selected/reported application roles with
  ANSI badges. Keep redirected output and JSON plain by default, honour `NO_COLOR`,
  and expose full module/network fingerprints through `fleet --details`.
- Make plain `canic fleet show` a readable terminal summary, including optional manager
  and wallet reports, unknown observations and operations without receipts.
  Scripts must use `--json` for the unchanged structured output. No browser opens.
- Start the rewrite's version line at 0.200.0, distinct from legacy Canic.
  Share development through GitHub and defer crates.io publication until mature;
  keep publication disabled for all packages.
- Adopt Toko Tooling 0.43.0 through the authenticated installer in both workspaces,
  retaining the existing executable selection and lint policy. Use `toko-cloc`
  instead of the retired `make cloc` command.
- Adopt IC Testkit 0.104.0 in qualification, retaining raw PocketIC calls and
  directly owned Wire encoding without adding the optional typed-call adapter.

### 🧪 Testing

- Cover multiple fleets per app, shared-journal isolation, unchanged old digests,
  duplicate declarations and wrong-fleet upgrade refusal. Exercise a valid
  configured upgrade through the installed CLI and retained recovery path.
- Verify mainnet manager maintenance preserves its complete journal, spent
  approval and worker profile. Recover an interrupted post-install receipt from
  the original retained reply with no signer or second installation.
- Add `make growth-test`: operator-absent worker creation, authenticated use,
  five interruption boundaries, manager upgrade preservation, network binding,
  expiry, reserve refusal and no duplicate paid effects.
- Verify the normal autonomous growth path on mainnet: one manager-created worker,
  exact module/controller bindings, retained effect evidence and authenticated
  profile use. Extend the mainnet fleet snapshot to all five known canisters.
- Qualify mainnet entry/worker deployment, original-caller profile operations,
  direct readiness checks and a worker upgrade preserving the complete profile.
- Add `make profile-test` for real deployment, authorization, limits, failed-upgrade
  rollback and retained application data; measure Wasm and stable-memory footprint.
- Extend the CLI journey to two deployed canisters and fleet observation across
  uncertain upgrades, historical receipts and an unavailable local provider.

## [0.1.1] - 2026-11-13

### ➕ Added

- Add `make canic` to build the CLI and show its executable path.
- Add the first `canic` CLI and greeting demo: build, inspect, plan, apply/resume,
  call and status, with a second greeting executable for same-canister upgrades.
  Bind approval to network, controller, artifact and funding inputs; retain exact
  signed requests and results before advancing deployment. Use ICP CLI for offline
  signing and preserve its encrypted identity storage.

### 🔧 Changed

- Adopt Toko Tooling 0.42.2 under `vendor/toko-tooling/`, with `make deps-list`
  and `make deps-update` covering product and qualification workspaces.
  Release review on 2026-11-13 found 0.42.6 available; defer this optional
  tooling refresh to the next dependency-maintenance batch. The selected direct
  IC Host, Platform, Runtime, Wire and Testkit suppliers match current stable
  registry releases in the locked graphs.

### 📚 Documentation

- Admit IC Relay as an optional communication module alongside direct calls,
  with explicit lifecycle/recovery boundaries and a measured multi-canister pilot.
- Require a runnable demo fleet visualization from the first multi-canister
  increment, no later than M2, with actual topology, status and recovery evidence.
- Revise the delivery design around runnable team increments, with local and
  mainnet Hello World deployment in M1 and feedback throughout development.
  Keep later stateful, recovery and production proofs explicit.
- Accept the architectural basis and scope M0 around first-effect commit
  boundaries and a narrow consumer invariant. Clarify candidate topology,
  selected service arrangements, conditional frontend export and preservation
  guarantees; gate later integrations on admitted consumer needs.

### 🧪 Testing

- Add `make lint` for complete product/qualification lint, Rustdoc, formatting
  and tooling checks. Cover the M1 qualification documentation and run each
  selected lint/documentation check once in the aggregate.
- Add a bounded artifact-custody experiment: an executor retains an approved
  worker module across upgrades and installs it without another operator command.
  Cover lost upload/install callbacks and occupied-target refusal; report physical
  storage separately. Run it with `make m0-artifact-test`.
- Add a local owner-maintenance rehearsal across two executables: preserve
  creation/funding evidence and uncertain exposure, reject unauthorized upgrades,
  and recover from a trapping upgrade without repeating paid effects.
  Run it separately with `make m0-maintenance-test`.
- Add a runnable local M0 creation experiment with explicit PocketIC preparation,
  dispatch/callback trap injection and no-duplicate replay checks. Keep it in a
  separate qualification workspace; verify that bounded evidence remains readable
  when the creation result is uncertain.
- Extend M0 to direct-cycle funding: test successful deposits and rejected-target
  refunds across interruption boundaries, bounded admission and replay without
  duplicate funding. Retain uncertain exposure when callback evidence is lost.

## [0.1.0]

### ➕ Added

- Initialize the blank `canic` Rust library workspace with pinned development
  tooling, shared engineering rules, and formatting, lint and documentation commands.

### 📚 Documentation

- Rename the rewrite to `canic`, including its library package, workspace paths
  and design document; distinguish the predecessor as `canic-legacy`.

- Define the proposed Canic system architecture, complete capability disposition,
  paused-design direction and implementation milestones, with a source inventory.
  Treat predecessor behavior as research input and derive the starting architecture
  from a complete application journey, with explicit removal decisions and
  workload-driven topology, configuration and admission.
  Bring consumer composition and persistence proofs forward, specify approval,
  effect custody and recovery contracts, and distinguish historical coverage
  from first-consumer implementation scope.
