# Changelog

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
