# Idea: Standalone Blob Service Extraction

Date: 2026-08-06

## Status

- Classification: deferred, unnumbered idea. Its former working number was
  `0.109`; neither the external service nor the Canic extraction is scheduled.
- Former review status: accepted for M0 investigation; full implementation
  remained proposed before deferral.
- Release boundary: reinstall only. Canic 0.109 does not migrate, import,
  adopt or decode blob state written by Canic 0.69 through 0.108.
- Implementation approval: M0 evidence gathering and build-topology proof
  only. M1 through M7 and the Canic hard cut remain unapproved until M0 exits
  successfully.
- Sequence: this design follows the
  [0.100 Fleet Subnet Root design](../../archive/0.100-multi-subnet-fleet-coordinator-and-registry-synchronization/0.100-design.md)
  and the
  [0.101 Component Group design](../../archive/0.101-fleet-authoritative-service-provisioning-and-publication/0.101-design.md).
- Existing evidence: the archived 0.69, 0.70 and 0.71 designs and the current
  Caffeine/Cashier protocol inventories describe the behavior that must be
  re-inventoried before extraction. They are evidence, not authority for the
  new standalone API.
- Package names: `blob-protocol`, `blob-client`, `blob-service` and
  `blob-service-canister` are logical names in this design. A publishing prefix
  may be added if an unprefixed package name is unavailable; that does not
  change ownership.
- Conditional integration package: if M0 proves it necessary,
  `blob-service-canic` lives in the standalone repository, depends on Canic and
  is not part of Canic's dependency graph.
- Version ownership: `0.109` is the Canic integration/removal release, not the
  version of the standalone packages. The standalone project versions and
  releases independently.
- Repository boundary: the implementation belongs in a standalone
  project. Canic 0.109 owns the Canic hard cut and the ordinary Component
  integration contract. Automated Canic work must not mutate the external
  project.

No current Canic blob API, state or operator command survives by default or
through an alias. Optional Canic API management is supplied from the
standalone repository through generic Canic facilities.

## Summary

0.109 removes blob-storage product semantics from Canic and establishes a
standalone, multi-tenant blob service that can be consumed by applications
which do not use Canic.

Canic remains responsible for Fleet topology, placement, artifact admission,
Canister lifecycle, cycles funding and Component authority. It may deploy the
blob service as an ordinary application Component, but it does not understand
blob digests, media types, references, tenants, gateways, retention, quotas or
deletion.

The standalone project owns three reusable core crates:

| Crate | Sole responsibility |
| --- | --- |
| `blob-protocol` | Backend-neutral bounded identities, references, requests, responses and typed errors. |
| `blob-client` | Checkpointable hashing, manifest, upload/download and byte-verification operations for native and Canister consumers. |
| `blob-service` | Tenant authority, durable object/reference state, quotas, Caffeine integration, retention, garbage collection and operational status. |

`blob-protocol` is portable boundary data. `blob-client` is cross-runtime
client logic. `blob-service` is deliberately IC-specific service logic with
stable state and Caffeine callbacks; it is standalone, not portable in the
same sense as the protocol crate. A fourth required package,
`blob-service-canister`, is the tiny standalone Wasm shell and contains no
service policy.

Caffeine remains the sole byte backend for the first implementation. The
service keeps its Caffeine adapter private. It does not publish a general
backend plugin API and does not build a second chunk store.

The primary application flow is:

~~~text
Toko Feed
  -> fetch and validate provider image bytes
  -> blob-client computes raw content identity and a Caffeine manifest
  -> blob-client uploads bounded pages and persists progress
  -> blob-client verifies the completed bytes
  -> blob-service retains one tenant-owned logical reference
  -> IcyDB stores the service-qualified retained reference
  -> Toko stores source and attribution as application metadata
~~~

Backend location, gateway selection, temporary URLs, Cashier identity and
serving policy never enter the durable `BlobRef`.

## Physical Workspace and Build Targets

The standalone repository uses this physical Cargo layout:

~~~text
standalone-blob/
  Cargo.toml
  crates/
    blob-protocol/
    blob-client/
    blob-service/
  canisters/
    blob-service-canister/
  integrations/
    blob-service-canic/          # created only if M0 proves it is needed
  internal/
    blob-caffeine-internal/      # created only if M0 proves shared code is needed
~~~

The root workspace contains the three core libraries and the standalone
Canister package. Conditional packages enter the workspace only after their M0
exit condition is recorded.

| Cargo package | Requirement | Artifact and ownership |
| --- | --- | --- |
| `blob-protocol` | required | publishable `rlib`; sole blob application/operator wire/Candid authority; no Wasm exports |
| `blob-client` | required | publishable `rlib`; local hashing, transfer and checkpoint logic; no Wasm exports |
| `blob-service` | required | publishable `rlib`; authoritative service state, policy and handlers; no Wasm exports |
| `blob-service-canister` | required | standalone `cdylib` Wasm shell; exports the canonical blob API and delegates immediately |
| `blob-service-canic` | conditional | external `rlib` integration; exists only if generic Canic composition is required |
| `blob-caffeine-internal` | conditional, unpublished | one private provider source of truth if M0 proves both client and service must share it |
| new Canic package | forbidden | Canic removes its existing blob subsystem and adds no blob package |
| new Toko/IcyDB package | not required | consumers depend on the standalone packages through their own existing targets |

`blob-service` is library-only. It has no `cdylib` target, no IC endpoint
attributes and no feature or macro that emits endpoints. Therefore depending
on it can never export a Canister method through Cargo feature unification.

`blob-service-canister` is the sole standalone exporter. It depends on
`blob-protocol` and `blob-service`, defines the minimal `init`, query, update,
timer and source-required provider-callback adapters, and immediately
delegates to service handlers. It is an artifact package, not a reusable
library dependency, and `blob-service-canic` must not depend on it.

If M0 requires Canic composition, `blob-service-canic` remains an `rlib` and
exports nothing merely by being linked. It provides one external
`blob_service_canic::export_service!` macro which the consuming application's
final `cdylib` invokes exactly once. That expansion emits the canonical blob
API, source-required provider callbacks and generic Canic Component lifecycle
endpoints, then delegates to the same `blob-service` handlers. The consuming
application owns the final Wasm, stable-memory selection and the single macro
invocation.

No core package has an endpoint-emission Cargo feature. The standalone shell
and the consuming Canic application are mutually exclusive final targets, and
the dependency graph must reject any final target which links the standalone
shell or expands more than one blob endpoint inventory. Feature unification
therefore cannot create duplicate IC exports.

M0 may choose not to create `blob-service-canic` when the standalone Wasm can
participate directly in ordinary Component lifecycle. M0 may choose not to
create `blob-caffeine-internal` when one existing core package can be the sole
owner of all shared provider definitions. Neither conditional package is a
reason to duplicate source-of-truth types.

## Non-Negotiable Invariants

### 1. Complete hard cut

The existing Canic blob subsystem is removed completely. Its implementation,
state, stable-memory ownership, DTOs, API helpers, feature names, endpoint
macros, CLI, Medic behavior and compatibility surface do not become the
standalone service and do not remain as a fallback.

The extracted service is a new authority boundary. It does not read old Canic
state, delegate back into `canic-core` or preserve the old API under deprecated
names.

### 2. Optional Canic-managed API without Canic blob coupling

Applications which use Canic may still ask Canic to manage the service's API
and lifecycle envelope. That integration lives in `blob-service-canic` in the
standalone repository and depends on Canic. Canic does not depend on it and
contains no blob feature, blob API module or blob-named endpoint macro.

`blob-service-canic` may use ordinary Canic endpoint-marshalling, lifecycle,
metrics and Candid facilities. It delegates every blob operation to the same
`blob-service` handler used by the standalone Canister target and defines no
second blob state or policy authority.

M0 must first prove whether ordinary Component machinery can install and
operate the standalone Wasm directly. If the maintained Component lifecycle
requires an envelope, `blob-service-canic` supplies it. If even that cannot be
deployed through generic Component machinery, M0 records the exact missing
generic capability and 0.109 pauses for a separate backend-neutral Canic
design. A blob-specific feature inside Canic is not the fallback.

Both endpoint envelopes construct the same service-owned request context from
the IC caller and time. The context type belongs to `blob-service`; it contains
no Fleet, root, Component, Directory or Canic token field.

## Decision

Blob storage is an application service, not Canic infrastructure.

The only built-in Fleet infrastructure roles remain:

1. Fleet Coordinator;
2. Fleet Subnet Root; and
3. Wasm Store.

`BlobService`, `BlobStore`, `Caffeine`, `Cashier` or an equivalent role must
not become a fourth infrastructure artifact, Registry authority or root-owned
subsystem.

When Canic deploys a blob service:

- checked-in application configuration declares an ordinary Component Spec;
- ordinary topology and release-set compilation admit its exact artifact;
- an ordinary Fleet Subnet Root creates, installs, funds, registers, drains
  and removes it;
- application configuration supplies opaque service init arguments;
- the service owns all blob-specific state and policy; and
- clients call the service directly through `blob-protocol` or `blob-client`.

Canic treats the service as a business-level black box. The optional
`blob-service-canic` package may compose the maintained Component lifecycle
and endpoint envelope with the standalone handlers. The dependency points
toward Canic from the integration package; no blob dependency enters a
published or production Canic graph. The closeout fixture exception is
test-only and carries no runtime authority.

## Goals

0.109 must:

1. define one Canic-independent blob protocol usable by Canic and non-Canic
   consumers;
2. preserve a raw portable content identity separately from Caffeine's backend
   object identity;
3. separate shared physical objects from tenant-owned logical references;
4. prevent one tenant from releasing or deleting another tenant's object;
5. retain Caffeine as the only first-version byte backend;
6. move gateway authorization, Cashier integration, quotas, retention and
   deletion authority into `blob-service`;
7. let Canic deploy the service only through ordinary Component machinery;
8. remove all existing blob-specific runtime, stable-memory, feature, macro,
   CLI and Medic surfaces from Canic;
9. prove one standalone consumer which has no Canic dependency;
10. prove one Canic-managed deployment directly or, only when M0 requires it,
    through `blob-service-canic`, without adding a blob feature or blob
    authority to Canic;
11. freeze one unambiguous library, standalone-Wasm and optional-composed-Wasm
    Cargo topology with no duplicate endpoint exports;
12. freeze one canonical blob Candid authority, one externally sourced
    provider-callback contract and one bounded explicit service
    initialization/operator contract; and
13. keep standalone-project, Canic and downstream-consumer releases
    independently owned.

## Non-Goals

0.109 does not:

- design a universal object-storage platform;
- add a public backend plugin framework;
- build a replacement for Caffeine;
- support more than one byte backend in the first version;
- make Canic a data plane, upload proxy, download proxy or metadata database;
- place provider URLs, gateway URLs or temporary credentials in `BlobRef`;
- add private/confidential blob encryption;
- infer tenant membership from Canic controller status;
- use a Fleet, Component Spec, Component instance or Canic role as the
  portable tenant identity;
- add an implicit global public-write surface;
- add cross-release migration, mixed Canic versions or existing-state
  adoption;
- preserve the current `canic blob-storage` command as an alias;
- preserve or replace the current Canic blob feature flags or endpoint macros;
- add a new blob-specific crate to the Canic workspace;
- require Canic 0.109 to wait for Toko/IcyDB production deployment;
- add cross-release Component upgrades; or
- require a non-Canic consumer to compile, configure or understand Canic.

The first version is for immutable public-by-reference assets such as images.
Tenancy protects mutation, reference ownership, accounting and metadata
inspection. It does not claim confidentiality when the selected Caffeine
serving path permits anyone holding a digest or URL to fetch the bytes.

## Architectural Boundary

~~~text
                         application plane

  Toko Feed -----------+                         non-Canic consumer
                       |                                |
                       v                                v
                 blob-client                     blob-protocol
                       |                                |
                       +---------------+----------------+
                                       |
                                       v
                                 blob-service
                         tenant/reference authority
                           retention, quota and GC
                                       |
                              private adapter only
                                       |
                                       v
                              Caffeine + Cashier
                              physical byte plane

                         deployment plane

  Canic host -> Fleet Subnet Root -> ordinary Component lifecycle envelope
                                         |
                                         v
                           blob-service-canister Wasm

  optional standalone-repository composition:
  consuming app Wasm
    -> blob-service-canic -> generic Canic lifecycle APIs -> same handlers
~~~

The deployment plane can replace, stop or remove the Canister under ordinary
Component authority. It cannot mutate blob records through a Canic-specific
shortcut. The application plane cannot install Wasm, change controllers or
claim Canic Registry membership.

## Authority Map

| Concern | Sole authority |
| --- | --- |
| Fleet topology, placement and Component admission | Canic application topology and Fleet authorities |
| Qualified blob-service application artifact | ordinary Canic application release set when Canic deploys it |
| Blob-service Canister lifecycle | ordinary Fleet Subnet Root, or the non-Canic deployment owner |
| Blob object and logical-reference state | exact blob-service Canister |
| Tenant identity, actors and quotas | blob-service tenant registry |
| Application record ownership and attribution | consuming application, such as IcyDB/Toko |
| Physical byte upload, retrieval and deletion | Caffeine under the service's backend contract |
| Gateway caller authority | blob-service backend-authority state sourced from the exact Caffeine/Cashier contract |
| Cashier balance and top-up result | Cashier, observed and acted on by blob-service |
| Temporary serving route | blob-service backend resolver |
| Byte integrity at the consumer boundary | `blob-client` verification against `BlobRef` |
| Optional Canic endpoint/lifecycle envelope | standalone `blob-service-canic` integration package |

Neither a Canic Directory nor a blob-service cache may become a second tenant,
reference or object authority. A successful gateway or Cashier response is not
durable service commitment until the service commits the matching state or
receipt.

## Standalone Crate Boundaries

### `blob-protocol`

`blob-protocol` is a passive boundary crate. It owns:

- `BlobDigest` and its canonical binary/text forms;
- `MediaType`, `BlobRef` and `RetainedBlobRef`;
- `BlobServiceId`, `BlobReferenceId` and `BlobTenantId`;
- actor-epoch and monotonic-sequence operation identities;
- `ServingPolicy` plus bounded service manifest, upload, retain, release,
  resolve, session-status and operator DTOs;
- `BlobServiceInitV1` and its bounded install-only authority bindings;
- the sole canonical application/operator `blob-service.did`, method names,
  method modes and version-1 wire contract; and
- protocol error codes.

It must not contain:

- Canic types, features, macros or stable-memory IDs;
- IC CDK endpoint macros;
- stable storage;
- operational Caffeine/Cashier DTOs, ICFS object identities or client
  continuations beyond the two explicit install-only authority bindings;
- gateway URLs;
- async workflows;
- tenant-policy decisions; or
- application-specific source, attribution or record types.

The crate must compile for ordinary Rust and Wasm consumers. Its wire types
must be bounded before allocation and must reject unknown or malformed
identity encodings rather than normalize arbitrary input.

### `blob-client`

`blob-client` depends on `blob-protocol` and owns:

- incremental raw content hashing;
- the private Caffeine/ICFS tree-hash adapter;
- deterministic chunk boundaries;
- chunk, metadata and tree-root hashing;
- bounded upload preparation, checkpointing, inspection and transfer
  sequencing;
- local `ManifestPreparationCheckpoint` and `UploadCheckpoint` types;
- retrieval;
- exact size and digest verification; and
- small transport adapters for native and Canister consumers where required.

It must not own tenant grants, quotas, retention, garbage collection, billing
or service state. A transport success alone must not be returned as a durable
logical reference; the client returns a retained reference only after the
service's terminal commit response.

`blob-client` may hide Caffeine-specific upload mechanics in its first
implementation. Caffeine-specific request types must not escape through the
portable application API. Its local checkpoints are application-persisted
client state, not Candid DTOs, and never enter `blob-protocol` or a service
request. Any backend continuation inside them remains an opaque client-owned
value.

### `blob-service`

`blob-service` depends on `blob-protocol` and owns:

- implementations of every canonical protocol method;
- service-local stable state and invariants;
- tenant creation, actor grants and quota policy;
- upload intents and terminal receipts;
- object and reference state machines;
- bounded reference listing and status;
- retention and garbage collection;
- Caffeine gateway compatibility endpoints;
- gateway-principal synchronization;
- Cashier observation and explicit funding workflows; and
- private backend adaptation.

It must not depend on Canic configuration, runtime macros, role attestations,
Directories, Registries or stable-memory allocation constants.

`blob-service` is the library-only handler and authority package described by
the physical workspace contract. `blob-service-canister` implements its
standalone Wasm shell. The optional `blob-service-canic` integration package
and consuming application binary compose the Canic-managed target; neither is
a fourth core blob crate. Dependency direction is always
`blob-service-canic -> Canic + blob protocol/service`, never
`Canic -> blob-service`.

Neither Wasm target owns or hand-writes another Candid contract. The
standalone shell's generated application/operator projection must equal
`blob-service.did`. Any required provider-callback projection must separately
equal the frozen source-owned Caffeine contract. The composed target's
canonical blob and provider projections must equal those same authorities;
its only additional methods are the non-overlapping generic Canic Component
lifecycle surface.

## Caffeine Adapter Ownership

The client and service have non-overlapping provider responsibilities.

`blob-client` owns only the byte-plane adapter:

- deterministic ICFS chunking and hashing;
- bounded construction of upload pages from caller-owned bytes;
- execution of transient upload/download instructions returned by the
  service;
- local progress checkpoints; and
- raw-size and digest verification after retrieval.

It owns no Caffeine account, gateway, liveness, deletion, Cashier, quota or
service-authority decision.

`blob-service` owns only the authority/control-plane adapter:

- exact provider request/response validation at service boundaries;
- upload authorization and completion evidence;
- the `BlobRef` to private backend-object mapping;
- project/account, gateway and deletion-namespace authority;
- liveness, retention and deletion publication; and
- Cashier observation and funding journals.

It does not fetch application source bytes or persist a caller's local client
checkpoint.

M0 must inventory every Caffeine/ICFS/Cashier type, constant, hash primitive
and method exactly once. If both packages genuinely need the same private
provider DTO or hashing primitive, M0 creates the unpublished
`blob-caffeine-internal` package and makes both depend on it. Otherwise the
single owning core package exposes the smallest crate-private or
workspace-visible helper needed by the other. Copying provider definitions
between `blob-client` and `blob-service` is forbidden. The internal package, if
required, is a source-sharing mechanism rather than a public backend
abstraction or fourth core product API.

## Content and Backend Identities

The existing Canic `BlobRootHash` is a Caffeine/ICFS tree root whose hash also
binds transport metadata. It is not a raw file digest and therefore cannot be
the backend-independent content identity.

0.109 keeps the identities separate:

~~~rust
pub struct BlobDigest {
    pub algorithm: BlobDigestAlgorithm,
    pub bytes: [u8; 32],
}

pub enum BlobDigestAlgorithm {
    Sha256,
}

pub struct BlobRef {
    pub digest: BlobDigest,
    pub size_bytes: u64,
    pub media_type: MediaType,
}
~~~

`BlobDigest::Sha256` is SHA-256 over the exact unchunked object bytes. Binary
Candid uses exactly 32 digest bytes. Text and JSON use canonical lowercase
`sha256:<64-hex>`. That prefix is accurate for the raw content digest.

The Caffeine adapter separately owns a private `IcfsTreeDigest` or equivalent
backend object ID. Its diagnostic rendering uses
`icfs-tree-sha256:<64-hex>` even where the external Caffeine protocol requires
the legacy `sha256:<64-hex>` spelling. Provider DTO conversion is the only
place that strips or substitutes that explicit diagnostic prefix.

The private ICFS identity freezes the source-proven:

- `icfs-chunk/` chunk domain separation;
- `icfs-metadata/` metadata domain separation;
- `ynode/` node domain separation;
- canonical `Content-Type` and `Content-Length` headers;
- deterministic chunking rules; and
- DSBMTWH tree construction.

The service stores an exact mapping from one canonical `BlobRef` to its
current backend object identity. A replacement backend may change that private
identity while preserving the public raw digest, size and media type.

Version-1 `MediaType` accepts only a parameter-free `type/subtype` value. It
lowercases ASCII type and subtype, rejects whitespace, parameters, wildcards,
control characters and malformed tokens, and is at most 127 bytes. The
service never guesses a missing media type.

Identical bytes always have the same raw digest. Semantically equivalent MIME
spellings normalize to one `MediaType`. Different media types produce distinct
`BlobRef` values and may require distinct ICFS backend objects because the
tree root binds `Content-Type`. Cross-tenant physical deduplication therefore
occurs only for an exact canonical `BlobRef`, not for raw digest alone.

Wrong digest length, prefix, case, whitespace, control characters and unknown
algorithms fail closed. The exact byte, chunk, header and tree construction is
locked by source-backed golden vectors before implementation.

## Service-Qualified Logical References

`BlobRef` deliberately contains no backend location, tenant, gateway, expiry
or mutable lifecycle field. A retained reference must additionally identify
the exact authority which owns it:

~~~rust
pub struct RetainedBlobRef {
    pub service: BlobServiceId,
    pub tenant_id: BlobTenantId,
    pub reference_id: BlobReferenceId,
    pub blob: BlobRef,
}
~~~

`BlobServiceId` is a strongly typed, non-anonymous IC Canister Principal. It
is an authority address, not a backend location. Every service endpoint
requires it to equal the live receiver before reading or mutating a reference.

`BlobReferenceId` is exactly 32 bytes and is allocated by the service before
the retain commit from the exact service, tenant and an exhaustion-checked
monotonic durable reference sequence under `blob/reference/v1` domain
separation. Its text boundary is exactly 64 lowercase hexadecimal characters.
The sequence high-water mark is permanent even after reference-record
reclamation, so an ID is never reused. The tuple
`(service, tenant_id, reference_id)` is the complete logical-reference
identity.

The serialized value is a locator, not self-authenticating authority. Resolve
and release load the authoritative reference record and require its exact
service, tenant, reference ID and `BlobRef` to match. A caller-presented
`RetainedBlobRef`, digest or knowledge of a public URL grants no mutation
permission.

Application databases store the complete `RetainedBlobRef`. The reference ID
is needed to release one exact logical reference. It is not a byte-serving
credential and never appears in a public asset URL.

One exact canonical `BlobRef` may identify one shared backend object while
many logical references exist:

~~~text
service S / backend object D
  <- tenant A / reference 1
  <- tenant A / reference 2
  <- tenant B / reference 3
~~~

Releasing reference 1 affects neither reference 2 nor reference 3. Physical
deletion becomes eligible only after every active reference to D under service
S has been released and all retention and in-flight-operation fences have
cleared.

## Initialization and Service-Operator Authority

`blob-protocol` owns one bounded install contract because both standalone and
Canic-composed targets must initialize identical service authority:

~~~rust
pub struct BlobServiceInitV1 {
    pub schema_version: u16,
    pub stable_layout_version: u16,
    pub initial_operators: Vec<Principal>,
    pub global_limits: BlobServiceGlobalLimitsV1,
    pub operation_retry_horizon_ns: u64,
    pub reference_reconciliation_horizon_ns: u64,
    pub object_retention_ns: u64,
    pub serving_policy: PublicServingPolicyV1,
    pub caffeine: CaffeineAuthorityBindingV1,
    pub cashier: CashierAuthorityBindingV1,
}
~~~

The concrete wire types use bounded collections rather than the illustrative
unbounded `Vec`. Both version fields must be exactly 1. Unknown fields, unknown
enum tags, zero durations, anonymous Principals, duplicate operators,
malformed provider identities and values above their frozen encoded-size or
numeric bounds reject before stable state or an external effect.

The initial operator set contains 1 through 16 unique non-anonymous
Principals. Every listed operator has the same explicit full service-operator
grant in version 1. The IC controller set, Canic Component membership, Fleet
roles and installer caller do not add an operator implicitly.

Initialization allocates one never-reused service-operator actor epoch per
initial Principal and commits operator revision 1 with the complete validated
init record before scheduling timers or invoking a provider. There is no
post-install defaulting or controller-derived repair path.

`BlobServiceGlobalLimitsV1` freezes finite maxima for every global counter,
encoded-byte ledger, tenant, actor, receipt, tombstone, upload and deletion
domain. Tenant ceilings may later move only within these immutable global
maxima. The two retry/reconciliation horizons, object-retention duration,
safe-serving policy, Caffeine authority binding, Cashier authority binding and
stable-layout version are immutable after installation in version 1.

`CaffeineAuthorityBindingV1` binds the exact canonical network, project or
account identity, deletion namespace, gateway authority source and accepted
provider-contract evidence. `CashierAuthorityBindingV1` binds the exact
canonical network, Cashier Canister, payer/account identity and accepted
contract evidence. They contain no backend object ID, upload continuation,
temporary URL or credential. These two install-only bindings are the only
provider-named DTOs in `blob-protocol`; operational Caffeine/Cashier DTOs and
ICFS identities remain private to the provider adapter.

Operator-set rotation is one durable service-scoped mutation using the
caller's next `BlobOperationId` and an exact expected operator revision. The
caller must belong to the current set. The replacement set is validated in
full and committed atomically, cannot be empty, must retain the rotation
caller, and cannot remove another operator with a nonterminal operator-scoped
operation. Replacing a sole operator therefore takes two explicit rotations:
the current operator first adds its successor, then the successor removes the
predecessor. Exact retry follows the common receipt horizon; conflicting
revision or payload rejects. Every other operator mutation, including
tenant-ceiling changes, Cashier funding and gateway-authority administration,
uses the same service-scoped monotonic operation and retry rules before its
first mutation or effect.

## Tenant Identity and Authorization

The portable service cannot use a Fleet or Component as its tenant identity,
because a non-Canic application has neither.

`BlobTenantId` is exactly 32 bytes, allocated by the service from its exact
Canister identity and a monotonic durable sequence under a domain-separated
version-1 derivation. It is never selected by an ordinary caller and is never
reused.

Each tenant has a protected binding containing:

- its immutable `BlobTenantId`;
- one or more exact administrator Principals;
- bounded actor grants;
- operator-managed quota ceilings;
- tenant-controlled reduction-only self-limits;
- creation operation and timestamp; and
- lifecycle state.

Version-1 actor grants are explicit:

| Grant | Permission |
| --- | --- |
| `TenantAdmin` | manage tenant actors and reduce tenant self-limits |
| `BlobWriter` | prepare/commit uploads and retain known live objects |
| `ReferenceManager` | release exact tenant-owned references |
| `BlobResolver` | inspect and resolve exact tenant-owned references |

The raw IC caller Principal is checked on every service endpoint before
mutation. A caller may hold several grants. Controller status alone grants no
tenant permission. Gateway principals are a separate backend authority and
never imply a tenant grant.

Actor addition/removal is an explicit tenant-administration operation with a
monotonic operation sequence and horizon-bounded exact-retry semantics.
Removing an actor does not release its tenant's references and is rejected
while that actor owns a nonterminal upload or mutation. Tenant deletion is a
separately fenced, bounded workflow and cannot proceed while active references
or operations remain.

An actor-set mutation must retain the caller's current `TenantAdmin` grant and
must leave at least one `TenantAdmin`. Replacing a sole tenant administrator is
the same two-step add-then-remove journey as service-operator replacement, so
the accepted caller remains authorized to recover an uncertain response.

A Canic-managed deployment may initialize tenant actors from opaque
application arguments or an application-owned adapter. Canic does not parse,
validate or infer the resulting tenant policy. Dynamic Component membership
does not automatically become tenant membership.

## Quotas and Accounting

Every configured limit is finite. At minimum, each tenant has ceilings for:

- maximum object bytes;
- maximum active references;
- maximum unique referenced objects;
- maximum logical referenced bytes;
- maximum concurrent upload intents;
- maximum retained operation receipts; and
- maximum retained released-reference tombstones.

The service operator owns each tenant's allocatable ceiling and may increase
or decrease it within the corresponding finite service-wide maximum. A tenant
ceiling is an admission bound, not a physical-capacity reservation, so tenant
ceilings may be oversubscribed in aggregate. Every accepted operation must
still fit both the tenant's effective ceiling and currently available global
capacity. A decrease cannot move a ceiling below current committed plus
reserved use.

The tenant may impose a lower self-limit and may only reduce that self-limit.
Only the service operator can raise or remove it, still within the operator
ceiling. This lets Toko grow deliberately without allowing a compromised
tenant actor to grant itself more capacity.

The service also has finite global ceilings for physical objects, physical
bytes, pending uploads, pending deletions, tenants, actors, retained receipts,
released-reference tombstones and stable encoded bytes.

Reservations occur atomically before an awaited backend effect. Exact retry
does not charge twice. Failure or terminal cancellation releases only the
reservation owned by that operation.

Version 1 charges tenant logical bytes once for each unique
`(BlobTenantId, BlobRef)` with at least one active reference, while the
reference-count ceiling counts every active logical reference. Global
physical bytes count one live backend representation for an exact `BlobRef`
once. Secondary indexes and counters must reconcile exactly with canonical
object and reference records.

Deduplication may reduce physical use. It never broadens authorization or
permits one tenant to observe another tenant's identities, counts, metadata or
retention policy.

## Operation Sequences and Retry Horizon

Version 1 does not accept arbitrary 32-byte operation IDs. Every service
operator or tenant-actor grant has one service-allocated, never-reused
`actor_epoch`, and the actor supplies a strictly monotonic sequence:

~~~rust
pub struct BlobOperationId {
    pub actor_epoch: u64,
    pub sequence: u64,
}
~~~

The complete operation identity is
`(service, authority_scope, caller, actor_epoch, sequence)`, where
`authority_scope` is either the service-operator scope or one exact tenant. A
newly accepted sequence must equal the actor epoch's durable high-water mark
plus one. The service commits that intent and advances the high-water mark
atomically before any await. Several accepted operations may then perform
independent effects concurrently within per-actor, tenant and global in-flight
bounds.

Every nonterminal operation remains durable. Terminal responses remain
exactly replayable until their terminal timestamp plus one immutable positive
service-wide retry horizon. A separate immutable positive released-reference
reconciliation horizon governs tombstones. Both durations are bounded
installation inputs and cannot be shortened. Admission reserves enough
receipt capacity to retain the response for the complete horizon; a full
receipt budget rejects new work instead of evicting live replay evidence.

After the horizon, the terminal receipt may be reclaimed. The actor epoch's
high-water mark remains permanently. A later request using any reclaimed or
older sequence returns typed `OperationExpired` and never performs the effect
again. Exact response replay is therefore explicitly horizon-bounded, while
replay prevention remains permanent.

Clients must persist pending operations and retry them before the published
horizon expires. `OperationExpired` is not success. The client reconciles the
operation's separately addressable domain object or session; it may allocate a
later sequence only when that observation proves the intended mutation did not
commit. If the outcome is no longer observable, the operation enters explicit
operator review rather than guessing or repeating an effect.

Removing and later re-adding a Principal allocates a new actor epoch. Removed
epoch high-water marks are never reused. Each tenant has a finite lifetime
actor-epoch ceiling and the service has a finite lifetime operator-epoch
ceiling; reaching either requires operator review rather than deleting replay
history. Sequence advancement uses checked `u64` arithmetic and never wraps;
an exhausted epoch admits no new operation.

## State Machines

### Physical object

~~~text
Absent
  -> UploadPrepared
  -> UploadInFlight
  -> Verifying
  -> Live
  -> RetentionPending
  -> DeletionPublished
  -> Deleted
~~~

Rules:

1. `UploadPrepared` is durable before any upload authorization or paid effect.
2. Only the exact originating caller and actor epoch may resume an upload, and
   the exact service, tenant, `BlobRef`, operation, manifest and backend
   context must still match.
3. `Live` requires the exact source-backed Caffeine completion evidence and
   client/service verification contract frozen by the M0 inventory.
4. A caller assertion or HTTP success alone is not authoritative completion.
5. References may commit only against `Live`.
6. Zero references moves an object to `RetentionPending`, not immediately to
   physical deletion.
7. A new retain during `RetentionPending` may cancel deletion only before the
   deletion has been published to the backend.
8. `DeletionPublished` rejects new references. A later use requires terminal
   deletion followed by a new upload operation.
9. Physical absence is accepted only from the exact typed backend evidence
   defined by the Caffeine contract; transport failure is never absence.
10. Deleted object identity may be uploaded again, but old reference IDs are
    never reactivated or reused.

### Logical reference

~~~text
Absent -> Reserved -> Active -> Released
~~~

Rules:

1. `BlobReferenceId` is allocated before an awaited effect or response.
2. `Reserved` binds one service, tenant, operation and canonical `BlobRef`.
3. `Active` is committed atomically with tenant/global counters and indexes.
4. Exact retry within the retained-receipt horizon returns the original
   `RetainedBlobRef`.
5. The same retained operation identity with a different request fails with
   conflict.
6. `Released` is terminal and remains queryable for a configured positive
   reconciliation horizon. The tombstone may then be reclaimed; the permanent
   reference-sequence high-water mark prevents identity reuse, while operation
   high-water marks prevent replay.
7. Release is authorized by the reference's exact tenant, not by digest alone.
8. One tenant cannot release, extend or resolve another tenant's reference.

### Tenant

~~~text
Active -> Draining -> Removed
~~~

Draining rejects new uploads and references but permits bounded release and
garbage collection. Removal requires zero active references, zero reservations
and no nonterminal tenant operation. Removed IDs are never reused.

## Caffeine Backend Boundary

Caffeine is the only version-1 backend. It remains an implementation detail of
`blob-service` and `blob-client`.

Before implementation, M0 must re-inventory actual Caffeine gateway and
Cashier source or deployed interfaces. The current Canic inventories were
accepted from Toko's project-side wrapper and explicitly did not identify a
separate gateway implementation. That evidence is insufficient to invent a
new upload-completion or deletion contract.

The re-inventory must freeze:

1. exact chunk and tree construction;
2. upload authorization and completion evidence;
3. retrieval and range behavior;
4. backend object existence semantics;
5. gateway deletion polling, ordering, bounds and confirmation;
6. gateway-principal discovery and rotation;
7. Cashier account, balance and top-up behavior;
8. typed absence versus transport failure;
9. maximum object, chunk, manifest, request, response and progress-page sizes;
10. whether a Caffeine project/account and deletion namespace bind exactly one
    owner Canister; and
11. production Canister IDs and immutable protocol provenance.

The private adapter must contain only operations proven by that inventory. It
must not expose a public trait until a second backend demonstrates a real
shared abstraction.

The six `_immutableObjectStorage*` compatibility methods may remain, with
their exact source-backed names and Candid, on `blob-service` if Caffeine
requires them. They are provider callbacks, not the portable client API. Their
continued existence in the standalone service is not Canic hard-cut residue.

`BlobRootHash`, ICFS tree identities, gateway-principal records, Cashier DTOs
and provider status types become private service-adapter types. The adapter
maps each accepted Caffeine tree root to one exact canonical `BlobRef` and
never presents the tree root or gateway URL as the portable content identity.

Version 1 enforces:

~~~text
one Caffeine project/account and deletion namespace
  <-> one exact blob-service Canister authority
~~~

The service stores this binding immutably and proves the live receiver is the
bound owner before upload authorization, liveness publication or deletion.
Two service Canisters must not publish liveness or deletion decisions for the
same Caffeine namespace. If M0 cannot prove provider-enforced exclusivity, the
operator must allocate a distinct project/account per service and the design
must record the external uniqueness procedure before implementation.

Toko starts with one shared blob-service Canister and one Toko tenant. It does
not shard blob-service until the Caffeine namespace and deletion authority are
proven safe for several services.

## Billing and Gateway Authority

The blob service, not each consuming application Canister, is the Caffeine
project/payment actor in version 1.

The service owns:

- exact Cashier configuration;
- its observed Cashier balance;
- gateway-principal synchronization;
- explicit top-up policy and journals;
- backend readiness; and
- tenant admission quotas which prevent one tenant consuming unbounded shared
  capacity.

Tenant-specific charging, invoicing and payment settlement are deferred. A
multi-tenant deployment shares one backend account while retaining exact
per-tenant usage counters. Deploying one service per application remains
valid for financial isolation only when each service has its own exclusive
Caffeine project/account and deletion namespace.

Cashier reads are observation. Cashier top-up and gateway-principal replacement
are durable service workflows. An uncertain transfer or sync is reconciled
from exact remote/local evidence before retry. Status never performs an
implicit transfer.

Canic may top up the blob-service Canister itself under ordinary Component
cycles policy. It does not decide how many cycles the service transfers to
Cashier and does not parse Cashier status.

## Public Service Operations

The exact application/operator Candid must be frozen in `blob-protocol` before
implementation. Version 1 requires these semantic operations without a `v2`
or parallel legacy form. Source-required provider callbacks belong only to the
separate frozen provider projection and are not application operations:

| Operation | Authority | Result |
| --- | --- | --- |
| rotate service operators | current service operator plus exact expected revision | atomically replaced nonempty operator set |
| create tenant | service operator | immutable tenant binding |
| update tenant actors | exact tenant admin | exact new actor revision |
| update tenant ceilings | service operator | exact operator-ceiling revision |
| reduce tenant self-limits | exact tenant admin | exact reduction-only self-limit revision |
| prepare upload | tenant blob writer | durable session bound to the manifest commitment and first bounded transfer page |
| inspect upload | exact originating caller, actor epoch and session | bounded checkpoint and next required action |
| commit upload | exact originating caller, actor epoch and session | terminal retained reference or typed pending status |
| retain tenant-known live object | tenant blob writer with an exact same-tenant active reference | new tenant-owned retained reference |
| release reference | tenant reference manager | terminal release receipt |
| get reference | tenant resolver | exact immutable retained reference |
| resolve download | tenant resolver | transient backend location plus immutable `BlobRef` |
| resolve public download | unauthenticated holder of an exact public `BlobRef` | transient safe serving descriptor only while a public reference keeps the object live |
| operation status | exact originating service operator or tenant actor | bounded current/terminal state |
| list tenant references | tenant resolver | bounded canonical page |
| service status | service operator | bounded backend and aggregate status |
| tenant status | tenant actor | bounded tenant counters and blockers |

All mutating operations take the caller's next `BlobOperationId` as their
first logical field. The service applies the actor-epoch, monotonic-sequence
and retry-horizon rules above. Exact retry during the retained-receipt horizon
returns the original terminal response. A sequence at or below the permanent
high-water mark whose receipt has expired returns `OperationExpired` without
performing an effect. Reusing a retained operation identity with any different
protected input fails before mutation.

Lists use opaque query-bound cursors no larger than 2 KiB, positive limits no
larger than 100, canonical ordering and bounded examined rows. No endpoint
returns an unbounded object, reference, tenant or operation vector.

Temporary upload/download instructions have a finite expiry and are never
valid as durable application data. Clients resolve again after expiry.

`resolve download` is an authenticated tenant operation because it reveals an
exact retained reference. Public-by-reference serving is separate. A tenant
may mark a reference `ServingPolicy::Public` only at retain time. The public
resolver accepts a canonical `BlobRef`, exposes no tenant, reference, quota or
operation data, and succeeds only while at least one exact public reference
keeps that object live. Toko may give this public route directly to browsers;
it need not persist or proxy temporary backend URLs.

## Public Serving and Active Media

Public serving uses a dedicated storage origin which carries no application
cookies or ambient application authority. Version 1 has one service-wide
inline-media allowlist frozen at installation. It initially permits only exact
canonical raster-image types whose browser behavior has been reviewed. HTML,
SVG, XML, script and other active or ambiguous media are never served inline.

Every resolved response must preserve the exact canonical `MediaType`, set
`X-Content-Type-Options: nosniff`, and use `Content-Disposition: inline` only
for the frozen allowlist. Other retained media is either returned with a safe
attachment disposition and sanitized filename or is unavailable through the
public resolver. Tenant policy may narrow, but never broaden, the service-wide
allowlist.

M0 must prove which of those headers and origins Caffeine itself can enforce.
If Caffeine cannot provide the exact safe response contract, the service must
refuse public resolution for that media class or place a separately proven
serving boundary in front of it. It must not manufacture security headers in
a descriptor when the eventual byte response will omit them.

## Upload, Read and Release Journeys

### Resumable upload and retain

The client contract is a bounded, checkpointable state machine:

~~~text
prepare manifest
  -> persist manifest-preparation checkpoint
  -> submit or receive next manifest page
  -> upload next chunk/page
  -> inspect progress
  -> verify
  -> commit
~~~

During source hashing, `blob-client` returns a bounded
`ManifestPreparationCheckpoint` containing the source cursor, declared size,
incremental raw-hash state, deterministic chunking state and manifest-page
progress. It contains no source credentials. Once preparation completes, the
application has a canonical `BlobRef` and manifest commitment without ever
materializing the complete object or manifest in one heap allocation. The
client's backend identity remains inside an opaque adapter checkpoint rather
than becoming application metadata.

After every successful service or backend step, `blob-client` returns a
version-1 `UploadCheckpoint` containing only bounded durable data:

- exact service, tenant, originating caller, actor epoch and operation identity;
- exact upload-session identity;
- canonical `BlobRef` and manifest hash;
- opaque bounded backend-continuation binding;
- next bounded manifest/chunk cursor; and
- exact acknowledged progress.

The checkpoint never contains a long-lived credential or gateway URL. A
consuming Canister persists it in application state and may resume after a
trap, timeout, upgrade within the same release, lost response or instruction
limit.

Version 1 deliberately uses origin-actor-owned sessions. Another current
`BlobWriter`, a tenant administrator, a service operator or a Canic controller
cannot resume, commit or take over that session. Actor removal is fenced until
its sessions terminate. Tenant-owned session transfer or repair by a different
actor requires a later design.

The upload journey is:

1. the application fetches no more than its configured bounded source page
   and validates application policy;
2. `blob-client` incrementally computes the raw content digest, canonical
   `BlobRef`, deterministic Caffeine manifest and private backend identity;
3. the application allocates its next `BlobOperationId` and asks the service
   to prepare the exact manifest commitment and declared bounds;
4. the service authorizes, reserves quota and persists the complete upload
   intent before returning transfer work;
5. the client persists the returned checkpoint before starting the next
   bounded backend effect;
6. the client uploads at most one bounded chunk/page, reconciles an uncertain
   result through the exact provider contract and persists advanced progress;
7. manifest and chunk pages repeat without placing the complete object or
   manifest in one ingress, response, instruction slice or heap allocation;
8. the same originating Principal and actor epoch call `inspect upload` to
   reconstruct the exact next action after interruption;
9. the backend/service completion boundary proves the exact private object
   identity and size;
10. `blob-client` retrieves or otherwise verifies the accepted bytes against
    the raw digest and size according to the frozen Caffeine contract;
11. the service commits or reuses one live physical object and allocates one
    tenant-owned logical reference; and
12. the application stores the returned complete `RetainedBlobRef` and clears
    its checkpoint.

M0 freezes positive finite maxima for object bytes, chunk bytes, chunks per
page, manifest bytes, request bytes, response bytes, in-flight sessions and
checkpoint bytes. The service and client reject a value before allocation or
effect when any bound would be exceeded.

A matching existing live object may skip physical upload only when the tenant
already presents an exact active same-tenant reference, or M0 proves a
cryptographic proof-of-possession protocol bound to the complete bytes. Mere
knowledge of a `BlobRef` or the existence of another tenant's backend object
cannot take the fast path. Otherwise the normal upload/verification journey
runs and the service may deduplicate internally afterward without revealing
the other tenant. Every path still creates one new tenant-owned reference and
charges the tenant's exact logical quota.

### Read

1. the application loads its `RetainedBlobRef` from IcyDB or another database;
2. it calls the exact `service` in the reference and asks it to resolve the
   exact tenant-owned reference;
3. the service returns a transient serving descriptor;
4. `blob-client` retrieves the bytes; and
5. `blob-client` verifies raw content digest and size before returning them.

The database never rewrites a stored gateway URL when routing changes.

### Release and garbage collection

1. before removing or replacing the application record, the application
   allocates its next `BlobOperationId`;
2. it atomically moves the record to an application-owned `ReleasePending`
   state containing the complete `RetainedBlobRef`, operation identity and
   intended mutation, and indexes that state in its release outbox;
3. if the database cannot atomically write a separate outbox row, the bounded
   `ReleasePending` record is itself the durable outbox and remains excluded
   from ordinary reads until the saga terminates;
4. a bounded worker calls the exact service and releases the exact
   service/tenant/reference tuple using the outbox operation;
5. the service commits the terminal reference tombstone and counters;
6. the application atomically applies the intended record mutation and
   removes the outbox or `ReleasePending` state after persisting the exact
   terminal receipt;
7. when no reference remains, the object enters its retention period;
8. bounded GC publishes an eligible backend deletion;
9. Caffeine confirms exact physical deletion; and
10. the service commits terminal object deletion.

Application record deletion and blob-reference release form an application
saga. The durable outbox is mandatory: deletion of the sole application copy
must never erase the authority address or operation identity needed to retry.
IcyDB or Toko owns its half. Blob-service never mutates application records.

## Canic Integration

The recommended topology is an ordinary application declaration, for example:

~~~toml
[component_specs.blob_service]
component_role = "blob_service"
maximum_instances = 1

[component_groups.project_cell.components.blob_service]
component_spec = "blob_service"

[component_group_deployments.project_cells]
component_group = "project_cell"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
~~~

The values are example application policy; the spelling uses the maintained
0.101 schema. 0.109 must not add a blob-specific configuration table.

The supported version-1 Canic topology is one blob-service Component shared by
the Fleet and bound to one exclusive Caffeine project/account and deletion
namespace. A separately deployed non-Canic Canister remains valid when it owns
its own exclusive backend binding and applications retain its exact Principal.

Multiple service Components, one-per-root locality and one-per-group-placement
topologies are deferred until deployment can provision and prove one distinct
backend account/namespace for every concrete service instance. Canic must not
infer or clone that authority from generic placement. Raising the example
limits before that proof would authorize an unsafe topology.

From Canic's perspective the service artifact is ordinary application Wasm:

- it appears in the application artifact union and admitted root release set;
- it uses ordinary Component placement and funding;
- it has no privileged Store or Coordinator access;
- it receives no blob-specific init DTO from Canic;
- it has no blob row in Fleet Registry; and
- it contributes to ordinary Component and Canister inventory counts.

### Optional `blob-service-canic` integration package

Canic provides no blob feature. When an application needs the maintained Canic
runtime envelope, it selects `blob-service-canic` from the standalone project:

~~~toml
[dependencies]
blob-service = { workspace = true }
blob-service-canic = { workspace = true }
canic = { workspace = true }
~~~

Outside a shared workspace, the consuming project selects exact published
versions through its normal dependency policy. `blob-service-canic` depends
on Canic; no published or production Canic package depends on a blob package.
The closeout fixture may use the exact qualified `blob-protocol` only as a
test-only dependency which is absent from every published dependency graph and
production Wasm.

The integration package may:

- compose ordinary public Canic endpoint and lifecycle facilities with the
  standalone service target;
- restore service state from an explicitly injected application-owned stable
  memory layout;
- pass opaque application init bytes to the service;
- obtain raw caller and time through generic Canic facilities;
- schedule the same standalone lifecycle handlers after Canic invariants are
  restored; and
- emit the exact standalone blob API and source-required provider callbacks
  plus only generic Canic Component lifecycle methods.

It must not:

- define a second protocol DTO, service handler, state record or policy;
- translate Fleet, root, Component, Directory, controller or Canic role
  membership into tenant authority;
- add root-only blob administration or make the root a data proxy;
- add Canic-specific caller eligibility to a blob endpoint or block the
  standalone public-resolver semantics;
- require a blob-named symbol or feature inside Canic;
- put blob protocol types in `canic-core`; or
- change the endpoint result merely because the service is Canic-managed.

The standalone and composed Canic targets must expose a byte-identical blob
API partition and identical source-required provider-callback partition, and
delegate them to the same handlers. The composed target may add only the
maintained generic Canic Component lifecycle endpoints, so its complete Candid
is the checked union of those three non-overlapping surfaces.
The service remains solely responsible for tenant authorization. The composed
target may apply Canic's generic runtime-readiness fence, but no Fleet,
Component or role check may narrow or broaden the blob API.

Direct deployment of the standalone Wasm through ordinary Component machinery
is the primary path. The integration package exists only when the consuming
application wants Canic lifecycle or endpoint composition. If M0 finds that
ordinary Component machinery cannot deploy either target, implementation
stops and records the missing backend-neutral Canic capability. It must not
add `canic::api::blob_service`, `canic_emit_blob_service_endpoints!` or a
Canic `blob-service` feature as a shortcut.

A non-Canic deployment uses the standalone Canister target and explicit
service/tenant configuration. Its client dependencies contain no Canic crate.

## Toko and IcyDB Boundary

Toko Feed owns provider ingestion:

- source URL and provider identity;
- download policy, timeout and maximum size;
- MIME validation and permitted media types;
- attribution and licensing metadata;
- retry policy for the upstream provider; and
- mapping the resulting reference into application records.

IcyDB owns application persistence:

- the complete service-qualified `RetainedBlobRef`;
- application record identity;
- source and attribution fields;
- application deletion intent;
- the durable release outbox; and
- the actor epoch, monotonic sequence and terminal receipt needed to complete
  or diagnose the release saga.

The blob service owns neither provider scraping nor application records. It
does not accept an arbitrary provider URL and fetch it on behalf of a tenant.

An IcyDB crate which does not use Canic depends on `blob-protocol` and,
optionally, `blob-client`. It calls the exact `BlobServiceId` retained in each
reference directly. No Fleet name, root Principal, Component binding or Canic
token is required by the portable contract.

Toko initially uses one shared blob-service authority and one Toko tenant
against one exclusive Caffeine project/account. It does not shard blob-service
authority until the backend deletion namespace and cross-service ownership
model have been separately proven.

## Project and Release Ownership

The standalone project and Canic have independent releases and closeout
criteria.

The standalone project must first publish one qualified immutable release. If
an artifact-only package is not published to a registry, the project release
still supplies an immutable source tag and content-addressed artifact. The
release contains:

- exact versions and source revisions for the three core libraries;
- the exact `blob-service-canister` Wasm, raw/compressed sizes and hashes;
- the canonical `blob-service.did`, frozen provider-callback inventory and
  generated-interface projection proofs;
- M0 provider evidence and the chosen conditional-package decisions; and
- standalone fixture evidence for init, upload, read, release, GC and restart.

That qualified standalone release is a prerequisite for Canic M5 and the Canic
hard cut. Canic 0.109 pins the exact standalone Wasm as an ordinary application
artifact, proves its release-set and Wasm Store journey, and never rebuilds or
vendors standalone source as a Canic infrastructure artifact.

Canic closeout uses a real Canic-repository fixture consumer and one ordinary
Component Spec. The fixture supplies `BlobServiceInitV1`, exercises the
service-qualified protocol and release outbox, and proves the hard cut without
depending on a Toko or IcyDB release. It is validation code, not another blob
implementation or production package, and must extend an existing Canic test
application rather than create a blob-specific Canic crate.

Toko Feed and IcyDB production adoption is a separate downstream release. It
must satisfy the consumer contract in this design before Toko switches, but it
does not block Canic 0.109 closeout once the qualified standalone release and
real fixture proof exist. Canic automation records downstream requirements and
must not modify the downstream or standalone repositories.

## Canic Hard Cut

0.109 removes the current blob subsystem rather than wrapping it.

The Canic repository must remove from maintained surfaces:

- `blob-storage` and `blob-storage-billing` Cargo features;
- `canic::api::blob_storage` and `BlobStorageApi`;
- Canic blob DTOs, domain values, models, views, policy, ops and workflows;
- Canic-owned `BlobRootHash`;
- stable blob roots, pending deletions, gateway principals and billing state;
- memory allocation keys and IDs 55 through 58 as active owners;
- Canic Caffeine and Cashier client wrappers;
- the old blob endpoint emission macros;
- blob protocol constants from Canic's public protocol surface;
- `canic blob-storage` CLI commands and JSON shapes;
- blob-specific Medic behavior;
- test Canisters and fixtures which implement or exercise the old Canic-owned
  subsystem, while retaining only the new black-box ordinary-Component
  consumer proof;
- current active integration and source-handoff runbooks;
- packaged-downstream blob CLI proofs; and
- CI gates which exist only to validate Canic's ownership of the old
  subsystem.

IDs 55 through 58 become unallocated `canic-core` reserve in 0.109. They are
not reassigned during the extraction, and no decoder reads their old bytes.

The hard cut must not remove generic Component deployment, cycles, status,
backup/restore or artifact behavior merely because the blob service consumes
it as an ordinary Component.

Historical changelogs and archived 0.69 through 0.71 designs remain intact.
The Caffeine/Cashier inventories may be archived as extraction evidence; they
must not remain described as current Canic runtime contracts.

The `_immutableObjectStorage*` names may exist in the standalone service's
provider adapter and its generated service Candid. They must be absent from
the Canic repository, Canic infrastructure Wasms, unrelated Components, CLI
fixtures and old Canic handler paths. `blob-service-canic` may expose the same
standalone blob API partition only from the standalone project; that does not
permit a blob-named Canic feature, module or macro.

There is no alias, deprecated wrapper, old-feature forwarding, state copier,
fallback decoder or mixed old/new endpoint emission. No adapter allowlist
permits old implementation code to survive in Canic.

Removal must not begin merely because this design is accepted. M0 first proves
the source-backed upload-completion, exact deletion and exclusive
Caffeine-project authority contracts, the concrete bounded transfer envelope,
and deployment of the replacement as an ordinary Component. Once those gates
pass, the hard cut is one coherent reinstall-only change: Canic must not ship a
half-removed old subsystem without an independently usable replacement.

## Stable State and Reinstall Boundary

Canic 0.109 is a fresh installation. Existing Canic-embedded blob state is not
copied into the service and is not read on startup.

The standalone service defines its own stable-memory ledger. It must:

- use service-owned keys and schemas;
- keep object, reference, tenant, quota, operation and backend-authority
  ownership distinct;
- use checked encoded-byte accounting;
- bound every record before an external effect;
- provide same-release restart, retry-horizon and snapshot/restore safety; and
- avoid hardcoding a Canic memory ID.

Where `blob-service-canic` composes one Wasm, the consuming application
supplies an explicit application-owned memory allocation from Canic's
application range. The three standalone core crates do not import that range
or choose the IDs themselves.

Cross-release service schema migration, Canic state adoption and rolling
mixed-version operation are outside 0.109. A future service release must
design them explicitly if the project remains pre-1.0.

## Failure and Interruption Safety

Every external upload, Cashier transfer, gateway-authority replacement and
physical deletion uses one durable, monotonic, identity-bound journal.

For each external boundary tests must cover:

1. before intent persistence;
2. after intent and before invocation;
3. effect completed with response lost;
4. response received before local commit;
5. local commit before caller receives the response;
6. restart and exact retry within the retained-receipt horizon; and
7. contradictory remote evidence.

Unknown upload completion cannot create a live shared object. Unknown Cashier
transfer cannot be repeated blindly. Unknown deletion cannot be interpreted as
absence. A failed reference release cannot decrement counters until its exact
terminal state commits.

Operations on separate tenants and digests may proceed independently. There is
no service-wide mutex across an awaited backend call. Per-tenant and global
reservations protect bounded capacity without serializing unrelated objects.

## Security Requirements

1. Every update authenticates the raw caller before mutation.
2. Tenant, operation, reference and digest binding is explicit.
3. A caller-provided `BlobTenantId` is never sufficient authorization.
4. Controller status alone grants no tenant access.
5. Backend gateway principals cannot call tenant APIs.
6. Tenant actors cannot call backend confirmation or billing-admin APIs.
7. One tenant cannot observe another tenant's reference or quota state.
8. Cross-tenant physical deduplication never merges logical ownership.
9. Release requires the exact reference identity, tenant and operation.
10. Download results are verified against both digest and size.
11. Media types reject control characters and are never copied into an HTTP
    header without safe parsing.
12. Temporary backend credentials are bounded, redacted from logs and never
    persisted in application databases.
13. Request decoding is quota-bounded before hashing, signature verification
    or stable mutation.
14. Caffeine/Cashier failures map to typed service errors without leaking
    secrets or treating transport errors as authoritative state.
15. Service operator access is distinct from tenant access and is explicitly
    configured at installation.

## Observability and Operations

`blob-service` exposes bounded operator and tenant status through its own
protocol and tooling.

Operator status includes:

- backend readiness;
- Cashier balance observation and funding blocker;
- gateway-authority revision and last successful sync;
- physical object/byte counts;
- pending upload/deletion counts;
- retained receipt use;
- stable encoded-byte use; and
- bounded GC progress.

Tenant status includes only that tenant's quota ceilings, current use,
reservations and blockers.

Canic generic inspection may report that the ordinary Component exists, its
role, root, Subnet, controllers, module, cycles and lifecycle status. It does
not reproduce blob-service status. Blob-specific operator commands belong to
the standalone project.

## Validation

### Protocol and client

- `blob-protocol` is the sole owner of application/operator method names,
  modes, wire DTOs and canonical `blob-service.did`;
- golden raw SHA-256 and private ICFS-tree vectors for empty, one-byte,
  exact-chunk, multi-chunk and maximum bounded inputs;
- cross-implementation private-backend vectors against the accepted
  Caffeine/ICFS algorithm without conflating them with `BlobDigest`;
- canonical Candid and JSON snapshots at schema version 1;
- canonical MIME normalization plus malformed digest, identity, media type,
  cursor and oversize rejection;
- service-qualified retained-reference round trips and wrong-service
  rejection;
- deterministic insertion-order-independent encoding where maps/sets exist;
- checkpoint encoding bounds and interruption/restart after every manifest,
  chunk, inspection, verification and commit boundary;
- `ManifestPreparationCheckpoint` and `UploadCheckpoint` remain local
  `blob-client` types and no provider identity enters a service request;
- upload and download raw-size/digest verification; and
- compilation of a consumer with no Canic dependency in its resolved graph.

### Service state

- bounded `BlobServiceInitV1` parsing, exact schema/layout versions, nonempty
  unique operator set and malformed/oversized init rejection before state;
- controller and Canic role principals gain no service-operator authority;
- expected-revision operator rotation, caller-retention, two-step sole-operator
  replacement, exact retry and removal rejection while another operator owns
  nonterminal work;
- tenant ID and reference ID monotonic allocation and non-reuse;
- monotonic actor sequence acceptance, exact retry within the configured
  horizon, permanent high-water replay rejection, `OperationExpired`, actor
  epoch non-reuse and conflicting-operation rejection;
- cross-tenant reference isolation;
- same-digest multi-reference and multi-tenant behavior;
- cross-tenant digest knowledge cannot use a retain fast path, and service
  responses/status do not disclose whether another tenant retains the object;
- quota reservation before awaits and exact settlement;
- retention cancellation before publication and rejection after publication;
- bounded GC paging and retry;
- released-reference reconciliation horizon, bounded tombstone reclamation
  and permanent reference-sequence non-reuse;
- operator ceiling growth/decrease rules and tenant reduction-only self-limit
  rules;
- stable-state round trips and corrupted-record rejection;
- independent-tenant concurrency; and
- only the originating caller and actor epoch can inspect, resume or commit an
  upload session; another `BlobWriter` and a removed actor reject; and
- no unbounded scan on upload, retain, resolve or release hot paths.

### Backend integration

- source-backed Caffeine endpoint and method-mode snapshots;
- one source owner for every provider DTO, method and hashing primitive, with
  no duplicate client/service definitions;
- exact object/chunk/manifest/request/response/progress bounds;
- resumable upload completion with response loss at every page;
- gateway authority rotation;
- deletion polling, repeat return, confirmation and typed absence;
- one exact Caffeine project/account and deletion namespace bound exclusively
  to one blob-service authority;
- safe public serving on a separate origin with exact `nosniff`, disposition
  and inline-media policy, or fail-closed rejection when the backend cannot
  provide it;
- Cashier balance and explicit top-up recovery;
- transport failure never treated as absence or successful funding; and
- one disposable Caffeine-compatible end-to-end upload/read/delete journey.

### Deployment integration

- all three core packages build as `rlib` without endpoint exports;
- `blob-service-canister` is the only standalone `cdylib`; its generated blob
  projection equals the canonical protocol file and its provider projection
  equals the frozen external contract;
- one standalone PocketIC service used by a Canister with no Canic dependency;
- direct installation of the standalone service as one ordinary Component;
- optional composition through external `blob-service-canic` when a consuming
  Canister needs the generic Canic lifecycle/endpoint envelope;
- default Canic and every Canic infrastructure Wasm contain no blob-service
  protocol, adapter, Caffeine or Cashier code;
- Canic's published package and production feature graph contains no blob
  dependency or blob-named feature; only the explicit fixture's qualified
  test dependency is permitted;
- standalone and `blob-service-canic` blob/provider partitions are
  byte-identical, while the composed full Candid adds only the maintained
  generic Canic Component lifecycle methods;
- linking `blob-service` or `blob-service-canic` emits no endpoint until the
  final target invokes its one allowed exporter, and feature unification cannot
  duplicate an IC export;
- every composed endpoint delegates to the same standalone handler;
- exact application release-set and Store artifact selection;
- no fourth infrastructure entry;
- no blob-specific Fleet Registry row;
- service and consumer may reside on different Subnets without changing
  authority; and
- two co-located Fleets cannot access each other's tenant references without
  explicit blob-service grants.

### Consumer contract and Canic fixture

- the Canic-repository fixture stores the complete service-qualified
  `RetainedBlobRef` and keeps fixture metadata outside blob-service;
- application record removal persists the complete release outbox before the
  reference can become unreachable;
- interruption before record mutation, after record mutation, after service
  release and before outbox removal converges without losing or duplicating
  the release;
- an expired release operation reconciles the exact reference tombstone before
  allocating a later sequence, and enters operator review when the separate
  reference-reconciliation horizon has also expired;
- the fixture uses one service authority, one tenant and one exclusive
  Caffeine project/account; and
- the downstream conformance checklist requires Toko to keep source,
  attribution and licensing outside blob-service and IcyDB to store the
  complete retained reference, without making their release a Canic gate.

### Canic hard cut

- no executable Canic occurrence of removed feature names, macros, APIs,
  state, CLI commands or handler paths;
- no `canic::api::blob_service`, `canic_emit_blob_service_endpoints!`, Canic
  `blob-service` feature or other new production blob integration survives in
  the Canic repository; the explicit black-box fixture may consume only the
  maintained external protocol and artifact;
- no old Canic Candid/generated fixture occurrence;
- memory IDs 55 through 58 have no active allocation owner;
- no prior state decoder, migration or compatibility feature;
- archived designs and changelogs remain intact; and
- ordinary Component and generic lifecycle tests continue to pass.

## Implementation Milestones

### M0: Source and boundary inventory

1. locate and freeze actual Caffeine/ICFS and Cashier source or deployed
   protocol evidence;
2. freeze independent raw-content and ICFS chunk/tree golden vectors;
3. prove upload-completion, page-resume and typed-deletion evidence;
4. freeze positive finite object, chunk, manifest, request, response, session,
   checkpoint and progress-page limits;
5. prove one Caffeine project/account and deletion namespace has one exclusive
   blob-service authority;
6. prove or reject the required separate-origin, `nosniff`, disposition and
   inline-media serving contract;
7. prove ordinary Component machinery can deploy the standalone Wasm or the
   externally composed `blob-service-canic` Wasm without a Canic blob feature;
8. freeze the physical workspace, package crate types, final Wasm owners and
   endpoint-emission graph, including proof that Cargo feature unification
   cannot duplicate exports;
9. decide whether `blob-service-canic` and `blob-caffeine-internal` are needed
   and record the evidence for each decision;
10. freeze `blob-protocol` as the sole canonical blob Candid authority, freeze
    the external provider callback inventory and prove both
    standalone/composed projection methods;
11. freeze every `BlobServiceInitV1` bound and the exact service-operator
    rotation/retry contract;
12. inventory every current Canic blob surface and stable allocation; and
13. choose the standalone repository and published package prefix.

Exit: no unresolved provider fact is represented as a protocol fact, the
replacement can be deployed without blob coupling inside Canic, and no old
Canic subsystem removal has begun. The conditional-package decisions, exact
Wasm target and Candid authorities are recorded. Failure of items 3, 5, 6,
7, 8, 10 or 11 blocks implementation and requires a revised design rather
than an invented adapter.

### M1: Portable protocol

1. implement bounded `BlobDigest`, canonical `MediaType`, `BlobRef`, complete
   `RetainedBlobRef`, service/tenant/reference identity and actor-sequence
   operation IDs;
2. implement bounded `BlobServiceInitV1` and service wire DTOs;
3. freeze method names, modes, version-1 canonical Candid and JSON;
4. freeze both immutable horizons and the `OperationExpired` contract;
5. implement canonical parsing and error codes; and
6. publish protocol documentation without Canic terms.

Exit: a non-Canic fixture consumes `blob-protocol` with no Canic dependency.

### M2: Client and integrity

1. implement incremental raw hashing plus private canonical ICFS
   hashing/chunking;
2. lock source-backed golden vectors for both identities;
3. implement bounded manifest/page preparation and durable
   local `ManifestPreparationCheckpoint` and `UploadCheckpoint` transitions;
4. implement inspect/resume and retrieval sequencing; and
5. verify raw size and digest on every completed retrieval.

Exit: deterministic bytes reproduce both their raw-content golden digest and
their independently named private Caffeine/ICFS identity, and a corrupted
retrieval fails closed.

### M3: Standalone service authority

1. implement `BlobServiceInitV1`, the explicit operator set, revision-bound
   operator rotation and immutable service policy;
2. implement tenants, actor grants and quotas;
3. implement object/reference state and stable indexes;
4. implement actor epochs, permanent sequence high-water marks and
   horizon-bounded exact-retry receipts;
5. implement origin-actor-only upload-session recovery;
6. implement bounded status, paging, retention and GC; and
7. prove same-release restart/restore behavior.

Exit: reference ownership and deletion are correct without any Canic crate.

### M4: Caffeine and Cashier adapter

1. implement only source-proven provider operations;
2. implement the recorded single-owner provider definitions, using the
   unpublished internal package only when M0 required it;
3. export required `_immutableObjectStorage*` adapters from the standalone
   shell and, when present, the conditional composed target, while retaining
   one handler implementation in `blob-service`;
4. enforce the exclusive project/account and deletion-namespace binding;
5. implement gateway authority and billing state;
6. prove bounded resumable upload/read/delete and interruption recovery;
7. prove the safe public-serving policy; and
8. retain no provider URL or private ICFS identity in `BlobRef`.

Exit: the standalone service completes one disposable real backend-compatible
journey.

### M5: Ordinary Component and optional external integration

1. consume one already-qualified standalone-project release;
2. pin the exact `blob-service-canister` Wasm as ordinary application artifact
   identity;
3. deploy it directly through ordinary 0.100/0.101 topology and release sets;
4. where M0 proved a consumer needs Canic runtime composition, use
   `blob-service-canic` only in the standalone repository using generic Canic
   facilities;
5. prove the standalone and composed blob/provider partitions are
   byte-identical, delegate to the same handlers and form a collision-free
   union with only the generic Canic Component lifecycle methods;
6. prove generic lifecycle, placement and cycles behavior; and
7. prove published/production Canic packages, default builds and
   infrastructure Wasms contain no blob dependency, adapter, protocol or
   service implementation.

Exit: the service is operational under Canic and from a non-Canic client.

### M6: Canic fixture and downstream contract

1. extend one existing Canic test application with a real fixture consumer of
   the qualified standalone release, without adding a blob-specific crate;
2. deploy the service through one ordinary singleton Component Spec;
3. prove complete service-qualified references and the mandatory application
   release outbox through the fixture;
4. publish the Toko Feed/IcyDB adoption checklist for their independent
   downstream release; and
5. prove the Canic fixture contains no provider-source, attribution or
   application-specific blob policy.

The Canic fixture is required for Canic closeout. Actual Toko Feed and IcyDB
production adoption remains downstream work and is not a Canic 0.109 release
dependency.

### M7: Canic hard cut and closeout

1. only after M0 through M6 pass, remove the complete current Canic blob
   subsystem in one coherent hard cut;
2. release memory IDs 55 through 58 to unallocated reserve;
3. remove CLI, Medic, test fixtures and ownership-specific CI gates;
4. archive or replace active guidance while preserving historical changelogs;
5. run hard-cut residue searches and current-surface validation; and
6. verify default Canic, Coordinator, root, Store and unrelated Component
   Wasms contain no blob-service, Caffeine or Cashier code.

Exit: Canic operates the service only as an ordinary Component.

## Completion Criteria

0.109 is complete only when:

1. the three core libraries and required `blob-service-canister` package build
   in the standalone workspace without a Canic dependency;
2. `blob-service` is library-only, `blob-service-canister` is the sole
   standalone `cdylib`, and no feature unification can duplicate endpoint
   exports;
3. each conditional integration/internal package exists only when M0 recorded
   its necessity, and optional `blob-service-canic` depends toward Canic from
   the standalone repository only;
4. `blob-protocol` is the sole blob application/operator
   method/mode/wire/Candid authority, Caffeine evidence is the sole provider
   callback authority, and both final targets conform to the exact projections;
5. `BlobServiceInitV1`, the explicit operator set, immutable global policy and
   revision-bound rotation rules are fully bounded and controller-independent;
6. raw content digest and private ICFS identity are distinct, source-proven
   and protected by golden vectors;
7. canonical MIME parsing and active-media serving fail closed;
8. the qualified standalone release and exact Wasm identity exist before the
   Canic hard cut;
9. a non-Canic Canister uses the standalone shell successfully;
10. every stored logical reference contains exact service, tenant, reference
   and immutable blob identity;
11. tenant-owned references prevent cross-tenant release and deletion;
12. one Caffeine project/account and deletion namespace has one exclusive
   blob-service authority;
13. every retained object has exact bounded quota and counter ownership;
14. operator ceilings can grow within global capacity while tenant self-limits
   remain reduction-only;
15. actor epochs plus operation and reference sequence high-water marks prevent
    replay or identity reuse after finite terminal receipts and reference
    tombstones expire;
16. local bounded client checkpoints resume after every page and interruption,
    while only the originating caller and actor epoch can resume the service
    session;
17. client byte-plane and service control-plane Caffeine responsibilities have
    one source of truth and no duplicated provider definitions;
18. Caffeine is the only backend and its adapter remains private;
19. upload, Cashier and deletion response loss recover without duplicate
    effects or fabricated success;
20. the real Canic fixture persists complete immutable references without
    gateway URLs and releases them through a durable application outbox;
21. a Canic Fleet deploys one singleton service as an ordinary Component with
    no special infrastructure role or Registry authority;
22. Toko/IcyDB adoption requirements are published as a downstream contract,
    not treated as a Canic release dependency;
23. Canic contains no old blob-specific feature, API, state, stable-memory,
    macro, CLI, Medic, fixture or generated surface;
24. Canic also contains no replacement blob feature, module, macro or
    production package dependency; the sole test-only protocol dependency is
    confined to the black-box fixture, while optional API/lifecycle
    composition exists only in external `blob-service-canic` and delegates to
    the standalone handlers;
25. IDs 55 through 58 are unallocated and no old decoder reads them;
26. current documentation describes the standalone boundary while historical
    changelogs remain intact; and
27. no migration, alias, fallback or mixed old/new path exists.

## Source Documents

- [Canic 0.100 Fleet Subnet Roots](../../archive/0.100-multi-subnet-fleet-coordinator-and-registry-synchronization/0.100-design.md)
- [Canic 0.101 Component Groups](../../archive/0.101-fleet-authoritative-service-provisioning-and-publication/0.101-design.md)
- [Archived 0.69 blob protocol design](../../archive/0.69-blob-storage/0.69-design.md)
- [Archived 0.70 blob billing design](../../archive/0.70-blob-storage-billing/0.70-design.md)
- [Archived 0.71 operator design](../../archive/0.71-blob-storage-operator-readiness/0.71-design.md)
- [Current Caffeine gateway inventory](../../../contracts/BLOB_STORAGE_INVENTORY.md)
- [Current Cashier inventory](../../../contracts/BLOB_STORAGE_CASHIER_INVENTORY.md)

## Deferred Work

The following require later designs:

- optional Canic backup archival, mandatory client-side encryption,
  recovery-capsule discovery and the external native repository-adapter
  contract are owned by the M0-only
  [encrypted canister-snapshot archive idea](../optional-encrypted-canister-snapshot-archives/design.md);
- private/encrypted objects and access-controlled byte delivery;
- tenant billing, invoicing or chargeback;
- a second physical backend;
- backend migration while preserving live service availability;
- geo-replicated or multi-Subnet byte storage;
- CDN policy;
- public backend plugins;
- cross-release service schema migration;
- rolling service upgrades; and
- automatic ingestion from arbitrary third-party URLs inside blob-service.

None of these may be implied by version-1 names or placeholder enum variants.
