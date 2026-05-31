# Post-46 Backlog: CI, GitOps, And Provenance

## Status

Partially superseded by 0.51.

This post-0.46 backlog topic originally captured CI/CD, GitOps, and
supply-chain needs that should build on deployment truth, authority, execution,
promotion, lifecycle, comparison, registry, and adoption foundations.

0.51 promoted and implemented the stable evidence-envelope and exit-class part
of this backlog as:

```text
docs/design/0.51-ci-gitops-provenance-evidence-envelopes/0.51-design.md
```

Source/build/artifact provenance is now proposed as the 0.52 line:

```text
docs/design/0.52-source-build-artifact-provenance/0.52-design.md
```

The remaining backlog items outside 0.52 are CI locks, project manifest
semantics, optional signing/attestation, and provider wrappers. This document
is retained as historical design source material, not as a competing active
envelope or provenance design.

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

0.51 now owns the stable evidence-envelope schema and exit-class taxonomy.
0.52 proposes source/build/artifact provenance. Future lines may still own CI
wrappers, signed plans and receipts, deployment locks, and a public project
manifest contract.

---

## Core Decision

CI/GitOps consumes the same deployment truth objects.

Signed plans, JSON, and CI locks improve automation, but they do not replace
live inventory.

---

## Machine-Readable Interfaces

Stable machine-readable output now uses the 0.51 evidence envelope:

```text
EvidenceEnvelopeV1
```

The stable exit taxonomy is:

```text
ExitClassV1
```

The envelope currently wraps selected passive command payloads without freezing
every nested DTO. New automation work should extend or consume
`EvidenceEnvelopeV1`; it should not introduce another public JSON envelope for
the same role.

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
EvidenceEnvelopeV1
ExitClassV1
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
canic deploy plan <deployment>
canic deploy check <deployment>
canic deploy receipt sign
canic deploy lock acquire
canic deploy lock release
canic project validate
```

Current deployment-truth commands print JSON by default; this topic would own a
stable public evidence envelope rather than relying on raw internal payloads.
0.51 now owns that envelope through `--format envelope-json` and
`canic evidence compare`. Future commands here should consume that surface
instead of inventing a second envelope.

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

### Slice 1: Evidence Envelope

Completed in 0.51. `EvidenceEnvelopeV1` wraps selected passive command output.

### Slice 2: Exit-Class Contract

Completed in 0.51 for the current envelope emitters. `ExitClassV1` maps
success, warnings, blockers, evidence conflicts, missing required evidence,
invalid input, execution failure, and internal errors into stable automation
classes.

### Slice 3: Provenance Fields

Remaining. Record build provenance in artifact manifests and receipts.

### Slice 4: Project Manifest

Remaining. Define and validate the public project manifest contract.

### Slice 5: CI Lock

Remaining. Add deployment-scoped lock acquire, refresh, and release behavior.

### Slice 6: Signed Receipts And Plans

Remaining. Add optional signing envelopes around plan and receipt digests.

### Slice 7: GitHub And GitLab Wrappers

Remaining. Publish thin CI wrappers around the stable command contract.

---

## Exit Criterion

This topic is ready to promote into a real release line when:

```text
CI can run plan, check, and diff with stable machine-readable output, policy
gates, and provenance without weakening live-state validation.
```
