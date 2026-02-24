# Canic Architecture Contract

This document is the canonical architecture contract for Canic.

If implementation and this contract diverge, implementation must be updated.

## Dependency Direction (Non-Negotiable)

```text
endpoints/macros
    ->
workflow
    ->
policy
    ->
ops
    ->
model
```

Rules:
- Dependencies are one-way only.
- Lower layers must not depend on higher layers.
- `dto` is a transfer format for endpoints/workflow/ops.
- `model` and `policy` must not depend on `dto`.

## Layer Contracts

### `endpoints/` and macros
- Define the IC boundary: auth, guards, argument/response marshaling, dispatch.
- Delegate immediately to workflow or ops.
- Must not include policy logic, orchestration, or direct model access.

### `workflow/`
- Coordinates multi-step behavior over time.
- May sequence ops calls, run timers, and handle retries/rollback.
- Must not mutate model directly.
- Must not embed policy logic inline.

### `policy/`
- Pure decision layer: deterministic, side-effect free.
- Evaluates state passed in as input; decides allow/deny/placement/eligibility.
- Must not perform IO, mutation, async, timers, or serialization.

### `ops/`
- Deterministic model access and mutation boundary.
- Owns Record/View/DTO conversion at storage boundaries.
- May perform approved single-step platform effects as system facade.
- Must not orchestrate multi-step workflows.
- Must not encode business decision logic.

### `model/`
- Authoritative domain state and local structural invariants.
- Stable memory layout and persisted schemas live here.
- Must not perform policy decisions, async orchestration, or platform calls.

## Data Boundary Contracts

### `dto/`
- Boundary payloads only.
- Data only (no logic, no ops/workflow/model calls).
- Commands/requests must not implement `Default`.

### `view/`
- Internal read-only projections.
- Not persisted as canonical state.

### `storage/**` records
- Persisted schema only.
- Record conversions are owned by ops adapters.

## Lifecycle Contract

- Lifecycle macros remain thin and synchronous.
- `init`/`post_upgrade` restore minimal environment synchronously.
- Async bootstrap work is scheduled (never awaited) via timers.
- Lifecycle adapters do not bypass ops/policy boundaries.

## Security Boundary Contract

- Authentication is enforced at endpoints/access predicates.
- Workflow and ops assume authenticated input.
- Model and policy must not embed endpoint auth concerns.

For delegated token and signature invariants, see:
- `docs/contracts/ACCESS_ARCHITECTURE.md`
- `docs/contracts/AUTH_DELEGATED_SIGNATURES.md`

## Enforcement

- New code must satisfy this contract end-to-end.
- If one feature spans multiple layers, migrate all layers coherently.
- If code appears to belong to multiple layers, split it.
