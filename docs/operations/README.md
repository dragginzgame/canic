# Operations Docs

This directory holds release, packaging, install, smoke-test, and operator
validation notes.

## Current Release Validation

- [Release validation matrix](release-validation-matrix.md) defines the
  current release-validation inventory. Use it for slice close-out,
  implementation close-out, RC promotion, and final release/tag validation.
- [Upgrade and state compatibility audit](upgrade-state-compatibility-audit.md)
  records the current upgrade/state evidence for replay-sensitive release
  surfaces.
- [Recovery and retry runbooks](recovery-retry-runbooks.md) document safe
  operator recovery decisions for replay-sensitive failures and uncertain
  operations.
- [Diagnostic consistency audit](diagnostic-consistency-audit.md) records
  whether current public errors, logs, metrics, tests, and docs distinguish the
  replay-sensitive failure classes needed for RC operation.
- [Release package and install validation](release-package-install-validation.md)
  records package, install, artifact, smoke-test, and environment-specific
  release gates.
- [RC readiness audit](rc-readiness-audit.md) records the implementation
  close-out verdict and separates remaining RC/final-release validation from
  additional implementation slicing.

## Release Probe Inventories

- [0.56 v1 release probe inventory](0.56-v1-release-probes.md) records the
  retained installed and packaged v1 release probes.
- [Installed CLI smoke](0.56-installed-cli-smoke.md) documents the installed
  `canic` binary smoke proof.
- [Packaged downstream CLI](0.56-packaged-downstream-cli.md) documents the
  packaged downstream CLI proof.
- [Packaged wasm store](0.56-packaged-wasm-store.md) documents the special
  packaged downstream `wasm_store` wrapper proof.
