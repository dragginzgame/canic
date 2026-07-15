# Canic Audit How-To

This document is the operational contract for defining, running, retaining,
and closing audits under `docs/audits/`.

## 1. Canonical Structure

```text
docs/audits/
├─ README.md
├─ AUDIT-HOWTO.md
├─ META-AUDIT.md
├─ METHODS.md
├─ retired-methods.md
├─ modular/
├─ recurring/
│  ├─ invariants/
│  └─ system/
├─ release-lines/
└─ reports/YYYY-MM/YYYY-MM-DD/
```

- `METHODS.md` is the active method and ownership catalog.
- `mandatory-trace-protocol.md` is the fingerprinted cross-cutting method for
  release-line mandatory end-to-end traces; it does not add a product-property
  owner to the retained definition count.
- `recurring/` contains reusable definitions, never run results.
- `modular/` contains the manual module-surface method and its explicitly
  finding-backed implementation workflow.
- `release-lines/` contains historical numbered closeouts and program-state
  reports, not reusable definitions.
- `reports/` contains dated primary evidence and only necessary supporting
  artifacts.
- `0.83-technical-debt/` is a frozen historical ledger exception.

Do not add a second active catalog, report root, or release-readiness verdict
path.

## 2. Definition Contract

Every active method must state or directly reference:

- stable audit ID and explicit method version;
- canonical repository owner and intended trigger;
- method kind and output profile;
- current scope and exclusions;
- canonical rules, source-of-truth files, and code sinks;
- deterministic commands and manual reasoning boundaries;
- false-positive exclusions and boundary cases;
- severity, confidence, risk, and result rules;
- report fields and verification states;
- tool prerequisites, cost class, and expected runtime;
- trace mode and permitted environment;
- comparability and method-change rules;
- artifact retention and redaction;
- positive, rejection, boundary, and regression fixtures where appropriate;
  and
- follow-up ownership for fail, partial, or blocked results.

The shared safety, state, evidence, and retention rules below may be referenced
instead of copied into each definition. Method-specific exceptions must remain
local and explicit.

### Output profiles

| Profile | Required output |
| --- | --- |
| `invariant` | Exact invariant, positive/rejection/boundary evidence, typed or observable failure, verification readout, findings, verdict. |
| `trend` | Frozen metric definitions, baseline identity, comparable delta, risk score, attribution, findings, verdict. |
| `measured` | Fixture/seed, execution environment, raw-to-summary derivation, bounded artifacts, uncertainty, comparison, findings, verdict. |
| `manual` | Versioned checklist, exact files and samples, unreviewed boundaries, named reviewer, disagreement handling, findings, verdict. |

Do not require generic hub, hotspot, or trend sections from an invariant method
unless that method says those signals contribute to its decision. This
prevents report-shape volume from masquerading as coverage.

## 3. Independent State Domains

Use exactly these state values:

```text
definition_disposition:
  retain | revise | merge | split | retire | manual_only | blocked

run_result:
  pass | fail | partial | blocked | not_applicable

result_validity:
  valid | invalid | superseded

finding_status:
  open | accepted | fixed | deferred | rejected | duplicate | blocked

closeout_verdict:
  pass | pass_with_limitations | fail | blocked
```

Rules:

- a definition disposition is not a run result;
- `manual_only` still produces a run result;
- a required invariant run that is `partial` or `blocked` blocks baselining
  and closeout;
- `not_applicable` requires evidence that a conditional trigger is absent;
- invalid or superseded results keep their original `run_result` but cannot
  support closeout; and
- missing evidence or an unavailable tool never becomes `pass`.

## 4. Immutable Run Identity

Every run records:

```text
release_anchor:
source_commit_full:
source_tree_hash:
product_tree_hash:
clean_worktree:
cargo_lock_hash:
rust_toolchain:
target_triple:
feature_set:
audit_method_id:
audit_method_version:
audit_method_fingerprint:
audit_script_hashes:
external_tool_versions:
fixture_or_seed:
environment_class:
started_at:
completed_at:
```

Use full commit hashes. A short hash is display-only. Record `not_applicable`
with a reason when a field genuinely does not apply; do not silently omit it.

Method fingerprints cover the definition plus named scripts, fixtures,
allowlists, trace templates, and other decision-bearing inputs. A method
change invalidates numerical comparison until both baselines are rerun with
the corrected method.

The canonical product-tree scope is
[product-tree-scope-v1.md](product-tree-scope-v1.md). Compute it only from a
committed snapshot with `scripts/ci/audit-product-tree-hash.sh`.

## 5. Trace Modes And Execution Safety

Every trace declares:

- `code_trace`: inspect reachable code, ownership, state transitions, errors,
  diagnostics, and projection without executing the operation; or
- `execution_trace`: execute only in an explicitly permitted disposable
  environment such as PocketIC or a named test deployment.

Read-only means read-only for tracked source and authoritative environments.
Build caches and isolated temporary output are allowed.

Before execution:

1. record `git status --porcelain`;
2. use an immutable source snapshot or disposable worktree;
3. isolate mutable state and `CARGO_TARGET_DIR` outside the source snapshot;
4. disable network by default;
5. confirm no production credentials or mainnet mutation are possible; and
6. name any authorized destructive disposable operation.

After execution:

1. record `git status --porcelain` again;
2. fail the run if tracked source changed unexpectedly;
3. retain only the required, redacted evidence; and
4. remove disposable runtime state without deleting primary evidence.

Instruction/Wasm generators may intentionally create the owning dated report
only when their method authorizes it. Product builds and local deployment
state still use isolated paths.

## 6. Running And Comparing Audits

For every run:

1. select one method from `METHODS.md`;
2. freeze scope, identity, fixtures, and environment;
3. run the declared commands and manual review;
4. record command outcomes without flattening typed causes;
5. write one new primary report under the dated layout;
6. update that day's and month's summaries; and
7. never overwrite an earlier report.

Mandatory end-to-end trace runs additionally follow
[mandatory-trace-protocol.md](mandatory-trace-protocol.md). A retained method
still owns each property; the trace protocol owns only cross-path completion
and evidence consistency.

### Same-day measurement reruns

For repeated measurement scope on one day:

- `<scope>.md` is the daily baseline and uses compared baseline `N/A`;
- `<scope>-2.md`, `<scope>-3.md`, and later reruns compare directly to
  `<scope>.md`; and
- reruns never chain and never select a baseline from another day.

### Finding-backed implementation comparisons

After a fix slice, retain both:

- causal comparison against the immediate parent commit; and
- cumulative comparison against the original frozen product baseline.

The parent comparison validates the slice. The baseline comparison reports
release-line cumulative risk.

### Post-freeze method defects

When a method defect is found after freeze:

1. mark affected results `result_validity: invalid` in a superseding report;
2. record an `audit_method_defect` finding;
3. increment and refingerprint the method;
4. rerun the original product baseline with the corrected method;
5. rerun the current fix commit; and
6. compare only corrected-method results.

If the original baseline cannot be reproduced, closeout is `blocked` or
explicitly non-comparable.

## 7. Findings And Deduplication

Assign canonical identity during triage from the owner and violated invariant,
not discovery order:

```text
finding_id: CANIC-<LINE>-<OWNER>-<NNN>
finding_class:
severity: P0 | P1 | P2 | P3
confidence: confirmed | high | medium | low
finding_status:
owner:
current_location:
intended_owner:
affected_surfaces:
source_audit_ids:
source_method_fingerprints:
baseline_commit:
first_observed_at:
evidence:
typed_cause_or_invariant:
risk:
verification_status:
duplicate_of:
recommended_slice:
fix_commit:
validation_commit:
waiver:
disposition:
```

Finding classes are `product_defect`, `audit_method_defect`, `evidence_gap`,
`governance_conflict`, `operational_risk`, and `documentation_drift`.
Severity is impact; confidence is evidence strength. Duplicate discoveries
list every source and point to one triage-assigned canonical ID.

## 8. Manual-Only Review

A manual method records:

- stable ID, version, and fingerprint;
- exact commit and files reviewed;
- versioned checklist;
- code references and sampled paths;
- explicit unreviewed boundaries;
- named reviewer; and
- second review or explicit single-review waiver for P0/P1 findings.

Disagreement is recorded as a finding or blocked result and escalated to the
maintainer. A narrative report without this record is not a passing manual
audit.

## 9. Evidence Manifest, Redaction, And Retention

Every run contains:

```text
command:
working_directory:
exit_code:
stdout_path:
stderr_path:
baseline_identity:
method_identity:
tool_versions:
timestamps:
artifact_hashes:
retention_class:
redactions_applied:
```

- Markdown is the primary evidence.
- Retain raw output only when needed to reproduce a finding or future
  comparison.
- Prefer one compact machine-readable form; do not retain duplicate JSON,
  CSV, TSV, text, and logs.
- Hash retained evidence with SHA-256 or a recorded equivalent.
- Scrub credentials, tokens, private material, sensitive principals, private
  paths, and environment secrets.
- Record absent stdout/stderr as `not_retained`.
- Never delete the only evidence for a finding, comparison, or typed cause.
- Check links before pruning.

Historical primary Markdown is append-only. Superseding reports may invalidate
or correct conclusions without rewriting them. A retired method must remain
recoverable through the immutable source snapshot and
[retired-methods.md](retired-methods.md); no compatibility wrapper remains.

## 10. Reports And Summaries

Report paths are:

```text
docs/audits/reports/YYYY-MM/YYYY-MM-DD/<scope>.md
docs/audits/reports/YYYY-MM/YYYY-MM-DD/summary.md
docs/audits/reports/YYYY-MM/summary.md
```

The day summary records run identities, risk, method/comparability notes,
findings, verification rollup, and follow-up. The month summary indexes run
days, records month status, and carries unresolved follow-up.

Historical report links and conclusions are not rewritten. Current day/month
summaries may be updated during that active period.

## 11. Closeout

Closeout is `pass` only when every required audit is complete and no accepted
limitation remains. An accepted waivable P1 or blocked informational/trend
method forces `pass_with_limitations`. P0 and the non-waivable P1 classes in
the accepted line design block closeout.

Unavailable broad maintainer-owned release gates are recorded, not fabricated.
Audit closeout does not authorize package versioning, commits, tags, pushes,
deployment, or a 1.0 readiness claim.
