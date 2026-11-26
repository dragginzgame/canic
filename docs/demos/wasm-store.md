# Versioned Wasm Store

Implementation and passive protocol now live in
[ic-wasm-store](https://github.com/dragginzgame/ic-wasm-store).
Canic selects the same locked crates.io protocol in both Cargo workspaces and
the matching runtime as a Host artifact input. `canic artifact build --package ic-wasm-store`
builds that registry archive with its packaged lockfile. Both selections and the
archive's protocol must match; Git/path overrides are refused. No sibling
checkout is needed. Host selects the toolchain in the consumer workspace, then runs
Cargo for the published archive outside the consumer’s local Cargo configuration.

Every app automatically includes Root and its Wasm Store.

Application children refer to `parent = "root"`. During fresh fleet bootstrap,
Host uses the operator's signer to install Root from its local Wasm and the
separate `store` slot from `artifacts/wasm_store.wasm`. Host then hands
control of Store to Root.
Each fleet binds its own principals. Builds reject explicit Root, Store and Coordinator definitions. When removing them, ensure the application build workspace's Cargo.lock
selects both supplier packages and the matching Store protocol.

Both Canic workspaces select the published Store 0.10.0 runtime and protocol. The protocol uses IC Wire 0.30.2. Install a fresh Store and rebuild artifact manifests for 1 MiB chunks. The supplier refuses upgrades from the 256 KiB format. The following evidence applies to Store 0.6.0.
A local Root-managed upgrade from 0.5.2 preserved the populated Store’s stable
memory. [Qualification record](../qualification/history.md#wasm-store-052-to-060--2026-11-21)

Store 0.6.0 used Storage 0.78 for its four named allocations. A fleet with the 0.3.3
Store on Storage 0.74.1 must retain its qualified executable. Installing 0.4.0 over that directory traps during upgrade.
The endpoint identities and Store record encodings are preserved, but the
underlying directory format changed.

The supplier also supports exact removal/restoration and tombstones. Root now
forwards exact deletion/cancellation through `remove_store_artifact`; Host retains their
exact signed attempts under the existing journal lock. New Root attempts use Store 0.8’s
`observe_preservation` (method 117): explicit active/removed entries ordered by name, active
before removed, then version. The retained cursor includes that flag, and a
separate digest domain commits to the entry kind. Pre-cut Roots retain their original scan contracts, digests and refusals.
Deploy the 0.8 Store with a fresh Root. Existing pairs finish maintenance on their original executables. Fresh Roots use CUJG for the entire fleet. Old executables refuse the new journal format.

The historical 0.2.3 rollout upgraded Store under the old Root before
upgrading Root. New attempts require the versioned supplier endpoint; no probing
or fallback selects a weaker scan. `bash scripts/dev/test-store.sh preservation`
qualifies removed/cancelled labels through Root and Store replacement, alongside
the historical refusal/migration case. These tests stage removal through the
fixture controller; the separate CLI witness below qualifies Host settlement.

`bash scripts/dev/test-store.sh removal` exercises actual Root deletion/cancellation,
pinned and wrong-manifest refusal, authorization, malformed requests, maintenance
fencing, shared-byte retention and tombstone quota. Its fixture retains the exact
successful response and expires the original removal deadline before restoration;
replaying that old request then fails. An injected lost Root reply demonstrates
why observing absence cannot replace a receipt. Root does not deduplicate freshly
signed repeats or renew deadlines; Host owns those restrictions.

## Removal and exact restoration

Use one canonical fleet journal and the current Root/Store executables. Removal
currently selects an independent publication operation, not a nested bootstrap
publication. Store pins still refuse removal. Before planning removal, Host requires
all known matching publication mutations (including nested ones) to have exact
successful replies and expired dispatch windows. An absence observation or expired
request alone cannot settle an ambiguous call.

```sh
canic fleet artifact list --app starter --fleet mainnet --details
canic fleet artifact plan-removal PUBLICATION_OPERATION --action delete --app starter --fleet mainnet
canic fleet status REMOVAL_OPERATION --app starter --fleet mainnet
```

Planning sends nothing. Review the exact Root/Store/code/content bindings, then
apply with `canic fleet apply REMOVAL_OPERATION --app starter --fleet mainnet --identity canic-mainnet`.
Use `--action cancel` instead of `delete` for an incomplete upload. Repeated
planning retains the same approval; apply/reconcile keep the same signed attempt.
Even a retained refusal cannot authorize a fresh mutation. Unresolved outcomes
remain blocked; this path has no automatic repair or replacement authority.

After the successful original reply is retained **and** its dispatch deadline has
expired, retain a new same-content restoration approval:

```sh
canic fleet artifact plan-restoration REMOVAL_OPERATION --app starter --fleet mainnet
canic fleet status RESTORATION_OPERATION --app starter --fleet mainnet
canic fleet apply RESTORATION_OPERATION --app starter --fleet mainnet --identity canic-mainnet
```

Restoration uses the publication owner and links the exact removal in its digest.
It cannot change label, bytes, network, controller or reviewed Root/Store code.
Every known removal of that label must be a settled, expired ancestor. Old
publication receipts remain historical evidence; resuming their approvals cannot
restore content. The normal 128-operation journal bound applies; no new catalog,
generation allocator, Root persistence or automatic retry engine is added.
This cannot coordinate a second journal or privileged calls made outside Canic.

`bash scripts/dev/test-publication.sh cli_removal_restoration` runs the actual
CLI against installed Root/Store: shared bytes survive deletion, successful
restoration needs new authority, fresh replay of the old payload expires, and an
injected lost cancellation reply remains blocked despite observed absence.

The standalone Store is the first implementation of the
[per-fleet artifact contract](../design/canic-system-design.md#per-fleet-versioned-wasm-store).
It keeps immutable `(name, version)` releases, content hashes and ordered chunks
outside Root. Uploads resume from missing positions; publication checks every
chunk and the complete raw payload. A second label for identical content reuses
bytes. Preparing a new version never removes earlier releases.

Root now supports explicit Store-backed creation and serial upgrades. It retains
immutable references, pin evidence and complete upgrade history in its original
artifact allocation, with no application Wasm bytes. Named `fleet plan --release`
now selects a completed publication for child creation and upgrades. Serial named
upgrades require the retained whole-fleet creation approval and completed predecessors.
Fresh whole fleets with a declared Store publish immutable child releases before
admission and support bounded serial upgrades. Existing-fleet migration remains
unimplemented; retained inline deployments keep their current limits and contracts.
The explicit removal/restoration flow above covers independent publications.
Canic exposes no reset or pin-release flow; incomplete uploads retain their
reservation until an admitted cancellation succeeds. No deployment approval or application readiness follows from Store
publication. Host and Store check bounded raw Wasm framing and byte integrity. Full
executable validation remains with the IC installation boundary.

## Whole-fleet artifacts

Inspect locally retained versions for an existing configured fleet:

```sh
canic fleet artifact list --app starter
canic fleet artifact list --app starter --details
canic fleet artifact list --app starter --json
```

The listing groups releases by Store and uses the same plan and signed-receipt
checks as publication status. It includes bootstrap publications once their exact
Root/Store identities and publication plans are retained, plus independent
publication plans. **Published** means a validated completion observation is in
this journal; **unconfirmed** means it is absent, not proof that nothing happened
remotely. `--details` exposes exact Wasm hashes, original Root/Store code bindings
and the outer **Operation** usable with `fleet status`. A different nested
publication approval identifies the narrower bootstrap step; use the outer
operation for status/apply/reconcile. JSON retains both identities and the same
structured evidence independently of `--details`.
Versions sort lexically without selecting a latest release. This is not a live
Store catalog or deployment listing. No signer or network request is used.
Keep `--config`, `--fleet` and any `--state-dir` override when selecting a journal.
Bad records are reported alongside valid rows and exit 1; they are never treated
as empty or complete. Missing journals remain absent.

`canic app init starter --source starter` generates Root, one `wasm-store` child
and one application definition, with zero fleets. Declare the deployment separately
with `canic fleet init mainnet --app starter --network ic`. Custom fresh fleets can declare the same topology with one or two application children,
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
selected child release, then admits children using exact references. The whole-fleet approval covers publication and child installation. Root uses
native IC snapshots on the subnet. Application
artifacts can be up to 8 MiB each, subject to Store quota. Identical content under
different application names reuses bytes. Root's scheduled work can continue
after the operator leaves. New plans dispatch when approved prerequisites are ready,
within ten minutes of planning; bootstrap and publication use that same window.
Resume the same operation promptly after an interruption rather than making a new
approval. Reconcile observes publication but cannot publish or admit a new child.
`--cycles` sets the total fleet debit: Store receives a separate 1T allowance,
and Root receives the remainder, including child funding. Store consumes no
application slot. Reconcile reports blocked until apply admits all children;
recovering Store alone is not fleet completion. Cancellation is unavailable after
effect preparation. Store addition/removal or replacement on a retained fleet
requires a separately qualified adoption/maintenance contract. Generation does
not modify existing projects or migrate retained inline fleets.

A completed one-child fleet can also add its second child through the same
publication-then-creation sequence. The additional creation has its own approval
and draws its attachment from Root's existing balance; the original fleet
approval and debit stay unchanged. See [adding a child](root.md#add-a-child-to-a-completed-fleet).

After creation completes, change and rebuild one child. For an unpublished
artifact, planning returns publication authority first. Apply it, then replan
and review installation. A generated app recipe can rebuild just the selected
child while keeping the existing Root and Store artifacts:

```sh
canic app build --app starter --canister app
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
Bounded repeat Store replacements are qualified below; inline-to-Store migration
remains an unqualified Host flow.

## Standalone artifact inspection

From this checkout, after `make canic` and Wasm toolchain preparation:

```sh
canic artifact build --package ic-wasm-store --output target/wasm_store.wasm
canic artifact inspect target/wasm_store.wasm
```

These commands build and inspect locally. They do not install the Store or open
a browser. `make store-test` runs native and installed PocketIC qualification
after `make qualification-setup`; it uses synthetic cycles and requires local
loopback access. `make store-clippy` checks the installed integration harness;
`make root-clippy` and `make root-doc` cover the management runtime and contracts.
The supplier repository owns Store runtime/protocol checks and standalone installed
qualification. Run installed build suites sequentially: their feature variants
share Cargo output paths.

Store management requires the current IC controller. The controller can query published
chunks with `read` or use `grant_read` to authorize a named installer to fetch chunks
for an exact pin through the `read_granted` update. `prepare`, `put` and
`publish` are idempotent for exact retained commitments. `inspect(Status)` queries
release progress and `observe(Status)` returns it through a replicated call.
`inspect(List)` returns at most four metadata entries after an exclusive release
cursor. These transport distinctions do not grant fresh installation authority.
`configure` changes admission quotas without reinstalling; `inspect(Info)`/`observe(Info)`
report their usage through query/replicated reads. Root controls the Store during
autonomous work. Root's controller-only
`publish_store_artifact` endpoint accepts three commands from
`canic-management-protocol::publication`: prepare an immutable manifest, supply
one chunk, or publish. Read retained progress with `observe_store(ReleaseStatus)`. Each request binds a canonical
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

Root also explicitly mounts `observe_store`: one replicated `observe(Info)` call
under the same controller, target and deadline checks. It returns Store's bounded
initialized-state report, without a release label, application slot or storage
write. `make store-readiness-test` proves refusal before controller handover,
successful actual Root calls afterward, and unchanged state across repeated reads,
reply fault 12 and Root maintenance. Invalid/expired requests and unavailable
targets cannot produce a successful report. This is one readiness input for
bootstrap; it does not certify module identity, sole control or sufficient quota
for future work. Host binds certified code/controller evidence separately in
named Store bootstrap.

For preservation inspection, Store's replicated `observe(Custody)` takes one exact
release and returns its progress plus every retained pin on the shared artifact.
Pins preserve their original owner, operation and release label, including former
controllers and different labels sharing those bytes. Root mounts
the custody variant of `observe_store` under the same controller/target/deadline checks, so a
Root-owned Store does not need to give the external operator direct access.
The result is bounded by one manifest and 32 compact pin records, reads metadata
only and creates no Root copy. It is not an atomic catalog snapshot, payload
integrity scan, maintenance approval or proof of installed code. Run the focused installed observation witness
with `bash scripts/dev/test-store.sh custody` after simulator preparation.

Store's `admission` endpoint accepts only close, observe and activate. It captures
the controller and retains the exact operation and increasing sequence in its
existing settings allocation. Root forwards close and activate through `change_store_admission`. Controllers read the retained attempt through the Admission selector of `observe_store`. Both enforce the original deadline and placement fence. While
closed, prepare/put/publish, pin acquisition, quota changes and fresh cache work
are refused; reads and exact attempt recovery remain available. Store upgrades
leave the gate closed until explicit matching activation. Old cache callbacks
are fenced even if admission has reopened. The gate does not certify that earlier
remote calls have drained or authorize installation. These are runtime primitives,
not another CLI upgrade path. `bash scripts/dev/test-store.sh admission` qualifies
them locally; its test controller still performs Store installation explicitly.
Store record migration and downgrade restrictions are owned by the supplier's
[settings codec](https://github.com/dragginzgame/ic-wasm-store/blob/05755252817fdaff23455ab6550d98695fe3a169/crates/ic-wasm-store/src/state/records.rs).

Root's `maintain_store` (Stage, Advance, Install or Activate) and
`observe_store_maintenance` mounts now qualify the next boundary under sole Root
control. Root retains each immutable approval and stop-request commitment outside
Store. Up to eight attempts are retained, with no eviction or reset. It fences child/publication mutations and raw gate changes, closes Store,
then issues stop once. Lost callbacks recover through observations; expiry never
renews dispatch authority. A stopped observation establishes this drain boundary.
The CUJG journal preserves current and completed attempts across supported Root
upgrades, within the existing 16 KiB ceiling. A pre-cut Root keeps its original
executable and evidence. Existing complete child-custody
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
commit management-cache contents. The current Root refuses the old CUJ5 backing. The predecessor retains its original stop and scan evidence.
Older Stores lacking this endpoint cannot complete the new pre-stop scan.

The explicit `maintain_store` Install request accepts the approved raw replacement
(at most 1 MiB for this first infrastructure path). Root uploads it once to Store's
management chunk cache, records exact upload/install commitments, and installs it
through Platform's existing chunked management contract. Host keeps the durable
bytes; Root never stores them. Lost upload replies recover by chunk observation;
installation recovery checks the approved replacement code and sole Root control.
Issued effects are never repeated, including after Root replacement.

The continuation starts the replacement with admission closed, repeats the same
catalog scan, and requires complete matching preservation before Verified.
The Advance request leaves admission closed. Only an explicit
Activate request issues the matching Store activation; lost replies
recover by observation. Ready is retained historical completion, not a fresh
readiness guarantee. It releases child/publication fences while retaining the
operation. Every new effect retains the original deadline; observation after
expiry cannot authorize a new start or activation. Rejected or incompatible
replacements remain blocked without installation replay.

`bash scripts/dev/test-store.sh maintenance` qualifies this runtime path using
sole Root control and compares all Store stable bytes before/after replacement,
including payloads, pins and an unfinished upload. Runtime verification itself
compares metadata; it is not a full payload integrity scan. A retained 0.215 Root
completes the first replacement before migrating, then the fixture admits seven
successors, recovers a lost second upload callback and refuses a ninth attempt.
Historical replay returns its own evidence without touching a successor. Callbacks
match both approval and phase. The CLI witness below qualifies two replacements
followed by Root and child upgrades; mainnet maintenance remains unqualified.

`fleet show` reads Root's maintenance metadata without a signer or continuation.
It selects only Roots and known Store identities from retained Store bootstrap
plans. A Root built with `store_maintenance_status` reports closure, stopping,
installation, verification and activation separately. Older Roots report unknown
when that method is unavailable. Configuration roles alone do not select queries.
The report is uncertified; completed maintenance is historical evidence, not
current Store admission or readiness. Terminal and localhost views identify the
reported attempt. `fleet status` and apply/reconcile progress identify their own
retained attempt, which may precede the latest fleet report. `--details` includes
the original deadline, before/after catalog counts and the current implementation's
qualified eight-attempt limit. That limit is not an observation of remaining
capacity or proof that an older deployed Root supports repeat maintenance.
For an existing selected app/fleet with its retained journal:

```sh
canic fleet show --details
canic fleet show --json
```

The same report appears in the opt-in localhost viewer. No view signs requests,
advances maintenance or changes completion receipts.

For Host reconciliation, Root mounts a distinct controller-only
`recover_store_maintenance`. It observes already-issued outcomes and can finish
post-install catalog verification, retaining that evidence locally. It never
issues a fresh close, stop, upload, install, start or activation, even with a live
approval. It retains the original operator and deadline; expired dispatch may
still be observed. The Advance request to `maintain_store` can dispatch new effects
and must not be used as observation-only recovery. The existing fleet commands
use these explicit contracts for bounded repeat Store replacements.

Before and after a signed Root installation, Host calls `observe_root_custody`
as the controller. The reply contains every service child journal, its artifact
and lifecycle evidence, the current Store operation and its completed predecessors.
Host compares the reply with the approved maintenance plan on both sides of the
installation. Any unapproved evidence blocks completion. Store plans bind the
sequence and read the exact approval with `observe_store_maintenance(Some(approval))`.

For a completed local fleet created on this Root generation, change the Store
Wasm input to the reviewed replacement. Run these commands from the app directory
with an identity that controls Root.

```sh
canic fleet plan --fleet local --identity canic-local
canic fleet status OPERATION --fleet local
canic fleet apply OPERATION --fleet local --identity canic-local
canic fleet reconcile OPERATION --fleet local --identity canic-local
```

Use the returned operation digest. Planning binds both canister identities, their
code, replacement bytes and original operator/deadline; it does not dispatch effects.
Apply closes, scans, stops, installs, restarts, verifies and explicitly activates
through Root. Each invocation has at most 32 continuations. Pending output suggests
reconcile for issued effects and verification, or explicit apply for new effects
within the original dispatch window. Reconcile can settle issued effects and verification, but
leaves a stopped or verified Store waiting for explicit apply. `status` reads only
retained local evidence. Interrupted attempts keep their signatures and deadlines.
Terminal status groups retained requests by evidence state so catalog scans do
not fill the display with repeated rows. Unknown and rejected attempts remain
visible; `--json` retains every individual attempt. Counts are not completion proof.
Before/after catalog counts describe retained metadata scans, not payload integrity.
Expired dispatch authority or a retained failure in a fresh-effect phase points
to status review; it does not authorize a new plan or replacement attempt.

The planner refuses concurrent artifact changes and unfinished child/publication
work. Authenticated Ready evidence advances the retained Store hash, enabling later
Root maintenance and child publication/upgrades. A completion pointer without its
signed observation cannot do so. Only a completed predecessor permits the next
sequence and code transition. The current scope is eight retained Store attempts,
at most 1 MiB of replacement Wasm and 4096 observations per attempt. No reset,
cancellation, history eviction or mainnet maintenance is qualified. Exhaustion or an
unrecoverable effect remains blocked with its original evidence.

To create a Store, use the existing named `fleet plan` workflow. The selected
fleet must have a bound top-level `root` and an unbound `wasm-store` child with a
Wasm input. The reviewed Root executable must mount `observe_store` and
`publish_store_artifact`. Planning retains exact Store bytes, Root identity/code,
operator, network and creation budget; it creates nothing.

From the app's config directory, after installing this CLI with `make canic`,
building Store, and binding the existing Root ID, declare the Store if absent.
The Wasm path is relative to the config. Substitute reviewed hashes and returned
operation/Store IDs below. Apply creates a canister using synthetic cycles on a
local fleet or debits the operator's cycles-ledger account on mainnet (default
ceiling 1T, including the ledger fee):

```sh
canic app canister add store --app starter --role wasm-store --parent root --wasm target/wasm_store.wasm
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
bounded chunk contract. Host installs Root modules up to 8 MiB, switching from direct ingress to journaled
chunked installation above 1,900,000 bytes. Inline children retain their own custody limits. No measured deployment
speedup is claimed. Whole-fleet release selection and Root maintenance with full
Store custody use the workflows above, including repeat Store replacement.
Existing-fleet migration remains pending.

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
several artifacts, but cannot change an existing pin's release label. `observe(Pin)`
returns replicated retention evidence. At most 32 pins per artifact are retained;
exhaustion refuses new obligations without evicting old ones. Pins do not expire
or authorize installation. Persistent pin formats and migrations are owned by the
supplier's settings and artifact codecs, not Root.

`prepare_cache` requires an exact retained pin and prepares its entire bounded
artifact. It observes the cache once, uploads only missing chunks from stable
storage, and verifies every returned hash. Its dispatch deadline is at most five
minutes from admission; caller control and deadline are checked before each
upload. There are no background retries or attached cycles. A cache hit is an
observation at that moment, not a guarantee against subsequent cache clearing.
Repeating a request observes current cache contents before repair. This does not
grant Root permission to replay an uncertain installation.

Root stages and reconciles Store-backed creation through `stage_child_creation`
and `reconcile_child_creation`. Code upgrades use `stage_child_upgrade`
and `reconcile_child_upgrade`. Both operations bind the exact lifecycle
inputs when supplied. `observe_root_custody` returns the complete maintenance
record. These contracts live in `canic-management-protocol::application`. Existing effect approvals retain their old
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
authorizes install replay. An operator-approved cache-repair flow, existing-fleet
conversion remain pending. Root-driven Store
maintenance and fleet CLI integration are qualified above. The management-cache
listing has a bounded 64 KiB reply decoder; an oversized
listing blocks preparation/recovery rather than allocating without a bound.

Inspect a retained child creation, child upgrade or whole-fleet operation locally:

```sh
canic fleet status OPERATION --app starter --fleet local
canic fleet status OPERATION --app starter --fleet local --json
```

Status shows the pin state and original cache preparation deadline from retained
authenticated Root observations. Partial recovery replies update child progress
without erasing earlier delivery evidence. A later full observation cannot move
a pin backwards, remove a retained deadline or replace it with a different one;
conflicting journals refuse status and further operation processing. Admission-only
replies contain no delivery evidence and display unknown until it is observed,
even when a release is approved.
Store-backed child JSON includes `delivery` (or `null`); inline children omit it.
The effect table distinguishes `prepare cache` from inline `upload`. Neither a
retained pin nor a past preparation establishes current cache contents. Status
does not call Root or Store, sign requests or grant a new repair allowance.

The current bounded contract accepts raw artifacts up to 8 MiB in 1 MiB chunks.
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
fixed 1T creation grant from its existing cycles balance. New plans default to
dispatch when ready, with an expiry ten minutes after planning. An explicit future
`--execute-at UNIX_SECONDS` retains pre-dispatch admission and the following
ten-minute effect window. Existing Root code must support elapsed-time admission;
older Roots need an explicit future schedule or separately reviewed maintenance.
Keep Root funded. Replanning never rewrites retained deadlines.

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

For an upgrade, publish a different executable under a new immutable
version, update the app's Wasm input, and use the existing target/predecessor
arguments with the new publication operation:

```sh
canic fleet plan --app starter --fleet mainnet --canister app --release NEXT_PUBLICATION_OPERATION --target APP_ID --previous-module PREVIOUS_SHA256 --root-module ROOT_SHA256 --identity canic-mainnet
canic fleet apply UPGRADE_OPERATION --app starter --fleet mainnet --identity canic-mainnet
```

The final command is the greeting demo's read-only query. Root obtains an exact
Store pin, prepares/reconstructs the derived management cache and performs the
upgrade through its existing effect owner. An inline child cannot switch to Store
custody through this flag. A retained whole-fleet creation approval lets named
planning use the same bounded serial history, Root/Store maintenance bindings and
completion checks as whole-fleet convergence. Standalone named deployments without
that fleet history retain their first-upgrade limit. No live mainnet run or latency
improvement is claimed.
