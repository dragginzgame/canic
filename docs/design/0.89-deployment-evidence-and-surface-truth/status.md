# 0.89 Status: Deployment Evidence and Surface Truth

Last updated: 2026-07-13

## Current State

The post-0.88 audit selected exactly three bounded slices, and all three shipped
in `0.89.0`. Install-state persistence owns one typed error, all six boxed
signatures are hard-cut, real read failures are no longer treated as missing
files, and ICP-root discovery exposes its actual I/O contract. Installed
deployment resolution and all ten command-owned projectors retain concrete
install-state and replica-query sources through local classification. Four
compilation-proven dead declarations are removed, the facade's generated
`ic-cdk` test consumer is explicitly accounted for, and the audited RPC
adapters are private to their dispatch owner.

A post-release conformance scan found one Slice A admission gap: decoded state
was not checked against schema version 2 or its requested deployment/network
path, while deployment-catalog enumeration maintained a second partial check.
One owner decoder now applies all three checks to named reads and catalog
enumeration. That correction is published as `0.89.1`.

The following closeout scan found one Slice B source-retention gap: passive
deployment inventory rendered `InstallStateError` into text immediately after
the canonical read. The inventory error now retains that concrete source while
preserving its display text. Package versions remain `0.89.1` until the
maintainer requests the next release bump.

## Checklist

### Slice A - Typed install-state persistence

- [x] Define one focused `InstallStateError`.
- [x] Type validation, mismatch, I/O, and JSON failures.
- [x] Remove boxed errors from install-state persistence.
- [x] Preserve schema, paths, values, and serialized bytes.

### Slice B - Typed installed-deployment evidence

- [x] Retain concrete install-state and replica-query sources.
- [x] Migrate command projections without changing local action policy.
- [x] Remove early string conversion of both source families.
- [x] Preserve exit, hint, finding, JSON, and successful output behavior.

### Slice C - Manifest and RPC surface truth

- [x] Remove exactly four unused dependency declarations.
- [x] Retain and account for the facade's generated `ic-cdk` test consumer.
- [x] Prove affected packages and build scripts still compile.
- [x] Remove two unused RPC adapter re-exports.
- [x] Make three concrete RPC adapters private.
- [x] Keep request ops/error visibility within the private ops tree.
- [x] Restore a clean Cargo Machete result.

### Post-release Slice A correction

- [x] Reject unsupported install-state schema versions on read and write.
- [x] Reject deployment/network identity mismatches on read.
- [x] Route deployment-catalog decoding through the same admission owner.
- [x] Reject path-like catalog networks before directory construction.
- [x] Preserve valid schema-2 bytes and existing catalog warning contracts.

### Post-release Slice B correction

- [x] Retain the concrete install-state source in passive deployment inventory.
- [x] Delete the early string conversion without adding a shared projector.
- [x] Preserve existing display text and successful inventory behavior.

## Validation

- Five focused install-state tests pass for exact serialized bytes, typed
  validation, retained read path/I/O source, retained decode path/JSON source,
  network mismatch, and retained write path/I/O source.
- Two focused deployment-registration caller tests pass.
- The focused deployment-plan caller test and targeted CLI library check pass
  after narrowing ICP-root discovery to `io::Result`.
- The install-state owner contains no boxed dynamic-error signature.
- Four focused installed-deployment tests pass, including retained JSON decode
  path/source and retained structured replica rejection.
- Twenty-seven focused CLI tests pass for command missing-deployment guidance,
  blob-storage JSON transport classification, and Medic source classification.
- Targeted warning-denied `canic-host` and `canic-cli` library Clippy passes.
- Source scans find no string-valued install-state or replica-query command
  variant and no early text conversion of either installed-deployment source.
- All ten command projectors remain local; no generic projection abstraction
  was introduced.
- Formatting, layering guards, and diff hygiene pass.
- Targeted root-build, runtime-probe test, integration-test, and facade-test
  compilation passes after the manifest changes.
- Facade integration-test compilation proves endpoint proc macros require the
  retained direct `ic-cdk` development dependency.
- Three focused request-dispatch tests pass for request shape, replay metadata,
  and response-variant checking.
- Source scans find no concrete RPC adapter consumer outside its dispatch owner
  and local tests, and no adapter re-export remains.
- Warning-denied Clippy passes for core, facade, test root, runtime probe, and
  integration-test packages.
- Cargo Machete reports no unused dependency.
- Nine focused install-state/deployment-observation tests pass, including
  schema, deployment, and network mismatch rejection.
- Eight focused deployment-catalog tests pass, including unsupported-schema
  exclusion and pre-path network validation while valid entries and report
  behavior remain unchanged.
- Five focused CLI deployment-catalog tests preserve parsing, dispatch, help,
  and JSON output behavior.
- Warning-denied `canic-host` library/test Clippy passes for the correction.
- The focused passive-inventory test retains the install-state decode path and
  JSON source through `DeploymentTruthError`.
- Source scans find no remaining install-state-to-string conversion in host or
  CLI deployment paths.
- Full workspace, PocketIC, deployment, and broad Wasm suites were not run.

## Next Action

The bounded `0.89.0` implementation remains closed and the Slice A correction
is published as `0.89.1`. The post-release Slice B correction is implemented
without opening another slice, and the root and detailed `0.89.2` changelog
entries are prepared. Package versions remain `0.89.1` until the maintainer
requests a release bump. Do not widen the line into dependency upgrades or
broad visibility cleanup.
