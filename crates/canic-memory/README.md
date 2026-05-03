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
- `MemoryApi` for runtime-selected stable-memory IDs.
- `eager_static!` and `eager_init!` for deterministic startup initialization.
- `impl_storable_bounded!` and `impl_storable_unbounded!` for CBOR-backed
  `Storable` implementations.
- A `canic_cdk` re-export at `canic_memory::cdk`.

## Install

Inside the Canic workspace, use the workspace dependency:

```toml
canic-memory = { workspace = true }
```

From another crate, depend on the published crate:

```toml
canic-memory = "0.29"
```

## Quick Start

Declare stable structures with `eager_static!` so they are touched during
startup, not lazily during the first endpoint call.

```rust
use canic_memory::cdk::structures::{
    BTreeMap, DefaultMemoryImpl,
    memory::VirtualMemory,
};
use canic_memory::{eager_static, ic_memory};
use std::cell::RefCell;

struct Users;

eager_static! {
    pub static USERS: RefCell<BTreeMap<u64, u64, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(Users, 10)));
}
```

Bootstrap memory during canister startup before any endpoint uses the stable
structures:

```rust
use canic_memory::api::MemoryApi;

fn init_memory() {
    MemoryApi::bootstrap_owner_range(env!("CARGO_PKG_NAME"), 10, 19)
        .expect("stable memory layout must be valid");
}
```

`bootstrap_owner_range(...)` performs the standalone startup sequence:

1. Touch every `eager_static!` thread-local.
2. Run every registered `eager_init!` body.
3. Reserve the caller's owner range.
4. Flush pending `ic_memory_range!` and `ic_memory!` registrations.

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
- ID `255`, which is reserved for stable-structures internals

Exact duplicate range reservations for the same crate are allowed so init and
post-upgrade can share the same bootstrap path.

## Runtime-Selected Slots

Use `MemoryApi` when the memory ID is chosen dynamically and `ic_memory!` is not
a good fit.

```rust
use canic_memory::api::MemoryApi;

fn open_commit_marker(memory_id: u8) {
    MemoryApi::bootstrap_owner_range("my_crate", 10, 19)
        .expect("stable memory layout must be valid");

    let memory = MemoryApi::register(memory_id, "my_crate", "CommitMarker")
        .expect("commit marker slot must be in range");

    let _ = memory;
}
```

`MemoryApi::register(...)` is idempotent for the same owner and label, but it
returns `MemoryRegistryError::DuplicateId` if the same ID is reused for a
different registration.

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

- `MemoryRegistry::export_range_entries()`
- `MemoryRegistry::export_ids_by_range()`

Prefer `MemoryApi` for normal supported reads.

## Storable Helpers

The storable macros implement `ic-stable-structures` `Storable` with Canic's
shared CBOR serializer.

```rust
use canic_memory::impl_storable_bounded;
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
    MemoryApi::bootstrap_owner_range(env!("CARGO_PKG_NAME"), 10, 19)
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

Accessing an `ic_memory!` slot on `wasm32` before bootstrap will panic with a
message pointing back to memory bootstrap. This is intentional: stable memory
layout problems should fail during lifecycle startup, not during a user call.

## Testing

Unit tests that touch the registry can reset global state with
`registry::reset_for_tests()`:

```rust
#[test]
fn reserves_and_registers() {
    canic_memory::registry::reset_for_tests();
    canic_memory::api::MemoryApi::bootstrap_owner_range("my_crate", 1, 2)
        .expect("bootstrap registry");
    canic_memory::registry::MemoryRegistry::register(1, "my_crate", "Slot")
        .expect("register slot");
}
```

`reset_for_tests()` is only available under `cfg(test)`.

## Module Map

- `api` - supported runtime API for bootstrapping, registration, and reads.
- `manager` - shared thread-local memory manager.
- `registry` - range reservation, ID registration, pending queues, and errors.
- `runtime` - eager TLS execution and registry startup glue.
- `macros` - exported memory, runtime, and storable macros.
- `serialize` - CBOR serialization helpers used by storable macros.

## Notes

- Memory IDs are `u8` values. Application code may use `0..=254`; `255` is
  reserved internally.
- `ic_memory!` labels are type paths. Define a small marker type, such as
  `struct Users;`, for each slot.
- Consumers outside Canic can import only `canic-memory` plus `canic-cdk`; the
  rest of the Canic stack is optional.
