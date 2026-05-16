# Ops Purity Audit - 2026-05-16

## Run Context

- Definition: `docs/audits/recurring/system/ops-purity.md`
- Related baseline:
  `docs/audits/reports/2026-05/2026-05-16/workflow-purity.md`
- Snapshot: `92ec102b`
- Branch: `main`
- Worktree: dirty
- Method: V1.0, focused ops responsibility scan
- Scope: `crates/canic-core/src/ops/**`, with comparisons against workflow,
  domain policy, access, and endpoint macros

## Executive Summary

Initial risk: **4 / 10**.

Post-remediation risk: **3 / 10**.

The audit found one concrete naming and ownership drift: delegated auth had an
ops module named `policy` with `*Policy`/`*PolicyError` types for certificate
TTL and root/key binding checks. The behavior was correctly local to token
material validation, but the names made ops look like the policy owner.

That module is now `cert_rules`, and the types/functions now describe
certificate rules and TTL limits instead of policy ownership. No public API
shape or verifier behavior changed.

## Findings

### FIXED - Delegated Auth Certificate Rules Were Named As Ops Policy

Severity: **Medium**.

`ops/auth/delegated/policy.rs` owned certificate issuance checks and exported
`DelegatedAuthTtlPolicy`, `CertPolicyError`, and
`validate_cert_issuance_policy`.

The code did not orchestrate workflow, but the naming violated the ops-purity
model: ops should run narrow token-material validation, while domain policy
ownership should stay explicit under `domain/policy`.

Remediation:

- Renamed the module to `ops/auth/delegated/cert_rules.rs`.
- Renamed `DelegatedAuthTtlPolicy` to `DelegatedAuthTtlLimits`.
- Renamed `CertPolicyError` to `CertRuleError`.
- Renamed `validate_cert_issuance_policy` to
  `validate_cert_issuance_rules`.
- Updated auth signing, minting, and verification call sites.
- Preserved the existing stable metric label `cert_policy` to avoid changing
  runtime metrics output in a cleanup patch.

### ACCEPTED - RPC Ops Is A Protocol Operation Hotspot

Severity: **Watchpoint**.

`ops/rpc/mod.rs` builds capability envelopes, caches root response
attestations, and executes a single protocol request. That is centralized and
security-sensitive, but it remains a narrow RPC operation boundary rather than
workflow orchestration.

Keep watching for future retry loops, multi-step recovery, or business
branching in this module. Those belong in workflow.

### ACCEPTED - Storage Ops Own Atomic State Transitions

Severity: **Low**.

`ops/storage/intent` and `ops/storage/placement/directory` contain explicit
state transitions. These are storage-level atomic mutation semantics, not
workflow state machines. The current shape is acceptable because the ops
functions mutate one local storage boundary and return typed results for
workflow to classify.

### ACCEPTED - Metrics Ops Own Metric Stores And Single-Operation Recording

Severity: **Low**.

Runtime metrics modules are large, but they remain stores/reporting helpers.
Platform/auth/runtime ops record bounded outcomes for the operation they own.
No cross-domain workflow was found running inside metrics ops.

## Checklist Results

### Workflow Dependency Direction

Status: **Pass with comment residue**.

Command:

```bash
rg -n 'crate::workflow|workflow::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Only one comment mentions a workflow path for context. No production ops code
imports or calls workflow.

### Orchestration Drift

Status: **Pass with watchpoints**.

Command:

```bash
rg -n 'retry|retries|loop\s*\{|while\s|join_all|spawn\(|sleep|backoff|orchestr|phase|step|transition' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Remaining hits are accepted categories:

- atomic storage state transitions;
- replay guard state;
- runtime bootstrap diagnostic phase labels;
- `IcOps::spawn` as a platform primitive;
- RPC protocol envelope construction.

### Policy Ownership

Status: **Pass after remediation**.

Command:

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|mod policy|policy::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

The delegated-auth `policy` module was removed. Remaining hits are conversion
mapper names, domain-policy metric labels, and `ops/topology/policy/mapper`,
which maps storage records into policy input shapes.

### Endpoint/Auth Semantics

Status: **Pass with auth hotspot watchpoint**.

Ops auth verifies delegated token material, checks local key/root bindings,
records bounded verifier metrics, and consumes replay state. Endpoint caller
binding and generated endpoint access semantics remain outside ops.

### Metrics Coordination

Status: **Pass**.

Ops metrics remain single-operation reporting and metric-store ownership. No
multi-domain report orchestration was found inside ops.

## Residual Watchpoints

| Area | Risk | Note |
| --- | --- | --- |
| `ops/rpc/mod.rs` | Medium | Capability envelope and attestation cache are security-sensitive. Keep retry/recovery outside ops. |
| `ops/auth/token.rs` | Medium | Auth ops is correctly hot, but endpoint subject binding must remain outside token-material verification. |
| `ops/topology/policy/mapper.rs` | Low | The path name says policy, but the file only maps storage records to policy inputs. Rename only if it starts causing guard noise. |
| `ops/storage/intent` | Low | Atomic storage state transitions are acceptable; workflow should own any higher-level business sequence. |

## Verification Readout

| Check | Result |
| --- | --- |
| Ops workflow dependency scan | PASS |
| Ops orchestration scan | PASS with accepted hotspots |
| Ops policy ownership scan | PASS after remediation |
| `cargo fmt --all` | PASS |
| `cargo check -p canic-core` | PASS |

## Final Verdict

Pass with watchpoints.

Ops remains narrow operational code after the delegated-auth naming cleanup.
The next useful follow-up is to keep `ops/rpc/mod.rs` and `ops/auth/token.rs`
under security review because they are expected hot paths, not because they are
currently violating the boundary.
