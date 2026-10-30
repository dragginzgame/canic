# Canic IcyDB adapter

The adapter owns native canister exports, caller/controller context, secure boot
entropy acquisition and lifecycle/watchdog scheduling. Consumers supply their
actual `icydb` facade binding to `start!`; emitted code is checked against that
binding. The adapter has no runtime IcyDB or Memory dependency, so it cannot create
a parallel allocator or duplicate the application's native database types.
Database semantics, accepted schema authority, retained state and precommit reply
admission remain with IcyDB. Native methods have no Candid envelope. Published
Platform owns runtime calls, the shared executor and raw IC0 boundary. The
external management `raw_rand` call retains its Candid protocol through Platform,
with bounded reply admission and the existing single-flight/cancellation policy.
Do not add a direct CDK or a second timer/memory runtime to consuming canisters.
Generated caller context uses Platform's owned Principal, matching IcyDB's
current boundary; external Candid identities stay within their protocol adapter.

## Application Wire methods

`wire_endpoint!(query, "method", CONTRACT_ID, { /* bounded native codec body */ })`
mounts an application method through the same export/executor owner as database
methods. Use `update` for updates. The consumer supplies its current contract ID
and bounded codec; the adapter emits its raw export and artifact identity.

Use `serve_raw` for synchronous handlers. An asynchronous handler is spawned in
the entrypoint's SDK executor and awaits `serve_raw_async`; both share admission
and reply handling. Encoding failure traps the current message. Effects already
committed before an await require the application's own recovery contract.

## Endpoint contracts

`canic_icydb::wire::Endpoint` owns the selected IcyDB method names and frame
identities. One declaration generates name/ID lookup in both directions; native
request and reply payloads remain database contracts. Envelope operations take
`ic_wire::Limits` from the database's existing resource policy. Generated mounts
carry `icydb::wire::limits` and its request/reply ceilings into those operations.
Host callers use the same owner and policy:

```rust,ignore
let bytes = canic_icydb::wire::Endpoint::Query.encode_request(
    sql,
    icydb::wire::limits(icydb::wire::MAX_REQUEST_BYTES),
)?;
```

Lifecycle argument envelopes use `canic_icydb::wire::encode_init_arguments` and
`encode_post_upgrade_arguments` with the same native request limits. Retain exact
frames for retries; decoding does not establish byte identity or authorization.

## Managed Canic composition

Depend on `canic-icydb` and the matching Canic family. Select managed mounting
explicitly with `canic_icydb::mount! { managed; /* declarations */ }`.
Managed methods use Canic's bounded raw endpoint adapter, including
startup/access admission and dispatch accounting; database controller/read guards
remain in the generated handler. Standalone mounting omits `managed;`.
This choice is local to the canister,
so Cargo feature unification cannot change another canister's exports. The adapter
library has no Canic runtime dependency; the managed consumer supplies Canic.

Compose the database's `icydb_memory_requests()` with `canic::memory::framework()`
and the application's other requests. Select one named pool provider with all
component grants. Include `icydb::db::prepare_memory_bootstrap` in the host's
`MemoryBootstrapAdmission` callback, preserving its typed error as
`MemoryRegistryError::Admission` and selecting a policy identity that covers the
actual database admission contract. Do not open a database in that callback.

```rust,ignore
canic_icydb::start!(icydb; participant,
    host = canic::start,
    runtime = canic::memory::runtime::shared_runtime,
    memory = application_memory(),
);
```

This is the sole host lifecycle invocation. It supplies both synchronous hooks;
applications do not call hidden participant functions or add another `start!`.
Canic commits memory first, then the adapter binds the exact committed instance
and invokes database startup, before deferred application hooks. The runtime
getter refuses before commitment and never constructs another allocator. Repeated
Canic bootstrap verifies the original declarations, grants and policy through
Memory's own binding check, even while a component holds the shared instance.

Partial participant configuration is a compile error. The standalone composed
callback form also requires both init and post-upgrade declarations. Database
restoration and actual application acceptance remain downstream-owned; the
framework probe tests host ordering, binding and same-release restoration without
an IcyDB dependency. Application Wire methods on a managed host use
`canic::query`, `canic::update` or the bounded raw attributes directly, with their
owned schema projections; `wire_endpoint!` is the standalone export helper.

## Metrics recording and exposure

The coordinated always-on IcyDB recorder retains bounded heap measurements even
when an application mounts no metrics endpoint. IC Metrics supplies arithmetic,
bounded storage and reset bookkeeping; IcyDB supplies database attribution,
replicated-execution admission, capacity policy and report types. The adapter
only exposes explicitly selected `icydb_metrics(authorization = controller)` or
`icydb_metrics(authorization = public)`, and controller-only `icydb_metrics_reset`.
No endpoint is mounted automatically. Recording does not enable history sampling,
add a timer or turn diagnostic counters into authoritative journal debt.

The shared [adapter reporting contract and ownership table](../../docs/features/runtime/public-observability.md#adapter-reporting-contract)
define the configuration/macro boundary. For `mount! { managed; ... }`, the role's existing Canic configuration is the
reporting ceiling:

```toml
[roles.my_role.observability]
metrics = false
```

This removes both `icydb_metrics` and `icydb_metrics_reset` from the compiled
managed surface, including their metadata and update-limit registration, even
when the mount declares them. `metrics` defaults to true; it permits the explicit
declarations without adding endpoints or changing their authorization. Rebuild
after configuration changes. `history` is independent: history sampling does
not re-enable these endpoints, and disabling reporting does not stop upstream
recording or reset stored observations. Standalone mounts have no Canic role
policy and retain their explicit endpoint declarations.

The adapter consumes the application's report and handler types without owning
their storage representation. Heap versus stable retention, reset continuity and
overflow quality belong to the upstream recorder and its storage adapter. Moving
measurements to stable storage must not change endpoint authorization or create
another sampler. The current shared bounded table is heap-only; mounting does
not promise persistence across upgrades.

Canic's `metrics_public` and `metrics_controller` integration targets compile the
real mounts against framework-owned reports composed from IC Metrics counters
and aggregates. They check typed Wire envelopes, native schema validity and
bounded decode refusal.
They require no IcyDB engine dependency and join ordinary qualification through
the existing test inventory. They do not simulate IC caller/controller checks;
installed authorization and actual database composition remain downstream proof.

This adapter source is prepared for the coordinated IcyDB hard cut that removes
its optional recording feature. Supplier publication and downstream installed
application qualification remain separate from this source change.
