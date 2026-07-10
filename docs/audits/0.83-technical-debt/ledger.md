# Canic 0.83 Technical Debt Ledger

Schema version: 1
Audit date: 2026-07-10
Repo ref: post-v0.83.27 working tree; current package surface 0.83.27
Status: pass

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
internal report values. The sixteenth follow-up fix tightens replay-policy
manifest command-kind labels into a typed manifest-owned value while leaving
runtime replay storage on `model::replay::CommandKind`. The seventeenth
follow-up fix tightens replay-policy manifest constructors so static
command-kind, command-manifest, quota-policy, and cycle-reserve labels are
typed at manifest call sites instead of being accepted as loose helper
arguments. The eighteenth follow-up fix tightens runtime bootstrap diagnostic
phase labels so process-local bootstrap status stores `BootstrapPhaseLabel`
while public bootstrap status responses keep the same phase strings. The
nineteenth follow-up fix tightens host install-root deployment-truth phase
labels so operation runners and completed-phase receipts use
`InstallPhaseLabel` while receipt JSON keeps the same phase strings. The
twentieth follow-up fix tightens host install-root timing summary output labels
so the timing renderer uses `InstallTimingLabel` while the table output keeps
the same phase labels. The twenty-first follow-up fix tightens host
install-root execution-preflight receipt labels so the receipt phase,
failure-code, and evidence-key labels are typed internally while receipt JSON
keeps the same strings. The twenty-second follow-up fix tightens
deployment-truth execution-preflight validation and text-output labels so
validation field names and text renderer field/section/status labels are typed
internally while error strings and operator text output keep the same labels.
The twenty-third follow-up fix tightens deployment-truth comparison report
validation and text-output labels so validation field names and text renderer
field/section/count/target/fallback labels are typed internally while error
strings and operator text output keep the same labels. The twenty-fourth
follow-up fix tightens deployment-truth authority report text-output labels so
report field/section/count/fallback labels and report-owned shared action
summary labels are typed internally while operator text output keeps the same
labels. The twenty-fifth follow-up fix hard-cuts delegated-auth verifier policy
and registry snapshot metadata out of the Candid trait surface while leaving
active delegated token, root proof, issuer proof, proof install, and proof
status Candid payloads unchanged. The twenty-sixth follow-up fix hard-cuts
`ids::BuildNetwork`, `ValidationReport`, and `ValidationIssue` out of the
Candid trait surface after audit showed they are local
runtime/config/policy/bootstrap validation metadata rather than active Candid
DTO payloads. The twenty-seventh follow-up fix tightens state manifest and
state-audit label ownership so state storage, migration policy, and audit
status labels are owned by their model/report enums. The twenty-eighth
follow-up fix tightens runtime introspection enum label ownership so runtime
domain enums own their canonical labels. The twenty-ninth follow-up fix
tightens deployment-truth status label ownership so deployment-truth model
status enums own stable status labels consumed by text renderers and medic.
The thirtieth follow-up fix tightens deployment-root verification text label
ownership so root verification and root observation enums own the exact labels
used by report and receipt text. The next four released fixes complete status,
root-verification, control-class, external-lifecycle, and promotion label
ownership through `CANIC-083-DEBT-031` to `CANIC-083-DEBT-034`.

The post-v0.83.27 closeout review fixes three additional findings. Receipt
duplicate detection now compares structured evidence rather than delimiter-
joined display strings. Promotion execution/status and staging evidence labels
now use model-owned labels while the unused public previous-receipt-kind label
method is hard-cut. The ledger metadata, handoff, and recommended-slice state
now agree that every recorded 0.83 finding is fixed and no deferred work
remains.

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
Affected surfaces: internal, rust_api
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

## CANIC-083-DEBT-017: Replay-Policy Manifests Own Command-Kind Labels As Raw Strings

Severity: P3
Category: replay_policy / boundary_ownership
Status: fixed
Owner: replay-policy manifest model
Current location:
`crates/canic-core/src/replay_policy/types.rs`,
`crates/canic-core/src/replay_policy/endpoint_manifest.rs`,
`crates/canic-core/src/replay_policy/pool_admin_manifest.rs`, and
`crates/canic-core/src/replay_policy/root_capability_manifest.rs`
Intended owner: typed replay-policy manifest value, with runtime replay storage
remaining on `model::replay::CommandKind`
Affected surfaces: internal
Release decision: fixed_in_0.83.12

Evidence:
- file: `crates/canic-core/src/replay_policy/types.rs`
- line or anchor: `ReplayPolicy::{ResponseIdempotent, ReplayProtected,
  MonotonicTransition, SnapshotConvergent, CommandDispatch,
  IntentionallyNonIdempotent}`
- module/function: replay-policy manifest type model
- command/search: `rg -n "command_kind: &'static str|ReplayPolicy::" crates/canic-core/src/replay_policy`
- reachability: active replay-policy manifest and release-blocker tests
- exact issue: replay-policy manifest command-kind labels were stored directly
  as raw `&'static str` fields even though the label vocabulary is owned by
  the replay-policy manifest, not by arbitrary callers.

Risk:

Low. The emitted/runtime command-kind strings were correct and covered by
manifest tests, but raw strings left manifest label ownership less explicit
than the runtime replay `CommandKind` storage boundary.

Recommendation:

Use a typed static manifest label for replay-policy command-kind metadata.
Keep runtime replay storage and guards on `model::replay::CommandKind`, because
that type validates command labels for operation IDs and persisted receipts.

Regression test:

Keep focused replay-policy tests asserting unchanged endpoint and command
manifest classifications, cost guard metadata, and release-candidate blocker
coverage.

Resolution:

- Added `ReplayCommandKindLabel` for static replay-policy manifest
  command-kind labels.
- `ReplayPolicy` variants now carry `ReplayCommandKindLabel` instead of raw
  `&'static str` command-kind fields.
- Endpoint, pool-admin, and root-capability manifest constructors convert the
  existing string labels into typed manifest labels at construction time.
- Runtime replay receipt storage, cost guards, workflow replay descriptors, and
  persisted command-kind handling continue to use `model::replay::CommandKind`.
- The Rust replay-policy manifest model changes as a pre-1.0 hard cut.
- Command behavior, endpoint surfaces, Candid, JSON, deployment truth,
  evidence/report schemas, and stable-state layout remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the replay-policy label typing change. |
| `cargo test --locked -p canic-core replay_policy --lib` | pass | 29 focused replay-policy tests passed. |
| `cargo clippy --locked -p canic-core --all-targets -- -D warnings` | pass | Clippy passed for `canic-core` targets. |

## CANIC-083-DEBT-018: Replay-Policy Manifest Constructors Accept Loose Manifest Labels

Severity: P3
Category: replay_policy / boundary_ownership
Status: fixed
Owner: replay-policy manifest constructors
Current location:
`crates/canic-core/src/replay_policy/endpoint_manifest.rs`,
`crates/canic-core/src/replay_policy/pool_admin_manifest.rs`, and
`crates/canic-core/src/replay_policy/root_capability_manifest.rs`
Intended owner: typed replay-policy manifest construction, with command-kind
and guard-policy labels typed at the manifest row call sites
Affected surfaces: internal, rust_api
Release decision: fixed_in_0.83.13

Evidence:
- file: `crates/canic-core/src/replay_policy/endpoint_manifest.rs`
- line or anchor: private `update_*` manifest constructors
- module/function: endpoint replay-policy manifest construction
- command/search: `rg -n "command_kind: &'static str" crates/canic-core/src/replay_policy -g '*.rs'`
- reachability: active replay-policy manifest and release-blocker tests
- exact issue: after `ReplayPolicy` itself carried a typed
  `ReplayCommandKindLabel`, private manifest constructors still accepted
  command-kind labels as raw `&'static str` inputs and converted them inside
  helper bodies.

Evidence:
- file: `crates/canic-core/src/replay_policy/types.rs`
- line or anchor: `ReplayPolicy::CommandDispatch.command_manifest`
- module/function: replay-policy manifest type model
- command/search: `rg -n "command_manifest" crates/canic-core/src/replay_policy -g '*.rs'`
- reachability: active replay-policy manifest and command-dispatch tests
- exact issue: the dispatch command-manifest ID was still represented as a raw
  `&'static str` field even though it is a closed replay-policy manifest
  label, parallel to the command-kind label.

Evidence:
- file: `crates/canic-core/src/replay_policy/types.rs`
- line or anchor: `EndpointReplayPolicy::quota_policy`,
  `EndpointReplayPolicy::cycle_reserve_policy`,
  `PoolAdminCommandReplayPolicy::quota_policy`, and
  `RootCapabilityCommandReplayPolicy::quota_policy`
- module/function: replay-policy manifest row types
- command/search: `rg -n "quota_policy: Option<&'static str>|cycle_reserve_policy: Option<&'static str>" crates/canic-core/src/replay_policy -g '*.rs'`
- reachability: active replay-policy manifest and cost guard metadata tests
- exact issue: quota and cycle-reserve policy IDs were still represented as
  loose optional string labels even though they are static replay-policy
  manifest guard-policy labels.

Evidence:
- file: `crates/canic-core/src/replay_policy/pool_admin_manifest.rs`
- line or anchor: private `pool_admin_*` manifest constructors
- module/function: pool-admin replay-policy manifest construction
- command/search: same as above
- reachability: active pool-admin command replay-policy manifest
- exact issue: command manifest constructors still accepted loose string
  command-kind labels.

Evidence:
- file: `crates/canic-core/src/replay_policy/root_capability_manifest.rs`
- line or anchor: private `root_capability_replay_protected`
- module/function: root-capability replay-policy manifest construction
- command/search: same as above
- reachability: active root-capability command replay-policy manifest
- exact issue: root-capability command manifest construction still accepted a
  raw string command-kind argument.

Risk:

Low. The stored manifest type was already typed, but the constructor boundary
still allowed accidental loose command-kind labels before the typed value was
created, and command-dispatch rows still had one adjacent raw manifest-owned
label. Quota and cycle-reserve policy IDs were another adjacent static
manifest-owned label set without compiler ownership.

Recommendation:

Make replay-policy manifest call sites construct `ReplayCommandKindLabel`
`ReplayCommandManifestLabel`, `ReplayQuotaPolicyLabel`, and
`ReplayCycleReservePolicyLabel` explicitly and make private constructors
accept the typed labels. Keep endpoint names and runtime replay command-kind
handling unchanged.

Regression test:

Keep focused replay-policy tests asserting unchanged endpoint and command
manifest classifications, cost guard metadata, and release-candidate blocker
coverage.

Resolution:

- Endpoint, pool-admin, and root-capability replay-policy manifest call sites
  now wrap command-kind string literals with a local `command_kind(...)`
  constructor.
- Endpoint command-dispatch rows now wrap command-manifest IDs with a local
  `command_manifest(...)` constructor.
- Replay quota and cycle-reserve policy constants now use
  `ReplayQuotaPolicyLabel` and `ReplayCycleReservePolicyLabel`.
- Private replay-policy manifest helpers now accept `ReplayCommandKindLabel`
  `ReplayCommandManifestLabel`, `ReplayQuotaPolicyLabel`, and
  `ReplayCycleReservePolicyLabel` rather than raw `&'static str` manifest
  label arguments.
- The remaining strings in replay-policy manifests are endpoint names,
  reasons, and tests.
- Runtime replay receipt storage, cost guards, workflow replay descriptors, and
  persisted command-kind handling continue to use `model::replay::CommandKind`.
- Command behavior, endpoint surfaces, Candid, JSON, deployment truth,
  evidence/report schemas, and stable-state layout remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the manifest constructor typing change. |
| `cargo test --locked -p canic-core replay_policy --lib` | pass | 29 focused replay-policy tests passed. |
| `cargo clippy --locked -p canic-core --all-targets -- -D warnings` | pass | Clippy passed for `canic-core` targets. |

## CANIC-083-DEBT-019: Runtime Bootstrap Status Owns Phase Labels As Raw Strings

Severity: P3
Category: runtime_status / boundary_ownership
Status: fixed
Owner: runtime bootstrap status ops
Current location: `crates/canic-core/src/ops/runtime/bootstrap.rs`,
`crates/canic-core/src/lifecycle/init/nonroot.rs`, and
`crates/canic-core/src/lifecycle/upgrade/nonroot.rs`
Intended owner: typed runtime bootstrap phase label, with
`BootstrapStatusResponse.phase` remaining the string DTO boundary
Affected surfaces: internal, rust_api
Release decision: fixed_in_0.83.14

Evidence:
- file: `crates/canic-core/src/ops/runtime/bootstrap.rs`
- line or anchor: `BootstrapStatusRecord.phase`,
  `BootstrapStatusOps::set_phase`, `mark_failed`, and `mark_ready`
- module/function: runtime bootstrap diagnostic status storage
- command/search: `rg -n "BootstrapStatusOps::set_phase\\(\"|phase: &'static str" crates/canic-core/src -g '*.rs'`
- reachability: active `canic_bootstrap_status` query projection and runtime
  introspection recent-failure metadata
- exact issue: process-local bootstrap status stored and accepted phase labels
  as raw `&'static str` values even though the label namespace is owned by
  runtime bootstrap diagnostics.

Risk:

Low. The public DTO already emitted string phase labels, but raw internal
phase values let lifecycle call sites pass arbitrary strings directly below
the bootstrap status boundary.

Recommendation:

Use a typed bootstrap phase label in runtime bootstrap ops and lifecycle call
sites. Keep `BootstrapStatusResponse.phase` as a string so
Candid/JSON/status output is unchanged.

Regression test:

Keep focused bootstrap/runtime status tests asserting idle, failed, ready, and
recent-failure correlation metadata.

Resolution:

- Added `BootstrapPhaseLabel`.
- Added associated constants for the maintained idle, failed, ready, root-init,
  root-upgrade, and nonroot lifecycle phase labels.
- `BootstrapStatusRecord.phase` now stores `BootstrapPhaseLabel`.
- `BootstrapStatusOps::set_phase` accepts `BootstrapPhaseLabel`.
- Root and nonroot lifecycle bootstrap scheduling pass typed phase constants
  instead of constructing labels from raw strings.
- `control_plane_support` re-exports `BootstrapPhaseLabel` alongside
  `BootstrapStatusOps` for root bootstrap workflow call sites.
- `snapshot()` still serializes the same phase strings.
- `mark_failed` preserves the same redacted recent-failure correlation ID
  behavior.
- Command behavior, endpoint surfaces, Candid, JSON, deployment truth,
  evidence/report schemas, and stable-state layout remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the bootstrap phase label typing change. |
| `cargo test --locked -p canic-core bootstrap --lib` | pass | 14 focused bootstrap/runtime tests passed. |
| `cargo check --locked -p canic-control-plane` | pass | Checked root bootstrap workflow call sites that consume the typed phase label through `control_plane_support`. |
| `cargo clippy --locked -p canic-core --all-targets -- -D warnings` | pass | Clippy passed for `canic-core` targets. |

## CANIC-083-DEBT-020: Install-Root Receipts Own Phase Labels As Raw Strings

Severity: P3
Category: host / deployment_truth / boundary_ownership
Status: fixed
Owner: host install-root operation and receipt builders
Current location: `crates/canic-host/src/install_root/operations/phase.rs`,
`crates/canic-host/src/install_root/phase_receipts.rs`,
`crates/canic-host/src/install_root/artifact_promotion/mod.rs`,
`crates/canic-host/src/install_root/deployment_truth_gate.rs`,
`crates/canic-host/src/install_root/preparation/mod.rs`,
`crates/canic-host/src/install_root/plan_artifacts/mod.rs`,
`crates/canic-host/src/install_root/install_state/mod.rs`,
`crates/canic-host/src/install_root/activation/mod.rs`, and
`crates/canic-host/src/install_root/staging.rs`
Intended owner: host install-root phase label type, with deployment-truth
receipt DTOs continuing to serialize phase labels as strings
Affected surfaces: internal, rust_api
Release decision: fixed_in_0.83.15

Evidence:
- file: `crates/canic-host/src/install_root/operations/phase.rs`
- line or anchor: `InstallPhaseOperation::phase`
- module/function: install-root operation runner
- command/search: `rg -n "phase: &'static str|fn phase\\(&self\\) -> &'static str|phase: \\\"" crates/canic-host/src/install_root -g '*.rs'`
- reachability: active `canic install-root` deployment-truth receipt creation
- exact issue: install-root operation phases and completed-phase receipts used
  raw string labels even though the maintained receipt phase namespace is
  closed within the current install-root flow.

Evidence:
- file: `crates/canic-host/src/install_root/artifact_promotion/mod.rs`
- line or anchor: `promotion_install_deployment_receipt`
- module/function: artifact-promotion install deployment receipt builder
- command/search: `rg -n "promoted_plan_install|materialize_artifacts" crates/canic-host/src/install_root -g '*.rs'`
- reachability: active artifact-promotion deployment receipt path
- exact issue: artifact-promotion role receipts and operation IDs duplicated
  the promoted-install phase label instead of deriving them from the same
  install-root phase namespace.

Risk:

Low. Receipt strings were already stable and tested, but raw internal labels
made it easier for operation IDs, phase receipts, and role-phase receipts to
drift inside the host install-root flow.

Recommendation:

Introduce a host-owned `InstallPhaseLabel` with constants for maintained
install-root phases. Keep deployment-truth DTO fields as strings by converting
through `InstallPhaseLabel::as_str()` only at the receipt construction
boundary.

Regression test:

Keep install-root receipt tests asserting unchanged operation IDs, phase
receipt strings, role-phase receipt strings, and failure receipt codes.

Resolution:

- Added `InstallPhaseLabel`.
- `InstallPhaseOperation::phase` now returns `InstallPhaseLabel`.
- `CompletedInstallPhase`, role receipt creation, deployment-truth phase
  receipt creation, and receipt operation ID construction now use typed phase
  labels internally.
- Artifact-promotion install receipts derive `promoted_plan_install` phase
  receipts, role receipts, and operation IDs from the same label constant.
- Activation, staging, preparation, plan-artifact, install-state, and tests use
  the maintained phase constants.
- Receipt JSON phase strings, operation IDs, command behavior, endpoint
  surfaces, Candid, JSON schemas, deployment truth schema, evidence/report
  schemas, and stable-state layout remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the install-root phase label typing change. |
| `cargo check --locked -p canic-host` | pass | Checked the host install-root phase label changes. |
| `cargo test --locked -p canic-host install_truth` | pass | 36 focused install-truth tests passed. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for `canic-host` targets. |

## CANIC-083-DEBT-021: Install-Root Timing Renderer Owns Row Labels As Raw Strings

Severity: P3
Category: host / cli_output / boundary_ownership
Status: fixed
Owner: host install-root timing renderer
Current location: `crates/canic-host/src/install_root/output/mod.rs` and
`crates/canic-host/src/install_root/timing/mod.rs`
Intended owner: host install-root timing label type, with timing table output
continuing to render labels as strings
Affected surfaces: internal
Release decision: fixed_in_0.83.15

Evidence:
- file: `crates/canic-host/src/install_root/output/mod.rs`
- line or anchor: `render_install_timing_summary`
- module/function: install-root timing summary renderer
- command/search: `rg -n "timing_row\\(\\\"" crates/canic-host/src/install_root -g '*.rs'`
- reachability: active install-root CLI timing summary output
- exact issue: install-root timing rows used raw string labels directly in the
  renderer even though the maintained timing row namespace is closed by
  `InstallTimingSummary`.

Risk:

Low. Output labels were already stable and tested, but raw renderer-owned
labels made timing rows easier to mistype or drift from the timing summary
fields.

Recommendation:

Introduce a host-owned `InstallTimingLabel` with constants for maintained
install-root timing rows. Keep the rendered timing table unchanged by
formatting through `InstallTimingLabel::as_str()` at the row boundary.

Regression test:

Keep the install timing summary table test asserting unchanged row labels and
elapsed-time formatting.

Resolution:

- Added `InstallTimingLabel`.
- `render_install_timing_summary` now builds rows from typed timing labels.
- Timing table row strings, command behavior, endpoint surfaces, Candid, JSON
  schemas, deployment truth schema, evidence/report schemas, and stable-state
  layout remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the install-root timing label typing change. |
| `cargo check --locked -p canic-host` | pass | Checked the host timing label changes. |
| `cargo test --locked -p canic-host install_timing_summary` | pass | Focused timing summary renderer test passed. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for `canic-host` targets. |

## CANIC-083-DEBT-022: Install-Root Execution Preflight Receipt Owns Labels As Raw Strings

Severity: P3
Category: host / deployment_truth / boundary_ownership
Status: fixed
Owner: host install-root execution preflight receipt builder and
deployment-truth execution preflight builder
Current location: `crates/canic-host/src/install_root/execution_preflight.rs`
and `crates/canic-host/src/deployment_truth/executor.rs`
Intended owner: host install-root phase label type plus execution-preflight
receipt label type, plus deployment-truth current-install execution phase
label type, with deployment-truth receipt/preflight DTOs continuing to
serialize labels as strings
Affected surfaces: internal
Release decision: fixed_in_0.83.16

Evidence:
- file: `crates/canic-host/src/install_root/execution_preflight.rs`
- line or anchor: `write_current_install_execution_preflight_receipt`
- module/function: current install execution-preflight receipt creation
- command/search: `rg -n "execution_preflight|execution_preflight_blocked|execution_preflight_status" crates/canic-host/src/install_root crates/canic-host/src/deployment_truth -g '*.rs'`
- reachability: active current install deployment-truth execution-preflight
  receipt path
- exact issue: execution-preflight receipt operation IDs, phase receipts,
  failure command-result codes, and evidence keys were built from raw string
  labels inside the receipt builder. The execution-preflight planned-phase
  list was also owned as raw string labels in the deployment-truth preflight
  builder.

Risk:

Low. Serialized receipt strings were already stable and tested, but the
current-install execution-preflight receipt path had several adjacent raw
labels that could drift independently. The planned-phase list had the same
drift risk against the install execution phase vocabulary.

Recommendation:

Use `InstallPhaseLabel::EXECUTION_PREFLIGHT` for execution-preflight receipt
phase and operation ID construction. Use a local typed
`ExecutionPreflightReceiptLabel` for the receipt failure code and evidence
keys. Keep deployment-truth DTO strings unchanged.
Use a private deployment-truth planned-phase label type for the
current-install execution preflight planned phase list.

Regression test:

Keep execution-preflight receipt tests asserting unchanged operation IDs,
phase strings, evidence strings, and blocked receipt behavior.
Keep execution-preflight tests asserting unchanged planned-phase strings.

Resolution:

- Added `InstallPhaseLabel::EXECUTION_PREFLIGHT`.
- Execution-preflight receipt operation IDs and phase receipts now derive from
  the typed install phase label.
- Added `ExecutionPreflightReceiptLabel`.
- Execution-preflight blocked command-result code and evidence keys now derive
  from typed receipt labels.
- Added `CurrentInstallExecutionPhaseLabel`.
- Deployment-truth execution-preflight planned phases now derive from typed
  current-install phase labels.
- Receipt JSON phase strings, operation IDs, evidence strings,
  planned-phase strings, command behavior, endpoint surfaces, Candid, JSON
  schemas, deployment truth schema, evidence/report schemas, and stable-state
  layout remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the execution-preflight label typing change. |
| `cargo check --locked -p canic-host` | pass | Checked the host execution-preflight label changes. |
| `cargo test --locked -p canic-host execution_preflight` | pass | 12 focused execution-preflight tests passed. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for `canic-host` targets. |

## CANIC-083-DEBT-023: Execution Preflight Validation, Blocker, And Text Labels Are Raw Strings

Severity: P3
Category: host / deployment_truth / boundary_ownership
Status: fixed
Owner: deployment-truth execution preflight validation, blocker, and text
renderer
Current location: `crates/canic-host/src/deployment_truth/executor.rs`
and `crates/canic-host/src/deployment_truth/text/execution_preflight.rs`
Intended owner: deployment-truth execution preflight field, blocker, subject,
and text label types, with validation errors, safety findings, and operator
text continuing to format labels as strings
Affected surfaces: internal
Release decision: fixed_in_0.83.17

Evidence:
- file: `crates/canic-host/src/deployment_truth/executor.rs`
- line or anchor: `validate_deployment_execution_preflight`
- module/function: execution-preflight validation
- command/search: `rg -n "plan_id|safety_report_id|authority_plan_id|required_capabilities|missing_capabilities" crates/canic-host/src/deployment_truth/executor.rs crates/canic-host/src/deployment_truth/text/execution_preflight.rs`
- reachability: active deployment-truth execution-preflight validation path
- exact issue: validation field names were passed as raw strings into missing
  field, duplicate capability, and source-check mismatch errors.

Evidence:
- file: `crates/canic-host/src/deployment_truth/executor.rs`
- line or anchor: `deployment_execution_blockers`
- module/function: execution-preflight blocker construction
- command/search: `rg -n "deployment_safety_blocked|executor_capability_missing|authority_controller_change_pending|authority_external_action_required|authority_observation_missing" crates/canic-host/src/deployment_truth/executor.rs`
- reachability: active deployment-truth execution-preflight blocker path
- exact issue: execution-preflight safety-finding codes and the static
  authority fallback subject were owned as raw strings.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/execution_preflight.rs`
- line or anchor: `deployment_execution_preflight_text`
- module/function: execution-preflight operator text renderer
- command/search: `rg -n "mode: passive|planned_phases|required_capabilities|missing_capabilities|blockers" crates/canic-host/src/deployment_truth/text/execution_preflight.rs`
- reachability: active execution-preflight text rendering path
- exact issue: text field, section, and status labels were owned as raw strings
  in the renderer.

Risk:

Low. Error field labels, safety-finding codes, fallback subjects, and operator
text labels were already stable and tested, but the validation, blocker, and
text paths duplicated the same execution-preflight vocabulary as ad hoc raw
strings.

Recommendation:

Introduce private execution-preflight field, blocker, subject, and text label
types. Keep validation error field strings, safety-finding code/subject
strings, and operator text output unchanged by converting labels to strings
only at the error/finding/text boundary.

Regression test:

Keep execution-preflight tests asserting unchanged source-check mismatch field
labels, safety-finding codes/subjects, and passive readiness text labels.

Resolution:

- Added `DeploymentExecutionPreflightFieldLabel`.
- Execution-preflight validation now passes typed field labels into missing
  field, duplicate capability, and source-check mismatch helpers.
- Added `ExecutionPreflightTextLabel`.
- Execution-preflight text rendering now derives title, field, section,
  status, and list labels from typed text labels.
- Added `DeploymentExecutionPreflightBlockerCode`.
- Execution-preflight blocker construction now derives maintained
  safety-finding codes from typed blocker-code labels while keeping the public
  string constants used by tests.
- Added `DeploymentExecutionPreflightSubjectLabel`.
- The static authority fallback subject now derives from a typed subject label.
- Error field strings, operator text output, blocker code strings, fallback
  subject string, command behavior, endpoint surfaces, Candid, JSON schemas,
  deployment truth schema, evidence/report schemas, and stable-state layout
  remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the execution-preflight validation/text label change. |
| `cargo test --locked -p canic-host execution_preflight` | pass | 12 focused execution-preflight tests passed after blocker-code label typing. |
| `cargo check --locked -p canic-host` | pass | Checked the host execution-preflight validation/blocker/text label changes. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for `canic-host` targets. |

## CANIC-083-DEBT-024: Comparison Report Validation And Text Labels Are Raw Strings

Severity: P3
Category: host / deployment_truth / boundary_ownership
Status: fixed
Owner: deployment-truth comparison report validation and text renderer
Current location: `crates/canic-host/src/deployment_truth/multi/validation.rs`
and `crates/canic-host/src/deployment_truth/text/comparison.rs`
Intended owner: deployment-truth comparison field and text label types, with
validation errors and operator text continuing to format labels as strings
Affected surfaces: internal
Release decision: fixed_in_0.83.18

Evidence:
- file: `crates/canic-host/src/deployment_truth/multi/validation.rs`
- line or anchor: `validate_deployment_comparison_report`
- module/function: comparison report validation
- command/search: `rg -n "report_id|report_digest|compared_at|left|right|field_name" crates/canic-host/src/deployment_truth/multi/validation.rs`
- reachability: active deployment-truth comparison validation path
- exact issue: comparison report validation field names, target sides, and
  target fields were passed around as raw strings, including a string fallback
  helper for target field names.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/comparison.rs`
- line or anchor: `deployment_comparison_report_text`
- module/function: comparison report operator text renderer
- command/search: `rg -n "mode: passive|execution: none|identity_diff|next_actions|missing" crates/canic-host/src/deployment_truth/text/comparison.rs`
- reachability: active deployment-truth comparison text rendering path
- exact issue: comparison report title, field, section, count, target, and
  fallback labels were owned as raw strings in the renderer.

Risk:

Low. Error field labels and operator text labels were already stable and
tested, but the comparison validation and text-rendering paths duplicated the
same report vocabulary as ad hoc raw strings.

Recommendation:

Introduce private comparison report field and text label types. Keep validation
error field strings and operator text output unchanged by converting labels to
strings only at the error/text boundary.

Regression test:

Keep comparison tests asserting unchanged digest mismatch fields, missing target
field labels, passive text labels, diff sections, and next-action labels.

Resolution:

- Added `DeploymentComparisonFieldLabel`.
- Added typed comparison target side and target field enums for validation.
- Comparison validation now derives required-field and digest-mismatch field
  strings from typed labels.
- Removed the raw-string target field-name fallback helper.
- Added `DeploymentComparisonTextLabel`.
- Comparison text rendering now derives title, field, count, section, target,
  status, and fallback labels from typed text labels.
- Validation error field strings, operator text output, command behavior,
  endpoint surfaces, Candid, JSON schemas, deployment truth schema,
  evidence/report schemas, and stable-state layout remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the comparison validation/text label change. |
| `cargo test --locked -p canic-host comparison` | pass | 8 focused comparison tests passed. |
| `cargo check --locked -p canic-host` | pass | Checked the host comparison validation/text label changes. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for `canic-host` targets. |

## CANIC-083-DEBT-025: Authority Report Text Labels Are Raw Strings

Severity: P3
Category: host / deployment_truth / boundary_ownership
Status: fixed
Owner: deployment-truth authority report text renderer
Current location:
`crates/canic-host/src/deployment_truth/text/authority/report/mod.rs` and
`crates/canic-host/src/deployment_truth/text/authority/shared/mod.rs`
Intended owner: deployment-truth authority report text label types, with
operator text continuing to format labels as strings
Affected surfaces: internal
Release decision: fixed_in_0.83.19

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/authority/report/mod.rs`
- line or anchor: `authority_report_text`
- module/function: authority report operator text renderer
- command/search: `rg -n "Authority reconciliation report|mode: dry_run|apply_readiness|hard_failures|observation_gaps|not recorded" crates/canic-host/src/deployment_truth/text/authority -S`
- reachability: active authority dry-run report text rendering path
- exact issue: authority report title, field, section, count, fallback, and
  list labels were owned as raw strings in the renderer.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/authority/shared/mod.rs`
- line or anchor: `append_blockers`, `append_next_actions`,
  `append_authority_action_summary`, and `authority_apply_blocker_label`
- module/function: authority report text helper labels
- command/search: same as above
- reachability: active authority report text helper path
- exact issue: report-owned shared labels for blockers, next actions,
  automatic actions, external actions, and authority apply blocker strings were
  owned as raw strings in shared helper code.

Risk:

Low. Operator text labels were already stable and tested, but the authority
report renderer and its report-owned shared helper labels duplicated maintained
report vocabulary as ad hoc raw strings.

Recommendation:

Introduce private authority report and shared authority text label types. Keep
operator text output unchanged by converting labels to strings only at the text
boundary.

Regression test:

Keep authority tests asserting unchanged authority report title, dry-run mode,
check ID, status, controller delta, and action-summary text.

Resolution:

- Added `AuthorityReportTextLabel`.
- Authority report text rendering now derives report title, fields, count rows,
  apply-readiness rows, fallback labels, and hard-failure/observation-gap
  section labels from typed text labels.
- Added `AuthoritySharedTextLabel`.
- Report-owned shared helper labels for blockers, next actions, automatic
  actions, external actions, hard failures, observation gaps, and apply blocker
  labels now derive from typed text labels.
- Operator text output, command behavior, endpoint surfaces, Candid, JSON
  schemas, deployment truth schema, evidence/report schemas, and stable-state
  layout remain unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the authority report text label change. |
| `cargo test --locked -p canic-host authority` | pass | 72 focused authority tests passed. |
| `cargo check --locked -p canic-host` | pass | Checked the host authority report text label changes. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for `canic-host` targets. |

## CANIC-083-DEBT-026: Delegated Auth Policy Snapshot Metadata Derives CandidType

Severity: P3
Category: core / auth / candid_surface
Status: fixed
Owner: delegated-auth verifier policy and canonical registry metadata
Current location: `crates/canic-core/src/dto/auth/proof.rs`
Intended owner: local verifier policy/canonical-hash metadata, not Candid
endpoint payloads
Affected surfaces: Rust trait surface
Release decision: fixed_in_0.83.20

Evidence:
- file: `crates/canic-core/src/dto/auth/proof.rs`
- line or anchor: `RootProofMode`, `RootKeyPolicyV1`,
  `DelegatedAuthRegistrySnapshotV1`, and
  `DelegatedAuthIssuerPolicySnapshotV1`
- module/function: delegated-auth verifier policy and registry snapshot DTO
  metadata
- command/search: `rg -n "RootKeyPolicyV1|DelegatedAuthRegistrySnapshotV1|DelegatedAuthIssuerPolicySnapshotV1|RootProofMode" crates canisters docs --glob '!target'`
- reachability: active canonical root-key policy hash and delegated-auth
  registry snapshot hash paths
- exact issue: local verifier policy and registry snapshot metadata carried
  `CandidType` derives even though these shapes are used for configured policy,
  canonical hashing, verifier config, and registry snapshots rather than active
  Candid endpoint payloads.

Evidence:
- file: `crates/canic/tests/protocol_surface.rs`
- line or anchor: `assert_root_delegation_batch_dtos_roundtrip`
- module/function: delegated-auth protocol-surface test
- command/search: `rg -n "RootKeyPolicyV1|DelegatedAuthRegistrySnapshotV1|RootProofMode" crates/canic/tests/protocol_surface.rs`
- reachability: protocol-surface test only
- exact issue: the protocol-surface test pinned the local verifier policy and
  registry snapshot metadata as Candid round-trip payloads, which made their
  Candid derives look intentional despite no active endpoint boundary relying
  on them.

Risk:

Low. The extra derives bloated the Rust/Candid trait surface and blurred the
DTO boundary, but active delegated token, root proof, issuer proof, proof
install, and proof status payloads already have separate Candid-bearing types
and tests.

Recommendation:

Remove `CandidType` from delegated-auth verifier policy and registry snapshot
metadata that are not active Candid endpoint payloads. Keep active token, proof,
install, and status payload derives unchanged. Remove stale protocol-surface
round-trip assertions that treat the metadata as Candid protocol payloads.

Regression test:

Keep protocol-surface round trips for `RootProof`,
`RootDelegationProofBatchInstallRequest`, active delegation proof status, issuer
renewal DTOs, and the delegated-auth Candid endpoint surfaces. Keep auth unit
tests covering canonical registry/policy hashes and delegated-token
verification.

Resolution:

- Removed `CandidType` from `RootProofMode`.
- Removed `CandidType` from `RootKeyPolicyV1`.
- Removed `CandidType` from `DelegatedAuthRegistrySnapshotV1`.
- Removed `CandidType` from `DelegatedAuthIssuerPolicySnapshotV1`.
- Removed stale protocol-surface Candid round-trip assertions for those local
  metadata shapes.
- Kept active delegated token, root proof, issuer proof, proof install, proof
  status, endpoint, JSON, deployment truth, evidence/report, and stable-state
  surfaces unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the Candid surface hard cut. |
| `cargo check --locked -p canic-core -p canic` | pass | Checked `canic-core` and the public `canic` facade after removing the derives. |
| `cargo test --locked -p canic --test protocol_surface` | pass | 19 protocol-surface tests passed, including the remaining delegated-auth Candid payload round trips. |
| `cargo test --locked -p canic-core auth --lib` | pass | 253 focused auth tests passed. |
| `cargo clippy --locked -p canic-core --all-targets -- -D warnings` | pass | Clippy passed for the affected core targets. |
| `cargo fmt --all -- --check` | pass | Formatting check passed. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |

## CANIC-083-DEBT-027: Local Metadata Derives CandidType Without Candid Boundary

Severity: P3
Category: core / ids / bootstrap / candid_surface
Status: fixed
Owner: build-network runtime/config/policy label and root bootstrap validation
metadata
Current location: `crates/canic-core/src/ids/network.rs` and
`crates/canic-core/src/dto/validation.rs`
Intended owner: local runtime/config/policy enum plus bootstrap validation
metadata with serde support, not Candid DTO payloads
Affected surfaces: Rust trait surface
Release decision: fixed_in_0.83.21

Evidence:
- file: `crates/canic-core/src/ids/network.rs`
- line or anchor: `BuildNetwork`
- module/function: build-network identifier enum
- command/search: `rg -n "BuildNetwork" crates canisters docs --glob '!target'`
- reachability: active runtime network detection, auth policy, chain-key
  signer/verifier policy, and bootstrap workflow paths
- exact issue: `BuildNetwork` still derived `CandidType` after the only
  Candid-bearing consumer found in this slice, `RootKeyPolicyV1`, was hard-cut
  out of the Candid trait surface.

Evidence:
- file: `crates/canic-core/src/dto/validation.rs`
- line or anchor: `ValidationReport` and `ValidationIssue`
- module/function: root bootstrap validation metadata
- command/search: `rg -n "root_validate_state|ValidationReport|ValidationIssue" crates/canic-core/src crates/canic-control-plane/src crates/canic/src crates/canic/tests canisters docs --glob '!target'`
- reachability: active root bootstrap validation path in
  `canic-control-plane`
- exact issue: bootstrap validation metadata still derived `CandidType` even
  though it is used to summarize root bootstrap validation failures and is not
  exposed as an active endpoint DTO.

Evidence:
- file: checked-in `.did` files
- line or anchor: not present
- module/function: public Candid surface
- command/search: `rg -n "BuildNetwork|ValidationReport|ValidationIssue" -g '*.did'`
- reachability: active and fixture Candid declarations
- exact issue: no checked-in Candid declaration references `BuildNetwork`,
  `ValidationReport`, or `ValidationIssue`, so the derives were not justified
  by the maintained `.did` surface.

Risk:

Low. The derives bloated the Rust/Candid trait surface for local metadata and
blurred the boundary between runtime/bootstrap data and Candid DTOs.
`BuildNetwork` serde support and stable `as_str()` labels remain intact, and
bootstrap validation metadata still supports serde deserialization.

Recommendation:

Remove `CandidType` from `BuildNetwork`, `ValidationReport`, and
`ValidationIssue`. Keep active endpoint payloads and delegated-auth Candid DTOs
unchanged.

Regression test:

Compile the affected core/facade/control-plane packages, run focused auth tests
that exercise build-network enforcement in signer, verifier, config, and
workflow paths, and compile the control-plane bootstrap validation path.

Resolution:

- Removed the `CandidType` derive from `BuildNetwork`.
- Removed the now-unused Candid import from `ids::network`.
- Removed the `CandidType` derives from `ValidationReport` and
  `ValidationIssue`.
- Replaced the validation DTO prelude import with direct `serde::Deserialize`.
- Kept endpoint surfaces, Candid payloads, JSON schemas, deployment truth,
  evidence/report schemas, and stable-state layout unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the local metadata Candid hard cut. |
| `cargo check --locked -p canic-core -p canic -p canic-control-plane` | pass | Checked the core, facade, and control-plane packages after removing the derive. |
| `cargo test --locked -p canic-core auth --lib` | pass | 253 focused auth tests passed, including build-network policy paths. |
| `cargo test --locked -p canic-control-plane --lib` | pass | 51 control-plane lib tests passed after the bootstrap validation metadata derive removal. |
| `cargo test --locked -p canic --test protocol_surface` | pass | 19 protocol-surface tests passed after the Candid trait-surface hard cut. |
| `cargo clippy --locked -p canic-core -p canic-control-plane --all-targets -- -D warnings` | pass | Clippy passed for the affected core and control-plane targets. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |

## CANIC-083-DEBT-028: State Manifest/Audit Labels Owned By Renderers Instead Of Model Owners

Severity: P3
Category: core / state_contract / runtime / cli_renderer / state_audit
Status: fixed
Owner: state manifest storage and migration-policy schema labels, plus
state-audit status labels
Current location: `crates/canic-cli/src/state/mod.rs`,
`crates/canic-cli/src/medic/mod.rs`, and
`crates/canic-core/src/api/runtime/mod.rs`
Intended owner: `crates/canic-core/src/state_contract.rs` and
`crates/canic-host/src/state_manifest/mod.rs`
Affected surfaces: Rust internals only
Release decision: fixed_in_0.83.22

Evidence:
- file: `crates/canic-cli/src/state/mod.rs`
- line or anchor: `status_label`, `storage_label`, and
  `migration_policy_label`
- module/function: `render_audit_text` and `render_manifest_text`
- command/search: `rg -n "status_label|storage_label|migration_policy_label|state_storage_name" crates/canic-core/src crates/canic-cli/src crates/canic-host/src crates/canic-control-plane/src -g '*.rs'`
- reachability: active `canic state manifest` text rendering
- exact issue: the CLI renderer locally matched `StateAuditStatus`,
  `StateStorage`, and `MigrationPolicy` variants into stable report/schema
  labels even though those labels belong to the state-audit report model or
  state contract model.

Evidence:
- file: `crates/canic-cli/src/medic/mod.rs`
- line or anchor: `state_audit_status_label`
- module/function: `state_audit_project_check`
- command/search: `rg -n "status_label|storage_label|migration_policy_label|state_storage_name" crates/canic-core/src crates/canic-cli/src crates/canic-host/src crates/canic-control-plane/src -g '*.rs'`
- reachability: active `canic medic project` state-audit summary check
- exact issue: medic locally matched `StateAuditStatus` variants into stable
  report labels even though the state-audit report model owns that label
  meaning.

Evidence:
- file: `crates/canic-core/src/api/runtime/mod.rs`
- line or anchor: `state_storage_name`
- module/function: `state_summary`
- command/search: `rg -n "state_storage_name" crates/canic-core/src crates/canic-cli/src crates/canic-host/src crates/canic-control-plane/src -g '*.rs'`
- reachability: active runtime introspection state summary builder
- exact issue: runtime state summaries duplicated the `StateStorage` label match
  instead of consuming a model-owned label.

Risk:

Low. The duplicated matches could drift from the serde `snake_case` schema and
report labels over time, especially because the same storage label appears in
both host/CLI state manifest output and runtime state summaries.

Recommendation:

Move the stable string labels to `StateStorage::as_str()`,
`MigrationPolicy::as_str()`, and `StateAuditStatus::label()`, then have
CLI/runtime consumers render from those owner-defined labels.

Regression test:

Pin the enum-owned labels in state-contract and state-manifest tests, then run
the state CLI, runtime, and host state-manifest test suites to confirm rendered
output and audit behavior remain unchanged.

Resolution:

- Added `StateStorage::as_str()`.
- Added `MigrationPolicy::as_str()`.
- Added `StateAuditStatus::label()`.
- Replaced the state CLI text renderer's duplicate storage and
  migration-policy label functions with calls to the state contract methods.
- Replaced the state CLI audit renderer's duplicate status label function with
  calls to the state-audit report method.
- Replaced medic's duplicate state-audit status label function with calls to
  the state-audit report method.
- Replaced the runtime state-summary storage-label helper with
  `StateStorage::as_str()`.
- Kept state manifest JSON labels, text output, runtime state summary strings,
  command behavior, endpoint surfaces, Candid, deployment truth,
  evidence/report schemas, and stable-state layout unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted the state-contract label ownership cleanup. |
| `cargo check --locked -p canic-core -p canic-cli -p canic-host` | pass | Checked affected core, CLI, and host packages. |
| `cargo test --locked -p canic-core state_contract --lib` | pass | State-contract tests passed, including the state storage and migration-policy label assertions. |
| `cargo test --locked -p canic-core runtime --lib` | pass | Runtime tests passed after the state-summary label helper was removed. |
| `cargo test --locked -p canic-cli state` | pass | State CLI tests passed after renderer label ownership moved to the model/report owners. |
| `cargo test --locked -p canic-cli medic` | pass | Medic tests passed after state-audit summary label ownership moved to the report model. |
| `cargo test --locked -p canic-host state_manifest --lib` | pass | Host state-manifest audit tests passed, including the audit-status label assertions. |
| `cargo clippy --locked -p canic-core -p canic-cli -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for the affected packages. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |

## CANIC-083-DEBT-029: Runtime Introspection Labels Owned By Inspect Renderer Instead Of Domain Enums

Severity: P3
Category: core / runtime / dto / cli_renderer
Status: fixed
Owner: runtime introspection enum labels
Current location: `crates/canic-cli/src/inspect/mod.rs` and
`crates/canic-core/src/dto/runtime.rs`
Intended owner: `crates/canic-core/src/domain/runtime.rs`
Affected surfaces: Rust internals only
Release decision: fixed_in_0.83.23

Evidence:
- file: `crates/canic-cli/src/inspect/mod.rs`
- line or anchor: `runtime_status_label`, `timer_status_label`,
  `state_domain_status_label`, and `failure_severity_label`
- module/function: `render_text_report`, `append_runtime_metadata_lines`, and
  `command_exit_result`
- command/search: `rg -n "runtime_status_label|timer_status_label|state_domain_status_label|failure_severity_label" crates/canic-core/src crates/canic-cli/src crates/canic-host/src crates/canic-control-plane/src -g '*.rs'`
- reachability: active `canic inspect` text rendering and failing-runtime-status
  exit diagnostics
- exact issue: the inspect renderer locally matched runtime DTO enum variants
  into their stable labels even though those labels are runtime-domain
  semantics and are already part of the runtime JSON/Candid contract.

Evidence:
- file: `crates/canic-core/src/dto/runtime.rs`
- line or anchor: `runtime_enums_serialize_canonical_snake_case_labels`
- module/function: runtime DTO serialization tests
- command/search: `rg -n "runtime_enums_serialize_canonical_snake_case_labels" crates/canic-core/src/dto/runtime.rs`
- reachability: active runtime DTO serde/Candid contract tests
- exact issue: DTO tests carried a second literal copy of representative runtime
  enum labels rather than comparing serde output to labels owned by the domain
  enum.

Risk:

Low. Duplicated enum-label matches can drift from the explicit serde labels
used by runtime introspection Candid/JSON payloads, especially when text output
and DTO tests each carry separate copies.

Recommendation:

Move stable runtime enum labels to `domain::runtime` via `label()` methods.
Have inspect rendering and DTO serde-label tests consume those owner-defined
labels.

Regression test:

Pin every runtime enum variant label in domain tests, keep DTO serde/Candid
round-trip tests, run inspect CLI tests, and run the protocol-surface test that
guards active Candid payloads.

Resolution:

- Added `label()` methods to `FailureSeverity`, `RuntimeFieldVisibility`,
  `RuntimeCheckStatus`, `RuntimeDiagnosticSeverity`,
  `RuntimeStateDomainStatus`, `HealthStatus`, `ReadinessStatus`,
  `RuntimeStatus`, and `TimerStatus`.
- Added a domain-runtime test that pins every owner-defined enum label.
- Replaced inspect renderer helpers for runtime status, timer status,
  state-domain status, and failure severity with calls to those enum labels.
- Updated runtime DTO serde-label tests to compare every runtime enum variant's
  serialized label against the domain-owned label.
- Kept inspect text output, runtime JSON labels, runtime Candid labels, command
  behavior, endpoint surfaces, deployment truth, evidence/report schemas, and
  stable-state layout unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted runtime enum label ownership cleanup. |
| `cargo check --locked -p canic-core -p canic-cli` | pass | Checked affected core and CLI packages. |
| `cargo test --locked -p canic-core runtime --lib` | pass | Runtime tests passed, including runtime enum label and DTO serde/Candid checks. |
| `cargo test --locked -p canic-cli inspect` | pass | Inspect CLI tests passed after renderer helpers moved to domain labels. |
| `cargo test --locked -p canic --test protocol_surface` | pass | Protocol-surface Candid tests passed after enum label owner methods were added. |
| `cargo clippy --locked -p canic-core -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for affected packages. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |

## CANIC-083-DEBT-030: Deployment-Truth Status Labels Owned By Text Renderers Instead Of Model Enums

Severity: P3
Category: host / deployment_truth / cli_renderer / text_renderer
Status: fixed
Owner: deployment-truth status labels
Current location: deployment-truth text-renderer helper functions and medic
receipt summaries
Intended owner: deployment-truth model status enums
Affected surfaces: Rust internals only
Release decision: fixed_in_0.83.24

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/execution_preflight.rs`
- line or anchor: `deployment_execution_preflight_status_label`
- module/function: deployment execution preflight text renderer
- command/search: `rg -n "status_label|deployment_execution_preflight_status_label" crates/canic-host/src/deployment_truth/text crates/canic-cli/src/medic/mod.rs -g '*.rs'`
- reachability: active deployment execution preflight text rendering
- exact issue: the execution preflight text renderer locally matched
  `DeploymentExecutionPreflightStatusV1` variants into stable labels instead of
  consuming a model-owned status label.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/mod.rs`
- line or anchor: `safety_status_label`
- module/function: deployment-truth text shared renderer helpers
- command/search: `rg -n "safety_status_label|deployment_execution_status_label|promotion_readiness_status_label|external_lifecycle_plan_status_label|external_upgrade_completion_status_label|verification_requirement_status_label" crates/canic-host/src crates/canic-cli/src -g '*.rs'`
- reachability: active deployment-truth comparison, authority, external
  lifecycle, and promotion text rendering
- exact issue: the shared text renderer locally matched `SafetyStatusV1`
  variants into stable labels instead of consuming a model-owned status label.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/authority/shared/mod.rs`
- line or anchor: `deployment_execution_status_label`
- module/function: authority receipt text helpers
- command/search: same as above
- reachability: active authority receipt/evidence text rendering and medic
  authority receipt summaries
- exact issue: authority text helpers locally matched
  `DeploymentExecutionStatusV1` variants into stable labels, and medic consumed
  the renderer-owned helper for receipt summaries.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/promotion/shared/mod.rs`
- line or anchor: `promotion_readiness_status_label`
- module/function: promotion text shared helpers
- command/search: same as above
- reachability: active promotion provenance, policy, readiness, identity,
  materialization, execution-receipt, and wasm-store text rendering
- exact issue: promotion text helpers locally matched
  `PromotionReadinessStatusV1` variants into stable labels even though the
  status meaning belongs to the promotion report model.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/lifecycle/shared/mod.rs`
- line or anchor:
  `external_lifecycle_plan_status_label`,
  `external_upgrade_completion_status_label`, and
  `verification_requirement_status_label`
- module/function: external lifecycle text shared helpers
- command/search: same as above
- reachability: active external lifecycle plan, completion, check, handoff,
  and verification-policy text rendering
- exact issue: lifecycle text helpers locally matched external lifecycle plan,
  completion, and verification-requirement status enum variants into stable
  labels instead of consuming model-owned labels.

Risk:

Low. These helper functions emitted the same labels as the corresponding serde
`snake_case` deployment-truth status values, but their duplication creates
drift risk between deployment-truth model status semantics, text rendering, and
medic summaries.

Recommendation:

Move stable text labels to the deployment-truth status enums with `label()`
methods. Have text renderers and medic summaries consume those owner-defined
labels instead of maintaining local match blocks.

Regression test:

Pin every affected status enum label in the deployment-truth model modules,
then run deployment-truth host tests and medic tests to confirm text-renderer
and medic consumers still compile and behave from the same model-owned labels.

Resolution:

- Added `label()` methods to `SafetyStatusV1`,
  `DeploymentExecutionPreflightStatusV1`, `DeploymentExecutionStatusV1`,
  `PromotionReadinessStatusV1`, `ExternalLifecyclePlanStatusV1`,
  `ExternalUpgradeCompletionStatusV1`, and
  `ExternalUpgradeVerificationRequirementStatusV1`.
- Added focused model tests that pin the owner-defined labels for each affected
  status enum.
- Removed duplicate status-label helper functions from deployment-truth text
  renderer modules.
- Replaced deployment-truth text renderer and medic summary call sites with
  calls to the model-owned labels.
- Kept operator text output labels, medic text, command behavior, endpoint
  surfaces, Candid, JSON schemas, deployment truth schema, evidence/report
  schemas, and stable-state layout unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted deployment-truth status label ownership cleanup. |
| `cargo check --locked -p canic-host -p canic-cli` | pass | Checked affected host and CLI packages. |
| `cargo test --locked -p canic-host deployment_truth --lib` | pass | Deployment-truth tests passed, including status-label owner tests. |
| `cargo test --locked -p canic-cli medic` | pass | Medic tests passed after receipt summary labels moved to deployment-truth model owners. |
| `cargo clippy --locked -p canic-host -p canic-cli --all-targets -- -D warnings` | pass | Clippy passed for affected packages. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-031: Deployment-Root Verification Text Uses Debug Formatting For Model Labels

Severity: P3
Category: host / deployment_truth / root_verification / text_renderer
Status: fixed
Owner: deployment-root verification text labels
Current location: deployment-root verification report and receipt text
renderers
Intended owner: deployment-truth root-verification and inventory model enums
Affected surfaces: Rust internals only
Release decision: fixed_in_0.83.25

Evidence:
- file: `crates/canic-host/src/deployment_truth/root/report/checks.rs`
- line or anchor: `root_observation_source_label_from_source`
- module/function: `root_verification_evidence_checks` and
  `ensure_root_verification_report_checks_consistent`
- command/search: `rg -n "root_observation_source_label|IcpCanisterStatus|LocalDeploymentState" crates/canic-host/src/deployment_truth/root/report/checks.rs crates/canic-host/src/deployment_truth/text/root_verification crates/canic-host/src/deployment_truth/model/inventory/mod.rs -g '*.rs'`
- reachability: active deployment-root verification evidence-check builder and
  report validation
- exact issue: root verification evidence checks locally matched
  `DeploymentRootObservationSourceV1` variants into labels and carried a
  duplicate expected `IcpCanisterStatus` string in construction and validation
  instead of consuming the inventory model-owned label.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/root_verification/report/mod.rs`
- line or anchor: `format!("evidence_status: {:?}", report.evidence_status)`
  and `format!("state_transition: {:?}", report.state_transition)`
- module/function: `deployment_root_verification_report_text`
- command/search: `rg -n "evidence_status: \\{:\\?\\}|state_transition: \\{:\\?\\}|source_report_.*\\{:\\?\\}|source_root_observation_source: \\{:\\?\\}" crates/canic-host/src/deployment_truth/text/root_verification -g '*.rs'`
- reachability: active deployment-root verification report text rendering
- exact issue: report text depended on enum `Debug` output for root
  verification evidence status and state-transition labels instead of consuming
  model-owned labels.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/root_verification/report/mod.rs`
- line or anchor: `format!("{source:?}")`
- module/function: `deployment_root_verification_report_text`
- command/search: same as above
- reachability: active deployment-root verification report text rendering
- exact issue: observed root observation source text depended on enum `Debug`
  output instead of consuming the inventory model-owned label.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/root_verification/receipt/mod.rs`
- line or anchor: `state_transition`, `previous_root_verification`,
  `new_root_verification`, `source_report_source`,
  `source_report_evidence_status`, `source_report_current_root_verification`,
  `source_report_state_transition`, and `source_root_observation_source`
- module/function: `deployment_root_verification_receipt_text`
- command/search: same as above
- reachability: active deployment-root verification receipt text rendering
- exact issue: receipt text depended on enum `Debug` output for root
  verification source, evidence status, state transition, root verification
  state, and root observation source labels.

Risk:

Low. The current labels are already operator text output, but using `Debug`
ties that output to Rust variant names and makes the text contract implicit.
That creates drift risk if variants are renamed or if future model/report
changes need different text labels.

Recommendation:

Move the exact current labels onto the model enums with `label()` methods, then
have root-verification report and receipt text renderers consume those
owner-defined labels. Preserve the current CamelCase labels to avoid an
operator text output change inside this no-behavior-change slice.

Regression test:

Pin every affected model-owned label in deployment-truth model tests, then run
root-verification and deployment-truth host tests to confirm report/receipt
text behavior remains covered.

Resolution:

- Added `label()` to `DeploymentRootVerificationSourceV1`,
  `DeploymentRootVerificationEvidenceStatusV1`,
  `DeploymentRootVerificationStateTransitionV1`,
  `DeploymentRootVerificationStateV1`, and
  `DeploymentRootObservationSourceV1`.
- Added focused model tests that pin the exact labels previously emitted by
  `Debug` formatting.
- Replaced deployment-root verification report and receipt text `Debug`
  formatting for those model labels with calls to model-owned labels.
- Replaced the duplicate root-observation-source label match and expected
  source string in root verification evidence-check construction and validation
  with the inventory model-owned label.
- Kept operator text output labels, command behavior, endpoint surfaces,
  Candid, JSON schemas, deployment truth schema, evidence/report schemas, and
  stable-state layout unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted deployment-root verification label ownership cleanup. |
| `cargo check --locked -p canic-host` | pass | Checked the affected host package. |
| `cargo test --locked -p canic-host root_verification --lib` | pass | 60 focused root-verification tests passed, including the new label-owner tests. |
| `cargo test --locked -p canic-host deployment_truth --lib` | pass | Deployment-truth tests passed after replacing root-verification `Debug` labels. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for the affected host package. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-032: Deployment-Truth Control-Class Labels Are Duplicated Outside Inventory Model

Severity: P3
Category: host / deployment_truth / model_labels / text_renderer / report_builder
Status: fixed
Owner: deployment-truth control-class labels
Current location: deployment-truth report, diff, lifecycle text, and
external-upgrade verification helpers
Intended owner: `CanisterControlClassV1`
Affected surfaces: Rust internals only
Release decision: fixed_in_0.83.26

Evidence:
- file: `crates/canic-host/src/deployment_truth/report/canisters.rs`
- line or anchor: `planned_canister_evidence_label` and
  `record_unsafe_canister_control_class`
- module/function: canister report diff construction
- command/search:
  `rg -n "control_class=\\{:\\?\\}|control=\\{:\\?\\}|Some\\(\"DeploymentControlled\"\\.to_string\\(\\)\\)" crates/canic-host/src/deployment_truth -g '*.rs'`
- reachability: active deployment-truth canister diff construction and safety
  finding generation
- exact issue: planned-canister evidence labels formatted
  `CanisterControlClassV1` with `Debug`, and unsafe canister control-class
  diffs carried a duplicate expected `DeploymentControlled` label literal
  instead of consuming an inventory model-owned label.

Evidence:
- file: `crates/canic-host/src/deployment_truth/report/pools.rs`
- line or anchor: `record_unsafe_pool_control_class`
- module/function: pool report diff construction
- command/search:
  `rg -n "Some\\(\"CanicManagedPool\"\\.to_string\\(\\)|format!\\(\\\"\\{:\\?\\}\\\", observed\\.control_class\\)" crates/canic-host/src/deployment_truth -g '*.rs'`
- reachability: active deployment-truth pool diff construction and safety
  finding generation
- exact issue: unsafe pool control-class diffs carried a duplicate expected
  `CanicManagedPool` label literal and formatted observed control classes with
  `Debug`.

Evidence:
- file: `crates/canic-host/src/deployment_truth/multi/diff.rs`
- line or anchor: `control_class_counts`
- module/function: multi-deployment inventory summary construction
- command/search:
  `rg -n "format!\\(\\\"\\{:\\?\\}\\\", canister\\.control_class\\)" crates/canic-host/src/deployment_truth -g '*.rs'`
- reachability: active multi-deployment diff summary construction
- exact issue: inventory summary grouping depended on enum `Debug` output for
  control-class labels.

Evidence:
- file:
  `crates/canic-host/src/deployment_truth/lifecycle/external_upgrade/verification/shared/mod.rs`
- line or anchor: `control_class_value`
- module/function: external-upgrade verification summary helper
- command/search:
  `rg -n "format!\\(\\\"\\{control_class:\\?\\}\\\"" crates/canic-host/src/deployment_truth -g '*.rs'`
- reachability: active external-upgrade verification summary construction
- exact issue: verification summaries depended on enum `Debug` output for
  control-class labels.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/lifecycle/shared/mod.rs`
- line or anchor: `control_class={:?}`
- module/function:
  `append_external_lifecycle_role_items` and
  `append_lifecycle_authority_items`
- command/search:
  `rg -n "control_class=\\{:\\?\\}" crates/canic-host/src/deployment_truth -g '*.rs'`
- reachability: active external lifecycle and lifecycle authority text
  rendering
- exact issue: lifecycle text output depended on enum `Debug` output for
  control-class labels.

Risk:

Low. The current labels are operator/report text and diff values, but using
`Debug` ties those labels to Rust variant names and duplicating expected labels
creates drift risk between inventory model semantics, report construction, and
text rendering.

Recommendation:

Move the exact current labels onto `CanisterControlClassV1` with a `label()`
method. Have report builders, diff summaries, lifecycle text renderers, and
external-upgrade verification helpers consume that owner-defined label instead
of formatting variants with `Debug` or duplicating literals.

Regression test:

Pin every `CanisterControlClassV1` label in the inventory model tests, then run
deployment-truth host tests to confirm report, diff, lifecycle, and
verification consumers compile and preserve existing output labels.

Resolution:

- Added `CanisterControlClassV1::label()` with the exact labels previously
  emitted by `Debug` formatting and duplicate literals.
- Added a focused inventory model test that pins every control-class label.
- Replaced control-class `Debug` formatting in canister evidence labels,
  multi-deployment inventory summaries, external lifecycle text rendering, and
  external-upgrade verification summaries with calls to the model-owned labels.
- Replaced duplicate expected `DeploymentControlled` and `CanicManagedPool`
  diff labels with `CanisterControlClassV1` owner-defined labels.
- Kept operator text output labels, diff values, command behavior, endpoint
  surfaces, Candid, JSON schemas, deployment truth schema, evidence/report
  schemas, and stable-state layout unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted deployment-truth control-class label ownership cleanup. |
| `cargo check --locked -p canic-host` | pass | Checked the affected host package. |
| `cargo test --locked -p canic-host deployment_truth --lib` | pass | 448 deployment-truth tests passed, including the new control-class label-owner test. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for the affected host package. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-033: External Lifecycle Text Owns Model Enum Labels

Severity: P3
Category: host / deployment_truth / lifecycle / model_labels / text_renderer
Status: fixed
Owner: deployment-truth external lifecycle labels
Current location: deployment-truth external lifecycle text shared helpers
Intended owner: deployment-truth external lifecycle model enums
Affected surfaces: Rust internals only
Release decision: fixed_in_0.83.26

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/lifecycle/shared/mod.rs`
- line or anchor:
  `lifecycle_mode_label`, `consent_channel_label`,
  `consent_subject_label`, and `verification_requirement_label`
- module/function: external lifecycle shared text renderer helpers
- command/search:
  `rg -n "lifecycle_mode_label|consent_channel_label|consent_subject_label|verification_requirement_label" crates/canic-host/src/deployment_truth -g '*.rs'`
- reachability: active external lifecycle role, authority, proposal, pending,
  handoff, and verification requirement text rendering
- exact issue: lifecycle text helpers locally matched `LifecycleModeV1`,
  `ConsentChannelKindV1`, `ConsentSubjectKindV1`, and
  `LifecycleVerificationRequirementV1` variants into stable text labels instead
  of consuming model-owned labels.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/lifecycle/shared/mod.rs`
- line or anchor:
  `external_upgrade_consent_state_label`,
  `external_upgrade_verification_result_label`, and
  `external_verification_observation_source_label`
- module/function: external lifecycle shared text renderer helpers
- command/search:
  `rg -n "external_upgrade_consent_state_label|external_upgrade_verification_result_label|external_verification_observation_source_label" crates/canic-host/src/deployment_truth -g '*.rs'`
- reachability: active external lifecycle consent evidence, receipt,
  completion, verification report, verification check, and verification policy
  text rendering
- exact issue: lifecycle text helpers locally matched
  `ExternalUpgradeConsentStateV1`, `ExternalUpgradeVerificationResultV1`, and
  `ExternalVerificationObservationSourceV1` variants into stable text labels
  instead of consuming model-owned labels.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/lifecycle/verification/mod.rs`
- line or anchor: `observation.observed_control_class`
- module/function: `external_upgrade_verification_check_text`
- command/search:
  `rg -n "observed_control_class.*format!\\(\\\"\\{value:\\?\\}\\\"|format!\\(\\\"\\{value:\\?\\}\\\"" crates/canic-host/src/deployment_truth -g '*.rs'`
- reachability: active external-upgrade verification check text rendering
- exact issue: observed control-class text depended on enum `Debug` output
  instead of consuming the inventory model-owned label.

Risk:

Low. These labels were already stable operator text, but text-owned label
helpers duplicate lifecycle model semantics and can drift from JSON/report
models if variants are renamed or new variants are added.

Recommendation:

Move the exact current external lifecycle labels onto the lifecycle model enums
with `label()` methods. Have lifecycle text renderers consume those
owner-defined labels instead of maintaining local helper matches.

Regression test:

Pin every affected external lifecycle enum label in the lifecycle model tests,
then run deployment-truth host tests to confirm lifecycle text consumers
preserve existing output labels.

Resolution:

- Added `label()` methods to `LifecycleModeV1`,
  `LifecycleVerificationRequirementV1`, `ConsentSubjectKindV1`,
  `ConsentChannelKindV1`, `ExternalUpgradeConsentStateV1`,
  `ExternalUpgradeVerificationResultV1`, and
  `ExternalVerificationObservationSourceV1`.
- Added focused lifecycle model tests that pin every moved text label.
- Removed duplicate lifecycle label helper functions from the shared lifecycle
  text renderer.
- Replaced external lifecycle role/proposal/handoff, consent-evidence, receipt,
  completion, verification-policy, and verification-check text call sites with
  calls to model-owned labels.
- Replaced observed control-class `Debug` formatting in external-upgrade
  verification check text with the inventory model-owned label.
- Kept operator text output labels, command behavior, endpoint surfaces,
  Candid, JSON schemas, deployment truth schema, evidence/report schemas, and
  stable-state layout unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted deployment-truth lifecycle label ownership cleanup. |
| `cargo check --locked -p canic-host` | pass | Checked the affected host package. |
| `cargo test --locked -p canic-host deployment_truth --lib` | pass | Deployment-truth tests passed, including the new lifecycle label-owner tests. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for the affected host package. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-034: Promotion Text And Identity Keys Use Debug Formatting For Model Labels

Severity: P3
Category: host / deployment_truth / promotion / model_labels / text_renderer
Status: fixed
Owner: deployment-truth promotion artifact and policy labels
Current location: deployment-truth promotion text renderers and identity-key
helpers
Intended owner: deployment-truth promotion, artifact, inventory, and execution
model enums
Affected surfaces: Rust internals only
Release decision: fixed_in_0.83.27

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/promotion/shared/mod.rs`
- line or anchor:
  `append_promotion_role_items`,
  `append_promotion_artifact_identity_role_items`,
  `append_promotion_artifact_identity_group_items`,
  `append_promotion_policy_decision_items`, and
  `append_promotion_transform_role_items`
- module/function: promotion shared text rendering
- command/search:
  `rg -n "\\{:\\?\\}|\\{[A-Za-z0-9_]+:\\?\\}" crates/canic-host/src/deployment_truth/text/promotion -g '*.rs'`
- reachability: active promotion readiness, identity, policy, and transform
  text rendering
- exact issue: promotion text renderers formatted promotion artifact level,
  role artifact source kind, promotion artifact identity kind, promotion policy
  requirement, promotion policy claim, and artifact source labels with enum
  `Debug` instead of consuming model-owned labels.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/promotion/provenance/mod.rs`
- line or anchor: role row formatter
- module/function: `artifact_promotion_provenance_report_text`
- command/search: same as above
- reachability: active artifact promotion provenance text rendering
- exact issue: provenance role rows formatted promotion artifact level and
  source kind with enum `Debug`.

Evidence:
- file:
  `crates/canic-host/src/deployment_truth/text/promotion/execution_receipt/mod.rs`
- line or anchor: role row formatter
- module/function: `artifact_promotion_execution_receipt_text`
- command/search: same as above
- reachability: active artifact promotion execution receipt text rendering
- exact issue: receipt role rows formatted promotion artifact level and role
  phase result labels with enum `Debug`.

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/promotion/wasm_store/mod.rs`
- line or anchor: `postcondition={:?}`
- module/function: `promotion_wasm_store_identity_report_text`
- command/search: same as above
- reachability: active promotion wasm-store identity text rendering
- exact issue: wasm-store identity rows formatted observation status labels
  with enum `Debug`.

Evidence:
- file: `crates/canic-host/src/deployment_truth/promotion/identity/group.rs`
- line or anchor:
  `artifact_identity_key_for_role` and `source_kind_identity_part`
- module/function: promotion artifact identity key construction
- command/search:
  `rg -n "source_kind=\\{:\\?\\}|format!\\(\\\"\\{kind:\\?\\}\\\"" crates/canic-host/src/deployment_truth/promotion -g '*.rs'`
- reachability: active promotion artifact identity group/key construction
- exact issue: promotion identity keys formatted role artifact source kind with
  enum `Debug` instead of consuming the source-kind model-owned label.

Risk:

Low. The current labels are operator text and report identity-key strings, but
using `Debug` ties them to Rust variant names and duplicates label semantics
outside the model owners. This creates drift risk if variants are renamed or
if text/report identity construction changes independently.

Recommendation:

Move the exact current labels onto the model enums with `label()` methods.
Have promotion text renderers and identity-key helpers consume those
owner-defined labels. Leave execution/status labels with existing mixed
snake-case and `Debug` conventions for a separate slice.

Regression test:

Pin every affected model-owned label in model tests, then run promotion and
deployment-truth host tests to confirm text rendering and identity key
construction preserve existing strings.

Resolution:

- Added exact text labels to `PromotionArtifactLevelV1`,
  `RoleArtifactSourceKindV1`, `PreviousArtifactReceiptKindV1`,
  `ArtifactTransportV1`, `PromotionArtifactIdentityKindV1`,
  `PromotionPolicyRequirementV1`, `PromotionPolicyClaimV1`,
  `ArtifactSourceV1`, `ObservationStatusV1`, and `RolePhaseResultV1`.
- Added focused model tests that pin every moved label.
- Replaced promotion readiness, identity, policy, transform, provenance,
  execution-receipt, and wasm-store text-renderer `Debug` formatting for those
  enums with calls to owner-defined labels.
- Replaced promotion identity-key source-kind `Debug` formatting with the
  source-kind model-owned label.
- Kept operator text output labels, identity-key strings, command behavior,
  endpoint surfaces, Candid, JSON schemas, deployment truth schema,
  evidence/report schemas, and stable-state layout unchanged.

Post-v0.83.27 correction:

- `CANIC-083-DEBT-036` completed staging transport/postcondition label
  consumption and removed the unused public
  `PreviousArtifactReceiptKindV1::label()` method that had no production
  consumer. Historical 0.83.27 output remains unchanged.

Fix validation:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | pass | Formatted promotion label ownership cleanup. |
| `cargo check --locked -p canic-host` | pass | Checked the affected host package. |
| `cargo test --locked -p canic-host promotion --lib` | pass | 173 focused promotion tests passed, including new promotion label-owner tests. |
| `cargo test --locked -p canic-host deployment_truth --lib` | pass | 458 deployment-truth tests passed, including model label-owner tests. |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | Clippy passed for the affected host package. |
| `cargo test --locked -p canic --test changelog_governance` | pass | Changelog governance test passed. |
| `cargo fmt --all -- --check` | pass | Format check passed after implementation. |
| `git diff --check` | pass | Whitespace diff check passed. |

## CANIC-083-DEBT-035: Receipt Duplicate Detection Uses Lossy Display Keys

Severity: P2
Category: host / deployment_truth / receipt_resume / safety
Status: fixed
Owner: deployment-truth receipt-resume safety comparison
Current location: receipt duplicate evidence grouping
Intended owner: typed phase and role-phase receipt evidence values
Affected surfaces: cli, json, evidence_report, internal_only
Release decision: fixed_in_0.83.28

Evidence:
- file: `crates/canic-host/src/deployment_truth/report/receipt_resume.rs`
- line or anchor: `receipt_phase_evidence_label` and
  `role_phase_evidence_label`
- module/function: receipt-resume duplicate validation
- command/search:
  `rg -n "status=\{:\?\}|result=\{:\?\}|evidence.join" crates/canic-host/src/deployment_truth/report/receipt_resume.rs`
- reachability: active passive `canic deploy inspect resume-report` safety
  reporting for persisted or explicitly supplied deployment receipts
- exact issue: duplicate detection compared delimiter-joined display strings.
  Distinct evidence arrays such as `["a,b"]` and `["a", "b"]`, or distinct
  role fields containing `;target=`, could produce the same comparison key and
  be reported as identical duplicates.

Evidence:
- file: `crates/canic-host/src/deployment_truth/report/mod.rs`
- line or anchor: `duplicate_evidence_groups`
- module/function: duplicate evidence grouping
- command/search: inspection of the `BTreeSet<String>` conflict decision
- reachability: active deployment-truth report producer
- exact issue: conflict status was derived from the number of formatted string
  values rather than structural evidence identities.

Risk:

Moderate. The current resume report is passive, but it is a safety report. A
collision could downgrade structurally conflicting receipt evidence from a
hard failure to an identical-duplicate warning and list the phase as
resumable.

Recommendation:

Compare typed evidence keys for conflict decisions and retain formatted labels
only for diagnostics. Preserve the current report schema and normal diagnostic
strings.

Regression test:

Cover phase evidence arrays and role-phase fields that produce identical old
display strings but contain different typed values.

Resolution:

- Added typed phase and role-phase evidence keys for duplicate conflict
  decisions.
- Added a keyed grouping helper that separates structural comparison from
  diagnostic rendering.
- Kept existing receipt/report schemas and normal diagnostic strings.
- Added collision regression coverage for comma-joined phase evidence and
  delimiter-bearing role-phase fields.

## CANIC-083-DEBT-036: Promotion Execution And Staging Labels Remain Partially Owned

Severity: P3
Category: host / deployment_truth / promotion / receipt / model_labels
Status: fixed
Owner: deployment-truth execution, promotion, and staging labels
Current location: promotion text renderers, staging evidence builder, and
promotion source model
Intended owner: deployment-truth execution, promotion, inventory, and artifact
transport enums
Affected surfaces: cli, deployment_truth, evidence_report, internal_only
Release decision: fixed_in_0.83.28

Evidence:
- file: `crates/canic-host/src/deployment_truth/text/promotion/`
- line or anchor: execution receipt and target-lineage/plan text renderers
- module/function: promotion execution/status text rendering
- command/search:
  `rg -n "\{:\?\}|\{[A-Za-z0-9_]+:\?\}" crates/canic-host/src/deployment_truth/text/promotion -g '*.rs'`
- reachability: active promotion execution receipt, artifact plan, and target
  execution lineage text
- exact issue: operation status, command result, executor backend, preflight
  status, and plan status still used enum `Debug`, despite the 0.83.27 notes
  reserving them for a separate label-ownership slice.

Evidence:
- file: `crates/canic-host/src/deployment_truth/receipt/deployment.rs`
- line or anchor: `staging_receipt_evidence`
- module/function: staging receipt evidence conversion
- command/search: `rg -n "staging_(transport|postcondition).*\?:"`
- reachability: active persisted deployment receipt evidence
- exact issue: artifact transport and observation status had model labels but
  the staging evidence consumer still used `Debug`.

Evidence:
- file:
  `crates/canic-host/src/deployment_truth/model/promotion/source/mod.rs`
- line or anchor: `PreviousArtifactReceiptKindV1::label`
- module/function: promotion source model
- command/search: production-use search for the method
- reachability: public Rust API with test-only use
- exact issue: 0.83.27 added a public label method without a production label
  consumer, expanding the Rust API outside the evidence-backed requirement.

Risk:

Low. Output was stable, but `Debug` formatting left label ownership coupled to
Rust variant names, while the unused method broadened the public host API
without an active contract.

Recommendation:

Move the exact existing promotion execution/status strings to model-owned
variant labels, consume existing transport/status labels in staging evidence,
and hard-cut the unused previous-receipt-kind label method.

Regression test:

Pin all new variant labels, promotion text consumers, and unchanged staging
evidence strings.

Resolution:

- Added model-owned variant labels for execution preflight status, execution
  status, command result, executor backend, and promotion readiness status.
- Replaced the remaining promotion text-renderer `Debug` formatting with those
  labels while preserving output exactly.
- Routed staging transport and postcondition evidence through existing
  model-owned labels.
- Removed the unused public `PreviousArtifactReceiptKindV1::label()` method as
  a pre-1.0 Rust API hard cut; the enum and its serialized shape are unchanged.

## CANIC-083-DEBT-037: Audit Metadata And Closeout State Are Stale

Severity: P3
Category: docs_drift / audit_governance
Status: fixed
Owner: 0.83 technical-debt audit artifacts and session handoff
Current location: ledger header/scope, recommended slices, and current status
Intended owner: canonical 0.83 ledger and compact handoff
Affected surfaces: docs
Release decision: fixed_in_0.83.28

Evidence:
- file: `docs/audits/0.83-technical-debt/ledger.md`
- line or anchor: header and scope
- module/function: audit metadata
- command/search: inspection against `git describe`, current package version,
  and finding/recommended-slice statuses
- reachability: active audit source of truth
- exact issue: the ledger referenced package surface 0.83.24, stopped its scope
  summary before the latest findings, and remained `pass_with_followups` after
  every recorded recommended slice was complete.

Evidence:
- file: `docs/status/current.md`
- line or anchor: Current Line
- module/function: compact session handoff
- command/search: inspection against the released `v0.83.27` tag
- reachability: mandatory new-session handoff
- exact issue: the handoff described 0.83.27 as a working slice and reported
  package surface 0.83.26.

Risk:

Low. Runtime behavior is unaffected, but stale audit state can cause the next
session to repeat completed work or miss a declared follow-up.

Recommendation:

Update the repo reference, package surface, scope summary, recommended slices,
and handoff together. Mark the ledger `pass` only after all new findings are
fixed and no deferred findings remain.

Regression test:

Review finding/recommended-slice status consistency and run whitespace plus
changelog-governance checks.

Resolution:

- Updated the ledger metadata and scope through the post-v0.83.27 closeout
  findings.
- Added completed recommended slices for `CANIC-083-DEBT-035` through
  `CANIC-083-DEBT-037`.
- Updated the handoff to package surface 0.83.27 and the 0.83.28
  release-preparation batch.
- Marked the ledger `pass`; no open or deferred 0.83 findings remain.

Closeout validation:

| Command | Result | Finding coverage |
| --- | --- | --- |
| `cargo fmt --all` | pass | CANIC-083-DEBT-035 through CANIC-083-DEBT-037 |
| `cargo test --locked -p canic-host deployment_truth::tests::execution_receipts::resume --lib` | pass; 10 tests | CANIC-083-DEBT-035 |
| `cargo test --locked -p canic-host promotion --lib` | pass; 172 tests | CANIC-083-DEBT-036 |
| `cargo test --locked -p canic-host deployment_truth --lib` | pass; 461 tests | CANIC-083-DEBT-035, CANIC-083-DEBT-036 |
| `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | pass | CANIC-083-DEBT-035, CANIC-083-DEBT-036 |
| `cargo test --locked -p canic --test changelog_governance` | pass | CANIC-083-DEBT-037 |
| `cargo fmt --all -- --check` | pass | CANIC-083-DEBT-035 through CANIC-083-DEBT-037 |
| `git diff --check` | pass | CANIC-083-DEBT-035 through CANIC-083-DEBT-037 |

## Closeout

The 0.83 ledger is `pass`. All 37 findings are fixed, no findings are deferred,
and every recommended slice is complete. The closeout recommendation is to
release 0.83.28 through the human-owned release flow or proceed to the next
feature line without another broad 0.83 refactor.

## Rejected / Non-Findings

See `rejected.md`.

## Deferred

See `deferred.md`.

## Recommended Slices

See `recommended-slices.md`.
