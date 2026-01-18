Below is a **clean, tightened rewrite** of your document.
I have preserved your intent and structure, but I have:

* Removed ambiguity and self-contradictions
* Normalized terminology across layers
* Made rules *testable* rather than aspirational
* Clarified where async is allowed and why
* Strengthened the separation between **decision**, **mutation**, and **orchestration**
* Reduced opportunities for “interpretive compliance”

This version is suitable as a **canonical architecture contract**.

---

# Canic Architecture Model

This document defines the **enforceable architectural boundaries** of the Canic core.
Each layer has a single responsibility and a clear dependency direction.

> If code appears to belong to more than one layer, it **must be split**.

---

## Dependency Direction (Non-Negotiable)

```
endpoints/macros
        ↓
     workflow
        ↓
      policy
        ↓
        ops
        ↓
       model
        ↓
       infra
```

* Dependencies are **one-way**
* Lower layers must never depend on higher layers
* Lateral dependencies are forbidden unless explicitly documented

---

## model/

### Responsibility

Owns **canonical domain state**.

The model layer defines:

* what is stored
* how it is stored
* what is structurally valid at rest

### Guarantees

* Stable memory layout
* Deterministic serialization
* Local structural invariants

New stable memory IDs MUST be allocated within an existing module’s reserved range.
Creating a new module or expanding a range requires an explicit architectural change.

### Allowed

* Stable data structures
* Registries, indexes, and keys
* Domain data types
* Type-local invariants (e.g. uniqueness within a registry)
* Import/export of stored state
* Encoding required for stable memory

### Forbidden

* Config access
* IC or system calls
* Async code
* Timers
* Cross-entity or system-wide rules
* Business decisions
* Lifecycle coordination

### Rule

> `model` defines what **is true** about persisted state.
> It never decides what should happen next.

---

## infra/

### Responsibility

Provides **raw platform capabilities**.

Infra answers:

> “How do we talk to the platform?”

### Guarantees

* No business meaning
* No application semantics
* Reusable outside Canic

### Allowed

* IC management canister calls
* Inter-canister call primitives
* Cryptography and signatures
* Timers, randomness, system clocks
* Performance counters
* Platform-required serialization
* Pure utility code

### Forbidden

* Domain concepts (roles, pools, shards, parents, children)
* Domain identifiers or enums
* Model access or mutation
* Ops or policy logic
* Workflow orchestration
* Authorization checks
* DTO assembly or pagination

### Rule

> `infra` provides **capabilities**, not **meaning**.
> It must be usable without knowledge of the application.

---

## ops/

### Responsibility

Provides **deterministic access to canonical state**.

Ops is the *only* layer allowed to mutate the model.

### Guarantees

* Deterministic behavior
* No timing dependence
* No retries or orchestration
* No policy decisions

### Allowed

* Reading and writing model state
* Enforcing **application preconditions**

  * existence
  * ownership
  * mode
  * invariant preservation
* Returning `Result`, `Option`, or typed errors
* Translating domain state into DTOs / views
* Wrapping infra capabilities with application semantics

### Forbidden

* Business policy or optimization logic
* “Should we?” decisions
* Multi-step workflows
* Cross-canister orchestration
* Async control flow that alters outcomes

### Rule

> `ops` applies changes **safely and deterministically**,
> but never decides whether those changes are desirable.

---

## policy/

### Responsibility

Pure **decision-making**.

Policy answers:

* “Is this allowed?”
* “Is this valid?”
* “Is this optimal?”

### Guarantees

* Deterministic
* Side-effect free
* Testable in isolation

### Allowed

* Reading configuration
* Evaluating observed state passed in by workflow
* Pure computation
* Eligibility, placement, scaling, sharding logic

### Forbidden

* State mutation
* IC calls
* Async
* Timers
* Side effects of any kind
* Storage or infra types

### Rule

> `policy` decides, but never acts.

---

## workflow/

### Responsibility

**Orchestration and lifecycle management**.

Workflow is where *things actually happen*.

### Guarantees

* Explicit sequencing
* Explicit failure handling
* Explicit side-effects

### Allowed

* Async execution
* IC management calls (via infra or ops wrappers)
* Timers, retries, and scheduling
* Cascades and rollbacks
* State mutation **only via ops**
* Calling policy to validate decisions

### Forbidden

* Direct model access
* Inline policy logic
* Bypassing ops for state changes

### Rule

> `workflow` coordinates behavior over time.

---

## endpoints / macros

### Responsibility

Defines the **system boundary**.

Endpoints translate the outside world into internal execution.

### Allowed

* Auth and guards
* Dispatch and wiring
* Input/output conversion
* Calling workflow or ops entrypoints

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

### Responsibility

Defines **external and semi-external data contracts**.

DTOs are passive representations of state.

### Characteristics

* Immutable
* Versionable (`dto::abi::v1`)
* May intentionally duplicate domain fields
* Never enforce invariants

### Rule

> DTOs describe data; they never validate or mutate it.

---

## Common Classification Examples

| Code does…                        | Layer    |
| --------------------------------- | -------- |
| Stores parent/child relationships | model    |
| Reads stable registry             | ops      |
| Writes stable registry            | ops      |
| Wraps IC management calls         | infra    |
| Adds metrics to IC calls          | ops      |
| Enforces singleton uniqueness     | policy   |
| Chooses a shard                   | policy   |
| Validates eligibility             | policy   |
| Creates a canister                | workflow |
| Cascades state                    | workflow |
| Schedules a timer                 | workflow |
| Converts domain → view            | ops      |

---

## Placement Checklist

When placing code, ask:

1. Does this store canonical state? → **model**
2. Does this expose platform mechanics? → **infra**
3. Does this read or mutate state deterministically? → **ops**
4. Does this decide whether something is allowed or optimal? → **policy**
5. Does this coordinate steps or perform effects? → **workflow**

If more than one answer applies, **split the code**.

---

## Non-Goals

* Minimal directory count
* Clever abstractions
* Avoiding duplication at all costs

Duplication is acceptable when representations differ.
Clarity always wins.

---

## Enforcement

* All new code must conform to this model
* Refactors must migrate **one feature end-to-end**
* Partial migrations that blur responsibilities are discouraged
* When in doubt: **split the code, not the layer**
