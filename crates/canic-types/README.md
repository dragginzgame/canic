# canic-types

Shared Canic domain types that are candid-friendly and stable-structures friendly.

This crate centralizes small wrappers so downstream canisters can import a single
set of types (either via `canic::types::*` or `canic::core::types::*`).

Includes:
- `Cycles` – human-friendly parsing/formatting and config decoding helpers for cycle amounts.
- `Decimal` – candid-friendly wrapper around `rust_decimal::Decimal` (encoded as `text`).
- `BoundedString<N>` – length-capped string wrapper implementing stable-structures `Storable`.
- `WasmModule` – wrapper over embedded WASM bytes with hashing helpers.

```rust
use canic_types::{BoundedString16, Cycles};

let cycles: Cycles = "5T".parse().unwrap();
let name = BoundedString16::new("tenant".to_string());
```
