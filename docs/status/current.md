# Current Status

Last updated: 2026-07-13

## Purpose

This is the compact handoff for new agent sessions. Read this file first and
inspect only the linked design, audit, or source files needed for the current
task.

Historical detail is archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md); and
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md).

## Current Release

- The workspace package version is `0.90.2`.
- `v0.90.2` is published and points to commit `4289dcc7`.
- The current line is documented under
  [0.90 receipt-backed intent reconciliation](../design/0.90-receipt-backed-intent-reconciliation/0.90-design.md).

## Current Decision

The Canic-owned 0.90 line is complete. The
[0.90 closeout audit](../audits/reports/2026-07/2026-07-13/0.90-closeout.md)
passes without a correction release. Do not add `0.90.3` work unless a new
concrete defect is found.

Toko mint remains the first planned downstream consumer. Toko developers own
its request, caller-scoped receipt, evidence validation, retry, cancellation,
and domain tests after consuming the published Canic release. Automated Canic
work must not edit the Toko repository or move mint concepts into Canic core.

The
[post-0.90 deployment health audit](../audits/reports/2026-07/2026-07-13/post-0.90-deployment-health.md)
found two bounded next-line candidates: canonical role-name admission and
complete-build-only release-set publication. A
[0.91 role admission and complete-build manifest publication](../design/0.91-role-admission-and-manifest-publication/0.91-design.md)
design now fixes the proposed owners, current-output proof, limits, and
deletion list. The design is accepted, and both slices are implemented.
Complete configuration validation, fleet role mutations, and loaded
release-set manifests use one core-owned canister role-name predicate.

That predicate admits lowercase snake_case only. Kebab-case, case variants,
and malformed underscore forms fail rather than being normalized or aliased.

The same validated snapshot must parameterize every required builder and the
manifest writer. Writer-side coverage, exact-path, read, hash, validation, and
serialization failures must leave an existing manifest unchanged.

Install-root now resolves that snapshot before mutation, builders consume its
admitted package and output paths, and the normal manifest writer accepts only
the exact role-labelled outputs returned by the current complete build.
Single-role builds no longer create, rewrite, report, or attest a root
release-set manifest. Filesystem existence is not completion evidence.

Multi-step claim orchestration is deferred to a separately accepted future
design. It is not reserved for 0.91 or any other numbered line.

## 0.90 Outcome

- `0.90.1` added the generic exact-key receipt-backed intent primitive and
  hard-cut automatic call-builder intent orchestration.
- `0.90.2` added exact public-facade adapter conformance and the downstream
  integration handoff without changing production APIs or stable state.
- One `OperationId` map on memory ID 43 owns receipt-backed rows. Existing
  local intent allocations remain on IDs 39 through 42.
- Local and receipt-backed reservations share one persisted resource
  aggregate. Receipt-backed rows never enter the local TTL index or change the
  metadata's expirable-pending count.
- Begin and settlement are non-awaiting exact-key operations. Terminal replay
  is idempotent; contradictory evidence cannot change counters.
- No mint type, receipt protocol, resolver, background reconciler, global scan,
  or compatibility path exists in Canic.

## Recent Closed Lines

- `0.89` typed deployment evidence end to end and narrowed dependency/RPC
  surfaces. Design:
  [0.89 deployment evidence](../design/0.89-deployment-evidence-and-surface-truth/0.89-design.md).
- `0.88` completed artifact durability, publication-atomic CLI output, and the
  typed fleet-config boundary. Design:
  [0.88 artifact durability](../design/0.88-artifact-durability-and-config-errors/0.88-design.md).
- `0.87` completed operator-boundary hygiene and the product environment-input
  hard cut. Design:
  [0.87 operator boundary](../design/0.87-operator-boundary-hygiene/0.87-design.md).
- Older release-line history remains in the archived status files above.

## Focused Validation

- The 0.91 Slice A core admission, complete-config rejection, fleet-mutation,
  and release-set manifest tests pass.
- The 0.91 Slice B current-output coverage, exact-path, snapshot-ordering,
  unchanged-manifest rejection, provenance, and install-operation tests pass.
- Targeted `canic-host` test-target Clippy passes after the Slice B hard cut.
- The bounded
  [0.91 closeout audit](../audits/reports/2026-07/2026-07-13/0.91-closeout.md)
  passes with one normal writer, exact current-output proof, and no retired
  readiness or optional-manifest surface.
- Targeted `canic-core`, `canic-host`, and `canic-cli` check and Clippy pass.
- The published 0.90.2 PocketIC proof covers local capacity plus receipt
  creation, replay, conflicts, commit, rollback, terminal replay, released
  rollback capacity, and pending-state upgrade recovery.
- A post-release targeted core rerun passes all seven receipt-backed storage
  and canonical-snapshot tests.
- The closeout scan confirms one facade, one operation store, one accounting
  authority, canonical ID 43 ownership, low-cardinality metrics, no receipt
  timer path, no retired call-builder surface, and no mint-domain leakage.
- Full workspace, broad PocketIC, deployment, and release suites remain
  maintainer-owned and were not rerun for the closeout documentation pass.

## Next Action

The accepted 0.91 implementation is complete and ready for maintainer push
review. The 0.91.0 root and detailed changelog entries are prepared. Package
version preparation remains maintainer-owned; the workspace is still 0.90.2.
