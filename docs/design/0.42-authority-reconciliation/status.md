# 0.42 Status: Authority Reconciliation

Last updated: 2026-05-23

## Purpose

This file is the permanent implementation status log for the 0.42 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

`0.42.0` ready for push.

0.42 depends on 0.41 establishing deployment truth objects, observed inventory,
diffs, safety reports, and installer gating.

## Implemented

- Added the first passive authority reconciliation model:
  `AuthorityReconciliationPlanV1`, `CanisterAuthorityActionV1`,
  `AuthorityExternalActionV1`, `AuthorityActionV1`, and
  `AuthorityReconciliationStateV1`.
- Added a dry-run authority reconciliation planner that consumes
  `DeploymentCheckV1` and classifies observed controller state without
  mutating IC state.
- Added read-only `canic deploy authority check <fleet>` output for the
  authority reconciliation plan.
- The first planner reports:
  - already-correct controller sets;
  - deployment-controlled controller deltas that can be applied automatically
    by a later apply path;
  - external-action cases for pool, imported, jointly controlled, and
    user-controlled canisters;
  - unsafe blocked cases for unknown/unsafe canisters.

## Not Implemented Yet

- Rich operator-facing authority reports beyond raw JSON plan output.
- Apply path for safe automatic controller changes.
- Post-apply re-inventory and authority receipts.
- Pool ownership reconciliation beyond dry-run classification.
- Operator-visible authority change reports.

## Drift Log

- No implementation drift recorded yet.

## Release Bar

0.42 is releasable once Canic can produce a dry-run authority reconciliation
plan from 0.41 deployment truth and explain exact external actions without
mutating controller state. Applying safe controller changes remains optional or
later unless explicitly promoted into this line.
