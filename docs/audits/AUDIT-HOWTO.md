# Canic Audit How-To

This document defines how to run, record, and compare architectural/security
audits for the Canic repository.

Audits are structural checks. They are not refactor sessions.

## 1. Purpose

Audits are used to detect drift in:
- architecture layering and ownership
- delegated token/security invariants
- lifecycle symmetry and environment integrity
- complexity and governance pressure

Audits must be:
- deterministic
- date-stamped
- comparable across runs
- explicit about what is verified vs missing

Audits must not:
- speculate beyond code evidence
- mix multiple audit definitions into one report
- mutate the audit definition during a run

## 2. Audit Sources

Audit definitions live in:

```text
docs/audits/
```

Architecture/security contracts live in:

```text
docs/contracts/
```

Current key contracts:
- `ARCHITECTURE.md`
- `ACCESS_ARCHITECTURE.md`
- `AUTH_DELEGATED_SIGNATURES.md`

## 3. When to Run Audits

Run audits at these checkpoints:
- weekly governance run
- after auth/access/macro changes
- after lifecycle/env changes
- after delegated signing/token changes
- before release cut

Minimum weekly set:
- caller-subject binding audit
- architecture contract conformance review
- delegated-signature contract conformance review

## 4. Audit Run Procedure

For each audit run:
1. Pick one audit definition file in `docs/audits/`.
2. Freeze the prompt/definition for the run.
3. Run against current workspace state.
4. Record exact evidence (files/functions/checks).
5. Classify findings by severity.
6. Mark ambiguous paths as ambiguous.
7. Create or update the date folder result files.
8. Update that date folder `summary.md` immediately.

Run context to capture in each report:
- date
- branch
- `git rev-parse --short HEAD`
- dirty/clean worktree note

## 5. Baseline Commands

Use these baseline commands during each audit day.

Codebase size snapshots:

```bash
cd crates
cloc . --not-match-f='(^|/)(tests\.rs$|tests/)'
cloc . --match-f='(^|/)(tests\.rs$|tests/)'
```

Rust test count:

```bash
rg -o '#\[(tokio::)?test\]' crates --glob '*.rs' | wc -l
```

ECDSA boundary scan (management ECDSA APIs must stay inside the approved ops façade):

```bash
rg -n 'sign_with_ecdsa|ecdsa_public_key|verify_ecdsa' crates -g '*.rs' \
  | rg -v 'crates/canic-core/src/ops/ic/ecdsa.rs'
```

Legacy signature infrastructure regression scan:

```bash
rg -n 'SignatureInfra::prepare|SignatureInfra::get|SignatureInfra|set_certified_data|data_certificate' crates -g '*.rs'
```

Relay/bearer-path scan:

```bash
rg -n 'AuthenticatedRequest|presenter_pid|canic_response_authenticated|relay' crates -g '*.rs'
```

Subject-binding scan:

```bash
rg -n 'claims\.sub|sub == caller|authenticated\(' crates/canic-core/src crates/canisters -g '*.rs'
```

## 6. Where to Store Results

Store results in date folders:

```text
docs/audit-results/YYYY-MM-DD/
```

Example:

```text
docs/audit-results/2026-02-24/
  caller-subject-binding.md
  auth-delegated-signatures.md
  architecture-conformance.md
  summary.md
```

Rules:
- use ISO date only (`YYYY-MM-DD`)
- never overwrite previous date folders
- add new files or append updates for new runs

## 7. Required `summary.md`

Each date folder must contain `summary.md`.

`summary.md` is a rolling daily artifact and must be updated after every audit
run that day.

Required sections:
- run contexts
- risk index summary table
- risk index vertical blocks
- codebase/test snapshots
- drift since previous audit date
- high/medium/low risk findings

Template:

```md
# Audit Summary — YYYY-MM-DD

## Risk Index Summary

| Risk Index                    | Score | Run Context |
| ---------------------------- | ----- | ----------- |
| Auth Binding Integrity       | X/10  | ...         |
| Delegation Chain Integrity   | X/10  | ...         |
| Root Authority Integrity     | X/10  | ...         |
| Access Boundary Integrity    | X/10  | ...         |
| Layering Integrity           | X/10  | ...         |
| Lifecycle Integrity          | X/10  | ...         |
| Complexity Pressure          | X/10  | ...         |

## Risk Index Summary (Vertical Format)

Auth Binding Integrity
- Score: X/10
- Run Context: ...

Delegation Chain Integrity
- Score: X/10
- Run Context: ...

Root Authority Integrity
- Score: X/10
- Run Context: ...

Access Boundary Integrity
- Score: X/10
- Run Context: ...

Layering Integrity
- Score: X/10
- Run Context: ...

Lifecycle Integrity
- Score: X/10
- Run Context: ...

Complexity Pressure
- Score: X/10
- Run Context: ...

## Snapshots

- Non-test cloc: files=..., blank=..., comment=..., code=...
- Test-only cloc: files=..., blank=..., comment=..., code=...
- Rust test count: ...

## Drift Since Previous Audit

- ...

## Findings

### High
- ...

### Medium
- ...

### Low
- ...
```

## 8. Risk Index Scale

All indices are risk-oriented (lower is better):
- `1-3`: low risk, structurally healthy
- `4-6`: moderate risk, manageable pressure
- `7-8`: high risk, active monitoring required
- `9-10`: critical risk, immediate action required

Do not inflate scores. Scores must be backed by code evidence.

## 9. Drift Comparison Procedure

For each new run:
1. Compare to the previous date folder.
2. Record deltas in auth/architecture invariants.
3. Record new/removed critical paths.
4. Record metric deltas (`cloc`, tests, grep counts where used).

Canic-specific drift signals:
- subject-binding enforcement moved or split
- relay/bearer semantics reintroduced
- legacy two-step/raw signature paths reintroduced
- root authority mutability introduced
- audience/scope checks weakened
- cross-layer shortcuts introduced

## 10. Findings Handling

Classify findings as:
- Critical
- High
- Medium
- Low
- Informational

Only after classification should refactor planning begin.

Never refactor during the audit run itself.

## 11. Discipline Rules

Never:
- modify audit definitions mid-run
- collapse unrelated audits into one report
- downgrade severity without evidence
- replace structured findings with only narrative prose

Always:
- date-stamp results
- include concrete file/function evidence
- keep risk polarity consistent (lower score is better)
- update daily `summary.md` after each run

## 12. Optional Validation Commands

After implementing fixes from findings, validate repository quality gates:

```bash
make fmt-check
make clippy
make test
```

For delegated-crypto verification in CI-equivalent strict mode:

```bash
CANIC_REQUIRE_THRESHOLD_KEYS=1 cargo test -p canic --test root_replay --locked delegation_issuance_routes_through_dispatcher_non_skip_path -- --nocapture --test-threads=1
```
