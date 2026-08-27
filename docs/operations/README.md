# Operations Docs

This directory holds release, packaging, install, smoke-test, and operator
validation notes.

## Current Release Validation

- [Release validation matrix](release-validation-matrix.md) defines the
  current release-validation inventory. Use it for slice close-out,
  implementation close-out, RC promotion, and final release/tag validation.
- [Supported host and target matrix](../governance/supported-platforms.md)
  defines the sole release-supported host/native/Wasm cell and distinguishes
  unvalidated installer branches.
- [Fleet ensure](../features/operations/fleet-ensure.md) documents the sole
  current convergence and interruption-replay workflow.
- [Recovery and retry runbooks](recovery-retry-runbooks.md) define the current
  same-release replay, ambiguous-result and operator retry safety procedures.
- [Release package and install validation](release-package-install-validation.md)
  records package, install, artifact, smoke-test, and environment-specific
  release gates.

Standing diagnostic, upgrade-state, and RC-readiness audit verdicts were
hard-cut during 0.92. Current evidence belongs to dated audit reports and the
active release-line closeout; this directory retains operator contracts and
validation procedures only.

## Blob Storage Operations

- [Blob storage integration](blob-storage-integration.md) documents the 0.69
  non-billing gateway endpoint wiring, lifecycle API contract, gateway
  principal handling, and focused validation commands for downstream canisters.
- [Blob storage source handoff](blob-storage-source-handoff.md) records the
  source and inventory evidence used to unlock the 0.69 implementation line.

## Intent Integration

- [Receipt-backed intent adapter handoff](receipt-backed-intent-adapter.md)
  defines the narrow begin, evidence-validation, settlement, and focused
  conformance contract for downstream effect adapters.

## Release Probe Inventories

- [0.56 v1 release probe inventory](0.56-v1-release-probes.md) records the
  retained installed and packaged v1 release probes.
- [Installed CLI smoke](0.56-installed-cli-smoke.md) documents the installed
  `canic` binary smoke proof.
- [Packaged downstream CLI](0.56-packaged-downstream-cli.md) documents the
  packaged downstream CLI proof.
- [Packaged wasm store](0.56-packaged-wasm-store.md) documents the special
  packaged downstream `wasm_store` bootstrap proof.
