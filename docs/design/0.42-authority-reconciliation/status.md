# 0.42 Status: Authority Reconciliation

Last updated: 2026-05-22

## Purpose

This file is the permanent implementation status log for the 0.42 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Not started.

0.42 depends on 0.41 establishing deployment truth objects, observed inventory,
diffs, safety reports, and installer gating.

## Implemented

- No 0.42 implementation work has landed yet.

## Not Implemented Yet

- Authority reconciliation against observed controller sets.
- Explicit handling for root/controller drift.
- Reconciliation planning for deployment-controlled, pool-managed, imported,
  jointly controlled, and user-controlled canisters.
- Operator-visible authority change reports.

## Drift Log

- No implementation drift recorded yet.

## Release Bar

0.42 should not start in earnest until 0.41 can reliably say what the intended
authority model is and what live authority state was observed.

