# Canic Meta-Audit Contract

Method ID: `CANIC-META-001`

Method version: `1`

This contract defines what every maintained Canic audit must treat as active
authority. It does not replace the source documents. Auditors must inspect the
current authority and code sink directly rather than copying an old report's
conclusion.

## Authority Precedence

Resolve active authority in this order:

1. repository-root `AGENTS.md` and the governance documents it makes
   normative;
2. accepted active designs and finalized hard-cut decisions;
3. published public, serialized, stable-state, configuration, and operator
   contracts unless an accepted design authorizes a hard cut or migration;
4. active package and operator documentation;
5. implementation and generated surfaces as evidence of actual behavior;
6. tests as evidence of enforced behavior;
7. status and implementation trackers; and
8. historical reports and archived designs.

An active disagreement is a `governance_conflict` finding. Do not silently
select the source that matches current implementation. A published or stable
data contract still requires an explicit hard-cut or migration decision even
when a higher-level prose rule suggests a different shape.

## Repository And Release Boundaries

- Automated mutation stays inside the Canic repository.
- Package versioning, staging, commits, tags, pushes, deployment, and broad
  release gates remain maintainer-owned.
- Pre-1.0 removals are hard cuts: no alias, shim, fallback, deprecated API,
  duplicate path, or compatibility wrapper remains unless explicitly
  approved.
- Historical evidence may describe removed surfaces but cannot make them
  current authority.
- A desired behavior with no active authority is new scope, not an audit fix.

## Architecture And Data Invariants

The dependency direction is:

```text
endpoints -> workflow -> policy -> ops -> model
```

Audits must verify actual ownership, not directory names alone:

- endpoints authenticate, marshal, and delegate immediately;
- workflow owns multi-step orchestration and does not construct or mutate
  records;
- policy is pure and owns decisions, not mutation, async work, timers, IC
  calls, DTOs, serialization, or storage access;
- ops owns deterministic access, conversions, atomic mutations, and approved
  single-step platform effects;
- model owns authoritative state and storage invariants;
- DTOs are passive boundary data, views are internal read-only projections,
  and persisted schemas use `*Record` names;
- `export()` and `import()` are reserved for canonical `*Data` snapshots; and
- cross-layer data uses named structures rather than boundary aliases.

## Lifecycle And Stable-State Invariants

- `canic::start!` and lifecycle adapters stay thin and synchronous.
- Runtime invariants are restored before asynchronous bootstrap or user hooks
  are scheduled.
- User hooks run after Canic restoration and must tolerate replay.
- Stable-memory ownership, schema changes, migrations, interrupted upgrades,
  backup/restore boundaries, and recovery evidence are explicit.
- Unsupported state never silently decodes or falls back to an older path.

## Security And Failure Invariants

- Endpoint authentication is enforced before workflow execution.
- Caller, subject, audience, target, parent, subnet, role, epoch, and replay
  bindings are explicit where applicable.
- Capability, delegation, attestation, and token verification fail closed.
- Replay reservation, external effect, authoritative mutation, response
  publication, abort, and recovery-required transitions occur in a proven
  order.
- Failures preserve typed causes internally and intentional public projection
  at documented boundaries.
- Missing tools, missing evidence, decode ambiguity, stale state, or partial
  execution cannot become a passing result.

## Public And Operator Contract Invariants

Audits distinguish internal types from documented public surfaces:

- generated Candid and public Rust APIs;
- JSON fields and configuration schemas;
- CLI commands, exit codes, diagnostics, and text output where documented;
- stable-state and snapshot schemas;
- package features and published contents; and
- operator runbooks, recovery evidence, and release artifacts.

Exact strings are tested only when they are documented operator contracts.
Internal boundaries assert typed causes and observable state.

## Build, Supply-Chain, And Release Invariants

The retained method catalog must assign an owner or an explicit exclusion for:

- dependency advisories, licenses, and lockfile integrity;
- `build.rs`, procedural macros, and generated-code trust;
- unsafe code inventory and justification;
- CI permissions and third-party action pinning;
- secret scanning;
- release artifact provenance and checksums;
- reproducible or explained non-reproducible Wasm; and
- supported host and target environments.

These checks may use different cost classes and execution environments, but a
required unavailable check is `blocked`, not `pass`.

## Audit Evidence Invariants

Every run follows [AUDIT-HOWTO.md](AUDIT-HOWTO.md) and a method listed in
[METHODS.md](METHODS.md). Evidence must establish:

- immutable source and product identity;
- stable method identity and fingerprint;
- exact scope, exclusions, tools, fixtures, and environment;
- positive, rejection, boundary, and regression proof appropriate to the
  method;
- separate severity, confidence, run result, validity, and finding status;
- bounded, hashed, redacted retained artifacts; and
- explicit unreviewed boundaries and follow-up ownership.

Report volume, prior pass counts, status prose, or a successful literal-doc
guard is not proof that a current invariant holds.

## Meta-Audit Verdict

The audit system is eligible for a product baseline only when:

- every active method has one catalog owner and disposition;
- required methods satisfy their declared contract and fixtures;
- no required run is partial or blocked;
- competing legacy authority is removed;
- method fingerprints and the product snapshot are frozen separately; and
- the required holistic coverage map has no unexplained gap.
