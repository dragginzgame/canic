
Each layer has a **single responsibility** and must only depend on layers
to its right.

---

## model/

**Purpose**
- Owns stable memory and in-process registries
- Defines data layout and persistence
- Contains no system rules or orchestration

**Allowed**
- Stable structures
- Registries
- Serialization / encoding
- Plain data types

**Forbidden**
- Config access
- IC management calls
- Async code
- Timers
- Business rules or invariants

**Rule**
> `model` stores data, it does not decide or act.

---

## ops/

**Purpose**
- Thin, deterministic accessors over `model`
- Safe read/write façades
- Views, pagination, lookup helpers

This includes `ops/storage/*`.

**Allowed**
- Reading and writing model state
- Simple validation of inputs (structural, not semantic)
- Returning `Option` or simple errors

**Forbidden**
- IC management calls
- Async
- Timers
- Multi-step coordination
- System-wide invariants

**Rule**
> `ops` knows *how* to access state, not *when* or *why*.

---

## policy/

**Purpose**
- System rules and decision-making
- Invariant enforcement
- Placement, scaling, sharding, and cardinality logic

**Allowed**
- Reading config via ops
- Reading state via ops
- Deterministic decisions
- “Can we?” / “Should we?” logic

**Forbidden**
- State mutation
- IC calls
- Async
- Timers

**Rule**
> `policy` decides, but never acts.

---

## workflow/

**Purpose**
- Multi-step behavior and orchestration
- Lifecycle actions
- IC management calls
- Cascades, retries, timers, rollbacks

**Allowed**
- Async
- IC calls
- State mutation via ops
- Calling policy to validate decisions

**Forbidden**
- Direct model access
- Embedding policy logic inline

**Rule**
> `workflow` is where things happen.

---

## endpoints / macros

**Purpose**
- Boundary layer
- Auth, dispatch, wiring
- No business logic

**Allowed**
- Calling workflow
- Guards and auth
- DTO marshalling

**Forbidden**
- Direct model access
- Policy decisions
- Orchestration logic

---

## Common Classification Examples

| Code does… | Layer |
|----------|------|
| Reads stable registry | ops |
| Writes stable registry | ops |
| Enforces singleton cardinality | policy |
| Chooses a shard | policy |
| Creates a canister | workflow |
| Cascades state | workflow |
| Schedules a timer | workflow |
| Stores parent/child relationships | model |

---

## Architectural Tests (Mental Checklist)

When placing code, ask:

1. Does this **store data**? → model
2. Does this **access data**? → ops
3. Does this **decide if something is allowed or recommended**? → policy
4. Does this **do something or coordinate steps**? → workflow

If more than one answer applies, the code is in the wrong place and should
be split.

---

## Non-Goals

- This architecture does not attempt to optimize for minimal directories.
- Thin layers are acceptable if they preserve responsibility boundaries.
- Refactors should prefer clarity over consolidation.

---

## Enforcement

All new code should conform to this document.
Large refactors should migrate one feature end-to-end (model → workflow)
rather than partially.

If in doubt, prefer **splitting** over collapsing responsibilities.
