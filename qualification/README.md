# Canic qualification

Run the commands below from the repository root. Installed tests use a prepared local
simulator and synthetic cycles. Prepare the simulator once with `make qualification-setup`.
Set `IC_TESTKIT_SERVER_DIRECTORY` to use an existing prepared bundle. Signer and browser
suites have additional prerequisites in their respective guides.

| To check | Run |
| --- | --- |
| Fleet CLI and persistent counter upgrades | `make journey-test`, `make counter-test` |
| Root and Store custody, maintenance and publication | `make root-test`, `make store-test`, `make publication-test` |
| Placement, reserve and worker reuse | `make miner-test`, `bash scripts/dev/test-store.sh sharding`, `bash scripts/dev/test-store.sh directory` |
| Provisioning and role-authorized calls | `bash scripts/dev/test-store.sh user-directory`, `bash scripts/dev/test-store.sh db-roles` |
| Auth enrollment, issuer and browser sessions | `make auth-test`, `make auth-profile-test` |
| Blob, DB and Backup composition | `make blob-test`, `make db-test`, `make backup-signer-test` |
| Wire ABI, endpoint guards and reply rollback | `make check-wire-endpoints`, `make endpoint-test` |
| Creation, funding and retained artifact experiments | `make effect-test`, `make artifact-custody-test` |
| Profile ownership and autonomous growth regressions | `make profile-test`, `make growth-test` |
| Metrics and notebook examples | `make metrics-test`, `make notebook-test` |

The [qualification history](../docs/qualification/history.md) records dated results and
inputs. Each runner selects its own cases and features.

## Miner Pool

`make miner-test` exercises the miner’s batch replay across an upgrade and the
installed pool’s growth, parking and reuse under one Root. The Host planner
binds the simulator identities to the example’s named `worker_pools` grant.
The witness checks production across hub and Root upgrades, outsider refusal
and the four-worker ceiling. A qualification-only Root traps the uninstall callback
after management applies it. The normal Root reopens the issued intent and
the controller resumes the same reset to recover exactly one available spare.
A second installed fleet has two physical slots, one hub and a separately funded spare. Its worker grant permits one live worker. Its zero creation budget refuses growth
until the spare exists. Requests for distinct worker ordinals compete for that
spare. Only the next ordinal gets an allocation. The other stays unassigned
until the worker returns through stop, uninstall, an observed
balance refusal, manual top-up and restart. Concurrent hub calls recover the same
principal and management slot at a fresh generation. Root archives the completed journal and the original ordinal stays unavailable. Root reopens before and after reassignment,
the miner rejects an old batch, and the new generation completes work.
The same journey also starts with a worker paid for by its hub grant. Root reopens after release, returns the principal to the fleet reserve, and reuses it. A third allocation is refused despite room under the subnet cap because the original creation spent the grant.

The reserve case plans an exact fleet policy, fills the initial deficit, claims
a worker, replenishes and exhausts the lifetime allowance. It exercises a named
former Root release, two-hour import quarantine, a Root upgrade, and use of the
imported principal with zero worker creation budget. A later policy approval
can append unassigned surplus after spending expires. The installed Registry
lookup fixture reports a distinct subnet for a surplus canister created on the
simulator’s NNS subnet. Root refuses the import on retry and its inventory stays
unchanged. Stopping a candidate during quarantine blocks final admission. The
simulator restarts it as Root and the same import completes exactly once.
The release case uses the former Root principal as the simulated caller. Its
immediate retry leaves the candidate awaiting handoff. Trying to release another
candidate in the same policy leaves the inventory unchanged.

The two-Root handoff case uses distinct controllers, an actual controller change,
a one-slot source, and upgrades of both Roots. The destination's first custody
observation frees source capacity when the source reconciles. The destination
separately completes quarantine. The directory suite installs two permanent hubs, a shared shard and a dedicated project from four ready spares. Each child records zero create dispatches. The three-service case adds a discovery hub to two existing services, retries all three approvals and checks independent routing for two hubs. The Store dispatch case upgrades the third service while preserving both earlier journals and pins. A separate case assigns the same key in two named pools under one hub and recovers both placements after Root reopens. It also retires and reuses both a newly created shard and a spare-backed shard.
[Fleet and expected rounds](../apps/examples/miner-pool/DESIGN.md)

## Named service and dedicated instance role issuance

`bash scripts/dev/test-store.sh directory` exercises an installed Root, named hubs,
a dedicated project and a shared shard. The named project hub and its project
request proofs for the accounts hub. The witness checks their signatures against
Root’s public key, the approved role, the original creation generation, and renewal
after each sender reopens. Both refuse the shard-only retirement operation.
The same suite covers shared-shard drain, expiry and copied proofs.

## Native endpoint inspection

`make check-wire-endpoints` builds the default User Directory hub and shard
from their app recipes. Linux PR CI, local CI and release admission use this target to check each actor’s mounted names and call
modes against the callable Wasm exports, then compares the generated per-method
Wire manifests with the [reviewed hub and shard baselines](../apps/examples/user-directory/wire-baseline/ex-user-directory/contracts.json).
These include the routing ID and both request and reply schema graphs. Optional
role-service builds are outside this baseline.

The downstream command and baseline adoption workflow are documented in the
[Wire CI guide](../docs/wire-ci.md). On drift the CLI retains a separate candidate
and reports both paths. For an intentional contract change, review the affected
methods and client compatibility, update the tracked JSON in the same PR, and
rerun the check. CI never updates the baseline.

`bash scripts/dev/test-store.sh user-directory` builds the fleet and runs the installed signup journey. The inspection suite checks a scoped request, a named
contract, Auth and readiness mounts, a recursive request and a nested enum
change. Custom Serde layouts still require application byte fixtures.
[Consumer instructions](../docs/demos/auth.md)

## Database-backed role calls

`bash scripts/dev/test-store.sh db-roles` installs two DB hubs and one shared
shard through Root and Store. An enrolled browser registers, then requests a
project from its shard. The shard checks the browser’s write permission and
looks up `project_hub` in its cascaded directory. The receiver admits the
shard’s role, inserts an application row, and returns the same ID after a
receiver upgrade and retry. The test also submits missing and incorrect issuer
pins to Root before staging the valid install. The directory witness also calls a named
service from a worker built with role attestation and no database. Run
`bash scripts/dev/test-store.sh directory` for copied proofs, expiry and retirement.
Toko owns qualification of its project payload, reservation rules and account association.

The installed DB role suite also exercises an Auth-wrapped dedicated instance
through Host planning, Root allocation and registry-only bootstrap. It verifies
an enrolled owner's write and an unrelated account's refusal. A separate
fresh Root reopens the populated allocation before enrollment and returns the
same child for the original key. Run `bash scripts/dev/test-store.sh db-roles`.

## Runtime shard placement

`bash scripts/dev/test-store.sh directory` exercises Root availability with two
installed hubs. Each hub requests its first shard while Root is stopped. After
restart, lookup finds no claim for that key. Retrying through the same hub
returns a pending allocation and then its matching ready shard. Once the hub has
received the directory, it can route to that shard while Root is stopped again.
The installed assertions check Root as the sole controller of both hub and shard.
The `miner` suite checks the same controller graph for a reserve-backed worker
and a freshly created worker.

Run `bash scripts/dev/test-store.sh sharding` after `make qualification-setup`.
The installed hub uses the public `canic/sharding` feature. Root creates the two
shards through its existing Store pin, creation, installation and readiness
journals. The hub is installed independently and retains its configured Root in
one named cell. The fleet declaration supplies the shard component and its hub
parent. Only the controller can admit the resulting grant. The approved application
init bytes bind the hub principal as owner of each installed database shard.
The test reads each shard's catalogue as that principal, checks the operator is
refused and checks the original generation against the argument-bound Store pin.

The witness checks same-key concurrent signup, lost replies, separate shard
principals and generations, pool exhaustion and replay after hub and Root upgrades.
An independently installed copy of the hub calls the same Root for the same user.
Root refuses both lookup and assignment, preserves its retained bytes, and
continues returning the original assignment to the approved hub.
A fresh sharding Root owns its claims in a named stable B-tree. The large-pool
case submits 20,000 distinct signups through the installed hub and retries boundary
accounts after Root reopen. For the hard-cut check, supply an inline-ledger Root
as `CANIC_PREDECESSOR_SHARD_ROOT_WASM`. An upgrade of that populated Root
must refuse and leave its original assignment usable.
The fixture adds a project registry with a `[registry]` placement declaration to the same fleet, reruns
`canic fleet plan`, and checks that the original shard request is byte-identical.
The controller submits the CLI-generated project request to `configure_instances`.
The fixture checks the topology frame, conflicting grants, unauthorized calls,
upgrade with both grants and refusal to downgrade to the selected predecessor.
With both user slots full, one user submits two project keys through the project
registry. The fixture discards the first reply, retries the key, and verifies two distinct
ready children with the approved project registry as their database owner. The third
key is refused. Reopening Root preserves both projects and the user's shard.
The controller then raises both pool counts from two to six under one eight-child
subnet ceiling using the planned Wire requests. The hubs race for the final
slot and replay their keys after Root reopens. Forged funding, extended expiry
and stale count changes refuse. Cross-hub placement requests are refused in both directions. A separate case
approves the project hub first and proves a zero creation budget leaves no claim.
Set `TOKO_USER_SHARD_WASM` to a built Toko user shard to repeat the same installed
CLI journey with its actual Wasm and 16-byte installation nonce. The fixture
hub requests placements and the Toko shard must report readiness and refuse
unauthorized provisioning. Its release and init bytes are approved through the
normal `fleet plan` request. Each run closes its local simulator. From the Canic
checkout with Toko's locked dependencies available locally:

```sh
canic artifact build --manifest ../toko-new/Cargo.toml --package user_shard --output target/toko-user-shard.wasm
TOKO_USER_SHARD_WASM="$PWD/target/toko-user-shard.wasm" bash scripts/dev/test-store.sh sharding
```

Two further cases refuse unauthorized or unfunded growth, and lose Root's create
callback after the effect. The latter permits observation of the original intent
and keeps the second slot empty. Native tests check legacy HRW score vectors, first
candidate tie breaking, and refusal to resolve replaced generations or principals.
Run `make store-test` for these and the other installed Store cases.

### Route an unseen principal within a named pool

A hub can keep its pool name in the application protocol and use the same call
regardless of how many pools its directory contains:

```rust
let destination = canic::topology::get_shard(canic::shard_pool!("discovery_shard"), wallet)?;
```

`canic app build` supplies the selected component’s direct children. Native Cargo checks the nearest ancestor `canic.toml` and identifies the hub through its package’s build recipe. Each call site marks the hub Wasm with its required pool. Whole-fleet planning checks the pool kind, component and parent against the selected fleet before approving work.

Use the `component` value from the fleet's shard-pool declaration. Root already
approves that name and carries it to the hub. The synchronous lookup returns a
principal from the acknowledged direct shards of that pool. The directory suite
checks the published binding through an installed Root. Canic's native tests
cover two pool names with the same child role, an absent directory, pending
children, and stable routing after reopen. Root currently provisions one shared
shard pool per Root. The directory's v2 delivery and stored format require
compatible publishers and fresh indexes.

### Provision an account on its assigned shard

The [hub](../apps/examples/user-directory/hub/src/lib.rs) and
[shard](../apps/examples/user-directory/shard/src/lib.rs) are the runnable
implementation. Run `bash scripts/dev/test-store.sh user-directory` with the
prepared simulator. For Toko's existing payload, storage layout, exact migration
and test commands, see the [provisioning migration](../docs/demos/provisioning.md).

The scoped reads and async registration use the handler-local `auth` identity.
With a conflicting account ID on the installed shard, registration returns
`BindingConflict` and both stores retain their original bytes. Stopping the
shard returns `ProvisionPending`. Replaying a pending hub reservation against
the populated shard confirms the original ID without rewriting the shard.
During a suspended registration a second browser uses its own read scope. A
late provision acknowledgement after logout or after Root revocation and
refresh leaves the hub's pending row untouched.

The test also upgrades a populated volatile shard under its actual Root controller.
The database bytes and hub route survive. The shard refuses the previous session,
a freshly signed admission and a controller-triggered enrollment refresh.

## Optional Root snapshot executor

`bash scripts/dev/test-root.sh snapshot::` qualifies Canic #6's optional fresh Root
layout against an actual Root-created persistent counter. It covers exact
execute/lookup replay, one-MiB transfer, upload/load with stopped completion,
populated stable-byte preservation, authority and argument refusal, expiry,
call-cost admission and callback-evidence loss across Root upgrade. Protocol and
ledger unit tests cover full-extent framing, conflicting original bindings,
finite slots and unretained oversized replies. The installed consumer decodes
execute and lookup with `Exchange::decode_response`. A response for another
method, a changed original binding or an oversized inner result fails closed.
The decoder accepts payloads only after the caller authenticates the outer ingress.

The separate Backup integration below exercises the public Host consumer. Canic #7 owns application fencing and reopening. No
mainnet work or existing-Root layout migration is qualified. The ordinary Root
build keeps its existing two-record layout; `backup` adds a third bounded record.

## Storage 0.78 adoption

`cargo test --locked -p canic --features storage storage::tests` creates a
directory with lower-level Storage admission and reopens it with Canic under
the same owner and independent prefix. It checks retained IDs and cell bytes,
incompatible budgets, missing or extra names, and out-of-prefix refusal.
`make profile-test` exercises named Wire mounts on an installed actor,
including byte-exact replies, caller rejection before decode, wrong direction
and method, bounded requests, synchronous reply rollback and an async worker check.

The Root journal uses CUJE at every fleet size. The installed `storage_reopen::`
case injects the predecessor CUJD tag into the current named cell and checks the
journal-specific refusal, stable bytes, module hash and subnet canister count.
The installed sharding journey grows a fresh Root through shared and dedicated
child allocations and checks their identities after upgrade.

Fresh counter and Root installations use Storage named cells. Application codecs retain
their payload bounds. Storage owns directory encoding and allocation. Keep deployed
0.76 records with their qualified executable until a physical-image migration is qualified.

The DB suite checks populated and refused upgrades and damaged storage. The Blob
suite checks fresh composition, the managed fleet journey and refusal to upgrade
the populated 0.100.6 catalog. The predecessor retains its stable bytes, management
report and surviving asset. [Storage decision](https://github.com/dragginzgame/ic-storage/issues/8)

## Backup integration boundary

The optional `canic-host/backup` adapter connects IC Backup 0.105.6 to
Tooling's published `ic-host-sign` 0.1.0. It passes the original management
arguments or status request ID, sender, effective target, exact expiry, deadline
and spending limits. Backup commits to the adapter version, provider, identity and
configuration at approval. Tooling owns key access and process cleanup. Backup
owns signing reservations, signed bytes, transport and observation.

On Linux with ICP CLI, D-Bus and GNOME Keyring installed, run:

```sh
make qualification-setup
make backup-signer-setup
make backup-signer-test
```

The fixture creates its own session bus, keyring, secp256k1 identity and stopped
canister. It drops the response to a real snapshot take, reopens the journal and
reconstructs the signer. Changing its public manifest path refuses recovery.
The original configuration permits one explicit status observation. The fixture
checks the snapshot count, journal bytes and signature spending, verifies the
completed capture offline, overwrites the fixture's stable bytes, restores and
checks the bytes and stopped status.

The managed case installs a fresh Backup-enabled Root and its persistent counter.
The signer controls Root, and Root is the counter’s sole controller. Supply
`canic_host::backup::SnapshotCodec` to Backup’s `with_root_manager` connection
and `with_root_codec` store. The codec binds each step to its original management
arguments and checks the returned binding before exposing the retained result.
The fixture drops the take and load replies, reopens the operations, uses one
explicit lookup apiece, restores the original bytes and checks stopped completion.
Root fences and stops the counter for each operation. The app checkpoints value
1 under fence 1, advances to value 2, and closes under fence 2 before rewind.
Host derives operator calls with `canic_host::backup::recovery_request`.
Its `CounterRecoveryGuard` checks authenticated Root receipts locally. Root
checks its ledger at dispatch. Allocation and load need separate operator
admissions. After start, the restored counter reports value 1 and fence 1.
Replaying the old successful opening leaves writes closed. The operator accepts
generation 3, verifies the counter is still fenced, then opens it explicitly.
Only then does the next public increment succeed. The fixture reconstructs
guard evidence by looking up the original requests. It discards the ingress
handles for restore start, acceptance and opening and observes Root's retained
results. Changing Root's controller after acceptance removes the original
operator's lookup and execution authority. The replacement controller cannot
use the original plan to open the counter. With the gateway stopped, reopening
the journal yields the exact restore receipt and unchanged signature, network
and observation spending.

The test uses Testkit operator ingress for these lifecycle calls and the
external signer for Backup's snapshot procedure. The one-capture/one-restore
ledger has no reusable generation slots. Unresolved callbacks stay blocked.
The snapshot fault test also exercises application rejection and a child fence
whose acknowledgement was lost across a Root upgrade. The app has no timers,
outbound work or other participants to reconcile.

These are local Linux qualification routes. The provider requires a
keyring-backed identity. Password-encrypted ICP identities use a different
storage contract. The same test exercises partial and oversized signer output,
provider death, deadline expiry and cancellation after launch. Each failed capture
retains one signing reservation, sends zero replica requests and refuses to sign
again after journal reopen. Real signed envelopes also exercise method, argument,
expiry, sender and original-status binding checks for [Canic #5](https://github.com/dragginzgame/canic/issues/5).
For an original-request status lookup, the adapter returns the exact signed
envelope when the request matches. It rejects a different request ID and also
rejects a one-nanosecond change to the requested ingress expiry.
Root-managed execution and application fencing/reopening have their own
acceptance in [Canic #6](https://github.com/dragginzgame/canic/issues/6) and
[Canic #7](https://github.com/dragginzgame/canic/issues/7).

## Qualified baseline

The CLI journey installs and upgrades the stateless greeting witness at the same principal.
The counter checks persistent state, write fencing, failed-upgrade rollback and refusal to
reinitialize damaged storage. The managed variant publishes through Store and upgrades
through Root. Run `bash scripts/dev/test-publication.sh counter_managed::` for that path.

The CLI journey also builds a standalone consumer's locked crates.io Root with
Auth and role issuance. It checks that both updates are exported, the Wasm fits
direct installation, the paired native client encodes the role-key command, and
the consumer lockfile stays byte-identical. To run that case on its own after
`make canic`, use `bash scripts/dev/test-locked-root.sh "$(command -v canic)"`
from the repository root.

Auth tests exercise distinct receiver grants, enrollment, issuance, recovery and logout.
The installed browser and Blob suites select `profile-state` for independent readback of
persisted application rows. Blob also checks that catalog and reference changes roll back
together, and that reopen requires activation of the matching maintenance attempt.
[Auth](../docs/demos/auth.md) · [Blob](../docs/demos/blob.md)

Store tests cover retained pins and cache delivery, stopped owners, interrupted handovers
and upgrades. The publication runner exercises the operator CLI and signer and records
cold and warm publication measurements under `target/publication/`.
[Store qualification and operator commands](../docs/demos/wasm-store.md)

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
The [Root guide](../docs/demos/root.md) covers ordinary two-child deployments.
The [runtime placement witness](#runtime-shard-placement) covers shared shards and dedicated projects under one eight-child subnet ceiling. Dated decisions and measured results
live in [qualification history](../docs/qualification/history.md).

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
under `target/qualification/server/prepared`. Tests only check that prepared bundle; they never
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

| Installed guarantee | Focused runner |
| --- | --- |
| Two pending children retain distinct approvals across Root maintenance | `bash scripts/dev/test-root.sh children::` |
| Unsupported or corrupt Root images refuse reopen before writes | `bash scripts/dev/test-root.sh storage_reopen::` |
| A failed Root upgrade preserves the old executable and signed attempt | `bash scripts/dev/test-root.sh predecessor::` |
| Store handover readiness and custody survive interrupted calls | `make store-readiness-test`, `bash scripts/dev/test-store.sh custody` |
| Publication resumes the original approval and reuses immutable releases | `bash scripts/dev/test-publication.sh bootstrap_store_then_publish_resume_and_retain_versions` |
| CLI deployment binds the issuer key into the approved fleet installation | `bash scripts/dev/test-publication.sh role_fleet` |
| Profile retains private ownership, capacity, discovery binding and state through upgrade | `make profile-test` |

Frozen Wasms are actual test inputs. Their [provenance and digests](canic-qualification/fixtures/README.md)
identify the predecessor each test needs. Simulator lifecycle checks can be selected with
`cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test server`
after simulator preparation.

## Application-owned growth journey

`make growth-test` composes the real profile worker with a one-slot manager,
IC Storage and IC Timers. It uses the Canic deployment CLI and native profile
approval client, then creates and installs one worker without another mutating
operator command. It exercises authenticated application use, five interruption
boundaries, a distinct manager executable, exact replay, network binding, expiry
and insufficient reserve. Subnet counts independently detect duplicate creation.
Lost creation replies remain blocked; upload/install outcomes can be reconstructed
by observation, including callback-trap recovery without replacing the manager.
Abandoned upload/install callbacks remain blocked across further timer
opportunities before explicit reconciliation. All cycles are synthetic.

The test also qualifies maximum artifact/evidence codec budgets and emits a fleet
snapshot under `target/growth/journey-N/fleet.html`. Serve the selected snapshot
with `canic fleet view PATH --port 8788 --open`, then use **http://localhost:8788/** in
the normal desktop browser. Ctrl-C stops the viewer; the replica has already
closed. See the [growth guide](../docs/demos/growth.md) for the selected topology,
limits, fresh mainnet instructions and recovery limitations. The focused lint and
documentation checks are `make profile-clippy` and `make profile-doc`.

## Typed database composition

The installed DB witness also exercises the shared `metrics` query. It reads
commit and failure counts before upgrade and observes the reset after reopen.
In the supplier-enabled fixture it observes one completed one-shot job and two
fresh entropy requests (one success and one refusal while suspended). Run
`make db-test` for those populated sources and `make metrics-test` for the
controller guard, Root client selections and absent recorders.

`make db-test` selects only `canic-db-witness`. The application declares its own
`Item` schema, stores the installation owner separately, initializes IC DB
explicitly and supplies synchronous inspect/query/write handlers to
`canic::mount_db_endpoints!`. The `db` feature adds no IC DB runtime dependency to
Canic. Its passive `db::inspection` contract is versioned independently as
`method_id("canic.db.inspect.v1")`. The separate optional `db-schema` feature adds
stateless projection of compiled IC DB metadata, with no database executor,
storage owner or permission policy. The witness consumes crates.io IC DB and its schema macros at 0.27.0; transport never becomes its durable codec.

```sh
make qualification-setup
make db-test
make db-clippy
```

The installed test checks authorization before malformed decoding, wrong
method/direction and oversize refusal, atomic batch conflicts, sparse empty pages,
cursor binding, populated upgrades, reply-encoding rollback, frozen ordinary access,
supported cancellation and incompatible-executable refusal without changing stable
backing. A rejected upgrade preserves the prior module, stable bytes and a live
query cursor. Empty and corrupt storage directories both refuse upgrade without
changing the supplied bytes. The compatible executable reopens all seven rows. Normal Wasm excludes the private fault/maintenance
entrypoint. Requests bound results to 1–3 and candidates to 1–4, with 1 KiB post-fetch
output and a 64 KiB frame envelope; writes contain at most four rows. These are
application witness bounds, not new engine limits.

The managed test in `make db-test` builds the CLI from the checkout and uses
it to deploy a DB child through separate Root and registry Wasm Store actors.
The qualification-only `managed-fixture` build pins a synthetic application
user for Root's empty install argument. DB readiness reports whether the stored
schema opens for ordinary access. The test populates two rows, publishes a
replacement, and traps Root's install callback after the replacement commits.
A fresh CLI invocation reconciles the original approval, signed request and
single install attempt. The child advances one executable version, retains its
stable bytes and both rows, denies Root's application query, and admits the
original user's next write. These runs use local synthetic cycles.

The fault artifact also deliberately expands an inspection reply after successful
schema projection. The installed endpoint must refuse its final encoding rather
than return truncated metadata, preserve the entire stable backing, and resume
the original catalogue/incarnation/cardinality after the fault is cleared. This
tests the final application reply ceiling; the injected metadata is not a claim
that IC DB admits that compiled schema. Ordinary artifacts have no such control.

Inspection returns a complete catalogue of at most 64 exposed entities and a
selected entity bound to the observed executable schema fingerprint. Names have
64-byte bounds; fields, indexes, named definitions and enum alternatives are each
capped at 64, index components at 16, with the shared 64 KiB reply ceiling. Recursive
types use finite named links. References expose logical target/supporting-index
facts separately from value types. Limits refuse without truncating complete
metadata. There is no pagination or snapshot-consistency claim. Executable schema,
readable incarnation and closed/incompatible admission are distinct facts.
Cardinality uses `Database::cardinality`, only under application permission and
ordinary admission; closed/unsupported state never triggers a row scan or repair.
Native schema cases cover recursion, reference/index projection and capacity.

`target/db/` retains the source archive, lockfile/compiler, Cargo artifact records, selected
feature graphs, normal/fault/incompatible Wasm and installed run log. Simulator setup is
explicit and uses the existing `IC_TESTKIT_SERVER_DIRECTORY` selection. This
qualifies typed composition, not arbitrary remote queries, production authority
storage, full migration orchestration or a deployed-database CLI. All database
semantics remain in IC DB; permission and any tenant/row restrictions remain in
the application. Mounting adds no initialization or lifecycle hooks.

### Retained application store

The database declaration selects the IC DB canister schema:

```rust
canic::mount_db!(schema = toko_schema::canister::user_shard::UserShard);
```

Select `canic/db-store` to use this mount. Include `database::keys()` in the
application's Storage bootstrap. Call `database::attach(&host, Some((nonce,
principal)))` on install and `database::attach(&host, None)` on reopen.
`db!().get::<User>(id)` routes to the store declared for `User`.
The [two-store witness](canic-schema-role/src/lib.rs) uses ordinary lifecycle
hooks. Its installed upgrade and rollback test runs with
`bash scripts/dev/test-store.sh db-roles`.

The installed User Directory suite exercises both receiver layouts. With
stable Auth it upgrades the populated hub and shard, checks both refuse reads
before refresh, and reads the same route and ID with their original sessions
after refreshing each receiver. The volatile case still checks lost sessions
remain closed. Run `bash scripts/dev/test-store.sh user-directory` after testkit
setup. [Auth lifecycle](../docs/demos/auth.md)

The User Directory adds a receiver lifecycle beside its database declaration.
For a shard with an existing parent cell, keep its original storage policy:

```rust
canic::mount_receiver_lifecycle! {
    parent_storage_key = "toko.user.shard.hub.v1",
    authority = "user_shard",
    prefix = "toko.user.shard.",
    auth_read = "read",
}
```

This lifecycle attaches the database and retains the parent principal from
the approved topology envelope. Declare the parent once in `canic.toml`.
The storage key identifies the cell where that binding survives upgrade. When
the prefix is the authority followed by a dot, omit `prefix`. This example
keeps a custom prefix because its existing keys use that namespace.
The hub's receiver lifecycle selects its retained `config_key` and page
budget, and restores its routing Root. Both layouts keep their Auth policy,
readiness check and strict reopen behavior. Adding a store to an installed
directory requires migration.

For a generated schema with an application-owned lifecycle,
`canic::declare_db_store!(pub mod database, store = app_schema::model::Items);` exposes
`database::SCHEMA` and `database::DATABASE_KEY` natively and
`database::store::{attach, with}` on Wasm.

For an application that constructs its schema explicitly, `canic/db-store` also
accepts an allocation key and a schema function:

```rust
canic::declare_db_store!(pub(crate) mod users,
    key = super::DATABASE_KEY, schema = super::schema);
```

Keep the application's existing Storage bootstrap and hub cell. In the lifecycle
hook, pass `Some(original_incarnation)` on install and `None` on upgrade:

```rust
users::attach(&host, incarnation);
```

An authorized handler can use `users::with(|db| get_me(db, &identity))`.
The application computes the incarnation only on install. A Storage or DB refusal
traps the lifecycle message. The helper exports no endpoints. Applications that
need to inspect a fenced database can handle IC DB's open result directly.

`make db-test` includes a populated supplier-created predecessor and checks
allocation IDs, adjacent bytes, strict reopen and schema refusal. The
[installed Toko shard test](../docs/qualification/history.md#retained-toko-shard-store)
also checked hub-only admission across a Wasm upgrade.
