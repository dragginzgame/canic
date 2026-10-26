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
