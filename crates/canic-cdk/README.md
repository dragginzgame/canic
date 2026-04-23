# canic-cdk

Canic's lightweight wrapper around `ic-cdk` types and helpers used across the stack.

This crate currently declares MSRV `1.91.0`. The Canic repo itself builds with a
newer pinned internal toolchain, but downstream crates that compile
`canic-cdk` from source should only need Rust `1.91.0` or newer.

This crate exists to give Canic (and downstream canisters) a stable import surface:

- `canic::cdk::api` and `canic::cdk::mgmt` re-export `ic_cdk` APIs.
- `canic::cdk::timers` re-exports `ic_cdk_timers`.
- `canic::cdk::candid` re-exports `candid`.
- `canic::cdk::structures` re-exports `ic-stable-structures` plus a small `BTreeMap` wrapper.
- `canic::cdk::types` provides common IC types (`Principal`, `Nat`, `Int`, `Account`, …).
- `canic::cdk::utils` hosts small WASM-safe helpers like `time::now_*` and `wasm::get_wasm_hash`.

Most users should access this crate via `canic::cdk` (from the facade crate).

## Example

```rust
use canic::cdk::{api, types::Principal};

#[canic::cdk::update]
fn whoami() -> Principal {
    api::caller()
}
```

For installation, workspace usage, and the repo-level internal toolchain pin,
see `../../README.md`.
