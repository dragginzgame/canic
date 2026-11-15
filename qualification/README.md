# Canic qualification

This workspace owns installed effect-boundary and product regression tests.
Its packages are named by responsibility, independently of design milestones:

| Package | Responsibility |
| --- | --- |
| `canic-qualification` (`tests/`) | Native harnesses, retained predecessor binaries and explicit shared simulator preparation |
| `canic-effect-probe` (`effect-probe/`) | Small Wasm probe isolating creation/funding commit boundaries and owner rollback |
| `canic-asset-catalog` (`asset-catalog/`) | Real embedded Blob/catalog composition with explicit lifecycle and qualification-only failure points |
| `canic-artifact-probe` (`artifact-probe/`) | Small Wasm probe isolating retained-byte installation onto an existing target |

M0/M1 package and Make names are retired without aliases. `make qualification-setup`
prepares the simulator for every suite. Select `make effect-test`,
`make artifact-custody-test` or `make journey-test` for the renamed workflows;
`root-test`, `growth-test`, `profile-test` and `endpoint-test` keep their names.
`make store-test` qualifies versioned Store custody and Root's Store-backed
creation/upgrade engine, after `make canic` and simulator preparation. It builds
Store and Root executables through the public artifact builder, checks bounded
Wire frames, resumes publication across Store maintenance and verifies byte reuse.
Root-mediated publication remains usable after controller handover: incomplete
uploads resume after Root maintenance without changing Root stable bytes.
Malformed/unauthorized/expired requests preserve state; lost Root replies retain
Store commits and exact replays recover through replicated observation.
Actual Root calls acquire pins, install a child larger than the former inline
limit, and upgrade it using the Store cache. Lost pin/cache callbacks recover
through observation after Root maintenance. Missing cache, unpublished releases,
expired authority, retained upgrade history and mixed inline/Store custody have
explicit cases. A separate provider fixture reconstructs a cleared cache and
performs real chunked installation/upgrade. Fleet CLI deployment remains on its
existing inline path. `make publication-test` focuses on the actual CLI, ICP signer
and installed Root/Store publication path: interrupted upload, read-only
reconciliation, exact resumption, immutable version reuse, certified-code refusal
and signed observation custody. It needs the prepared simulator and ICP signer,
uses synthetic cycles, and leaves Root stable memory unchanged. Publication does
not create a canister or reserve a deployment slot; see [Store scope](../docs/demos/wasm-store.md).
`make blob-test` builds two embedded catalog executables using the optional
Canic adapter and the real Blob Service at its pinned Git revision. Five installed cases cover user
isolation, shared references, catalog/Blob rollback, malformed frames, capacity,
non-resurrecting replay, drain/reopen/exact activation and reopen refusal.
Supplier source/files and both consumer locks are fingerprinted before/after;
no supplier checkout files are changed. The immutable input is explicit
in [AGENTS](../AGENTS.md#local-blob-composition). See [scope and evidence](../docs/demos/blob.md).
Tests never install the simulator themselves. Preparation and tests use the same
`IC_TESTKIT_SERVER_DIRECTORY` override, defaulting to `target/qualification/server`.
Existing files under `target/m0/` are historical outputs and are not deleted or
silently used as a fallback; run setup once for the new location.

## Coverage retained from the initial experiments

The first retirement review keeps every effect scenario because the product
fixtures do not yet replace all of their obligations:

| Coverage | Why it remains |
| --- | --- |
| Creation dispatch, callback-write and later-message traps | Isolates each IC commit boundary; product growth covers a narrower set of interruption points |
| Direct-cycle funding, rejected recipient and refunds | Product runtime has no equivalent funding journey yet |
| Failed/successful owner upgrade with empty, successful, rejected and uncertain journals | Preserves the combined creation/funding rollback matrix |
| Retained artifact, occupied target, lost upload/install replies and maintenance | Proves install-only behavior on a pre-existing target; product growth creates its own target |

The separate native M0 harness has been removed; all tests now share the native
qualification package. The two probe implementations remain intentionally
isolated. Delete one only after product regressions cover its remaining cases.
The current Root-plus-two-child scope is described in the [Root guide](../docs/demos/root.md);
dated decisions and measured results live in [qualification history](../docs/qualification/history.md).

## Management-effect boundaries

These installed PocketIC experiments ask whether the management owner can retain
creation, funding and artifact evidence across traps and owner upgrades. They use
the published Platform runtime and real local management calls. These are
qualification fixtures, not a deployable Canic runtime or the Hello World CLI.

From the repository root, after the normal development tool preparation:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.100.0
cargo fetch --locked --manifest-path qualification/Cargo.toml
make qualification-setup
make effect-test
```

`qualification-setup` explicitly provisions Testkit's authenticated PocketIC bundle
under `target/qualification/server`. Tests only check that prepared bundle; they never
download a server. An already prepared bundle can be selected with
`IC_TESTKIT_SERVER_DIRECTORY=/absolute/path make effect-test`. Tests need permission
to bind a local loopback port. They create isolated local instances, use synthetic
cycles, and close each instance after its case. No mainnet endpoint or signer is
configured.

The runner builds the probe and selects its Wasm from Cargo artifact messages.
It first checks that maximum retained evidence fits the Wire status contract,
then runs five installed creation scenarios:

| Scenario | Observation required |
| --- | --- |
| Trap after enqueue, before dispatch commit | No new canister; intent and reservation roll back |
| Trap before recording the returned principal | One new canister; committed intent remains without a retained principal; another attempt is blocked |
| Write principal, then trap in the same callback | Same uncertainty as the preceding case: the write was not a commit |
| Commit principal, then trap in a separate self-call | Principal and original outcome survive; replay returns that evidence without creating another child |
| No injected fault | Creation evidence and controller assignment are retained; replay has no creation effect |

Each executor admits at most one creation and attaches 2 trillion synthetic
cycles. The harness bootstraps it with PocketIC's default 100 trillion cycles.
The reported executor balance decrease includes transferred cycles and execution
charges; it is not a measurement of consumed cost alone or a mainnet quote.
Uncertain attempts retain their full attachment as exposure and cannot reset it.

PocketIC subnet counts and controller inspection are independent test oracles.
They are not supplied to the executor to reconstruct a lost result. A child can
exist while its principal is absent from the executor's committed evidence.
The fixture deliberately demonstrates that blocked window rather than guessing
the principal or automatically retrying.

The funding experiment repeats the five interruption windows for both an empty
live recipient and a deleted recipient on the same subnet. The latter produces
a management rejection and refunds the attachment. Each fresh executor admits
one deposit of at most 1 trillion synthetic cycles, retaining the destination,
amount and exact management request. Successful callbacks retain completion with
zero refund; rejecting callbacks retain their raw reject code and matching refund.
Any unrefunded rejection exposure stays reserved. A retained result is returned
on identical replay; an uncertain attempt stays blocked. Changed destinations
or amounts cannot overwrite either slot state, including a fully refunded rejection.

Independent recipient balances prove whether a second credit occurred. Executor
balance changes help check physical refunds, but neither balance is accepted by
the probe as operation evidence. A lost callback stays blocked even when the
harness can see the transfer or refund. Separate admission cases check zero and
excessive amounts, malformed principals and unauthorized callers, followed by a
valid deposit to prove invalid commands did not consume the slot. Creation and
funding use independent test slots; this is not a shared product allowance policy.

The maintenance rehearsal uses two different executables with the same journal
format. It captures both journals outside the executor, stops it, injects a trap
after the candidate restores and writes stable memory, and restarts the surviving
executable. It then performs a successful upgrade and repeats the failure with
populated stable memory. Module identity, controller graph and exact journals
must survive each failed upgrade; the successful upgrade must select the changed
executable without resetting outstanding exposure. Replaying commands must create
no extra child or deposit. Cases cover an empty slot, retained successes, a
retained rejection/refund, lost callback evidence and failure in later work.
An unauthorized principal must be unable to upgrade the executor.

Run just that rehearsal with `make effect-maintenance-test`. `make effect-test` includes
it alongside the creation/funding boundary cases. The runner builds both Wasms
and saves their separate inputs under `target/effects/` before executing the tests.
The checkpoint stores each journal in its existing bounded Wire frame, using
one 64 KiB stable page through Platform Runtime. This isolated test owns that
raw backing; it does not qualify a production `ic-storage` integration. Existing
heap-only probes are not upgrade inputs: the harness installs fresh executors.

These cases qualify a maintenance window after message executions have settled,
including uncertain effects left by failed callbacks. They do not qualify
upgrading with callbacks in flight, schema migration, unconditionally trapping
pre-upgrade hooks, host crash recovery, governance, ledger-based funding, artifact
availability during operator absence or mainnet executor maintenance. The host
harness retains observations in memory, not a durable operator journal. Those
remain separate proofs. The continuation is an actual self-call; an ordinary
Rust-only `await` is not a commit boundary.

## Artifact custody without the operator

Run `make artifact-custody-test` after the same preparation. This separate fixture
keeps the preceding creation/funding probe unchanged. It builds the actual Hello
World worker and two executor executables from Cargo artifact records. The
controller approves one exact target, uncompressed Wasm hash, retained module,
dispatch time and interruption mode; invalid hashes, oversized uploads and
unauthorized callers cannot consume the slot. An admitted approval cannot be
replaced, even after a rejection or unknown outcome.

The executor survives a failed upgrade and a successful upgrade before dispatch.
With no subsequent operator command, a test-only heartbeat uploads the retained
bytes to its own IC-managed chunk store and calls `install_chunked_code` on an
existing empty worker. This fixture qualifies the on-chain chunked API selected
during the initial experiment; current supplier versions are pinned by Cargo.lock. It uses
one chunk and adds no Store canister. Direct host installation remains the CLI
baseline; this is not a second product deployment engine.

Four installed cases cover success, a lost upload callback, a lost installation
callback and refusal to install over existing code. A later executor upgrade
must preserve the original request commitments and attempt counts. A lost upload
callback remains blocked, even when the test oracle can see the stored chunk.
A lost install callback can be reconciled through the management API's module
hash and sole-controller evidence, without reinstalling or claiming that the
original callback was retained. Worker canister versions independently check
that rejected installs and repeated approvals did not replace code again.

The fixture limits artifacts to 64 KiB. Its checkpoint owns raw stable backing
only within this experiment; the provider chunk store is separately measured.
Neither proves production Storage integration, a production scheduling policy,
multi-chunk handling, chunk cleanup, callback-in-flight upgrades, application
readiness or a mainnet cost estimate. The worker is pre-created by the harness.
Product Root/growth fixtures separately qualify combined creation, installation,
readiness, maintenance and fleet views; this probe does not replace those tests.
The CLI's initial JSON/HTML inventory is separately exercised by `make journey-test`
against two greeting deployments, an uncertain upgrade and a stopped provider.

Focused checks are `make artifact-custody-clippy` and `make artifact-custody-doc`.
The aggregate `make ci` includes the artifact experiment separately from
`make effect-test`.

## Checks and evidence

`make profile-test` runs the [Canic-owned profile demo](../docs/demos/profile.md)
through the existing deployment CLI, with private ownership, bounded capacity,
failed-upgrade rollback and preservation across two executables. It also deploys
the discovery entry, checks immutable binding and upgrade preservation, exercises
direct worker observation and unavailable-worker recovery, and renders the pair.
Its entry, worker and client are products under `apps/profile/`; only its installed harness belongs
to this qualification workspace. `make profile-clippy` and `make profile-doc`
cover its native and Wasm graphs. These targets are included in the corresponding
repository aggregates.

Focused checks are `make effect-clippy`, `make effect-doc`, `make fmt-check` and
`make toko-tooling-check`. The aggregate `make ci` includes the fixture and
requires prior `qualification-setup`; it does not install tools. Qualification packages have
their own catalog and lockfile, with no dependency from the product workspace.
Build output lives in `qualification/target`; `target/effects` and
`target/artifact-custody` retain probe inputs and Cargo artifact records.

The [historical record](../docs/qualification/history.md) retains dated observations
and decisions. Passing these experiments does not establish
general effect recovery. Local dispatch-resource failures, cross-subnet funding
and production approval of blocked exposure remain unqualified.

## Application-owned growth journey

`make growth-test` composes the real profile worker with a one-slot manager,
IC Storage and IC Timers. It uses the Canic deployment CLI and native profile
approval client, then creates and installs one worker without another mutating
operator command. It exercises authenticated application use, five interruption
boundaries, a distinct manager executable, exact replay, network binding, expiry
and insufficient reserve. Subnet counts independently detect duplicate creation.
Lost creation replies remain blocked; upload/install outcomes can be reconstructed
by observation after maintenance. All cycles are synthetic.

The test also qualifies maximum artifact/evidence codec budgets and emits a fleet
snapshot under `target/growth/journey-N/fleet.html`. Serve the selected snapshot
with `canic fleet view PATH --port 8788 --open`, then use **http://localhost:8788/** in
the normal desktop browser. Ctrl-C stops the viewer; the replica has already
closed. See the [growth guide](../docs/demos/growth.md) for the selected topology,
limits, fresh mainnet instructions and recovery limitations. The focused lint and
documentation checks are `make profile-clippy` and `make profile-doc`.
