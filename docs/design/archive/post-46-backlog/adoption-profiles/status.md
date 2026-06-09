# Post-46 Backlog Status: Adoption Profiles

Last updated: 2026-06-09

## Purpose

This file is the archived status log for a post-46 backlog topic.

## Current State

Partially superseded by 0.50. Remaining active adoption/import work is optional
idea material, not an active release requirement.

This topic depends on deployment truth and authority reconciliation so adoption
can be an explicit profile-driven operation rather than a best-effort inference
from current config.
The 0.41-0.46 foundation supplies inventory, authority, and lifecycle evidence.
0.50 promoted and implemented the passive adoption profile model and
read-only adoption reports. Active adoption/import remains unfinished.

## Implemented

- 0.50 passive adoption profiles.
- 0.50 read-only adoption report model.
- Non-executing adoption recommendations and safety findings.

## Not Implemented Yet

- Active import/adoption safety checks.
- Adoption receipts and operator reports for executed adoption.
- Guardrails against silently blessing foreign state.

## Drift Log

- 2026-06-09: passive adoption is superseded by 0.50; active adoption/import
  moved to optional ideas.

## Promotion Criteria

Do not implement active adoption/import unless it requires explicit observed
facts, explicit operator intent, and a clear safety report.
