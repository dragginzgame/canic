# Canic Audit How-To

Apply the reviewed [Shared audit contract](../../audits/README.md) first.
It owns common execution authority, source-bound evidence, finding ownership,
structural severity/verdicts and historical preservation. [METHODS.md](METHODS.md)
selects Canic properties; [META-AUDIT.md](META-AUDIT.md) supplies product invariants.
This overlay retains local report paths and frozen domain measurement/closeout
contracts. Structural reviews use their shared-method overlays, not the older
score, tier or report templates required by a different domain method.

## 1. Canonical Structure

- `METHODS.md` is the single active method/owner catalog.
- `mandatory-trace-protocol.md` owns release-line end-to-end trace completion;
  property findings stay with the retained methods, without another method count.
- `recurring/` holds reusable domain definitions, `modular/` surface/cleanup overlays.
- `release-lines/` holds numbered closeouts and retained supporting records;
  `working/` holds bounded investigations owned by an active design batch.
- `reports/YYYY-MM/YYYY-MM-DD/` holds dated primary reports and necessary artifacts.
- `0.83-technical-debt/` is a frozen historical exception, not a current backlog.

Do not create another active catalog, report root or standing readiness verdict.
Use the [directory ownership map](README.md#directory-ownership) for retention.

## 2. Definition Contract

An active method states or references its ID/version, canonical owner/trigger,
kind/profile, scope/exclusions, source authorities and sinks, deterministic
commands/manual boundaries, false-positive cases, prerequisites/cost/runtime, trace mode,
comparison identity and affected success/rejection/boundary/regression proof.
Shared report, severity and authority requirements are referenced, not recopied.
Domain exceptions and failure/partial/blocked follow-up stay explicit locally.

The existing local release-integrity method stays authoritative until the
reviewed supplying Shared revision is adopted under
[#618 — Adopt shared CI/release integrity](https://github.com/dragginzgame/canic/issues/618).
This cleanup adopts no unpublished method or moving sibling source.

### Output profiles

Retain the profile selected by the owning domain method:

| Profile | Domain proof |
| --- | --- |
| `invariant` | Exact invariant, positive/rejection/boundary evidence, typed or observable failure, verification and verdict. |
| `trend` | Frozen metrics, baseline, comparable delta, method-owned risk score and attribution. |
| `measured` | Fixture/seed, environment, raw-to-summary derivation, bounded artifacts and uncertainty. |
| `manual` | Versioned checklist, exact files/samples, unreviewed boundaries, named reviewer and disagreement handling. |

These do not introduce scores into shared finding-based structural methods or
require generic hotspot/hub sections where they prove no selected obligation.

## 3. Independent State Domains

Domain records keep their independent values; none substitutes for another:

| Field | Values |
| --- | --- |
| `definition_disposition` | `retain`, `revise`, `merge`, `split`, `retire`, `manual_only`, `blocked` |
| `run_result` | `pass`, `fail`, `partial`, `blocked`, `not_applicable` |
| `result_validity` | `valid`, `invalid`, `superseded` |
| `finding_status` | `open`, `accepted`, `fixed`, `deferred`, `rejected`, `duplicate`, `blocked` |
| `closeout_verdict` | `pass`, `pass_with_limitations`, `fail`, `blocked` |

`manual_only` still has a run result. Required invariants with partial/blocked
proof block baselining and closeout. `not_applicable` needs evidence that its
conditional trigger is absent. Invalid/superseded results retain their original
result but cannot support closeout; missing tools/proof never become a pass.
Shared structural reports retain their own severity/verdict vocabulary.

## 4. Immutable Run Identity

Domain runs retain these field identities:

```text
release_anchor, source_commit_full, source_tree_hash, product_tree_hash,
clean_worktree, cargo_lock_hash, rust_toolchain, target_triple, feature_set,
audit_method_id, audit_method_version, audit_method_fingerprint,
audit_script_hashes, external_tool_versions, fixture_or_seed,
environment_class, started_at, completed_at
```

Use full commits; short hashes are display-only. A genuinely inapplicable field
records `not_applicable` and its reason. Fingerprints bind definitions and all
named decision-bearing scripts, fixtures, allowlists and trace templates.
[method-fingerprints-v1.md](method-fingerprints-v1.md) admits their current bytes.
A changed method requires corrected-method baselines before numerical comparison.
The [product-tree scope](product-tree-scope-v1.md) and
`scripts/ci/audit-product-tree-hash.sh` hash committed Git objects only.

## 5. Trace Modes And Execution Safety

Select `code_trace` for reachable source/authority inspection, or
`execution_trace` only for a named, explicitly authorized disposable environment
such as PocketIC or a test deployment. Apply
[Shared execution authority](../../audits/README.md#authority-and-execution);
an explicit read-only constraint prohibits writes, including execution outputs.

For authorized execution, record Git state before/after; use immutable source
or a disposable worktree and isolate mutable state/target output. Disable
network by default, exclude production credentials/mainnet mutation and name
any authorized destructive disposable operation. Unexpected tracked changes
fail the run. Retain required redacted evidence before disposable-state cleanup.
An owning instruction/Wasm method may authorize its dated report output;
that never supplies authority for product deployment or broad gates.

## 6. Running And Comparing Audits

Select a catalog method and freeze scope, identity, fixtures and environment.
Execute only declared, authorized proof, preserve typed causes, and write one
new primary report with day/month summary links. Apply the
[trace protocol](mandatory-trace-protocol.md) for mandatory end-to-end runs;
it does not replace the property owner. Never overwrite a previous report.

### Same-day measurement reruns

`<scope>.md` is that day's measurement baseline, compared baseline `N/A`.
`<scope>-2.md` and later reruns compare directly with it, never another day or
the preceding rerun. This does not impose a measurement scheme on structural audits.

### Finding-backed implementation comparisons

Retain causal evidence against the immediate parent and cumulative evidence
against the original frozen product baseline. They answer slice correctness
and cumulative release-line risk respectively.

### Post-freeze method defects

A superseding report marks affected results `result_validity: invalid` and
records `audit_method_defect`. Increment/refingerprint the affected method,
rerun original product baseline and current fix with the corrected method,
and compare only those results. An unreproducible original baseline leaves
closeout blocked or explicitly non-comparable. Historical results stay intact.

## 7. Findings And Deduplication

Apply [Shared finding ownership](../../audits/README.md#ownership-and-history).
GitHub issues track active work; immutable reports may retain domain finding IDs
`CANIC-<LINE>-<OWNER>-<NNN>` as evidence joins, not a parallel status ledger.
Triage by owner/violated invariant and link duplicate discoveries to one owner.

Domain finding records retain `finding_id`, `finding_class`, `severity`,
`confidence`, `finding_status`, `owner`, `current_location`, `intended_owner`,
`affected_surfaces`, `source_audit_ids`, `source_method_fingerprints`,
`baseline_commit`, `first_observed_at`, `evidence`, `typed_cause_or_invariant`,
`risk`, `verification_status`, `duplicate_of`, `recommended_slice`, `fix_commit`,
`validation_commit`, `waiver` and `disposition`.

Domain classes are `product_defect`, `audit_method_defect`, `evidence_gap`,
`governance_conflict`, `operational_risk` and `documentation_drift`. Their
severity remains `P0`–`P3`, confidence `confirmed`, `high`, `medium` or `low`.
Do not silently convert old scores/tiers into shared structural severities.

## 8. Manual-Only Review

Record method ID/version/fingerprint, exact source/files/checklist, sampled
paths, unreviewed boundaries and named reviewer. P0/P1 domain findings need
second review or an explicit single-review waiver. Record disagreements as
findings or blocked results for maintainer resolution; narrative alone is no pass.

## 9. Evidence Manifest, Redaction, And Retention

Required domain evidence fields remain:

```text
command, working_directory, exit_code, stdout_path, stderr_path,
baseline_identity, method_identity, tool_versions, timestamps,
artifact_hashes, retention_class, redactions_applied
```

Markdown is primary evidence. Keep raw output only for needed reproduction or
comparison; prefer one compact machine-readable form over duplicate formats.
Hash retained evidence, redact credentials/private material, sensitive principals,
private paths and environment secrets, and record absent streams as `not_retained`.
Never delete the sole proof of a finding, comparison or typed cause; check links
before pruning. Historical primary reports remain immutable. Superseding reports
may correct/invalidate conclusions. Retired definitions stay reproducible from
immutable source and [retirement records](retired-methods.md), without active aliases.

## 10. Reports And Summaries

```text
docs/audits/reports/YYYY-MM/YYYY-MM-DD/<scope>.md
docs/audits/reports/YYYY-MM/YYYY-MM-DD/summary.md
docs/audits/reports/YYYY-MM/summary.md
```

The day summary links run identity, risk, method/comparability, findings,
verification and follow-up. The month summary indexes days, month status and
unresolved follow-up links. Active summaries may change; historical reports,
links and conclusions do not. GitHub remains the active tracker.

## 11. Closeout

Canic domain closeout is `pass` only with all required audits complete and no
accepted limitations. An accepted waivable P1 or blocked informational/trend
method requires `pass_with_limitations`. P0 and non-waivable P1 classes in the
accepted line design block closeout. Record unavailable maintainer-owned gates
without fabricating results. Audit closeout supplies no release/version, Git,
publication/deployment or 1.0-readiness authority.
