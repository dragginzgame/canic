# canic-utils

Small, deterministic utilities that Canic (and your canisters) rely on without pulling in the whole stack.

What you get:
- Fast xxHash3 helpers for non-crypto hashing.
- ChaCha20 PRNG seeded from the management canister `raw_rand` (reseeding recommended).
- String casing helpers (snake/constant/title/etc.).
- Small formatting helpers for logs/UI (`ellipsize_middle`, instruction suffixes).

Quick hits
```rust
use canic_utils::{format, hash, instructions, rand};

let digest = hash::hash_u64(b"tenant-123");      // fast sharding key
rand::seed_from([7; 32]);                        // tests only; use raw_rand in canisters
let sample = rand::next_u64().expect("seeded RNG");
let bytes = rand::random_bytes(16).expect("seeded RNG");
let hex = rand::random_hex(16).expect("seeded RNG");
let short = format::ellipsize_middle("abcdef0123456789", 10, 4, 4);
let pretty = instructions::format_instructions(12_345_678);
```

Determinism notes
- Hashing: xxHash3 is **not** cryptographic. Use it for sharding, cache keys, and fingerprints—not for signatures or certified data.
- RNG: ChaCha20 PRNG seeded from `raw_rand` and deterministic between reseeds. Use frequent reseeding for sensitive randomness, seed in init + post_upgrade (Canic runtime schedules this automatically), and call RNG helpers from update methods so state advances.

Casing helper
```rust
use canic_utils::case::{Case, Casing};
assert_eq!("hello_world".to_case(Case::Constant), "HELLO_WORLD");
assert!("Title Case".is_case(Case::Title));
```

Testing
- RNG has basic sanity checks (not a statistical entropy test).

Layout
```
canic-utils/
├─ src/
│  ├─ case/          # casing helpers (snake, constant, title)
│  ├─ format.rs      # tiny fmt helpers
│  ├─ hash.rs        # xxHash3 helpers
│  ├─ instructions.rs# low-level wasm instr helpers
│  ├─ rand.rs        # ChaCha20 PRNG seeded via raw_rand
│  └─ lib.rs
└─ Cargo.toml
```
