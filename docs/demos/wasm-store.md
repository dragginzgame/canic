# Versioned Wasm Store

The standalone Store is the first implementation of the
[per-fleet artifact contract](../design/canic-system-design.md#per-fleet-versioned-wasm-store).
It keeps immutable `(name, version)` releases, content hashes and ordered chunks
outside Root. Uploads resume from missing positions; publication checks every
chunk and the complete raw payload. A second label for identical content reuses
bytes. Preparing a new version never removes earlier releases.

Root now supports explicit Store-backed creation and serial upgrades. It retains
immutable references, pin evidence and complete upgrade history in its original
artifact allocation, with no application Wasm bytes. This is not yet wired into
fleet plan/apply: selected-release policy, generated Store topology and existing-
fleet migration remain unimplemented. Existing inline deployments keep their
current limits and contracts.
There is no delete/reset or pin-release endpoint; incomplete uploads keep their
reservation. No deployment approval or application readiness follows from Store
publication. Host and Store check bounded raw Wasm framing and byte integrity. Full
executable validation remains with the IC installation boundary.

From this checkout, after `make canic` and Wasm toolchain preparation:

```sh
canic artifact build --package canic-wasm-store --output target/wasm-store.wasm
canic artifact inspect target/wasm-store.wasm
```

These commands build and inspect locally. They do not install the Store or open
a browser. `make store-test` runs native and installed PocketIC qualification
after `make qualification-setup`; it uses synthetic cycles and requires local
loopback access. `make store-clippy` and `make store-doc` check Store code;
`make root-clippy` and `make root-doc` cover the management consumer.

All Store endpoints require the current IC controller. `prepare`, `put` and
`publish` are idempotent for exact retained commitments. `status`/`observe` expose
query/replicated progress; `read`/`read_chunk` expose query/replicated published
bytes. `list` returns at most four metadata entries after an exclusive release
cursor. These transport distinctions do not grant fresh installation authority.
`configure` changes admission quotas without reinstalling; `info` reports their
usage. Root controls the Store during autonomous work. Root's controller-only
`store_publication` endpoint now accepts four fixed commands from
`canic-management-protocol::publication`: prepare an immutable manifest, supply
one chunk, publish, or observe retained progress. Each request binds a canonical
Store principal and a dispatch deadline within five minutes. Root performs one
call with a five-second bounded wait and no cycle attachment, storage write,
background retry, deployment or cache preparation. Store independently checks
that Root controls it. There is no arbitrary method/management forwarding.

This route remains usable after the external operator hands Store control to
Root. Publication tests use actual Root calls, resume an incomplete upload after
Root maintenance, retain earlier releases, and prove that Root stable bytes stay
unchanged. Exact replays return retained progress. Malformed frames, unauthorized
callers, invalid targets and expired dispatch preserve state. Fault point 12 in
qualification builds makes Root's reply encoder trap after each remote mutation:
Store commits survive, replicated observation reconstructs progress, and exact
replay does not add a reservation or another chunk. Local callback rollback is
not remote rollback. Call/decode/binding failures return uncertainty; a lost reply
is never evidence of no effect. The calling Host must retain exact commands and
source/code authority before using this route in fleet operations.

The fleet Host now uses this route for explicit publication into an existing
Store. Prerequisites: the selected fleet has bound `root` and `store` slots, with
roles `root` and `wasm-store`; `store` and the app-defined artifact slot
`app` share that Root parent. The artifact slot has a Wasm path. Root must have
the signing operator as its sole controller; Store must have Root as its sole
controller. Both reviewed module hashes must match certified metadata. Existing
fleets need a separately qualified Store installation/handover; these commands
do not bootstrap it. Whole-fleet and named application-child planning refuse a
Store declaration until its bootstrap/readiness contract is qualified.

With those prerequisites and the repository signer prepared, substitute the
reviewed hashes and returned operation digest:

```sh
canic fleet plan --app starter --fleet mainnet --canister app --publish-version 1.0.0 --store store --root-module ROOT_SHA256 --store-module STORE_SHA256 --identity canic-mainnet
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet status OPERATION --app starter --fleet mainnet
canic fleet reconcile OPERATION --app starter --fleet mainnet --identity canic-mainnet
```

Planning retains exact bytes, release label, network, operator and Root/Store code
identities. Apply prepares the manifest, transfers only missing chunks and
publishes; it creates no canister and grants no installation authority. Repeated
planning of the same release reuses the original approval or refuses changed
bytes/authority. Publication uses v3 records; v1/v2 deployment digests stay fixed.
A publication receipt refers to its exact retained signed observation and never
becomes a deployment receipt or reserves an app slot. Inspect it with `fleet status`;
canister inventory excludes artifact-only operations.

Each mutation retains its dispatch deadline and original signed attempt under
the existing journal lock. Reconcile only observes the approved release; fresh
read authority cannot renew upload or publish authority. An expired uncertain
mutation can be bypassed only when exact Store evidence establishes that step.
An unfinished step still needs its original live authority. Observation history
is bounded to 32 attempts; exhaustion blocks without deleting evidence.

Publication accepts up to 8 MiB, within Store's bounded chunk contract. The
existing artifact builder/inspector and direct-install path still have a 1.5 MB
limit; larger inputs currently need an external build. No measured deployment
speedup is claimed. Generated Store topology, deployment release selection,
Store maintenance through Root and existing-fleet migration remain pending.

`make publication-test` qualifies the real CLI and signer against installed Root
and Store, using synthetic cycles. It interrupts signing before the first chunk,
observes without mutation, resumes the original upload, reuses bytes for a second
immutable version and rejects mismatched code and missing receipt evidence.

`pin` retains an exact published manifest for the authenticated caller's operation.
The identity is caller, operation digest and artifact hash; an operation can pin
several artifacts, but cannot change an existing pin's release label. `observe_pin`
returns replicated retention evidence. At most 32 pins per artifact are retained;
exhaustion refuses new obligations without evicting old ones. Pins do not expire
or authorize installation. Metadata schema `CWA2` reads the original `CWA1`
records with no pins, using the same four stable allocations.

`prepare_cache` requires an exact retained pin and prepares its entire bounded
artifact. It observes the cache once, uploads only missing chunks from stable
storage, and verifies every returned hash. Its dispatch deadline is at most five
minutes from admission; caller control and deadline are checked before each
upload. There are no background retries or attached cycles. A cache hit is an
observation at that moment, not a guarantee against subsequent cache clearing.
Repeating a request observes current cache contents before repair. This does not
grant Root permission to replay an uncertain installation.

Root's explicit methods are `stage_stored_child`, `reconcile_stored_child`,
`stage_stored_upgrade`, `reconcile_stored_upgrade` and
`observe_stored_children`. Their passive contracts live in
`canic-management-protocol::stored`. Existing effect approvals retain their old
encoding; each new request binds that approval, child slot and exact Store
reference into a domain-separated operation digest. Store pins are acquired before
child creation or upgrade preparation. The pin callback commits before subsequent
management work. Root retains one cache request deadline and one dispatch; it
installs from Store using the approved ordered chunk list and full module hash.

A lost pin reply is reconstructed by replicated pin observation. Lost cache
completion is reconstructed from Store's actual management cache. Root maintenance
preserves both the existing journals and all Store references. Old complete-custody
and reconciliation methods refuse Store-backed operations rather than omit the
supplier binding. Inline and Store-backed children can coexist; an existing
child cannot switch artifact backends through an upgrade request.

The installed fixtures use actual Root calls for pin acquisition, autonomous
creation and installation, serial upgrades and callback-loss recovery. Executables
larger than the old inline limit are installed with Root's own chunk cache empty.
Other cases cover unpublished releases, expired authority, mixed inline/Store
custody and historical replay. The separate Store fixture clears its cache and
rebuilds it after Store maintenance.

The first Root path deliberately remains blocked if an uncertain cache preparation
has missing chunks at reconciliation. It does not renew the original deadline or
reissue preparation automatically. Cache loss after installation dispatch never
authorizes install replay. An operator-approved repair flow, Store maintenance
through Root, existing-fleet conversion and fleet CLI integration are remaining
gates. The management-cache listing has a bounded 64 KiB reply decoder; an oversized
listing blocks preparation/recovery rather than allocating without a bound.

The initial bounded contract accepts raw artifacts up to 8 MiB in 256 KiB chunks.
The default quota reserves 128 MiB of distinct artifact payloads and 1,024 release
labels, including incomplete uploads. Controller changes may raise those quotas
to 256 MiB and 4,096 labels. Chunk sharing does not reduce the conservative
artifact reservation. Physical backing has a separate 1 GiB limit, including
collection overhead; those admission quotas are not a promise of usable physical
capacity. Exhausted backing traps and rolls back the message. Increasing the
artifact or backing ceiling requires another qualified executable, not a silent
limit bypass. Stable records and independent collections grow incrementally;
Root has no copy of this catalog or its payloads.
