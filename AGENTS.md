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
- `apps/profile/` owns the profile protocol, native client, discovery entry and
  persistent worker and one-slot autonomous manager. It uses IC Storage; Toko application integration is outside
  the current scope. Use `make profile-test`, `make profile-clippy` and
  `make profile-doc` for its affected checks.
  `make growth-test` qualifies operator-absent creation and manager maintenance
  using synthetic cycles. Never select `fault-injection` for a live deployment.
- Use Rust 1.100.0 and the inherited workspace lint policy.
- Packages start with `publish = false`; no first-party public licence is selected.
- The rewrite uses the 0.200.x version line to distinguish it from legacy Canic.
  Keep crates.io publication disabled until the product is mature and the
  maintainer explicitly authorizes publication. GitHub pushes do not authorize it.
- There are no local policy exceptions or release adapters. The experimental
  deployment path selects local/mainnet authority through an explicit plan.

## Development checks

Do not automatically open desktop windows, terminals or browsers. The maintainer
has paused automatic launches; only launch a window when explicitly requested.
Browser-facing demos and reports must be served over localhost. Give the
maintainer a clickable `http://localhost:PORT/` URL; do not offer a filesystem
HTML link or expect VS Code to render it. If explicitly requested, open previews
in the normal external desktop browser, not VS Code's embedded browser.
Keep a requested viewer running for review and state how to stop it. Use
`canic view REPORT` for a saved fleet snapshot, `canic fleet --serve` for a fresh
saved view, or `canic fleet --watch` for bounded, read-only automatic observations.
Omit `--open` unless a browser launch is explicitly requested.

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
