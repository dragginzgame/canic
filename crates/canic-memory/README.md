# canic-memory

Shared stable-memory utilities extracted from the Canic toolkit. This crate can be used on its own by any IC canister crate (including non-Canic projects) to:

- manage stable-memory segments via a shared `MemoryManager`
- declare and validate per-crate memory ID ranges
- register stable structures declaratively with macros
- force eager initialization of thread-local statics that allocate memory

It depends on `canic-types` (bounded types), `canic-utils` (time/serialize helpers), `canic-macros` (storable helpers), and `canic-cdk` (IC stable-structures glue). There is **no dependency on the `canic` crate**.

## Modules

- `manager` — thread-local `MemoryManager<DefaultMemoryImpl>` used by all helpers.
- `registry` — range reservation + ID registry with pending queues for macro-driven registration.
- `ops` — helper to flush pending reservations/registrations into the registry during startup.
- `runtime` — eager TLS initialization helper.
- `macros` — `ic_memory!`, `ic_memory_range!`, `eager_static!`, `eager_init!`.

## Quick start

Add the crate to your `Cargo.toml`:

```toml
canic-memory = { workspace = true }
```

### Reserve a range and declare a memory slot

```rust
// Reserve IDs 10–19 for this crate (usually in a module's init or ctor).
canic_memory::ic_memory_range!(10, 19);

// Declare a stable-memory slot at ID 10 and wrap it in a stable BTreeMap.
use canic_memory::ic_memory;
use canic_memory::cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory};
use std::cell::RefCell;

thread_local! {
    static USERS: RefCell<BTreeMap<u64, u64, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(BTreeMap::init(ic_memory!(Users, 10)));
}
```

### Flush pending registrations during startup

Call the ops helper once during your init/post-upgrade flow to validate ranges and IDs and apply any pending registrations queued by macros:

```rust
use canic_memory::ops::MemoryRegistryOps;

fn init_memory() {
    // Optionally reserve an initial range for this crate before flushing queues.
    // Pass `None` if you reserve exclusively via `ic_memory_range!` calls.
    MemoryRegistryOps::init_memory(Some((env!("CARGO_PKG_NAME"), 10, 19))).unwrap();
}
```

`init_memory` will:
1) reserve the optional initial range,
2) apply all pending range reservations,
3) apply all pending ID registrations (sorted),
4) return a summary of ranges/entries for logging or inspection.

### Eagerly initialize thread-locals that allocate memory

When you wrap stable structures in `thread_local!`, make sure they initialize deterministically before your canister entrypoints execute:

```rust
use canic_memory::{eager_init, eager_static, runtime::init_eager_tls};
use std::cell::RefCell;

eager_static! {
    static CACHE: RefCell<u32> = const { RefCell::new(0) };
}

eager_init!({
    // any one-time setup before entrypoints (optional)
});

fn init() {
    // force eager TLS initialization first
    init_eager_tls();
    // then flush memory registrations
    canic_memory::ops::MemoryRegistryOps::init_memory(None).unwrap();
}
```

## Error handling

The registry surfaces `MemoryRegistryError` for:
- duplicate ranges, overlapping ranges, invalid range (start > end)
- registration outside the crate's reserved ranges
- conflicting registrations on an ID with a different label
- missing range for the crate

Handle these at init time so your canister fails fast on invalid memory layout.

## Testing helpers

`registry::reset_for_tests()` clears the registry and pending queues to keep unit tests isolated. Example:

```rust
#[test]
fn reserves_and_registers() {
    canic_memory::registry::reset_for_tests();
    canic_memory::ops::MemoryRegistryOps::init_memory(Some(("my_crate", 1, 2))).unwrap();
    canic_memory::registry::MemoryRegistry::register(1, "my_crate", "Slot").unwrap();
}
```

## Notes

- The macros automatically namespace memory IDs by crate (`CARGO_PKG_NAME`) when validating ranges.
- If you don't want an initial range, omit it and rely solely on `ic_memory_range!` calls before `init_memory`.
- Consumers outside Canic can import only `canic-memory` plus `canic-types`/`canic-utils`/`canic-macros` and `canic-cdk`; the rest of the stack is optional.
