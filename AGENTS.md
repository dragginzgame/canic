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
- Keep one canonical workflow for each task. Do not add convenience shortcuts,
  command aliases or parallel configuration mechanisms without a demonstrated need.
  The hierarchy is App → Fleets → Canisters. Initialization declares an app and
  its first fleet; canister layout belongs in that fleet's config.
- Group CLI actions under `app`, `artifact` and `fleet`; wallet inspection takes
  a public principal. Keep command help alphabetical. Only `--json` is global;
  scope options belong after the action that actually consumes them. Reject
  unrelated context rather than silently ignoring it.
- Render terminal tables with IC Host's `ic-host-text::table` API. Canic owns
  columns, cell values and output policy; the shared renderer owns escaping,
  display widths and alignment. Do not hand-pad columns or add a formatter alias.
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
- `canic app init` creates local `canic.json` declarations only. Configured journals
  default to `.canic/apps/APP/fleets/FLEET/` beside the config. Preserve legacy `.canic/`
  operations and their v1 digests; `--no-config` selects that original flow.
  Named v2 plans bind fleet/app/canister names and never renew occupied-slot authority.
  This checkout's version 2 `canic.json` declares `hello-world/mainnet` and
  `profile/mainnet`; both select `journal: ".canic"`. Keep the original records
  in place and filter observations by selected app/fleet membership. Fleet slots are objects:
  `{}` for a future deployment, `{"id":"PRINCIPAL"}` for an existing canister.
  Existing IDs are observation declarations and block new creation in that slot.
  `profile_manager: true` selects only the demo's read-only status adapter.

## Changelog examples

At each user-facing increment, include a few short, copyable `canic` command
examples in fenced `sh` blocks in the root `CHANGELOG.md`. Place them near the
top of the current entry or beside the relevant feature, rather than burying
usage in prose or only linking to another document. Show supported fleet tasks,
state necessary prerequisites and effects, and keep browser launches opt-in.

## Development checks

Do not automatically open desktop windows, terminals or browsers. The maintainer
has paused automatic launches; only launch a window when explicitly requested.
Browser-facing demos and reports must be served over localhost. Give the
maintainer a clickable `http://localhost:PORT/` URL; do not offer a filesystem
HTML link or expect VS Code to render it. If explicitly requested, open previews
in the normal external desktop browser, not VS Code's embedded browser.
Keep a requested viewer running for review and state how to stop it. Use
`canic fleet view REPORT` for a saved fleet snapshot, `canic fleet show --serve` for a fresh
saved view, or `canic fleet show --watch` for bounded, read-only automatic observations.
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
