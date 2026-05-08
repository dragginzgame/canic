# AGENTS.md

This file is normative for automated agents and contributors. If code conflicts
with this file, the code is wrong.

## Session Handoff
- At the start of a new session, read `docs/status/current.md` first. Treat it
  as the compact handoff and avoid replaying old chat history unless needed.

## CI, Git, and Deployment
- Follow `docs/governance/ci-deployment.md`; it is the authoritative policy for
  commands, git boundaries, versioning, release, network selection, and
  automation language rules. Do not duplicate its rules here.

## Changelog
- Follow `docs/governance/changelog.md`; it is the authoritative changelog
  policy. Do not duplicate its rules here.

## Ownership
- Runtime/facade: `canic`, `canic-core`, `canic-cdk`, `canic-memory`,
  `canic-macros`.
- Canister control plane/store: `canic-control-plane`, `canic-wasm-store`.
- Host/operator: `canic-cli`, `canic-host`, `canic-backup`.
- Testing: `canic-testkit`, `canic-testing-internal`, `canic-tests`.
- `scripts/dev/*` are intentional maintainer helpers, not stale CLI leftovers.
- Keep flat `crates/` unless doing a full Cargo/CI/docs/publish migration.

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
- `canic::start!` and `canic::start_root!` must stay thin.
- Lifecycle adapters restore synchronously and schedule async work; no `await`.
- User hooks run after Canic invariants are restored, via zero-delay timers, and
  should be idempotent.

## Style
- Rust edition is 2024.
- Prefer existing local patterns; keep imports at file top.
- Use directory modules with `mod.rs`; never keep both `foo.rs` and `foo/`.
- Comment intent/invariants only. See `docs/governance/code-hygiene.md`.
- Shipped `CandidType` types should avoid `///` unless metadata is intentional.

## Testing
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
- Update changelogs for user-facing changes.
- Respect CLI/host/backup ownership boundaries.
- Prefer targeted tests first; broaden when risk warrants it.
