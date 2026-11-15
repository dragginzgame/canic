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
Fresh whole fleets with a declared Store publish immutable child releases before
admission and support bounded serial upgrades. Existing-fleet migration remains
unimplemented; retained inline deployments keep their current limits and contracts.
There is no delete/reset or pin-release endpoint; incomplete uploads keep their
reservation. No deployment approval or application readiness follows from Store
publication. Host and Store check bounded raw Wasm framing and byte integrity. Full
executable validation remains with the IC installation boundary.

## Whole-fleet artifacts

Declare a new Root, one `wasm-store` child and one or two application children,
with built Wasm inputs and no existing ID bindings. The usual whole-fleet plan
selects Store delivery automatically. Release names are application canister names;
versions are full content hashes, frozen with the ordered chunk commitments.
There is no mutable latest selection or extra configuration switch.

From the app's config directory, after `make canic` and signer preparation:

```sh
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
canic fleet status FLEET_OPERATION --app starter --fleet mainnet
canic fleet apply FLEET_OPERATION --app starter --fleet mainnet --identity canic-mainnet
```

Apply spends the reviewed creation budget: 4T total for Root, Store and one app,
or 5T with two apps. It creates Root/Store, hands Store to Root, publishes every
selected child release, then admits children using exact references. Application
artifacts can be up to 8 MiB each, subject to Store quota. Identical content under
different application names reuses bytes. Root's scheduled work can continue
after the operator leaves. Default dispatch remains ten minutes after planning;
resume the same operation promptly after an interruption rather than making a new
approval. Reconcile observes publication but cannot publish or admit a new child.
`--cycles` sets the total fleet debit: Store receives a separate 1T allowance,
and Root receives the remainder, including child funding. Store consumes no
application slot. Reconcile reports blocked until apply admits all children;
recovering Store alone is not fleet completion. Cancellation is unavailable after
effect preparation. Store addition/removal or replacement on a retained fleet
requires a separately qualified adoption/maintenance contract. The generated
starter still begins with Root and one app; declare Store explicitly.

After creation completes, change and rebuild one child. For an unpublished
artifact, planning returns publication authority first. Apply it, then replan
and review installation:

```sh
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
canic fleet apply PUBLICATION_OPERATION --app starter --fleet mainnet --identity canic-mainnet
canic fleet plan --app starter --fleet mainnet --identity canic-mainnet
canic fleet apply UPGRADE_OPERATION --app starter --fleet mainnet --identity canic-mainnet
```

Repeat that upgrade apply after dispatch to retain completion. Each child retains
up to eight upgrades. Pending installation must finish before a successor; exact
replanning retains its original schedule and authority. After child work completes,
a Root-only artifact change uses the same maintenance plan/apply flow while
preserving full Store custody. See [Root maintenance](root.md#root-maintenance).
An existing release with retained publication completion can be selected directly
for the next upgrade, including after Root maintenance. This reuses historical
artifact evidence; it cannot renew an unfinished publication's old code authority.
Store upgrades and inline-to-Store migration remain unqualified Host flows.

## Standalone artifact inspection

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

For preservation inspection, Store's replicated `observe_custody` takes one exact
release and returns its progress plus every retained pin on the shared artifact.
Pins preserve their original owner, operation and release label, including former
controllers and different labels sharing those bytes. Root mounts
`observe_store_custody` under the same controller/target/deadline checks, so a
Root-owned Store does not need to give the external operator direct access.
The result is bounded by one manifest and 32 compact pin records, reads metadata
only and creates no Root copy. It is not an atomic catalog snapshot, payload
integrity scan, maintenance approval or proof of installed code. Host-driven
Store upgrades remain unqualified. Run the focused installed observation witness
with `bash scripts/dev/test-store.sh custody` after simulator preparation.

Store's `admission` endpoint accepts only close, observe and activate. It captures
the controller and retains the exact operation and increasing sequence in its
existing settings allocation. Root forwards those commands through
`store_admission` with the same controller/deadline checks as publication. While
closed, prepare/put/publish, pin acquisition, quota changes and fresh cache work
are refused; reads and exact attempt recovery remain available. Store upgrades
leave the gate closed until explicit matching activation. Old cache callbacks
are fenced even if admission has reopened. The gate does not certify that earlier
remote calls have drained or authorize installation. These are runtime primitives,
not another CLI upgrade path. `bash scripts/dev/test-store.sh admission` qualifies
them locally; its test controller still performs Store installation explicitly.
The CWS2 settings codec loads recognized prior CWS1 settings. Unknown or malformed
records fail closed, and downgrading after writing CWS2 is unsupported.

Root's `stage_store_maintenance`, `advance_store_maintenance` and
`observe_store_maintenance` mounts now qualify the next boundary under sole Root
control. Root retains one immutable approval and stop-request commitment outside
Store. It fences child/publication mutations and raw gate changes, closes Store,
then issues stop once. Lost callbacks recover through observations; expiry never
renews dispatch authority. A stopped observation establishes this drain boundary.
The CUJ6 journal preserves the attempt across Root replacement and loads prior
recognized journals without inventing an attempt. Existing complete child-custody
observations refuse any retained Store maintenance, including completion,
preventing the current Host Root-maintenance path from omitting it.
Run `bash scripts/dev/test-store.sh quiescence` for the installed proof.

Before stop, Root now walks `observe_preservation` one release per continuation.
Each page requires the exact closed attempt and captured owner. Root retains the
quota/counters, ordered metadata digest and last release, rather than a catalog
copy. It requires unchanged counters, strictly increasing keys, the expected
release count and an explicit end page. Aliases, incomplete-upload progress and
all shared pins are included. Root replacement resumes the retained cursor;
concurrent replies cannot count it twice. This does not scan payload bytes or
commit management-cache contents. CUJ5 attempts load without scan evidence;
already-stopped attempts never gain a fabricated preservation commitment.
Older Stores lacking this endpoint cannot complete the new pre-stop scan.

The explicit `install_store_maintenance` mount accepts the approved raw replacement
(at most 1 MiB for this first infrastructure path). Root uploads it once to Store's
management chunk cache, records exact upload/install commitments, and installs it
through Platform's existing chunked management contract. Host keeps the durable
bytes; Root never stores them. Lost upload replies recover by chunk observation;
installation recovery checks the approved replacement code and sole Root control.
Issued effects are never repeated, including after Root replacement.

The continuation starts the replacement with admission closed, repeats the same
catalog scan, and requires complete matching preservation before Verified.
`advance_store_maintenance` never activates implicitly. Only explicit
`activate_store_maintenance` issues the matching Store activation; lost replies
recover by observation. Ready is retained historical completion, not a fresh
readiness guarantee. It releases child/publication fences while retaining the
operation. Every new effect retains the original deadline; observation after
expiry cannot authorize a new start or activation. Rejected or incompatible
replacements remain blocked without installation replay.

`bash scripts/dev/test-store.sh maintenance` qualifies this runtime path using
sole Root control and compares all Store stable bytes before/after replacement,
including payloads, pins and an unfinished upload. Runtime verification itself
compares metadata; it is not a full payload integrity scan. One attempt remains
retained, with no cancellation, reset or successor history. Host/CLI upgrade execution
and live use remain unqualified; **do not use these runtime mounts live**.

`fleet show` reads Root's maintenance metadata without a signer or continuation.
It selects only Roots and known Store identities from retained Store bootstrap
plans. A Root built with `store_maintenance_status` reports closure, stopping,
installation, verification and activation separately. Older Roots report unknown
when that method is unavailable. Configuration roles alone do not select queries.
The report is uncertified; completed maintenance is historical evidence, not
current Store admission or readiness. The normal terminal view is compact;
`--details` includes the original deadline and before/after catalog counts.
For an existing selected app/fleet with its retained journal:

```sh
canic fleet show --details
canic fleet show --json
```

The same report appears in the opt-in localhost viewer. No view signs requests,
advances maintenance or changes completion receipts.

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

For new unbound Root/Store/app deployments, use the
[whole-fleet workflow](#whole-fleet-artifacts).

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
speedup is claimed. Whole-fleet release selection and Root maintenance with full
Store custody use the workflows above. Host upgrade execution for Store maintenance and
existing-fleet migration remain pending.

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
limit. Serial upgrade convergence and Root maintenance with full Store custody
are qualified through the whole-fleet workflow above; they do not expand this
named first-upgrade planner. No live mainnet run or latency improvement is claimed.
