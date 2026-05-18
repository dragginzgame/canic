# canic-memory

`canic-memory` provides stable-memory helpers for Internet Computer canisters. It
can be used on its own, without the rest of Canic, when a crate needs one shared
memory manager, deterministic thread-local initialization, and validation for
stable-memory ID ownership.

The crate currently declares MSRV `1.91.0`. The Canic workspace may build with a
newer pinned toolchain, but downstream crates compiling `canic-memory` from
source should only need Rust `1.91.0` or newer.

## What It Provides

- A shared `MemoryManager<DefaultMemoryImpl>` used by all helpers.
- Per-crate memory ID range reservation and overlap validation.
- `ic_memory!` and `ic_memory_range!` for declarative stable-memory slots.
- `MemoryApi` for declaring startup-selected stable-memory IDs and opening
  validated slots.
- `eager_static!` and `eager_init!` for deterministic startup initialization.
- Compatibility re-exports of `impl_storable_bounded!` and
  `impl_storable_unbounded!` from `canic-cdk`.
- A `canic_cdk` re-export at `canic_memory::cdk`.

## Install

Inside the Canic workspace, use the workspace dependency:

```toml
canic-memory = { workspace = true }
```

From another crate, depend on the published crate:

```toml
canic-memory = "0.38"
```

## Quick Start

Declare stable structures with `eager_static!` so they are touched during
startup, not lazily during the first endpoint call.

```rust
use canic_memory::cdk::structures::{
    BTreeMap, DefaultMemoryImpl,
    memory::VirtualMemory,
};
use canic_memory::{eager_static, ic_memory_key};
use std::cell::RefCell;

struct Users;

eager_static! {
    pub static USERS: RefCell<BTreeMap<u64, u64, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(
            ic_memory_key!("my_app.users.v1", Users, 100),
        ));
}
```

Bootstrap memory during canister startup before any endpoint uses the stable
structures:

```rust
use canic_memory::api::MemoryApi;

fn init_memory() {
    MemoryApi::bootstrap_owner_range(env!("CARGO_PKG_NAME"), 100, 109)
        .expect("stable memory layout must be valid");
}
```

`bootstrap_owner_range(...)` performs the standalone startup sequence:

1. Collect constructor-registered `ic_memory_key!` and `ic_memory!`
   declarations without opening their virtual memories.
2. Run every registered `eager_init!` body so `ic_memory_range!` declarations
   are collected.
3. Reserve the caller's owner range and validate the sealed declaration
   snapshot against the persisted ledger.
4. Touch every `eager_static!` thread-local after validation so stable stores
   can open their already-approved memory handles.

When using the full Canic facade (`canic::start!` or `canic::start_root!`), Canic
runs this lifecycle wiring for you.

## Memory Ranges

Stable-memory IDs are global inside one canister. Reserve a range for each crate
that owns stable structures, then keep that crate's IDs inside the range.

```rust
use canic_memory::{eager_init, ic_memory_range};

eager_init!({
    ic_memory_range!(20, 29);
});
```

Range validation catches:

- overlapping ranges
- `start > end`
- duplicate IDs
- IDs outside the owner's reserved range
- IDs owned by another crate
- historical reuse of an ID or range recorded by the persisted layout ledger
- ID `255`, which is the unallocated-bucket sentinel and is not a usable
  virtual memory ID

Exact duplicate range reservations for the same crate are allowed so init and
post-upgrade can share the same bootstrap path.

`canic-memory` reserves ID `0` for the persisted layout ledger. ID `0` stores
every owner range and memory ID that has been registered through the bootstrap
path, so removed declarations remain historical reservations rather than
becoming silently reusable. Canic framework keys (`canic.*`) must use IDs
`10-99`; downstream application keys must use `100-254`. IDs `0-9` are
reserved for `ic-memory` allocation-governance infrastructure, not application
or Canic framework stores. The full Canic runtime stack currently uses `11-79`
for core runtime stores and `80-85` for control-plane stores.

Bootstrap distinguishes brand-new stable memory from existing state before
mutating the declaration snapshot. Empty stable memory may initialize the
genesis ledger. Foreign or corrupt raw stable memory fails closed, and existing
`MemoryManager` state must already contain a valid ID `0` Canic ABI ledger.

## Runtime-Selected Slots

Use `MemoryApi` when the memory ID is chosen during startup and
`ic_memory_key!` is not a good fit. Declaration and opening are separate:
declare the slot before bootstrap, then open it only after bootstrap validates
the sealed declaration snapshot. Endpoint code must not call declaration APIs
as a dynamic allocator.

```rust
use canic_memory::api::MemoryApi;

fn declare_commit_marker(memory_id: u8) {
    MemoryApi::declare_with_key(
        memory_id,
        "my_crate",
        "CommitMarker",
        "my_crate.commit_marker.v1",
    )
        .expect("commit marker declaration must be valid");
}

fn bootstrap_memory() {
    MemoryApi::bootstrap_owner_range("my_crate", 100, 109)
        .expect("stable memory layout must be valid");
}

fn open_commit_marker(memory_id: u8) {
    let _memory = MemoryApi::register_with_key(
        memory_id,
        "my_crate",
        "CommitMarker",
        "my_crate.commit_marker.v1",
    )
        .expect("commit marker slot must be in range");
}
```

`MemoryApi::declare_with_key(...)` is the allocation claim. It is accepted only
before bootstrap seals the runtime declaration snapshot and it does not open the
underlying virtual memory. `MemoryApi::register_with_key(...)` opens an
already-validated slot after bootstrap has succeeded; it is not a dynamic
allocation API and endpoint code must not use it to introduce new declarations.
Reusing the same ID for a different stable key or moving the same stable key to
another ID remains fatal. Owner and label metadata may change across refactors;
the stable key must not.

If a store wants diagnostic schema metadata in the ledger, use
`declare_with_key_metadata(...)`:

```rust
MemoryApi::declare_with_key_metadata(
    memory_id,
    "my_crate",
    "CommitMarker",
    "my_crate.commit_marker.v1",
    Some(1),
    Some("sha256:abc123"),
)
    .expect("commit marker declaration must be valid");
```

Schema metadata is optional and informational in `0.38`. It does not change
allocation ownership and is not fatal during bootstrap when it drifts for the
same stable key and ID. The ledger records the latest values and preserves
declaration history for diagnostics. When present, `schema_version` must be
greater than zero. `schema_fingerprint` must be a non-empty ASCII string, must
not contain ASCII control characters, and must fit within 256 bytes.

## Registry Introspection

Use the supported `MemoryApi` reads for validation, diagnostics, or endpoint
responses:

```rust
use canic_memory::api::MemoryApi;

fn validate_slots(memory_id: u8) {
    if let Some(info) = MemoryApi::inspect(memory_id) {
        assert_eq!(info.owner, "my_crate");
        let _range = info.range;
        let _label = info.label;
    }

    let all_registered = MemoryApi::registered();
    let owned = MemoryApi::registered_for_owner("my_crate");
    let marker = MemoryApi::find("my_crate", "CommitMarker");

    let _ = (all_registered, owned, marker);
}
```

Lower-level registry snapshot helpers also exist for debugging and tests:

- `MemoryApi::ledger_snapshot()` for a fallible persisted-ledger diagnostic
  read, including the canonical `0-9` `ic-memory`, `10-99` Canic, and
  `100-254` application authority ranges. On wasm this path decodes only the
  ID `0` ABI ledger from raw stable memory so operators can inspect ledger
  state without opening application or framework stores.
- `MemoryRegistry::export_range_entries()`
- `MemoryRegistry::export_ids_by_range()`

Prefer `MemoryApi` for normal supported reads.

## Storable Helpers

The storable macros implement `ic-stable-structures` `Storable` with Canic's
shared CBOR serializer. They now live in `canic-cdk`; `canic-memory` re-exports
them only as compatibility glue while this crate is being retired.

```rust
use canic_cdk::impl_storable_bounded;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct UserRecord {
    id: u64,
    name: String,
}

impl_storable_bounded!(UserRecord, 512, false);
```

Use `impl_storable_bounded!(Type, max_size, is_fixed_size)` when the serialized
size has a known bound. Use `impl_storable_unbounded!(Type)` only for data that
is expected to grow beyond a practical fixed bound.

## Standalone Lifecycle

For standalone canisters, call one of the bootstrap helpers from init and
post-upgrade before handling user calls:

```rust
use canic_memory::api::MemoryApi;

fn bootstrap_memory() {
    MemoryApi::bootstrap_owner_range(env!("CARGO_PKG_NAME"), 100, 109)
        .expect("stable memory layout must be valid");
}
```

If all owner ranges are already queued through `ic_memory_range!`, and the
caller does not need to reserve an additional initial range, use:

```rust
use canic_memory::api::MemoryApi;

fn bootstrap_memory() {
    MemoryApi::bootstrap_pending().expect("stable memory layout must be valid");
}
```

Accessing an `ic_memory!` or `ic_memory_key!` slot before bootstrap will panic
with a message pointing back to memory bootstrap. This is intentional: stable
memory layout problems should fail during lifecycle startup, not during a user
call.

## Testing

Unit tests that touch the registry can reset global state with
`registry::reset_for_tests()`:

```rust
#[test]
fn reserves_and_registers() {
    canic_memory::registry::reset_for_tests();
    canic_memory::api::MemoryApi::bootstrap_owner_range("my_crate", 100, 101)
        .expect("bootstrap registry");
    canic_memory::registry::MemoryRegistry::register_with_key(
        100,
        "my_crate",
        "Slot",
        "my_crate.slot.v1",
    )
        .expect("register slot");
}
```

`reset_for_tests()` is only available under `cfg(test)`.

## Module Map

- `api` - supported runtime API for bootstrapping, registration, and reads.
- `manager` - shared thread-local memory manager.
- `registry` - range reservation, ID registration, pending queues, and errors.
- `runtime` - eager TLS execution and registry startup glue.
- `macros` - exported memory and runtime macros.
- `serialize` - compatibility re-export of `canic_cdk::serialize`.

## Notes

- Memory IDs are `u8` values. `ic-memory` governance uses `0-9`, Canic
  framework code uses `10-99`, application code uses `100-254`, and ID `255`
  is the unallocated-bucket sentinel and is permanently invalid as a virtual
  memory ID.
- Prefer `ic_memory_key!` for every Canic-managed memory. The stable key is the
  ABI identity and should not be renamed when packages, modules, or marker types
  move. `ic_memory!` remains available for standalone explicit-ID users outside
  the Canic runtime bootstrap contract.
- Consumers outside Canic can import only `canic-memory` plus `canic-cdk`; the
  rest of the Canic stack is optional.
