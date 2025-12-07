# canic-utils

Small, deterministic utilities that Canic (and your canisters) rely on without pulling in the whole stack. These helpers are WASM-safe, avoid std-only randomness, and aim to be predictable across replicas.

What you get:
- MiniCBOR codecs with clear errors (`serialize` / `deserialize`).
- Fast xxHash3 helpers for non-crypto hashing.
- Deterministic RNG (`tinyrand` + shared seed) for tests/sampling.
- Time helpers (`now_*`) that work in WASM and native.
- String casing helpers (snake/constant/title/etc.).
- Perf counter helper (`PERF_LAST`) for lightweight timing.
- WASM hashing (module hash) helpers.

Quick hits
```rust
use canic_utils::{hash, rand, serialize, time};

let digest = hash::hash_u64(b"tenant-123");      // fast sharding key
let now = time::now_secs();                      // UNIX seconds (WASM-friendly)
let bytes = serialize::serialize(&["ok"]).?;     // CBOR-encode with clear errors
let sample = rand::next_u64();                   // non-crypto RNG for tests
```

Determinism notes
- Hashing: xxHash3 is **not** cryptographic. Use it for sharding, cache keys, and fingerprints—not for signatures or certified data.
- RNG: seeded from `now_nanos()` once per process (shared `LazyLock<Mutex<StdRand>>`). Good for tests and sampling; not for secrets.
- Time: `now_*` wraps `api::time()` on WASM; `SystemTime` on native. Casts are clamped to u64 with truncation allowed where noted.

Error handling
- `serialize`/`deserialize` return `SerializeError` instead of panicking. Downstream macros (`impl_storable_*`) panic with contextual messages so corrupt stable data is obvious in logs.

Perf helper
```rust
use canic_utils::perf::PERF_LAST;
use canic_utils::cdk::api::performance_counter;

let before = PERF_LAST.with(|p| *p.borrow());
let now = performance_counter(1);
let elapsed = now.saturating_sub(before);
PERF_LAST.with(|p| *p.borrow_mut() = now);
```

Casing helper
```rust
use canic_utils::case::{Case, Casing};
assert_eq!("hello_world".to_case(Case::Constant), "HELLO_WORLD");
assert!("Title Case".is_case(Case::Title));
```

WASM helper
```rust
use canic_utils::wasm::get_wasm_hash;
let hash = get_wasm_hash(&[0u8; 16]); // 32-byte SHA-256 of the module
```

Testing
- RNG tests cover determinism and basic entropy.
- Time helpers sanity-check epoch values.
- Storable macros have round-trip + corrupt-data tests to ensure failures are loud.

Layout
```
canic-utils/
├─ src/
│  ├─ case/          # casing helpers (snake, constant, title)
│  ├─ format.rs      # tiny fmt helpers
│  ├─ hash.rs        # xxHash3 helpers
│  ├─ instructions.rs# low-level wasm instr helpers
│  ├─ perf.rs        # PERF_LAST counter slot
│  ├─ rand.rs        # tinyrand-based RNG
│  ├─ serialize.rs   # MiniCBOR codecs
│  ├─ time.rs        # now_* helpers
│  └─ wasm.rs        # WASM module hashing helpers
└─ Cargo.toml
```
