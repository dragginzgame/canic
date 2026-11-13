# Canic local instructions

This repository adopts the reviewed Toko Tooling engineering baseline in
[TOKO-TOOLING.md](TOKO-TOOLING.md). Read that baseline before changing this repo.
Shared files are managed by `.toko-tooling.snapshot`; update them through the
authenticated tooling installer.

## Repository choices

- The maintainer treats legacy Canic (`canic-legacy`) as an intensely iterated design document
  spanning four years and four generations of LLMs. Derive Canic from user
  outcomes and platform constraints. Old code, terminology, package boundaries
  and roadmaps are research inputs; they may be discarded or redesigned. The
  current design is also revisable. Coverage does not require feature parity.
- Keep the virtual Cargo workspace at the root and product packages in `crates/`.
- `crates/canic` owns the small greeting Wire contract. Host and CLI packages
  remain native; the demo canister is in `apps/hello/canister`.
- Use Rust 1.100.0 and the inherited workspace lint policy.
- Packages start with `publish = false`; no first-party public licence is selected.
- There are no local policy exceptions or release adapters. The experimental
  deployment path selects local/mainnet authority through an explicit plan.

## Development checks

Use `make check`, `make test`, `make clippy` or `make doc` for the affected library
check. Use `make fmt-check` for Rust and manifest formatting, and
`make toko-tooling-check` for snapshot and dependency agreement. `make ci` is the
aggregate repository check. Tool preparation is explicit: `make install-tools`,
then `make tools-check`. The formatting hook is available via `make install-hooks`.
`make lint` checks both workspaces' Clippy, Rustdoc, formatting and tooling
agreement without running application or simulator tests. Every package inherits
its workspace lint table; scoped `expect` attributes need an explicit reason.
Use `make m1-test`, `make m1-clippy` and `make m1-doc` for the CLI/demo journey;
simulator and pinned ICP signer preparation remain explicit.
