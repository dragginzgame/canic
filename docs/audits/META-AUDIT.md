# Canic Meta-Audit Contract

This document defines the architecture contracts every audit evaluates.

## 1. Layering Invariants

Dependency flow must remain:
- `endpoints -> workflow -> policy -> ops -> model`

Hard rules:
- `policy` is pure (no storage/IC calls/async/serialization side effects).
- `workflow` orchestrates; it must not mutate model records directly.
- DTO/record/view conversions belong in `ops` adapters, not `workflow`.

## 2. Root Capability and Replay Invariants

- Capability proof must be validated before execution.
- Replay metadata (`request_id`, TTL) must be validated before mutation.
- Duplicate-same requests should return cached response where applicable.
- Replay reservations created before execution must not leak on policy denial/failure.

## 3. Lifecycle Invariants

- Lifecycle adapters are synchronous glue only.
- Init/post-upgrade must schedule async bootstrap; never await in lifecycle hooks.
- Root/non-root restoration must keep env invariants intact (`root_pid`, `parent_pid`, `subnet_pid`, role consistency).

## 4. Security and Access Invariants

- Endpoint auth guards enforce caller boundaries.
- Root/internal endpoints must reject unauthorized callers.
- Delegation/attestation claims must bind subject/caller/audience/subnet/epoch correctly.

## 5. Data Ownership Invariants

- `model` owns truth/state.
- `ops` owns deterministic state access and bounded side effects.
- `workflow` owns multi-step coordination.
- `dto` is passive transfer shape only.

## 6. Audit Evidence Standard

Findings must be evidence-based and include:
- concrete file path(s)
- function/type names
- command output where relevant
- risk classification rationale
