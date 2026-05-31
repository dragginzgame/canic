# Post-46 Backlog Status: CI/GitOps Provenance

Last updated: 2026-05-31

## Purpose

This file is the status log for a post-46 backlog topic. It is not an approved
numbered release line.

## Current State

Partially superseded by 0.51.

This topic depends on deployment truth, receipts, artifact provenance, and
promotion state so CI/GitOps can prove what it intended and what was actually
deployed.

0.51 promoted and implemented the stable public evidence envelope and
exit-class contract. Source/build/artifact provenance is now proposed as the
0.52 line. Remaining work outside 0.52 is CI locks, project manifest
semantics, optional signing/attestation, and provider wrappers.

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

## Not Implemented Yet

- Source/build/artifact provenance model. Proposed in 0.52.
- Commit/build/deployment linkage.
- Machine-readable release evidence.
- CI lock acquire/refresh/release behavior.
- Public project manifest contract.
- Optional signing/attestation workflow.
- Provider-specific CI wrappers.

## Drift Log

- 2026-05-31: stable envelope and exit-class backlog items are superseded by
  the implemented 0.51 design.

## Promotion Criteria

Do not promote this into a numbered release line until automated deployment
evidence can be tied back to source, build artifacts, deployment plans, and
receipts.
