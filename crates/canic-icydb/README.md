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
