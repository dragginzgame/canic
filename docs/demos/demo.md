# Managed counter example

## Accepted application command boundary

The maintainer selected application-owned named commands and direct user ingress
on 2026-11-17. The app owns command syntax and request/reply codecs. Canic selects
the fleet canister, captures the selected signing principal and retains exact
request bytes under its existing plan, journal lock and signed-attempt owner.
Root is not a call proxy; controller or tree membership grants no application
permission. Repeating an unresolved command must not create another increment.
Observation keeps the original request ID and deadline; expired uncertainty stays
blocked. A decode failure after a reply is retained cannot authorize another call.

The narrow implementation uses an optional native command codec in the app's
existing Cargo package, built alongside its Wasm. It encodes named commands and
decodes retained replies; it owns no network transport or signing. No endpoint
catalog, runtime plugin registry, query/update inference or generated schema
interpreter is needed for this consumer. The initial commands are replicated
updates (`read` and `increment`); application Wire budgets remain app-owned.

Legacy review at `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` inspected
`crates/canic-cli/src/{lib.rs,endpoints/mod.rs,endpoints/tests.rs}`. Its endpoint
listing preserves codec budgets and requires a finalized build identity, solving
schema/installed-binding confusion rather than supplying generic application
mutation authority. Retain that separation and strict malformed-output admission;
do not rebuild its release-schema machinery for two app-owned commands. Existing
Canic signed-request regression cases own request ID, sender, bytes and expiry.
History review also inspected `b9ed7f298bdaf8c9cceac142edf7e56b15d33208`:
its endpoint/trap guards distinguish owned Wire bindings from raw exports and
restrict lifecycle exceptions to the actual owning boundary. This increment adds
no raw endpoint or lifecycle exception; the existing counter actor is unchanged.

The selected stateful application is now a small persistent counter, replacing
the profile app as the hands-on consumer. Capture and restore will be explicit
operator maintenance operations; restoration keeps writes fenced until the
operator explicitly accepts and reopens the application. The Backup signer exercise below qualifies this path locally.

The counter stores a bounded 256-byte payload in a named Storage 0.78 cell.
Its installed local witness checks an eight-page backing limit. The app lives in `apps/examples/counter/canic-demo`; it owns its protocol, value,
installation authority and maintenance attempts. No temporary raw memory
implementation or fallback Storage owner is added. The profile packages
now live only in `qualification/`, preserving existing regression cases. Its old
mainnet identities remain in `qualification/fixtures/profile-mainnet.toml`, with
the original journal location; no live canisters or retained records were removed.

Fresh Root now uses the application readiness contract for its children and
accepts only `Ready`. The selected counter permits public increments, with recovery
and reopening controlled explicitly by the operator. Its readiness response
distinguishes `Unavailable`, `Fenced` and `Ready`; checking an application after
restore cannot clear its recovery fence. The counter mounts this replicated
observation over its actual opened state. Compatible upgrades retain a closed
fence until the matching explicit activation. Its nonanonymous installation caller
is the immutable maintenance authority; controller membership is not application
permission. In the installed managed witness that caller is Root. Its separate
Store publishes the counter; the normal fleet CLI creates and upgrades it while
preserving value. A counter installed with closed admission stays fenced and
does not receive a deployment completion receipt. Repeated observations cannot
activate it. The local Backup witness below exercises Root-mediated fencing and reopening.

This is the small Canic-owned companion to Toko development. Both use the same
app declarations, Cargo build recipes and explicit actor composition. The demo
has its own [configuration](../../apps/examples/counter/canic.toml), separate from the
existing hello-world and profile mainnet deployments. `canic-demo` is one
application crate; its native contract does not link the Storage runtime.

```text
Root
├─ Wasm Store
└─ App
```

Root owns management effects; the supplied Store owns versioned artifacts. The
operator is an external controller, not another node in this canister tree.
The `mainnet` fleet is a declaration with no deployed identities or journal yet.

## Try the app now

From the repository root, install the current CLI, then enter the demo directory:

```sh
make canic
cd apps/examples/counter
canic app canister list
canic fleet list
canic app build
canic artifact inspect artifacts/app.wasm
```

These commands require no identity, funding or network deployment. Build uses
the locked workspace graph and produces counter, Root and Store under
`apps/examples/counter/artifacts/`. Use `--canister app` to rebuild only the counter.
The [actor](../../apps/examples/counter/canic-demo/src/actor.rs) explicitly creates or opens its record
and mounts readiness; its `counter` endpoint accepts application-owned Wire
`Read`, `Increment`, `Fence { attempt }` and `Activate { attempt }` requests.
An uncertain increment must not be automatically retried: this simple app has
no business-operation deduplication. Overflow and fenced writes refuse before
mutation; post-write failures trap.

With the simulator already prepared, run the installed rehearsal from the root:

```sh
make counter-test
```

It tests two public callers, value and authority preservation, closed admission,
stale activation, malformed requests, incompatible schemas and rollback after a
candidate writes then traps. It also refuses empty/corrupt state without creating
a replacement record. `counter-v2`, `initially-fenced`, `reject-upgrade` and `incompatible-schema` are
qualification-only build selections, not live deployment features. For checkpoint recovery, Root retains the generation outside the counter. With
the disposable Linux signer and simulator prepared, run the installed journey:

```sh
make backup-signer-test
```

It restores the counter from 2 to 1, starts it fenced, rejects premature
opening, and permits writes only after separate operator acceptance and reopening
through Root. The [qualification guide](../../qualification/README.md) describes
the original-request custody and the limits of this single-counter deployment.

The managed counter journey uses synthetic cycles and the real fleet CLI:

```sh
bash scripts/dev/test-publication.sh counter_managed::
```

The installed witness also runs the real named commands, loses a local increment reply, refuses a replacement command and recovers the original request without incrementing twice.

## Local edit, call and upgrade session

From the repository root:

```sh
make demo
```

This installs the current CLI on PATH, verifies the prepared ICP tool and starts
an explicitly owned local simulator. If setup is missing, run `make install-tools`
and `make qualification-setup` once; ordinary sessions download no simulator.
No terminal or browser opens automatically. Leave this command running and use
another terminal for the `cd` command it prints.

The session creates a separate `counter/local` declaration beneath a new
`target/demo/` directory. It contains Root, separate Wasm Store and the counter,
an explicit local trust binding and a fresh journal. The `counter/mainnet`
configuration remains unchanged. Only a routing canister is prepared
automatically; managed deployment still requires your plan/apply.

From the printed session directory:

```sh
canic app build
canic fleet plan --identity anonymous
canic fleet apply OPERATION --identity anonymous
canic fleet call --canister app --identity anonymous read
canic fleet call --canister app --identity anonymous increment
canic fleet status OPERATION
```

Use the operation from the plan; repeat apply on that same operation while Root
is pending. The local anonymous identity spends synthetic cycles, not the mainnet
wallet. Edit the counter in `apps/examples/counter/canic-demo/src/actor.rs`, then run `canic app build`.
`canic fleet plan --identity anonymous` first selects publication, then upgrade;
apply each returned operation. A compatible upgrade keeps the value and identity.

Application commands use `build.client = "canic-demo-client"`. This executable
implements `encode COMMAND [ARGS...]` returning JSON `{method, arguments}` and
`decode` reading raw reply bytes on stdin and returning app-owned JSON. Canic
bounds process time/output and request bytes; the app codec applies its actual
Wire policy. The executable gets no signer or transport handle. Treat it like
other trusted local application build code, not a sandboxed plugin.

If a call is uncertain, retain its printed operation and use
`canic fleet reconcile OPERATION --identity anonymous`. This observes only the
original signed request; it cannot issue another increment. Recovery reports raw
reply bytes. Expired uncertainty remains blocked. Calls share the existing
128-operation inventory bound; no history eviction or uncertainty reset is added.

Press Enter in the first terminal to stop. Ctrl-C, input EOF and the one-hour
session limit also end the session. Shutdown attempts instance deletion and stops
the owned server; deletion-attempt completion is not independent proof of remote
deletion. Local config, artifacts and journals remain for inspection. A new run
gets a fresh instance and directory; old session receipts cannot authorize work
in it. The interactive session preserves state across a compatible upgrade. The separate
`make backup-signer-test` journey qualifies checkpoint restoration. Simulator
process restart remains outside both witnesses.

## Actor composition

The [actor file](../../apps/examples/greeting/canic-hello/src/actor.rs) selects two adapters:

```rust
canic::mount_greeting_endpoints!(crate::greeting);
canic::mount_metrics_endpoint!(crate::actor::snapshot);
```

The fixed greeting adapter exposes `greet` and the replicated `probe` used by
this demo's readiness check. The application owns its response. This probe is
not a database, Auth or Toko readiness contract. The metrics query reports the
observation time and unavailable timer data: no timer provider exists here.
Mounting neither adapter initializes storage, starts timers or grants authority.
Custom application endpoints can still use Platform entrypoints directly.

## Deployment boundary

The maintainer selected **mainnet when Storage is ready**. The managed counter
creation/upgrade path now passes local qualification; no mainnet deployment has
been performed. Interactive counter calls and recovery operations remain separate
work. Fresh Root uses journal schema 2: old allocator images and schema-1 greeting
Roots must retain their old executable. Do not reset an existing fleet to bypass
that boundary. For an explicitly approved fresh deployment, build all three artifacts
and use the existing [Root plan/apply workflow](root.md#deploy-the-generated-application)
with an explicitly selected controller identity and funded cycles account.

For now, `make journey-test` from the repository root installs the actual greeting
actor in the prepared local simulator, calls it, tests both mounts and upgrades
the same canister. This uses synthetic cycles and the direct Host journey; it
does not qualify or deploy the declared Root/Store fleet. Run
`make qualification-setup` explicitly if simulator assets have not been prepared.

As Toko exposes a new authoring or composition problem, reproduce the smallest
useful case here before expanding the framework. Durable application behavior,
authentication and further canisters need their own explicit ownership and
installed qualification. The counter's local preservation proof does not yet
qualify managed deployment or backup restoration.
