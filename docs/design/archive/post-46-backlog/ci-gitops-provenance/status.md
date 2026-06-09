# Post-46 Backlog Status: CI/GitOps Provenance

Last updated: 2026-06-09

## Purpose

This file is the archived status log for a post-46 backlog topic.

## Current State

Partially superseded by 0.51, 0.52, and 0.53. Remaining CI locks, optional
signing/attestation, release evidence, and provider wrappers are optional idea
material, not active release requirements.

This topic depends on deployment truth, receipts, artifact provenance, and
promotion state so CI/GitOps can prove what it intended and what was actually
deployed.

0.51 promoted and implemented the stable public evidence envelope and
exit-class contract. 0.52 implemented source/build/artifact provenance. 0.53
implemented CI policy gates and project evidence manifests. Remaining work is
CI locks, optional signing/attestation, machine-readable release evidence, and
provider wrappers.

## Implemented

- 0.51 stable evidence envelope:

  ```text
  EvidenceEnvelopeV1
  ```

- 0.51 stable exit taxonomy:

  ```text
  ExitClassV1
  ```

- 0.51 passive envelope emitters and comparison:

  ```text
  canic fleet adoption report <fleet> --profile <profile> --format envelope-json
  canic deploy check <deployment> --format envelope-json
  canic evidence compare --left <path> --right <path>
  ```

- 0.52 source/build/artifact provenance:

  ```text
  canic build <fleet> <role> --provenance <path>
  ```

- 0.53 policy gates and project evidence manifests.

## Not Implemented Yet

- Machine-readable release evidence.
- CI lock acquire/refresh/release behavior.
- Optional signing/attestation workflow.
- Provider-specific CI wrappers.

## Drift Log

- 2026-05-31: stable envelope and exit-class backlog items are superseded by
  the implemented 0.51 design.
- 2026-06-09: build provenance and project evidence manifests are superseded
  by 0.52 and 0.53; remaining automation backlog moved to optional ideas.

## Promotion Criteria

Do not implement locks, signing, or provider wrappers unless they preserve the
existing passive evidence boundaries and do not create deployment authority.
