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
- `canic app init` creates local `canic.json` declarations only: it creates a missing
  config or adds a new app and its first fleet, refusing existing app names and
  preserving other apps. Configured journals are not created by initialization.
  `canic fleet init` adds an empty fleet to an
  existing app with an explicit network, using atomic config replacement and a
  persistent `CONFIG.canic-lock` sidecar. It refuses existing fleet names.
  `canic fleet canister add/list` declares and lists slots with explicit app/fleet
  selection. These commands use configuration only; listing does not infer
  deployment state from the presence or absence of a configured ID.
  `add --id PRINCIPAL` records an existing identity for observation, with no
  network calls or authority claim. It refuses occupied names and conflicting
  membership on the same network; it never rebinds slots or rewrites journals.
  Configured journals default to `.canic/apps/APP/fleets/FLEET/` beside the config. Preserve legacy `.canic/`
  operations and their v1 digests; `--no-config` selects that original flow.
  Named v2 plans bind fleet/app/canister names and never renew occupied-slot authority.
  This checkout's version 2 `canic.json` declares `hello-world/mainnet` and
  `profile/mainnet`; both select `journal: ".canic"`. Keep the original records
  in place and filter observations by selected app/fleet membership. Fleet slots are objects:
  `{}` for a future deployment, `{"id":"PRINCIPAL"}` for an existing canister.
  Existing IDs are observation declarations and block new creation in that slot.
  `profile_manager: true` selects only the demo's read-only status adapter.

## Legacy research before subsystem work

Before substantive subsystem implementation or redesign, inspect the relevant
`canic-legacy` implementation, regression tests and Git history. Treat it as
accumulated engineering evidence, not merely a source of feature names.

- Identify the problem the legacy solution addressed, its failure cases and
  safety requirements, and which platform or consumer assumptions have changed.
- Reuse or adapt sound solutions where they fit the current Canic design. Explain
  why a relevant solution is being replaced or omitted rather than rediscovering
  the same failure through trial and error.
- Bring applicable regression cases into Canic and run them against the new
  implementation. Preserve the demonstrated invariant, not incidental legacy
  structure or terminology.
- Record the inspected revision, relevant source/test paths and the resulting
  decision in the subsystem's existing design/work notes. Reuse prior research
  when it covers the same revision and question; identify gaps in the evidence.

This research does not require feature parity or give an existing mechanism a
presumption of inclusion. Derive the implementation from current user outcomes
and platform constraints, retaining protections for threats that still apply.
Inspect the legacy repository read-only unless changes there are separately
authorized.

## Post-push design review

After every push, review the pushed changes and their affected callers against
the current design and actual consumer needs before starting the next increment.
The purpose is to prevent the drift, accumulated leftovers and technical debt
that made legacy Canic difficult to evolve.

- Check domain boundaries, ownership, CLI/config semantics and documentation for
  disagreement with each other or with the intended App → Fleets → Canisters model.
- Look for obsolete paths, terminology, aliases, compatibility scaffolding,
  duplicated policy, unused dependencies and temporary mechanisms left behind.
- Challenge abstractions, modes and infrastructure without a demonstrated need.
  Prefer deletion or simplification while preserving required safety and recovery
  guarantees; do not replace sediment with another layer of machinery.
- Report concrete findings with code locations and the smallest corrective action,
  or state the reviewed scope and that no actionable findings were found. Record
  deferred findings in the relevant design/work notes so they are not forgotten.

Keep this a focused review, not an automatic full audit or test-suite rerun.
The review does not itself authorize another push or an unrelated redesign.

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
