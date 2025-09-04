# ✨ ICU – Internet Computer Utilities

**ICU** (Internet Computer Utilities) is a Rust framework that simplifies the development and management of multi-canister systems on the DFINITY **Internet Computer (IC)**. It provides a set of utilities and macros to coordinate multiple canisters (smart contracts) working together, making it easier to create complex canister-based dapps that scale across canister boundaries (even across multiple subnets).

ICU addresses common challenges in multi-canister architectures, including canister creation & upgrades, cross-canister state management, stable memory handling across upgrades, and establishing a clear canister hierarchy (with a **root** canister orchestrating child canisters). By using ICU, developers can focus on application logic rather than reinventing boilerplate for managing canister lifecycles and interactions.

## Quickstart

Add ICU to your workspace and wire a canister:

1) In your canister crate `build.rs`:

```rust
fn main() { icu::icu_build!("../icu.toml"); }
```

2) In your canister `lib.rs`:

```rust
use icu::prelude::*;
icu_start_root!(); // or icu_start!(icu::EXAMPLE)

const fn icu_setup() {}
async fn icu_install() {}
async fn icu_upgrade() {}
```

See `crates/canisters/root` and `crates/canisters/example` for full patterns.

MSRV: Rust 1.89.0 (pinned via `rust-toolchain.toml`).

## Contributing

See Repository Guidelines in `AGENTS.md` for project structure, coding style, testing, and PR requirements. For versioning and releases, refer to `VERSIONING.md` and `RELEASE_GUIDE.md`.

### Setup

Install required toolchain components once:

```bash
make install-canister-deps
```

## Examples

Example files:

- [crates/icu/examples/auth_rules.rs](crates/icu/examples/auth_rules.rs) — basic auth rule composition
- [crates/icu/examples/minimal_root.rs](crates/icu/examples/minimal_root.rs) — minimal root canister scaffold
- [crates/icu/examples/ops_create_canister.rs](crates/icu/examples/ops_create_canister.rs) — create-canister request flow

Build all examples:

```bash
make examples
# or
cargo build -p icu --examples
```

Run a specific example: `cargo run -p icu --example auth_rules`

Note: The `ic` cfg is used internally for tests/build tooling and is not a user-settable feature flag.
