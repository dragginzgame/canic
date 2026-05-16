# Audit: Access Purity

## Purpose

Verify that access code remains a thin endpoint boundary and does not absorb
workflow, policy, storage, transport, or business behavior.

## Risk Model / Invariant

Access may:

- authenticate;
- authorize;
- bind subject and caller;
- decode or encode endpoint-boundary authentication material;
- delegate immediately to ops, policy, workflow, or endpoint handlers.

Access must not own:

- business branching;
- policy definitions;
- storage records or stable storage types;
- orchestration loops;
- retry or recovery behavior;
- DTO conversion ownership outside endpoint-boundary auth material;
- transport parsing beyond boundary unmarshalling;
- auth state mutation outside narrow replay/session boundary calls.

## Scope

Primary scope:

- `crates/canic-core/src/access/**`

Boundary comparison scope:

- `crates/canic-core/src/ops/**`
- `crates/canic-core/src/domain/policy/**`
- `crates/canic-core/src/workflow/**`
- `crates/canic-macros/src/endpoint/**`

## Run This Audit After

- adding access predicates;
- changing endpoint auth macro lowering;
- changing delegated-token verification or delegated-session resolution;
- changing app/environment endpoint guards;
- adding metrics to access paths.

## Checklist

### 1. Storage And Stable Type Leakage

Access must use ops boundaries, not storage records or stable storage types.

```bash
find crates/canic-core/src/access -name '*.rs' -print0 \
  | xargs -0 awk 'FNR == 1 { in_test = 0 } /^#\[cfg\(test\)\]/ { in_test = 1 } !in_test { print FILENAME ":" FNR ":" $0 }' \
  | rg 'stable::|storage::.*Record|AppMode|EnvRecord|AppStateRecord'
```

Expected:

- no production access imports of stable storage or record types;
- test fixtures may import records inside `#[cfg(test)]` modules.

### 2. Workflow / Orchestration Drift

Access must not call workflow or coordinate multi-step workflows.

```bash
rg -n 'crate::workflow|workflow::|retry|retries|loop\s*\{|while\s|join_all|spawn\(|sleep|backoff|orchestr|phase|step|transition' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- no workflow calls;
- no retry/recovery loops;
- expression-tree short-circuit loops are accepted as access evaluation, not
  business orchestration.

### 3. Policy Ownership

Access may apply access expressions, but must not define domain policy.

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|policy::|mod policy' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- no domain policy definitions;
- access expression predicates stay endpoint-boundary rules.

### 4. Transport And DTO Boundary

Access may decode the delegated token from ingress bytes. It must not own
general endpoint payload parsing or DTO conversion.

```bash
rg -n 'serde_json|serde_yaml|from_str|parse\(|impl From|impl TryFrom|record_to|to_dto|from_dto|IDLDeserialize|msg_arg_data' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- delegated-token first-argument decoding is allowed;
- broad payload parsing or conversion is a violation.

### 5. Auth State And Metrics

Access may call narrow auth/session/replay ops and emit normalized access
metrics through the access metrics facade.

```bash
rg -n 'AuthStateOps|consume_delegated_token_use|clear_delegated_session|Metrics::|metrics::|ops::runtime::metrics' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- direct runtime metric backend calls should stay isolated in
  `access/metrics.rs`;
- auth state changes should stay narrow and endpoint-boundary related.

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

- Pass - access remains a thin endpoint boundary;
- Pass with watchpoints - no hard violations remain, but hotspots exist;
- Fail - access owns workflow, storage schema, or domain policy behavior.
