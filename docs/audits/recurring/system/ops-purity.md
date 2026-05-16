# Audit: Ops Purity

## Purpose

Verify that ops code remains narrow operational code and does not accumulate
workflow orchestration or domain-policy ownership.

## Risk Model / Invariant

Ops may own:

- deterministic state access;
- conversions between records, DTOs, views, and platform shapes;
- platform calls;
- atomic mutations;
- narrow operational semantics.

Ops must not own:

- retry loops;
- orchestration branching across multiple workflow steps;
- workflow sequencing;
- policy decisions;
- metrics coordination across domains;
- cross-domain business workflows;
- endpoint semantics;
- auth endpoint decisions;
- business state machines.

## Scope

Primary scope:

- `crates/canic-core/src/ops/**`

Boundary comparison scope:

- `crates/canic-core/src/workflow/**`
- `crates/canic-core/src/domain/policy/**`
- `crates/canic-core/src/access/**`
- `crates/canic-macros/src/endpoint/**`

## Run This Audit After

- adding ops modules;
- moving logic from workflow into ops;
- adding retry, replay, funding, lifecycle, RPC, auth, or metrics behavior;
- adding state-machine-like mutation helpers;
- changing endpoint auth/access lowering.

## Checklist

### 1. Workflow Dependency Direction

Ops must not call workflow.

```bash
rg -n 'crate::workflow|workflow::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- no production ops imports or calls into workflow;
- comments may mention workflow only as context, but avoid them if they cause
  guard noise.

### 2. Orchestration Drift

Ops must not coordinate workflow sequences or retries.

```bash
rg -n 'retry|retries|loop\s*\{|while\s|join_all|spawn\(|sleep|backoff|orchestr|phase|step|transition' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Classify matches:

- atomic storage transitions are allowed;
- platform spawn primitives are allowed if workflow owns when to call them;
- cross-canister request helpers are allowed only when they execute one
  protocol operation.

### 3. Policy Ownership

Ops may consume domain policy outputs and may map storage records into policy
inputs. Ops should not define domain policy types or named policy modules.

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|mod policy|policy::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- pure policy definitions live in `domain/policy`;
- mapper names ending in `PolicyInputMapper` are conversion helpers, not policy;
- ops metric labels may reference policy error types only for bounded reporting.

### 4. Endpoint/Auth Semantics

Ops may verify token material, consume replay state, and perform key/platform
operations. Ops must not own endpoint subject binding or generated endpoint
authorization semantics.

```bash
rg -n 'verify_caller|authenticated_with_scope|requires\(|canic_update|canic_query|endpoint|DelegatedToken|consume_delegated_token_use|verify_delegated_token' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- endpoint auth semantics stay in access/macros/API guard paths;
- ops auth remains token-material verification, replay consumption, key
  resolution, and bounded metrics.

### 5. Metrics Coordination

Ops may record metrics for one operation. Workflow should coordinate
multi-domain metric sequences.

```bash
rg -n 'Metric|Metrics::record|record_.*metric|metrics::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- single-operation metrics in platform/auth/runtime ops are allowed;
- metrics modules are allowed as aggregation stores;
- ops should not run multi-step workflows just to produce one report.

## Output Requirements

Result reports must include:

- exact scope and commit;
- pass/fail for each checklist item;
- accepted hotspots;
- any code changes made;
- residual watchpoints;
- validation commands.

## Final Verdict

Choose one:

- Pass - ops remains narrow operational code;
- Pass with watchpoints - no hard violations remain, but hotspots exist;
- Fail - ops owns orchestration or policy that should move.
