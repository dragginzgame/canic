# AGENTS.md

This file is normative for automated agents and contributors. If code conflicts
with this file, the code is wrong.

## Session Handoff
- At the start of a new session, read `docs/status/current.md` first. Treat it
  as the compact handoff and avoid replaying old chat history unless needed.

## Repository Scope
- Automated edits are restricted to this Canic repository. Do not modify,
  format, generate files in, or otherwise mutate sibling or external
  repositories, even when a Canic design names a downstream integration.
- Treat every repository outside the Canic repository root as read-only.
  Record or report downstream work that remains instead of implementing it in
  another repository.

## CI, Git, and Deployment
- Follow `docs/governance/ci-deployment.md`; it is the authoritative policy for
  commands, git boundaries, versioning, release, network selection, and
  automation language rules. Do not duplicate its rules here.
- Automated agents must never change Cargo package versions unless the
  maintainer explicitly asks for a version bump, release-preparation version
  change, or exact version correction. This includes
  `workspace.package.version`, workspace `canic*` dependency versions, package
  versions in any `Cargo.toml`, release-script default versions, install URLs,
  and the matching `Cargo.lock` package versions.

## Changelog
- Follow `docs/governance/changelog.md`; it is the authoritative changelog
  policy. Do not duplicate its rules here.

## Ownership
- Runtime/facade: `canic`, `canic-core`, `canic-macros`.
- Canister control plane/store: `canic-control-plane`, `canic-wasm-store`.
- Host/operator: `canic-cli`, `canic-host`, `canic-backup`.
- Testing: sibling `ic-testkit`, `canic-testing-internal`, `canic-tests`.
- `scripts/dev/*` are intentional maintainer helpers, not stale CLI leftovers.
- Keep flat `crates/` unless doing a full Cargo/CI/docs/publish migration.

## Pre-1.0 Hard Cuts
- Before 1.0, removed surfaces are hard-cut. Do not add aliases, shims,
  compatibility wrappers, legacy fallback paths, or backwards-compatibility
  layers unless the maintainer explicitly asks.
- Do not add anti-resurrection tests for removed legacy behavior or command
  forms. Current behavior tests should cover the maintained surface only.
- When deleting stale code, remove the old path completely and update active
  docs/examples to the current surface instead of preserving compatibility
  breadcrumbs.

## Layering
Dependency direction is strict: `endpoints -> workflow -> policy -> ops -> model`.
- `dto/` is passive boundary data only.
- `model/` owns authoritative state and storage invariants.
- `ops/` owns deterministic state access, conversion, and approved single-step
  platform side effects.
- `policy/` is pure decision logic: no mutation, async, timers, IC calls, DTOs,
  storage access, or serialization.
- `workflow/` owns multi-step orchestration and may call ops/policy.
- `endpoints` and macros marshal/authenticate and delegate immediately.
- Conversions belong in `ops::*`; workflow must not construct/mutate records.

## Data Shapes
- DTOs are data-only boundary contracts.
- Command/request/mutation DTOs must not implement `Default` unless neutral.
- Views are internal read-only projections and live under `view/`.
- Records are persisted storage schema and end in `*Record`.
- `export()` and `import()` are reserved for canonical `*Data` snapshots.
- Cross-layer data should use named structs/enums, not boundary type aliases.

## Lifecycle
- `canic::start!` must stay thin.
- Lifecycle adapters restore synchronously and schedule async work; no `await`.
- User hooks run after Canic invariants are restored, via zero-delay timers, and
  should be idempotent.

## Style
- Follow `docs/governance/code-hygiene/README.md`; it is the authoritative
  style policy for imports, module headers, type documentation, comments,
  visibility, and hygiene checks. Do not duplicate its rules here.
- Prefer `#[expect(...)]` over `#[allow(...)]` for lint suppressions so stale
  suppressions surface automatically. Use `#[allow(...)]` only for confirmed
  false positives where the lint may legitimately stop firing.
- Rust edition is 2024.
- Use directory modules with `mod.rs`; never keep both `foo.rs` and `foo/`.
- Do not use `#[path = "..."]` for module layout. Rename files/directories so
  Rust's normal module discovery works.

## Testing
- Automated agents must run only targeted checks for the files, package, and
  behavior they changed. Do not run full workspace, release-matrix, or broad
  PocketIC suites such as `make test`, `make clippy`, or workspace-wide Cargo
  test/Clippy commands unless the maintainer explicitly requests that exact
  broad gate.
- The maintainer owns full deployment and publish validation. After targeted
  checks pass, agents should state whether the current change set is ready to
  push and whether its changelog/version surfaces are ready to publish; an
  unrun full suite is not, by itself, a blocker.
- Unit tests live next to code; integration tests live in `tests/`.
- Canister creation/install/upgrade/inter-canister tests must use PocketIC.
- Do not add production `cfg(test)` behavior to fake IC management.
- Assert typed errors or observable state, not error strings.

## Security
- Auth is enforced at endpoints.
- Workflow and ops assume authenticated input.
- Subnet, parent, subject, audience, and caller bindings must be explicit.

## Checklist
- Preserve dirty worktree state and keep edits scoped.
- Treat focused code slices as development work, not as release patches. Use
  `docs/governance/changelog.md` for `Unreleased` notes and release-finalized
  changelog rules.
- Respect CLI/host/backup ownership boundaries.
- Run targeted checks only, following the Testing policy above.
