# Operations Reference

These documents are for people operating Canic or preparing a Canic release.
They cover deployment, recovery, funding, diagnostics, packaging, and release
checks.

<img src="../../assets/256x256/mechanic-notes.png" align="left" width="110" alt="The Canic mechanic holding an operations checklist" />

If you are deploying an application for the first time, begin with
[Installing Canic](../../INSTALLING.md), then follow the
[Fleet ensure guide](../features/operations/fleet-ensure.md). The remaining
pages are references for a specific operational task or failure.

<br clear="left" />

## Choose An Operations Path

| If you need to… | Start here |
| --- | --- |
| Deploy or change a Fleet | [Operations and diagnostics](../features/operations/README.md) |
| Choose the correct Fleet Ensure workflow | [Fleet Ensure overview](../features/operations/fleet-ensure.md) |
| Recover an interrupted operation | [Recovery and retry runbooks](recovery-retry-runbooks.md) |
| Diagnose Fleet funding | [Fleet funding](fleet-funding.md) |
| Back up or restore canisters | [Backup and restore](../features/backup-and-restore/README.md) |
| Validate a release | [Release validation matrix](release-validation-matrix.md) |
| Diagnose CI or packaging | [CI diagnostics](ci-diagnostics.md) and [package validation](release-package-install-validation.md) |

## Current Release Validation

- [Release validation matrix](release-validation-matrix.md) defines the
  current release-validation inventory. Use it for slice close-out,
  implementation close-out, RC promotion, and final release/tag validation.
- [Supported host and target matrix](../governance/supported-platforms.md)
  defines the supported Linux and macOS hosts, native and Wasm targets, and
  outstanding platform qualification.
- [Fleet ensure](../features/operations/fleet-ensure.md) documents the sole
  current convergence and interruption-replay workflow.
- [CI diagnostics](ci-diagnostics.md) distinguishes the one offline workspace
  Medic command from state-audit detail and live Fleet drift checks.
- [Recovery and retry runbooks](recovery-retry-runbooks.md) define the current
  same-release replay, ambiguous-result and operator retry safety procedures.
- [Release package and install validation](release-package-install-validation.md)
  records package, install, artifact, smoke-test, and environment-specific
  release gates.

Standing diagnostic, upgrade-state, and RC-readiness audit verdicts were
hard-cut during 0.92. Current evidence belongs to dated audit reports and the
active release-line closeout; this directory retains operator contracts and
validation procedures only.

## Fleet Operations

- [Fleet Ensure overview](../features/operations/fleet-ensure.md) selects the
  correct workflow for ordinary convergence, reinstall, bootstrap, automation,
  or recovery.
- [Desired state](../features/operations/fleet-ensure-desired-state.md) covers
  generation and the complete deployment contract.
- [Plan and apply](../features/operations/fleet-ensure-plan-and-apply.md) covers
  review, exact-digest approval, resumption, and no-effect replay.
- [Supplied infrastructure bootstrap](../features/operations/fleet-ensure-bootstrap-and-capacity.md#supplied-infrastructure-bootstrap)
  reviews explicit infrastructure IDs and Coordinator setup.
- [Capacity import](../features/operations/fleet-ensure-bootstrap-and-capacity.md#add-supplied-capacity-to-a-current-fleet)
  adds supplied canisters on an initialized Root's subnet.
- [Clean reinstall](../features/operations/fleet-ensure-clean-reinstall.md)
  replaces every pre-1.0 installation from selected physical inventory.
- [Recovery and cycle safety](../features/operations/fleet-ensure-recovery-and-cycle-safety.md)
  covers interruption, conservation, retirement, and exceptional recovery.
- [Fleet funding](fleet-funding.md) documents Coordinator funding, direct
  cycle top-up, and manual Root ICP conversion and recovery.
- [Backup and restore](../features/backup-and-restore/README.md) covers verified
  snapshots and journaled same-release recovery.
- [Local development Fleet](../features/operations/local-development-fleet.md)
  covers persistent PocketIC sessions and exact-session reset.

## Wasm Diagnostics

- [Wasm capability size report](wasm-capability-size-report.md) documents the
  focused machine-readable symbol-attribution helper. It is supporting
  diagnostic evidence, not a deployment gate or replacement audit method.

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

## Continue From Here

- [Read the Fleet operations guide](../features/operations/README.md)
- [Configure an App](../../CONFIG.md)
- [Choose the Canic features you need](../features/README.md)
- [Build your first managed application](../getting-started/minimal-managed-fleet.md)
- [Browse all documentation](../README.md)
