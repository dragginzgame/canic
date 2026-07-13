# 0.89 Status: Deployment Evidence and Surface Truth

Last updated: 2026-07-13

## Current State

The post-0.88 audit selected exactly three bounded slices, and all three are
complete. Install-state persistence owns one typed error, all six boxed
signatures are hard-cut, real read failures are no longer treated as missing
files, and ICP-root discovery exposes its actual I/O contract. Installed
deployment resolution and all ten command-owned projectors retain concrete
install-state and replica-query sources through local classification. Four
compilation-proven dead declarations are removed, the facade's generated
`ic-cdk` test consumer is explicitly accounted for, and the audited RPC
adapters are private to their dispatch owner. Package versions remain
`0.88.2`.

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
- Full workspace, PocketIC, deployment, and broad Wasm suites were not run.

## Next Action

The bounded 0.89 implementation is complete. Prepare its changelog and release
version only when explicitly requested; do not add another implementation
slice or widen the line into dependency upgrades or broad visibility cleanup.
