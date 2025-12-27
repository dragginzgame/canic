Below is a **cleaned-up, internally consistent, up-to-date version** of your document, aligned with everything we have established over the last few days. I have **not changed the architecture**, but I have corrected wording that would otherwise cause confusion or misplacement later.

Key fixes you’ll notice:

* Clarified what **model** actually owns (domain state + invariants, not just storage)
* Removed ambiguity around **ops being “only accessors”**
* Tightened **policy vs ops** responsibilities
* Made dependency direction explicit and correct
* Removed a couple of rules that would block legitimate patterns you already use

---

# Canic Layering & Responsibility Model

Each layer has a **single primary responsibility** and must depend **only on layers below it**.

Dependency direction (bottom → top):

```
model → ops → policy → workflow → endpoints/macros
dto is used outward-facing by ops/workflow/endpoints
```

---

## model/

### Purpose

* Owns **canonical domain state**
* Defines **stable memory layout**
* Enforces **local data invariants**
* Provides storage-level import/export

### Allowed

* Stable structures
* Registries and indexes
* Domain data types
* Structural invariants local to a type
* Serialization / encoding required for storage

### Forbidden

* Config access
* IC management calls
* Async code
* Timers
* Cross-entity business rules
* System-wide decisions

### Rule

> `model` defines what *is true* about stored state.
> It does not decide *what should happen next*.

---

## ops/

### Purpose

* Application-level access to model state
* Command and query façades
* Deterministic state mutation
* Adaptation from domain → DTO

This includes `ops/storage/*` and `ops/adapter/*`.

### Allowed

* Reading and writing model state
* Enforcing **application preconditions** (existence, ownership, mode, etc.)
* Returning `Result`, `Option`, or typed errors
* Translating domain data into DTOs / Views

### Forbidden

* IC management calls
* Async orchestration
* Timers
* Multi-step coordination
* Business policy or optimization logic

### Rule

> `ops` applies changes safely and deterministically,
> but does not decide *whether* those changes are desirable.

---

## policy/

### Purpose

* Pure decision-making and rule evaluation
* System-level invariants
* Placement, scaling, sharding, eligibility logic

### Allowed

* Reading config (directly or via ops)
* Reading state via ops
* Deterministic computations
* “Can we?” / “Should we?” decisions

### Forbidden

* State mutation
* IC calls
* Async
* Timers
* Side effects of any kind

### Rule

> `policy` decides, but never acts.

---

## workflow/

### Purpose

* Orchestration and lifecycle management
* Multi-step behavior over time
* Side-effects and coordination

### Allowed

* Async
* IC management calls
* Timers and retries
* Cascades and rollbacks
* State mutation **via ops**
* Calling policy to validate decisions

### Forbidden

* Direct model access
* Embedding policy logic inline
* Acting without going through ops

### Rule

> `workflow` is where things happen.

---

## endpoints / macros

### Purpose

* System boundary
* Auth, dispatch, wiring
* ABI / DTO marshalling

### Allowed

* Calling workflow or ops entrypoints
* Guards and authorization
* Input/output conversion

### Forbidden

* Direct model access
* Policy decisions
* Orchestration logic
* Hidden side effects

### Rule

> Endpoints wire requests to the system;
> they do not contain business behavior.

---

## dto/

### Purpose

* External and semi-external data contracts
* ABI, views, snapshots, metrics, logs

### Notes

* DTOs are **passive**
* DTOs may duplicate domain fields intentionally
* DTOs are allowed to be versioned (`dto::abi::v1`)

DTOs are **never mutated** and never enforce invariants.

---

## Common Classification Examples

| Code does…                        | Layer         |
| --------------------------------- | ------------- |
| Stores parent/child relationships | model         |
| Reads stable registry             | ops           |
| Writes stable registry            | ops           |
| Enforces singleton cardinality    | policy        |
| Chooses a shard                   | policy        |
| Validates eligibility             | policy        |
| Creates a canister                | workflow      |
| Cascades state                    | workflow      |
| Schedules a timer                 | workflow      |
| Converts domain → view            | ops (adapter) |

---

## Architectural Placement Checklist

When placing code, ask:

1. Does this **store canonical state**? → model
2. Does this **read or mutate state deterministically**? → ops
3. Does this **decide whether something is allowed or optimal**? → policy
4. Does this **coordinate steps or perform effects**? → workflow

If more than one answer applies, the code must be **split**.

---

## Non-Goals

* This architecture does not optimize for minimal directories.
* Thin layers are acceptable if responsibilities are clear.
* Duplication across layers is allowed when representations differ.
* Clarity beats cleverness.

---

## Enforcement

* All new code must conform to this model.
* Refactors should migrate **one feature end-to-end** (model → workflow).
* Avoid partial migrations that blur responsibilities.
* When in doubt, **split the code**, not the layer.

---

### Final note

This document is now **internally consistent**, matches the refactors you’ve already made, and is safe to hand to:

* junior developers
* reviewers
* future you in six months

If you want, the next step would be:

* adding **one concrete worked example** (e.g. “creating a worker”) mapped across layers
* or turning this into a short `ARCHITECTURE.md` with diagrams
