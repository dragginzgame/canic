# Post-46 Backlog: CI, GitOps, And Provenance

## Status

TBD.

This is a post-0.46 backlog topic. It captures CI/CD, GitOps, and
supply-chain needs that should build on deployment truth, authority, execution,
promotion, lifecycle, comparison, registry, and adoption foundations. It is
not a promised numbered follow-on release.

---

## Goal

A future line may make deployment commands reliable automation inputs without
weakening live-state validation.

Core CI question:

```text
Can automation plan, check, gate, sign, and record deployments using stable
machine-readable evidence?
```

---

## Dependency On Completed Deployment Foundation

This topic consumes:

- 0.41 plans, inventories, safety reports, receipts, and artifact manifests;
- 0.42 authority reconciliation;
- 0.43 executor result objects and execution context;
- 0.44 promotion reports;
- 0.45 external lifecycle proposals and receipts;
- 0.46 deployment comparisons;
- post-46 artifact registry metadata where available;
- post-46 adoption profiles where CI is onboarding or checking a project.

0.43 internal executor result shapes are not the public JSON contract.

This topic would own the stable public JSON schema, exit-code contract, CI
wrappers, signed plans and receipts, and public project manifest contract if it
is promoted into a real release line.

---

## Core Decision

CI/GitOps consumes the same deployment truth objects.

Signed plans, JSON, and CI locks improve automation, but they do not replace
live inventory.

---

## Machine-Readable Interfaces

Stable machine-readable output should use an envelope:

```text
JsonEnvelopeV1 {
  schema_version,
  command,
  generated_at,
  deployment_id,
  result_kind,
  payload,
  warnings,
  hard_failures,
}
```

Exit codes should be coarse and policy-friendly:

```text
ExitCodeClassV1 {
  ok,
  diff_found,
  unsafe_blocked,
  external_action_required,
  invalid_input,
  execution_failed,
}
```

The JSON payload can carry completed deployment foundation objects and any
promoted post-46 backlog objects. The envelope makes command parsing stable
without forcing every internal model to be frozen forever.

---

## Provenance And Attestation

Build provenance should be recorded where available:

```text
BuildProvenanceV1 {
  source_revision,
  workspace_dirty,
  allow_dirty_recorded,
  cargo_lock_hash,
  builder_version,
  rust_toolchain,
  build_profile,
  build_environment_digest,
}
```

Signed receipts wrap receipt identity, not live truth:

```text
SignedReceiptEnvelopeV1 {
  receipt_id,
  receipt_digest,
  signature_kind,
  signature,
  signer,
}
```

Signed plans and receipts help policy gates and promotion workflows. They do
not prove current deployment state without fresh inventory.

---

## CI Locks

CI needs explicit operation locking so two jobs do not mutate the same
deployment concurrently.

Tentative shape:

```text
CiDeploymentLockV1 {
  deployment_id,
  lock_id,
  holder,
  expires_at,
  observed_epoch,
}
```

Locks should be leases with explicit expiry and observed deployment epoch.
Apply phases must still verify current live state before mutation.

---

## Project Manifest

This topic would own the public project manifest contract for split
repositories and CI roots if promoted into a real release line.

Tentative shape:

```text
ProjectManifestV1 {
  workspace_root,
  icp_root,
  deployment_group,
  declaration_output,
  default_json,
}
```

The manifest should bind operator commands to the intended project roots. It
should not make filesystem layout part of `DeploymentPlanV1`.

---

## Data Model

Core objects:

```text
JsonEnvelopeV1
ExitCodeClassV1
BuildProvenanceV1
SignedReceiptEnvelopeV1
CiDeploymentLockV1
ProjectManifestV1
```

These objects wrap and reference deployment truth artifacts. They do not create
an independent deployment state model.

---

## Commands

Tentative operator surface:

```text
canic deploy plan --json
canic deploy check --json
canic deploy receipt sign
canic deploy lock acquire
canic deploy lock release
canic project validate
```

Command names are tentative. The stable part is the JSON envelope, exit-code
classes, provenance, and lock model.

---

## Non-Goals

This future work should not:

- make signed receipts truth;
- let CI bypass live inventory;
- let CI bypass authority reconciliation;
- require every local workflow to use GitOps;
- turn project manifests into deployment plans;
- require signed receipts before ordinary local development can work.

---

## Implementation Slices

### Slice 1: JSON Envelope

Define stable envelopes for deployment command output.

### Slice 2: Exit-Code Contract

Map safety, diff, external-action, and execution outcomes to stable exit-code
classes.

### Slice 3: Provenance Fields

Record build provenance in artifact manifests and receipts.

### Slice 4: Project Manifest

Define and validate the public project manifest contract.

### Slice 5: CI Lock

Add deployment-scoped lock acquire, refresh, and release behavior.

### Slice 6: Signed Receipts And Plans

Add optional signing envelopes around plan and receipt digests.

### Slice 7: GitHub And GitLab Wrappers

Publish thin CI wrappers around the stable command contract.

---

## Exit Criterion

This topic is ready to promote into a real release line when:

```text
CI can run plan, check, and diff with stable machine-readable output, policy
gates, and provenance without weakening live-state validation.
```
