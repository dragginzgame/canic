# Canic 0.83 Technical Debt Ledger

Schema version: 1
Audit date: 2026-07-08
Repo ref: working tree after 0.82.41 push; package surface 0.82.41
Status: pass_with_followups

## Scope

Initial 0.83 inventory and focused command-surface fixes. The first pass
created the audit ledger, recorded baseline repo-health commands, and
classified the first evidence-backed debt candidate. The first follow-up fix
hard-cuts that finding by standardizing the affected report command surfaces on
`--json` for raw JSON and `--evidence-envelope` for stable evidence-envelope
output. The second follow-up fix hard-cuts default-JSON advanced deploy report
families to JSON by default plus `--text` for human-readable output.

## Baseline Validation

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all -- --check` | pass | Format check passed. |
| `cargo check --locked -p canic-core -p canic` | pass | Checked `canic-core` and `canic`. |
| `cargo test --locked -p canic-cli` | pass | 601 `canic-cli` lib tests passed; bin/doc tests had 0 tests. |
| `cargo test --locked -p canic --test protocol_surface` | pass | 19 protocol-surface tests passed. |
| `cargo test --locked -p canic-host deployment_truth --lib` | pass | 428 filtered `canic-host` deployment-truth tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed after adding the 0.83 changelog. |
| `git diff --check` | pass | Whitespace diff check passed after audit files were written. |

## Findings

## CANIC-083-DEBT-001: Mixed JSON Flag Conventions Across Active Report Commands

Severity: P2
Category: config_ownership / diagnostic_ownership
Status: fixed
Owner: CLI command families
Current location: mixed CLI report parsers and help text
Intended owner: command-surface convention owned per report family, with a
documented repo-level convention for operator reports
Affected surfaces: cli, docs, json
Release decision: fixed_in_0.83.0

Evidence:
- file: `crates/canic-cli/src/state/mod.rs`
- line or anchor: lines 56-63
- module/function: state manifest help text
- command/search: `rg -n -e "--format json" -e "Format::Json" crates/canic-cli/src docs`
- reachability: active CLI help text
- exact issue: `canic state manifest` explicitly advertises `--json` and says
  it does not accept `--format json`.

Evidence:
- file: `crates/canic-cli/src/deploy/catalog.rs`
- line or anchor: lines 36-58 and 144-158
- module/function: deploy catalog help text and `write_report`
- command/search: `rg -n -e "--format json" -e "Format::Json" crates/canic-cli/src docs`
- reachability: active CLI help text and active report writer
- exact issue: deployment catalog reports advertise and render JSON through
  `--format json`.

Evidence:
- file: `crates/canic-cli/src/fleets/command.rs`
- line or anchor: lines 87-109
- module/function: fleet adoption help text
- command/search: `rg -n -e "--format json" -e "Format::Json" crates/canic-cli/src docs`
- reachability: active CLI help text
- exact issue: fleet adoption reports advertise `--format json` and
  `--format envelope-json`, while newer operator-report commands use `--json`.

Evidence:
- file: `crates/canic-cli/src/evidence/command.rs`
- line or anchor: lines 78-79
- module/function: evidence gate help text
- command/search: `rg -n -e "--format json" -e "Format::Json" crates/canic-cli/src docs`
- reachability: active CLI help text
- exact issue: evidence gate examples still use `--format json`.

Risk:

Operator report commands now have two active JSON-selection conventions:
newer surfaces such as state/medic/inspect/deploy-plan use `--json`, while
older report families still use `--format json` or `--format envelope-json`.
That is not a correctness bug, but it creates automation and docs drift risk
because command families disagree on whether JSON is a boolean output mode or a
format enum.

Recommendation:

Create a focused command-surface convention slice. Decide whether older
multi-format evidence/report commands keep `--format` as intentionally
different, or whether selected families should hard-cut to `--json` with a
separate behavior-changing slice. Do not change parser behavior inside the
audit pass.

Regression test:

For the chosen convention, pin parser/help behavior for each affected family:
state, medic, inspect, deploy plan, deploy catalog, fleet adoption, evidence
gate, and evidence compare.

Follow-up slice:

0.83 JSON output convention hardening. Completed for the affected report
families in 0.83.0.

Resolution:

- `canic deploy inspect catalog list|inspect` now use `--json` for raw JSON
  catalog reports and keep text as the default.
- `canic deploy check <deployment>` now uses text by default, `--json` for the
  raw `DeploymentCheckV1` payload, and `--evidence-envelope` for stable
  CI/GitOps evidence-envelope output.
- `canic fleet adoption report <fleet>` now uses text by default, `--json` for
  the raw adoption report payload, and `--evidence-envelope` for stable
  CI/GitOps evidence-envelope output.
- `canic evidence gate` now uses text by default, `--json` for raw gate report
  payloads, and `--evidence-envelope` for stable evidence-envelope output.
- `canic evidence compare` now uses text by default and `--json` for the
  structured comparison report.
- Active docs and examples for the affected families now use the maintained
  flags. Archived docs and historical changelogs remain historical.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the command-surface changes. |
| `cargo test --locked -p canic-cli deploy` | pass | 176 filtered deploy-related CLI tests passed. |
| `cargo test --locked -p canic-cli fleet` | pass | 71 filtered fleet-related CLI tests passed. |
| `cargo test --locked -p canic-cli evidence` | pass | 23 filtered evidence-related CLI tests passed. |
| `cargo test --locked -p canic-cli` | pass | 600 `canic-cli` tests passed. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `git diff --check` | pass | Whitespace diff check passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |

## CANIC-083-DEBT-002: Advanced Deploy Report Families Still Use Multi-Value `--format`

Severity: P2
Category: config_ownership / diagnostic_ownership
Status: fixed
Owner: advanced deploy report command families
Current location: deploy compare, root, authority, external, and promote
report parsers
Intended owner: command-surface convention owned per report family, with a
documented repo-level convention for default-JSON report tools
Affected surfaces: cli, docs, json
Release decision: fixed_in_0.83.1

Evidence:
- file: `crates/canic-cli/src/deploy/compare.rs`
- line or anchor: `FORMAT_ARG` and `value_name("json|text")`
- module/function: deployment-check artifact comparison command parser
- command/search: `rg -n -e '\\.long\\("format"\\)' -e 'FORMAT_ARG: &str = "format"' -e 'value_name\\(".*json' crates/canic-cli/src -S`
- reachability: active CLI parser
- exact issue: deploy compare still uses `--format json|text` for output
  selection.

Evidence:
- file: `crates/canic-cli/src/deploy/root.rs`
- line or anchor: `value_name("json|text")`
- module/function: root verification report parser
- command/search: same as above
- reachability: active CLI parser
- exact issue: root verification reports still use `--format json|text` for
  output selection.

Evidence:
- file: `crates/canic-cli/src/deploy/authority.rs`
- line or anchor: `value_name("json|text")`
- module/function: authority evidence/check/report/receipt parser
- command/search: same as above
- reachability: active CLI parser
- exact issue: authority reports still use `--format json|text` for output
  selection.

Evidence:
- file: `crates/canic-cli/src/deploy/external/command.rs`
- line or anchor: `value_name("json|text")`
- module/function: external lifecycle report parser
- command/search: same as above
- reachability: active CLI parser
- exact issue: external lifecycle reports still use `--format json|text` for
  output selection.

Evidence:
- file: `crates/canic-cli/src/deploy/promote/command.rs`
- line or anchor: `value_name("json|text")`
- module/function: promotion report parser
- command/search: same as above
- reachability: active CLI parser
- exact issue: promotion reports still use `--format json|text` for output
  selection.

Risk:

The first 0.83 fix standardizes text-default report families and
evidence-envelope emitters, but advanced deploy report families still use a
multi-value `--format` enum. These commands default to JSON and use
`--format text` for human summaries, so they need a separate command-surface
decision rather than a blind mechanical rewrite.

Recommendation:

Create a focused advanced-deploy report convention slice. Decide whether these
default-JSON report/request tools should hard-cut to `--text`, keep default
JSON, and remove `--format`, or whether they intentionally remain a separate
format-enum class. Do not mix this with the already-fixed text-default report
families.

Regression test:

Pin maintained parser/help behavior for deploy compare, root verification,
authority reports, external lifecycle reports, and promotion reports.

Follow-up slice:

0.83 advanced deploy output convention hardening. Completed in 0.83.1.

Resolution:

- `canic deploy inspect compare` now defaults to JSON and uses `--text` for
  human-readable comparison summaries.
- `canic deploy inspect root` and `canic deploy root verify` now default to
  JSON and use `--text` for human-readable root verification output.
- `canic deploy authority check|evidence|report|receipt` now default to JSON
  and use `--text` for human-readable dry-run authority summaries.
- `canic deploy external` report builders now default to JSON and use `--text`
  for human-readable external lifecycle summaries.
- `canic deploy promote` report builders now default to JSON and use `--text`
  for human-readable promotion summaries.
- The removed `--format json|text` parser routes were not kept as aliases or
  compatibility shims, and old-format anti-resurrection tests were removed.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the advanced deploy output-convention changes. |
| `cargo test --locked -p canic-cli deploy` | pass | 168 filtered deploy-related CLI tests passed. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |

## CANIC-083-DEBT-003: State Manifest Help Mentions Removed `--format json`

Severity: P3
Category: docs_drift / legacy_surface
Status: fixed
Owner: state CLI command help
Current location: `canic state manifest` help text
Intended owner: active help text only documents maintained command forms
Affected surfaces: cli, docs
Release decision: fixed_in_0.83.1

Evidence:
- file: `crates/canic-cli/src/state/mod.rs`
- line or anchor: `MANIFEST_HELP_AFTER`
- module/function: state manifest help text
- command/search: `rg -n -- '--format json|--format text' crates/canic-cli/src docs/audits/0.83-technical-debt docs/changelog/0.83.md`
- reachability: active CLI help text
- exact issue: `canic state manifest` help said it does not accept
  `--format json`, which is a removed-form breadcrumb rather than maintained
  operator guidance.

Risk:

Low. The command parser already uses the maintained `--json` flag and does not
expose the removed form, but active help should not preserve pre-1.0
compatibility breadcrumbs.

Recommendation:

Remove the removed-form mention and keep the help focused on current behavior:
the command renders the derived manifest to stdout and does not write manifest
files.

Regression test:

Existing state help tests cover the maintained `canic state manifest` surface.
Do not add anti-resurrection tests for the removed `--format json` spelling.

Resolution:

- Removed the `--format json` breadcrumb from `canic state manifest` help.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the help-text cleanup. |
| `cargo test --locked -p canic-cli state` | pass | 12 filtered state-related CLI tests passed. |
| `git diff --check` | pass | Whitespace diff check passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |

## Rejected / Non-Findings

See `rejected.md`.

## Deferred

See `deferred.md`.

## Recommended Slices

See `recommended-slices.md`.
