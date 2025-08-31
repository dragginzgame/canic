# ✨ ICU – Internet Computer Utilities

**ICU** (Internet Computer Utilities) is a Rust framework that simplifies the development and management of multi-canister systems on the DFINITY **Internet Computer (IC)**. It provides a set of utilities and macros to coordinate multiple canisters (smart contracts) working together, making it easier to create complex canister-based dapps that scale across canister boundaries (even across multiple subnets).

ICU addresses common challenges in multi-canister architectures, including canister creation & upgrades, cross-canister state management, stable memory handling across upgrades, and establishing a clear canister hierarchy (with a **root** canister orchestrating child canisters). By using ICU, developers can focus on application logic rather than reinventing boilerplate for managing canister lifecycles and interactions.

* ✨ Overview of what ICU is
* 📦 Installation instructions
* 🧩 Usage examples
* 🧠 Architecture overview
* 🤝 Contribution guidelines
* 📄 License section

... rest to come ...

## Contributing

See Repository Guidelines in `AGENTS.md` for project structure, coding style, testing, and PR requirements. For versioning and releases, refer to `VERSIONING.md` and `RELEASE_GUIDE.md`.

## Examples

Example files:

- [crates/icu/examples/auth_rules.rs](crates/icu/examples/auth_rules.rs) — basic auth rule composition
- [crates/icu/examples/minimal_root.rs](crates/icu/examples/minimal_root.rs) — minimal root canister scaffold (use `--features ic`)
- [crates/icu/examples/ops_create_canister.rs](crates/icu/examples/ops_create_canister.rs) — create-canister request flow (use `--features ic`)

Build all examples:

```bash
make examples
# or
cargo build -p icu --examples
cargo build -p icu --examples --features ic
```

Run a specific example (non-IC): `cargo run -p icu --example auth_rules`

IC-specific examples compile with `--features ic` and are intended for canister contexts.
