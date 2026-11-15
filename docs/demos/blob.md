# Embedded Blob composition

The optional `canic` feature `blob` mounts three endpoints around the real
`ic_blob_storage::local::persistence::Service`. The installed application witness
is [canic-asset-catalog](../../qualification/canic-asset-catalog/src/lib.rs), in Canic's
qualification workspace. It is a finite local catalog example, not a provider
client or a production deployment recipe.

## Ownership and use

| Owner | Responsibility |
| --- | --- |
| Blob | Grants, object/reference identity, accounting, durable record and exact maintenance-attempt admission |
| Canic | Optional endpoint composition, captured caller/time, bounded Wire hooks; future fleet host integration |
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
failure traps so the IC rolls back both stores. The qualification application's
controller-only fault selector exercises those failures; it is not in the product
adapter. This test application must not be mistaken for a production actor.

Fresh installation takes explicit users, manager and Blob limits, binding both
records to the actual canister. Upgrade uses a method-bound unit argument, opens
both existing stores, and refuses missing/corrupt/foreign records. It never falls
back to initialization. Pre-upgrade requires Blob drained; post-upgrade retains
that state. `probe` uses Canic's greeting contract but refuses while drained.
Only the matching explicit Blob activation can reopen admission. Inspection and
cleanup remain available under ordinary permissions while drained.

## Run locally

Prerequisites: authenticated Git access to the pinned Blob repository,
Rust 1.100.0 with the Wasm target, `make canic`, and previously prepared simulator
assets from `make qualification-setup`. Nothing opens a desktop window.

```sh
canic artifact build --manifest qualification/Cargo.toml --package canic-asset-catalog --output target/blob-catalog.wasm
canic artifact inspect target/blob-catalog.wasm
make blob-test
```

`make blob-test` builds original and maintenance-candidate executables through
Canic's existing artifact builder and runs five installed tests with synthetic
cycles. `make blob-clippy` and `make blob-doc` select the adapter and actual caller.
The two artifacts and input receipts stay under `target/blob/`; no live IC or
Caffeine request is issued. Optional feature selection keeps Blob/Storage/Runtime
out of Canic's default runtime graph. The library now comes from the exact
0.100.6 commit `decde204acb6d7c8362a0003e904beef72ff89b6`, which contains the
originally qualified Service implementation. The mutable sibling checkout is no
longer a Cargo input; it stays read-only. This removes the external-path exception
and the isolated formatting hook's former sibling-resolution limitation.

The input recorder resolves the supplier via locked Cargo metadata, requires
an exact Git revision, and records library/context hashes and consumer locks
before/after each installed run. Release evidence is recorded in the
[qualification history](../qualification/history.md).

Canic now selects Wire 0.24.1 for its adapter and application catalog. The pinned
Blob service retains Wire 0.22.4 for its independently owned persistence codec.
`canic::blob::RPC_LIMITS` projects the supplier's numeric budgets into the adapter's
Wire type; it does not change admission policy or add a codec implementation.
The installed composition checks qualify that boundary. The older input hashes
below describe their dated run, not the current dependency selection.

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

## Management boundary still to integrate

Current Canic Host and Root install/upgrade effects send empty application
arguments. They do not retain Blob's drain attempt or perform its required
stop/reopen/verification/explicit-activation sequence. The installed refusal is
intentional: no default configuration, init-on-open-error, implicit activation,
readiness bypass or second management engine has been added to make it fit.

Before fleet plan/apply can manage this app, its existing immutable approval and
effect owners must bind explicit lifecycle arguments and service maintenance
attempts, preserve original request outcomes, and qualify the complete sequence.
The present fixture uses a trusted simulator controller and a distinct explicit
manager; it does **not** claim Root-owned creation or a Canic host management
journal for Blob. This is a Canic integration obligation, not a missing supplier
Service contract.

Blob remains experimental: finite 64-record supplier history, four lifetime
catalog entries and declared-byte accounting. Releases do not free historical
slots or establish provider settlement. There are no Caffeine calls, certificates,
provider proofs, compaction, snapshot recovery or completed provider-journey claim.
The [supplier input policy](../../AGENTS.md#local-blob-composition) requires
the pinned immutable supplier input and requalification when it changes.
