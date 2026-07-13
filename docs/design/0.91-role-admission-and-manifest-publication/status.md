# 0.91 Role Admission and Complete-Build Manifest Publication - Status

Last updated: 2026-07-13

## Current State

The post-0.90 audit and design review are complete. Both accepted slices are
implemented. Configuration validation, fleet mutations, and release-set
manifest validation share one core-owned canister role-name rule. Its ASCII
snake_case grammar also replaces the matching private scaffold implementation
without sharing the role-specific length policy.

Install-root now resolves one validated configuration-backed target snapshot,
passes its admitted package and output paths to every normal role build, keeps
the role-labelled outputs returned by that invocation, and supplies those
exact outputs to the sole crate-private normal manifest writer. The writer
rejects missing, duplicate, unexpected, or path-mismatched outputs and derives
entry order from the snapshot.

The 0.91.0 root and detailed changelog entries are prepared. Package versions
remain unchanged and await the maintainer-owned release bump.

## Accepted Scope

The line contains only:

1. one core-owned canister role-name admission rule reused by config,
   fleet-mutation, and release-set boundaries; and
2. explicit current-invocation output proof for release-set publication, with
   every single-role manifest side effect removed.

The writer must consume the same validated install snapshot and exact
role-labelled outputs collected by the complete build. It may not reload
configuration or infer completion from artifact-path existence.

Every required builder must also receive identity, selection, release-version,
and expected-path inputs from that snapshot. Pre-publication coverage, path,
read, hash, validation, or serialization failures leave an existing manifest
unchanged.

## Hard-Cut Inventory

Completed removals:

- length-only and host-local canister role predicates;
- both `emit_root_release_set_manifest_if_ready*` functions;
- the unused public no-config manifest writer;
- `CanisterArtifactBuildOutput::manifest_path`;
- single-role build manifest emission; and
- fleet-manifest inclusion in per-role build provenance.

No alias, shim, sanitization, readiness replacement, build epoch, or
compatibility path is proposed.

## Explicit Limits

- `CanisterRole` remains passive; construction is not admission.
- Role-derived artifact-path sinks must be dominated by canonical validation.
- Path safety is lexical and does not claim symlink or cross-platform
  collision protection.
- Lowercase snake_case is the only admitted role spelling; kebab-case and
  case variants are rejected rather than normalized or aliased.
- A partial build leaves an old manifest unchanged but may make it fail normal
  artifact-hash validation.
- Only one mutating build/install command may target an install root at once;
  concurrent writers are unsupported.
- Multi-step claim orchestration is deferred to a separately accepted future
  design and is unrelated to this line.

## Validation State

- 0.90 closeout: passed.
- Post-0.90 deployment audit: complete with two findings.
- Revised 0.91 documentation whitespace: `git diff --check` passed.
- Targeted changelog governance: one test passed.
- Slice A lowercase snake_case predicate acceptance and rejection tests:
  passed.
- Slice A complete configuration rejection test: passed.
- Existing canister-role length-bound regression test: passed.
- Existing fleet configuration fixture validation: passed.
- Shared scaffold snake_case parser acceptance and rejection tests: passed.
- Candid/Serde unsupported-rename guard, including `canic-host`: passed.
- Slice A fleet mutation typed-admission test: passed.
- Slice A release-set manifest acceptance and rejection tests: passed.
- Slice B exact current-output coverage, path equality, snapshot ordering, and
  unchanged-existing-manifest rejection tests: passed.
- Slice B per-role build-provenance tests pass without a fleet-manifest
  artifact.
- Slice B install operation evidence tests: passed.
- Targeted `canic-core`, `canic-host`, and `canic-cli` check: passed.
- Targeted `canic-host` check and test-target Clippy after Slice B: passed.
- Bounded [0.91 closeout audit](../../audits/reports/2026-07/2026-07-13/0.91-closeout.md):
  passed.
- Full workspace, broad PocketIC, deployment, and release suites: not run per
  repository policy.

## Next Action

The accepted implementation and 0.91.0 changelog are complete and ready for
maintainer push review. Package version preparation remains maintainer-owned.
