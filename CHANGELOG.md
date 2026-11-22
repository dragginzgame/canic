# Changelog

## [0.256.0] - 2026-11-22

The product and qualification lockfiles select IC DB and its macros at 0.27.1.

Canic adopts IC Auth 0.49.0. Session admission and read replies carry the
issuer’s optional application bytes. Update browser clients alongside receivers
for the revised Wire reply. Check an app’s reviewed contracts after updating.

```sh
canic app check-wire --config apps/examples/user-directory/canic.toml
```

Fleet config can declare a threshold and fixed top-up amount per direct child.
For example, `top_up_children = [{ child = "project_registry", below_cycles = "10.3T", amount_cycles = "0.1B" }]`
covers that registry only. Existing name-only entries retain their original
eligibility. For a named service directly installed by Root, the operator can
approve a lifetime budget and deadline. Root checks its balance once a minute.
To fund the registry without subsidising project creation, set
`creation_payer = "requester"` on the registry grant. Its hub first requests a
quote, then attaches cycles to settle the exact allocation. Use a fresh Root
for the PLS2 pool ledger and generate fresh pool approvals with the v2 grant frames.

```sh
# In a configured fleet, inspect the funding edges and their amounts.
canic fleet canister list
# For a bound registry, review the approval before submitting it as the controller.
approval=$(canic fleet canister plan-funding project_registry --budget-cycles 2000000000000 --expires-at-ns "$DEADLINE_NS" --json)
canic fleet call --canister root --identity "$IDENTITY" configure-child-funding "$approval"
```

## [0.255.0] - 2026-11-22

Canic’s DB adapter now accepts IC DB 0.27 schemas. The installed two-store
fixture commits a timestamped batch through `db!()`, reads the row returned by
`replace`, and verifies the update survives reopening. [#80](https://github.com/dragginzgame/canic/issues/80)

Use fresh Root and Wasm Store 0.8 pairs. Finish existing maintenance with the
original pair. The new Root requires journal CUJE and Store's
`observe_preservation` method 117. Platform Runtime is 0.23.25.

The one-child-per-key placement pattern is called **registry**. Rename
`[apps.<app>.fleets.<fleet>.instance_pool]` to
`[apps.<app>.fleets.<fleet>.registry]` in existing configs. Its fields,
including `component` and `max_instances`, carry over.

Dedicated instance pools now support fleet-selected Auth receivers. Host binds
the declared parent, application role and install bytes to the receiver policy.
Root checks the same installation on approval and when reopening the grant.

```sh
# With a bound local Root, registry and Store, inspect the exact instance approval.
canic fleet plan --app toko --fleet local --controller "$CONTROLLER" --json
```

The Miner Pool keeps the job that requested a worker across a Root outage,
a hub upgrade and a change of goal. Once the worker is ready it runs that
batch once, even if the next goal parks the worker.

```sh
# From a fresh local Miner Pool fleet, use its configured operator identity.
canic fleet call --canister pool --identity "$IDENTITY" tick 1 1
canic fleet call --canister pool --identity "$IDENTITY" tick 2 0
# Repeat the second tick until it reports settled.
```

Fleet declarations accept `top_up_children = ["project_registry"]` to select which
direct children may receive parent-funded top-ups. Omitted children receive no
such permission. This declares policy; cycle transfers and reserve reimbursement
are tracked in [#78](https://github.com/dragginzgame/canic/issues/78).

```sh
# Inspect the selected fleet's declared funding policy.
canic fleet canister list
```

Use the `receiver-role` feature for hubs without IC DB. Database-free hubs can mount directory, receiver lifecycle and readiness together,
select session endpoints separately, and supply IC-named lifecycle hooks.
Receiver callbacks can require the installed issuer directly. [Canic #77](https://github.com/dragginzgame/canic/issues/77)

## [0.254.0] - 2026-11-22

### 🌳 Fleet growth and named pools

Fresh Roots can host up to eight permanent services. Adding a service preserves
the slots and upgrade targets already approved for its siblings. Permanent
services and placement pools share the subnet ceiling. Roots with the old
two-service journal require their original executable. [Canic #75](https://github.com/dragginzgame/canic/issues/75)

### ▶️ Try it

```sh
# From the configured fleet, inspect the declared children and retained deployments.
canic fleet canister list
canic fleet show
```

`canic::topology::get_shard(pool_name, principal)` routes unseen principals among a hub’s acknowledged shards in that exact pool. The pool name is the approved shard component. Root publishes it in directory delivery v2. The v3 index records retired generations, requires a fresh install and retains the existing wallet scoring. [Canic #71](https://github.com/dragginzgame/canic/issues/71)

Declare shared placement grants with repeatable `[[apps.<app>.fleets.<fleet>.shard_pools]]` tables. Assignment and lookup carry the component name and key. Root’s new placement layout requires a fresh install. Confirmed worker returns free live capacity while retaining the original ordinal and creation debit. Root waits for the former hub to confirm withdrawal before reinstalling a retired shard.

A hub names the child directly at the call site with `canic::shard_pool!("discovery_shard")` or `canic::worker_pool!("miner")`. Canic supplies the selected hub’s children from its app definition. Unknown names fail compilation. Whole-fleet planning reads the resulting Wasm requirements and checks pool kind and parent. Declare worker allowances in repeatable `[[apps.<app>.fleets.<fleet>.worker_pools]]` tables. Each component has independent ordinals, count and spending bounds. Worker calls now carry the component and ordinal. Roots with the old single-worker-pool ledger require their original executable.

```sh
# From the Miner Pool example, build its checked hub. The local walkthrough uses synthetic cycles.
canic app build
# From the repository root, qualify named worker pools against installed canisters.
make miner-test
```

Plain endpoint mounts now check that the shared method does not require role attestation. Host inspection exposes the request and reply codec limits for application and built-in mounts.

Root logs when an approved install finds the configured reserve pool empty and schedules a paid creation. When returns or imports take the pool above target, Root logs the surplus and keeps the canisters available.

```sh
# In a configured fleet, inspect the ready count and refill status.
canic fleet show
```

Directly created workers can now enter the subnet reserve pool after their hub releases them and Root verifies the reset. The original creation still counts against the worker grant.

Root rechecks the original operator after observing a worker release or reset. If control changes while an observation is pending, the successor can resume from the last committed phase.

Root build recipes can select the crates.io package locked by the consumer workspace. The Wasm and native operator client use the same source and recipe features. Large Root builds retain stripping and use size optimization for Root and Management. [Canic #72](https://github.com/dragginzgame/canic/issues/72)

Conventional hubs with role attestation now attach their outgoing proof cache on install and reopen. Use `call_service_by_role` for a unique published role. The passive declaration is now `#[canic::attested_method(roles = …)]`; the receiver references it with `#[canic::update(attested_method = …)]`. [Canic #73](https://github.com/dragginzgame/canic/issues/73) · [Canic #74](https://github.com/dragginzgame/canic/issues/74)

```sh
# From a consumer with a locked Root recipe, build both artifacts and inspect the Wasm.
canic app build --canister root
canic artifact endpoints artifacts/root.wasm
```

Updated IC DB and its schema macros to 0.26.11 in the product and qualification workspaces. The optional Canic DB adapter keeps its existing API.

## [0.253.0] - 2026-11-21

A hub can call `Index::acknowledged_children(role)` to enumerate its direct children locally, including unnamed shards that confirmed directory delivery. The lookup survives reopening the index. [Canic #71](https://github.com/dragginzgame/canic/issues/71)

Set `standby_pool.min_available_spares` once for the subnet fleet. `canic fleet show` and `canic fleet plan` display that configured target. Fleet show also queries Root for ready spares and refill status. All Root-managed child installs claim a ready spare before issuing a paid create, subject to their existing placement and install approvals.

A fleet’s `standby_pool` can keep a minimum number of available canisters,
bound automatic creations by count and expiry, and accept named surplus
canisters after verified handoff and a two-hour quarantine. The
[Miner Pool instructions](apps/examples/miner-pool/DESIGN.md#keeping-spare-canisters-ready)
cover configuration, approval and imports. Each TOML import group declares its
network and an ordered array of canister principals. When the Registry places a candidate on a different subnet, Root reports the candidate, its subnet and the required subnet. Import errors also identify installed code, a stopped candidate or the cycle shortfall.

```sh
# Review a fleet with standby_pool configured, then inspect its installed Root.
canic fleet show
canic --json fleet plan --identity "$IDENTITY"
canic fleet call --canister root --identity "$IDENTITY" inspect-standby
# Import an approved surplus principal into the selected local fleet.
canic fleet call --canister root --identity "$IDENTITY" import-standby "$PRINCIPAL"
make miner-test  # Installed reserve, growth and handoff on the local simulator.
```

Two same-subnet fleets can hand off an empty spare. First approve its import
on the destination with the source as `previous_root`. Use each fleet's own
controller identity and the same absolute dispatch deadline on every export.

```sh
canic fleet call --fleet "$SOURCE" --canister root --identity "$SOURCE_OWNER" export-standby "$SPARE" "$DEST_ROOT" "$DEADLINE_NS"
canic fleet call --fleet "$DEST" --canister root --identity "$DEST_OWNER" import-standby "$SPARE"
canic fleet call --fleet "$SOURCE" --canister root --identity "$SOURCE_OWNER" export-standby "$SPARE" "$DEST_ROOT" "$DEADLINE_NS"
```

A role-enabled Root can return a drained shared shard to its fleet’s standby
pool. The controller approves the original allocation, dormant Store release,
minimum balance and deadline. Repeat the exact approval to advance or observe
the operation. A worker can then reuse the principal under a fresh generation.
The `inspect_standby` Wire contract moves to v3 to include transfer progress and larger retained inventories. Each inventory row identifies
its source (manual refill, automatic refill, external import or retired shard). Rebuild its clients with Root.

```sh
# On a role-enabled fleet, after drain, exact placement release and issuance closure.
canic fleet call --canister root retire-shard "$APPROVAL"
canic fleet call --canister root inspect-standby
# Exercise the full handover with the prepared local simulator.
bash scripts/dev/test-store.sh directory
```

Fresh Roots use one journal format (CUJC) as the fleet grows. An upgrade from
a pre-cut journal is refused. Keep that Root and its outstanding operations
under its original executable. Start a new fleet with a fresh Root.

```sh
# With Rust and the prepared local simulator, check refusal and fleet growth.
bash scripts/dev/test-root.sh storage_reopen::
bash scripts/dev/test-store.sh sharding
```

Canic and its qualification workspace use IC DB 0.26.8. When a build selects
both Canic endpoint inspection and the IC DB adapter, inspection can describe
DB types such as `ic_db::Ulid` and `ic_db::QueryCursor` through IC DB’s Wire feature.

```sh
canic app build --config apps/examples/user-directory/canic.toml
make check-wire-endpoints  # Verify the reviewed hub and shard contracts.
```

## [0.252.0] - 2026-11-21

DB-backed Auth receivers now use one Wasm for both fleet storage policies.
Choose `storage = "volatile"` (default) or `"stable"` in `receiver_policy`
before installation. Stable sessions require a Root refresh after upgrade.
Custom receiver mounts now use `storage = open` in place of `stable = open`.

```sh
# Build the same hub and shard for either policy.
canic app build --config apps/examples/user-directory/canic.toml
# Exercise both on the prepared local simulator.
bash scripts/dev/test-store.sh user-directory
```

Root can return a released Miner Pool worker to standby and install it again
under a fresh allocation generation. It checks the actual controllers, module,
snapshot inventory, snapshot memory usage and cycle balance. A returned worker
retains one physical subnet slot. A hub with zero creation budget can claim it.
Root rejects changes to the minimum return balance on a retained grant.

```sh
canic app build --config apps/examples/miner-pool/canic.toml
make miner-test   # Exercise release, underfunding, return and same-principal reuse on the local simulator.
# In an installed fleet, repeat with the same target and deadline through Returned:
canic fleet call --canister root reset-worker "$TARGET" "$DEADLINE_NS"
```

## [0.251.0] - 2026-11-21

Auth Wire 0.47.0, Blob Storage 0.109.0 and Wasm Store 0.7.0 now use IC Wire 0.28.0 in both workspaces. Store 0.7.0 requires a fresh Store and rebuilt manifests for its 1 MiB chunks. Keep existing 256 KiB fleets on their qualified release.

Receiver mounts accept an optional `max_login_secs` ceiling for portable logins. The default follows IC Auth’s starting profile.

Fresh DB-backed hubs and shards can preserve browser sessions across upgrades.
Build each receiver with `canic/auth-receiver-stable` and set its fleet receiver
policy to `storage = "stable"`. Refresh each receiver after upgrading.
[Auth setup](docs/demos/auth.md) · [Canic #70](https://github.com/dragginzgame/canic/issues/70)

```sh
# Build the example roles locally and exercise both storage modes on a prepared simulator.
canic app build --config apps/examples/user-directory/canic.toml
bash scripts/dev/test-store.sh user-directory
```

### ▶️ Try it

After `make qualification-setup`, run either disposable fleet in one terminal.
In Miner Pool, enter `miners 3` to see live attempts with a 25% find rate.
Enter `miners 1` to park workers, or `miners 0` to drain and idle the pool.

```sh
make demo DEMO_APP=miner-pool   # Set the active count and watch miners find blocks.
make demo DEMO_APP=notebook     # Checkpoint a note, restore, accept and reopen.
```

A sharding Root can retain an operator-funded empty canister and supply it to
the next approved worker. Each miner checks its allocation generation on every
batch. A parked miner can resume work. Explicit release permanently withdraws
the hub's route. Root retains the released allocation pending cleanup.

```sh
# From a configured miner fleet with the corresponding controller/operator identities
canic fleet call --canister root inspect-standby
canic fleet call --canister pool report
```

## [0.250.0] - 2026-11-21

### ✨ Features

A caller can import `canic::method_descriptor as update` to derive its Wire contract
from a guarded receiver’s handler. The Miner Pool hub now shares the miner’s
`mine` declaration. [#69 · Support typed caller descriptors for guarded Wire handlers](https://github.com/dragginzgame/canic/issues/69) · [Caller setup](docs/wire-ci.md#share-a-guarded-endpoint-with-a-rust-caller)

```sh
# Build both actors, then exercise parent-only dispatch on the prepared local simulator.
make miner-test
```

## [0.249.0] - 2026-11-21

Attested queries and updates can declare `roles = &[PROJECT, PROJECT_REGISTRY]`
directly on the handler. Existing typed clients keep the same proof frame.
A sender can derive its contract from the same endpoint source in a passive
protocol crate. [Sharing inline endpoints](docs/wire-ci.md#share-an-inline-attested-endpoint-with-a-rust-sender) · [Canic #68](https://github.com/dragginzgame/canic/issues/68)

Host now strips symbols from Blob-enabled Root builds before hashing the Wasm for approval.
This keeps the Blob qualification Root within the direct-install size limit.

Typed application calls can select an unbounded wait with
`canic::method::call_unbounded::<M>(target, &input).await`. Existing bounded
calls keep their timeout. Both submit once. [Canic #67](https://github.com/dragginzgame/canic/issues/67)

For Toko’s `apps/toko/user_hub/src/ops/outbound.rs`, define shard contracts in
the shared protocol for wallet lookup, authority lookup, membership add and
membership remove. Keep each deployed shard name and numeric ID. Use
`USER_LOOKUP_REQUEST` for lookups, `PROJECT_MEMBERSHIP_REQUEST` for writes and
`USER_HUB_REPLY` for replies. For a wallet contract named `WalletByUser`, the
replacement is `call_unbounded::<WalletByUser>(route.shard, &route.user_id).await`.
Map request/reply codec errors to Toko’s `ServiceUnavailable` and transport
errors to `IcCallFailed`, then remove the generic encode/dispatch/decode helper.
Toko still selects the shard and the membership operation.

A role-protected Wire method can accept several caller roles under one export and
method ID. Change `const ROLE: &'static str = PROJECT;` to
`const ROLES: &'static [&'static str] = &[PROJECT];` and add another role only
when it has the same authority over the request. An empty slice denies every
caller. Existing proof and method frames keep their encoding. [Canic #66](https://github.com/dragginzgame/canic/issues/66)

```sh
# With the local simulator prepared, exercise the installed sender and receiver.
bash scripts/dev/test-store.sh directory
```

## [0.248.0] - 2026-11-21

Canic endpoint attributes and built-in mounts now resolve Platform through one
hidden macro path. Applications using a former Canic runtime re-export should
declare `ic-platform-runtime` directly and import `ic_platform_runtime`.
Canic supplies the runtime path for generated query and update endpoints. [Canic #65](https://github.com/dragginzgame/canic/issues/65)

An opted-in Root now issues receiver-bound role proofs to ready dedicated
instances and permanent named services. Its existing two-hour proof lifetime
and upgrade checks also apply to these callers. They use their original install
generation. Shard retirement remains specific to shared shards. [Canic #64](https://github.com/dragginzgame/canic/issues/64)

Build the Miner Pool fleet, then run six local rounds with synthetic cycles.
The hub requests workers under its own approved grant and parks them for reuse.
Run `make qualification-setup` once to prepare the simulator.

```sh
canic app build --config apps/examples/miner-pool/canic.toml
make miner-test
```

On a bound local miner fleet, the installed pool operator can advance one step
of round one toward five blocks. Its reply identifies an outstanding Root
allocation by worker ordinal or an outstanding miner batch by round and ordinal.

```sh
canic fleet call --canister pool --identity operator tick 1 5
```

The guarded metrics reader now reports database operations, job outcomes and
supplier-managed fresh entropy requests. For a deployed child with the
corresponding recorder, use its controller Root and approved slot:

```sh
canic fleet call --canister root --identity operator child-metrics 0 db
canic fleet call --canister root --identity operator child-metrics 0 jobs
canic fleet call --canister root --identity operator child-metrics 0 entropy
```

Build the Asset Library or run its two-user walkthrough on the prepared local simulator.

```sh
canic app build --config apps/examples/asset-library/canic.toml
make asset-library-test
```

Build summaries and fleet placement reports group canisters under their declared
parents. A filtered build names an omitted parent beside its child.

```sh
canic app build --config apps/toko/canic.toml --app toko
```

## [0.247.0] - 2026-11-21

Config validation reports the fleet, field, supplied value and supported limit
for rejected pool settings. Use the app listing to validate declarations before building.

```sh
canic app list --config apps/toko/canic.toml
```

Fresh sharding Roots support a configurable ceiling up to 100,000 live keys per
shard. Toko can select `keys_per_shard = 10_000` and `max_shards = 2`.
The indexed layout requires a fresh sharding Root. Upgrading an old inline
ledger to this layout refuses. Placement counts in the drain and status APIs
are now 32-bit.

```sh
# Inspect a configured fleet's shard policy
canic fleet placements
```

Check an application's mounted Wire contracts against its built Wasms and reviewed
schemas. The generated starter includes the inspector and its initial baseline.

```sh
# From a configured app with its inspection examples and baseline
canic app check-wire
```

See the [Wire CI guide](docs/wire-ci.md) for existing apps and baseline review.

## [0.246.0] - 2026-11-21

### 💥 Breaking changes

Canisters with `canic/metrics` expose one `metrics` query. Its typed request
selects timers, Blob command outcomes or a page of endpoint measurements.
`canic/endpoint-metrics` enables the handler recorder. The default guard admits
actual canister controllers. An app can set one guard for the entire query
with `canic::metrics::set_guard(check)` on install and reopen. This changes the
Wire request and reply and retires the separate `endpoint_metrics` query.
Anonymous fleet inspection reports protected Root metrics as unavailable.

```sh
make demo DEMO_APP=metrics
# At the prompt, run ping hello, then metrics.
```

Mount callbacks resolve in the declaring module. Pass local functions directly, for
example `mount_readiness_endpoint!(readiness)`. Volatile Auth lifecycle decoders
now receive the full install envelope through `decode =`; use its `application`
field for app bytes. A receiver storage prefix defaults to its authority followed
by a dot. Keep an explicit prefix if your retained keys use another namespace.

```sh
# Exercise the installed receiver and timer query on the prepared local simulator.
make auth-test metrics-test
```

Both workspaces select the registry Wasm Store runtime and protocol 0.6.0.
Store and Canic now share the IC Wire 0.27 budget types.

```sh
canic artifact build --package ic-wasm-store
```

`mount_db!` replaces `mount_db_role!` and takes `schema =` with the canister type. Its compiled
roster selects every store and `db!()` routes by entity. For an existing
single-store role, keep the original schema key. Move its retained cell keys,
storage policy and Auth read scope into `mount_receiver_lifecycle!`. Database-only
apps use `mount_db!(schema = …)` with their own lifecycle.

```sh
# Build the example hub and shard with their schema-defined stores.
canic app build --config apps/examples/user-directory/canic.toml
# On the prepared local simulator, exercise the two-store upgrade.
bash scripts/dev/test-store.sh db-roles
```

## [0.245.0] - 2026-11-20

Packages selecting `canic/entropy` can use the consolidated ic-entropy 0.6.0 crate with the shared
metrics and logging hooks. Call `canic::entropy::supplier::fresh_seed()` for
a fresh acquisition. On the prepared local simulator:

```sh
make db-test  # installed package composition, including Entropy after reopen
```

Root and application packages now share `canic::lifecycle::initialize_timers()`.
Select `timers` or `jobs`, and add `metrics` to record timer activity. Canic
lifecycle mounts initialize diagnostics automatically. Start timers explicitly
before binding application work.

```sh
# Installed Root timer accounting before and after upgrade
bash scripts/dev/test-root.sh metrics::
```

The embedded Blob adapter uses IC Blob Storage 0.108.0. Selecting `blob` with
`metrics` or `logging` attaches the service to the hosting canister’s diagnostics
on install and reopen. Run `make blob-test` on the prepared simulator.

Canic lifecycle mounts attach selected DB and Jobs metrics to the canister’s
shared store and forward DB diagnostics to its shared log when logging is enabled.
Run the package-composition witness on the prepared local simulator:

```sh
make db-test
```

Both Cargo workspaces pin Toko Tooling 0.46.0. Verify the installed snapshot
with `make toko-tooling-check`.

Any actor can select `canic/role-attestation` and call a named service through
`canic::topology::call_service`. A fresh DB hub or shard gets the same wiring
from `mount_db_role!`; a plain store needs only `canic/db-store`.
For a resolved Root built with `auth,role-issuer` and its command codec, retain
the signer’s public key before planning named DB receivers. Each hub needs an
Auth policy, a `role_receivers` entry and a 16-byte nonzero installation nonce.

```sh
canic fleet call --canister root prepare-role-key
# Installed DB shard → named project hub, using the prepared local simulator
bash scripts/dev/test-store.sh db-roles
```

Unbound local fleets can opt into `lifetime = "ephemeral"`. The Canic Host `ephemeral` feature runs installed tests with a dedicated Testkit simulator and temporary journal.

Root now requires IC Timers 0.46.13 for its shared timer metrics storage. Run
`make metrics-test` for the example and `bash scripts/dev/test-root.sh metrics::`
for timer reporting across a Root upgrade.

The local demo builds and deploys its fleet before opening an application prompt in the same terminal. Metrics automatically counts completed synchronous Canic updates and reports latest, average and maximum handler instructions by endpoint. The fleet operator reads them through Root. The app recipe selects `features = ["canic/endpoint-metrics"]`. Canic supplies the guarded reader automatically. The builder checks its module and query export before replacing the Wasm. Both interactive demos offer the same `metrics` and `logs [before]` commands for the application’s endpoint measurements and Root’s operational log.

```sh
make demo DEMO_APP=metrics
# At metrics> enter each command
ping hello
checksum hello
metrics
# Root controller: read newest operational log entries
logs
exit
```

Canic #61 changes the shard provisioning reply to `Result<Binding, Refusal>`.
The hub can return an application refusal while retaining the original reservation
for uncertain calls. Upgrade hub and shard together. With a prepared simulator,
run the installed User Directory example:

```sh
bash scripts/dev/test-store.sh user-directory
```

Rename the provisioning selector `Account` to `AuthSubject`. Import
`canic::sharding::provisioning::AuthSubject` for the issuer and subject
used at signup. Ledger accounts continue to use IC Platform’s `Account`.

```sh
# Build the example hub with the renamed provisioning type.
canic app build --config apps/examples/user-directory/canic.toml --canister ex-user-directory-hub
```

On a Root built with `role-issuer`, the controller can close proof renewal
for an empty, drained shard. Use the generation returned by `inspect-shard`.
The reply reports the original deadline as a decimal string in IC nanoseconds,
including on retry after a Root upgrade.

```sh
canic fleet call --canister root --identity operator inspect-shard "$SHARD"
canic fleet call --canister root --identity operator close-role-issuance "$SHARD" "$GENERATION"
```

Build the supplied dormant canister for local retirement qualification. The
artifact records the source Root, retired principal, generation and proof deadline.

```sh
canic artifact build --package canic-dormant
```

## [0.244.0] - 2026-11-20

An opt-in Root can issue receiver-bound, two-hour role proofs to its ready shared
shards. Root retains its Ed25519 key and the renewal deadlines for closed assignments
across same-layout upgrades. In a fresh Store-backed fleet, set
`role_receivers = ["accounts_hub"]` in the fleet table and build its Root with
`role-issuer`. The selected receiver opens the install pin and mounts methods
with `#[canic::update(role_contract = M)]` or `#[canic::query(role_contract = M)]`.
The sender’s `RoleCalls` renews on demand. Both endpoints use the role declared
by the shared method. The controller can call `begin_shard_drain` to stop new placements
and inspect the remaining count. Existing accounts keep their routes and
the shard can renew its proof during drain. After committing its application
change, the approved hub calls `ReleaseShard` for the exact key and allocation.
Closing issuance requires an empty draining shard.

```sh
# With built fleet artifacts, review the Root and selected receiver.
canic fleet plan --identity operator
# Use the displayed digest to deploy or resume with the same key.
canic fleet apply DIGEST --identity operator
```

For a deployed shard pool with the Root client built and a controller identity,
inspect the shard first. Use the slot and generation from that reply to start
the drain. Inspect again to see the current count.

```sh
canic fleet call --canister root --identity operator inspect-shard "$SHARD"
canic fleet call --canister root --identity operator drain-shard "$SHARD" "$SLOT" "$GENERATION"
```

Root now rejects a shared-shard grant when its direct Canic topology frame
names a different Root or parent hub. On upgrade, both placement pool types
cross-check their retained grants against the installed arguments, Store release,
allocation generation and ready principal. Existing consistent records keep
their format. A conflicting retained record stops reopening.

```sh
# Inspect the declared pool before submitting its operator approval.
canic fleet placements --help
# With the local simulator prepared, exercise grant refusal and Root reopen.
bash scripts/dev/test-store.sh directory
```

Application endpoints can guard a method with a fixed fleet service name. The
guard runs before request decoding and composes with ordinary Rust
`Result::and_then` and `Result::or_else`. Existing zero-argument guard paths
keep their syntax. The experimental Root `peers` feature, operator-managed peer
grants and their storage layout are retired. Use fresh installations for fleets
that tried that feature.

```rust
use canic::topology::require_canister;

#[canic::update(guard = require_canister("project_hub"))]
fn project_sync(request: ProjectSync) { /* application operation */ }
```

```sh
# With the local test server prepared, exercise named callers and composed guards.
bash scripts/dev/test-store.sh directory
# Inspect the witness endpoints produced by that run.
canic artifact endpoints target/store/directory.wasm
```

## [0.243.0] - 2026-11-20

### ✨ Features

Internal coordinators can call `canic::auth::session::authorize("write")?` to
obtain a scoped authorization or a typed refusal. Call `auth.reauthorize()?`
after each await before recording progress or starting another protected effect.
[Canic #59 — Fallible scoped authorization for internal async operations](https://github.com/dragginzgame/canic/issues/59).

```sh
# Exercise admission and logout during an outstanding call on the prepared simulator.
bash scripts/dev/test-auth.sh stable_session_and_logout
```

Inspect callable IC exports and their modes before deployment:

```sh
# Inspect a built Wasm artifact.
canic artifact endpoints WASM_FILE
```

The opt-in `peers` Root build retains grants and revocations for a named sender,
receiver and method. `observe_peer` reports pending until the receiver acknowledges
the exact decision. Retry the same `change_peer` after the receiver restarts.

Applications can look up a published service by its exact name with `Directory::by_name`, even when two hubs share a role.
Applications can resolve one target with `Directory::unique_by_role`.
A missing role and a role shared by several services give distinct refusals.
The installed demo exercises a hub reopening with its previous directory,
then catching up through an operator replay while Root stays online.
Root now refuses a permanent child install whose topology frame names another
installer or a parent absent from its ready installation records before reserving a slot or issuing a creation call. Topology-enabled
pools now require an explicit component role and fresh installs. The v2 install
frame binds the role and parent. Directory recipients retain their own role,
check it on delivery and forward the approved branch to unnamed children.
Each parent retains the revision acknowledged by each child across upgrades.
Recipients reject directory revisions that remove or change a published hub's name, role or principal.
Adding an unnamed shard or project sends the current directory only along its branch.

```sh
# Include the peer revocation and held-callback journey.
# Run the directory journeys on the prepared local simulator.
bash scripts/dev/test-store.sh directory
```

## [0.242.0] - 2026-11-20

New starter apps declare their endpoints in `src/lib.rs`. Native builds now
type-check Root and its management engine. Native actor inspectors can enable
`canic/inspection` to read the Wire contracts behind app, Auth and readiness
methods. The User Directory check compares those contracts with both built actors.

```sh
# Create a fresh starter and build its declared artifacts.
canic app init sample --source ./sample
canic app build --config ./sample/canic.toml
```

Scoped queries and updates now take only their Wire arguments. Use the local
`auth` binding for the admitted issuer and subject and pass `&auth` to
provisioning. The User Directory exercises registration while a shard reply is held.

```sh
# Build the actors, then exercise private reads and registration on the prepared simulator.
canic app build --config apps/examples/user-directory/canic.toml
bash scripts/dev/test-store.sh user-directory
```

When a named service first becomes ready, Root automatically delivers the new
directory to all ready named hubs. Root replays the current revision on upgrade.
An operator can retry a missed delivery to one recipient. Run the installed
directory tests with synthetic cycles and a prepared local simulator. They also
exercise an unnamed child receiving its hub’s directory under a Root-approved
parent binding. With an approved subnet ceiling, direct installations and both
placement pools share one child count. A pending creation consumes a slot.
Retries and Root upgrades preserve the original reservation.

```sh
bash scripts/dev/test-store.sh directory
```

## [0.241.0] - 2026-11-20

IC DB 0.24.0 moves schema compilation into the schema owner. In `mount_db_role!`,
replace the literal database key with the store type declared in that crate.
Give its `#[store]` the existing allocation key. The User Directory now has one
schema build script shared by both actors.

```sh
canic app build --config apps/examples/user-directory/canic.toml
bash scripts/dev/test-store.sh user-directory
```

Canic now uses IC Wire 0.27.0 directly. Blob and DB share the request framing
and reply encoder while retaining their own budgets and authorization.

Removed the obsolete Rust constant `stored::SHARD_SLOTS`. Use `ChildId::SHARDS`
for the ordered shard identities. The fleet grant determines how many are eligible.
Shard-only and combined fleet previews now use the same text layout for each
approval, including its release, budget and deadline. Preview a configured fleet
or run the installed growth witness on a prepared simulator.

```sh
canic fleet plan --config apps/toko/canic.toml --controller "$CONTROLLER"
bash scripts/dev/test-store.sh sharding
```

## [0.240.0] - 2026-11-19

### 🚀 Features

A placement Root can admit up to eight managed children across its subnet’s
shared-shard and dedicated-project pools. A live count increase preserves the
original creation budget, expiry, assignments and outstanding work.

For a bound fleet with built artifacts, set `subnet_child_limit` on the fleet
and the desired count on each pool. For an existing pool, set `increase_from`
to its current approved count. Preview the requests locally, or run the
installed upgrade and contention test on a prepared simulator.

```sh
canic --json fleet plan --config apps/toko/canic.toml --controller "$CONTROLLER"
bash scripts/dev/test-store.sh sharding
```

## [0.239.0] - 2026-11-19

### 🚀 Features

A sharding Root can retain independent controller approvals for a user hub
and a project hub. A bound fleet with built Wasm and future grant deadlines
can preview its `shard_pool` and `instance_pool` together. Set `CONTROLLER` to
the Root controller principal. Planning reads local inputs and prints the exact
requests. The last command runs the installed regression on a prepared simulator.
The approved project hub can now use `canic::sharding::assign_instance` and
`lookup_instance` to recover the same dedicated child after reply loss or a
Root upgrade. One Root supports up to two dedicated project children alongside
both shared shards. [#52 · Dedicated project placement alongside user shards](https://github.com/dragginzgame/canic/issues/52)

```sh
canic fleet plan --config apps/toko/canic.toml --controller "$CONTROLLER"
canic --json fleet plan --config apps/toko/canic.toml --controller "$CONTROLLER"
bash scripts/dev/test-store.sh sharding
```

App builds report parent slots, compact Wasm sizes, compiled DB and Auth mounts,
and elapsed build time. The command codec column appears when a recipe produces
a native codec. JSON includes exact artifact bytes and SHA-256, parent, reported
mounts and elapsed milliseconds. Older Wasm builds without mount markers show
a dash in the modules column.

```sh
canic app build --config apps/template/canic.toml
canic --json app build --config apps/template/canic.toml
```

### 🧪 Testing

CI setup now uses Toko Tooling’s shared diagnostic excerpts. Failed phases
retain the original exit status and up to four explicitly reported attempt
logs, even when the report scrolls out of the console tail. [#49 · Retain CI setup diagnostics](https://github.com/dragginzgame/canic/issues/49)

### ⚠️ Breaking changes

`#[canic::query]` and `#[canic::update]` derive the Wire ID from the exported
name. To keep a fixed ID, declare it once on a shared `Contract` and select
`contract = MyContract` on the handler. The standalone `method = ...` option
has been removed.

Build the template locally or exercise its fixed-ID greeting protocol with a
prepared simulator.

```sh
canic app build --config apps/template/canic.toml
make journey-test
```

## [0.238.0] - 2026-11-19

Receiver enrollment no longer blocks application upgrades. Volatile sessions close on upgrade. Fresh Auth installs can select stable Ed25519 receiver and issuer checkpoints.
With a prepared simulator, run the installed upgrade and logout regression:

```sh
make auth-test
```

The optional Auth and Blob adapters now use Auth Wire 0.45.0 and Blob Storage 0.107.1. Both workspaces resolve a single IC Wire version (0.26.1).

Build the local Notebook and restore a fenced draft from a stopped-canister snapshot. The installed walkthrough checks the 256-byte edit limit, upgrade persistence and separate acceptance and reopening.

```sh
canic app build --config apps/examples/notebook/canic.toml
make notebook-test
```

## [0.237.0] - 2026-11-19

User Directory now uses ordinary application rows and a checked placement handle
for registration. Build the two actors locally and run the installed retry and
revocation cases with a prepared simulator.

```sh
canic app build --config apps/examples/user-directory/canic.toml
bash scripts/dev/test-store.sh user-directory
```

## [0.236.0] - 2026-11-19

The workspace allows blank lines after Rust doc comments for spaced type headers.

Counter is the starter example. Greeting has moved into the private CLI qualification workspace.

Counter uses Canic’s default 512-byte Wire budget for both its endpoint and native CLI codec. The
user-directory hub and shard use `provisioning::session_row` for scoped account reads.

Counter uses the zero-argument form of `canic::mount_lifecycle!` for install and upgrade.

The counter groups its endpoints in `src/endpoints.rs`. The user-directory
roles group theirs at the top of each library root. Their headers identify access and payload
limits. Build an example locally from the repository root:

```sh
canic app build --config apps/examples/counter/canic.toml
```

## [0.235.0] - 2026-11-19

Both workspaces now use IC DB 0.23.17 and Toko Tooling 0.45.19.

Mounted roles can use `db!()` to obtain independent handles to their attached
IC DB database. The user-directory hub demonstrates a compound-index lookup
from an ordinary operations module. Build it from the repository root:

```sh
canic app build --config apps/examples/user-directory/canic.toml
```

Hub registrations can pass `canic::sharding::provisioning::new_ulid` to
`provision` to select the standard IC DB generator.

Canic builds IC Wasm Store 0.5.0 from crates.io and uses the matching protocol’s
Wire limits directly. From the Canic checkout, build and inspect the Store locally:

```sh
canic artifact build --package ic-wasm-store --output target/wasm-store.wasm
canic artifact inspect target/wasm-store.wasm
```

## [0.234.0] - 2026-11-19

Both workspaces now use Toko Tooling 0.45.18 and registry IC DB 0.23.15.
IC Timers supplies IC Metrics 0.30.17 for Root’s timer counters.
Canic and qualification use IC Wire 0.26.0. The Blob adapter takes all four
Wire budgets from the supplier protocol.
Shared issuer DTOs use the Wire derive on native and Wasm whenever the
`auth-browser` feature is enabled.

Canic actor mounts, caller guards, scoped endpoints and shard provisioning now
type-check on native targets using registry Platform Runtime 0.23.21 and IC DB.
Fresh entropy forwards the caller’s bounded wait to Platform in a single attempt.
Root’s four Auth enrollment endpoints also type-check on the host. IC authority
and effects still require canister execution.

Retired the `ex-backup-counter` example fleet. The installed Backup test now
builds its Root and counter artifacts directly.

Native rust-analyzer now keeps handwritten `#[canic::query]` and
`#[canic::update]` signatures active. The IC wrappers still compile only on
Wasm. [#46](https://github.com/dragginzgame/canic/issues/46)

Example Cargo names keep their `ex-` prefix. Single-crate examples build from
their fleet directories. The user directory keeps its components in `hub/`,
`shard/` and `schema/`. Provisioning resources inherit the sharding Wire budget
unless the application supplies a different limit.

```sh
cd apps/examples/user-directory
canic app canister list
canic app build --canister ex-user-directory-hub
```

## [0.233.0] - 2026-11-19

### 🧱 Actor declarations

Hub and shard declarations in `mount_db_role!` now expose their storage settings
and Auth read scope through the generated `database` module on native and Wasm
targets. Only the IC lifecycle and endpoint wiring is Wasm-specific.

Example app names, application canister slots and Cargo packages use the
`ex-` prefix. From the repository root, list the examples and build the counter:

```sh
cd apps/examples
canic app list
cd counter
canic app build --canister ex-counter
```

## [0.232.0] - 2026-11-19

### 🎯 Qualification

Grouped sibling test suites under one guard per parent ([#45](https://github.com/dragginzgame/canic/issues/45)).
Both workspaces adopt Toko Tooling 0.45.16. Provisioning uses the admitted
unique account index for a bounded one-row lookup on IC DB 0.23.14.

### Application endpoints

The Hello World example and starter own their greeting endpoints. The public
`canic/greeting` feature and its fixed mount have been removed. From the repository
root, build and inspect the example locally:

```sh
cd apps/examples/greeting
canic app build
canic artifact inspect artifacts/greeting.wasm
```

## [0.231.0] - 2026-11-19

Both workspaces adopt Toko Tooling 0.45.13. Example package directories now
match their Cargo names. Each fleet keeps its existing config and build commands.

The [example fleets](apps/examples/) each have a buildable config and a local exercise.
The persistent counter moved from `apps/demo` to `apps/examples/counter`. The Backup
example selects its Root plugin with `build.features = ["backup"]`. With the current
CLI installed, these commands list the apps and build the Backup example locally:

```sh
cd apps/examples
canic app list
cd backup-counter
canic app canister list
canic app build
```

The fourth example uses an Auth-enrolled hub and a pool of user shards.
From the repository root, inspect the roles and run the installed signup exercise
with the local simulator prepared:

```sh
canic app canister list --config apps/examples/user-directory/canic.toml
bash scripts/dev/test-store.sh user-directory
```

## [0.230.0] - 2026-11-19

### 🔧 Changed

Account provisioning now uses ordinary Rust functions with the application's
shared IC DB entities. Replace `mount_provisioning!` and
`declare_provisioning_row!` with the hub or shard mapping and a call to
`provision` or `receive`. Keep the role’s existing database and lifecycle
mount. See the [worked example](qualification/README.md#provision-an-account-on-its-assigned-shard).

New apps and the greeting demo compile their application handlers on native
and Wasm targets. Canic endpoint attributes keep the handler visible to Rust
Analyzer and generate the IC export on Wasm. The generated Wire codecs and
provisioning store accessors also compile natively. Both workspaces adopt Toko
Tooling 0.45.11 and IC DB 0.23.12 (minimum 0.23.11). Provisioning still examines at most one
candidate and rejects encoded output over 1 KiB after IC DB decodes it.

From a configured app directory, build the fleetless app with `canic app build`.
To check the starter handlers in the Canic checkout:

```sh
cargo check -p template
canic app build --config apps/demo/canic.toml --app demo
```

## [0.229.0] - 2026-11-18

### ✨ Added

`canic::mount_provisioning!` mounts a conventional ULID hub or shard from one
declaration. The app keeps its private rows, validation and retained storage
layout. A typed `ProvisioningRole` selects behavior. App builds supply the
configured role name and standalone builds can use a Rust constant.
The shard validates retries before acknowledging an existing record.
[Complete hub and shard declarations](qualification/README.md#provision-an-account-on-its-assigned-shard)

From a consumer app directory using this revision, build the two declared actors:

```sh
canic app build --canister user_hub
canic app build --canister user_shard
```

### 🔧 Changed

The Host and endpoint macros use Syn 3.0.10, already selected in both workspaces.
The DB adapter requires IC DB 0.23.10. Version 0.23.11 removed the production
query byte limit used for provisioning. Revisit this constraint when a supplier
API can enforce the same 1 KiB bound.

## [0.228.0] - 2026-11-18

### 🔧 Changed

Hub and shard provisioning rows can be declared directly in `lib.rs` using
qualified schema paths. Rust Analyzer can now check their field mappings and
payload validation on native targets.

The retained store adapter now requires IC DB 0.23.10 within the 0.23 series
and uses its target-selected database handle. Existing hub and shard stores
reopen with their original allocation keys and schema. [Canic #40](https://github.com/dragginzgame/canic/issues/40)

From a Toko workspace using this revision, check both roles locally and build
the installed actors from the app directory.

```sh
cargo clippy -p user_hub -p user_shard --all-targets -- -D warnings
(cd apps/toko && canic app build)
```

## [0.227.0] - 2026-11-18

### ✨ Added

An authenticated hub can provision an account on its assigned shard with
`sharding::provisioning::provision`. The app supplies its ID and private
schema. [Hub and shard example](qualification/README.md#provision-an-account-on-its-assigned-shard)

Build both declared roles from the app directory:

```sh
canic app build --canister user_hub
canic app build --canister user_shard
```

### 🔧 Changed

Canic’s optional Blob adapter now uses Blob Storage 0.106.0 and shares its
Wire 0.25 protocol limits directly. Both workspaces include the Host and
Platform patch updates.

A shard's `mount_db_role!` now derives the parent cell key from its storage prefix.
Remove `parent = "..."` for the standard Toko layout. For a custom retained
layout, rename it to `parent_storage_key` and keep the exact value.
[Canic #37](https://github.com/dragginzgame/canic/issues/37)

Rebuild the shard from its app directory:

```sh
canic app build --canister user_shard
```

### 🧪 Testing

Both workspaces use Toko Tooling 0.45.9. Installed counter and recovery
tests share the same application client.

### 🎯 Qualification

Host rejects a symlink at the root of a declared `src` tree and refuses a
named pipe with a `.rs` extension before opening it. The starter's Canic
dependency is scoped to its Wasm target.

The installed database upgrade tests preserve populated records through a refused
upgrade and recover a lost install reply against the original operation.
[Canic #20](https://github.com/dragginzgame/canic/issues/20)

## [0.226.0] - 2026-11-18

### ✨ Added

Declare an application endpoint with `#[canic::query]` or `#[canic::update]`.
A protected endpoint selects its scope explicitly. An existing protocol can supply
one shared contract to preserve its method ID and Wire budgets.
[Migration example](apps/template/README.md#adding-an-application-endpoint) · [Canic #38](https://github.com/dragginzgame/canic/issues/38)

Generated canister crates check method, schema and lifecycle locations during
build. Each crate can override its own source layout. [Canic #26](https://github.com/dragginzgame/canic/issues/26)

Stateful hub and shard canisters can declare their generated IC DB schema,
retained keys, Auth receiver and lifecycle together. Native clients can use
the same compiled schema. [Role setup](qualification/README.md#retained-application-store)
· [Canic #37](https://github.com/dragginzgame/canic/issues/37)

Build the changed Wasm from a configured app directory:

```sh
canic app build
```

### 🔧 Changed

Fleet placement and subnet comparison now read through the published
`ic-query-topology` supplier. Existing schema 1 JSON reports retain their
format. [Canic #36](https://github.com/dragginzgame/canic/issues/36)

From an app directory with a configured mainnet fleet, inspect its bound canisters:

```sh
canic fleet placements --json
```

### 🧪 Testing

The installed profile route exercises qualified Rust paths for the method ID
and both Wire budgets. [Canic #29](https://github.com/dragginzgame/canic/issues/29)

## [0.225.0] - 2026-11-18

### ✨ Added

- A shard pool can opt into `install_topology = true` to deliver the approved Root and parent. The shard checks its actual caller and reopens its retained parent on upgrade. Hubs can resolve an assigned shard while keeping its allocation and generation. [#28](https://github.com/dragginzgame/canic/issues/28) · [#19](https://github.com/dragginzgame/canic/issues/19)
- Actors can mount bounded install and reopen hooks and declare a retained database once per role. [#27](https://github.com/dragginzgame/canic/issues/27) · [#32](https://github.com/dragginzgame/canic/issues/32)
- Native callers can encode and decode request and reply frames directly from a named method contract. [#35](https://github.com/dragginzgame/canic/issues/35)

### 🧪 Testing

The installed Backup witness now exercises operator disconnection and controller replacement between restore acceptance and reopening.

With a local Toko checkout using the updated actor adapters:

```sh
canic app build --config apps/toko/canic.toml --app toko
```

## [0.224.0] - 2026-11-18

### 🔧 Changed

Adopt IC Auth Wire 0.44.0 and Toko Tooling 0.45.7 in both workspaces. The generated browser codecs are byte-compatible with the existing fixtures.

Stateful actors can attach a named IC DB store using the optional `db-store`
feature. Run the [retained-store checks](qualification/README.md#retained-application-store)
with a prepared local simulator:

```sh
make db-test
```

### ▶️ Try it

Generated canisters put callable methods under `src/endpoints/` and check the
package's selected source layout on build. With a matching CLI and source checkout:

```sh
canic app init hello --source hello
cd hello
canic app build --canister app
canic artifact inspect artifacts/app.wasm
```

A fleet can select a shared Auth 0.48.0 receiver policy for its hub and shards.
Each receiver keeps its own Root grant. [Configuration and scoped endpoint example](docs/demos/auth.md#opt-in-volatile-receiver)

Applications can mount a volatile Auth receiver, share a typed Wire contract with
a calling canister, and request fresh IC entropy with an explicit timeout. Run the
local installed witnesses with the prepared simulator and signer:

```sh
make auth-test
make profile-test
```

## [0.223.1] - 2026-11-18

### 🔧 Changed

Auth receiver refresh uses one Wire contract for direct controller calls, Root forwarding and qualification clients. Native callers can use `enrollment::encode_refresh_request` and `enrollment::decode_refresh_response`.

A new source workspace starts with the single actor from the [fleet app template](apps/template/README.md).
From a matching Canic checkout with the CLI installed, create a fresh directory
and build the app locally:

```sh
canic app init hello --source hello
cd hello
canic app build --canister app
```

### ➕ Added

- Applications can mount their own named Wire queries and updates from a function signature. Canic supplies the export name, Wire codecs and bounded public ingress by default. [Canic #24](https://github.com/dragginzgame/canic/issues/24)
- Single-owner storage layouts can keep their existing authority, key prefix and Storage configuration when adopting Canic. [Canic #25](https://github.com/dragginzgame/canic/issues/25)

With the Rust Wasm target installed, build the qualification actor locally. Run its installed journey with the prepared simulator and signer.

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-profile-entry --features named-profile-witness
make profile-test
```

## [0.223.0] - 2026-11-18

Both workspaces now select IC Wire 0.25.0 directly. Auth's browser codec uses
the Wire version exported by IC Auth Wire. The Wasm Store integration reads
the supplier's byte, allocation, node and depth limits. Build the demo locally
or exercise a populated counter upgrade on the prepared simulator:

```sh
canic artifact build --package canic-demo --json
make counter-test
```

Concurrent Wasm builds keep their own final Cargo output and share the compiler
cache. By default each build returns a path containing the Wasm SHA-256. With
a prepared Rust Wasm target, these commands leave both demo variants available:

```sh
canic artifact build --package canic-demo --json
canic artifact build --package canic-demo --features counter-v2 --json
```

Use `--output PATH` for a replaceable file. Each command reports the digest of
the bytes it published, including when another build later replaces that path.

The demo counter can recover a checkpoint through its installing Root ([#7](https://github.com/dragginzgame/canic/issues/7)). Root
retains both rewind approvals, starts the child fenced, and keeps acceptance
separate from reopening. Build the Backup-enabled Root locally or run the
installed capture/restore with the prepared simulator and disposable signer:

```sh
canic artifact build --package canic-root --features backup --json
make backup-signer-test
```

One Auth-enabled Root can approve the hub and managed shards independently.
With both receiver slots bound in the local fleet, these commands retain
separate plans for operator review:

```sh
canic fleet auth plan --app toko --fleet local --receiver user_hub --controller "$OPERATOR"
canic fleet auth plan --app toko --fleet local --receiver user_shard --controller "$OPERATOR"
canic fleet auth plan-revocation "$APPLIED_SHARD_CONFIGURATION" --app toko --fleet local
```

The [Auth guide](docs/demos/auth.md#one-root-for-the-hub-and-managed-shards)
covers the receiver tables, apply steps and protected refresh calls.

Both workspaces now pin Toko Tooling 0.45.6. In a clean release checkout,
check the locked dependencies, simulator and package gates before publication.
Repeated Linux runs can reuse verified installed effect tests.
[Preflight #22](https://github.com/dragginzgame/canic/issues/22) · [Retry #23](https://github.com/dragginzgame/canic/issues/23)

```sh
make qualification-setup
make publication-admission
```

Declare local signup pool capacity before starting a simulator. With the adjacent
Toko checkout, inspect the declaration or build the application offline:

```sh
canic fleet list --app toko --config ../toko-new/apps/toko/canic.toml
canic app build --app toko --config ../toko-new/apps/toko/canic.toml
```

## [0.222.0] - 2026-11-18

Install the CLI and list local app declarations:

```sh
cargo install --locked --version 0.222.0 canic-cli
canic app list
```

The optional `db-schema` adapter now uses `ic-db` from crates.io.

### Auth planning for independently installed hubs

Set `receiver_controller = "operator"` in the fleet Auth table when the same
operator controls Root and the receiver. With bound principals and reviewed
Wasms, preview the grant and inspect the retained authority:

```sh
canic fleet auth plan --app toko --fleet local --controller "$CONTROLLER"
canic fleet status "$OPERATION" --app toko --fleet local
```

### Runtime shard placement

A hub can use `canic::sharding::assign` and `lookup` with an opaque account key.
Root retains generation-bound assignments and creates Store-backed shards within
the controller's approved capacity and cycle limits. The installed test covers
a hub and two shards, reply loss, concurrent signup and upgrades. A pool can
carry the application's exact installation bytes so each shard can retain its
trusted provisioning hub before it reports ready.
[Canic #19](https://github.com/dragginzgame/canic/issues/19)

With one configured pool fleet, preview the exact Store release, installation
bytes and growth limits. The JSON includes the complete Root request. Planning
is offline. The installed test uses synthetic cycles and prepared replica assets.

```sh
canic fleet plan --app toko --fleet local --controller "$CONTROLLER"
canic artifact build --package canic-root --features sharding --output target/shard-root.wasm
bash scripts/dev/test-store.sh sharding
```

## [0.221.0] - 2026-11-18

The optional Auth browser adapter now takes the selected device's SDK public
key as `browserKey`. Reopening an issuance intent with another device's key
fails before the account submits anything. With browser dependencies prepared:

```sh
make auth-browser-test
```

The local DB fleet now qualifies recovery after Root loses an upgrade callback.
With the repository ICP CLI prepared, run both the direct and managed DB tests:

```sh
make qualification-setup
make db-test
```

Both workspaces now pin Toko Tooling 0.45.3. Use `make toko-tooling-check`
to verify the installed snapshot against the locked archive.

IC Backup 0.105.6 supports Root-managed capture and restore through Canic’s optional Host codec. Both controller routes use a real external keyring signer.
On Linux with ICP CLI, D-Bus and GNOME Keyring, prepare the local tools and run:

```sh
make qualification-setup backup-signer-setup
make backup-signer-test
```

The managed fixture captures a Root-owned counter, recovers the original take and load results after lost replies, and checks the restored stable bytes while the counter is stopped.

Wasm Store 0.4.1 and its protocol are selected for fresh fleets. The operator can
build the published runtime locally from the Canic workspace with the installed CLI:

```sh
canic artifact build --package ic-wasm-store --output target/wasm-store.wasm
```

This writes a local Wasm. Existing 0.3.3 Stores require a separately qualified
migration from their Storage 0.74.1 directory.

Fresh counter and Root installations use Storage 0.78 named cells and the local
simulator uses Testkit 0.110. Blob 0.105 and DB 0.22.2 use the same Storage release.
The managed catalog reports explicit readiness and stays fenced across upgrade
until its matching activation. The populated Blob 0.100.6 catalog retains its original executable under the hard-cut decision in [Storage #8](https://github.com/dragginzgame/ic-storage/issues/8). Existing fleets require a separately
qualified storage migration.

### 🧩 Restore TOML configuration

**Configuration cut:** `canic.toml` replaces `canic.json` as the only app/fleet
source format. The declaration model stays the same. Initialization, discovery,
starter generation and CLI edits use TOML; edits preserve comments and existing
values. Repository configs are converted with their original canister IDs and
journal paths. External JSON configs need conversion, not just renaming.
Retained journals, signed requests and `--json` output are unchanged. Remove the
custom duplicate-name decoder: the TOML parser owns duplicate-key rejection.

### ▶️ Try it

From the repository root, install the updated CLI, then inspect the demo locally:

```sh
make canic
canic app list --config apps/demo/canic.toml
canic fleet canister list --config apps/demo/canic.toml
```

In your own project, create a fleetless app and then its deployment declaration:

```sh
canic app init example
canic fleet init mainnet --app example --network ic
```

These commands write local TOML only; no credentials, funding or deployment needed.

### 🧩 Root snapshot executor

Add an optional `backup` Root feature with authenticated `snapshot_execute` and
`snapshot_lookup` endpoints. A bounded surviving ledger retains the exact original
argument commitment, dispatch deadline, call-cost ceiling and inner result. Duplicate
delivery and lookup never redispatch. Missing callback evidence stays pending
through compatible Root upgrades. Ordinary private management endpoints stay
fenced once snapshot work is admitted.

Backup verifies the outer ingress and replays the exact inner result using Canic’s codec. Application fencing and reopening remain under [Canic #7](https://github.com/dragginzgame/canic/issues/7).
The optional Storage layout requires a fresh Root; no live migration is supplied.

Build/inspect only, from the repository root with `canic` installed:

```sh
canic artifact build --package canic-root --features backup --output target/backup-preview.wasm
canic artifact inspect target/backup-preview.wasm
```

With simulator assets already prepared, run the local Root/counter witness:

```sh
bash scripts/dev/test-root.sh snapshot::
```

It uses synthetic cycles and a qualification-only callback fault build. No backup
CLI command, mainnet effect, automatic activation or browser launch is introduced.

### 🧩 Application-owned commands

**CLI cut:** `fleet call OPERATION --name NAME` is retired. App definitions can
select an optional native `build.client` target in their existing package.
`app build` produces its codec beside the Wasm; `fleet call` invokes named commands
directly as the selected user. The counter owns `read`/`increment` and Wire codecs;
Canic owns target selection and exact signed-request custody. Root grants no
application permission and does not proxy these calls.

Uncertain calls retain their original request and block replacement calls from
that user to that target. `fleet apply`/`reconcile` observe an already signed call
without redispatching it. Reply decoding happens after retention. Call records
store request bytes, not another copy of the application Wasm.

### ▶️ Try it

From the repository root, after explicit tool/simulator preparation:

```sh
make demo
```

In the session directory it prints, build and deploy with synthetic cycles:

```sh
canic app build
canic fleet plan --identity anonymous
canic fleet apply OPERATION --identity anonymous
```

Repeat apply on that operation if Root is still pending. Once complete:

```sh
canic fleet call --canister app --identity anonymous read
canic fleet call --canister app --identity anonymous increment
canic fleet show
```

Increment changes the local persistent counter once. No window opens and no
mainnet wallet is used. Enter in the original terminal stops the simulator.

### 🧩 Managed counter and Host adoption

Fresh Root now calls explicit application readiness and accepts only `Ready`.
The installed fleet CLI publishes the counter through its separate Wasm Store,
creates it through Root and preserves its value across a managed upgrade. A
fenced counter stays closed and cannot receive a completion receipt through
repeated apply. Journal schema 2 refuses older greeting-approved Root state
without resetting it; existing fleets must keep their old executable.

Adopt Artifacts/FS 0.80.3 and product Process 0.81.0. Qualification retains Process
0.80.5 to match Testkit's public cleanup report. Text stays at 0.77.2. After explicit
tool and simulator preparation, build locally and run the managed rehearsal:

```sh
make canic
canic app build --config apps/demo/canic.toml
canic artifact inspect apps/demo/artifacts/root.wasm
bash scripts/dev/test-publication.sh counter_managed::
```

No mainnet deployment occurs. Interactive counter calls, Root-mediated fencing
and Backup recovery remain pending; `make demo` still uses the greeting actor.

### 🧩 Demo fleet

Implement the persistent counter in `apps/demo` using published Storage 0.76.2:
one bounded record, public increments, explicit installation-authority fencing,
matching activation and application readiness. Installed tests cover compatible
upgrades with writes still closed, stale activation, malformed input, failed
target-write rollback and corrupt/missing state refusal. From the repository root
with `canic` installed and simulator assets explicitly prepared:

```sh
canic app build --config apps/demo/canic.toml --canister app
canic artifact inspect apps/demo/artifacts/app.wasm
make counter-test
```

The counter build creates no canisters. Qualification uses synthetic cycles and
direct local control. Interactive counter calls and Backup restore
remain pending; `make demo` continues to rehearse the greeting actor.

Move fresh Root persistence onto Storage's public records API, keeping existing
custody payloads and mutation boundaries. Two bounded records occupy 17 stable
pages; no local allocator, Cell facade or upgrade flush is added. Compatible
executables preserve child identities, attempts and Store custody. Old 0.74
images refuse without reset; retain their old Root executable. Build/inspect only:

```sh
canic app build --config apps/demo/canic.toml --canister root
canic artifact inspect apps/demo/artifacts/root.wasm
bash scripts/dev/test-root.sh storage_reopen::
```

Add an optional application-owned `readiness` mount with distinct `Unavailable`,
`Fenced` and `Ready` responses. The greeting actor and generated starter expose
the bounded replicated observation; requests cannot select a state or activate
the application. Fresh Root now consumes this contract. Build the
counter actor without deployment:

```sh
canic app build --config apps/demo/canic.toml --canister app
canic artifact inspect apps/demo/artifacts/app.wasm
```

Select a persistent counter with explicit recovery acceptance for the ongoing
demo. The current interactive local session remains the
stateless greeting. Retire the profile product and `make profile` entrypoint;
move its five packages into the qualification workspace to retain existing
Auth/Blob, ownership and management regression coverage. After `make canic`:

```sh
canic app list --config apps/demo/canic.toml
# Explicit read-only inspection of the retired deployment:
canic fleet canister list --config qualification/fixtures/profile-mainnet.toml
```

The retired declaration retains the original canister IDs and root journal.
No live canister, controller or retained operation changes.

`make demo` starts a bounded interactive local actor session, with a fresh
config/journal and prepared simulator. It installs `canic` on PATH and prints
the normal build, plan, apply, call and upgrade commands. After explicit tool
and simulator preparation:

```sh
make demo
# In another terminal, cd to the printed session directory:
canic app build
canic fleet plan --canister app --identity anonymous
```

Application deployment remains explicit. Enter, Ctrl-C, input EOF or the one-hour
limit ends the session; local evidence remains. The session uses synthetic cycles
and the direct Host path, leaving the managed mainnet declaration untouched.

Add a fresh `demo/mainnet` declaration with supplied Root, Store and one
application, separate from existing mainnet demos. The stateless actor explicitly
mounts greeting and metrics; a missing timer provider is reported as unavailable.
The greeting adapter replaces duplicated application/starter codec wrappers.
After `make canic`, work from the repository root without deploying:

```sh
canic app canister list --config apps/demo/canic.toml
canic app build --config apps/demo/canic.toml --canister app
canic fleet list --config apps/demo/canic.toml
```

The managed counter has not been deployed to mainnet. The local installed greeting
journey remains runnable: persistent host-readback dependencies are selected
explicitly by the Blob/Auth-browser suites instead of every qualification test.

### 🛠️ Fixed

`app build` refuses outputs that overlap its config, any declared workspace
manifest or Cargo lockfile, including aliases and a lockfile not yet created.
All recipes are checked before Cargo runs, even when selecting one canister.
From a configured app, these commands still build only local Wasm artifacts:

```sh
canic app build
canic app build --canister app
```

### 🧩 Changed

Select registry IC Storage 0.76.2 directly in both workspaces. Its published
bounded records and Platform backing unblock implementation of the fresh counter
and Root record migration. This remains an incomplete consumer migration: existing
Root/profile imports and composed Blob/DB owners still need adaptation and
installed qualification; the version update alone does not make them compile.
[Storage #5](https://github.com/dragginzgame/ic-storage/issues/5) records the actual
callers, required contracts and installed acceptance. Existing suppliers still
select 0.74.1 transitively; no compatibility shim or retained-image conversion
is added. The deployed/installed 0.220.0 release is unchanged.

## [0.220.0] - 2026-11-17

### 🧩 Changed

Apps can be declared and built with zero fleets. `app init` now rejects `--fleet`
and network arguments, including with `--source`; use `fleet init` separately.
Existing app/fleet configs and retained deployment evidence stay readable.
App validation checks names, recipes and relationships even without a deployment.

From an existing workspace such as Toko (`apps/toko`, package `toko-app`), after
`make canic` in the Canic checkout:

```sh
canic app init toko
canic app canister add app --manifest Cargo.toml --package toko-app --wasm artifacts/toko.wasm
canic app build
```

This adds local declarations and builds the existing package without creating
another workspace, adding infrastructure, selecting an identity or deploying.
When deployment policy is ready, declare a fleet separately (still no deployment):

```sh
canic fleet init mainnet --network ic
canic fleet list
```

Toko topology, worker allocation and application readiness remain separate work;
the greeting probe is not database or authenticated-service readiness.

### 🧹 Cleanup

Signed-request fixtures now use Platform's bounded ingress encoder and typed
envelopes. Remove Canic's direct CBOR dependency and raw CBOR mutation code;
retain caller-binding, malformed-packet, signature, expiry and exact-byte tests.
Move the remaining inline Store custody tests into their owning module's test file.

### 🧪 Testing

Adopt Testkit 0.109.0 with Auth 0.47.4 and Runtime 0.23.17. Its bounded
clock API supplies the missing [Tooling #11](https://github.com/dragginzgame/ic-tooling/issues/11)
contract: an installed same-receiver test proves an expired active reply cannot
clear a newer pending refresh or overwrite its committed revocation, in either
callback order. This completes the delayed-response proof for
[Canic #9](https://github.com/dragginzgame/canic/issues/9).

[Canic #18](https://github.com/dragginzgame/canic/issues/18) now has an installed oversized-reply check: the endpoint refuses
encoding, preserves the full stored backing and returns normal inspection after
clearing the qualification-only fault. After `make canic`, build the ordinary
witness locally (no deployment):

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-db-witness --output target/db/normal.wasm
canic artifact inspect target/db/normal.wasm
```

Store qualification now builds and selects the CLI from the current checkout,
matching the other installed journeys. An older `canic` on PATH can no longer
silently build its test artifacts. No application or stored-state contract changes.
The `ci` and `lint` aggregates now serialize their stages under parallel Make,
so tests wait for lint completion and shared artifacts have one active builder.
A focused CI wiring check covers lint failure and successful stage ordering.

The installed Auth/profile browser journey now covers replacing a revoked login:
the new key keeps the same profile owner, delayed old-key disposal cannot remove
it, and old revocations cannot revoke its session. An active login cannot dispose
its key through Auth's terminal cleanup hook. Journals and revoked issuer slots
remain retained; this does not add production wallet custody or enable Auth by default.
Read-only sessions now have an installed write-refusal check. The browser journey
also revokes one actual Root grant: after receiver refresh, cached readiness and
an exact configuration retry cannot restore access; the other receiver stays usable.
The native enrollment regression now also refuses a stale active reply after a
newer revoked snapshot commits. Installed same-receiver supersession now exercises the supported Testkit clock boundary.

After `make canic`, build and inspect the optional Auth/profile witness locally:

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-auth-witness --features profile --output target/auth/profile.wasm
canic artifact inspect target/auth/profile.wasm
```

These commands neither deploy nor open a browser.

After `make canic`, inspect a locally built Store without deploying it:

```sh
canic artifact build --package ic-wasm-store --output target/store/local.wasm
canic artifact inspect target/store/local.wasm
```

## [0.219.0] - 2026-11-17

Breaking changes: local network plans require
`--root-subnet`. Applications embedding the profile engine must supply their
admitted Storage runtime; the standalone bootstrap remains separate.
Auth issuer mounts now require an explicit `logout` handler for browser-bound
revocation.

### 🧩 Changed

Make the profile engine reusable within an actor's shared Storage bootstrap.
The optional catalog/profile witness composes real Blob and profile state under
one lifecycle, preserving both through upgrades and keeping access closed until
the matching activation. Participant additions/removals require explicit migration;
the standalone profile's storage format and capacity remain unchanged.
Installed corruption checks also prove that a damaged profile store refuses the
upgrade without recreating state, changing Blob/catalog records or opening readiness.
Catalog rollback now also covers populated profiles; a full catalog still permits
profile updates and cleanup, with malformed requests preserving shared state.
After `make canic`, build and inspect this local composition:

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --features profile --output target/blob/composed.wasm
canic artifact inspect target/blob/composed.wasm
```

These commands neither deploy nor open a browser. See the
[composition example](docs/demos/blob.md#composing-another-durable-module).

Compose the existing persistent profile engine with optional Auth mounts in an
installed, headless Chromium witness. Two independently enrolled receivers keep
application ownership separate from browser identity. Lost issuance/admission
replies, renewal, cross-tab logout and reply rollback exercise the actual Auth
coordinator and IndexedDB journal. This is qualification, not a new default
profile deployment or production login service. After `make canic`:

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-auth-witness --features profile --output target/auth/profile.wasm
canic artifact inspect target/auth/profile.wasm
```

The artifact builder now selects Cargo's exact requested package identity, so
a dependency emitting its own Wasm cannot make a composed application ambiguous.
The profile persistence engine is reused within its existing package; standalone
canister exports remain enabled by default. See the [Auth guide](docs/demos/auth.md)
for explicit browser preparation and `make auth-profile-test`.
The nine-case browser journey now includes network loss during cleanup and a
late authenticated issuer result arriving after durable logout. Original cleanup
deadlines/attempts and unknown issuance remain visible; neither path can restore
readiness or repeat an application write. The [actor excerpt](docs/demos/blob.md#what-the-actor-looks-like)
shows the current service-mount API using the real catalog actor.

Add optional DB inspection contracts and Auth browser-bound issuer logout plus
the generated browser transport adapter. DB inspection exposes bounded typed
schema and maintained cardinality, preserving closed/incompatible state. Auth
keeps its original login/result owner; Canic adds no session journal or scheduler.
After `make canic`, build and inspect the local qualification applications:

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-auth-witness --output target/auth/witness.wasm
canic artifact build --manifest qualification/Cargo.toml --package canic-db-witness --output target/db/witness.wasm
canic artifact inspect target/db/witness.wasm
```

These commands build local artifacts, without deploying or opening a browser.
See the [Auth adapter](crates/canic/browser/README.md) for explicit npm preparation
and the [installed qualification](qualification/README.md) for real-library tests.
Qualify registry Auth 0.47.2 with its passive Wire/Types 0.43.0 and the pinned
0.47.1 browser source. Backup remains admitted,
with exact signing and manager-route prerequisites retained in
[Canic #5](https://github.com/dragginzgame/canic/issues/5) and
[Canic #6](https://github.com/dragginzgame/canic/issues/6). Relay work is deferred
at the maintainer's request; its optional design remains unchanged.

Remove Platform 0.33.7 from both dependency graphs. Host now uses Platform
0.34.13 for certified observations, queries and exact retained request dispatch.
The old Agent and dual-facade alias are gone; fixed wire fixtures preserve the
legacy codec comparisons without a legacy test dependency.

New local fleets require `--root-subnet` alongside URL, root key and routing
canister. Old local approvals retain their bytes/digests and refuse network
access when that trust input is absent; they are never rewritten automatically.
Mainnet keeps its compiled trust inputs. With an existing configured fleet:

```sh
canic fleet show --app profile --fleet mainnet
canic fleet status OPERATION --app profile --fleet mainnet
```

Use Platform's prepared-packet admission for retained updates and status reads.
Select signed or anonymous admission from the approved controller, refuse mixed
authentication fields and tampered signatures, and retain exact bytes and expiry.
This adopts the anonymous contract in
[Platform #25](https://github.com/dragginzgame/ic-platform/issues/25). No new
dispatcher or signing path is added.

Adopt Platform 0.34.13 and Runtime/Macros/Guard 0.23.16 in both workspaces, with
Timers 0.46.3 and Topology 0.109.2. These published supplier updates remove the
previous blockers ([Query #2](https://github.com/dragginzgame/ic-query/issues/2),
[Scheduling #3](https://github.com/dragginzgame/ic-scheduling/issues/3)). Both
workspaces now select Protocol 0.3.5 through compatible supplier requirements.
No supplier override or second executor is introduced.
Root selects Metrics 0.30.9. The timer and metrics implementations are unchanged.

Fix wallet overflow refusal to work in the normal CLI dependency graph without
relying on a numeric error trait enabled only by development dependencies.

Move Host's publication-removal module to `removal/mod.rs` beside its tests,
removing the remaining `foo.rs` plus `foo/` layout conflict. Module paths and
behavior are unchanged.

Retain sanitized CI setup failures before tests can start, including explicitly
reported hidden Cargo-tool attempt logs. Failure artifacts expire after seven
days; missing evidence and upload failure remain visible, and collection cannot
replace the original setup exit status. No application command changes.

Move Host and runtime management
codecs to Protocol, including local creation, controller handover, chunked
installation and status-version checks. Remove the runtime's legacy codec
dependency and redundant principal conversions. Existing request bytes, approvals,
stable records and recovery deadlines are preserved. This advances
[Canic #14](https://github.com/dragginzgame/canic/issues/14), including Host's
transport replacement and explicit local root-subnet admission.

Adopt registry Topology 0.109.2 for read-only fleet placement and subnet comparison.
Use canonical Types at both caller boundaries and remove the old Git Platform,
Protocol and Types dependencies. Partial reports, configured-slot order and
read-only authority remain unchanged. With an existing mainnet fleet configured:

```sh
canic fleet placements --app profile --fleet mainnet
canic fleet compare-subnets LEFT_SUBNET RIGHT_SUBNET --app profile --fleet mainnet
```

Derive an address locally, or inspect an existing retained operation without sending
effects (the latter needs the matching fleet config and operation):

```sh
canic wallet OWNER_PRINCIPAL --address-only
canic fleet status OPERATION --app demo --fleet local
```

Preserve and qualify published Blob Storage 0.103.2 and Host Process 0.80.4,
with Host Artifacts/FS 0.80.3. Adopt Testkit 0.108.0; its held-reply fixtures now
share Protocol 0.3.5 and remove qualification's old Protocol 0.2 dependency.
Types 0.21.2 remains selected. Blob's local Service implementation and
retained-state format are unchanged. No optional supplier becomes mandatory.

With development tools prepared, build and inspect the embedded catalog locally;
these commands do not deploy or open a browser:

```sh
canic artifact build --manifest qualification/Cargo.toml \
  --package canic-asset-catalog --output target/blob-catalog.wasm
canic artifact inspect target/blob-catalog.wasm
```

## [0.218.0] - 2026-11-16

### ⚠️ Breaking

Canic's actor mounts now use Platform Runtime 0.23.14 and Timers 0.46.0's single
executor. Applications composing these mounts must select that Runtime cohort
and its explicit entrypoint macros. Rebuild generated starter applications from
the current template. Upgrades now stop the target and drain callbacks before
installation, then restart before checking readiness. Interrupted stop/start calls
retain their original authority. New pending phases require the new reader;
existing completed records and approval digests retain their formats. An unfinished
legacy Host installation without retained stop evidence remains blocked.
Canic's explicit entrypoint error policy traps guard refusals as `CanisterError`;
it no longer uses the former CDK's `CanisterReject` representation. Authorization
still precedes argument copying, and response-encoding failures still roll back.

### ▶️ Try it

Inspect existing grant commands without sending effects. The status command needs
an existing fleet config and retained operation:

```sh
canic fleet auth --help
canic fleet status OPERATION --app demo --fleet local
```

After development-tool preparation, provision the authenticated local simulator
explicitly and check its ownership boundary (synthetic local tests, no mainnet effects):

```sh
make qualification-setup
cargo test --locked --manifest-path qualification/Cargo.toml \
  -p canic-qualification --test server
```

With an existing Root/Store fleet, an application-generated initialization frame
and a completed release publication, retain and approve exact application inputs:

```sh
canic fleet plan --canister catalog --root-module ROOT_HASH --release PUBLICATION \
  --install-args catalog-init.bin --identity canic-mainnet
canic fleet apply OPERATION --identity canic-mainnet
```

For an existing configured catalog, use a Root built with `blob`, a published
candidate, its method-bound upgrade frame and the next supplier attempt. Reopening
keeps admission closed until the separate activation command:

```sh
canic fleet plan --canister catalog --root-module ROOT_HASH --release PUBLICATION \
  --target CANISTER --previous-module OLD_HASH --install-args catalog-upgrade.bin \
  --blob-attempt 1 --identity canic-mainnet
canic fleet apply UPGRADE_OPERATION --identity canic-mainnet
canic fleet activate UPGRADE_OPERATION --identity canic-mainnet
```

### ✨ Blob fleet lifecycle

Complete [Canic #12 — approved Blob provisioning](https://github.com/dragginzgame/canic/issues/12)
and [Canic #13 — exact managed Blob maintenance](https://github.com/dragginzgame/canic/issues/13)
through the existing plans, signed requests, Store pins and Root effect owner.
Bounded application arguments survive in the existing artifact lease; newer
custody is explicit and older observers refuse. Optional Root Blob support owns
management calls while the supplier retains all admission and accounting rules.
The installed witness preserves both users, shared references and unresolved
declarations through lost callbacks and Root reopening, then separately activates
the matching attempt. No new crate, Blob ledger, generic workflow engine,
automatic activation, provider calls or live deployment is introduced.

Raise the bounded direct-install input ceiling to 1,900,000 bytes so the current
Root, including optional Blob support, can be bootstrapped through the CLI.
Qualify the encoded management request's ingress headroom and reject larger
plans. Store-backed child and inline-child limits remain independently owned.

### 🔧 Dependencies

Adopt Platform 0.34.9 for Host's signed-envelope admission and the installed Auth
application's Ed25519 signing/native verifier. Retain original signed bytes,
request IDs and deadlines; reject unknown call, status and envelope fields.
Move enrollment and issuer entropy framing to Protocol's bounded `raw_rand`
capability, removing those direct legacy Platform dependencies. Product Protocol
selects 0.2.7; qualification retains 0.2.6 because Testkit 0.107.3 pins it exactly.
Actual IC entropy still passes through the original reservations, caller checks
and deadlines. Runtime captures refunds with their matching completion. Unknown
outcomes remain uncertain and never authorize replacement calls.

Full replacement remains open under
[Canic #14 — Platform migration](https://github.com/dragginzgame/canic/issues/14):
Host still uses the legacy Agent and management/ledger codecs; runtime management
DTOs retain an explicit passive identity conversion at that codec boundary.
The newly published alternatives need a separate custody-preserving migration.
Canic continues to leave signing keys with ICP CLI.

Adopt Toko Tooling 0.44.11 through the authenticated installer in both workspaces.
Complete [Canic #15 — bounded simulator ownership](https://github.com/dragginzgame/canic/issues/15) using Testkit
0.107.3's optional managed server across both installed harnesses and explicit
preparation. Remove the 0.106.0 lifecycle dependency and its older Host selections.
Servers have explicit startup/lifetime/output bounds; instances attempt deletion
before server stop. Qualify refusal, startup failure, teardown and fallback ownership
without copying supplier lifecycle code or adding a compatibility wrapper.
Adopt Host Text 0.77.2 and call its simpler table API
directly, removing repeated options. Other IC Host packages already select 0.80.2.

Select published Blob 0.102.0 and Wasm Store 0.3.0 with locked archive checksums,
plus committed Auth and DB inputs that use Platform Types 0.21.2. Qualify reopening
the retained populated 0.100.6 executable. Remove the obsolete private-Blob fetch
key wiring from CI; the supplier checkout remains read-only.

The earlier compatibility adoption selected Topology 0.107.0 at committed revision
`12844b628d96bfd203ffbd900f10d02c4a896988`: the previous pinned release requires
`thiserror` 2.0.21. Both lockfiles select Wire 0.24.3. Storage's old
Runtime dependency supplies raw stable-memory operations only, not a second
executor. No uncommitted supplier checkout is consumed.

Release freshness review confirms these registry selections against Cargo's
official index. Auth's newer committed 0.47.0 source is deferred to
[Canic #16 — browser logout integration](https://github.com/dragginzgame/canic/issues/16)
and [Canic #17 — browser client integration](https://github.com/dragginzgame/canic/issues/17).
The newer Query repository checkpoint is deferred to the remaining Host/Topology
migration in Canic #14. DB retains its current committed 0.19.0 release.

### 🧪 Qualification

Extend [Canic #9 — Auth qualification](https://github.com/dragginzgame/canic/issues/9) with actual Root
responses held before publication, released in reverse receiver order. Active
and revoked grants retain their own caller, challenge and original deadline;
the test barrier supplies no authentication data. Same-receiver expired-request
reordering remains open. Explicit Cargo test entrypoints prevent helper modules
from being misidentified as independent suites.

```sh
# Inspect enrollment commands; this sends no effects.
canic fleet auth --help
```

Add optional typed database mounts and `make db-test`. Applications retain their
schema, passive protocol, caller policy and explicit storage initialization.
The installed witness checks bounded pages, cursor refusal, atomic batches,
five upgrades, maintenance closure and reply-encoding rollback. It consumes
committed IC DB without adding a database dependency to the adapter.

```sh
make db-test
```

Add bounded optional issuer mounts for
[Canic #10 — Auth issuance, renewal and lookup-only reconciliation](https://github.com/dragginzgame/canic/issues/10).
Applications own caller authentication and keys; Auth owns retained intents and
results. The installed local-key witness exercises real issuance and renewal,
receiver verification, duplicate/lost replies, callback rollback, expiry and
pruning without replacement signatures. Production key custody, browser issuer
logout and external signing recovery remain pending.

With the toolchain prepared, build the experimental composition witness locally:

```sh
canic artifact build --manifest qualification/Cargo.toml \
  --package canic-auth-witness --output target/auth/receiver.wasm
canic artifact inspect target/auth/receiver.wasm
```

Strengthen [Canic #9 — Fleet-authorized Auth enrollment](https://github.com/dragginzgame/canic/issues/9)
with installed concurrency and expiry cases. Overlapping refreshes retain one
reservation, Root call and original deadline. Separate-subnet expiry cases
interrupt real entropy/snapshot awaits, preserve prior enrollment and require a
new explicit admission before another attempt. Reuse the existing fixture setup;
no production endpoint, workflow, dependency or failure switch is added.

Also qualify loss of an active Root by stopping and upgrading it. Failed refreshes
preserve the original busy window, snapshot and session deadlines; access refuses
at the original freshness boundary without another application write.

### 📚 Documentation

Remove the stale claim that no Auth CLI exists. Grant configuration/revocation
are available; issuer mounts now exist, while browser login remains unfinished.

## [0.217.0] - 2026-11-16

### ▶️ Try it

After `make canic`, inspect the new optional Auth commands without sending effects:

```sh
canic --version
canic fleet auth --help
canic fleet status OPERATION --app starter --fleet mainnet
```

The last command requires an existing fleet configuration and retained operation.

### ➕ Added

Optional Auth receiver mounting in `canic` for
[#8 — Mount IC Auth receiver contracts](https://github.com/dragginzgame/canic/issues/8).
Admission, session reads and logout preserve the supplier's Wire contracts,
captured callers and synchronous rollback. See the [Auth guide](docs/demos/auth.md).

Add the first runtime boundary for
[#9 — Fleet-authorized Auth enrollment](https://github.com/dragginzgame/canic/issues/9):
an optional Root authority for one explicit grant, direct receiver pulls with
IC-generated challenges, terminal revocation and atomic callback publication.
Refresh preserves original session deadlines. **Both Auth owners stay closed
after upgrade**; managed Auth preservation and the complete browser journey
remain unfinished.

Host now compiles an optional fleet `auth` declaration into exact configuration
and terminal revocation approvals. They reuse the existing journal and signed
attempts, bind installed Root/receiver code and controllers, and produce historical
Auth replies rather than deployment/readiness receipts. Known Auth approvals
fence preservation upgrades. Host consumes passive contracts without the engine.

With an installed Auth-enabled Root/receiver and the explicit grant described in
the [Auth guide](docs/demos/auth.md), retain and review a plan before submission:

```sh
canic fleet auth plan --app demo --fleet local \
  --controller ROOT_CONTROLLER
canic fleet status CONFIGURATION_OPERATION --app demo --fleet local
# Requires the original successful configuration reply; sends nothing:
canic fleet auth plan-revocation CONFIGURATION_OPERATION \
  --app demo --fleet local
```

After `make canic`, build and inspect the qualification application without
deploying or opening a browser:

```sh
canic artifact build --manifest qualification/Cargo.toml \
  --package canic-auth-witness --output target/auth/receiver.wasm
canic artifact build --package canic-root --features auth \
  --output target/auth/root.wasm
canic artifact inspect target/auth/receiver.wasm
```

### 🧪 Qualification

Wire hosted Linux/macOS native and Wasm product checks, formatting, snapshot/pin
admission and one full-history secret scan for
[#3 — Complete hosted CI wiring](https://github.com/dragginzgame/canic/issues/3). Adopted Toko Tooling 0.44.5
fixes admission of the required exact tooling pin and independent private
qualification versions. Hosted Linux and policy checks passed; macOS qualification
remains pending after the path-validation fix below.
Private Blob dependency access uses a dedicated read-only deploy key in the
`CANIC_BLOB_DEPLOY_KEY` Actions secret, with pinned SSH host verification and
fetch-only credential lifetime. Authenticated source access is verified locally
and by hosted dependency preparation.

Adopt published Store/runtime protocol 0.2.3 for
[#4 — Qualify Store deployment and maintenance flows](https://github.com/dragginzgame/canic/issues/4).
The installed boundary witness confirms its versioned page distinguishes removed
labels from empty restored uploads while preserving the historical page contract.
Root now commits to that distinction and retains the scan contract and removed-entry
cursor across Root/Store upgrades. Historical attempts preserve their exact scan
and refusal. Root provides exact deletion/cancellation with pin, identity, deadline
and maintenance checks. Host plans removal from an independent publication,
retains one signed attempt, and permits same-content restoration only under a new
approval after successful settlement and expiry of every original removal deadline.
Old publication approvals are fenced. Lost replies and ambiguous publication calls
stay blocked; observing absence cannot substitute for their exact replies.

With the current Root/Store installed, the selected fleet's original journal, and
an independent publication operation (not a nested bootstrap publication), these
commands retain plans only:

```sh
canic fleet artifact plan-removal PUBLICATION_OPERATION --action delete \
  --app starter --fleet mainnet
canic fleet status REMOVAL_OPERATION --app starter --fleet mainnet
# After successful removal and expiry of its original dispatch deadline:
canic fleet artifact plan-restoration REMOVAL_OPERATION \
  --app starter --fleet mainnet
```

Use `--action cancel` for an incomplete upload. After reviewing each returned
plan, `canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet`
sends its approved effects. `fleet reconcile` reuses the original removal attempt;
it cannot sign a replacement. Restoration retains immutable version/content identity.
Pinned content remains protected. The installed CLI witness covers that refusal
and explicit restoration after both deletion and successful cancellation. This
is locally qualified; no mainnet effects were submitted.

Update the profile/Store upgrade witness to inspect the versioned preservation
contract, including the explicit entries scan and exact post-upgrade match.
The old flat-JSON assertion no longer described current Store evidence.

For an existing configured fleet and retained operation, inspect evidence without
signing or opening a browser:

```sh
canic fleet show --app starter --fleet mainnet
canic fleet status OPERATION --app starter --fleet mainnet
```

### 🔧 Dependencies

Both workspaces and the authenticated tooling snapshot now select published
Toko Tooling 0.44.6. Existing tooling entrypoints and prepared tool versions are
preserved. Local release qualification remains required; hosted CI proceeds
asynchronously after delivery.

Assessed the published Platform replacement (Platform 0.34.3, Runtime 0.23.9,
Types 0.21.1). Adoption is blocked: the new packages do not supply Canic's current
Agent/signed-envelope, management/ledger, Ed25519/raw-rand or full CDK lifecycle
interfaces, and its suppliers still use the earlier Types/Runtime cohort. Keep
the qualified Platform 0.33.7 / Runtime 0.22.5 / Types 0.20.4 graph until those
contracts have a qualified migration. No compatibility facade, second runtime
or bypass of the Platform dependency boundary was added.

Adopt published `ic-host-fs` / `ic-host-artifacts` 0.80.2 and
`ic-host-process` 0.80.2. Text remains at the current published 0.76.0.
Host now uses held directories for journal writes and complete fleet publication,
and bounded process reports for Cargo and ICP. Report publication shares Host's
file policy; the CLI no longer depends directly on FS. Wasm footprint parsing
stays in qualification, outside the shipped host graph.

Output writes require normal path components: use a path without `..` or
symlinked parents. Artifact inspection still follows selected aliases; retained
records refuse final symlinks. Existing files and approval records are preserved.

Build/install the updated CLI with `make canic`. For an existing configured fleet:

```sh
canic --help
canic fleet show --app starter --fleet mainnet
canic fleet show --app starter --fleet mainnet --html target/starter-fleet.html
```

The report command creates a new file and refuses replacement. These commands
do not launch a browser or submit deployment effects.

### ⚠️ Breaking

New Store-maintenance attempts require Store 0.2.3. Existing fleets must upgrade
Store under their old Root before upgrading Root. The shared management journal
now writes CUJ9; historical records migrate without changing approval digests,
but older executables refuse the new journal. New scan evidence uses an explicit
Wire variant and JSON `{"entries": ...}` wrapper; historical evidence keeps its
original encoding. Rust callers use `Scan` for maintenance preservation and
verification rather than `Option<Preservation>`.

Host callers that exhaustively match `PlanAction`, `ApplyOutcome` and `StatusDetail`
must handle removal. `Publication` adds optional `after_removal` authority; old
approval bytes/digests remain unchanged when it is absent.

Host callers that exhaustively match `FleetPlanning` must handle the new
`AddChild` result. CLI JSON consumers must accept `action: "add_child"` for
growth plans. Existing approval records remain unchanged.
Named fleet-owned upgrades now require completed initial fleet creation and no
unrelated unfinished fleet operation. Finish or reconcile that work first.
Rust callers now use `children::ChildId` instead of the `Slot` enum. Use the
checked `ChildId::new` constructor or `FIRST`/`SECOND` constants. Numeric identity
does not expand capacity: only 0 and 1 are admitted. Existing JSON/Wire names,
approval digests and Store pin identities remain unchanged; there is no old-type alias.

Root and profile managers migrate their stored child records to identified
collections during upgrade, preserving the original allocations and complete
custody. Older executables refuse the new format. Upgrade through reviewed Root
maintenance; returning to a pre-migration executable is no longer supported.
The two-child capacity and existing artifact budget remain unchanged.

For a completed starter whose Root build input has been updated, build and review
its maintenance plan before applying the returned operation:

```sh
canic app build --app starter --canister root
canic fleet plan --app starter --identity canic-mainnet
canic fleet apply OPERATION --app starter --identity canic-mainnet
```

### ➕ Added

A completed one-child fleet can now add its second child through the existing
plan/apply workflow. The new child has a separate, finite creation approval;
the original fleet budget, approval and child identity remain unchanged. Store
fleets publish the new release first. Later upgrades and Root maintenance retain
both children, including a new name that sorts before the original child.

For a completed generated `starter` fleet with one app, add a second instance of
its application package and build its artifact:

```sh
canic app canister add worker --app starter --parent root --role worker \
  --wasm artifacts/worker.wasm --manifest Cargo.toml --package starter-app
canic app build --app starter --canister worker
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
```

Review the returned operation, then apply it. If it publishes a Store release,
repeat plan/apply to approve creation separately:

```sh
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
canic fleet show --app starter --fleet mainnet
```

Creation draws 1T from Root's existing balance; Root must retain its operating
reserve. Planning does not fund Root or submit effects. Pending creation resumes
its original authority; removing its declaration or changing its artifact cannot
replace it. The two-child bound remains; general allocation and reusable pools
are still pending.

The installed profile demo also exercises second-worker growth after Root/Store
maintenance, private data on both workers, the added worker's upgrade and later
Root replacement. User IDs remain local to each worker; no profiles move.
For a completed fleet prepared by the [profile guide](docs/demos/profile.md):

```sh
canic app canister add archive --app profile-store --parent root --role worker \
  --wasm artifacts/archive.wasm --manifest ../Cargo.toml --package canic-profile-worker
canic app build --app profile-store --canister archive
canic fleet plan --app profile-store --identity canic-mainnet
```

Apply each reviewed publication/creation operation separately, as above.

### 🧹 Cleanup

Host now decodes complete Root custody once for Store-backed child recovery and
Root maintenance. Each child journal stays paired with its artifact references
under its stable identity, including empty and uncertain slots. Existing signed
contracts, approval digests and the two-child limit remain unchanged.

For an existing configured starter, inspect the retained operation and fleet:

```sh
canic fleet status OPERATION --app starter --fleet mainnet
canic fleet show --app starter --fleet mainnet
```

These commands do not submit deployment effects or open a browser.

Management timers, active-operation guards and qualification fault selection now
use child identities directly, matching the durable journal. This removes their
fixed array-position dependency without changing approvals, timer
recovery rules or the current two-child capacity bound.
The public child identity is now numeric; historical enum names belong to its
compatibility codec. Unsupported values refuse before entering runtime authority.
Persistent collection readers also reject duplicate, missing, reordered or unknown
identities instead of assigning records by their position or discarding entries.
Compact artifact encoding preserves full existing allocations and checks Storage
framing before writing; it replaces the old per-child writers, retaining migration readers.

### 🩹 Fixed

App build rejects regular files used as directory ancestors consistently on Linux
and macOS, including trailing slashes, dot components and symlink targets.
Validation happens before generating a lockfile or starting a build. This fixes
the macOS failure found by [#3 — Complete hosted CI wiring](https://github.com/dragginzgame/canic/issues/3).

Named upgrades now use the same retained serial upgrade chain as whole-fleet
planning, including children added later and completed Root/Store maintenance.
Both paths return the same pending approval and enforce the eight-upgrade,
artifact-custody and dispatch limits. Earlier approvals remain immutable; missing
completion or a stale predecessor cannot authorize the next upgrade.
Installed qualification also repeats B → C → B → C: reused artifacts get fresh
installation approvals, while Store reuses the existing immutable publications.
This does not establish application schema downgrade or state rollback support.

For a completed inline child, rebuild its changed artifact and substitute the
exact identities and current predecessor hash from retained receipts:

```sh
canic app build --app starter --canister worker
canic fleet plan --app starter --fleet mainnet --canister worker \
  --target WORKER_ID --previous-module WORKER_SHA256 \
  --root-module ROOT_SHA256 --identity canic-mainnet
```

Named Store-backed plans also require their bound Store and completed publication
via `--release`. Serial named upgrades require the retained whole-fleet creation
approval. Whole-fleet planning resolves these inputs automatically.

Named replanning checks the complete bounded reservation scope before reusing
an approval. A matching record can no longer hide a conflicting or duplicate
approval, malformed later record or exceeded inventory limit because of directory
order. Valid retries retain their original digest and deadline.

For an existing configured fleet, inspect an operation and retained publications
without signing or submitting effects:

```sh
canic fleet status OPERATION --app starter --fleet mainnet
canic fleet artifact list --app starter --fleet mainnet
```

## [0.216.5] - 2026-11-16

### 🩹 Fixed

Whole-fleet planning now explains the supported application-child count when a
fleet declares zero or more than two direct children. Wasm Store is counted
separately. Refusal happens before reading artifacts or opening the journal;
an existing approval and its signed child admissions remain unchanged.

### 🧹 Cleanup

Fleet planning, Root maintenance and runtime continuation now share the protocol's
ordered child-slot list. This removes a Host-owned copy without changing slot
identities, approval bytes or the current two-child limit. General allocation
remains a separate, preservation-qualified change.

Runtime creation, upgrades and artifact custody now look up children by identity.
Historical array positions stay inside the storage codec, preserving the exact
on-chain format and existing approvals. Installed witnesses check pending work
and Store-backed children across the previous and updated executables.

For an existing configured fleet, these commands inspect declarations and retained
or observed deployment evidence without submitting effects:

```sh
canic fleet canister list --app starter --fleet mainnet
canic fleet show --app starter --fleet mainnet
```

## [0.216.4] - 2026-11-16

### ▶️ Try it

From the Canic checkout, install the updated CLI with `make canic`. For an existing
configured fleet, inspect both supported child slots of each selected Root, even
when another operator created a child and its deployment journal is absent here:

```sh
canic fleet show --app starter --fleet mainnet
canic fleet show --app starter --fleet mainnet --json
```

These commands only observe. Empty additional slots stay out of the display;
unavailable slot endpoints are reported as unknown inventory. Retained approvals
still constrain observations; discovery never creates deployment receipts.

### 🔧 Changed

Child reports in terminal and browser views now identify their stable slot and
local name, or the reported principal when no name is available. Conflicting
historical names cannot silently select the first record. Child names are no
longer labelled as applications in the browser view.

The saved/live fleet browser view now includes collapsed timer diagnostics, using
the same counter labels, exact totals and overflow quality as terminal details.
It shows each owner's initialization window and sample time; unavailable reports
stay unknown. This reuses existing observations and adds no collection or history.

For a configured fleet, serve a fresh snapshot or watch read-only refreshes at
`http://localhost:8787/`. Neither command opens a browser; Ctrl-C stops the viewer:

```sh
canic fleet show --app starter --fleet mainnet --serve
canic fleet show --app starter --fleet mainnet --watch
```

### 🩹 Fixed

Named child upgrade planning now carries the slot from retained creation custody.
The fleet reservation check resolves the requested child instead of always checking
the first. This fixes first upgrades of the second child through the existing
`--canister` path; it adds no slot flag or additional runtime capacity.

For a retained inline fleet with a declared `worker`, rebuild its changed artifact,
then substitute the exact principals/hashes from its receipts to retain a first
upgrade plan. Planning does not submit the upgrade:

```sh
canic app build --app starter --canister worker
canic fleet plan --app starter --fleet mainnet --canister worker \
  --root-module ROOT_SHA256 --target WORKER_PRINCIPAL \
  --previous-module WORKER_SHA256 --identity canic-mainnet
```

Whole-fleet planning remains the normal workflow, resolving these inputs for you.
Store-backed named plans also require their completed publication via `--release`.

## [0.216.3] - 2026-11-15

### 🔧 Changed

Adopt IC Timers 0.45.10 for Root and the profile manager. Child slots use retained
timer identities; an abandoned callback requires explicit timer recovery through
the existing authenticated operation reconciliation. Recovery still follows the
durable effect evidence and cannot renew approval or replay an uncertain effect.
Root and the profile manager enable the supplier's optional metrics feature in
normal builds. Canic's optional metrics endpoint adapter exposes six aggregate
timer counters, with overflow quality and a lifecycle reset window. Recording
stays supplier-owned; default application builds do not acquire it.

For a configured, deployed fleet, these read-only commands show the selected
management owners' timer observations. Details keeps them out of the compact view:

```sh
canic fleet show --app starter --fleet mainnet --details
canic fleet show --app starter --fleet mainnet --json
```

Older or unreachable endpoints are unknown, never zero. Counters reset on owner
upgrade, are uncertified and never prove effect completion. JSON counter totals
are decimal strings to preserve full integer precision.

For an existing approved operation, inspect retained evidence, then reconcile
observable outcomes with its controller identity when needed:

```sh
canic fleet status OPERATION --app starter --fleet mainnet
canic fleet reconcile OPERATION --app starter --fleet mainnet --identity canic-mainnet
```

Reconciliation can resume eligible, unissued work under the original approval.
This supplier change requires rebuilding and upgrading the management canister;
updating the native CLI alone does not change an installed fleet.

### 🩹 Fixed

App builds now compare resolved output destinations before compiling. Recipes
that alias the same Wasm file through absolute/relative paths, parent traversal
or symlinks are refused before creating a Cargo lockfile or build output. This
also applies when selecting one canister; original paths and the artifact writer's
independent filesystem checks are preserved.

For a generated starter or another app with complete build recipes:

```sh
canic app build --app starter --canister app
canic app build --app starter
```

## [0.216.2] - 2026-11-15

### 🔧 Changed

Rebuild one app canister with its existing recipe, without spelling out its Cargo
package or output path. Other canisters are not rebuilt. All recipes are validated
first; existing lockfiles stay locked, and the command does not deploy:

```sh
# After editing the generated starter's application source:
canic app build --app starter --canister app
# Rebuild every recipe when intended:
canic app build --app starter
```

`fleet status` now shows a Store-backed child's retained pin state and original
cache preparation deadline, for creation and upgrade operations and nested fleet
steps. The effect table calls Store delivery `prepare cache`; inline artifacts
keep `upload`. These are local historical observations, not live cache checks or
permission to repeat preparation or installation.

For an existing Store-backed child or whole-fleet operation:

```sh
canic fleet status OPERATION --app starter --fleet local
canic fleet status OPERATION --app starter --fleet local --json
```

JSON adds `delivery` only to Store-backed child status: `null` means no retained
delivery observation; otherwise it contains the validated Root delivery record,
including exact release, pin operation and nanosecond deadline. Existing fields,
approvals, journals and inline status shapes are unchanged.

Partial Root recovery replies now preserve earlier delivery evidence. Host
refuses later observations that regress a pin or remove/change the original cache
deadline, using the same reader for status, continuation and maintenance planning.

### 🩹 Fixed

Command failures use readable terminal text by default. `--json` preserves the
existing structured error fields, and failures still exit unsuccessfully on
stderr. Multiline diagnostics stay readable while terminal controls are escaped.

## [0.216.1] - 2026-11-15

### 🔧 Changed

Wasm Store and its protocol now come from crates.io with matching locked versions.
The artifact builder uses the locked runtime archive and its packaged lockfile, requires a matching
protocol, and removes the Git checkout layout dependency. Store is a build input
only; its runtime is not linked into Host.

With the Wasm toolchain prepared and registry access for uncached sources:

```sh
make canic
canic artifact build --package ic-wasm-store --output target/wasm-store.wasm
canic artifact inspect target/wasm-store.wasm
```

Building does not deploy or change retained fleet approvals. Supplier removal
and restoration endpoints are not exposed through Canic's publication workflow.

### 🧪 Testing

The installed maintenance witness now starts from a retained pre-registry Store
0.1.0 executable as well as the current build. It verifies cross-version
preservation of uploads, shared releases and pins, including callback loss,
Root replacement and explicit activation, through the existing upgrade workflow.

### 🩹 Fixed

Store maintenance status, progress and fleet views identify the exact attempt,
so successive upgrades are distinguishable. Detailed output includes the qualified
eight-attempt history limit; it does not infer remaining capacity or current
Store readiness. The demo no longer lists repeat maintenance as pending.

For an existing Store-backed fleet, these commands only read observations or
retained operation evidence; use the operation digest returned by planning:

```sh
canic fleet show --app starter --fleet local
canic fleet show --app starter --fleet local --details
canic fleet status OPERATION --app starter --fleet local
```

### ➕ Added

`fleet artifact list --details` shows exact Wasm and authority bindings alongside
the operation IDs needed for status and recovery. The ordinary listing stays
compact, and JSON retains its existing shape. For a fleet with retained publication
evidence, copy the displayed outer Operation into the status command:

```sh
canic fleet artifact list --app starter --fleet local --details
canic fleet status OPERATION --app starter --fleet local
```

## [0.216.0] - 2026-11-15

### ✨ Added

Fleet planning supports repeat Wasm Store upgrades through the existing Root-owned
maintenance path. Root retains up to eight exact attempts, fences stale callbacks
by approval and phase, and preserves all history through its own upgrades. An
unfinished or uncertain operation cannot be replaced, and full history is never
evicted. The installed CLI witness completes two Store upgrades, recovers an
interrupted second upload, then upgrades Root and its application.

### ▶️ Try it

For a completed local starter fleet with a prepared signer, first upgrade Root to
the current executable in a separate approved window. Then change only the Store
Wasm to a reviewed compatible replacement and use the returned operation digest:

```sh
canic fleet plan --app starter --fleet local --identity canic-local
canic fleet status OPERATION --app starter --fleet local
canic fleet apply OPERATION --app starter --fleet local --identity canic-local
```

After interruption, retain the same approval; reconciliation observes issued work:

```sh
canic fleet reconcile OPERATION --app starter --fleet local --identity canic-local
```

### ⚠️ Compatibility

New plans require the history-aware Root endpoints. Historical plans keep their
original encoding, digests and signed methods. CUJ7 reopens recognized prior
journals, including a qualified 0.215 executable, in the existing allocation;
older executables refuse the new state. Rust `UpgradeStore` literals gain a
sequence and `StoreMaintenance` gains history. Limits remain experimental: eight
attempts, 1 MiB replacement Wasm, no reset/cancellation or mainnet qualification.

### 🧹 Cleanup

This extends the existing engine, approval codec and planner. Store stays at its
qualified supplier revision; no new crate, allocator, workflow engine or mirrored
artifact catalog is introduced. Prior publication evidence remains reusable
across the validated Store upgrade chain.

## [0.215.0] - 2026-11-15

### ⚠️ Breaking

The Store implementation and protocol move to `dragginzgame/ic-wasm-store` as
`ic-wasm-store` and `ic-wasm-store-protocol`. Existing source recipes must change
their Store `build.package` to `ic-wasm-store`, retaining the Canic manifest path.
Rust consumers use the new protocol package name. Endpoint identities, Wire/state
formats, allocation names and retained approval bytes are unchanged; no reinstall
or journal migration is required.

### ▶️ Try it

From the Canic checkout, with the updated CLI installed and dependency-fetch access:

```sh
make canic
canic artifact build --package ic-wasm-store --output target/wasm-store.wasm
```

For an existing starter with the Store recipe updated (build only; no deployment):

```sh
canic app build --app starter
canic fleet artifact list --app starter
```

### 🧹 Cleanup

Canic consumes an immutable Git revision and builds the standalone Store from that
same source and its committed lockfile. No sibling checkout or copied runtime is
required. Supplier persistence tests and standalone installed qualification move
with their owner; Root orchestration and end-to-end fleet qualification stay here.
The extraction follow-up removes the unused Store-candidate fixture variable and
the redundant `store-doc` target; `root-doc` already owns management documentation.
Store harness linting no longer repeats the native management lint pass.

## [0.214.0] - 2026-11-15

### ⚠️ Breaking

The native Host `status` API now returns typed operation evidence instead of a
JSON value. Rust callers should match the returned operation and use its fields;
serialize it when JSON is needed. CLI JSON, retained journals and approval bytes
keep their existing formats. No fleet migration or reinstall is required.

### ▶️ Try it

With a configured starter fleet and its retained journal, inspect local Wasm
publication evidence without contacting the network or opening a browser:

```sh
make canic
canic fleet artifact list --app starter
canic fleet artifact list --app starter --json
```

### ✨ Added

`fleet artifact list` shows immutable version labels, artifact sizes and retained
publication completion, grouped by Store. It includes both materialized whole-fleet
bootstrap publications and separate publication approvals. JSON preserves exact
hashes, Root/Store code bindings and the operation address for `fleet status`.
Missing completion stays **unconfirmed**, even if a remote publication may have
succeeded. Invalid journal entries remain explicit partial results with exit 1;
they cannot become publication or deployment receipts. App, fleet and exact
network bindings keep histories separate. A missing journal is left absent.

This is historical local evidence, not the complete live Store catalog, current
availability or installed application versions. No latest-version alias, copied
catalog, new runtime endpoint or deployment authority is introduced.

### 🧹 Cleanup

Fleet inspection reuses validated plans and artifacts within the journal lock,
and status flows directly from Host to CLI without JSON round trips. Host owns
Store continuation classification; CLI renders its recommendation. Historical
recovery readers and independent canister authority checks remain intact.

## [0.213.0] - 2026-11-15

### ▶️ Try it

From this checkout, install the development CLI on PATH, then generate a new
starter in an unused directory and build its Root, Store and application locally:

```sh
make canic
canic app init wire-demo --fleet mainnet --network ic \
    --source wire-demo
canic app build --app wire-demo
canic fleet canister list --app wire-demo
```

This creates configuration and Wasm artifacts; it does not deploy, spend cycles
or open a browser. Rust 1.100.0 with the Wasm target and dependency access are required.

### ✨ Added

The profile manager can serve as the management owner for a Store-backed profile
fleet. Its existing profile readiness check, private user state and versioned
artifact custody survive worker, Store and owner upgrades. An installed application
with the wrong readiness protocol cannot receive a completion receipt.
The installed witness also checks continued private writes, stale-write rejection
and a subsequent approved worker code change after infrastructure maintenance,
preserving current data when reselecting a compatible earlier executable.
Lost worker-upgrade callback evidence is qualified through explicit reconciliation:
the existing approval, signed admission and single installation attempt survive.

For an admitted worker upgrade that reports an uncertain outcome, retain its
operation and journal. Inspect it and request reconciliation with the same identity:

```sh
canic fleet status UPGRADE_OPERATION --app profile-store
canic fleet reconcile UPGRADE_OPERATION --app profile-store \
    --identity canic-mainnet
```

Reconciliation requests evidence recovery under the original approval; it does
not authorize another installation or guarantee that every uncertainty is recoverable.
Pending worker-upgrade output now prints the appropriate next command: reconciliation
for issued/uncertain effects, apply for ordinary progress, and status for expired or
reserve-blocked work. It retains the selected app, fleet, paths and signer; it no
longer suggests repeated apply calls for a known uncertain installation.

After following the [profile fleet setup](docs/demos/profile.md#store-backed-profile-fleet),
build locally and inspect the configured fleet without opening a browser:

```sh
canic app build --app profile-store
canic fleet show --app profile-store
```

This is local composition qualification for one profile worker. Existing mainnet
deployments remain unchanged; the default generated starter still uses its greeting Root.

### 🔧 Changed

Adopted Toko Tooling 0.44.4 and IC Wire 0.24.1 in both workspaces. Canic's
runtime, host, generated starter and qualification callers now use Wire's explicit
payload/frame encoding API without a compatibility facade. Blob's pinned supplier
still owns its internal Wire 0.22.4 dependency; the optional adapter preserves its
numeric RPC limits while using Canic's selected codec.

Demo and qualification package directories now match their Cargo package names.
For example, the greeting lives at `apps/hello/canic-hello`, and the native
qualification harness at `qualification/canic-qualification`. New starter source
lives at `starter-app/`; its configured canister slot remains `app`. Existing
generated projects are not rewritten. If updating their Wire dependency, migrate
the endpoint calls to the explicit `Encoding::Frame` API as well.

Root's async observation exclusion now releases during callback-trap cleanup.
Previously a committed worker upgrade could remain blocked until Root was replaced,
because the trapped task left its heap busy flag set. Explicit reconciliation can
now inspect that original outcome without resetting its durable effect journal.

Cargo build and lockfile-preparation failures now include a bounded, escaped
diagnostic: the first compiler error when available, otherwise Cargo's stderr.
The original process failure remains attached. Failed builds preserve the previous
Wasm output. For an existing starter, fix the reported source or recipe, then rebuild
and review the resulting fleet plan with a prepared identity:

```sh
canic app build --app starter
canic fleet plan --app starter --identity canic-mainnet
```

Building is local; planning retains approval without deploying or spending cycles.

## [0.212.0] - 2026-11-15

### 🔧 Changed

New source starters declare Root, a separate versioned Wasm Store and one greeting
application. Root/Store use the supplying Canic workspace; the app stays independent
of Store implementation dependencies. Existing projects and journals are unchanged.
From the Canic checkout, choose an unused directory; initialization and build are local:

```sh
canic app init starter --fleet mainnet --network ic --source starter
canic app build --app starter
canic fleet plan --app starter --identity canic-mainnet
```

Planning requires a prepared identity and retains an approval without network
effects. Review the default **4T** creation budget (3T Root including app funding,
1T Store) before applying. Store-backed changes publish new bytes before planning
the child upgrade; the generated README includes both steps. Existing inline
fleets keep their original path. No browser opens automatically.

The installed starter witness now edits and rebuilds the generated application
itself, publishes its new version and verifies the changed greeting at the same
principal. An unchanged rebuild leaves artifacts and Cargo.lock unchanged and
requires no new deployment approval. After completing an upgrade, read its greeting:

```sh
canic fleet call UPGRADE_OPERATION --app starter --name team
```

Scheduled child status labels the original admission cutoff separately from the
effect deadline. A past dispatch timestamp no longer uses the generic `expired`
label, which could be mistaken for deployment failure. Authority and JSON are
unchanged. Inspect an existing operation without network effects:

```sh
canic fleet status OPERATION --app starter --fleet local
canic fleet status OPERATION --app starter --fleet local --json
```

## [0.211.1] - 2026-11-15

### 🔧 Changed

New fleet and named child plans dispatch once approved prerequisites are ready,
with a fixed ten-minute effect window from planning. Bootstrap and publication
consume that same window. Explicit future `--execute-at` schedules and retained
plans keep their original admission cutoffs, timestamps and digests. Use a Root
built with this timing contract; older Roots require a future schedule or reviewed
Root maintenance. For a built/configured local starter and prepared local signer:

```sh
canic fleet plan --app starter --fleet local --identity canic-local
canic fleet apply OPERATION --app starter --fleet local --identity canic-local
canic fleet status OPERATION --app starter --fleet local
```

Copy the returned operation digest after reviewing the plan. Apply spends its
approved budget; status only reads the journal. No scheduling override is needed.

Store upgrade status summarizes retained attempt states instead of listing every
catalog-read and continuation row. Unknown outcomes and rejections stay visible;
counts never establish completion. Full attempt details remain in `--json`.
Both forms read only the selected local journal and open no browser:

```sh
canic fleet status STORE_OPERATION --app starter --fleet local
canic fleet status STORE_OPERATION --app starter --fleet local --json
```

Store maintenance output shows before/after catalog scan progress and suggests
`reconcile` for issued effects or verification. Fresh-effect phases suggest `apply`
only within the original dispatch window and without a retained failure. Expired
or failed continuations instead point to local status review. The runtime still
owns effect admission; these hints grant no retry or replacement authority.
For an already-approved operation with an issued effect, use its original signer:

```sh
canic fleet reconcile STORE_OPERATION --app starter --fleet local --identity canic-local
```

The installed publication witness now records separate local timings for planning,
interrupted prepare, reconciliation, resumed upload and warm version reuse. It
decodes retained chunk requests to check the cold unique-chunk payload total and zero payload
uploads for the reused version. This adds measurement evidence, not a deployment
speedup or a change to existing dispatch deadlines.

Root admission now treats `execute_at_ns` as the earliest dispatch time: creation
and upgrade may be admitted after it, before the original expiry. The existing
timer and effect journal still own execution. Installed Store-backed qualification
covers immediate work, future scheduling, expiry, replay and Root maintenance.
Wire layouts and finite limits are retained. The installed CLI journey qualifies
fresh default-timed fleet creation and a later child upgrade, including intervening
Store recovery and Root maintenance. These are local synthetic-cycle checks, not
mainnet timing measurements.

## [0.211.0] - 2026-11-15

### ⚠️ Breaking

Host's `StoreMaintenance` Rust struct adds `operation`; callers constructing it
must supply `None` for the existing child-only contract or the exact retained
Store operation for complete Root custody. Existing serialized approvals omit
the absent field and retain their original digests and signed read methods.
`PlanAction` adds `UpgradeStore` and `ApplyOutcome` adds `StorePending`.
`StoredArtifact` adds optional `maintenance`: literal callers use `None` before
Store replacement; subsequent whole-fleet child plans bind its completed attempt
and select complete Root custody. Absent fields preserve existing encodings.

### ➕ Added

Root's explicit `recover_store_maintenance` contract settles already-issued
closure, stop, upload, install, start and activation through observation. It can
finish the post-install catalog scan, but never starts a new effect or activates
a verified replacement. Replay, concurrent recovery and expired dispatch authority
retain the original operation. Existing methods and stored formats are unchanged.
Host reconciliation uses this restricted contract; live use remains unqualified.

Root exposes one controller-only `observe_root_custody` snapshot of both child
slots, artifact references and the surviving Store operation. Host Root maintenance
can bind that operation and compare the complete snapshot before and after its
existing direct installation. Older child-only readers still refuse to omit it.
Whole-fleet planning now selects one Store replacement when its configured Wasm
changes, after completed child and publication work. Existing `apply`, `status`
and `reconcile` retain the original approval, signed attempts and authenticated
completion. Subsequent Root maintenance preserves the Store operation; child
publication/upgrades bind the new code and complete custody. Reconcile never
starts installation or activation. This local qualified path permits one Store
attempt with a 1 MiB replacement; successor maintenance remains blocked.

`fleet compare-subnets LEFT RIGHT` compares two explicit candidates on a configured
mainnet fleet using Query's existing resource, configuration, coverage and overlap
evidence. It does not choose a destination or deploy anything. Partial results stay
visible and exit 1; `--json` retains the full supplier evidence.

### ▶️ Try it

Build the development Root locally and inspect an existing selected fleet. These
commands do not deploy, recover or activate a Store, or open a browser:

```sh
canic artifact build --package canic-root --output target/root.wasm
canic fleet show --app starter --fleet mainnet --details
```

For an already completed local Root/Store/app fleet, set the Store's Wasm input to
the reviewed replacement, then plan and inspect it. Use the returned operation
digest below; `apply` performs maintenance, while `reconcile` only settles issued
effects. Keep the same operation if interrupted; a bounded apply may need repeating.

```sh
canic fleet plan --app starter --fleet local --identity canic-local
canic fleet status OPERATION --app starter --fleet local
canic fleet apply OPERATION --app starter --fleet local --identity canic-local
canic fleet reconcile OPERATION --app starter --fleet local --identity canic-local
```

After `make canic`, from an existing configured app directory, compare two mainnet
subnets without signing, changing the journal or opening a browser:

```sh
canic fleet compare-subnets \
  tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe \
  brlsh-zidhj-3yy3e-6vqbz-7xnih-xeq2l-as5oc-g32c4-i5pdn-2wwof-oae \
  --app starter --fleet mainnet
```

## [0.210.0] - 2026-11-15

### ⚠️ Breaking

Root's CUJ6 journal retains Store metadata commitments and installation evidence. Recognized
CUJ5 records load without inventing preservation evidence; downgrade is unsupported.
New Store-maintenance attempts require Store's closed-catalog observation contract.
Older Stores without that endpoint remain blocked, with no fallback installation.
Store-maintenance observations use their scoped Wire budget; child budgets are unchanged.

### ▶️ Try it

After `make canic`, build and inspect the development Store locally. These commands
spend no cycles and do not upgrade an existing canister or open a browser.

```sh
canic artifact build --package canic-wasm-store --output target/wasm-store.wasm
canic artifact inspect target/wasm-store.wasm
```

For an existing `starter` app and `mainnet` fleet, inspect its current deployment
without applying this experimental runtime upgrade:

```sh
canic fleet show --app starter --fleet mainnet
canic fleet show --app starter --fleet mainnet --details
```

### ➕ Added

- Add `canic fleet placements` for configured mainnet bindings, using IC Query's
  bounded placement collection pinned to Git revision
  `0a3eca919f05d777798406087049cc51ab7ae2a8` (topology 0.106.1).
  Keep unbound slots and partial outcomes visible, retain source evidence in JSON,
  and reject local networks or more than 32 unique bound IDs before IO. This is
  a one-shot diagnostic: no journal, signer, discovery or deployment effects.
  Partial or all-unbound output exits 1; complete selected reads exit 0.
  Text summarizes read/configured bindings, unresolved bindings, unbound slots
  and distinct subnets from successful reads, without extra queries or JSON changes.

  After `make canic`, from an existing configured app directory:

  ```sh
  canic fleet placements --app starter --fleet mainnet
  canic fleet placements --app starter --fleet mainnet --json
  ```

  See [configured placement diagnostics](docs/design/fleet-placements.md).
  Only explicitly configured identities are included; retained creation outputs
  are not discovered. Query packages remain unpublished, selected by immutable Git
  revision rather than a sibling-path dependency.

Root captures every closed Store release before stopping, including unfinished
uploads, version aliases and shared pins. Each continuation reads one entry;
Root retains only a digest, counters and cursor. Root replacement resumes the
scan and concurrent continuations cannot count a page twice. Matching release
count and an explicit end page establish completion. This is metadata evidence,
not a payload integrity scan.

Root can now upload and install the approved Store replacement once, restart it
with admission closed, compare the preserved catalog, and activate only on an
explicit matching request. Host supplies at most 1 MiB of raw Wasm and retains
its durable copy; Root stores no Wasm. Installed tests preserve every Store stable
byte and recover lost callbacks across Root replacement. Expiry allows observation,
not new effects; rejected or incompatible replacements stay blocked.
One maintenance attempt remains retained. Host/CLI upgrade execution, repeatable
maintenance and live use are not yet qualified.

`fleet show` includes a compact, uncertified Store-maintenance report from Roots
named by retained Store deployment plans. Details show the before/after catalog
scan and original dispatch deadline; JSON and localhost reports carry the same
evidence. Missing methods and foreign Store reports stay unknown. Completed
maintenance is historical, not a claim about current Store admission or readiness.
Viewing never signs, resumes work or changes the journal. Older Roots require a
separately qualified upgrade before this query is available.

Store-maintenance admission refuses anonymous operators before retaining state,
matching Root's existing reopen invariant even in locally controlled fixtures.

## [0.209.0] - 2026-11-15

### ⚠️ Breaking

Store's error contract adds `Closed`. Its settings codec now retains maintenance
admission in `CWS2`, loading recognized `CWS1` settings without inventing a prior
attempt. Older executables cannot read the new settings; downgrade is unsupported.
Root's CUJ5 journal retains Store-maintenance intent beside child journals,
loading prior recognized records without a maintenance attempt. Older executables
cannot read CUJ5. Publication's error contract adds `Maintenance`.

### ▶️ Try it

From this checkout after `make canic` and Wasm toolchain preparation, build and
inspect the development Store locally. These commands spend no cycles and open
no browser; they do not upgrade an existing Store.

```sh
canic artifact build --package canic-wasm-store --output target/wasm-store.wasm
canic artifact inspect target/wasm-store.wasm
```

### ➕ Added

Store's replicated `observe_custody` reports one release's upload progress and
every retained pin on its shared artifact, including other release labels and
former controllers. Root explicitly mounts `observe_store_custody` through its
existing controller/deadline checks. Observations read existing metadata, retain
no Root copy and grant no maintenance authority. This is preservation evidence
for the next Store upgrade flow; Host-driven Store upgrades remain blocked.

Store now persists a controller-owned close/observe/activate attempt in its
existing settings allocation. Publication, pins, quota changes and fresh cache
work stop while closed; reads remain available. Upgrade/reopen does not activate
the Store. Exact activation and a monotonic sequence prevent old requests or
cache callbacks from reopening or continuing a newer maintenance attempt. Root
forwards this fixed contract through `store_admission`; it does not install code.
The installed witness covers lost replies, preserved pins/uploads across Store
and Root upgrades, stale activation and an interrupted cache repair. Closure is
not proof of drained calls.

Root now retains one Store-maintenance approval, fences competing mutations,
closes Store and issues stop once. Installed tests recover lost closure/stop
callbacks and preserve the journal across Root replacement; stopped-state
observation can settle after the original dispatch deadline. This qualification
primitive ends at Stopped and has no CLI entrypoint: installation, restart,
explicit activation and repeatable maintenance remain pending. Do not use it live.

## [0.208.0] - 2026-11-15

### ⚠️ Breaking

Fresh whole-fleet plans with a declared `wasm-store` now publish application
artifacts there before admitting children. Unpublished child artifacts require a
publication plan/apply step, followed by a separately reviewed upgrade plan/apply.
Host library callers must handle `FleetPlanning::Publication` and supply the
optional `FleetChild.release` field. Retained inline approvals remain unchanged.
`PlanAction::MaintainRoot` also carries optional Store custody; use `None` for
inline maintenance.

### ▶️ Try it

After `make canic`, run from a fresh app's config directory with built Root,
Store and application Wasms declared. Select your app/fleet; planning spends
nothing, while apply funds and installs the approved fleet. No browser opens.

```sh
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
canic fleet apply FLEET_OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet status FLEET_OPERATION --app starter --fleet mainnet
```

After creation completes, change and rebuild one child, then run `fleet plan`
again. Apply the returned publication operation, replan, and review/apply the
returned upgrade operation. See the [Store workflow](docs/demos/wasm-store.md#whole-fleet-artifacts).

### ➕ Added

Whole fleets bind immutable releases by application name and full content hash,
reuse shared bytes, and support bounded successive child upgrades. Both releases
must have retained publication evidence before either child is admitted. Existing
signed attempts survive interrupted publication and admission. Root keeps
references; Store owns application bytes and cache rebuilding.

Root maintenance now preserves the complete Store references, pins and historical
upgrade deliveries. It verifies Root, child and Store code/controller bindings,
compares full custody before and after installation, and recovers the original
install attempt after an interrupted completion check. Subsequent child work uses
the upgraded Root's code binding and can reuse releases whose original publication
completion is retained. Unfinished publication authority is never renewed by a
Root upgrade. See [Root maintenance](docs/demos/root.md#root-maintenance).

### 🩹 Fixed

Store bootstrap now identifies both unsupported release selection and scheduling
in its refusal message.

## [0.207.0] - 2026-11-15

### ⚠️ Breaking

Host library callers must supply the new optional publication-operation argument
when creating a managed plan, and an explicit optional artifact source when
constructing a management owner. Use `None` for inline delivery. Existing CLI
commands and retained inline approvals keep their contracts.

### ▶️ Try it

Run `make canic` from this checkout to install the updated CLI on PATH. From an
app's config directory, with Root/Store bound and publication completed in the
same fleet, select that exact release for the sole application child. Its configured
Wasm must match the publication. Substitute the reviewed hash and operation IDs;
apply authorizes Root to spend its creation grant and install the child. No browser opens.

```sh
canic fleet plan --app starter --fleet mainnet --canister app --release PUBLICATION_OPERATION --root-module ROOT_SHA256 --identity canic-mainnet
canic fleet apply DEPLOYMENT_OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet status DEPLOYMENT_OPERATION --app starter --fleet mainnet
```

### ➕ Added

Named child creation and its first upgrade can use an immutable Store release,
including artifacts above the inline limit. Root retains references and pins;
Store owns bytes and cache reconstruction. Lost admission replies recover through
source-bound custody without replacing signed attempts. The [Store guide](docs/demos/wasm-store.md#deploy-a-published-release)
includes upgrade commands and the existing dispatch schedule. Whole-fleet release
selection and migration from inline custody remain pending.

### 🧹 Cleanup

Fleet convergence reads typed Store completion evidence instead of status JSON;
Store readiness and its status view share one observation limit.

## [0.206.0] - 2026-11-15

### ▶️ Try it

From this checkout, install the updated CLI on PATH and build Store locally
after tool/dependency preparation:

```sh
make canic
canic artifact build --package canic-wasm-store --output target/wasm-store.wasm
```

In the app's config directory, with an existing bound Root, reviewed Root hash,
and an unbound `store` definition (`role: wasm-store`, `parent: root`, built Wasm
path), plan its bootstrap. These examples select `starter/mainnet`; substitute
the configured app/fleet and returned operation. Planning spends nothing; apply
debits the signer's cycles-ledger account up to the approved creation ceiling
(default 1T), installs Store and hands sole control to Root. No browser opens.

```sh
canic fleet plan --app starter --fleet mainnet --canister store --root-module ROOT_SHA256 --identity canic-mainnet
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet status OPERATION --app starter --fleet mainnet
```

The [Store guide](docs/demos/wasm-store.md) includes declaration, binding and
publication. Installed qualification uses local synthetic cycles; no live
mainnet bootstrap was performed for this increment.

For a fresh, unbound fleet whose built definitions include Root, Store and one
or two applications, omit `--canister` to review all creation together. Defaults
are 4T total for one application or 5T for two; applying spends that reviewed
budget and admits apps only after Store handover/readiness completes:

```sh
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet show --app starter --fleet mainnet
```

### 🩹 Fixed

Artifact build and inspection now accept the Store’s 8 MiB payload capacity.
Direct deployment plans retain their existing 1.5 MB transport ceiling; Root’s
inline child limits also remain unchanged. With the Wasm toolchain and
dependencies prepared, build and inspect locally:

```sh
canic artifact build --package canic-hello --output target/hello.wasm
canic artifact inspect target/hello.wasm
```

The publication witness now inspects and uploads a real Wasm larger than 1.5 MB.
The aggregate lint and CI targets include the existing optional Blob checks,
using its pinned immutable source. No browser or mainnet operation is involved.

### ✨ Added

Root can now observe initialized Store state after controller handover, using a
bounded replicated read. Installed tests cover handover, refusal, reply failure
and Root upgrade while preserving both empty application slots. Named CLI Store
bootstrap now combines this read with certified code/controller checks, original
creation/install custody and an exact handover attempt. It retains Root's subnet
before creation and selects that placement. A lost handover reply can be reconciled
without repeating creation/install or renewing mutation authority. Whole-fleet
bootstrap composes this same Store action under one approval, with a separate
budget and retained evidence. Both app slots remain available. Status and fleet
views include Store; cancellation and replan preserve its pending effects.
Store artifact replacement and existing-fleet adoption remain refused.
Store-backed release selection remains pending.
Build its two production artifacts locally after `make canic` and tool preparation:

```sh
canic artifact build --package canic-root --output target/root.wasm
canic artifact build --package canic-wasm-store --output target/wasm-store.wasm
```

## [0.205.1] - 2026-11-15

### 🔧 Changed

Adopt Toko Tooling 0.44.3 through the verified installer, including the shared
post-push review rule and compact LOC reporting. Qualify the current
Platform 0.33.7 / Wire 0.22.3 selections in both workspaces. Runtime remains
0.22.5 and Types 0.20.4; compatible manifest minimums and unrelated dependencies
are retained. Rebuild the installed CLI with `make canic`, then inspect commands
and local fleet declarations without network effects or browser launches:

```sh
canic --help
canic fleet list
```

### ➕ Added

The optional `canic/blob` adapter mounts captured-caller Blob endpoints without
owning service state or lifecycle. A real embedded asset catalog qualifies shared
references, user isolation, atomic catalog/reference rollback, non-resurrecting
replay and exact-attempt maintenance across two executables. It consumes the
exact Blob 0.100.6 Git revision, preserving the originally qualified library; this is local composition,
not provider qualification or fleet-managed Blob deployment.

With authenticated Git access to Blob and the Wasm toolchain prepared, build/inspect the
qualification application locally. These commands do not deploy or open a browser:

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --output target/blob-catalog.wasm
canic artifact inspect target/blob-catalog.wasm
```

Use `make blob-test` for the installed witness after simulator preparation.
See [Blob composition and qualified inputs](docs/demos/blob.md).

A standalone Wasm Store retains multiple immutable versions, resumes chunk uploads
after maintenance, reuses identical content and exposes bounded history and quota
updates. Caller-owned operation pins survive maintenance; bounded cache requests
reuse present chunks or reconstruct missing ones from stable bytes. Root now
supports Store-backed child creation and serial upgrades without retaining
application Wasm. Exact references, retention evidence and historical sources
survive Root maintenance; lost pin/cache replies can be reconstructed without
reissuing creation or installation. Root now also forwards bounded publication
commands after Store controller handover, without storing artifact bytes or a
second catalog. Interrupted uploads resume after Root maintenance; a failed Root
reply preserves the Store commit and can be recovered by observation. Fleet
plan/apply now publishes explicit immutable releases through an existing Root-owned
Store, retaining exact approval and signed-attempt custody. Reconcile observes
without renewing mutation authority. Publication receipts remain separate from
deployments; old plan digests stay unchanged. Pin release and deletion remain
unavailable.

With a configured `starter/mainnet` fleet, app-defined `app` Wasm and bound
`root`/`store` identities, substitute their reviewed hashes and the returned
operation digest. Root must solely control Store; the signer must solely control
Root. Apply uploads/publishes only; it does not deploy an app:

```sh
canic fleet plan --app starter --fleet mainnet --canister app --publish-version 1.0.0 --store store --root-module ROOT_SHA256 --store-module STORE_SHA256 --identity canic-mainnet
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet status OPERATION --app starter --fleet mainnet
```

From this checkout with `make canic` and the Wasm target prepared, build and
inspect Store and Root locally; these commands do not deploy or open a browser:

```sh
canic artifact build --package canic-wasm-store --output target/wasm-store.wasm
canic artifact inspect target/wasm-store.wasm
canic artifact build --package canic-root --output target/root.wasm
canic artifact inspect target/root.wasm
```

See the [Store scope and local qualification](docs/demos/wasm-store.md).

### 🩹 Fixed

Retained maintenance status now reports the actual preflight observations selected
for the approved Root. The supported predecessor shows separate creation and
upgrade custody results instead of an incorrect generic “not submitted” step.
Each child's detailed fleet status is labelled with its retained app canister name.

With a retained operation in app `starter`, inspect it without sending requests:

```sh
canic fleet status OPERATION --app starter
canic fleet status OPERATION --app starter --json
```

### 📚 Documentation

The managed-fleet design now selects a separate dynamic, versioned Wasm Store.
It specifies immutable releases, retained history, resumable uploads, reuse,
retention pins and independent maintenance. Root keeps approval/effect evidence
instead of being the long-term artifact repository. Deployment-latency measurement
and a qualified transition from current Root custody are required. The standalone
Store above does not yet change fleet deployment or the dispatch default.

## [0.205.0] - 2026-11-15

### ⚠️ Breaking

Host plans now contain one explicit `PlanAction`: direct creation, direct upgrade,
Root-owned creation, Root-owned upgrade, or Root maintenance. Rust callers supply
that action instead of combining optional target/management/maintenance fields.
`FleetPlan.children` is an ordered collection with permanent slot identities;
maintenance contains an ordered custody collection. `ManagedOwner.slot` remains
`None` for the original unscoped request contract.

Saved approval JSON, digests, journal paths and signed-attempt bytes are unchanged;
private record codecs retain the old fields. No migration or renewed approval is
needed. Fleet apply/reconcile JSON now returns `children: [{name, outcome}]`, and
fleet status returns `children: [{name, state}]`; consumers must use these named
collections instead of top-level `child`/`sibling`. The nested `fleet_plan`
approval retains its original encoding.

### ➕ Added

Fleet plan/apply now supports a new Root with one or two direct children. Each
child retains its own creation, eight-entry upgrade history and recovery evidence.
The same CLI plans independent child upgrades and Root maintenance covering both
complete journals. Terminal and HTML fleet views show both named children.

Create a fresh two-child greeting project from this checkout, with the Wasm target
prepared. Initialization/build are local only; these commands open no browser:

```sh
canic app init siblings --fleet mainnet --network ic --source siblings
canic app canister add worker --app siblings --parent root --role worker --wasm artifacts/worker.wasm --manifest Cargo.toml --package siblings-app
canic app build --app siblings
canic fleet plan --app siblings --identity canic-mainnet
```

Planning retains a local approval. With the signer prepared and cycles supplied,
review its budget before applying: the two-child default is 4T cycles. Replace
`OPERATION` with the displayed approval ID; apply creates real mainnet canisters:

```sh
canic fleet apply OPERATION --app siblings --identity canic-mainnet
canic fleet show --app siblings
canic fleet call OPERATION --app siblings --canister worker
```

Change one artifact per upgrade approval. Existing one-child fleets retain their
original format and cannot acquire a second child through replanning. General
trees, dynamic growth and adopting existing deployments remain unqualified.

### 🔧 Changed

Help headings put the version and command description on one line, for example
`Canic 0.205.0 : Manage Canic apps, fleet deployments and Wasm artifacts`.

```sh
canic --help
canic fleet --help
```

The v4 journal and v3 artifact payload retain both children inside the original
IC Storage allocations. Combined creation/current-upgrade Wasm custody is capped
at 1,048,512 bytes; admission refuses overflow without changing either child.
Each artifact still has the existing 512 KiB ceiling. Migration refuses an older
payload above the new aggregate bound and rolls back the executable change.
Original signed request contracts retain their first-slot meaning. Old full-custody
reads refuse an occupied second slot; the new controller-only observation includes
both slots. Multi-child maintenance verifies both live children before and after
installation; untracked work in either child blocks the upgrade.

Inspect a retained operation without submitting work or opening a browser:

```sh
canic fleet status OPERATION --app siblings
canic fleet status OPERATION --app siblings --json
```

### 🧹 Cleanup

Share signed method selection across creation, upgrade, recovery and fresh-read
authorization. Retain a golden unsigned v1 fleet approval to guard old digests.
Remove one-child assumptions from convergence and fleet observation instead of
adding a parallel workflow. `app canister add` now accepts the existing manifest/
package build recipe, so adding a child can preserve the normal `app build` flow.
Keep names and slots bound to retained approvals even when configuration changes.

Qualification no longer uses M0/M1 package names: one native
`canic-qualification` harness owns shared simulator preparation and installed
tests, alongside the effect and artifact probes. Use `make qualification-setup`,
`make effect-test`, `make artifact-custody-test` and `make journey-test`;
the old Make names have no aliases. The prepared server now lives under
`target/qualification/server` (or the shared `IC_TESTKIT_SERVER_DIRECTORY`
override). Existing outputs are preserved. Unique failure scenarios remain;
dated M0 decisions and later evidence move to `docs/qualification/history.md`.

## [0.204.1] - 2026-11-14

### ➕ Added

Qualify maintenance from the retained pre-0.204 single-upgrade Root through
`fleet plan` and `fleet apply`. Its exact module hash selects two authenticated
preflight reads; the replacement must retain the same complete journal through
the current custody endpoint. Empty and completed slots migrate; untracked
scheduled work blocks installation. Unknown older binaries have no fallback.
Keep original plans and signed attempts, including interrupted-install recovery.

The generated `starter/mainnet` demo now runs this transition: Root and app keep
their identities, then the maintained Root upgrades the greeting app. Local
qualification covers interruption recovery; the mainnet run verifies the normal
path. See the [retained journey evidence](docs/qualification/history.md#mainnet-predecessor-transition-and-subsequent-app-upgrade).

### ▶️ Try it

For a completed generated fleet whose predecessor is qualified, build the current
Root into its declared input, then review and apply its maintenance plan. Keep
the operator window exclusive and change the app in a later approval:

```sh
canic artifact build --package canic-root --output starter/artifacts/root.wasm
canic fleet plan --app starter --identity canic-mainnet
canic fleet apply MAINTENANCE_OPERATION --app starter --identity canic-mainnet
canic fleet show --app starter
```

The current demo is already complete. With the maintainer's local `starter/`
project and its retained journal, inspect it or call its upgraded app:

```sh
canic fleet show --app starter
canic fleet call 0a68a8de48d4ca642147087a893813359ca4b7eda6847cf554acfaaa91c757cc --app starter --name team
```

### 🧹 Cleanup

Limit the plan retention helper to its owning module after the post-push review found no
remaining cross-module callers. No plan format, command alias or runtime
persistence schema changes are needed for this increment.

### 🔧 Changed

Qualify the selected IC Timers 0.44.3 lockfile update with the release suite.
Keep the operator's generated `starter/` project and deployment journals local;
its reusable source templates and the exact predecessor regression fixture ship
with Canic.

## [0.204.0] - 2026-11-14

### ⚠️ Breaking

**Observation contract:** replicated `observe_upgrade` now requires the exact
approval (ID 29); public latest-status queries retain ID 25. Historical on-disk
responses remain readable, with no unbound live fallback or re-signing of old
attempts. Use a matching reviewed Root; existing mainnet owners and operation
bindings are not automatically migrated. The v3 journal reads v1/v2 payloads in
the original two allocations, retaining historical outcomes and current artifact
bytes. Qualification covers three CLI upgrades, eight runtime upgrades, full
history refusal, owner maintenance and scheduled/completed v2 migration.

Host `Plan` adds optional maintenance custody, and fleet-planning JSON adds
`action: "maintenance"`. Existing records without custody retain their original
bytes and digests; Rust callers constructing plans must supply `maintenance: None`
for ordinary deployment.

Host callers must replace `create_fleet_plan` with `plan_fleet` and handle the
`FleetPlanning` variants `Create`, `Upgrade`, `Maintenance` and `Unchanged`
instead of destructuring a creation-only tuple.

### ➕ Added

`fleet plan` now compares the declared artifacts of a completed Root-plus-app
fleet and prepares its next child upgrade or a separate Root maintenance plan. Resolve the child, previous code and
Root authority from retained completion; no copied principal or hash is needed.
Reuse the existing upgrade plan, signer and runtime engine. Matching artifacts
report no changes against retained completion, without claiming a fresh network
check. An unfinished operation keeps its original approval and deadlines.

### ▶️ Try it

From a generated project with a completed fleet, edit the application source:

```sh
canic app build
canic fleet plan --identity canic-mainnet
canic fleet status UPGRADE_OPERATION
```

Review the returned upgrade digest, then admit it; apply again after dispatch
to retain completion. Select `--fleet NAME` when there is more than one fleet:

```sh
canic fleet apply UPGRADE_OPERATION --identity canic-mainnet
canic fleet show
canic fleet plan --identity canic-mainnet
```

Unrelated individual approvals and simultaneous Root/child changes remain blocked. The
experimental Root now retains up to eight serial child upgrades. Each successor
requires verified successful completion; failed, expired or uncertain work stays
blocked. History is never evicted, and old approvals replay their historical
outcomes without changing current work. Repeat the build/plan/apply examples for
each new change. Qualification exercises lost-reply recovery,
two isolated fleets and persistent profile data/ownership through the CLI path.
The profile manager now mounts authenticated creation `observe` for that same
host workflow. Existing mainnet canisters are unchanged; no browser opens.

Root-only changes use the same plan/apply commands after all child work completes.
Maintenance verifies the full journal before and after installation, preserves
controllers and identities, and binds future child approvals to the new Root code.
An uncertain install retains its original signed attempt; it cannot be replaced.

From this Canic checkout, with a current, completed generated `starter` fleet:

```sh
canic artifact build --package canic-root --features maintenance-candidate --output starter/artifacts/root.wasm
canic fleet plan --app starter --identity canic-mainnet
canic fleet status MAINTENANCE_OPERATION --app starter
```

Review the digest before applying. Keep the operator window exclusive and add
`--fleet NAME` if needed:

```sh
canic fleet apply MAINTENANCE_OPERATION --app starter --identity canic-mainnet
canic fleet show --app starter
```

Both Root executables must support the new controller-only `observe_custody`
contract (ID 30). Older owners are refused before installation; their transition
is not automatic. This does not change existing mainnet deployments.

### 🔧 Changed

Adopt Toko Tooling 0.43.2 in both workspaces through its authenticated installer.
The selected 47 snapshot files retain their existing bytes; the package pins,
locks and snapshot provenance advance together.

### 📚 Documentation

Clarify the intended allocation patterns in the design: per-parent singleton,
interchangeable scaling workers, shared-capacity shards and dedicated keyed
instances. Keep subnet placement separate and require maintenance/recovery for
each pattern. These are design commitments, not new CLI options or runtime APIs;
the runnable upgrade examples above retain the current one-child boundary.

## [0.203.1] - 2026-11-14

Root and the profile manager now explicitly mount their management endpoints
through one Canic adapter. Replace 14 handwritten codec functions with shared
generation using Platform's IC entrypoint hooks and Wire's bounded codecs.
Keep method names, controller checks and request/reply limits;
lifecycle and application readiness remain explicit. This completes the first
local migration for [#2](https://github.com/dragginzgame/canic/issues/2), with no
new crate or supplier dependency. Wire Endpoint retirement remains supplier work.

Add `make endpoint-test` for installed request admission, query/observation
contracts, production fault-export exclusion and synchronous/async reply failure
recovery. Reply faults are restricted to synthetic `fault-injection` builds.
No mainnet upgrade has been performed.

Adopt published IC Host FS, Process and Artifacts 0.79.0 in the product and direct
qualification selections, Text 0.76.0 for terminal rendering, and Testkit 0.106.0
for qualification. Testkit now shares the 0.79 Host family, removing the older
duplicate graph. Keep readable UTC deadlines through the existing Time supplier
and render cycle budgets exactly, without rounded-up authority. Replace
timestamp-based host test directories with the supplier's unique-directory
allocator after a parallel fixture encountered lock contention.

Add `canic fleet reconcile OPERATION` for explicitly requesting recovery of a
retained Root creation or child upgrade, including the child of a whole-fleet
operation. Preserve the exact approval, original effect deadlines and spent
slots. Retain the signed recovery request before submission; an unresolved
expired request remains blocked. Status and fleet watching remain read-only.

**Recovery contract change:** reconciliation now requires the exact approval
(method IDs 27/28). Retire the unbound unit requests 22/26 without a fallback.
Older owners reject the new request; existing journals, stage/status contracts
and deployed mainnet canisters remain unchanged. Use the command only with a
compatible, reviewed Root matching the operation's pinned module.

### ▶️ Try it

With an existing configuration, list local declarations without network effects:

```sh
canic app list
canic fleet list
```

For admitted work on a compatible Root, use the original operation digest.
`status` reads local evidence; `reconcile` signs a recovery request and may let
Root continue previously approved work. It does not renew spending authority:

```sh
canic fleet status OPERATION --app starter
canic fleet reconcile OPERATION --app starter --identity canic-mainnet
```

## [0.203.0] - 2026-11-14

### ⚠️ Host API

- Add the `ApplyOutcome::Fleet` outcome for composed approval results. Fleet
  inventory contains deployment-step records sharing the parent approval digest;
  use that digest for status/apply. No compatibility alias or new retry owner.
- App/fleet listing rows include `config_path`; multi-config listings expose
  `config_paths`. Duplicate app identities require explicit config selection.

### ▶️ Try it

From the Canic repository root, build the CLI and inspect the existing profile
fleet. These inspection commands do not deploy or spend cycles:

```sh
make canic
canic fleet canister list --app profile --fleet mainnet
canic fleet show --app profile --fleet mainnet
```

To try the new Root-plus-app deployment, first generate and build a separate
starter. Run this once from the same repository root; `./starter` must not exist.
This setup writes local files and downloads build dependencies, but deploys nothing:

```sh
canic app init starter --fleet mainnet --network ic --source starter
canic app build --app starter
canic app list
```

Stay in the repository root. Prepare the signer and manually fund its cycles-ledger
account first. Planning reads the public principal of `canic-mainnet`; copy
the returned operation digest in place of OPERATION. `--app starter` locates
`starter/canic.json`; its sole fleet is selected automatically:

```sh
canic fleet plan --app starter --identity canic-mainnet
canic fleet status OPERATION --app starter
canic fleet apply OPERATION --app starter --identity canic-mainnet
```

Plan/status only read or retain local evidence. Apply spends mainnet cycles, creates and installs Root,
then admits the application; Root runs autonomously at the planned dispatch time.
After Root reports Ready, repeat the **same** apply to retain completion:

```sh
canic fleet show --app starter
canic fleet apply OPERATION --app starter --identity canic-mainnet
canic fleet call OPERATION --app starter --name team
```

If the plan expires before any apply, cancel it before planning again:

```sh
canic fleet cancel OPERATION --app starter
canic fleet plan --app starter --identity canic-mainnet
```

Back up `starter/canic.json` and the entire hidden `starter/.canic/` together.
The journal is ignored by Git; deleting a project never removes mainnet canisters.

The existing `profile` and `hello-world` fleets are inspection examples; this new
deployment path cannot adopt them. No command above opens a browser.

### ✨ One approval for a starter fleet

- `make canic` builds and installs the CLI on PATH, verifies command resolution,
  and leaves all demo instructions using plain `canic ...`.
- Apply defaults to the qualified signer prepared in the checkout used to build
  Canic. `--icp-bin` is an optional override, absent from normal demo commands.
- `app build` builds the app and supplied Root from declared Cargo recipes,
  preparing a missing lockfile once and preserving existing locks. Source
  generation defaults to the checkout used to build Canic.
- Listings, config edits and observations share current/direct-child discovery;
  app identity comes from the config, and duplicate identities are refused.
  Explicit `--config` wins; journals/artifacts stay beside the selected config.
- Plan/apply output shows readable networks, budgets, UTC deadlines and exact
  follow-up commands. `--identity` derives the controller without copying a principal.
- `fleet cancel` releases only never-prepared whole-fleet approvals. Revoked
  operations remain retained and cannot apply; uncertain attempts remain blocked.
- Plan and apply the generated Root-plus-app fleet under one immutable approval.
  Root's future principal is a typed creation output; both Wasm inputs, budgets,
  network and deadlines are retained before effects. Replanning returns the same
  authority, and completed replay verifies both installations without signing again.
- Status, call and fleet views understand the composed operation. Existing
  single-canister approvals keep their formats and recovery path. This first
  fleet path requires a new, unbound two-canister fleet; existing-fleet adoption,
  fleet upgrades and larger trees remain upcoming.

### ⬆️ Dependencies

- Adopt Toko Tooling 0.43.1 through its authenticated snapshot installer and
  Testkit 0.105.0 for simulator qualification. Canic already uses fresh instances,
  so removal of Testkit's baseline pools needs no consumer API migration.
  Testkit now shares IC Host 0.78 with the qualification artifact reader,
  removing the older Host copies and their `command-fds`/`nix` dependencies.

- Qualify the selected Platform 0.33.3, Runtime 0.22.4 and Types 0.20.4 graph
  alongside these Host and Testkit versions.

## [0.202.0] - 2026-11-14

### ⬆️ Dependencies

- Adopt IC Host FS, Process and Artifacts 0.78.0 in the product and direct
  qualification dependencies. Text remains at its current independent 0.75.0
  release. Testkit retains its own private Host 0.77 dependencies. No replacement
  wrappers, command aliases or journal-format changes accompany this adoption.

### ⚠️ Breaking changes

- Host consumers must replace `Phase` imports with `ManagementPhase` and
  `ManagedCreation` with `ManagedOwner`; no compatibility aliases remain.
  Fleet projections add optional upgrade approval/report fields.
- Upgrading Root or the profile manager migrates its stable payload to v2 within
  the existing allocations. Older executables cannot read this payload; downgrade
  is unsupported. Existing creation authority, keys and Wire messages are retained.

### 👀 Live upgrade progress

- `fleet show`, saved reports and localhost watch now show the planned Root
  upgrade alongside creation. Reports must match the full retained approval and
  exact network. Failed queries remain unknown; watching never signs, reconciles
  or publishes a completion receipt.

From a project with a retained child-upgrade plan, select its fleet. Both commands
only observe; watch serves localhost until Ctrl-C without opening a browser:

```sh
canic fleet show --fleet mainnet
canic fleet show --fleet mainnet --watch
```

Use `canic fleet status UPGRADE_OPERATION --fleet mainnet` for retained evidence;
a live `Ready` report still requires the original apply to retain completion.

### 🔎 Retained Root diagnostics

- `fleet status` now shows Root dispatch counts, retained request evidence,
  creation identity/refund, upgrade versions and failure reasons. Its next-step
  guidance distinguishes pending work, receipt completion and owner recovery
  review, without treating expiry or added cycles as renewed authority.

For an existing operation, retain the same config and journal selection. These
commands only read local evidence; JSON remains unchanged:

```sh
canic fleet status OPERATION
canic fleet status OPERATION --json
```

Root maintenance/reconciliation still has no qualified CLI command. The status
view explains this limit and never retries effects or opens a browser.

### ✨ Root-owned child upgrade

- Admit one in-place child upgrade through the existing fleet plan/apply/status
  workflow. Bind the completed creation receipt, previous and replacement code,
  Root authority and deadline. Preserve creation evidence and issue installation
  at most once, including recovery after a lost callback.
- Qualify persistent profile ownership/data, rejected application upgrades and
  infrastructure maintenance. Migrate existing Root/manager payloads within their
  original checked allocations; no directory rewrite or second storage owner.

From a generated project with a completed child receipt and an upgrade-capable
Root, edit the greeting and rebuild. Replace the uppercase placeholders with the
receipt and reviewed Root values. Planning is local; apply changes the selected
network. Repeat apply after scheduled dispatch to retain completion:

```sh
canic artifact build --package starter-app --output artifacts/app.wasm
canic fleet plan --canister app --target CHILD_PRINCIPAL --previous-module PREVIOUS_SHA256 --root-module ROOT_SHA256 --controller OPERATOR_PRINCIPAL
canic fleet status UPGRADE_OPERATION
canic fleet apply UPGRADE_OPERATION --identity canic-mainnet
```

The experimental Root has one permanent upgrade slot. Application persistence
must be qualified by its own lifecycle. `fleet show` separates the Root's creation
and upgrade reports from the operation's retained status. No
mainnet deployment, publication or browser launch accompanies this increment.

### 🧪 Qualification

- Exercise the generated starter through public CLI Root bootstrap and child
  deployment, resolving Root from its receipt without a configured ID. Deploy the
  same app into two fleets; check separate journals, identities and observations,
  exact replay, lost-reply recovery and refusal of cross-fleet apply before signing.

From a generated project, choose an unused fleet name. These commands add local
declarations only; the second fleet shares app definitions and deploys nothing:

```sh
canic fleet init second --network ic
canic fleet list
canic fleet canister list --fleet second
```

Use explicit `--fleet NAME` for subsequent deployment commands. See the
[two-fleet walkthrough](docs/demos/root.md#reuse-the-app-in-another-fleet).

### 🧹 Cleanup

- Render app trees directly from app definitions, without selecting a fleet.
  Keep generic route labels free of Profile-specific behaviour claims.
- Separate Clap declarations from command execution and rename the Host phase
  export to `ManagementPhase`, with no compatibility alias. Update stale starter
  documentation; command syntax and retained deployment formats are unchanged.

With this checkout's config, these commands inspect local declarations only;
they make no network calls and open no browser:

```sh
canic app canister list --app profile
canic fleet canister list --app profile --fleet mainnet
```

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
canic fleet show
canic fleet show --watch
```

Root reports are uncertified observations. A `Ready` report does not create a
completion receipt; use the original `fleet apply` operation to retain one.

From this checkout, generate a new project in an unused directory. This writes
source and configuration only; the `mainnet` name selects a declaration, not a
deployment. The generated README has the build steps:

```sh
canic app init starter --fleet mainnet --network ic --source ./starter --canic-source .
canic app canister list
canic fleet canister list
```

After building the generated artifacts and deploying its Root, copy the reviewed
Root hash from `artifact inspect` and use your operator principal. Planning is
local; apply admits one 1T-cycle child creation to Root and changes mainnet when
the selected fleet is mainnet. Replace the uppercase placeholders with the
returned values; see [the complete journey](docs/demos/root.md):

```sh
canic artifact inspect starter/artifacts/root.wasm
canic fleet plan --canister app --controller OPERATOR_PRINCIPAL --root-module ROOT_SHA256
canic fleet status CHILD_OPERATION
canic fleet apply CHILD_OPERATION --identity canic-mainnet
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
