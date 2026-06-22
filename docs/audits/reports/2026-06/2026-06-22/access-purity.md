# Access Purity Audit - 2026-06-22

## Report Preamble

- Scope: `crates/canic-core/src/access/**`, endpoint macro access lowering,
  and boundary comparison against ops/workflow/domain policy ownership.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/access-purity.md`
- Code snapshot: `4bcad983`
- Branch: `main`
- Worktree: dirty; unrelated host install-root work, prior layer-audit work,
  and blob-storage cleanup edits were preserved.
- Method tag/version: `access-purity-current-root-proof-boundary`
- Comparability status: partially comparable. The definition was tightened
  before this run to make root proof provisioning and root issuer policy
  mutation explicit non-access responsibilities.

## Executive Summary

Verdict: **PASS with watchpoints**.

Risk score: **2 / 10**.

Access remains a thin endpoint boundary. It resolves authenticated identity,
performs delegated-token first-argument decoding, evaluates access expressions,
records access metrics through the access facade, and uses narrow auth/session
ops. It does not own storage records, workflow orchestration, domain policy,
root-proof provisioning, root issuer policy mutation, or broad DTO conversion.

No production code changes were required. The only change was to the recurring
audit definition:

- require `bash scripts/ci/run-layering-guards.sh` as guard-parity evidence;
- add explicit root delegation proof/root issuer policy boundary checks.

## Findings

### PASS - No Storage Or Stable Type Leakage

Command:

```bash
find crates/canic-core/src/access -name '*.rs' -print0 \
  | xargs -0 awk 'FNR == 1 { in_test = 0 } /^#\[cfg\(test\)\]/ { in_test = 1 } !in_test { print FILENAME ":" FNR ":" $0 }' \
  | rg 'stable::|storage::.*Record|AppMode|EnvRecord|AppStateRecord'
```

Result: no output.

The executable layering guard also passed:

```bash
bash scripts/ci/run-layering-guards.sh
```

### PASS - No Workflow Or Orchestration Drift

Command:

```bash
rg -n 'crate::workflow|workflow::|retry|retries|loop\s*\{|while\s|join_all|spawn\(|sleep|backoff|orchestr|phase|step|transition' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs' \
  | rg -v ':\s*//'
```

Result: no output.

Access expression recursion remains local predicate evaluation, not workflow,
retry, or recovery ownership.

### PASS - No Domain Policy Ownership

Command:

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|policy::|mod policy' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Result: no output.

### PASS WITH WATCHPOINT - Delegated Token Boundary Decode

Command:

```bash
rg -n 'serde_json|serde_yaml|from_str|parse\(|impl From|impl TryFrom|record_to|to_dto|from_dto|IDLDeserialize|msg_arg_data' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Accepted output:

```text
crates/canic-core/src/access/auth/token.rs:11: api::msg_arg_data
crates/canic-core/src/access/auth/token.rs:12: candid::de::{DecoderConfig, IDLDeserialize}
crates/canic-core/src/access/auth/token.rs:107: msg_arg_data()
crates/canic-core/src/access/auth/token.rs:127: IDLDeserialize::new_with_config(...)
```

This remains accepted endpoint-boundary auth-material parsing: access decodes
only the first ingress argument as `DelegatedToken` with bounded Candid
decoding.

### PASS WITH WATCHPOINT - Narrow Auth State And Metrics

Command:

```bash
rg -n 'AuthStateOps|delegated_session|upsert_delegated_session|clear_delegated_session|consume_delegated|consume_update|_token_use|_token_once|Metrics::|metrics::|ops::runtime::metrics' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Accepted production output:

- `access/auth/identity.rs` reads and clears delegated-session state through
  `AuthStateOps`.
- `access/metrics.rs` is the only production access module that calls runtime
  metric backends directly.
- `access/expr/evaluators.rs` and `access/expr/mod.rs` use access metrics
  facade calls.

This remains narrow endpoint-boundary identity/metrics behavior. The
delegated-token verifier guard test still proves access does not introduce
verifier-local token-use storage.

### PASS - Root Proof And Issuer Policy Boundary

Command:

```bash
rg -n 'delegation_proof|root_issuer|issuer_policy|RootIssuerPolicy|prepare_delegation|install_active_delegation|canic_get_delegation_proof|canic_prepare_delegation_proof' crates/canic-core/src/access -g '*.rs' --glob '!**/tests.rs'
```

Result: no output.

Access does not own root delegation proof preparation, retrieval, installation,
or root issuer policy mutation.

### PASS - Endpoint Macro Lowering

Command:

```bash
rg -n 'resolve_authenticated_identity|eval_access|protected_internal|verify_internal_invocation_proof|caller::has_role|caller::has_any_role|authenticated_arg_error|DelegatedToken' crates/canic-macros/src/endpoint crates/canic-core/src/access -g '*.rs'
```

Result: accepted structural access plumbing only. Endpoint macro expansion
resolves authenticated identity, builds an access context, evaluates access
expressions, validates authenticated endpoint signatures, and delegates to user
handlers.

No workflow, topology mutation, proof lifecycle behavior, or hidden business
policy was found in macro output.

### PASS - Stale Delegated-Token Audience/Version Terms

Command:

```bash
rg -n 'multi-role|multiple roles|role audience|principal audience|0\.67|0\.68|compat|legacy' crates/canic-core/src/access crates/canic-macros/src/endpoint docs/audits/recurring/system/access-purity.md
```

Result: only the audit definition's current expected behavior wording matched.
No stale version-specific or role-audience compatibility language was found in
access or endpoint macro code.

## Structural Hotspots

| Hotspot | Status | Evidence |
| --- | --- | --- |
| `access/auth/token.rs` | Accepted | Owns bounded delegated-token first-argument decode plus verify/bind/scope ordering. |
| `access/auth/identity.rs` | Accepted | Owns delegated-session identity fallback and invalid-subject cleanup. |
| `access/expr/mod.rs` | Accepted | Owns access expression short-circuit evaluation and metrics recording. |
| `access/metrics.rs` | Accepted | Only access module allowed to call runtime metric backends directly. |
| `crates/canic-macros/src/endpoint/expand/access.rs` | Accepted | Emits structural access plumbing only. |

## Hub Module Pressure

Pressure score: **2 / 10**.

`crates/canic-core/src/access` remains compact and purpose-bound. Import
pressure is concentrated in token decode, identity resolution, expression
evaluation, and metrics facade modules, which are the expected endpoint access
surface.

## Early Warning Signals

- Keep delegated-token decode limited to the first auth argument.
- Keep delegated-session cleanup as fallback hygiene, not recovery workflow.
- Keep runtime metric backend calls behind `access/metrics.rs`.
- Keep root proof provisioning and root issuer policy mutation outside access.
- Keep endpoint macro output structural and free of proof lifecycle logic.

## Checklist Results

| Checklist Item | Status | Notes |
| --- | --- | --- |
| Storage and stable type leakage | PASS | No production matches; layering guard passed. |
| Workflow/orchestration drift | PASS | No production matches after comment filtering. |
| Policy ownership | PASS | No `*Policy` ownership in access. |
| Transport and DTO boundary | PASS with watchpoint | Only bounded delegated-token first-argument Candid decode. |
| Auth state and metrics | PASS with watchpoints | Narrow delegated-session access and access metrics facade use only. |
| Root proof and issuer policy boundary | PASS | No proof provisioning or root issuer policy matches. |
| Endpoint macro lowering | PASS | Authenticate, evaluate access, delegate. |
| Stale delegated-token audience/version terms | PASS | No code matches. |

## Verification Readout

| Check | Result |
| --- | --- |
| Access storage/stable leakage scan | PASS |
| `bash scripts/ci/run-layering-guards.sh` | PASS |
| Access workflow/orchestration scan | PASS |
| Access policy ownership scan | PASS |
| Access transport/DTO scan | PASS with accepted delegated-token decode |
| Access auth-state/metrics scan | PASS with accepted narrow ops/facade usage |
| Root proof / issuer policy boundary scan | PASS |
| Endpoint macro lowering scan | PASS |
| Stale delegated-token audience/version scan | PASS |
| `cargo test --locked -p canic-core access:: --lib -- --nocapture` | PASS, 27 tests |
| `cargo test --locked -p canic-macros endpoint --lib -- --nocapture` | PASS, 32 tests |

## Follow-Up Actions

No required code follow-up from this audit.

Keep the root-proof/provisioning boundary check in the recurring definition so
future delegated-auth changes do not move proof lifecycle behavior into access.

## Final Verdict

Pass with watchpoints - access remains a thin endpoint boundary.
