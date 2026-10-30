# Owned endpoint and frontend Wire hard cut

Accepted for the current 0.112 batch on 2026-10-22, under
[Canic #613](https://github.com/dragginzgame/canic/issues/613). This supersedes the
prior owned-Candid exception. It is an implementation contract, not a claim that
the migration is delivered. Owned frontend delivery and runtime endpoints use
Wire; the 2026-10-26 boundary also covers initialization, cursors and hash inputs.

The dated supplier and qualification sections below preserve historical milestones.
Their remaining-work statements describe those milestones, not current blockers.
Use the [current handoff](../../status/current.md) for implemented delivery and
the concrete remaining endpoint, Fleet mutation-authority and acceptance work.
The normative hard-cut requirements and implementation sequence remain binding.

## Accepted completion boundary — 2026-10-26

The maintainer clarified that Canic-owned production structs must no longer derive
or manually implement `CandidType`. The cut includes owned initialization payloads,
internal cursors and canonical hash inputs, with their producers, validators,
retained current-release records and fixtures changed together. Earlier statements
retaining those Canic-owned Candid encodings are superseded by this decision.
This does not relabel old Candid hashes as Wire hashes or migrate predecessor state.

Required external IC contracts use published `ic-platform` types. Other external
protocols use their owning upstream contracts. Remove local mirrored IC DTOs where
the selected supplier exposes their contract; record a named supplier/publication
gap where it does not. Generic external-Candid adapter bounds and historical
evidence may still mention `CandidType`; they are not Canic-owned data contracts.
Provider fixtures must retain the external protocol they actually model.

Preserve explicit authority bindings, semantic validation, bounded decode and paid
effects, original submitted bytes, uncertain-effect reconciliation, same-release
recovery, cycle conservation and Blob obligations. The persistence replacement
remains distinct from the Wire protocol cut. The maintainer's 2026-10-29 decision
selects bounded native Wire for owned stable values and checkpoints in the pending
0.113 batch, superseding the earlier serde_cbor representation. See the
[current storage contract](../../features/runtime/stable-memory-layout.md#owned-stable-value-format).
No dependency upgrade, sibling edit, publication or installation retirement is
implied. Measure final Root code against an identified artifact/profile; the
8,876,601-byte retained Root already has native Wire endpoints and is not a
pre-Wire baseline.

## Canonical owners and transport choice

Canic Contracts owns passive application types and exact method contracts.
Canic Macros emits endpoint registration and execution from the same declaration.
Core owns admission, authentication, dispatch and lifecycle. Host owns compiled
role selection, release artifacts, browser transport configuration and integrity.
IC Wire owns bounded Postcard frames, schema projection and TypeScript codecs;
IC Build owns artifact transactions/cache mechanics.

Use Wire's native Serde codec with explicit per-method request and response
limits. This retains existing public Rust shapes, including generic pagination
and dynamic collections, without changing stable-storage serialization as a
side effect. Add missing Serialize implementations at their actual type owner.
Do not substitute an unchecked serializer or a generic unbounded decoder.

Metadata must describe those actual Serde bytes. Unsupported Serde attributes,
custom serializers or custom validators fail generation explicitly until their
owning type supplies a verified projection. Do not silently describe a custom
text serialization as its underlying byte array. Canic's CanonicalNetworkId is
one concrete example: its serializer emits canonical hexadecimal text.

The selected registry supplier is published IC Wire 0.6.4. Its native codec,
host-only metadata `Wire` derive and TypeScript registry/runtime provide the
selected route. The former finite codec API is removed. Canic Contracts now
uses the optional host-only `wire-schema` feature to derive metadata from the
actual `Page<T>` and `PageRequest` declarations, with explicit Serde serialization.
Native tests qualify concrete generic projection, declaration-order bytes and
frame identity/direction/size/trailing refusals. The former missing-derive gap
tracked at [IC Wire #17](https://github.com/dragginzgame/ic-wire/issues/17) is
addressed by the published API; complete Canic schema coverage is still pending.

Blob 0.43.0 now owns the coordinated native tenant migration: its tenant methods
use `0043/001a` and `0043/001b` with zero-based failure tags. Canic consumes the
existing upstream encode/decode functions and catalog; no codec shim is needed.
Both managed compositions pass on Wire 0.6.5, and Wire 0.5 is removed from the
framework and independent consumer locks. Other Blob method bytes and storage
encodings are unchanged. Canic's endpoint registry, generated-client artifacts
and release identities remain separate unfinished work under this contract.

## Qualified scalar/client foundation and remaining supplier gaps

Canic's actual CanonicalNetworkId now supplies explicit host-only text metadata
with its lowercase-hex validator. The `wire_conformance` Contracts example emits
PageRequest and Page<CanonicalNetworkId> schemas, generated TS/runtime, exact Rust
frames and a Node assertion harness. It does not register a production method.
Strict TypeScript 7.0.2 compilation with ES2022 modules/declarations and Node
24.21.0 execution pass for exact request/response bytes, invalid domain values,
missing checks and wrong-method/direction/trailing/oversized frames. This local
compiler is read-only sibling tooling evidence, not Canic release-tool admission.
The deployed Contracts graph excludes Wire metadata and serde_json.

Owned Wire DTOs use `ic_platform::Principal`, never ExternalPrincipal. The latter
is confined to required external Candid contracts with explicit conversions at
those boundaries. Do not change shared stable-state encodings as a side effect
of changing endpoint DTOs.

Published Wire 0.6.4 resolves deny_unknown_fields and qualified serde_bytes field
metadata. Platform 0.6.9 now provides native Principal metadata under its host-only
`wire-schema` feature, resolving the supplier gap in
[Platform #48](https://github.com/dragginzgame/ic-platform/issues/48).
Canic's generated client qualification uses actual Page<ic_platform::Principal>
with empty, anonymous and maximum-length identities. Rust and generated TypeScript
agree on exact frames and reject 30-byte identities. The normal Contracts Wasm
graph excludes JSON without schema selection. Explicitly enabling wire-schema
currently selects JSON through Wire's typescript feature; the owner-boundary gap
is recorded in [Wire #17](https://github.com/dragginzgame/ic-wire/issues/17#issuecomment-6382090840).
This qualifies the scalar foundation; production endpoint/caller conversion,
endpoint-derived registry, release identities and frontend bundles remain pending.
ExternalPrincipal remains external-Candid-only; no wrapper identity or duplicate
DTO catalog is needed.

The host example can be reproduced with `cargo run --locked --offline
-p canic-contracts --features wire-schema --example wire_conformance -- OUTPUT`.
Compile its four emitted TS files with the qualified TypeScript compiler using
strict, noUncheckedIndexedAccess, exactOptionalPropertyTypes, ES2022, bundler
module resolution and declaration output into OUTPUT/compiled, then run
OUTPUT/check.mjs with Node. No package download occurs during generation.

## Owned Account consumer qualification — 2026-10-24

Published Platform 0.6.12 supplies Account metadata. The existing Contracts
`wire_conformance` example now contains a test-only Account envelope; it registers
no production endpoint and does not replace external ledger Candid account DTOs.
Nine cases combine empty/anonymous/29-byte principals with absent, explicit-zero
and nonzero 32-byte subaccounts. Rust and generated TypeScript preserve exact
request/reply bytes and representation, including equal effective accounts whose
bytes differ. Encoders reject malformed lengths; decoders reject oversized owners,
truncation and trailing bytes. No implicit signing/deduplication normalization or
storage change is introduced. The fixture uses finite 1024-byte, 8192-allocation,
256-node and 16-depth limits; production endpoint budgets are unchanged.

Focused strict Clippy, native vectors, strict TypeScript 7.0.2/Node 24.21.0,
Rust 1.91 example compilation and Contracts Wasm compilation pass. The sibling
compiler is local test evidence, not admitted production browser delivery.
Explicit Wasm schema-feature selection still includes Wire's JSON dependency;
compile success does not satisfy that graph-exclusion acceptance. Track this and
default-branch integration in [Canic #628](https://github.com/dragginzgame/canic/issues/628).
No installed Account endpoint or native macOS qualification is claimed.

## Compiled native registry foundation — 2026-10-23

Canic's optional `wire-schema` feature now registers typed Wire endpoint
signatures directly from the endpoint macro. `canic::endpoint::schema::snapshot(&compiled)`
returns schema_version 1 with sorted methods and named request/reply projections,
exact IDs and both bytes/allocation/nodes/depth budgets. Composite queries retain
`kind = composite_query` while their generated browser transport uses query.
Wire owns type projection, schema/reference checks and TypeScript generation;
Canic rejects empty and duplicate-name/ID inventories. Unsupported metadata fails
rather than being inferred from Candid. The registry is native-only and disabled
in deployed Wasm, including when the feature is selected for a Wasm target.

Compiled query/update/composite endpoint tests and deterministic/refusal tests
qualify this foundation. Typed adapters also emit private Wasm records containing
the exact method name, ID, export kind and both codec budgets. Host validates
their corresponding exports, rejects duplicate IDs/names and overlapping codec
owners, and requires shrink and Binaryen to preserve those declarations.
This is compiled identity evidence, not a type-layout schema or sidecar hash.
The native registry requires those final compiled records and compares its exact
inventory, IDs, export kinds and all budget fields before returning metadata.
Callers must obtain those records from the admitted final Wasm. It does not yet
compose upstream Blob metadata in production artifacts, bind schema hashes or
replace production callers and frontend bundles. Native projection alone does
not prove capability/target parity; the artifact pipeline must establish that
before admitting a release. No production endpoint migration or complete #613
qualification is claimed by this step.

Published Blob/contracts 0.43.4 supplies native schemas, TypeScript generation,
its custom scalar adapter and compile-time mounted descriptors. Canic consumes
the mounting callback to emit complete raw Wire records from the upstream ID and
both allowances; no method roster or numeric policy is copied. Dedicated and
embedded managed proofs recover those records from final manifest-verified Wasm
and compare exact IDs, query/update modes and all eight budget fields with the
supplying catalog/schema. Provider Candid methods must have no Wire record;
application-owned Wire methods may coexist in embedded artifacts. Duplicate names
and IDs refuse. This is compiled identity/budget parity, not payload-layout proof.

The published supplier gaps in [Blob #100](https://github.com/dragginzgame/ic-blob-storage/issues/100)
and [Blob #101](https://github.com/dragginzgame/ic-blob-storage/issues/101) are
resolved locally. Native combined-schema production, immutable release binding
and generated frontend integration remain unfinished. Provider-credit
qualification under #494 remains separate. No local DTO schema or decimal
validator copy is needed; existing frame/storage/lifecycle behavior is unchanged.

## Supplier runtime composition

Blob's upstream browser adapter owns its decimal validators. Canic Contracts'
network schema owner supplies a composable client adapter which retains supplied
checks, adds the canonical Canic network validator and refuses replacement of
that check. Blob can delegate to it while both use one Wire runtime. There is no
copied Blob scalar policy, second codec, authorization or transport in this layer.
The native `wire_composition` example generates the complete Blob catalog plus a
qualification-only method combining actual Canic network and Blob relationship
types. Strict generated TypeScript/Node checks match Rust frames and refuse bad
values from both owners. This qualifies runtime composition; the managed combined
schema producer and exact release/frontend binding below remain unfinished.

## Endpoint-derived artifact contract

The planned owned sidecar is `wire-schema.json`, with schema_version 1. It contains
one deterministic, sorted registry of named types and compiled methods. Each
method records its exact name, query/update/composite-query kind, explicit stable
u64 ID, request and response type references, and both codec limit records
(bytes, allocation, nodes, depth). IDs belong to the declaration; do not hash a
Rust name, depend on traversal order or assign capability-dependent enum ordinals.
An endpoint may name its owner's u64 constant path instead of repeating the
numeric literal. Calls and arithmetic are not accepted in the attribute.
A raw owner-coded endpoint can also select this contract: Canic derives its
pre-copy request byte limit and emits its compiled declaration, while the
supplier retains framing, traversal/allocation and reply admission. The native
producer must register the supplier's actual payload projections; a raw
zero-argument/unit handler signature is not a payload schema.
Reject duplicate method names/IDs, missing references and unsupported schemas.
The projected variant layout must match the capability-selected artifact.

The existing `dto::wire` unions are Candid contracts, despite their directory
name. Their capability attributes remove enum variants, and their narrow
`projection` consumers rely on Candid structural decoding. They cannot be
migrated by changing codec hooks alone. For example, managed public-status
`Overview` follows `Health` and optional `Metrics`/`History`; the Host readiness
projection contains only `Overview`. Native Serde enum indices therefore differ.
Published Wire 0.6.10 forwards Serde's variant index to its encoder; it does not
provide Candid-style structural projection. This is unfinished Canic migration,
not an identified Wire defect. Before cutting over a union endpoint, establish
one matching payload layout for its actual Rust callers and capability-selected
artifact, remove the obsolete projection consumers, and qualify cross-profile
request/reply bytes. A matching method ID or generated browser schema alone does
not qualify a Rust caller using a different enum layout.

The maintainer selected separate typed endpoints per operation on 2026-10-23.
Cut the capability-selected status unions over to directly typed methods; keep
capability selection at endpoint emission, not within payload enum layout.
The contract owner assigns each operation an explicit method identity and bounds.
Migrate role dispatch, Rust callers, generated schema/bindings, fixtures and
release identities together, then delete the corresponding union projections.
Do not retain the old union endpoint as an alias or add a fixed full-union lane.
This selection is accepted work, not a claim that these endpoints exist yet.

Generate the sidecar from the same compiled declarations that emit actual
entrypoints. Compare its method inventory with the final Wasm exports. External
standard methods retain their own Candid ABI and explicit classification; they
must not acquire Wire metadata merely because they share an artifact.

Host `canister_build::generate_wire_bindings` now admits a bounded native schema
against final Wasm declarations and exports before invoking Wire's TypeScript
generator. It requires exact IDs, export kinds, browser modes and both complete
budgets, and delegates type/reference admission to Wire. This API does not yet
run automatically from the managed artifact build.

Host `produce_wire_schema` can acquire a selected native producer's exact output
through IC Build's retained transaction, after final-Wasm schema admission. The
dedicated and embedded Blob consumers supply qualified native examples linked to
their mounted registries. The explicit `build_workspace_wire_schema` API resolves a configured
role, preserves its runtime profile/features, adds metadata only to the native
invocation, and selects Cargo's reported executable. IC Build's Cargo snapshot
guards acquisition through commit. Normal artifact-build invocation and release
hash binding remain unfinished.

Root, Coordinator and Store fast/local Wasm builds and configured native producer
acquisition pass on 2026-10-24 (81.05s, 49.18s and 20.38s). Together with both
Blob shapes, this exercises all five producer fixture selections through the
shared owner. Logs and artifact/lock hashes: `/tmp/canic-fleet-schema/`. Root and
Coordinator Wasm were built before the root lockfile advanced Memory/Storage
from 0.50.13 to 0.50.14; Coordinator native acquisition and Store qualification
used the later graph. These are explicit producer/admission checks, not one
stable release-graph qualification or a complete framework Wire cutover.

Embedded qualification on 2026-10-24 passes its governed Wasm build, configured
source-snapshot acquisition and retained schema/TypeScript recovery after output
corruption. Changing a compiled method identity is refused without replacing the
previous outputs. Evidence and input hashes: `/tmp/canic-embedded-schema/`.
The Host graph selects Timers 0.30.9; the private embedded graph selects 0.30.8.
The embedded application counter query/update now also use native Wire with
application-owned IDs, unit requests and u64 replies. The contract bounds frames
to 64 bytes, allocation to 256 bytes, traversal to eight nodes and depth to four.
The native regression covers full-width responses; the managed PocketIC case
qualifies wrong-method/trailing-frame refusal before mutation and retained
counter state across all five recovery scenarios. Native schema acquisition
passes against that exact managed backend artifact. Evidence: `/tmp/canic-counter-wire/`
(189s PocketIC, 45.36s schema acquisition). Installation envelopes are unchanged.
This is not installed-browser or complete framework ABI cutover evidence.

Release records must introduce an explicitly named wire_schema_sha256 identity
and bind the exact sidecar bytes to role, capabilities, module and release. Do not
rename or reuse candid_sha256 values. Retain external Candid sidecar identity only
where an external protocol actually consumes it. This is reinstall-only; no old
record reader, predecessor import or mixed-version recovery is included.

Frontend manifests retain the admitted release, role, module, network, origin and
trust identities. Replace the owned DID/IDL factory artifacts with the admitted
Wire schema, generated TypeScript codecs/runtime, JavaScript ES modules and type
declarations. Compile via a pinned TypeScript compiler using strict ES2022 module
output with declaration emission, no package download during export and no
handwritten transpiler. Pin the compiler through the governed tool owner before
activation. Canic retains authenticated query/update transport; Wire owns bytes.
Composite-query projection must preserve the IC export distinction even though
browser invocation uses query transport.

Hash every generated file in the bundle. Include schema bytes, Wire generator
and runtime identity, compiler identity/options and all input declarations in
cache admission. Retain complete outputs and publish the manifest last using
existing artifact custody. Preserve mismatch refusal and no-follow bounded reads.

Public metrics now has a shared typed `canic_public_metrics` method across
Root, Store and metrics-capable managed/local artifacts. Its direct native
snapshot and application sampler dimensions use Platform Principal. Observatory,
Blob composition and timer/Fleet fixtures call the new method; exhausted public
Metrics variants and metrics-only projections are removed. Cached History now
uses the typed `canic_public_history` method, including Observatory and timer
callers. Its public union selectors, projections and Candid payload derives are
removed; cache and storage semantics remain unchanged. Production release/schema
binding and generated browser delivery below remain unfinished.

## Coherent implementation sequence

1. Qualify published native metadata generation with generic pagination, actual
   principal/domain scalar serializers, enums and bounded runtime collections.
   Exercise Rust/TypeScript exact bytes and explicit unsupported-type diagnostics.
2. Add endpoint-derived method/type registration and exact frame codecs. Migrate
   each owned endpoint and all Rust/host/test callers together, preserving auth,
   pre-copy ingress limits, reply admission, traps and synchronous rollback.
   External management, ledger, provider and standard methods remain Candid.
3. Carry the schema through managed build finalization, role/profile identities,
   Registry bindings, immutable release records and frontend manifest admission.
   Compose the existing Build owner under Tooling #225; no duplicate build cache.
4. Generate/compile browser modules and replace their consumers. Remove exhausted
   owned DID compiler paths only when no caller remains. Platform #38 tracks
   eventual compiler export retirement; external Candid parsing stays required.
5. Qualify installed query/update endpoints against generated clients, exact
   Rust/TS frames, wrong method/direction, malformed/trailing/oversized frames,
   rejected authorization and valid calls afterward. Test manifest/schema/module/
   network tampering, deterministic rebuild, interrupted artifact publication and
   replay using retained exact bytes. Native macOS and real installed-IC evidence
   remain distinct from local Node, native Rust, managed Wasm and PocketIC proofs.

No step alone completes #613. A partly migrated working tree is not releasable;
keep the issue open until the entire consumer path is on the default branch.
Do not add a dual-codec compatibility period to make an incomplete cut look ready.

## Inspected implementation

- `crates/canic-host/src/frontend/ops/mod.rs`: sealed Wire schema admission and
  bundle creation; `ops/compiler/mod.rs` compiles exact retained browser sources
  through shared artifact custody. `ops/bundle/mod.rs` owns files and hashes.
- `crates/canic-host/src/protocol_binding/{mod.rs,release/mod.rs}`: current exact
  Candid/profile admission. Release application/infrastructure persistence owns
  the corresponding immutable hashes.
- `crates/canic-macros/src/endpoint/{mod.rs,expand/mod.rs}`: ordinary Candid
  endpoints and existing raw entrypoints. Raw capability alone supplies no schema.
- `crates/canic-contracts/src/dto/wire/`: capability-selected Candid unions, not
  IC Wire schemas; `dto/page/mod.rs` supplies generic dynamic pagination.
- `crates/canic-contracts/src/ids/network/mod.rs`: custom canonical-text identity.

Existing Blob Wire contracts remain upstream-owned. Do not copy their schemas
or reinterpret their external Candid GC callbacks as owned Canic protocols.
