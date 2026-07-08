# Canic 0.83 Technical Debt Ledger

Schema version: 1
Audit date: 2026-07-08
Repo ref: baseline working tree after 0.82.41 push; current package surface 0.83.4
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
funding-status, and readiness-state labels.

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

## Rejected / Non-Findings

See `rejected.md`.

## Deferred

See `deferred.md`.

## Recommended Slices

See `recommended-slices.md`.
