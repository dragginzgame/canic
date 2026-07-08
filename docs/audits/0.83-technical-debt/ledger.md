# Canic 0.83 Technical Debt Ledger

Schema version: 1
Audit date: 2026-07-08
Repo ref: baseline working tree after 0.82.41 push; current package surface 0.83.10
Status: pass_with_followups

## Scope

Initial 0.83 inventory and focused command-surface fixes. The first pass
created the audit ledger, recorded baseline repo-health commands, and
classified the first evidence-backed debt candidate. The first follow-up fix
hard-cuts that finding by standardizing the affected report command surfaces on
`--json` for raw JSON and `--evidence-envelope` for stable evidence-envelope
output. The second follow-up fix hard-cuts default-JSON advanced deploy report
families to JSON by default plus `--text` for human-readable output. The third
follow-up fix tightens the runtime inspect report wrapper so command,
endpoint, health/readiness slots, source, response format, and aggregate status
labels are typed internally while preserving the existing JSON/text output
contract. The fourth follow-up fix tightens the auth renewal status report
wrapper so CLI-owned report kind, local Candid source, and aggregate status
labels are typed internally while preserving decoded canister response strings
as data. The fifth follow-up fix tightens the metrics report wrapper so
canister-row status labels are typed internally while preserving existing JSON
labels and text output. The sixth follow-up fix applies the same ownership
rule to cycles report canister status and coverage status labels. The seventh
follow-up fix tightens blob-storage report kind, Candid source, action,
funding-status, and readiness-state labels. The eighth follow-up fix tightens
backup create, list, prune, status, and inspect report layout/status/action
labels. The ninth follow-up fix tightens token and cycles wallet command
parsing so maintained subcommand sets are represented by typed parser values.
The tenth follow-up fix tightens blob-storage action method-mode ownership so
the closed `query`/`update` action-report labels are stored as typed model
values instead of being derived to strings before report construction. The
eleventh follow-up fix tightens replica status-source ownership so the closed
`canic replica status --json` source labels are represented by typed internal
report values. The twelfth follow-up fix tightens deploy-plan future-apply
preview rows so phase, operation, and status labels are represented by typed
internal report values. The thirteenth follow-up fix tightens deploy-plan
diagnostic category, severity, and source labels into typed internal report
values. The fourteenth follow-up fix tightens state-audit report scope,
category, and source labels into typed internal report values. The fifteenth
follow-up fix tightens deployment-root verification check names into typed
internal report values.

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

## CANIC-083-DEBT-004: Runtime Inspect Report Wrapper Owns Report Labels As Untyped Values

Severity: P3
Category: runtime_introspection / diagnostic_ownership
Status: fixed
Owner: runtime inspect CLI report wrapper
Current location: `crates/canic-cli/src/inspect/mod.rs`
Intended owner: typed inspect report model, with renderers formatting typed
values
Affected surfaces: internal, json
Release decision: fixed_in_0.83.2

Evidence:
- file: `crates/canic-cli/src/inspect/mod.rs`
- line or anchor: `InspectReport`, `TargetResolution`, and
  `RuntimeStatusPayload`
- module/function: inspect report wrapper
- command/search: `rg -n "command: String|endpoint: String|health_status: Option<serde_json::Value>|readiness_status: Option<serde_json::Value>|status: String|source: String|response_format: String" crates/canic-cli/src/inspect/mod.rs`
- reachability: active `canic inspect` JSON/text report path
- exact issue: the report wrapper stored command and endpoint labels as raw
  strings, dormant health/readiness status slots as loose JSON values, and
  aggregate status, source attribution, and response format as raw strings,
  even though the 0.81 runtime introspection contract already has typed DTOs
  and stable command/endpoint/source/status values.

Risk:

Low. The emitted strings were correct and tests covered the current output, but
raw strings leave the CLI report wrapper able to drift from the typed
runtime-observed/source-attribution contract without compiler help.

Recommendation:

Use typed command and endpoint values, typed health/readiness DTO slots, typed
local values for inspect source attribution and response format, and reuse the
endpoint `RuntimeStatus` enum for the report aggregate status. Keep
serialization labels identical so JSON/text output does not change.

Regression test:

Keep focused inspect serialization/text tests asserting `cli_arg`,
`runtime_observed`, `candid`, and status labels in the JSON/text output.

Resolution:

- `InspectReport.command` and `InspectReport.endpoint` now store typed
  `InspectCommandKind` and `InspectEndpoint` values instead of strings.
- `InspectReport.status` now stores `RuntimeStatus` instead of `String`.
- `InspectReport.health_status` and `InspectReport.readiness_status` now use
  typed `CanicHealthStatus` and `CanicReadinessStatus` slots instead of loose
  JSON values.
- `TargetResolution.source` and `RuntimeStatusPayload.source` now store a typed
  `InspectSource`.
- `RuntimeStatusPayload.response_format` now stores a typed
  `InspectResponseFormat`.
- Serde output labels remain unchanged: `canic inspect canister`,
  `canic inspect deployment`, `canic_runtime_status`, `ok`, `degraded`,
  `failing`, `unknown`, `cli_arg`, `deployment_record`, `runtime_observed`,
  and `candid`.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo test --locked -p canic-cli inspect` | pass | 35 filtered inspect-related CLI tests passed. |
| `cargo fmt --all` | pass | Formatted the inspect report typing change. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `git diff --check` | pass | Whitespace diff check passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |

## CANIC-083-DEBT-005: Auth Renewal Status Report Owns CLI Labels As Raw Strings

Severity: P3
Category: diagnostic_ownership / auth
Status: fixed
Owner: auth renewal CLI report wrapper
Current location: `crates/canic-cli/src/auth/mod.rs`
Intended owner: typed auth renewal report model, with renderers formatting
typed CLI-owned labels and preserving decoded canister response strings as data
Affected surfaces: internal, json
Release decision: fixed_in_0.83.3

Evidence:
- file: `crates/canic-cli/src/auth/mod.rs`
- line or anchor: `AuthRootTarget`, `AuthIssuerTarget`, and
  `AuthRenewalStatusResult`
- module/function: auth renewal status report wrapper
- command/search: `rg -n "AUTH_RENEWAL_STATUS_KIND|AUTH_RENEWAL_STATUS_ACTIVE_ATTEMPT|AUTH_RENEWAL_CANDID_SOURCE_INSTALLED_DEPLOYMENT|kind: String|candid_source: String|status: String" crates/canic-cli/src/auth`
- reachability: active `canic auth renewal status` JSON/text report path
- exact issue: the report wrapper stored the CLI-owned report kind, local
  Candid-source label, and aggregate renewal status as strings even though
  those values are closed within the CLI report model.

Risk:

Low. The emitted JSON/text labels were correct and covered by tests, but raw
strings let the CLI report wrapper drift from its own stable labels without
compiler help. Decoded issuer and active-attempt status strings are response
data from canisters and intentionally remain strings.

Recommendation:

Use typed local values for the report kind, local Candid source, and aggregate
renewal status. Keep serialization labels identical so JSON/text output does
not change.

Regression test:

Keep focused auth renewal tests asserting the typed values and JSON labels:
`auth_renewal_status`, `installed_deployment`, `active_attempt`, and
`unavailable`.

Resolution:

- `AuthRenewalStatusResult.kind` now stores `AuthRenewalReportKind`.
- `AuthRootTarget.candid_source` and `AuthIssuerTarget.candid_source` now store
  `AuthRenewalCandidSource`.
- `AuthRenewalStatusResult.status` now stores `AuthRenewalStatusCode`.
- Renderers format the typed status label, and JSON labels remain unchanged.
- Decoded canister response fields such as active-attempt and issuer-observed
  status remain strings.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo test --locked -p canic-cli auth` | pass | 23 filtered auth-related CLI tests passed. |
| `cargo fmt --all` | pass | Formatted the auth report typing change. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `git diff --check` | pass | Whitespace diff check passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |

## CANIC-083-DEBT-006: Metrics Canister Report Owns Row Status As Raw Strings

Severity: P3
Category: diagnostic_ownership / metrics
Status: fixed
Owner: metrics CLI report wrapper
Current location: `crates/canic-cli/src/metrics/model.rs`
Intended owner: typed metrics report model, with renderers formatting typed
status labels
Affected surfaces: internal, json
Release decision: fixed_in_0.83.4

Evidence:
- file: `crates/canic-cli/src/metrics/model.rs`
- line or anchor: `MetricsCanisterReport.status`
- module/function: metrics model and transport builders
- command/search: `rg -n "status: \"|status\\.to_string|status: String" crates/canic-cli/src/metrics -g '*.rs'`
- reachability: active `canic info metrics` JSON/text report path
- exact issue: metrics row status labels (`ok`, `empty`, `unavailable`,
  `error`) were built and stored as strings even though the set is closed
  inside the CLI report model.

Risk:

Low. The emitted JSON/text labels were correct and covered by focused tests,
but raw strings let metrics report rows drift from the stable status set
without compiler help.

Recommendation:

Use a typed metrics canister status value and keep serde/text labels identical
so JSON and text output do not change.

Regression test:

Keep focused metrics tests asserting typed statuses and JSON labels:
`empty` and `unavailable`.

Resolution:

- `MetricsCanisterReport.status` now stores `MetricsCanisterStatus`.
- Metrics transport builders use enum variants for `ok`, `empty`,
  `unavailable`, and `error`.
- Text renderers call the typed status label method.
- JSON labels remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the metrics report typing change. |
| `cargo test --locked -p canic-cli metrics` | pass | 14 filtered metrics-related CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-007: Cycles Canister Report Owns Status Labels As Raw Strings

Severity: P3
Category: diagnostic_ownership / cycles
Status: fixed
Owner: cycles CLI report wrapper
Current location: `crates/canic-cli/src/cycles/model.rs`
Intended owner: typed cycles report model, with renderers formatting typed
status labels
Affected surfaces: internal, json
Release decision: fixed_in_0.83.4

Evidence:
- file: `crates/canic-cli/src/cycles/model.rs`
- line or anchor: `CyclesCanisterReport.status` and
  `CyclesCanisterReport.coverage_status`
- module/function: cycles model and transport summarizer
- command/search: `rg -n "status: \"|coverage_status: \"|status\\.to_string|coverage_status\\(" crates/canic-cli/src/cycles -g '*.rs'`
- reachability: active `canic info cycles` JSON/text report path
- exact issue: cycles row status labels (`ok`, `empty`, `error`) and coverage
  status labels (`covered`, `partial`, `none`) were built and stored as
  strings even though both sets are closed inside the CLI report model.

Risk:

Low. The emitted JSON/text labels were correct and covered by focused tests,
but raw strings let cycles report rows drift from their stable status sets
without compiler help.

Recommendation:

Use typed cycles canister status and coverage status values. Keep serde/text
labels identical so JSON and text output do not change.

Regression test:

Keep focused cycles tests asserting typed statuses and JSON labels: `ok` and
`partial`.

Resolution:

- `CyclesCanisterReport.status` now stores `CyclesCanisterStatus`.
- `CyclesCanisterReport.coverage_status` now stores `CyclesCoverageStatus`.
- Cycles transport builders and summarizers use enum variants for `ok`,
  `empty`, `error`, `covered`, `partial`, and `none`.
- Text renderers call typed label methods.
- JSON labels remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the cycles report typing change. |
| `cargo test --locked -p canic-cli cycles` | pass | 43 filtered cycles-related CLI tests passed. |
| `cargo test --locked -p canic-cli metrics` | pass | 14 filtered metrics-related CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-008: Blob-Storage Reports Own Closed Labels As Raw Strings

Severity: P3
Category: diagnostic_ownership / blob_storage
Status: fixed
Owner: blob-storage CLI report wrapper
Current location: `crates/canic-cli/src/blob_storage/model.rs`
Intended owner: typed blob-storage report model, with renderers and error
boundaries formatting typed labels
Affected surfaces: internal, json
Release decision: fixed_in_0.83.5

Evidence:
- file: `crates/canic-cli/src/blob_storage/model.rs`
- line or anchor: `BlobStorageTarget.candid_source`,
  `BlobStorageErrorResult.kind`, `BlobStorageAction.name`,
  `BlobStorageActionResult.kind`, `BlobStorageStatusResult.kind`,
  `BlobStorageFundingStatus.status`, and `BlobStorageReadinessStatus.state`
- module/function: blob-storage model, parser, renderer, and medic summary
- command/search: `rg -n "kind: String|candid_source: Option<String>|state: String|status: String|name: String" crates/canic-cli/src/blob_storage/model.rs`
- reachability: active `canic blob-storage` JSON/text report paths and
  medic blob-storage summary path
- exact issue: blob-storage report kind labels, local Candid source labels,
  action labels, funding status labels, and readiness state labels were stored
  as strings even though these sets are closed inside the CLI report model.

Risk:

Low. The emitted JSON/text labels were correct and covered by focused tests,
but raw strings let blob-storage report wrappers drift from the stable labels
without compiler help.

Recommendation:

Use typed values for closed report labels. Keep free-form command strings,
error messages, blocker/warning code arrays, and canister-derived text values
as strings.

Regression test:

Keep focused blob-storage tests asserting typed readiness/funding values and
unchanged JSON labels for report kind, Candid source, action, funding status,
and readiness state.

Resolution:

- `BlobStorageTarget.candid_source` and `BlobStorageErrorTarget.candid_source`
  now store `BlobStorageCandidSource`.
- `BlobStorageErrorResult.kind` and `BlobStorageStatusResult.kind` now store
  `BlobStorageReportKind`.
- `BlobStorageActionResult.kind` now stores `BlobStorageActionResultKind`.
- `BlobStorageAction.name` now stores `BlobStorageActionName`.
- `BlobStorageFundingStatus.status` now stores
  `BlobStorageFundingStatusCode`.
- `BlobStorageReadinessStatus.state` now stores `BlobStorageReadinessState`.
- Renderers, medic summary, and readiness-check errors format typed labels at
  the output/error boundary.
- JSON labels remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the blob-storage report typing change. |
| `cargo test --locked -p canic-cli blob_storage` | pass | 30 filtered blob-storage-related CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-009: Backup CLI Reports Own Layout And Status Labels As Raw Strings

Severity: P3
Category: diagnostic_ownership / backup
Status: fixed
Owner: backup CLI report wrappers
Current location: `crates/canic-cli/src/backup/model.rs`
Intended owner: typed backup report model, with renderers and JSON boundaries
formatting typed labels
Affected surfaces: internal, json
Release decision: fixed_in_0.83.6

Evidence:
- file: `crates/canic-cli/src/backup/model.rs`
- line or anchor: `BackupCreateReport`, `BackupListEntry`,
  `BackupPruneEntry`, `BackupDryRunStatusReport`, and `BackupInspectReport`
- module/function: backup model, create/status/list/prune/inspect builders,
  and renderers
- command/search: `rg -n "layout_status: String|status: String|action: String|mode: String|layout: String" crates/canic-cli/src/backup/model.rs`
- reachability: active `canic backup create|list|prune|status|inspect`
  report paths
- exact issue: backup report mode, layout, status, and prune action labels
  were stored as strings even though these sets are closed inside the CLI
  report model.

Risk:

Low. The emitted JSON/text labels were correct and covered by focused tests,
but raw strings let backup report wrappers drift from the stable label sets
without compiler help.

Recommendation:

Use typed values for closed backup report labels. Keep dynamic backup scope,
operation kind/state, paths, errors, and canister-derived data as strings.

Regression test:

Keep focused backup tests asserting typed status/action values and unchanged
JSON/text labels for dry-run status, create layout/status, list status, prune
action, and inspect layout status.

Resolution:

- `BackupCreateReport.mode`, `BackupCreateReport.layout`, and
  `BackupCreateReport.status` now store typed `BackupCreateMode`,
  `BackupCreateLayout`, and `BackupRunStatus` values.
- `BackupListEntry.status` and `BackupPruneEntry.status` now store
  `BackupListStatus`.
- `BackupPruneEntry.action` now stores `BackupPruneAction`.
- `BackupDryRunStatusReport.layout_status` and
  `BackupInspectReport.layout_status` now store
  `BackupExecutionLayoutStatus`.
- Create/list/prune/status/inspect builders use enum variants internally;
  renderers and JSON serialization format labels at the output boundary.
- JSON labels and text output remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the backup report typing change. |
| `cargo test --locked -p canic-cli backup` | pass | 65 filtered backup-related CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-010: Wallet Command Parsers Own Command Kinds As Raw Strings

Severity: P3
Category: command_parser / wallet
Status: fixed
Owner: token and cycles CLI parsers
Current location: `crates/canic-cli/src/token.rs` and
`crates/canic-cli/src/cycles/wallet.rs`
Intended owner: typed wallet command parsers, with token symbols and delegated
ICP CLI command/error strings remaining caller/runtime data
Affected surfaces: internal
Release decision: fixed_in_0.83.7

Evidence:
- file: `crates/canic-cli/src/token.rs`
- line or anchor: `TokenCommandRequest.command`, `run`, and
  `split_token_command`
- module/function: token command parser and dispatcher
- command/search: `rg -n "TokenCommandRequest|request.command|command: String" crates/canic-cli/src/token.rs`
- reachability: active `canic token balance|transfer` command path
- exact issue: `TokenCommandRequest` stored the closed `balance`/`transfer`
  command set as a raw string and the dispatcher matched the same string
  labels again.

Evidence:
- file: `crates/canic-cli/src/cycles/wallet.rs`
- line or anchor: `run_cycles_command`, `WalletCommand`, and command-name
  constants
- module/function: cycles wallet command parser and dispatcher
- command/search: `rg -n "BALANCE_COMMAND|CONVERT_COMMAND|MINT_COMMAND|TRANSFER_COMMAND|TOPUP_COMMAND|match command" crates/canic-cli/src/cycles/wallet.rs`
- reachability: active `canic cycles balance|convert|mint|transfer|topup`
  command path
- exact issue: cycles wallet dispatch used raw command strings and separate
  command-name constants even though the maintained wallet command set is
  closed inside the parser.

Risk:

Low. Parser behavior was correct and covered by focused tests, but wallet
dispatchers could drift from their maintained command sets because command kind
ownership was encoded in string matching and separate constants.

Recommendation:

Use small internal command-kind enums for maintained wallet command sets. Keep
token symbols, receivers, pending-operation command strings, and delegated ICP
CLI command/error strings as strings.

Regression test:

Keep focused token and cycles parser tests covering default command shapes,
explicit token-prefix shape, missing transfer receivers, compact
deployment-target receiver parsing, and cycles wallet subcommand options.

Resolution:

- `TokenCommandRequest.command` now stores `TokenCommandKind`.
- `split_token_command` parses only the maintained `balance` and `transfer`
  command kinds, preserving the optional token-prefix behavior.
- `run` dispatches on `TokenCommandKind` variants instead of matching strings.
- `cycles::wallet` now owns `WalletCommandKind` for the maintained
  `balance`, `convert`, `mint`, `transfer`, and `topup` command set.
- `run_cycles_command`, `cycles_command`, command construction, and help
  command builders use the typed cycles wallet command labels.
- Token symbols, pending-operation command strings, and ICP CLI command/error
  strings remain strings.
- CLI command forms and help text remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the wallet parser typing change. |
| `cargo test --locked -p canic-cli token` | pass | 4 filtered token-related CLI tests passed. |
| `cargo test --locked -p canic-cli cycles` | pass | 43 filtered cycles-related CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-011: Blob-Storage Action Reports Own Method Mode As Raw Strings

Severity: P3
Category: diagnostic_ownership / blob_storage
Status: fixed
Owner: blob-storage CLI action report wrapper
Current location: `crates/canic-cli/src/blob_storage/model.rs` and
`crates/canic-cli/src/blob_storage/target.rs`
Intended owner: typed blob-storage action report model, with target resolution
supplying typed method-mode values and renderers/JSON boundaries formatting
labels
Affected surfaces: internal, json
Release decision: fixed_in_0.83.8

Evidence:
- file: `crates/canic-cli/src/blob_storage/model.rs`
- line or anchor: `BlobStorageAction.mode`,
  `BlobStorageActionResult::dry_run`, `BlobStorageActionResult::completed`,
  and `BlobStorageActionResult::new`
- module/function: blob-storage action report model and builders
- command/search: `rg -n "mode: String|target\\.method_mode\\.label\\(\\)" crates/canic-cli/src/blob_storage -g '*.rs'`
- reachability: active `canic blob-storage sync-gateways|fund` dry-run and
  completed report paths
- exact issue: `BlobStorageAction.mode` stored the closed action method-mode
  label as a raw string, and call sites converted the resolved method mode to
  `query`/`update` labels before constructing the action report.

Risk:

Low. The emitted JSON/text labels were correct and covered by focused tests,
but raw strings split ownership between target resolution and report
construction even though the `query`/`update` set is closed inside the
blob-storage command model.

Recommendation:

Move method-mode ownership into the blob-storage report model and store the
typed value in `BlobStorageAction`. Keep canister response data, guidance
action labels, delegated command strings, and error text as strings.

Regression test:

Keep focused blob-storage tests asserting unchanged JSON `action.mode` labels
and text `Mode: update` output for dry-run and completed action reports.

Resolution:

- `BlobStorageMethodMode` now lives in `blob_storage::model`.
- `BlobStorageAction.mode` now stores `BlobStorageMethodMode`.
- `BlobStorageActionResult::dry_run`, `completed`, and `new` accept typed
  method-mode values.
- Target resolution supplies the typed method mode directly to action-report
  construction.
- Renderers and JSON serialization format `query`/`update` labels at the
  output boundary.
- Response-derived `sync_action`, next-action guidance strings, command
  strings, and error text remain strings.
- JSON labels and text output remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the blob-storage method-mode typing change. |
| `cargo test --locked -p canic-cli blob_storage` | pass | Focused blob-storage-related CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-012: Replica Status Reports Own Status Source As Raw Strings

Severity: P3
Category: diagnostic_ownership / replica
Status: fixed
Owner: replica status JSON report wrapper
Current location: `crates/canic-cli/src/replica/mod.rs`
Intended owner: typed replica status report model, with JSON serialization
formatting the stable source labels
Affected surfaces: internal, json
Release decision: fixed_in_0.83.9

Evidence:
- file: `crates/canic-cli/src/replica/mod.rs`
- line or anchor: `ReplicaStatusJsonReport.status_source` and
  `run_status_json`
- module/function: replica status JSON report builder
- command/search: `rg -n "status_source|icp_cli_stale|http_status" crates/canic-cli/src/replica -g '*.rs'`
- reachability: active `canic replica status --json` report path
- exact issue: replica status JSON reports stored the closed source labels
  `icp_cli`, `icp_cli_stale`, `http_status`, and `none` as string literals
  rather than as a typed report value.

Risk:

Low. The emitted JSON labels were correct and covered by focused tests, but
the source-label set was owned by string literals in the status builder rather
than the report model.

Recommendation:

Use a private typed source enum for the closed replica status-source labels.
Keep delegated ICP command/error text and HTTP status payloads as runtime
data.

Regression test:

Keep focused replica tests asserting unchanged `status_source` JSON labels for
HTTP fallback and stale ICP CLI status cases.

Resolution:

- `ReplicaStatusJsonReport.status_source` now stores a private
  `ReplicaStatusSource` enum.
- `ReplicaStatusSource` serializes to the existing `icp_cli`,
  `icp_cli_stale`, `http_status`, and `none` labels.
- `run_status_json` constructs typed source variants instead of string
  literals.
- JSON labels and command behavior remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the replica status-source typing change. |
| `cargo test --locked -p canic-cli replica` | pass | 18 filtered replica/status-related CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-013: Deploy-Plan Future-Apply Preview Rows Own Labels As Raw Strings

Severity: P3
Category: diagnostic_ownership / deploy_plan
Status: fixed
Owner: deploy-plan report wrapper
Current location: `crates/canic-cli/src/deploy/plan.rs`
Intended owner: typed deploy-plan report model, with text/JSON serialization
formatting stable future-apply preview labels
Affected surfaces: internal, json
Release decision: fixed_in_0.83.9

Evidence:
- file: `crates/canic-cli/src/deploy/plan.rs`
- line or anchor: `ProposedOperationLabel`,
  `proposed_operations`, `operation`, and `append_operations`
- module/function: deploy-plan report builder and renderer
- command/search: `rg -n "FUTURE_APPLY_PREVIEW_PHASE|PROPOSED_OPERATION_NOT_EXECUTED|OP_" crates/canic-cli/src/deploy/plan.rs`
- reachability: active `canic deploy plan <deployment>` text and JSON report
  paths
- exact issue: deploy-plan future-apply preview rows stored the closed phase
  label, operation label, and status label as raw strings even though the
  value sets are owned by the deploy-plan report model.

Risk:

Low. The emitted JSON/text labels were correct and covered by deploy-plan
renderer tests, but raw strings let proposed operation labels drift from the
stable 0.79 report contract without compiler help.

Recommendation:

Use private typed values for deploy-plan future-apply preview phase,
operation, and status labels. Keep diagnostic codes, subjects, details, next
actions, and embedded `DeploymentPlanV1` data as their existing report data.

Regression test:

Keep focused deploy-plan tests asserting unchanged text/JSON labels,
deterministic proposed-operation ordering, duplicate suppression, and
no-apply-safety wording.

Resolution:

- `ProposedOperationLabel.phase` now stores `ProposedOperationPhase`.
- `ProposedOperationLabel.label` now stores `ProposedOperationKind`.
- `ProposedOperationLabel.status` now stores `ProposedOperationStatus`.
- Text rendering formats typed labels through explicit label methods.
- JSON serialization still emits the existing `future_apply_preview`,
  operation, and `not_executed` labels.
- Command behavior, JSON fields, and text output remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the deploy-plan preview label typing change. |
| `cargo test --locked -p canic-cli deploy_plan` | pass | 20 filtered deploy-plan/medic-related CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-014: Deploy-Plan Diagnostics Own Category, Severity, And Source As Raw Strings

Severity: P3
Category: diagnostic_ownership / deploy_plan
Status: fixed
Owner: deploy-plan report wrapper
Current location: `crates/canic-cli/src/deploy/plan.rs`
Intended owner: typed deploy-plan report model, with text/JSON serialization
formatting stable diagnostic labels
Affected surfaces: internal, json
Release decision: fixed_in_0.83.10

Evidence:
- file: `crates/canic-cli/src/deploy/plan.rs`
- line or anchor: `PlanDiagnostic`, `DigestFact`,
  `sort_diagnostics`, `append_diagnostics`, and diagnostic fixture helpers
- module/function: deploy-plan diagnostic report builder and renderer
- command/search: `rg -n "category: &'static str|severity: &'static str|source: &'static str|SEVERITY_|CATEGORY_|SOURCE_" crates/canic-cli/src/deploy/plan.rs`
- reachability: active `canic deploy plan <deployment>` text and JSON report
  paths
- exact issue: deploy-plan diagnostics stored closed category, severity, and
  source labels as raw string values even though those label sets are owned by
  the deploy-plan report model.

Risk:

Low. The emitted JSON/text labels were correct and covered by deploy-plan
tests, but raw strings left diagnostic category/source/severity semantics
without compiler ownership.

Recommendation:

Use private typed values for deploy-plan diagnostic category, severity, and
source labels. Keep diagnostic codes, subjects, details, next actions, and
embedded `DeploymentPlanV1` data in their existing report shapes.

Regression test:

Keep focused deploy-plan tests asserting unchanged JSON field order, text
output, diagnostic ordering, status derivation, no-mutation contract, and
no-apply-safety wording.

Resolution:

- `PlanDiagnostic.category` now stores `PlanDiagnosticCategory`.
- `PlanDiagnostic.severity` now stores `PlanDiagnosticSeverity`.
- `PlanDiagnostic.source` now stores `PlanDiagnosticSource`.
- `DigestFact` uses typed category/source values.
- Sorting and text rendering format typed labels explicitly.
- JSON serialization still emits the existing category, severity, and source
  labels.
- Command behavior, JSON fields, and text output remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the deploy-plan diagnostic typing change. |
| `cargo test --locked -p canic-cli deploy_plan` | pass | 20 filtered deploy-plan/medic-related CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-015: State-Audit Reports Own Scope, Category, And Source As Raw Strings

Severity: P3
Category: diagnostic_ownership / state_audit
Status: fixed
Owner: state-audit report producer
Current location: `crates/canic-host/src/state_manifest/mod.rs` and
`crates/canic-cli/src/state/mod.rs`
Intended owner: typed state-audit report model, with text/JSON serialization
formatting stable report labels
Affected surfaces: internal, json
Release decision: fixed_in_0.83.11

Evidence:
- file: `crates/canic-host/src/state_manifest/mod.rs`
- line or anchor: `StateAuditReport`, `StateAuditCheck`, `SCOPE_*`,
  `CATEGORY_*`, `SOURCE_*`, `pass`, `warn`, `fail`, and `sort_checks`
- module/function: state-audit report builder
- command/search: `rg -n "category: &'static str|source: &'static str|scope: &'static str|SCOPE_|CATEGORY_|SOURCE_" crates/canic-host/src/state_manifest/mod.rs`
- reachability: active `canic state audit` text and JSON report paths
- exact issue: state-audit reports stored closed scope, category, and source
  labels as raw string values even though those label sets are owned by the
  state-audit report model.

Risk:

Low. The emitted JSON/text labels were correct and covered by state-audit
tests, but raw strings left state-audit scope/category/source semantics
without compiler ownership.

Recommendation:

Use typed values for state-audit scope, category, and source labels. Keep audit
codes, subjects, details, next actions, manifest data, and command strings in
their existing report shapes.

Regression test:

Keep focused state-audit tests asserting unchanged JSON schema version, scope,
check labels, text rendering, exit-code behavior, and medic summary behavior.

Resolution:

- `StateAuditReport.scope` now stores `StateAuditScope`.
- `StateAuditCheck.category` now stores `StateAuditCategory`.
- `StateAuditCheck.source` now stores `StateAuditSource`.
- Text rendering formats typed labels explicitly.
- JSON serialization still emits the existing scope, category, and source
  labels.
- Command behavior, JSON fields, and text output remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the state-audit typing change. |
| `cargo test --locked -p canic-host state_manifest --lib` | pass | 17 filtered state-manifest tests passed. |
| `cargo test --locked -p canic-cli state` | pass | 12 filtered state/medic/deploy-plan CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for `canic-host` targets. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-016: Deployment-Root Verification Reports Own Check Names As Raw Strings

Severity: P3
Category: diagnostic_ownership / deployment_truth
Status: fixed
Owner: deployment-root verification report producer
Current location:
`crates/canic-host/src/deployment_truth/root/report/checks.rs` and
`crates/canic-host/src/deployment_truth/root/report/validation.rs`
Intended owner: typed deployment-root verification report builder and
validator, with serialized `DeploymentRootVerificationCheckV1` names remaining
stable strings
Affected surfaces: internal, json
Release decision: fixed_in_0.83.11

Evidence:
- file: `crates/canic-host/src/deployment_truth/root/report/checks.rs`
- line or anchor: `root_verification_identity_checks`,
  `root_verification_evidence_checks`, and `push_check`
- module/function: deployment-root verification report builder
- command/search: `rg -n '"deployment_name"|"root_observation_source"|"source_check_id"' crates/canic-host/src/deployment_truth/root/report`
- reachability: active `canic deploy inspect root` and
  `canic deploy root verify` report/receipt paths
- exact issue: deployment-root verification report check names were repeated
  as raw strings in the report builder and validator even though the allowed
  row names are a closed report vocabulary.

Risk:

Low. The emitted report shape was correct and covered by root-verification
tests, but duplicated raw strings left the builder and validator free to drift.

Recommendation:

Use a typed internal check-name value for root-verification report rows. Keep
the persisted/serialized `DeploymentRootVerificationCheckV1.name` field as a
string so existing JSON reports, receipts, and digests remain unchanged.

Regression test:

Keep focused root-verification tests asserting accepted evidence, rejected
stale/missing check rows, digest stability, and CLI root command parsing.

Resolution:

- Added `RootVerificationCheckName` for the closed identity/evidence check-row
  names.
- The report builder now converts typed check names to the existing serialized
  labels at the `DeploymentRootVerificationCheckV1` boundary.
- The report validator now uses the same typed names for expected row lists and
  value checks.
- JSON report labels, report digest semantics, command behavior, and text
  output remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the root-verification check-name typing change. |
| `cargo test --locked -p canic-host root_verification --lib` | pass | 56 filtered deployment-root verification tests passed. |
| `cargo test --locked -p canic-cli deploy_root` | pass | 5 filtered deploy-root CLI tests passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for `canic-host` targets. |
| `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for `canic-cli` targets. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## Rejected / Non-Findings

See `rejected.md`.

## Deferred

See `deferred.md`.

## Recommended Slices

See `recommended-slices.md`.
