# 0.46 Status: Multi-Deployment Operations

Last updated: 2026-05-22

## Purpose

This file is the permanent implementation status log for the 0.46 design line.
The design document captures intent; this status file records what actually
landed, what drifted, and what remains open.

## Current State

Not started.

0.46 depends on deployment truth, promotion, and external lifecycle state so
multiple deployment targets can be compared and operated without conflating
template identity with live deployment identity.

## Implemented

- No 0.46 implementation work has landed yet.

## Not Implemented Yet

- Multi-deployment identity and grouping model.
- Cross-deployment diff reports.
- Multi-target operator workflows.
- Deployment drift and promotion views across related deployments.

## Drift Log

- No implementation drift recorded yet.

## Release Bar

0.46 should not close until Canic can distinguish fleet templates, deployment
targets, and deployment groups in operator-visible workflows.

