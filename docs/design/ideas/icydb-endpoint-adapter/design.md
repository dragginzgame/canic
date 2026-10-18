# Candid-free IcyDB endpoint composition

Status: framework raw entrypoint prerequisite implemented; database adapter outstanding.
Implementation and follow-up tracking belong to
[Canic #590](https://github.com/dragginzgame/canic/issues/590) and
[IcyDB #373](https://github.com/dragginzgame/icydb/issues/373).
The framework fixture qualifies raw dispatch; it does not qualify database composition.

## Need and evidence

IcyDB is removing its Candid-aware contracts and proposes moving generated IC
exports into a Canic integration. The ownership precedent is
[`canic-blob-service`](../../../../integrations/blob-service/README.md): the
service owns operations and native contracts while the adapter supplies host
endpoint and lifecycle composition.

Canic main inspected for this proposal is
`53f1f22d2e23a16c5566b8590f3b0a2a643b6d84`. Its endpoint adapter explicitly
generates Candid decoding, reply encoding and declaration attributes. Blob's
current endpoint macros therefore cannot mount IcyDB's IC Wire ABI unchanged.
The local Canic runtime updates and IcyDB Wire migration are separate unfinished
inputs, not changes included in this proposal.

## Proposed ownership and dependency boundary

Add a thin `canic-icydb` package depending on Canic and IcyDB. IcyDB remains
independent of Canic. The integration owns public `icydb_*` exports, caller
context, endpoint authorization, IC Wire method envelopes and transport errors.
It delegates immediately to native IcyDB capabilities. Database planning,
accepted-schema authority, storage, errors, recovery and precommit resource
admission stay with IcyDB.

Use the ordinary `crates/canic-icydb/` layout in the root workspace and inherit
its catalog, version and lockfile. Blob's retained `integrations/` location is an
existing exception, not authority for another nested workspace. Publication
order, package inventory and selected validation must include the adapter when
it is implemented; this design does not select a new package version.

Canic currently forbids IcyDB dependencies in framework validation. Implementation
must explicitly reconcile that policy for the integration package, preserving
the rule for core/host code and existing framework fixtures. The adapter's own
dependency is the proposed narrow exception; actual application/database
composition acceptance remains downstream. Do not add database-specific behavior
to Canic's endpoint engine or infrastructure graph.

## Framework entrypoint prerequisite

Provide one explicit protocol-neutral integration point in the existing endpoint
machinery. It must retain startup admission, declared authorization, dispatch
instrumentation and exact IC export names/modes while letting the adapter own
bounded decoding and replies. Select the smallest concrete API after reviewing
the native IcyDB binding. Avoid a general codec registry or a codec selector in
database execution. Canic's ordinary Candid endpoints keep their current owner.

The Wire path must admit request bytes before allocating/copying, validate
method/direction and canonical payloads, preserve node/depth/native allocation
limits, and omit these methods from Candid discovery. One adapter contract owns
method IDs, DTO projection, envelope bounds and artifact declarations for both
server and callers. Do not manufacture Candid types for native database values.

Authorization denial must precede database effects. SQL read guards and current
controller defaults remain explicit; extraction must not silently open a method
or replace public read admission with trusted execution.

After a synchronous mutating handler, unexpected response-encoding failure must
trap rather than merely reject, rolling back the current message. Typed database
errors remain valid encoded replies. The real database rollback/retry test is
owned by [IcyDB #371](https://github.com/dragginzgame/icydb/issues/371); Canic
qualifies the mechanism with a framework-owned bounded fixture. A trap cannot
undo previously committed messages or effects across await boundaries.

## Lifecycle and memory composition

The application mounts selected endpoints and composes IcyDB's synchronous
init/restoration participant into its sole Canic lifecycle. Canic admits the
combined explicit memory requirements first; IcyDB then restores its database
invariants before deferred work. The adapter does not create another runtime,
install a database automatically or become an async lifecycle owner.

Provider ergonomics stay with [#514](https://github.com/dragginzgame/canic/issues/514)
and explicit memory composition with
[#527](https://github.com/dragginzgame/canic/issues/527). Reuse those contracts.

## Delivery and acceptance

1. Qualify the protocol-neutral endpoint capability with Canic-owned fixtures:
   positive/denied admission, startup refusal, exact modes/names, malformed and
   noncanonical request refusal, bounds before copying, no Candid declaration,
   encoding-failure rollback and valid calls after refusal.
2. Implement the thin adapter against IcyDB's deliberate native binding; retain
   selected feature/fixture gates, request execution scope and a single contract.
3. Qualify a real downstream database/application canister and move its callers,
   artifact declarations and lifecycle composition together. Delete superseded
   IcyDB exports in the paired cut; keep no compatibility aliases or fallback.

The real consumer must cover the 12 selected IcyDB method families, SQL RETURNING
refusal before commit, exact retry controls and same-contract restoration with
one memory owner. This does not promise cross-release Canic upgrades. Browser
client qualification remains separately tracked in
[IcyDB #372](https://github.com/dragginzgame/icydb/issues/372).

Record matched raw Wasm, available IC cycles/instructions, dependency changes and
source/type/documentation deltas. Native elapsed time is not a performance metric.
No performance result is claimed by this design.

The simplest alternative is retaining IcyDB-owned exports. The intended
state-space delta is transferring that endpoint owner to one integration, with
no new service state, database format, host lifecycle, codec fallback or
application configuration mode.

## Implemented framework boundary

`canic_raw_query` and `canic_raw_update` accept synchronous `fn()` handlers.
They require explicit access and `payload(max_bytes = N)`; handlers own bounded
byte decoding and replies. The existing access plan supplies startup/authorization
and instrumentation, and the common IC entrypoint supplies query/update executor
context. Asynchronous raw handlers and decoded-argument guards are outside this
prerequisite. The `canic:raw` section records exact names/modes for the Host's
combined export check without manufacturing Candid declarations.

The framework probe covers admitted/denied callers, malformed/trailing/oversized
commands, exact query/update bytes, typed application failures and synchronous
trap rollback. Codec-specific identity/canonicality, installed startup refusal,
actual database operations and single-runtime lifecycle composition remain
adapter acceptance. The committed Canic graph selects Memory 0.44; the pending
IcyDB native graph selects exclusive Memory 0.46. A coordinated published native
binding and compatible framework memory owner are required before the downstream
adapter can consume that graph. Sibling dirty source is not an integration input.
