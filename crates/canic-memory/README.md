# canic-memory

Shared stable-memory helpers you can drop into any IC canister, even if you are not using the rest of Canic. It keeps you honest about which IDs you use and makes TLS-backed stable structures initialize in a predictable order.

What you get:
- Reserve and validate a per-crate stable-memory ID range.
- One place to register stable structures (`ic_memory!` + registry).
- A supported runtime API for dynamic memory registration/opening (`MemoryApi`).
- Eager TLS init so thread-locals that allocate memory are ready before entrypoints.
- Zero dependency on the `canic` crate.

Sample boot logs when everything is wired correctly:
```
17:27:24.796 [...] [Init] 🔧 --------------------- 'canic v0.6.x -----------------------
17:27:24.796 [...] [Init] 🏁 init: root (Prime)
17:27:24.796 [...] [Memory] 💾 memory.range: canic-core [5-30] (15/26 slots used)
17:27:24.796 [...] [Wasm] 📄 registry.insert: app (1013.10 KB)
...
17:27:26.879 [...] [CanisterLifecycle] ⚡ create_canister: nssc3-p7777-77777-aaawa-cai (5.000 TC)
17:27:27.549 [...] [Init] 🏁 init: app
17:27:27.549 [...] [Memory] 💾 memory.range: canic-core [5-30] (15/26 slots used)
```

## Modules

- `manager` — thread-local `MemoryManager<DefaultMemoryImpl>` used by all helpers.
- `registry` — range reservation + ID registry with pending queues for macro-driven registration.
- `runtime` — eager TLS initialization and registry startup helpers.
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

### Bootstrap and register one runtime-selected memory slot

Use `MemoryApi` when the memory ID is chosen at runtime and a macro is not a good fit.

```rust
use canic_memory::api::MemoryApi;

fn init_dynamic_slot(memory_id: u8) {
    let _ = MemoryApi::bootstrap_registry("my_crate", 10, 19).unwrap();
    let memory = MemoryApi::register_memory(memory_id, "my_crate", "CommitMarker").unwrap();

    // `memory` is a normal `VirtualMemory<DefaultMemoryImpl>` handle.
    let _ = memory;
}
```

### Inspect one runtime-selected memory slot

Use `inspect_memory(...)` when validation code needs to confirm who owns an id
and whether that slot already has a registered label.

```rust
use canic_memory::api::MemoryApi;

fn validate_slot(memory_id: u8) {
    if let Some(info) = MemoryApi::inspect_memory(memory_id) {
        assert_eq!(info.owner, "my_crate");
        let _label = info.label;
        let _range = info.range;
    }
}
```

### Flush pending registrations during startup

Call the runtime registry initializer once during init/post-upgrade to validate ranges and apply any pending registrations queued by macros. Repeated calls are allowed when the initial range is identical; conflicts return a `MemoryRegistryError`.

```rust
use canic_memory::runtime::registry::MemoryRegistryRuntime;

fn init_memory() {
    // Optionally reserve an initial range for this crate before flushing queues.
    // Pass `None` if you reserve exclusively via `ic_memory_range!` calls.
    MemoryRegistryRuntime::init(Some((env!("CARGO_PKG_NAME"), 10, 19))).unwrap();
}
```

`init_memory` will:
1) reserve the optional initial range,
2) apply all pending range reservations,
3) apply all pending ID registrations (sorted),
4) return a summary of ranges/entries for logging or inspection.

If you want the same flow from the supported public API surface, use:

```rust
use canic_memory::api::MemoryApi;

fn init_memory() {
    let _ = MemoryApi::bootstrap_registry("my_crate", 10, 19).unwrap();
}
```

### Eagerly initialize thread-locals that allocate memory

Why bother? `thread_local!` values are lazy. If a stable `BTreeMap` (or similar) spins up the first time an endpoint is called, you get:
- unpredictable init order (especially across upgrades),
- memory allocations happening under a user call instead of during init,
- possible panics if the registry/ranges were not flushed yet.

`eager_static!` and `eager_init!` make TLS setup a deliberate part of startup instead of a hidden first-use side effect.

If you are using the full Canic facade (`canic::start!` / `canic::start_root!`), you do not need to call anything extra: Canic runs eager TLS, executes registered `eager_init!` blocks, and then flushes the memory registry during synchronous lifecycle bootstrap. This work happens during real lifecycle execution, so debug-only Candid extraction can reuse the same artifact without a separate eager-init build variant.

If you are using `canic-memory` standalone without Canic lifecycle wiring, the startup order is: `init_eager_tls()` → run `eager_init!` work → flush the registry. After that, every endpoint starts with the same, prebuilt memory layout.

```rust
use canic_memory::{eager_init, eager_static, runtime::init_eager_tls};
use canic_memory::runtime::registry::MemoryRegistryRuntime;
use std::cell::RefCell;

eager_static! {
    static CACHE: RefCell<u32> = const { RefCell::new(0) };
}

eager_init!({
    // any one-time setup before entrypoints (optional)
});

fn init() {
    // standalone canisters should force eager TLS initialization first
    init_eager_tls();
    // then flush memory registrations
    MemoryRegistryRuntime::init(None).unwrap();
}
```

## Error handling

The registry surfaces `MemoryRegistryError` for:
- overlapping ranges or duplicate ID registrations
- invalid range (start > end)
- registration outside the crate's reserved ranges or owned by another crate

Handle these at init time so your canister fails fast on invalid memory layout.

## Testing helpers

`registry::reset_for_tests()` clears the registry and pending queues to keep unit tests isolated. Example:

```rust
#[test]
fn reserves_and_registers() {
    canic_memory::registry::reset_for_tests();
    canic_memory::runtime::registry::MemoryRegistryRuntime::init(Some(("my_crate", 1, 2))).unwrap();
    canic_memory::registry::MemoryRegistry::register(1, "my_crate", "Slot").unwrap();
}
```

## Registry introspection

For diagnostics, the registry can provide:
- ranges with owners via `MemoryRegistry::export_range_entries()`
- registered IDs grouped by range via `MemoryRegistry::export_ids_by_range()`

These helpers are intended for debugging and tests, not as a stable API contract.
For ordinary read-only runtime validation of one id, prefer `MemoryApi::inspect_memory(...)`.

## Notes

- The macros automatically namespace memory IDs by crate (`CARGO_PKG_NAME`) when validating ranges.
- If you don't want an initial range, omit it and rely solely on `ic_memory_range!` calls before `init_memory`.
- Consumers outside Canic can import only `canic-memory` plus `canic-cdk`; the rest of the stack is optional.
