# Versioned Wasm Store

The standalone Store is the first implementation of the
[per-fleet artifact contract](../design/canic-system-design.md#per-fleet-versioned-wasm-store).
It keeps immutable `(name, version)` releases, content hashes and ordered chunks
outside Root. Uploads resume from missing positions; publication checks every
chunk and the complete raw payload. A second label for identical content reuses
bytes. Preparing a new version never removes earlier releases.

Root now supports explicit Store-backed creation and serial upgrades. It retains
immutable references, pin evidence and complete upgrade history in its original
artifact allocation, with no application Wasm bytes. Named `fleet plan --release`
now selects a completed publication for child creation and its first upgrade.
Whole-fleet bootstrap still uses inline application artifacts; release selection
for that envelope and existing-fleet migration remain unimplemented. Existing
inline deployments keep their current limits and contracts.
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
`configure` changes admission quotas without reinstalling; `info`/`observe_info`
report their usage through query/replicated reads. Root controls the Store during
autonomous work. Root's controller-only
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

Root also explicitly mounts `observe_store`: one replicated `observe_info` call
under the same controller, target and deadline checks. It returns Store's bounded
initialized-state report, without a release label, application slot or storage
write. `make store-readiness-test` proves refusal before controller handover,
successful actual Root calls afterward, and unchanged state across repeated reads,
reply fault 12 and Root maintenance. Invalid/expired requests and unavailable
targets cannot produce a successful report. This is one readiness input for
bootstrap; it does not certify module identity, sole control or sufficient quota
for future work. Host binds certified code/controller evidence separately in
named Store bootstrap.

To create a Store, use the existing named `fleet plan` workflow. The selected
fleet must have a bound top-level `root` and an unbound `wasm-store` child with a
Wasm input. The reviewed Root executable must mount `observe_store` and
`store_publication`. Planning retains exact Store bytes, Root identity/code,
operator, network and creation budget; it creates nothing.

From the app's config directory, after installing this CLI with `make canic`,
building Store, and binding the existing Root ID, declare the Store if absent.
The Wasm path is relative to the config. Substitute reviewed hashes and returned
operation/Store IDs below. Apply creates a canister using synthetic cycles on a
local fleet or debits the operator's cycles-ledger account on mainnet (default
ceiling 1T, including the ledger fee):

```sh
canic app canister add store --app starter --role wasm-store --parent root --wasm target/wasm-store.wasm
canic fleet plan --app starter --fleet mainnet --canister store --root-module ROOT_SHA256 --identity canic-mainnet
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet status OPERATION --app starter --fleet mainnet
canic fleet canister bind store --app starter --fleet mainnet --id STORE_ID
```

Creation retains Root's certified subnet first and explicitly selects that subnet
through the cycles ledger; local creation routes through Root. A changed Root
subnet or a created Store elsewhere blocks progress. Host uses its original
direct creation/install attempts while the operator controls Store, then retains
an exact `update_settings` attempt handing sole control to Root. After handover
starts, recovery validates the original creation/install evidence without trying
to reinstall. Completion requires certified Store code/sole Root control and a
retained signed replicated readiness reply. Root code/control are checked too;
keep the operator's maintenance window exclusive during this process.

If the handover reply is lost, `canic fleet reconcile OPERATION` with the same
app/fleet and identity can establish completion from certified sole control and
fresh read authority. It never creates, installs or hands over. Apply may resume
an original live handover attempt, but cannot replace an expired uncertain one.
Unknown creation/install results remain blocked under the existing direct-effect
contract. Readiness has 32 retained attempts; unresolved expired reads and
exhaustion remain blocked. Keep the journal and signer. A receipt is historical
completion evidence, not perpetual readiness or a Store maintenance contract.

For a fresh fleet, declare `store` under `root` before whole-fleet planning. The
fleet may have one or two application children in addition to Store; every slot
must be unbound. Root, Store and application Wasm paths must already be built.
From that app's config directory:

```sh
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
canic fleet apply OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet status OPERATION --app starter --fleet mainnet
canic fleet show --app starter --fleet mainnet
```

The default total debit is 4T for Root + Store + one application, or 5T for two
applications. `--cycles` sets the total: Store receives a separate 1T allowance,
and Root receives the remainder, including its child funding. The preview shows
each debit and the total. One immutable approval binds Store to Root's retained
creation output. Apply completes Root, then Store installation/handover/readiness,
then admits the application children under the original dispatch deadline.
Store does not consume either application slot. Retained fleet digests without
Store remain unchanged.

Reconcile can recover Store before application admission, but reports blocked
until apply admits every approved child. That is not a completed fleet. Replan
keeps the original operation and deadlines; cancellation is unavailable after
any effect preparation. Completed Store evidence is required for convergence.
Adding/removing Store or changing its artifact on a retained fleet is refused:
existing-fleet adoption and Store maintenance need their own qualified contracts.
The generated starter still begins with one app and Root; Store is explicitly
declared rather than imposed on every application.

The fleet Host uses this route for explicit publication into an existing
Store. Prerequisites: the selected fleet has bound `root` and `store` slots, with
roles `root` and `wasm-store`; `store` and the app-defined artifact slot
`app` share that Root parent. The artifact slot has a Wasm path. Root must have
the signing operator as its sole controller; Store must have Root as its sole
controller. Both reviewed module hashes must match certified metadata. Complete
the named bootstrap above and bind its returned Store ID before publication.
After whole-fleet bootstrap, bind both returned Root and Store IDs for this
publication workflow; bindings project identity and do not grant new authority.

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

Publication, artifact building and inspection accept up to 8 MiB, within Store’s
bounded chunk contract. Direct-install plans retain their 1.5 MB transport bound,
and inline child plans retain their smaller custody limits. No measured deployment
speedup is claimed. Whole-fleet release selection, Store maintenance through Root
and existing-fleet migration remain pending.

`make publication-test` qualifies the real CLI and signer against installed Root
and Store, using synthetic cycles. It creates Store through the CLI, refuses
changed Root control before creation, interrupts readiness after handover, removes
the local handover reply and reconciles without repeating creation/install or
changing Root's stable bytes. Missing retained readiness evidence invalidates a
receipt. It then interrupts signing before the first chunk,
observes without mutation, resumes the original upload, reuses bytes for a second
immutable version and rejects mismatched code and missing receipt evidence. It
then creates a child from a published release above the inline size limit, recovers
a lost admission reply, reconstructs a cleared cache during upgrade, calls the
upgraded application and proves replay creates no extra canisters.

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

## Deploy a published release

Use the completed publication operation from this fleet's retained journal.
The named application must be the sole application child under its bound Root;
Store is infrastructure and does not count as an application child. Keep the
app's configured Wasm input identical to the selected publication. Planning checks
its completed signed publication evidence, app/fleet/network/operator and both
owner identities, then freezes the exact manifest and Store code in the child
approval. It never resolves a moving latest version. There is no payload upload
from Host to Root during admission.

From the app's config directory, with Root and Store bound and the release already
published, replace the operation/hash placeholders below. Apply authorizes Root's
fixed 1T creation grant from its existing cycles balance. The default dispatch
remains ten minutes after planning; an explicit future `--execute-at UNIX_SECONDS`
selects a different schedule within the bounded plan window. Keep Root funded.

```sh
canic fleet plan --app starter --fleet mainnet --canister app --release PUBLICATION_OPERATION --root-module ROOT_SHA256 --identity canic-mainnet
canic fleet apply DEPLOYMENT_OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet status DEPLOYMENT_OPERATION --app starter --fleet mainnet
```

Apply again after dispatch to retain completion. If admission's reply was lost,
apply reads Root's complete Store custody and checks the selected release and
operation pin. It retains the original signed admission. Explicit `fleet reconcile`
uses the same source-bound recovery contract and cannot renew spent or expired
effect authority. Certified Store code/control and same-Root-subnet checks precede unfinished work;
completed deployment replay verifies the application receipt without requiring
Store to be available. Publication evidence is required when planning, while the
resulting deployment journal independently retains its approved source and bytes.

For the first upgrade, publish a different executable under a new immutable
version, update the app's Wasm input, and use the existing target/predecessor
arguments with the new publication operation:

```sh
canic fleet plan --app starter --fleet mainnet --canister app --release NEXT_PUBLICATION_OPERATION --target APP_ID --previous-module PREVIOUS_SHA256 --root-module ROOT_SHA256 --identity canic-mainnet
canic fleet apply UPGRADE_OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet call UPGRADE_OPERATION --app starter --fleet mainnet --name team
```

The final command is the greeting demo's read-only query. Root obtains an exact
Store pin, prepares/reconstructs the derived management cache and performs the
upgrade through its existing effect owner. An inline child cannot switch to Store
custody through this flag. The named planner retains its existing first-upgrade
limit; serial upgrade convergence, whole-fleet release selection and host-managed
Root maintenance with complete Store custody need separate qualification. Runtime
maintenance evidence already has its installed fixture, but that does not qualify
the Host maintenance workflow. No live mainnet run or latency improvement is claimed.
