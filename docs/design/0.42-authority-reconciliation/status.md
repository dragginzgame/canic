# 0.42 Status: Authority Reconciliation

Last updated: 2026-05-23

## Purpose

This file is the permanent implementation status log for the 0.42 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

`0.42.1` ready for validation/push.

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
- Added `AuthorityReportV1` and read-only
  `canic deploy authority report <fleet>` output as the summarized
  operator-facing authority view.
- Expanded external-action records so plan and report output include the full
  subject, control class, state, observed controller set, desired controller
  set, action, and reason.
- Added expected and observed pool canisters to the dry-run authority plan:
  expected pools with unobserved controller state are explicit unknown
  external-action records, and unplanned observed pool canisters are reported
  as adoption/external-action cases.
- Added explicit `AuthorityAutomaticActionV1` records to the dry-run plan and
  report surfaces so future apply logic has a narrow list of automatic
  candidates with observed/desired controller evidence.
- Authority reports now include next-action guidance for safe dry-run plans
  that contain automatic candidates.
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
