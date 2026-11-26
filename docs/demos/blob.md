# Embedded Blob composition

The optional `canic` feature `blob` mounts three endpoints around the real
`ic_blob_storage::local::persistence::Service`. The installed application witness
is [canic-asset-catalog](../../qualification/canic-asset-catalog/src/lib.rs), in Canic's
qualification workspace. It is a finite local catalog example, not a provider
client or a production deployment recipe.

The published 0.113.0 Service accepts the application’s Storage 0.80 lease.
Use `canic::blob::open_service` on install and reopen to attach the
Canic metrics store and log sink. Custom lifecycle hooks call
`canic::lifecycle::initialize_diagnostics()` first.
Use fresh backing for this Storage generation. The historical 0.100.6 catalog retains its original executable. [Storage #8](https://github.com/dragginzgame/ic-storage/issues/8)

The [Asset Library walkthrough](../../apps/examples/asset-library/DESIGN.md) builds the same actor without the qualification fault endpoint and installs it for a two-user exercise.

## Ownership and use

| Owner | Responsibility |
| --- | --- |
| Blob | Grants, object/reference identity, accounting, durable record and exact maintenance-attempt admission |
| Canic | Optional endpoint composition, exact Host approvals and Root management-effect custody |
| Application | Two admitted users, catalog names, four lifetime catalog entries, publication and release decisions |
| Platform / Wire | Generic IC execution and entrypoints / bounded codecs |
| Storage | One application-owned bootstrap with granted catalog and `blob.control.v1` leases |

`canic::mount_blob_endpoints!` binds explicit local execute, inspect and management
handlers. It mounts no lifecycle, storage owner, timer or activation. Public
uploader/manager calls preserve the actual ingress principal. Catalog operations
check the real user's permission before calling the same local Service under the
application's actual installation principal. No self-canister calls or awaits
occur in these transactions. A controller or manager is not automatically a user.

Catalog names, identities and capacity are checked before Blob mutation. The
catalog stores the original authorization and its own publication decision; it
does not mirror Blob liveness, counters or admission. Supplier ordinary errors
leave state unchanged. A later catalog persistence-encoding or reply-encoding
failure traps so the IC rolls back both stores. The qualification build's
controller-only fault selector (enabled by `fault-injection`) exercises those failures; it is not in the product
adapter. This test application must not be mistaken for a production actor.

Fresh installation takes explicit users, manager and Blob limits, binding both
records to the actual canister. Upgrade uses a method-bound unit argument, opens
both existing stores, and refuses missing/corrupt/foreign records. It never falls
back to initialization. Pre-upgrade requires Blob drained; post-upgrade retains
that state. The replicated `readiness` endpoint reports `Fenced` while drained.
Only the matching explicit Blob activation can reopen admission. Inspection and
cleanup remain available under ordinary permissions while drained.

## What the actor looks like

The actor is an ordinary Rust module. This is the composition portion of the
current [catalog actor](../../qualification/canic-asset-catalog/src/canister/mod.rs),
with imports, application handlers and lifecycle hooks omitted:

```rust
canic::mount_blob_endpoints! {
    execute = crate::canister::execute_blob,
    inspect = crate::canister::inspect_blob,
    management = crate::canister::inspect_management,
}

fn execute_blob(caller: Principal, command: Command, now: u64) -> Result<Outcome, BlobError> {
    STATE.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("catalog")
            .blob
            .execute(caller, command, now)
    })
}
```

The mount captures the real caller and time, admits and decodes the supplier
contract, calls this handler locally and encodes the reply in the same transaction.
The application's `State` owns the real Blob Service and catalog. Its explicit
Platform `init`, `pre_upgrade` and `post_upgrade` hooks create, drain-check and
reopen those owners; mounting does not run them.

Auth uses the same composition style through `canic::mount_auth_endpoints!` and
`canic::mount_auth_issuer_endpoints!`, each with its own typed handlers. Custom
application methods use Platform's `#[runtime::query(...)]` or
`#[runtime::update(...)]` with explicit guards and codecs. There is no universal
actor base class or generic `canic::mount!` API in the current rewrite. Root and
Store remain separate infrastructure canisters; an application chooses its helpers.

## Composing another durable module

The catalog's optional `profile` feature embeds the existing profile engine
alongside Blob. Both use one Storage runtime and one application lifecycle. Profile
contributes its two stable requests to the combined manifest; the application
explicitly grants its namespace. After bootstrap, the lifecycle selects creation
or strict reopening:

```rust
// Fresh installation, after the actor's combined Storage bootstrap:
let blob = Service::create(
    blob_memory,
    installation,
    Installation {
        application: installation,
        manager: input.manager,
        limits: input.limits,
    },
).expect("fresh Blob");
let profiles = canic_profile_worker::state::State::create(&runtime);
```

The separate upgrade branch strictly reopens both:

```rust
let blob = Service::open(blob_memory, installation)
    .expect("existing Blob; no initialization fallback");
assert!(blob.is_drained(), "reopen retains closed admission");
let profiles = canic_profile_worker::state::State::open(&runtime);
```

The [composition root](../../qualification/canic-asset-catalog/src/canister/state.rs)
opens catalog, Blob and profile state. Profile owns its existing record format,
owner rules and revisions; it no longer allocates a private backing runtime.
Its separate `storage::open` helper preserves the standalone demo's original
two allocations and 32-page ceiling. This composed application admits four
allocations under its own 64-page ceiling. No generic participant registry,
second allocator or lifecycle state machine was added.

Application profile calls check actual catalog-user membership and profile
ownership. The actor chooses to close those calls while Blob is drained; it reads
Blob's actual state rather than copying an admission flag. Both profiles reopen
on upgrade, but access remains closed until the matching explicit activation.
Changing the durable participant set requires an explicit migration: upgrading
between Blob-only and profile-composed artifacts is refused without changing state.
An installed corruption case damages each profile allocation separately, through
Storage's named leases. The candidate refuses reopening while preserving every
byte of the damaged backing and the intact Blob/catalog observations; readiness
stays closed. A healthy sibling component never authorizes replacing failed state.
The same catalog rollback checks also run with both profiles populated: failed
Blob/catalog authorization and release preserve the entire backing. Filling the
four lifetime catalog slots still permits profile updates and asset cleanup.
Malformed profile frames and stale revisions preserve all three owners. The
application admits only two profile users, so this witness does not exercise the
profile engine's separate 16-owner capacity limit or physical memory exhaustion.
This is a local composition witness, not an Auth+DB+Blob production recipe.

After `make canic`, build the optional composition without deploying it:

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --features profile --output target/blob/composed.wasm
canic artifact inspect target/blob/composed.wasm
```

With simulator preparation complete, run its installed boundary checks using
`bash scripts/dev/test-blob.sh blob_profile_tests`.

## Run locally

Prerequisites: Rust 1.100.0 with the Wasm target, `make canic`, the prepared ICP
signer and simulator
assets from `make qualification-setup`. Nothing opens a desktop window.

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --output target/blob-catalog.wasm
canic artifact inspect target/blob-catalog.wasm
make blob-test
```

`make blob-test` builds Blob-only and optional profile-composed executables through
Canic's existing artifact builder and runs ten installed tests with synthetic
cycles. `make blob-clippy` and `make blob-doc` select the adapter and actual caller.
The two artifacts and input receipts stay under `target/blob/`; no live IC or
Caffeine request is issued. Optional feature selection keeps Blob/Storage/Runtime
out of Canic's default runtime graph. The previous installed witness used published Blob 0.103.3,
archive SHA-256 `559aa9a43ac96817d4c703285007e7bbd3b413b134efc2952d17646c04c52639`,
from source `ff1aaafc75d80cd51c12f232dd758d4883a7aa21`.
That witness used Runtime 0.23.16 and Types 0.21.2; Storage's older raw-memory
runtime edge has no executor.
The mutable sibling checkout stays read-only and is not a Cargo input.

The input recorder resolves the supplier via locked Cargo metadata, requires
the same registry archive in both locks, and records library hashes and consumer locks
before/after each installed run. Release evidence is recorded in the
[qualification history](../qualification/history.md).

Canic and Blob use Wire 0.31.1. `canic::blob::RPC_LIMITS` re-exports the
supplier's protocol constant. Blob owns its persistence codec.
The older input hashes below describe their dated run.

## Original local qualification evidence — 2026-11-15

The local supplier identifies itself as **0.100.6**, with HEAD
`3c26f6c5c9bd0a32e0ad1a4c7d644f056c6c5e46` **plus dirty files**. This is not a claim
that GitHub or crates.io contains the qualified input. No supplier change was
necessary or made by Canic. Its separate qualification workspace was inspected
read-only. A concurrent supplier version bump from 0.100.5 to 0.100.6 was detected
at final inspection; both consumer locks and all focused checks were refreshed.

`scripts/dev/blob-inputs.pl` records HEAD/status, per-file library/manifest/lock
hashes and both consumer locks. `test-blob.sh` compares the receipts before and
after its run. The recorded supplier-content digest is
`018858ca8a04f3b68930b063aac5d22ec093da4b532b13b154b1345403ee64f3`.
That digest covers the sorted path-to-hash map, including supplier context locks;
the actual builds resolve through Canic's qualification lock, not Blob's lock.
Consumer root lock SHA-256:
`f75d30f4a0ca0a987517d979b508e719254a818650c7f1c21b2bcbecb1283fc8`;
qualification lock:
`3b166cfa6565a712271d5d18bd6b8f299e37c6e1917cc0339abe819db5a97943`.
The source/lock receipts stayed identical before, after and at final inspection.

The deployed witness graph has one Storage 0.74.1, Runtime 0.22.5, Types 0.20.4
and Wire 0.22.3. Requalified after the requested Platform/Wire and Tooling
0.44.0 adoption; supplier source bytes remain unchanged.
Original Wasm SHA-256:
`5f37416477383a8385d19597844643ed4128a2a7af375af6716088d5fe91ed0e`;
candidate:
`bbbb34567037869d3a49d487df0a283943e0a94b0c7e00689c686fcac396e4c3`.

The five installed cases passed in 7.27 seconds:

- Two users attach distinct references to one object; direct cross-user, forged
  application-authority, controller-as-user and controller-as-manager calls refuse.
- Catalog conflicts/capacity leave Blob unchanged. Failures after Blob mutation
  and after both durable writes roll back both stores; successful release keeps
  another live use. Historical replay cannot republish a released use.
- Malformed, truncated, trailing, oversized, wrong-method and wrong-direction
  frames at all seven application/adapter endpoints preserve stable memory.
- Undrained upgrade and incompatible empty upgrade arguments refuse. Drain,
  stop, two different executables, reopen and explicit activation preserve both
  owners. Old drain replay does not close an active attempt; stale activation
  cannot reopen a later attempt, including after another upgrade.
- Disposable recipients refuse empty, corrupt and foreign stable memory rather
  than initialize. The original application and Blob state remain unchanged.

Focused native/Wasm Clippy and Rustdoc passed. The default Canic graph excludes
Blob; the composed graph proves shared identities. Runtime logs are in
`target/qualification/adoption/blob-test.log`; lint/doc and graph evidence is
beside it. These are local simulator results, not mainnet, provider or macOS execution.

## Fleet provisioning and maintenance

The additional installed witness uses the real CLI, signer, separate Store and
Root. `fleet plan --install-args FILE` admits at most 4096 exact application bytes
with a published release. Apply never rereads that file. The creation plan, Store
pin, Root lease and actual management call bind the same bytes. The catalog's
initialization explicitly names Root as manager; its real installation principal
owns both stores. Arguments are not inferred from controller or parent roles.

For an upgrade, the app supplies its method-bound upgrade frame and the operator
selects `--blob-attempt N`. Every production Root supports this operation.

```sh
canic fleet plan --canister catalog --root-module ROOT_HASH --release PUBLICATION \
  --target CANISTER --previous-module OLD_HASH --install-args catalog-upgrade.bin \
  --blob-attempt 1 --identity canic-mainnet
canic fleet apply OPERATION --identity canic-mainnet
canic fleet status OPERATION
canic fleet activate OPERATION --identity canic-mainnet
```

These commands require an existing configured Root/Store fleet, an app-defined
candidate Wasm and completed publication. They do not fund or deploy this fixture
automatically. The qualification-only Root also enables `fault-injection`; never
select that feature for a live deployment.

Root retains the original supplier attempt and request commitments in the existing
artifact lease, alongside its creation/upgrade journal. It observes closed admission,
stops the child, upgrades with exact bytes, verifies code/version, starts and inspects
the reopened service. Admission remains closed until `fleet activate` separately
authorizes that exact attempt. Unknown outcomes retain issued exposure;
reconciliation observes without resubmitting those effects. The witness loses each
drain/stop/upgrade/start/activation callback, reopens Root and resumes with new CLI
processes. Both users, shared references and a pending declaration survive. Changed
authority/arguments, malformed frames and stale activation refuse. Existing local
transaction rollback cases remain separate.

The predecessor test requires Storage 0.80 to refuse the populated 0.100.6 directory and preserve its bytes and maintenance admission. See its [binary provenance](../../qualification/canic-qualification/fixtures/README.md).
The named CLI journey retains its existing one-successor bound; repeated Root
admission is separately exercised for stale-attempt refusal. Whole-fleet configured
arguments and Host-driven Root/Store preservation of these records remain
unqualified and refuse rather than truncate custody. CUA5 records reuse the original
checked lease; older executables must refuse them. No supplier changes were needed.

Blob remains experimental: finite 64-record supplier history, four lifetime
catalog entries and declared-byte accounting. Releases do not free historical
slots or establish provider settlement. There are no Caffeine calls, certificates,
provider proofs, compaction, snapshot recovery or completed provider-journey claim.
The [supplier input policy](../../AGENTS.md#blob-boundary) requires
the pinned immutable supplier input and requalification when it changes.
