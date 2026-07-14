# Current Status

Last updated: 2026-07-14

## Purpose

This is the compact handoff for new agent sessions. Read this file first and
inspect only the linked design, audit, or source files needed for the current
task.

Historical detail is archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md); and
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md).

## Current Release

- The workspace package version is `0.91.1`.
- `v0.91.1` is published and points to commit `526f4068`.
- The current line is documented under
  [0.91 role admission and complete-build manifest publication](../design/0.91-role-admission-and-manifest-publication/0.91-design.md).

## Current Decision

`0.91.1` is live. The accepted 0.91 design is implemented, and the
[0.91 closeout audit](../audits/reports/2026-07/2026-07-13/0.91-closeout.md)
passes. The patch adds current-path-only root issuer readiness provisioning
after install/reinstall without restoring retired delegation-proof APIs.
Keep post-release work on the 0.91 line bounded to concrete defects,
dependency maintenance, and focused cleanup; no 0.92 line is assigned.

The 0.91.2 changelog is prepared for a maintenance batch that updates
`ic-memory` from 0.10.0 to 0.11.1 and makes loaded release-set manifests reject
lexically unsafe artifact paths at admission instead of deferring rejection
until staging. The dependency update is a persisted-format hard cut: a
canister with a 0.10.x protected allocation ledger cannot upgrade in place and
requires the documented destructive-reinstall dependency closure. No decoder,
migration, or compatibility path is being added.

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

## 0.91 Outcome

- `0.91.0` added canonical lowercase snake_case role admission and made normal
  root release-set manifests prove one current complete build's exact outputs.
- `0.91.1` added
  `AuthApi::provision_chain_key_delegation_proof_for_issuer_root` so an
  application root can establish issuer readiness after install/reinstall
  through the existing chain-key batch and issuer-install authority.
- The readiness facade accepts no caller-supplied proof material, adds no
  generated wire endpoint, and restores no retired proof workflow.
- Named build environments still resolve through `icp.yaml`; only `local` and
  `ic` are implicit, and no staging/mainnet aliases exist.

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
- The packaged downstream CLI, packaged downstream `wasm_store`, and isolated
  installed-CLI proofs pass against the 0.91.0 package surfaces. Their hard-cut
  command probes now use the maintained `--json` and `--help` forms and assert
  Medic's exit code 1 for blocking drift.
- The 0.91.1 focused PocketIC proof moves a freshly installed issuer from
  `Missing` to `Valid` active-proof status with the expected root and issuer
  bindings through the public root facade.
- Targeted provisioning tests, public-facade signature coverage, package
  checks, formatting, diff hygiene, and warning-denied Clippy pass for 0.91.1.
- For the active `ic-memory 0.11.1` batch, targeted package checks,
  warning-denied library Clippy, all 13 `canic-core` memory unit tests, the
  stable-memory ABI guard, four memory protocol-surface tests, changelog
  governance, formatting, and diff hygiene pass. Canic's application memory
  keys and IDs remain unchanged.
- For release-set path admission, all ten manifest-admission/loading tests and
  all three canonical artifact-path resolution and symlink-containment tests
  pass, together with targeted `canic-host`/`canic-cli` checks and
  warning-denied `canic-host` library and test-target Clippy.
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

Review the prepared 0.91.2 changelog with `ic-memory 0.11.1` as a
destructive-reinstall boundary, not an in-place-upgrade-safe patch. The package
version remains 0.91.1; the maintainer-owned release flow must perform version
preparation. Do not assign a 0.92 line or begin deferred multi-step claim
orchestration without a separately accepted design.
