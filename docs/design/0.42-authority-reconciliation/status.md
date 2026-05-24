# 0.42 Status: Authority Reconciliation

Last updated: 2026-05-24

## Purpose

This file is the permanent implementation status log for the 0.42 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

`0.42.10` is live. Work is continuing on the 0.42 line by tightening the
dry-run report/evidence model while keeping reconciliation read-only.

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
  observation gaps, and unplanned observed pool canisters are reported as
  adoption/external-action cases.
- Added explicit `AuthorityAutomaticActionV1` records to the dry-run plan and
  report surfaces so future apply logic has a narrow list of automatic
  candidates with observed/desired controller evidence.
- Authority reports now include next-action guidance for safe dry-run plans
  that contain automatic candidates.
- Added read-only dry-run authority receipts and evidence bundles.
- Added hard authority findings for staging/emergency category overlap with the
  normal expected controller set.
- Narrowed `external_actions_required` so it records actual external authority
  actions only. Unknown controller observations are reported as observation
  gaps; unsafe blockers are reported as hard failures.
- Authority dry-run receipts now preserve unresolved controller-observation
  gaps directly, so the standalone receipt output remains complete enough to
  explain missing authority evidence.
- Authority dry-run reports now include an explicit `apply_readiness` summary
  that distinguishes automatic-action candidates from hard failures, missing
  observation blockers, and required external actions.
- Authority actions, automatic-action candidates, external-action records, and
  dry-run receipt observations now carry typed controller deltas so consumers
  can read exact add/remove sets without recomputing them from observed and
  desired controller lists.
- Authority dry-run receipts now include the source authority report ID so
  standalone receipt provenance remains explicit outside the full evidence
  bundle.
- Authority reports now carry the inventory ID and authority profile hash from
  the reconciliation plan, making standalone report output self-describing.
- Authority reports and dry-run receipts now carry check IDs, inventory IDs,
  and authority profile hashes, matching evidence-bundle provenance so
  standalone outputs remain self-describing.
- Authority dry-run receipt construction rejects mismatched report/plan/check
  provenance instead of producing mixed evidence.
- Authority dry-run receipt construction also rejects report content that no
  longer matches the reconciliation plan's automatic actions, hard findings, or
  external actions.
- Complete authority dry-run evidence bundles are validated before CLI output
  so top-level evidence IDs cannot disagree with nested report or receipt
  provenance.
- Evidence validation also rejects attempted controller actions and mutated
  controller observations inside dry-run receipts.
- `canic deploy authority check|evidence|report|receipt --format text` now
  renders the existing authority DTOs as deterministic human-oriented
  summaries while JSON remains the default machine-readable output. The text
  renderers preserve per-canister decisions and detailed hard-failure,
  observation-gap, and external-action evidence, and live in the host
  deployment-truth layer rather than CLI-only formatting code. Text output
  also includes evidence generation time and controller add/remove deltas for
  automatic and external authority actions, plus verified controller
  observations with observed and desired controller sets.
- Authority dry-run evidence validation now rejects schema-version drift and
  receipts whose operation status or command result no longer represents a
  completed successful dry run. It also recomputes report summaries from the
  reconciliation plan and rejects mutated report counts, readiness, breakdowns,
  observation gaps, or next actions. Completed dry-run receipts must include
  `finished_at`, and evidence `generated_at` must match that completion time.
  This is a passive evidence-coherence guard; it does not make receipts
  authority over live controller state.
- Authority dry-run evidence validation now rejects blank required identity
  fields and full evidence bundles whose nested report or receipt omits source
  check provenance. Completed receipts also reject `finished_at` timestamps
  earlier than `started_at`. Authority dry-run evidence bundle construction now
  lives in `canic-host` deployment-truth code, authority report construction
  from a full deployment check has a host-owned helper, and local authority
  report/receipt/evidence IDs are generated by the host layer. CLI authority
  tests now cover parsing, format rejection, and host-helper delegation, while
  detailed authority DTO and text-rendering behavior stays in `canic-host`.
  The four read-only authority CLI leaves now share one parse/load/render
  helper and explicitly test JSON as the default authority output format. This
  keeps the CLI as a consumer of validated host evidence and keeps archived
  evidence self-contained without becoming authority over live controller state.
- Authority apply-readiness blockers now distinguish unsafe canister authority
  from other hard authority findings. Unsafe canister hard-failure evidence is
  still preserved in the report and receipt, but report counts and next-action
  guidance no longer double-count it as a separate hard authority-profile
  finding. Blocked authority reports also keep external-action and
  missing-observation next actions alongside unsafe/hard blocker guidance
  instead of hiding that follow-up work until the blockers are resolved, and
  blocked report summaries now include those warning-level counts when they
  coexist with blocking authority findings. Reports with blockers also keep
  next-action guidance for automatic dry-run candidates, so reviewable
  controller changes stay visible even when they cannot be applied yet.
  Evidence validation now has explicit regression coverage for mutated
  unsafe-blocker readiness, keeping archived evidence tied to the report model
  that produced it.
- Standalone dry-run receipt construction now rejects unsupported source
  schema versions, missing source report check provenance, blank receipt
  identity inputs, and missing completion timestamps before emitting receipt
  evidence.
- The first planner reports:
  - already-correct controller sets;
  - deployment-controlled controller deltas that can be applied automatically
    by a later apply path;
  - external-action cases for pool, imported, jointly controlled, and
    user-controlled canisters;
  - unsafe blocked cases for unknown/unsafe canisters.

## Not Implemented Yet

- Apply path for safe automatic controller changes.
- Post-apply re-inventory and authority receipts.
- Pool ownership reconciliation beyond dry-run classification.
- Controller-mutating authority change reports.

## Drift Log

- The implementation followed the post-0.41 report-first boundary more closely
  than early 0.42 design text. The design has been narrowed so the 0.42
  release bar is dry-run reconciliation, exact external-action reporting,
  read-only receipts/evidence, hard authority findings, and no controller
  mutation. Apply, remote lock/epoch checks, pool mutation, and post-apply
  verification remain promoted-or-later work unless explicitly accepted into
  the line.

## Release Bar

0.42 is releasable once Canic can produce a dry-run authority reconciliation
plan from 0.41 deployment truth and explain exact external actions without
mutating controller state. Applying safe controller changes remains optional or
later unless explicitly promoted into this line.
