# Canic IcyDB adapter

The adapter owns native canister exports, caller/controller context, secure boot
entropy acquisition and lifecycle/watchdog scheduling. Consumers supply their
actual `icydb` facade binding to `start!`; emitted code is checked against that
binding. The adapter has no runtime IcyDB or Memory dependency, so it cannot create
a parallel allocator or duplicate the application's native database types.
Database semantics, accepted schema authority, retained state and precommit reply
admission remain with IcyDB. Native methods have no Candid envelope. The upstream
SDK management call used for entropy retains its own Candid protocol.

## Application Wire methods

`wire_endpoint!(query, "method", CONTRACT_ID, { /* bounded native codec body */ })`
mounts an application method through the same export/executor owner as database
methods. Use `update` for updates. The consumer supplies its current contract ID
and bounded codec; the adapter emits its raw export and artifact identity.

Use `serve_raw` for synchronous handlers. An asynchronous handler is spawned in
the entrypoint's SDK executor and awaits `serve_raw_async`; both share admission
and reply handling. Encoding failure traps the current message. Effects already
committed before an await require the application's own recovery contract.

## Framework-managed memory

After the selected Canic and IcyDB releases resolve the same Memory family,
select the host runtime explicitly:

```rust,ignore
canic_icydb::start!(icydb; participant, runtime = canic::memory::runtime::shared_runtime);
canic::start!(
    memory = application_memory(),
    lifecycle_participant(
        init = crate::__icydb_lifecycle_participant::init,
        post_upgrade = crate::__icydb_lifecycle_participant::post_upgrade,
    ),
);
```

`application_memory()` supplies one complete `canic::memory::MemoryBootstrap`:
framework requests plus `icydb_memory_requests()`, one checked pool and composed
pre-commit admission. The generated participant binds Canic's exact committed
runtime before startup; no second manager is constructed. With IcyDB's explicit
participant startup, remove the application-owned pool provider and forwarding
functions. Standalone hosts retain their explicit standalone bootstrap and pool
provider.

This contribution depends on [Canic's shared runtime handoff](https://github.com/dragginzgame/canic/pull/595),
[IcyDB's explicit participant startup](https://github.com/dragginzgame/icydb/pull/376)
and their aligned published Memory family. Its native facade-substitute tests do
not establish installed managed-application acceptance. Complete the application-owned init/upgrade,
restore-before-timer and retained-data checks before deployment.
