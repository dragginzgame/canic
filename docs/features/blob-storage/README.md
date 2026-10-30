# Blob Service Composition

Application blob storage belongs to [ic-blob-storage](https://github.com/dragginzgame/ic-blob-storage).
It owns content, tenant authority, provider access, references and accounting.
Canic owns Fleet lifecycle and deployment of ordinary application Components.

The Canic-owned adapter lives at `crates/canic-blob-service` in the main Canic
workspace, sharing its dependency catalog, lockfile and release version. The upstream library has
no Canic dependency. See the [extraction design](../../design/0.112-standalone-blob-service-extraction/0.112-design.md)
and [current handoff](../../status/current.md) for implementation evidence.
The shared [package-adapter boundary](../../architecture/independent-package-adapters.md)
also covers backup.

The adapter selects published `ic-blob-storage` and uses the contracts package's
shared endpoint declarations. Canic supplies endpoint authentication,
bounded decoders, actual caller context and synchronous lifecycle participants. It registers
its service memory grants before Canic bootstraps the sole memory runtime. Its
managed lifecycle retains Fleet admission.

Blob unit/protocol/provider tests live upstream. The adapter qualifies
its dedicated and embedded managed composition through Canic's public fixtures
and a caller-owned PocketIC server. It adds no separate server owner or standalone
service mode to the framework workspace. Canic retains its generic endpoint,
Fleet, lifecycle and memory tests. The upstream service suite owns blob authority,
certificate replies, restoration, provider behavior and accounting; those results
do not establish deployment qualification for an arbitrary wrapper.

The root dependency catalog selects the published Blob Storage/contracts and Memory families.
Timers and arithmetic-only Metrics follow the selected Canic runtime in the root
and independent consumer lockfiles.
Each complete managed artifact must share one Memory and Timers runtime.
See [Canic#444](https://github.com/dragginzgame/canic/issues/444) and the
[current handoff](../../status/current.md) for scoped qualification. Building
the shell does not establish live provider behavior. Upload capacity replies expose
tenant logical, global physical and global billing-liability byte headroom
separately; `remaining_bytes` is their minimum. Consumers must rebuild against
the selected Blob contracts. This is a pre-1.0 hard cut with cross-release reinstall, while
same-release restoration and the upstream mutation fence remain required.

## Retained browser sources

The native producer embeds Wire's runtime, Canic's network validator and Blob's
published decimal adapter in the same schema document as the mounted methods.
The release schema hash covers these sources. Host preserves them verbatim;
frontend delivery must not replace them with its own dependency versions.
Multiple browser-adapter registrations are refused; composition is not inferred.

The dedicated managed proof checks exact source retention. The
[retained-codec check](../../../crates/canic-testing-internal/examples/wire_composition/retained.mjs)
exercises compiled generated bindings and both owners' validators. This is local
codec qualification. Normal frontend export now compiles these retained sources
with pinned tools and binds all emitted modules to its manifest. Installed-client
qualification of the shared delivery path passes on the selected Blob 0.45 graph:
the installed Fleet SDK query/update proof exercises admitted calls and caller
denials (451s; `/tmp/canic-blob-browser-current/`). That case calls a framework
application probe, not Blob methods. Blob-specific browser upload/download and
live provider access remain unqualified.

The opt-in `pic::blob_service::tests::managed_browser_compositions` now qualifies
generated Blob tenant query/update codecs against actual installed dedicated and
embedded endpoints on Blob 0.46/Wire 0.7.3. It checks exact scope, caller denial
and unchanged mutation replay, using the normal compiler on retained supplier
modules. PocketIC supplies transport; this is not an HTTP/browser UI proof.

## Framework completion boundary

Canic qualifies the dedicated service and an embedded application through the
same managed build, installation and memory runtime. Acceptance covers exact
endpoint contracts, caller permissions, tenant mutation replay, bounded ingress,
capacity accounting and same-release restoration with the mutation fence.
Root must bind installation input to the actual provisioned component identity.
Malformed, wrong-method, trailing-byte and over-limit tenant requests must leave
service stable state unchanged; ordinary authorized requests must still succeed.

This framework boundary does not qualify a live storage provider or a downstream
application. Optional paid funding remains separate under
[#494](https://github.com/dragginzgame/canic/issues/494), and dynamic parent-owned
provisioning under [#493](https://github.com/dragginzgame/canic/issues/493).
Upstream owns provider and service-policy acceptance; Canic must not duplicate
that engine to complete its adapter. See the current handoff for the exact graph
and completed local qualification, rather than treating this scope as evidence.

## Current dependency selection

The root and both private consumer catalogs select published Blob/contracts
**0.58.0**, Tooling/Testkit **0.70.2**, Platform/Call **0.8.2**,
Memory/Storage/Metrics Stable **0.56.0**, Auth **0.23.0**, Metrics **0.17.4**,
Logging **0.17.2**, Timers **0.32.1** and Wire **0.7.23**. Each consumer graph
has one Memory/Timers runtime. The [current handoff](../../status/current.md)
names the exact local qualification and its remaining acceptance boundaries.

## Earlier selected framework evidence

Published Blob/contracts **0.47.0** with Tooling/Testkit **0.68.1**, Platform
**0.6.15**, Wire **0.7.5**, Auth **0.16**, Memory/Storage **0.51.0** and Timers
**0.31** pass the two-composition generated-client proof (536s) and all five
embedded profiles (255s). Fresh Wasm/schema builds and retained supplier bindings
pass, including caller/service-binding refusal, malformed frames and mutation replay.
Logs are retained in `/tmp/canic-blob047-installed/`; dependency admission,
strict adapter/consumer Clippy and adapter Wasm evidence is in
`/tmp/canic-blob047/`. This is Linux/PocketIC framework evidence,
not live-provider, browser UI or installed-IC acceptance. Earlier dated results
retain their original version scope.

Published Blob/contracts 0.43 use trusted registration: issuing the one-shot
certificate atomically registers retention intent and the first live reference.
`Registered` does not prove uploaded bytes, provider availability or retention.
The adapter has no completion verifier, attestation endpoint or worker.
Tenant messages use the upstream native Serde codec with method IDs `0043/001a`
and `0043/001b` and zero-based failure tags. Other method bytes and persisted
records are unchanged by upstream 0.43. Canic deployments still use clean
retirement/reinstall, without carrying predecessor state or retry bytes forward.

Both dedicated and embedded managed PocketIC cases pass on selected 0.43.0
packages, Tooling 0.65.0 and Wire 0.6.5. They cover exact endpoint declarations, bounded installation decoding,
tenant and uploader isolation, two uploads, preparation, certificate issuance,
uncollected certificate replies, retained first references, reference replay and
release, download-descriptor authority, expired/revoked permissions and
same-release restoration. Retained references are observable while fenced;
mutations require authenticated recovery. Registration is inspected using the
original identity and never authorizes a replacement certificate request.
Public accounting, sampler restoration and embedded application state are covered.

Canic-owned framework acceptance for
[#620](https://github.com/dragginzgame/canic/issues/620) and
[#621](https://github.com/dragginzgame/canic/issues/621) is locally qualified;
default-branch delivery remains separate. Native strict Clippy covers the migrated
packages and both independent consumers, including isolated internal feature
selection. Controlled fixture bytes do not establish provider availability.

These journeys invoke every mounted method but do not establish exhaustive
behavioral coverage. The
[0.41.1 ownership and coverage audit](../../audits/working/blob-0411-adoption.md)
records the mounted gateway/account and receipt/history acceptance evidence under
[#622](https://github.com/dragginzgame/canic/issues/622). Export parity alone is
not complete behavioral coverage.
The first follow-through adds admission/manifest, content/headroom, history and
receipt checks in both compositions, including original observations after
restoration. Local status and empty-journal funding inspection also cover operator
authority, exact scope, explicit blockers and restored observations. Controlled
provider replies now cover gateway refresh/revocation/cancellation and account
balance inspection, including malformed data, lost replies and restored fences.
Oversized account and gateway replies produce typed refusals; account state stays
unchanged and the pending gateway sequence remains available for exact cancellation.
Payment-relationship inspection also covers exact service/payer bindings, signed
figures and wide counters, absent relationships, structured provider errors and
malformed/oversized responses without changing local funding state.
Populated tenant upload history crosses both result and scan bounds, preserves
ordered complete continuation, and rejects tenant/filter cursor reuse without
changing stable memory.
Operator service-wide pagination also matches the union of both independently
authorized tenant histories, with caller and cursor scope checks across pages.
The three GC callbacks also have mounted checks for liveness, gateway authority,
malformed roots, atomic batch refusal, idempotent acknowledgment and restore fences.
Malformed and over-limit acknowledgment batches preserve eligible deletion entries;
a maximum-size valid repeated-root batch succeeds. Acknowledgment preserves
physical and liability accounting.
The embedded fixture also inspects two original refunded funding records through
history and exact-outcome queries, including cursor boundaries, altered-intent
conflicts, caller isolation and stable-memory preservation across restore fences.
This does not qualify accepted or credited outcomes.
All current methods have direct calls. Published Testkit 0.64.1 additionally
qualifies cancellation/revocation before held gateway callbacks in both shapes:
released stale replies return Conflict without replacing committed membership.
Paid-funding acceptance is separate under #494.

End-to-end production readiness still needs upstream/provider evidence:
[serving](https://github.com/dragginzgame/ic-blob-storage/issues/5),
[reference/deletion](https://github.com/dragginzgame/ic-blob-storage/issues/6),
[multi-user uploads](https://github.com/dragginzgame/ic-blob-storage/issues/31) and
[GC callbacks](https://github.com/dragginzgame/ic-blob-storage/issues/32).
No native macOS, installed-IC, real-provider deletion or billing-cessation evidence
is claimed by these Linux PocketIC tests. See the current handoff for exact logs.

## Dependency version ownership

The service requirement has one source:
[`Cargo.toml`](../../../Cargo.toml),
under `[workspace.dependencies]`. Its package inherits that declaration with
`ic-blob-storage.workspace = true`; passive wire contracts come directly from
`ic-blob-storage-contracts` on the same release line. The two consumer examples depend on the
adapter and resolve those requirements through their own lockfiles.

Blob owns the method-bound Wire installation decoder and its byte ceiling.
An embedded application owns its outer configuration envelope and decoder limits;
that application envelope is not a second Blob installation protocol.

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
It mounts endpoints. Add the seventeen permanent requests returned by
`canic_blob_service::memory_requests()` to the host composition. The
artifact owns one pool with the service namespace and every other application's
namespace explicitly granted, for example:

```rust
canic::memory::memory_allocation_pool!(
    provider = application_memory_pool,
    authorities = [
        ("embedded-app", "embedded_app."),
        (canic_blob_service::MEMORY_AUTHORITY, canic_blob_service::MEMORY_KEY_PREFIX),
    ],
    exclusions = [],
);
canic_blob_service::mount!();

fn application_memory() -> canic::memory::MemoryBootstrap {
    let mut input = canic::memory::framework();
    input.pool = application_memory_pool().expect("host grants");
    input.requests.extend(canic_blob_service::memory_requests().expect("Blob requests"));
    // Extend with the application's complete request factory too.
    input
}
// Pass memory = application_memory(), first in canic::start!(...).
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
owning canister alongside application endpoints, and avoid duplicate method names.
Blob-owned methods use method-bound IC Wire frames, and the external
provider certificate retains Candid. Canic-owned framework endpoints use native
Wire; lifecycle arguments retain their declared installation encoding.

The complete [embedded example](../../../integrations/blob-service/embedded-consumer/src/lib.rs)
has its own persistent counter, endpoints, combined installation input and metrics
sampler. Its counter owns `embedded_app.counter.v1`; application and service
keys receive physical memories from the same checked allocation pool. It has a
separate workspace and can be built explicitly with:

```sh
cd integrations/blob-service/embedded-consumer
../../../target/cargo/debug/canic build embedded-app --workspace . --config canic.toml --icp-root . --profile fast --json
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

The shell depends on `canic-blob-service`, `canic` and `ic-platform`.
The adapter inherits Canic's package version and registry requirement from the
main workspace, with the published Blob selection from the root catalog. Cargo's package manifest
removes the development Canic path and excludes these private consumer fixtures.
Canic and `canic-blob-service` are published together at 0.111.0, including the
selected Memory runtime. Consumers can select both from crates.io; the coherent
family is tracked under [Canic #33](https://github.com/dragginzgame/canic/issues/33).
A compilable package archive cannot substitute for exact managed runtime admission.
Checked-in qualification uses the matching local facade. A disposable package
candidate can instead select registry Canic and the extracted normalized adapter,
without a sibling checkout.
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
../../../target/cargo/debug/canic build consumer-app --workspace . --config canic.toml --icp-root . --profile fast --json
```

The complete App build above stamps the release-build identity required by
managed installation. A single-role build is compile-only evidence and does not
supply that identity. Use the emitted release manifest and its qualified artifact
union for installation proof.

Installation requires `canic_blob_service::dto::configuration::ServiceInstallationInput`
as a Wire request from `ic_blob_storage_contracts::wire::encode_request("blob_install", input)`, nested
after Canic's protected native Wire init payload. Its configured
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
The controller submits `canic_root_bind_component_initialization` containing
that operation ID, exact target and 1–16,384 opaque application bytes. Host's
`component_initialization::encode_command` encodes the exact native Wire frame;
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

The registered production Root case installs the dedicated service from a
complete managed App build. It verifies controller and target refusals, a
same-release Root restart before binding, lost-response reconciliation,
conflicting rebinding and replay after installation, including the service's
submitted operator, payer and limits. This is separate from the dedicated and
embedded composition fixtures and from browser/provider or downstream application
acceptance.

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
The owning shell compiles its exact topology. Raw Blob endpoints carry retained
method declarations; provider certificate and GC callbacks join Canic's Candid declaration
scope. Artifact admission compares both declarations with callable Wasm exports.

## Browser and per-project adoption

The enrolled tenant grants an exact per-upload permission naming the browser
uploader and exclusive deadline. The certificate endpoint authenticates that
actual uploader against the retained permission; the installation no longer
selects one trusted uploader. Application sessions and capabilities still belong
to the consumer. The adapter delegates this contract to
[Blob #31](https://github.com/dragginzgame/ic-blob-storage/issues/31); it does not
relay an ingress response certificate to another user.

The adapter exports `_immutableObjectStorageBlobsAreLive`,
`_immutableObjectStorageBlobsToDelete` or
`_immutableObjectStorageConfirmBlobDeletion` through the shared catalog and GC
decoders. Liveness conservatively protects unknown roots; deletion listing and
acknowledgment require current gateway membership and active owners. Acknowledgment
is retained through same-release restoration but does not settle physical bytes
or billing liabilities. [Blob #32](https://github.com/dragginzgame/ic-blob-storage/issues/32)
owns the provider contract. Certificate issuance records trusted retention intent,
not completion evidence. Applications decide when to publish an asset and must
handle unavailable provider bytes. Verified download remains a separate shared
Blob operation; no operator-controlled attestation or verifier deployment is
required by the maintained contract.

Production post-allocation binding currently requires a Root controller. A
shared Blob Component can use this existing controller route;
its Component Spec must require application initialization and its application
producer must retain the target-bound command bytes. The
existing parent/peer provisioning APIs do not authorize an application hub to
bind target-dependent bytes. [Canic #493](https://github.com/dragginzgame/canic/issues/493)
tracks that future authority path. The selected 0.112 shared Blob deployment
uses the controller route and does not require dynamic parent provisioning.
A configured project `payment_account` also does not
establish provider linkage or attributable external top-up credit. The mounted
funding views remain passive; the opt-in application Rust API described below
uses the installed shared journal to compose guarded payments. Blob distinguishes matching
successful provider acknowledgment from independently confirmed credit; neither
refunds spent attachments. Controlled acknowledged-payment and uncertain-outcome
qualification passes. The Canic adapter exposes no independent-credit
reconciliation API; live-provider payer linkage remains unqualified
([Blob #34](https://github.com/dragginzgame/ic-blob-storage/issues/34),
[Canic #494](https://github.com/dragginzgame/canic/issues/494)).

## Opt-in application funding

`canic_blob_service::funding` connects the published Blob funding workflows to
this adapter's installed journal. It mounts no endpoint and starts no timer.
Applications use upstream `ic-blob-storage` types directly and decide whether to
expose an authenticated update or trigger a payment from their own workflow.
The actual IC caller must be the installed Blob operator; controller status or
an application-supplied principal does not substitute for that caller.

- `prepare` authenticates the original scope before invoking the application's
  synchronous evidence callback, then rechecks shared preparation policy before
  reserving the exact intent. Preparation alone does not authorize dispatch.
- `dispatch` obtains fresh application evidence on first polling and delegates
  marking, actual platform liquidity checks, one call and original callback
  settlement to Blob. The installed journal borrow is released before awaits.
- `renew` delegates operator-authorized local budget increases within the
  installed ceiling, original accepted amount and retained history constraints.
  It sends no cycles, releases no restore fence and creates no new lifetime slots.

Evidence callbacks are trusted application Rust, never deserialized ingress
assertions. The application must establish provider/payer linkage, exact scope,
current account-wide activity, recovery and complete other liabilities. Local
journal clearance and a balance alone cannot establish those facts. Leave
unknown fields as `None` and provider qualification false; the shared engine
returns blockers without authorizing payment. Never default missing liabilities
to zero or reuse a pre-await observation as current spending authority.
Application-owned evidence acquisition and economic policy remain outside Canic.
Independent-credit reconciliation is unavailable through this adapter. The
provider-acknowledged success profile does not require it. Uncertain accepted
payments retain their original obligations and block renewal and subsequent
payment; neither a balance delta nor a fabricated receipt can clear them.

Every journal access refreshes the installed platform-version fence, including
callback settlement. Stable traps propagate. After a lost reply inspect the
original operation using the mounted funding views; do not issue a replacement
or resend an attempted intent. Same-release restoration retains history and
requires the existing authenticated continuity recovery.

The framework's private embedded fixture qualifies caller/scope refusal, missing
evidence, two known-refunded dispatches, attempted-identity replay refusal,
renewal refusal without acceptance, uncollected ingress replies and
same-release journal restoration. It is an isolated synthetic account with no
external competing activity, not a production evidence source. Published Testkit
0.65 supplies bounded cycle acceptance. The installed embedded case now qualifies
two acknowledged payments (100 offered, 75 accepted and 25 refunded each),
amount-bounded renewal and immutable grant replay before and after restoration.
Acknowledgment leaves the accepted amount uncredited; it is not an independent
receipt. A third partially accepted malformed reply remains unresolved, blocks
renewal and subsequent payment, and retains its facts across restoration.
Missing independent evidence changes nothing. A stale read-only continuity proof
is reacquired on typed `Binding` refusal at most twice; other refusals fail.

A separate fresh installation holds a provider response after bounded acceptance.
Two concurrent ingresses cannot repeat the intent or reserve a subsequent payment;
the exact pending provider request and journal summary remain unchanged. Releasing
a structured account-overflow error retains accepted/refunded totals separately
from response classification. The original ingress reply remains uncollected;
mounted status recovers settlement and restoration retains uncertainty. This is
separate from settlement-callback failure.

A third fresh installation deliberately traps after shared settlement writes in
the real IC callback. PocketIC reports `CanisterCalledTrap`; callback writes roll
back while the provider's earlier acceptance remains committed. The journal keeps
the entire original offer uncertain, leaves recorded acceptance/refund totals
unchanged, and activates its recovery fence. Authenticated same-release recovery
clears that fence without clearing the reservation or permitting either payment.
This qualifies callback rollback, not every possible platform outage.

The installed liquidity profile offers the full 9,900-cycle journal allowance.
It refuses when operating reserve consumes observed liquidity, when other
liabilities are `u128::MAX`, and when the apparent room covers the attachment but
not platform call costs plus final journal-write charges. Each refusal records
`NotDispatched` with a positive platform call-cost bound. The three original
attempts restore the full allocation, preserve the configured reserve, and remain
non-repeatable after restoration. This does not infer actual provider spendability
or independently measure every replica callback-cost component.

The installed budget profile reaches the cumulative renewal ceiling, then makes
two additional acknowledged payments within remaining allocation. A one-cycle
increase returns `RenewalCeiling`; exhausted lifetime history returns `JournalFull`.
Exact older grant replay remains effect-free and cannot replenish lifetime slots.
Mounted accounting and both refusals survive restoration/recovery.

Mounted per-operation outcomes are qualified separately from aggregate totals.
Successful callbacks return `ProviderAcknowledged(75)` with exact reported balance
and renewal amount. Accepted malformed/provider-error replies return
`CreditRequired(75)` with their distinct response classes; callback traps return
`TransferUnknown(100)` with no retained response. Original requests and fence
states remain exact through recovery. Repeated reads, caller refusals and changed
offer conflicts do not change stable memory.

Any future independent-credit integration needs authenticated evidence from the
application/provider owner and installed confirmation/replay qualification
(Blob #34). IC Blob Storage retains ownership of that contract and journal;
Canic does not expose an unqualified wrapper. Real-provider payer linkage and
deployment acceptance remain separate under
[Canic #494](https://github.com/dragginzgame/canic/issues/494).

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
| `blob.physical_bytes` | Registered physical bytes plus active reservations |
| `blob.liability_bytes` | Unsettled confirmed bytes plus active reservations; not a currency amount |
| `blob.reserved_bytes` | Active reservations, already included once in each byte total above |
| `blob.active_reservations` | Active or possibly exposed operations, including zero-byte uploads |
| `blob.operation_slots` | Lifetime retained operation slots, including cancelled and settled entries |
| `blob.fenced` | Last observed upstream mutation fence: 1 fenced, 0 unfenced; not a provider-readiness proof |

Do not sum the three byte totals or add reservations to them. Logical reference
release does not establish physical deletion or extinguish provider liabilities.
These are source accounting units, not measured stable-memory size or a bill.

Deploy the adapter-backed shell as an ordinary managed Component, configure
the upstream service and use its client APIs for blob operations. Then collect
the selected Fleet with the existing command:

```sh
canic --environment staging observatory snapshot shared-storage --out usage.json
```

Each private `roles[]` entry carries the storage canister ID and
`application_metrics`; numeric values are exact decimal strings. Public
JSON/HTML reports show these metrics under their role instance, including source
state, time and truncation. See [Fleet Observatory](../operations/fleet-observatory.md#application-metrics).
The collector reads the generic typed Wire `canic_public_metrics` cache and has no blob
dependency. It does not enumerate service canisters outside the retained Fleet
inventory, enable metrics remotely, or make paid calls.

This reports usage **per storage canister**. Usage attributed to a consuming
canister or tenant remains a separate upstream-authorized disclosure; the
adapter does not publish that breakdown or infer it from capacity headroom.
No application deployment, provider qualification or paid upload is implied by this
reporting integration.

Reinstalling or deleting a live service is a separate operation. Preserve records
of outstanding provider liabilities; logical reference release does not prove
physical deletion or billing cessation. The source extraction performs no paid
upload, provider cleanup, deployment, or reset.
