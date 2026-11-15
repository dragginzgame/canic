# Versioned Wasm Store

The standalone Store is the first implementation of the
[per-fleet artifact contract](../design/canic-system-design.md#per-fleet-versioned-wasm-store).
It keeps immutable `(name, version)` releases, content hashes and ordered chunks
outside Root. Uploads resume from missing positions; publication checks every
chunk and the complete raw payload. A second label for identical content reuses
bytes. Preparing a new version never removes earlier releases.

Root now supports explicit Store-backed creation and serial upgrades. It retains
immutable references, pin evidence and complete upgrade history in its original
artifact allocation, with no application Wasm bytes. This is not yet wired into
fleet plan/apply: selected-release policy, generated Store topology and existing-
fleet migration remain unimplemented. Existing inline deployments keep their
current limits and contracts.
There is no delete/reset or pin-release endpoint; incomplete uploads keep their
reservation. No deployment approval or application readiness follows from Store
publication. Host remains responsible for structural Wasm inspection; Store
checks raw Wasm framing and byte integrity, not executable validity.

From this checkout, after `make canic` and Wasm toolchain preparation:

```sh
canic artifact build --package canic-wasm-store --output target/wasm-store.wasm
canic artifact inspect target/wasm-store.wasm
```

These commands build and inspect locally. They do not install the Store or open
a browser. `make store-test` runs native and installed PocketIC qualification
after `make qualification-setup`; it uses synthetic cycles and requires local
loopback access. `make store-clippy` and `make store-doc` check Store code;
`make root-clippy` and `make root-doc` cover the management consumer.

All Store endpoints require the current IC controller. `prepare`, `put` and
`publish` are idempotent for exact retained commitments. `status`/`observe` expose
query/replicated progress; `read`/`read_chunk` expose query/replicated published
bytes. `list` returns at most four metadata entries after an exclusive release
cursor. These transport distinctions do not grant fresh installation authority.
`configure` changes admission quotas without reinstalling; `info` reports their
usage. Root controls the Store during autonomous work. The qualification fixture
publishes its inputs before handing Store control to Root; a production upload
and infrastructure-maintenance flow through Root remains part of Host integration.

`pin` retains an exact published manifest for the authenticated caller's operation.
The identity is caller, operation digest and artifact hash; an operation can pin
several artifacts, but cannot change an existing pin's release label. `observe_pin`
returns replicated retention evidence. At most 32 pins per artifact are retained;
exhaustion refuses new obligations without evicting old ones. Pins do not expire
or authorize installation. Metadata schema `CWA2` reads the original `CWA1`
records with no pins, using the same four stable allocations.

`prepare_cache` requires an exact retained pin and prepares its entire bounded
artifact. It observes the cache once, uploads only missing chunks from stable
storage, and verifies every returned hash. Its dispatch deadline is at most five
minutes from admission; caller control and deadline are checked before each
upload. There are no background retries or attached cycles. A cache hit is an
observation at that moment, not a guarantee against subsequent cache clearing.
Repeating a request observes current cache contents before repair. This does not
grant Root permission to replay an uncertain installation.

Root's explicit methods are `stage_stored_child`, `reconcile_stored_child`,
`stage_stored_upgrade`, `reconcile_stored_upgrade` and
`observe_stored_children`. Their passive contracts live in
`canic-management-protocol::stored`. Existing effect approvals retain their old
encoding; each new request binds that approval, child slot and exact Store
reference into a domain-separated operation digest. Store pins are acquired before
child creation or upgrade preparation. The pin callback commits before subsequent
management work. Root retains one cache request deadline and one dispatch; it
installs from Store using the approved ordered chunk list and full module hash.

A lost pin reply is reconstructed by replicated pin observation. Lost cache
completion is reconstructed from Store's actual management cache. Root maintenance
preserves both the existing journals and all Store references. Old complete-custody
and reconciliation methods refuse Store-backed operations rather than omit the
supplier binding. Inline and Store-backed children can coexist; an existing
child cannot switch artifact backends through an upgrade request.

The installed fixtures use actual Root calls for pin acquisition, autonomous
creation and installation, serial upgrades and callback-loss recovery. Executables
larger than the old inline limit are installed with Root's own chunk cache empty.
Other cases cover unpublished releases, expired authority, mixed inline/Store
custody and historical replay. The separate Store fixture clears its cache and
rebuilds it after Store maintenance.

The first Root path deliberately remains blocked if an uncertain cache preparation
has missing chunks at reconciliation. It does not renew the original deadline or
reissue preparation automatically. Cache loss after installation dispatch never
authorizes install replay. An operator-approved repair flow, Store maintenance
through Root, existing-fleet conversion and fleet CLI integration are remaining
gates. The management-cache listing has a bounded 64 KiB reply decoder; an oversized
listing blocks preparation/recovery rather than allocating without a bound.

The initial bounded contract accepts raw artifacts up to 8 MiB in 256 KiB chunks.
The default quota reserves 128 MiB of distinct artifact payloads and 1,024 release
labels, including incomplete uploads. Controller changes may raise those quotas
to 256 MiB and 4,096 labels. Chunk sharing does not reduce the conservative
artifact reservation. Physical backing has a separate 1 GiB limit, including
collection overhead; those admission quotas are not a promise of usable physical
capacity. Exhausted backing traps and rolls back the message. Increasing the
artifact or backing ceiling requires another qualified executable, not a silent
limit bypass. Stable records and independent collections grow incrementally;
Root has no copy of this catalog or its payloads.
