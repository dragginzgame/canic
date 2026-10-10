# Blob Service Composition

Application blob storage belongs to [ic-blob-storage](https://github.com/dragginzgame/ic-blob-storage).
It owns content, tenant authority, provider access, references and accounting.
Canic owns Fleet lifecycle and deployment of ordinary application Components.

The Canic-owned adapter lives at `integrations/blob-service` in the main Canic
workspace, sharing its dependency catalog, lockfile and release version. The upstream library has
no Canic dependency. See the [extraction design](../../design/0.112-standalone-blob-service-extraction/0.112-design.md)
and [current handoff](../../status/current.md) for implementation evidence.
The shared [package-adapter boundary](../../architecture/independent-package-adapters.md)
also covers backup.

The adapter pins the published `ic-blob-storage` library and uses public Canic
endpoints, bounded decoders and synchronous lifecycle participants. It registers
its service memory grants before Canic bootstraps the sole memory runtime. Its
managed lifecycle retains Fleet admission.

Blob unit/protocol/provider tests live upstream. The adapter qualifies
its dedicated and embedded managed composition through Canic's public fixtures
and a caller-owned PocketIC server. It adds no separate server owner or standalone
service mode to the framework workspace. Canic retains its generic endpoint,
Fleet, lifecycle and memory tests. The upstream service suite owns blob authority,
certificate replies, restoration, provider behavior and accounting; those results
do not establish deployment qualification for an arbitrary wrapper.

The root catalog selects published Blob Storage 0.22 and Memory 0.35.
Timers and arithmetic-only Metrics follow the selected Canic runtime in the root
and independent consumer lockfiles.
Each complete managed artifact must share one Memory and Timers runtime.
See [Canic#444](https://github.com/dragginzgame/canic/issues/444) and the
[current handoff](../../status/current.md) for scoped qualification. Building
the shell does not establish live provider behavior. Upload capacity replies expose
tenant logical, global physical and global billing-liability byte headroom
separately; `remaining_bytes` is their minimum. Consumers must rebuild against
Blob 0.22 contracts. This is a pre-1.0 hard cut with cross-release reinstall, while
same-release restoration and the upstream mutation fence remain required.

## Dependency version ownership

The service requirement has one source:
[`Cargo.toml`](../../../Cargo.toml),
under `[workspace.dependencies]`. Its package inherits that declaration with
`ic-blob-storage.workspace = true`; passive wire contracts come directly from
`ic-blob-storage-contracts` on the same 0.22 release line. The two consumer examples depend on the
adapter and resolve those requirements through their own lockfiles.

Locked builds select the exact service protocol and lifecycle composition with
the single Memory runtime. Adopting another service release requires renewed
managed composition qualification.

`canic.toml` configures Apps, roles, topology and runtime policy; it does not select
Rust crate versions. The adapter inherits the main workspace declaration; refresh
the root and both consumer lockfiles when adopting a published release.

## Rust toolchain

The adapter and both independent consumer workspaces support Rust 1.91.0.
This matches Canic's package floor; the checkout's pinned Rust 1.99 toolchain
remains the maintenance default. Install the minimum-version Wasm target with
`rustup target add --toolchain 1.91.0 wasm32-unknown-unknown`. Use
`RUSTUP_TOOLCHAIN=1.91.0 canic build ...` to qualify a managed consumer with
that compiler. Direct Cargo Wasm builds do not supply managed App/role authority.

The complete managed minimum-version builds include the current Coordinator
descriptor-gating correction. Adapter package verification alone does not qualify
older Canic build tools or generated infrastructure at that compiler floor.

## Choose placement

Blob management is optional and can run inside an existing application canister
or in a dedicated Fleet Component. Both use the same upstream library and Canic
adapter. A Rust package boundary does not require a separate canister.

Embedding shares the application's identity, cycles, stable-memory budget,
execution and lifecycle. A dedicated Component isolates those responsibilities
and adds calls between the application and service. Choose placement from the
workload and operational boundary, not just a byte total. Provider-managed blob
bytes are not all stored in Canic stable memory; these options do not establish
terabyte capacity or provider qualification.

## Embed in an application

Invoke `canic_blob_service::mount!();` once at the application's crate root.
It mounts endpoints and registers seventeen permanent memory requests. The
artifact owns one pool with the service namespace and every other application's
namespace explicitly granted, for example:

```rust
canic::memory::memory_allocation_pool!(
    authorities = [
        ("embedded-app", "embedded_app."),
        (canic_blob_service::MEMORY_AUTHORITY, canic_blob_service::MEMORY_KEY_PREFIX),
    ],
    exclusions = [],
);
canic_blob_service::mount!();
```

Core and Control Plane grants are included by Canic. All owners share eligible
physical IDs; there are no numeric component partitions. Exclusions protect
actual unmanaged physical memories only. The adapter owns service keys and
authority; one service instance is supported per canister.

The application keeps its existing `canic::start!` and `canic::finish!` and its
exact App/role metadata. Compose these synchronous functions in its lifecycle
participant, after Canic restores the framework:

```rust
// After bounded decoding of the application's installation input:
canic_blob_service::lifecycle::install(&input.blob);
// In the same-release post-upgrade participant:
canic_blob_service::lifecycle::restore();
```

The owning `start!` must bound the outer envelope before restoration. Decode
application configuration once under explicit limits before dispatching it to
participants; `lifecycle::ENVELOPE_LIMITS` and `CONFIGURATION_LIMITS` are available
for the example's envelope and nested input. Blob installation receives the typed
`ServiceInstallationInput` and verifies the actual hosting canister Principal.
Restoration retains the service fence and must finish synchronously before async
application work. Neither function installs a timer or replaces the application
metrics sampler.

The example's input schema is not mandatory. An application may construct the
typed installation input from its own compiled configuration and actual canister
Principal inside its synchronous participant. Externally supplied configuration
must use bounded decoding. The adapter validates the supplied service identity;
it does not silently rewrite it. Runtime-supplied installation arguments use the
production target binding described below, including both checked-in examples.

Compose `canic_blob_service::metrics::sample()?` with application rows in the
host's one `ApplicationMetricsSampler`. Register that callback after installation
and restoration. This preserves both families rather than replacing either
sampler; Canic retains the timer, limits and cache. Export blob endpoints in the
same Candid scope as application endpoints, and avoid duplicate method names.

The complete [embedded example](../../../integrations/blob-service/embedded-consumer/src/lib.rs)
has its own persistent counter, endpoints, combined installation input and metrics
sampler. Its counter uses memory 120 while blob grants use 150–166. It has a
separate workspace and can be built explicitly with:

```sh
cd integrations/blob-service/embedded-consumer
../../../target/debug/canic build embedded-app --workspace . --config canic.toml --icp-root . --profile fast --json
```

Build the complete App for managed installation: selecting only `backend` emits
a compile-only artifact without a release-build identity or manifest. Retain the
complete App manifest for the managed qualification cases.

## Dedicated consumer-owned canister

The adapter is a Rust composition library, not a pre-bound App artifact. A
consumer supplies a small canister package with its own exact
`[package.metadata.canic]` App/role and a `build.rs` calling
`canic::build!("path/to/canic.toml")` for that App. Keep versions and dependencies
in the consumer's workspace declarations. Its canister source is:

```rust
canic_blob_service::canister!();
```

The shell depends on `canic-blob-service`, `canic`, `candid` and `ic-cdk`.
The adapter inherits Canic's package version and registry requirement from the
main workspace, with published Blob `0.22`. Cargo's package manifest
removes the development Canic path and excludes these private consumer fixtures.
The local facade already uses Memory 0.35. Registry Canic
0.110.54 uses Memory 0.31, so an aligned framework publication and selected
registry requirement remain necessary under [Canic #33](https://github.com/dragginzgame/canic/issues/33).
A compilable package archive cannot substitute for exact managed runtime admission.
Its first registry publication remains separate from package verification; until
then a version-only adapter declaration cannot resolve. Checked-in qualification
uses the matching local facade. A disposable package candidate can instead select
registry Canic and the extracted normalized adapter, without a sibling checkout.
Use one Canic source throughout that graph: mixing local and registry facades is
rejected by managed admission.
See the [consumer manifest](../../../integrations/blob-service/consumer/Cargo.toml)
and [canister shell](../../../integrations/blob-service/consumer/src/lib.rs).
It does not declare an `ic-blob-storage` dependency. The adapter selects that
upstream library transitively and re-exports its typed boundary contracts as
`canic_blob_service::dto`; no duplicate DTO schema is introduced. This convenience macro grants the service namespace in one host pool, calls
`mount!()`, and owns
Canic start/finish, bounded endpoints, lifecycle participation and a blob-only
metrics sampler. Do not add a
second lifecycle or copy service dispatch into the consumer. Service memory
registration is emitted by this macro; importing only the re-exported DTOs does
not register blob memory in the calling application.

Select the **consumer shell package** in the consuming App's `roles.blob.package`.
Do not point it at the adapter library. The App/role metadata checks remain exact.
A composition library may share the role's exact Canic facade package; alternate
Canic identities, access to protected internals and undeclared feature activation
remain rejected.

The checked-in shell at `integrations/blob-service/consumer` demonstrates a
separate `consumer-app` workspace and configuration. Build it explicitly:

```sh
cargo build --locked -p canic-cli --bin canic
cd integrations/blob-service/consumer
../../../target/debug/canic build consumer-app --workspace . --config canic.toml --icp-root . --profile fast --json
```

The complete App build above stamps the release-build identity required by
managed installation. A single-role build is compile-only evidence and does not
supply that identity. Use the emitted release manifest and its qualified artifact
union for installation proof.

Installation requires `canic_blob_service::dto::configuration::ServiceInstallationInput`
as Candid bytes nested after Canic's protected init payload. Its configured
service Principal must equal the actual allocated canister ID; the adapter
validates this before publishing the installed service. Construct these bytes
after allocation, without rewriting that identity inside the adapter. The public
[managed Component qualification fixture](../build-and-evidence/managed-app-qualification.md)
supports `ManagedApplicationInit::ForCanister` for this purpose in PocketIC.

For production installation, set `application_init_required = true` on the blob
Component Spec, as both examples do. Root allocates the canister and waits before
installation. Controller-visible pool status supplies the allocated Principal
and its claim operation ID; controller-authorized Root operation status exposes
that member allocation and retained binding, including Component Group members.
Then encode the application's `ServiceInstallationInput` for that Principal.
The controller submits `RootCommand::BindComponentInitialization` containing
that operation ID, exact target and 1–16,384 opaque application bytes. Host's
`component_initialization::encode_command` encodes the complete Root command;
retain those command bytes for reconciliation and exact retries.

Root atomically retains and charges the binding before installation and includes
its hash in the install intent. Its ordinary and Component Group installers
supply the exact retained application bytes after Canic's protected payload.
Wrong targets and changed initializers are refused; an identical binding remains
an effect-free replay after installation. After an interrupted or lost reply,
inspect the existing Root operation, then retry the retained command if needed.
The application schema and producer remain application-owned; updating the
independent upstream preparer does not update this managed adapter or deliver
arguments to Root. `app.init_mode` controls Fleet operating mode.
[Canic#444](https://github.com/dragginzgame/canic/issues/444) records qualified
execution and downstream acceptance separately.

The maintained adapter selects published Blob Storage 0.22.1 and Memory 0.35.
Each complete managed Wasm graph must contain one Memory and Timers identity.
Canic owns the runtime and lifecycle;
the service participant restores synchronously before deferred work. These
configuration, wire and persisted record changes are a pre-1.0 hard cut requiring
clean reinstall across releases, with same-release retry and recovery retained.

Same-release restoration retains the upstream mutation fence. The configured
operator uses `blob_resume_current_instance` to prove continuity from IC history
before resuming mutation. Framework setup may also advance the platform version;
a service update checks continuity through the existing upstream workflow.
A conservative query fence or cached `blob.fenced` gauge is not by itself a
provider-readiness verdict.

The consuming App selects Fleet admission and public metrics in its own config.
The library's normal dependency graph contains no App-specific compiled topology.
The owning shell compiles its exact topology and emits endpoints into the same
Candid declaration scope as Canic's framework endpoints.

The source cut removes Canic's embedded blob runtime, feature flags, billing
commands and passive Medic inspection. Use the service's own operator tools for
its maintained API. Its status response does not promise the former readiness
exit code, and there is no qualified replacement for Canic's direct funding
command. Neither limitation requires retaining the old runtime.

## Browser and per-project adoption

The enrolled tenant grants an exact per-upload permission naming the browser
uploader and exclusive deadline. The certificate endpoint authenticates that
actual uploader against the retained permission; the installation no longer
selects one trusted uploader. Application sessions and capabilities still belong
to the consumer. The adapter delegates this contract to
[Blob #31](https://github.com/dragginzgame/ic-blob-storage/issues/31); it does not
relay an ingress response certificate to another user.

The adapter does not export `_immutableObjectStorageBlobsAreLive`,
`_immutableObjectStorageBlobsToDelete` or
`_immutableObjectStorageConfirmBlobDeletion`. Provider behavior when those
callbacks are absent is unqualified; logical release alone cannot establish
provider deletion. [Blob #32](https://github.com/dragginzgame/ic-blob-storage/issues/32)
owns that gateway contract. Upload completion also requires the configured
external `completion_verifier`; the adapter supplies no verifier deployment
([Blob #33](https://github.com/dragginzgame/ic-blob-storage/issues/33)). The service
includes an [application-operated private native worker recipe](https://github.com/dragginzgame/ic-blob-storage/blob/v0.22.1/docs/completion-verifier.md)
using its existing observation, attestation and exact-reconciliation commands.
Its two-browser reference uses a local provider substitute. Application signer
operation, worker availability and real provider acceptance remain required.

Production post-allocation binding currently requires a Root controller. The
top-level Toko Miner Blob installation can use this existing controller route;
its Component Spec must require application initialization and its application
producer must retain the target-bound command bytes. The
existing parent/peer provisioning APIs do not authorize an application hub to
bind target-dependent bytes. [Canic #493](https://github.com/dragginzgame/canic/issues/493)
owns that authority path. A configured project `payment_account` also does not
establish provider linkage or attributable external top-up credit. The current
passive funding views do not dispatch payments; provider funding evidence and
its adapter composition remain separate prerequisites
([Blob #34](https://github.com/dragginzgame/ic-blob-storage/issues/34),
[Canic #494](https://github.com/dragginzgame/canic/issues/494)).

## Usage reporting

The dedicated shell registers a synchronous `ApplicationMetricsSampler` after
install and restore. Embedded applications compose the public `metrics::sample`
function in their own sampler. It reads one maintained upstream upload-accounting record; it does
not scan operations, call a provider, mutate continuity state or maintain a
second ledger. Canic's existing sampling timer and public cache own scheduling,
history and freshness. Failed accounting reads retain the previous sample with
its original timestamp; they do not publish zeros.

The example consumer's `canic.toml` explicitly enables public aggregate reporting:

```toml
public_metrics = ["application"]
```

Omit `application` to disable publication and sampling. This is a public opt-in:
only canister-wide totals are published, without tenant, object or payment
identities. All rows are gauges, attributed to the storage canister itself.

| Metric | Meaning |
| --- | --- |
| `blob.logical_bytes` | Tenant logical bytes plus active reservations |
| `blob.physical_bytes` | Confirmed physical bytes plus active reservations |
| `blob.liability_bytes` | Unsettled confirmed bytes plus active reservations; not a currency amount |
| `blob.reserved_bytes` | Active reservations, already included once in each byte total above |
| `blob.active_reservations` | Active or possibly exposed operations, including zero-byte uploads |
| `blob.operation_slots` | Lifetime retained operation slots, including cancelled and settled entries |
| `blob.fenced` | Last observed upstream mutation fence: 1 fenced, 0 unfenced; not a provider-readiness proof |

Do not sum the three byte totals or add reservations to them. Logical reference
release does not establish physical deletion or extinguish provider liabilities.
These are source accounting units, not measured stable-memory size or a bill.

For Toko Miner, deploy its adapter-backed shell as an ordinary managed Component, configure
the upstream service and use its client APIs for blob operations. Then collect
the selected Fleet with the existing command:

```sh
canic --environment staging observatory snapshot toko-staging --out usage.json
```

Each private `roles[]` entry carries the storage canister ID and
`application_metrics`; numeric values are exact decimal strings. Public
JSON/HTML reports show these metrics under their role instance, including source
state, time and truncation. See [Fleet Observatory](../operations/fleet-observatory.md#application-metrics).
The collector reads the generic `canic_public_status` cache and has no blob
dependency. It does not enumerate service canisters outside the retained Fleet
inventory, enable metrics remotely, or make paid calls.

This reports usage **per storage canister**. Usage attributed to a consuming
canister or tenant remains a separate upstream-authorized disclosure; the
adapter does not publish that breakdown or infer it from capacity headroom.
No Toko installation, provider qualification or paid upload is implied by this
reporting integration.

Reinstalling or deleting a live service is a separate operation. Preserve records
of outstanding provider liabilities; logical reference release does not prove
physical deletion or billing cessation. The source extraction performs no paid
upload, provider cleanup, deployment, or reset.
