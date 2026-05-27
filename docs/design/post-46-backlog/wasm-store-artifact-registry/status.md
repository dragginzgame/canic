# Post-46 Backlog Status: Wasm-Store Artifact Registry

Last updated: 2026-05-27

## Purpose

This file is the status log for a post-46 backlog topic. It is not an approved
numbered release line.

## Current State

Topic implementation not started.

This topic depends on artifact truth and promotion work so the wasm-store can
become an explicit artifact registry rather than a hidden deployment helper.
The 0.41-0.46 foundation made wasm-store evidence visible to deployment truth
and promotion, but no post-46 registry, provenance, pinning, or GC model has
been promoted or implemented.

## Implemented

- No implementation work for this backlog topic has landed yet.

## Not Implemented Yet

- Wasm-store artifact registry model.
- Registry identity and provenance records.
- Artifact lookup and promotion integration.
- Safety checks connecting registry entries to deployment truth.

## Drift Log

- No implementation drift recorded yet.

## Promotion Criteria

Do not promote this into a numbered release line until wasm-store artifact
records can be audited as part of deployment truth and promotion provenance.
