# 0.44 Status: Artifact Promotion

Last updated: 2026-05-22

## Purpose

This file is the permanent implementation status log for the 0.44 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Not started.

0.44 depends on deployment truth and backend-agnostic receipts so artifact
promotion can prove what was built, uploaded, installed, and promoted.

## Implemented

- No 0.44 implementation work has landed yet.

## Not Implemented Yet

- Artifact promotion model.
- Promotion receipts and provenance.
- Promotion safety checks across deployment targets.
- Integration with wasm-store artifact identity.

## Drift Log

- No implementation drift recorded yet.

## Release Bar

0.44 should not close until promoted artifacts carry enough provenance to be
checked against deployment truth before use.

