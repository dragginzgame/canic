# Access Purity Audit - 2026-06-01

## Run Context

- Definition: `docs/audits/recurring/system/access-purity.md`
- Previous retained report:
  `docs/audits/reports/2026-05/2026-05-16/access-purity.md`
- Snapshot: `7a6e4e6d`
- Branch: `main`
- Worktree: dirty during audit, with scoped audit-definition and endpoint
  diagnostic cleanup changes
- Method: recurring access-boundary scan refreshed for the current hard-cut
  delegated-token and protected internal role-predicate surface
- Scope:
  - `crates/canic-core/src/access/**`
  - `crates/canic-macros/src/endpoint/**`
  - `crates/canic/src/macros/**`

## Executive Summary

Initial risk: **3 / 10**.

Post-remediation risk: **2 / 10**.

Access remains a thin endpoint boundary. The current access code authenticates,
binds caller/subject identity, performs narrow delegated-token decoding,
evaluates access expressions, records access metrics through the facade, and
delegates to ops-owned state helpers. The audit found no production stable
storage leakage, workflow orchestration, domain policy ownership, or broad DTO
conversion in access.

The only cleanup was stale version-specific endpoint macro diagnostic text.
The behavior was already current: protected internal role predicates are
update-only and internal-only, and `authenticated(...)` requires a first
`DelegatedToken` argument. The diagnostic text now describes the current rule
without `0.40` wording.

## Findings

### FIXED - Endpoint Macro Diagnostics Used Stale Version Language

Severity: **Low**.

The endpoint macro validator and protected-internal predicate expander still
used `0.40` / `0.40 V1` wording in current hard-cut diagnostics:

- `crates/canic-macros/src/endpoint/validate.rs`
- `crates/canic-macros/src/endpoint/expand.rs`
- `crates/canic-macros/src/endpoint/parse.rs`

Remediation:

- Updated protected internal role predicate diagnostics to state the current
  update-only and attested-role-only rules.
- Updated the removed `caller::has_app_role(...)` diagnostic and test to avoid
  version-specific compatibility wording.

### ACCEPTED - Delegated Token Boundary Decode

Severity: **Watchpoint**.

`crates/canic-core/src/access/auth/token.rs` decodes `DelegatedToken` from the
first ingress argument using bounded Candid decoding. This remains acceptable
endpoint-boundary auth material parsing. It does not parse general endpoint
payload DTOs.

### ACCEPTED - Delegated Session Cleanup

Severity: **Watchpoint**.

`crates/canic-core/src/access/auth/identity.rs` clears invalid delegated
sessions after rejecting a stored subject that resolves to infrastructure or
canister identity. This remains narrow auth-boundary cleanup and should not
grow into auth recovery workflow.

### ACCEPTED - Access Expression Evaluation Loops

Severity: **Low**.

`crates/canic-core/src/access/expr/mod.rs` short-circuits `All` and `Any`
access expressions. This is accepted access-expression evaluation, not
workflow orchestration.

## Checklist Results

### Storage And Stable Type Leakage

Status: **Pass**.

Command:

```bash
find crates/canic-core/src/access -name '*.rs' -print0 \
  | xargs -0 awk 'FNR == 1 { in_test = 0 } /^#\[cfg\(test\)\]/ { in_test = 1 } !in_test { print FILENAME ":" FNR ":" $0 }' \
  | rg 'stable::|storage::.*Record|AppMode|EnvRecord|AppStateRecord'
```

Result:

- No production access imports of stable storage or record types.
- App-mode endpoint guards continue to use `AppStateOps`.
- Whitelist checks continue to use `ConfigOps`.

### Workflow / Orchestration Drift

Status: **Pass**.

Command:

```bash
rg -n 'crate::workflow|workflow::|retry|retries|loop\s*\{|while\s|join_all|spawn\(|sleep|backoff|orchestr|phase|step|transition' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Result:

- No workflow calls, retry loops, recovery loops, backoff, or orchestration
  phases in access production code.
- The only match was a replay-consumption test comment in
  `access/auth/token.rs`.

### Policy Ownership

Status: **Pass**.

Command:

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|policy::|mod policy' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Result:

- No domain policy definitions or policy modules in access production code.
- Access predicates remain endpoint-boundary rules.

### Transport And DTO Boundary

Status: **Pass with delegated-token watchpoint**.

Command:

```bash
rg -n 'serde_json|serde_yaml|from_str|parse\(|impl From|impl TryFrom|record_to|to_dto|from_dto|IDLDeserialize|msg_arg_data' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Result:

- The only production matches are `msg_arg_data` and `IDLDeserialize` in
  `access/auth/token.rs`.
- This is the allowed bounded first-argument delegated-token decode.
- No broad endpoint payload parsing or DTO conversion ownership was found.

### Auth State And Metrics

Status: **Pass with watchpoints**.

Command:

```bash
rg -n 'AuthStateOps|consume_delegated_token_use|clear_delegated_session|Metrics::|metrics::|ops::runtime::metrics' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Result:

- Runtime access metric backend calls remain isolated in
  `access/metrics.rs`.
- Access expression code uses the access metrics facade.
- Auth state calls are narrow delegated-session lookup/cleanup and delegated
  token replay consumption.

### Endpoint Macro Lowering

Status: **Pass after diagnostic cleanup**.

Command:

```bash
rg -n 'resolve_authenticated_identity|eval_access|protected_internal|verify_internal_invocation_proof|caller::has_role|caller::has_any_role|authenticated_arg_error|DelegatedToken' crates/canic-macros/src/endpoint crates/canic-core/src/access -g '*.rs'
```

Result:

- Public endpoint wrappers resolve authenticated identity, evaluate access
  expressions, and then call the user handler.
- Protected internal role predicates lower through the internal envelope
  verification path and skip normal public access expression evaluation.
- `authenticated(...)` validation still requires a first argument of type
  `DelegatedToken`.
- Protected internal role predicates remain update-only and internal-only.

### Delegated-Token Audience Shape

Status: **Pass**.

Command:

```bash
rg -n '0\.40 V1|0\.40|RolesOrPrincipals|DelegationAudience::Roles|Roles\(' crates/canic-core/src/access crates/canic-macros/src/endpoint crates/canic/src/macros -g '*.rs'
```

Result:

- No multi-role delegated-token audience shape was found in the access or
  endpoint macro boundary.
- No stale versioned endpoint diagnostics remain in the inspected boundary.

## Residual Watchpoints

| Area | Risk | Note |
| --- | --- | --- |
| `access/auth/token.rs` | Medium | Preserve delegated-token decode, verify, subject bind, scope check, and update-token consume ordering. |
| `access/auth/identity.rs` | Medium | Invalid delegated-session cleanup must stay narrow and not become recovery orchestration. |
| `access/expr/mod.rs` | Low | Expression composition is acceptable, but avoid business predicates that hide workflow rules. |
| `access/metrics.rs` | Low | Keep runtime metrics backend hidden behind the access metrics facade. |
| `crates/canic-macros/src/endpoint/**` | Medium | Endpoint macro lowering must stay structural: authenticate, evaluate access, delegate, or protected-internal verify. |

## Validation Readout

| Check | Result |
| --- | --- |
| Access stable/storage scan | PASS |
| Access workflow/orchestration scan | PASS |
| Access policy ownership scan | PASS |
| Access transport/DTO scan | PASS with delegated-token watchpoint |
| Access auth-state/metrics scan | PASS with narrow auth-state watchpoints |
| Endpoint macro lowering scan | PASS after diagnostic cleanup |
| Multi-role delegated-token audience scan | PASS |

## Final Verdict

Pass with watchpoints.

Access remains a thin endpoint boundary. The current delegated-auth hotspots
are appropriate for access, and the endpoint macro still performs structural
access lowering rather than workflow or topology mutation. The only change
needed was cleanup of stale versioned endpoint diagnostics.
