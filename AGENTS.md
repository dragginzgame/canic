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
- Treat every repository outside the Canic repository root as read-only unless
  the maintainer explicitly authorizes creation of a named new repository at an
  exact target path. That exception permits creating and bootstrapping only the
  named repository for the stated task; if the target already contains
  meaningful repository state, stop and request confirmation before modifying
  it.
- A request to inspect, review, audit, diagnose, design for, or give feedback
  about another repository never authorizes edits there. Record or report
  downstream work that remains instead of implementing it unless the
  maintainer separately gives explicit mutation authority for that existing
  repository.

## CI, Git, and Deployment
- Follow `docs/governance/ci-deployment.md`; it is the authoritative policy for
  commands, git boundaries, versioning, release, network selection, and
  automation language rules. Do not duplicate its rules here.

## Delivery Cadence
- Follow `docs/governance/delivery-cadence.md`; it is the authoritative policy
  for implementation slices, release batches, continuation and push-readiness
  handoffs.
- Ordinary continuation wording such as `continue`, `keep going` or `next` is
  sufficient authority to finish the current accepted in-repository batch and,
  once it is complete, begin the next already accepted and sequenced
  in-repository batch. Do not require a batch identifier, exact phrase or
  separate mutation ceremony. This does not authorize broad validation,
  versioning, Git publication, deployment, destructive actions or external
  effects; those boundaries retain their own rules.
- A focused implementation slice is not a patch release. Continue the current
  planned release batch until its direct implementation, adversarial/recovery
  evidence, propagation and cleanup are complete.
- Do not recommend a push merely because one focused test or CI repair passes.
  There is no minimum release count for a minor line. As a soft guideline, keep
  each minor to no more than 12 published releases; if another release would
  exceed that count, reassess the minor boundary and record the decision.

## Changelog
- Follow `docs/governance/changelog.md`; it is the authoritative changelog
  policy. Do not duplicate its rules here.
- Changelog maintenance is part of the default completion of every meaningful
  code or behavior batch. Do not wait for a separate maintainer request, and
  keep extending the existing open patch entry until that version is tagged.

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
- Before 1.0, a new Canic-owned product protocol, Candid/config schema or
  stable-state generation must not advance beyond `v1`. Change the maintained
  current contract through a hard cut instead of adding `v2` or later lanes.
  Exact upstream version names, immutable historical records and versioned
  audit-method/evidence revisions are not product compatibility generations
  and may retain their truthful versions.
- Every pre-1.0 release transition is reinstall-only. Active designs must not
  specify cross-release upgrades, state migration or import, authority
  handoff, existing-installation adoption, mixed-version operation, rollback,
  or compatibility recovery unless the maintainer explicitly asks for an
  exception.
- The maintainer approved one bounded exception on 2026-08-18: scheduled
  0.110 may define one whole-Fleet, stop-the-world transition from one exact
  released predecessor to one successor. This does not authorize rolling or
  mixed-version operation, arbitrary historical adoption, downgrade, generic
  compatibility code or implementation before the design's own gates.
- Same-release interruption recovery, retry, idempotency, backup, and restore
  remain required. They are operational safety, not compatibility behavior.
- Do not add anti-resurrection tests for removed legacy behavior or command
  forms. Current behavior tests should cover the maintained surface only.
- When deleting stale code, remove the old path completely and update active
  docs/examples to the current surface instead of preserving compatibility
  breadcrumbs.

## Layering
Dependency direction is strict: endpoints call workflow; workflow may call
policy and ops independently; ops may call model. Policy never calls ops.
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
- A configured synchronous lifecycle participant runs after Canic restoration
  and before deferred work; it does not become a second lifecycle owner.
- User hooks run after Canic invariants are restored, via zero-delay timers, and
  should be idempotent.

## Style
- Follow `docs/governance/code-hygiene/README.md`; it is the authoritative
  style policy for imports, module headers, type documentation, comments,
  visibility, and hygiene checks. Do not duplicate its rules here.
- Prefer `#[expect(...)]` over `#[allow(...)]` for lint suppressions so stale
  suppressions surface automatically. Use `#[allow(...)]` only for confirmed
  false positives where the lint may legitimately stop firing.
- Treat long or mixed `&&`/`||` expressions as a design smell, especially in
  authority and persistence validation. Group exact field equality into named
  authority structs and compose named predicates or helpers so each invariant
  remains independently readable and testable.
- Keep every functional `canic-cli` command and subcommand list in ASCII
  lexicographic order across declarations, dispatch, documentation, and
  rendered help. Keep the recursive help-surface ordering test passing; Clap's
  generated `help` pseudo-command is excluded.
- Keep `canic-cli` help concise: every help page may show at most three
  representative command examples. Command-group help should orient the user;
  detailed option combinations belong on the relevant leaf command.
- Canic deployment identities are App and Fleet. Use workspace for the local
  checkout, config and state root; never introduce Project as a Canic identity,
  CLI scope or report scope. Reserve project terminology for exact upstream or
  external concepts such as an ICP project root.
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
- Generic continuation or readiness wording such as `continue`, `finish the
  batch`, `make it push-ready`, or `keep going until we can push` never counts
  as an explicit request for a broad gate. Do not infer authorization for
  `make validate` or any equivalent full-suite command from those requests.
- The maintainer-owned version and release flow runs the complete validation
  gate before versioning, tagging, and pushing. Automated agents must not
  pre-run that gate; run it only when the maintainer explicitly names
  `make validate` or the exact broad suite to execute.
- The maintainer owns full deployment and publish validation. After targeted
  checks pass, agents should state whether the complete planned release batch,
  not merely the latest slice, is ready to push and whether its
  changelog/version surfaces are ready to publish; an unrun full suite is not,
  by itself, a blocker.
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
- Treat focused code slices as development work and group them into coherent
  release batches. Follow `docs/governance/delivery-cadence.md` and
  `docs/governance/changelog.md` for open-batch and open-patch updates; do not
  allocate one patch version per slice.
- Respect CLI/host/backup ownership boundaries.
- Run targeted checks only, following the Testing policy above.
