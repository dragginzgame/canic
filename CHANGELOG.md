# Changelog

## [0.201.0] - 2026-11-14

### ▶️ Try it

From this Canic checkout, select the existing profile app. Its sole `mainnet`
fleet is inferred; these commands only observe deployments. Watch serves
localhost without opening a browser:

```sh
canic fleet show --app profile
canic fleet show --app profile --watch
```

Bare `fleet show` works only when the selected config has one app and one fleet.
This checkout contains `hello-world` and `profile`; generated projects have
their own config, selected by working directory or `--config`.

Once a child plan names its deployed Root, follow progress without signing or
resubmitting work. The live view serves localhost and does not open a browser:

```sh
canic fleet show --config starter/canic.json
canic fleet show --config starter/canic.json --watch
```

Root reports are uncertified observations. A `Ready` report does not create a
completion receipt; use the original `fleet apply` operation to retain one.

From this checkout, generate a new project in an unused directory. This writes
source and configuration only; the `mainnet` name selects a declaration, not a
deployment. The generated README has the build steps:

```sh
canic app init starter --fleet mainnet --network ic --source ./starter --canic-source .
canic app canister list --config starter/canic.json
canic fleet canister list --config starter/canic.json
```

After building the generated artifacts and deploying its Root, copy the reviewed
Root hash from `artifact inspect` and use your operator principal. Planning is
local; apply admits one 1T-cycle child creation to Root and changes mainnet when
the selected fleet is mainnet. Replace the uppercase placeholders with the
returned values; see [the complete journey](docs/demos/root.md):

```sh
canic artifact inspect starter/artifacts/root.wasm
canic fleet plan --config starter/canic.json --canister app --controller OPERATOR_PRINCIPAL --root-module ROOT_SHA256
canic fleet status CHILD_OPERATION --config starter/canic.json
canic fleet apply CHILD_OPERATION --config starter/canic.json --identity canic-mainnet
```

Build the experimental Root and its greeting application. These commands only
build local Wasm; they do not create canisters. With the prepared PocketIC server
(`make m0-setup`), `make root-test` exercises their installed controller graph
and Root maintenance using synthetic cycles, without opening windows:

```sh
canic artifact build --package canic-root
canic artifact build --package canic-hello
```

With one fleet configured for `profile`, omit its fleet selector. These commands
read local configuration only; if you add another fleet, supply `--fleet NAME`:

```sh
canic app canister list --app profile
canic fleet canister list --app profile
```

In a config containing just one app, `--app` can also be omitted. No current
app or fleet is saved between commands.

Inspect this checkout's configured profile fleet. These commands perform read-only
network observations; they do not fund, deploy or open a browser:

```sh
canic fleet show --app profile --fleet mainnet
canic fleet show --app profile --fleet mainnet --details
canic fleet show --app profile --fleet mainnet --json
```

Inspect the profile demo's explicit roles and parent/child declarations, without
network access:

```sh
canic fleet canister list --app profile --fleet mainnet
```

Inspect app definitions, then create a new `profile/staging` fleet inheriting
them. Choose an unused fleet name. These commands do not deploy anything:

```sh
canic app canister list --app profile
canic fleet init staging --app profile --network ic
canic fleet canister list --app profile --fleet staging
```

For a new app, declare its topology once for all its fleets. This is configuration
only: a `root` role does not install a Root or grant controller authority.

```sh
canic app init starter --fleet mainnet --network ic
canic app canister add root --app starter --role root
canic app canister add backend --app starter --role app --parent root
canic app canister list --app starter
```

With this checkout's original private journal, inspect the retained manager
maintenance operation. Both forms read local evidence only, with no network
calls or browser launches:

```sh
operation=8b54397d04474aa916e93bcb89f46b8db7e96fccd05391d4eeb4224930f601d6
canic fleet status --app profile --fleet mainnet "$operation"
canic fleet status --app profile --fleet mainnet "$operation" --json
```

### 🔄 Changed

- Clarify ambiguous app/fleet selection errors and distinguish unknown explicit
  names. Lead checkout examples with `--app profile`; infer only unique choices,
  without changing config or saving a default app.
- Automatically observe Roots named by the selected fleet's retained child plans
  in terminal and localhost views. Keep exact network bindings, reject reports
  for different approvals, and distinguish Root/child labels from profile
  managers/workers. Public reports never establish retained child identity or
  write completion receipts; watching consumes no signed observation attempts.
- Connect generated children to Root-owned `fleet plan` and `fleet apply`.
  Pin the parent identity/code, signer, child bytes, schedule and 1T attachment;
  retain exact staging attempts and replicated child evidence across response
  loss. Apply reports pending work separately from verified completion. Preserve
  direct-plan digests and project Root as the child's controller in receipts
  and fleet views. A declared child cannot silently use direct host creation.
- Extend `app init` with optional standalone source generation, a supplied Root
  declaration and an editable greeting app. Publish the complete directory
  without replacing existing paths; inherit the selected checkout's compiler,
  supplier and lint policies. Add `artifact build --output` for configured Wasm
  inputs. The installed Root fixture now builds the generated app through the
  CLI. Whole-fleet convergence remains upcoming.
- Add an experimental Root for one approved greeting application. Extract the
  profile manager's creation, artifact custody and recovery engine into
  `canic-management`, with a separate runtime-free management protocol. Both
  canisters own their lifecycle/endpoints and choose application readiness checks.
  Preserve the manager's wire/state formats. Child upgrades and whole-fleet
  convergence remain separate work. See
  [Root qualification](docs/demos/root.md).
- Infer a sole app/fleet consistently for definition and binding commands and
  `fleet init`. Require explicit selectors when ambiguous; reject unknown names
  before editing config. App commands do not require a unique fleet. Apply the
  same app-name validation to `fleet list --app`; qualify selection refusals
  before plan, apply, status, call and show proceed.
- Give app/fleet canister listings consistent headings and column names, blank
  lines around their tables, and no trailing help text.
- Move reusable role, parent and optional Wasm inputs into `apps.APP.canisters`;
  every fleet resolves them with its own network, journal and existing-ID bindings.
  Add `app canister add/list` and replace `fleet canister add` with
  `fleet canister bind SLOT --id PRINCIPAL`. No aliases or fleet overrides.
  Named creation can consume an app-defined Wasm path relative to the config;
  exact artifact retention and existing deployment-plan digests are unchanged.
  This is a breaking config change in 0.201.0. The old versioned config is refused;
  this checkout is migrated, and the README describes external config edits.
  General Root management and whole-fleet convergence remain upcoming.
- Record the accepted next product journeys: one reusable app definition across
  fleets, a generated Root-controlled Rust starter, whole-fleet convergence,
  cascading discovery and subtree operations. Evaluate legacy cascade source,
  regression tests and history; whole-fleet convergence, cascading discovery and
  subtree operations are not implemented yet.
  Exclude ICP conversion from the design in favor of manually supplied cycles;
  defer application frontend integration and keep operator handover simple.
- Simplify `fleet show` to one named row per canister, using IC Host tables.
  Controller references use fleet names or a short external reference, with each
  external principal listed once, separated from the table by a blank line.
  Remove badge stacks and routine help text;
  limit colour to controller/manager rows and unknown metadata. `--details`
  retains hashes, timestamps and manager attempt counts. Conflicts, failed
  observations and unresolved operations remain visible by default.
- Restore explicit application roles and immediate parent/child topology,
  following legacy Canic's separation from IC controllers. `app canister add` accepts
  `--role` and `--parent`; declaration listings and reports show the tree.
  Reject missing parents and cycles. The profile manager keeps its `manager`
  role, with `managed-worker` beneath it; entry/shared-worker routing does not
  create parentage. Existing principal bindings and retained plans are unchanged.
- `fleet status OPERATION` now prints a readable retained-plan summary and an
  IC Host table of retained effect evidence. Unknown outcomes stay explicit,
  upgrade creation is labelled inapplicable only when no creation was submitted,
  and receipts are historical evidence rather than current health checks.
  `--json` preserves the existing structured output.

### 🩹 Fixed

- Correct the growth fixture's report-label expectation and pause the Root
  fixture's automatic simulator clock during manual time advancement.
- Show network and identity conflicts directly beside a fleet's canister slots
  in terminal and browser reports. A retained operation on another trust binding
  is no longer presented as that slot's current identity, even when the URL and
  principal match. Original operation evidence remains visible and unchanged.

## [0.200.1] - 2026-11-14 (GitHub snapshot)

### ▶️ Try it

After `make canic`, check the version and list this checkout's declarations.
These commands make no network calls and open no browser:

```sh
canic --version
canic app list
canic fleet list --app profile
```

Add another app and its first fleet to this config, choosing an unused app name.
Existing apps are preserved; these commands deploy nothing:

```sh
canic app init another-app --fleet mainnet --network ic
canic app list
canic fleet list --app another-app
```

To add a second, empty mainnet fleet to the existing `profile` app, choose an
unused name. This edits `canic.json` only; it creates no journal or canister:

```sh
canic fleet init staging --app profile --network ic
canic fleet list --app profile
canic fleet show --app profile --fleet staging
```

Once that fleet exists, declare its first slot and inspect the declarations.
These commands do not create or contact canisters:

```sh
canic fleet canister add backend --app profile --fleet staging
canic fleet canister list --app profile --fleet staging
canic fleet plan --help
```

For an existing canister, set `canic_existing_id` to its principal and choose an
unused slot in the fleet for its actual network. This records a local declaration
only; it performs no network calls or controller changes:

```sh
canic fleet canister add existing --app profile --fleet staging --id "$canic_existing_id"
canic fleet canister list --app profile --fleet staging
```

### ➕ Added

- Extend `app init` to add a new app and its first fleet to an existing config,
  preserving other declarations through the locked atomic writer. Missing configs
  are created exclusively. Existing app names, malformed configs and capacity
  violations are refused; initialization never opens deployment journals.
- Allow `fleet canister add --id PRINCIPAL` to declare an existing canister without
  editing JSON. Refuse invalid principals, occupied slot names and duplicate
  membership on the same network. Declaration does not verify existence or control
  and does not authorize creation, funding or upgrades.
- Add `fleet canister add SLOT` and `fleet canister list`, both with explicit
  `--app APP --fleet FLEET`. Adding refuses existing slot names, preserves configured
  identities and uses the same atomic config update as fleet initialization.
  Listings show configured IDs only; use `fleet show` for deployment observations.
- Add `fleet init FLEET --app APP --network NETWORK` to declare another fleet
  under an existing app. It requires an explicit network, refuses existing names,
  and preserves other declarations and operation journals. Concurrent Canic edits
  are serialized; complete config updates are published atomically.

### 🔧 Changed

- Adopt published `ic-platform 0.33.0` in the product and qualification workspaces.
  Existing feature selections and stored deployment records are unchanged;
  rebuild with `make canic`. Testkit retains its independent native Platform 0.32
  dependency. The new registry codecs are not enabled by this increment.

### 🩹 Fixed

- Escape config paths in declaration messages and external text in fleet output
  through the shared IC Host renderer, including Unicode direction and line
  controls. JSON retains original values. Adapt legacy regression cases for
  display escaping, stable configuration snapshots and malformed edit refusal.
- Update the demo test runners to use `artifact build` with an explicit package.
  The local deployment journey now declares its slot through the CLI instead of
  editing JSON, and checks the inspected artifact against the retained plan.
  Demo instructions now describe the selected profile fleet rather than implying
  that its view also includes the separate Hello World fleet.
- Enforce the reader's 32 KiB limit before writing configuration, so initialization
  cannot leave a file that Canic subsequently refuses to load.

## [0.200.0] - 2026-11-14 (GitHub snapshot)

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
