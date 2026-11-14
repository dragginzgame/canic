# Changelog

## [0.200.0] - Unreleased

### ➕ Added

- Add `canic fleet --watch` for automatic read-only observations and a refreshing
  localhost page. Keep one observation round in flight, preserve the last snapshot
  with a visible warning on failure, and stop all monitoring with Ctrl-C.
  Print the localhost URL by default; launch a browser only with explicit `--open`.
- Add read-only `canic wallet PRINCIPAL`, offline receive-address derivation and
  an optional `fleet --wallet PRINCIPAL` panel. Keep exact ICP/cycles amounts,
  explicit uncertified observations and per-ledger unknown results.
- Add a one-slot profile manager with retained Wasm, IC Timers scheduling and
  a 1T worker-creation attachment. Add native growth approval/observation commands
  and `canic fleet --manager` for the manager and its reported worker.
- Serve fleet reports at localhost URLs: `canic fleet --open` starts the viewer
  and browser, `--serve` prints the link, and `canic view PATH` serves a saved
  snapshot without reconnecting to canisters. Ctrl-C stops the viewer.
- Add a persistent profile discovery entry with controller-only immutable binding,
  certified client resolution and bounded direct worker checks. Profile calls keep
  the original user identity. Show the pair with `canic fleet --link ENTRY:WORKER`.
- Add a Canic-owned persistent profile demo with a native client, owner-only
  reads/writes, immutable user mappings, bounded capacity and idempotent revisions.
  Deploy it through the existing CLI and qualify preservation across two executables.
- Add `canic fleet` and optional HTML snapshots of locally known deployments,
  with certified module/controller observations, retained operation evidence,
  distinct network trust bindings and explicit partial/unavailable results.
  `canic fleet --open` creates a fresh snapshot and launches the default browser.

### 🔧 Changed

- Start the rewrite's version line at 0.200.0, distinct from legacy Canic.
  Share development through GitHub and defer crates.io publication until mature;
  keep publication disabled for all packages.
- Adopt Toko Tooling 0.43.0 through the authenticated installer in both workspaces,
  retaining the existing executable selection and lint policy. Use `toko-cloc`
  instead of the retired `make cloc` command.
- Adopt IC Testkit 0.104.0 in qualification, retaining raw PocketIC calls and
  directly owned Wire encoding without adding the optional typed-call adapter.

### 🧪 Testing

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
