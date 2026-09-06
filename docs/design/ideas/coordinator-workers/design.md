# Idea: Coordinator Workers

Reviewed: 2026-09-06

## Status

- Deferred and unnumbered; no implementation or release is approved.
- Priority: low until a measured Coordinator bottleneck justifies another
  infrastructure canister role.
- Owner: Fleet Coordinator for policy, assignment and lifecycle authority.
- Repository scope: Canic only.

## Current Boundary

The Coordinator already owns Root funding requests, exact acceptance receipts
and bounded recovery in its
[workflow](../../../../crates/canic-control-plane/src/workflow/fleet_coordinator/mod.rs).
A Worker is not required to provide Root funding or ordinary Fleet operation.

The Coordinator remains the sole Fleet Registry writer. Roots retain
Component lifecycle and local estate authority. Adding execution partitions
must not create a second owner of either contract.

## Retained Direction

If measured cardinality or throughput exceeds the existing bounded design,
the Coordinator could assign one typed responsibility to a bounded Worker.
A Worker would:

- bind to one exact Coordinator, Fleet, assignment and operation;
- retain only the observations, intents and receipts for that assignment;
- perform effects under explicit finite budget and scope;
- return bounded progress and completion evidence; and
- have no Fleet Registry write authority, Component lifecycle authority,
  application-data routing or recursive worker creation.

Root funding is only a possible workload. Before choosing it, show why the
current request-driven funding protocol cannot satisfy the measured need.
Do not introduce heartbeats, central polling or Worker wallets just to create
work for a new layer. Any delegated executor must preserve one canonical
funding intent/receipt owner and exact lost-response reconciliation.

## Evidence Before Scheduling

- A measured bottleneck under a concrete supported Fleet size and workload.
- Comparison with bounded batching, indexing and scheduling in the existing
  Coordinator, including added cross-Subnet calls and operating cost.
- One selected Worker responsibility, final terminology, artifact carrier,
  placement authority and finite count/assignment limits.
- Exact assignment activation, draining, revocation and failure behavior
  without two executors owning the same effect.
- Intent-before-effect creation, bounded debit and uncertain-result recovery.
- Same-release interruption/restore, Worker failure isolation and terminal
  effect-free replay in PocketIC.
- Runtime/build/validation budgets and a complete accepted batch.

## Disposition

Retain only as a scaling contingency. The speculative Root heartbeat/top-up
architecture and former release plan are removed. If existing Coordinator
mechanisms meet the measured workload, drop the Worker proposal rather than
adding another infrastructure role.
