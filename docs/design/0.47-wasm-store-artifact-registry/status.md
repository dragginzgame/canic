# 0.47 Status: Wasm-Store Artifact Registry

Last updated: 2026-05-22

## Purpose

This file is the permanent implementation status log for the 0.47 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Not started.

0.47 depends on artifact truth and promotion work so the wasm-store can become
an explicit artifact registry rather than a hidden deployment helper.

## Implemented

- No 0.47 implementation work has landed yet.

## Not Implemented Yet

- Wasm-store artifact registry model.
- Registry identity and provenance records.
- Artifact lookup and promotion integration.
- Safety checks connecting registry entries to deployment truth.

## Drift Log

- No implementation drift recorded yet.

## Release Bar

0.47 should not close until wasm-store artifact records can be audited as part
of deployment truth and promotion provenance.

