# Canic 0.83 Recommended Debt Slices

## 0.83 JSON Output Convention Hardening

Status:
completed in 0.83.0 for the accepted `CANIC-083-DEBT-001` scope.

Source findings:
- CANIC-083-DEBT-001

Boundary:
CLI operator report output selection.

Current owner:
Individual CLI command families own their own output flag convention.

Intended owner:
Each report family keeps parser ownership, but the repo has an explicit
operator-report convention explaining when `--json` is used and when a
multi-value `--format` enum is intentionally retained.

Behavior impact label:
behavior_change_declared.

Public surfaces affected:
CLI.

Serialized surfaces affected:
None expected unless a follow-up changes output schemas. The finding is about
flag convention, not JSON payload shape.

Validation:
- `cargo test --locked -p canic-cli state`
- `cargo test --locked -p canic-cli medic`
- `cargo test --locked -p canic-cli inspect`
- `cargo test --locked -p canic-cli deploy`
- `cargo test --locked -p canic-cli evidence`
- targeted help/parser tests for any changed family

Explicit non-scope:
- no schema changes
- no evidence envelope changes
- no deployment truth changes
- no aliases or compatibility routes for removed `--format` report-selection
  forms

## 0.83 Advanced Deploy Output Convention Hardening

Status:
completed in 0.83.1 for the accepted `CANIC-083-DEBT-002` scope.

Source findings:
- CANIC-083-DEBT-002

Boundary:
Default-JSON advanced deploy report and request-inspection output selection.

Previous owner:
Each advanced deploy subfamily owned a local `--format json|text` parser.

Intended owner:
Each advanced deploy report family keeps parser ownership, but the repo has an
explicit convention for default-JSON report tools that need optional human text
output.

Behavior impact label:
behavior_change_declared if parser surfaces change.

Public surfaces affected:
CLI.

Serialized surfaces affected:
None expected if only output-selection flags change.

Validation:
- `cargo test --locked -p canic-cli deploy`
- focused parser/help tests for deploy compare, root, authority, external, and
  promote report families

Explicit non-scope:
- no schema changes
- no evidence envelope changes
- no deployment truth changes
- no aliases or compatibility routes for removed `--format` forms

## 0.83 Runtime Inspect Report Typing

Status:
completed in 0.83.2 for the accepted `CANIC-083-DEBT-004` scope.

Source findings:
- CANIC-083-DEBT-004

Boundary:
Runtime inspect CLI report wrapper, typed command/endpoint labels, typed status
slots, and source-attribution labels.

Previous owner:
The inspect report wrapper stored command, endpoint, aggregate status, source
attribution, and response format as raw strings, and dormant health/readiness
slots as loose JSON values.

Intended owner:
The inspect report wrapper owns typed report values, and the renderer only
formats or serializes those typed values.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust replay-policy manifest type API only. No CLI or endpoint surface changes.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-cli inspect`

Explicit non-scope:
- no endpoint changes
- no Candid changes
- no JSON field changes
- no command changes
- no broader runtime introspection behavior changes

## 0.83 Auth Renewal Report Typing

Status:
completed in 0.83.3 for the accepted `CANIC-083-DEBT-005` scope.

Source findings:
- CANIC-083-DEBT-005

Boundary:
Auth renewal CLI report wrapper, typed report kind/source/status labels, and
decoded canister response status strings.

Previous owner:
The auth renewal report wrapper stored report kind, local Candid source, and
aggregate renewal status as raw strings alongside decoded canister response
strings.

Intended owner:
The auth renewal report wrapper owns typed CLI labels; decoded canister
response fields remain response data.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-cli auth`

Explicit non-scope:
- no endpoint changes
- no Candid changes
- no JSON field changes
- no command changes
- no auth behavior changes

## 0.83 Metrics Report Status Typing

Status:
completed in 0.83.4 for the accepted `CANIC-083-DEBT-006` scope.

Source findings:
- CANIC-083-DEBT-006

Boundary:
Metrics CLI report wrapper canister-row status labels.

Previous owner:
Metrics transport builders stored row status labels as raw strings.

Intended owner:
The metrics report model owns typed canister-row statuses, and the renderer
formats or serializes those typed values.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-cli metrics`

Explicit non-scope:
- no endpoint changes
- no Candid changes
- no JSON field changes
- no command changes
- no metrics behavior changes

## 0.83 Cycles Report Status Typing

Status:
completed in 0.83.4 for the accepted `CANIC-083-DEBT-007` scope.

Source findings:
- CANIC-083-DEBT-007

Boundary:
Cycles CLI report wrapper canister-row status and coverage labels.

Previous owner:
Cycles transport builders and summarizers stored row status and coverage
labels as raw strings.

Intended owner:
The cycles report model owns typed canister-row and coverage statuses, and the
renderer formats or serializes those typed values.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-cli cycles`

Explicit non-scope:
- no endpoint changes
- no Candid changes
- no JSON field changes
- no command changes
- no cycles behavior changes

## 0.83 Blob-Storage Report Label Typing

Status:
completed in 0.83.5 for the accepted `CANIC-083-DEBT-008` scope.

Source findings:
- CANIC-083-DEBT-008

Boundary:
Blob-storage CLI report wrapper kind, source, action, funding, and readiness
labels.

Previous owner:
Blob-storage report builders stored closed report labels as raw strings.

Intended owner:
The blob-storage report model owns typed labels, and renderers/error
boundaries format or serialize those typed values.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-cli blob_storage`

Explicit non-scope:
- no endpoint changes
- no Candid changes
- no JSON field changes
- no command changes
- no blob-storage behavior changes
- no typing of free-form command strings or error messages

## 0.83 Backup Report Status Typing

Status:
completed in 0.83.6 for the accepted `CANIC-083-DEBT-009` scope.

Source findings:
- CANIC-083-DEBT-009

Boundary:
Backup CLI report wrapper mode, layout, status, and action labels.

Previous owner:
Backup report builders stored closed create/list/prune/status/inspect labels
as raw strings.

Intended owner:
The backup report model owns typed labels, and renderers/JSON boundaries
format or serialize those typed values.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-cli backup`

Explicit non-scope:
- no endpoint changes
- no Candid changes
- no JSON field changes
- no command changes
- no backup behavior changes
- no typing of dynamic backup scope, paths, operation kind/state, or error text

## 0.83 Wallet Command Parser Typing

Status:
completed in 0.83.7 for the accepted `CANIC-083-DEBT-010` scope.

Source findings:
- CANIC-083-DEBT-010

Boundary:
Token and cycles wallet CLI command parsers and dispatchers.

Previous owner:
The token parser stored the closed `balance`/`transfer` command kind as a raw
string, and the cycles wallet dispatcher routed maintained subcommands through
raw command strings and separate command-name constants.

Intended owner:
Wallet parsers own typed command kinds for maintained command variants;
caller-provided token symbols, pending-operation command strings, and
delegated ICP CLI command/error text remain strings.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None.

Validation:
- `cargo test --locked -p canic-cli token`
- `cargo test --locked -p canic-cli cycles`

Explicit non-scope:
- no command changes
- no help text changes
- no ICP CLI execution changes
- no token symbol typing
- no receiver or deployment-target behavior changes
- no pending-operation log schema changes

## 0.83 Blob-Storage Method Mode Typing

Status:
completed in 0.83.8 for the accepted `CANIC-083-DEBT-011` scope.

Source findings:
- CANIC-083-DEBT-011

Boundary:
Blob-storage CLI action report method-mode labels.

Previous owner:
Target resolution derived the typed call mode to a raw `query`/`update` string
before action report construction, and `BlobStorageAction.mode` stored that
label as a string.

Intended owner:
The blob-storage report model owns the typed action method mode, target
resolution supplies the typed value, and renderers/JSON serialization format
the label at the output boundary.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-cli blob_storage`

Explicit non-scope:
- no endpoint changes
- no Candid changes
- no JSON field changes
- no command changes
- no blob-storage behavior changes
- no typing of decoded canister response labels, next-action guidance strings,
  delegated command strings, or error text

## 0.83 Replica Status Source Typing

Status:
completed in 0.83.9 for the accepted `CANIC-083-DEBT-012` scope.

Source findings:
- CANIC-083-DEBT-012

Boundary:
Replica status JSON report source labels.

Previous owner:
`canic replica status --json` stored the closed `status_source` labels as
string literals inside the status builder.

Intended owner:
The replica status report model owns typed status-source values, and JSON
serialization formats the stable source labels.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-cli replica`

Explicit non-scope:
- no command changes
- no help text changes
- no ICP CLI execution changes
- no local replica probing behavior changes
- no typing of delegated ICP command/error text or embedded status payloads

## 0.83 Deploy-Plan Preview Label Typing

Status:
completed in 0.83.9 for the accepted `CANIC-083-DEBT-013` scope.

Source findings:
- CANIC-083-DEBT-013

Boundary:
Deploy-plan future-apply preview phase, operation, and status labels.

Previous owner:
`canic deploy plan` stored proposed-operation phase, label, and status values
as raw string constants in the report builder.

Intended owner:
The deploy-plan report model owns typed future-apply preview labels, and
text/JSON serialization formats the stable labels.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-cli deploy_plan`

Explicit non-scope:
- no command changes
- no help text changes
- no endpoint changes
- no Candid changes
- no JSON field changes
- no deployment truth or evidence schema changes
- no typing of diagnostic codes, subjects, details, next actions, or embedded
  `DeploymentPlanV1` data

## 0.83 Deploy-Plan Diagnostic Label Typing

Status:
completed in 0.83.10 for the accepted `CANIC-083-DEBT-014` scope.

Source findings:
- CANIC-083-DEBT-014

Boundary:
Deploy-plan diagnostic category, severity, and source labels.

Previous owner:
`canic deploy plan` stored diagnostic category, severity, and source values as
raw string constants in the report builder.

Intended owner:
The deploy-plan report model owns typed diagnostic labels, and text/JSON
serialization formats the stable labels.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-cli deploy_plan`

Explicit non-scope:
- no command changes
- no help text changes
- no endpoint changes
- no Candid changes
- no JSON field changes
- no deployment truth or evidence schema changes
- no typing of diagnostic codes, subjects, details, next actions, or embedded
  `DeploymentPlanV1` data

## 0.83 State-Audit Report Label Typing

Status:
completed in 0.83.11 for the accepted `CANIC-083-DEBT-015` scope.

Source findings:
- CANIC-083-DEBT-015

Boundary:
State-audit report scope, category, and source labels.

Previous owner:
`canic state audit` stored report scope and check category/source labels as
raw string constants in the state-manifest audit builder.

Intended owner:
The state-audit report model owns typed scope, category, and source labels,
and text/JSON serialization formats the stable labels.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels remain unchanged.

Validation:
- `cargo test --locked -p canic-host state_manifest --lib`
- `cargo test --locked -p canic-cli state`

Explicit non-scope:
- no command changes
- no help text changes
- no endpoint changes
- no Candid changes
- no JSON field changes
- no deployment truth or evidence schema changes
- no typing of audit codes, subjects, details, next actions, command strings,
  or embedded manifest data

## 0.83 Deployment-Root Verification Check Name Typing

Status:
completed in 0.83.11 for the accepted `CANIC-083-DEBT-016` scope.

Source findings:
- CANIC-083-DEBT-016

Boundary:
Deployment-root verification report check-row names.

Previous owner:
`canic deploy inspect root` and `canic deploy root verify` report paths
repeated the closed identity/evidence check-row names as raw strings in the
host report builder and validator.

Intended owner:
The deployment-root verification report builder owns typed check-name labels,
and serialization writes the stable string names at the
`DeploymentRootVerificationCheckV1` boundary.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None. JSON labels and report digest semantics remain unchanged.

Validation:
- `cargo test --locked -p canic-host root_verification --lib`
- `cargo test --locked -p canic-cli deploy_root`

Explicit non-scope:
- no command changes
- no help text changes
- no endpoint changes
- no Candid changes
- no JSON field changes
- no deployment truth schema changes
- no evidence-envelope changes
- no mutation behavior changes

## 0.83 Replay-Policy Command-Kind Label Typing

Status:
completed in 0.83.12 for the accepted `CANIC-083-DEBT-017` scope.

Source findings:
- CANIC-083-DEBT-017

Boundary:
Replay-policy manifest command-kind labels.

Previous owner:
Replay-policy manifest rows stored command-kind labels as raw `&'static str`
fields, while runtime replay storage used the validated
`model::replay::CommandKind` type.

Intended owner:
Replay-policy manifests own static command-kind labels through a typed manifest
value. Runtime replay storage and workflow guards continue to use
`model::replay::CommandKind`.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None.

Validation:
- `cargo test --locked -p canic-core replay_policy --lib`
- `cargo clippy --locked -p canic-core --all-targets -- -D warnings`

Explicit non-scope:
- no endpoint changes
- no command changes
- no Candid changes
- no JSON field changes
- no deployment truth or evidence schema changes
- no stable-state layout changes
- no changes to runtime replay `CommandKind`, receipt storage, operation IDs,
  cost guards, or workflow replay descriptors

## 0.83 Replay-Policy Manifest Constructor Label Typing

Status:
completed in 0.83.13 for the accepted `CANIC-083-DEBT-018` scope.

Source findings:
- CANIC-083-DEBT-018

Boundary:
Replay-policy manifest constructor command-kind, command-manifest,
quota-policy, and cycle-reserve inputs.

Previous owner:
Private replay-policy manifest constructors accepted raw `&'static str`
command-kind labels and converted them to `ReplayCommandKindLabel` inside
helper bodies. Command-dispatch rows also stored command-manifest IDs as raw
strings. Quota and cycle-reserve policy IDs were stored as optional raw
strings in manifest row types.

Intended owner:
Replay-policy manifest call sites own typed static command-kind and
guard-policy label construction, and private manifest helpers accept typed
labels.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
None.

Serialized surfaces affected:
None.

Validation:
- `cargo test --locked -p canic-core replay_policy --lib`
- `cargo clippy --locked -p canic-core --all-targets -- -D warnings`

Explicit non-scope:
- no endpoint changes
- no command changes
- no Candid changes
- no JSON field changes
- no deployment truth or evidence schema changes
- no stable-state layout changes
- no changes to runtime replay `CommandKind`, receipt storage, operation IDs,
  cost guards, workflow replay descriptors, endpoint-name labels, or
  quota/reserve policy string values

## 0.83 Runtime Bootstrap Phase Label Typing

Status:
completed in 0.83.14 for the accepted `CANIC-083-DEBT-019` scope.

Source findings:
- CANIC-083-DEBT-019

Boundary:
Runtime bootstrap status phase labels.

Previous owner:
Bootstrap status storage and root/nonroot lifecycle scheduling call sites used
raw `&'static str` phase labels.

Intended owner:
Runtime bootstrap ops own typed `BootstrapPhaseLabel` values. DTO projection
formats the same stable phase strings.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust bootstrap ops API only. No CLI or endpoint surface changes.

Serialized surfaces affected:
None.

Validation:
- `cargo test --locked -p canic-core bootstrap --lib`
- `cargo check --locked -p canic-control-plane`
- `cargo clippy --locked -p canic-core --all-targets -- -D warnings`

Explicit non-scope:
- no endpoint changes
- no command changes
- no Candid changes
- no JSON field changes
- no runtime introspection behavior changes
- no recent-failure content changes
- no lifecycle scheduling behavior changes
- no deployment truth or evidence schema changes
- no stable-state layout changes

## 0.83 Host Install-Root Phase Label Typing

Status:
completed in 0.83.15 for the accepted `CANIC-083-DEBT-020` and
`CANIC-083-DEBT-021` scopes.

Source findings:
- CANIC-083-DEBT-020
- CANIC-083-DEBT-021

Boundary:
Host install-root operation, deployment-truth receipt phase, and timing output
labels.

Previous owner:
Install-root operations, completed-phase receipts, artifact-promotion receipts,
deployment-truth gate operation IDs, and timing summary rows passed maintained
labels as raw strings.

Intended owner:
Host install-root owns typed `InstallPhaseLabel` and `InstallTimingLabel`
values. Deployment-truth DTO projection and timing-table rendering format the
same stable strings.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust install-root internals only. No CLI or endpoint surface changes.

Serialized surfaces affected:
None. Deployment-truth receipt phase strings, operation IDs, and timing table
labels remain unchanged.

Validation:
- `cargo check --locked -p canic-host`
- `cargo test --locked -p canic-host install_truth`
- `cargo test --locked -p canic-host install_timing_summary`
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON field changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes
- no install-root behavior changes
- no receipt phase string changes
- no receipt operation ID changes
- no timing table label changes

## 0.83 Host Install-Root Execution Preflight Receipt Label Typing

Status:
completed in 0.83.16 for the accepted `CANIC-083-DEBT-022` scope.

Source findings:
- CANIC-083-DEBT-022

Boundary:
Host install-root execution-preflight receipt phase, failure-code, and evidence
labels, plus deployment-truth execution-preflight planned-phase labels.

Previous owner:
The execution-preflight receipt builder constructed operation IDs, phase
receipts, failure command-result codes, and evidence keys from raw strings.
The deployment-truth execution-preflight builder also owned current-install
planned phases as raw strings.

Intended owner:
Host install-root owns the execution-preflight phase through
`InstallPhaseLabel`; the execution-preflight receipt builder owns failure-code
and evidence-key labels through `ExecutionPreflightReceiptLabel`.
Deployment-truth owns the current-install execution-preflight planned phases
through `CurrentInstallExecutionPhaseLabel`.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust install-root internals only. No CLI or endpoint surface changes.

Serialized surfaces affected:
None. Deployment-truth receipt phase strings, operation IDs, command-result
codes, evidence strings, and execution-preflight planned-phase strings remain
unchanged.

Validation:
- `cargo check --locked -p canic-host`
- `cargo test --locked -p canic-host execution_preflight`
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON field changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes
- no install-root behavior changes
- no receipt phase string changes
- no receipt operation ID changes
- no receipt evidence string changes

## 0.83 Execution Preflight Validation, Blocker, And Text Label Typing

Status:
completed in 0.83.17 for the accepted `CANIC-083-DEBT-023` scope.

Source findings:
- CANIC-083-DEBT-023

Boundary:
Deployment-truth execution-preflight validation field labels, blocker labels,
and text renderer labels.

Previous owner:
Execution-preflight validation helpers accepted raw field-name strings, and
execution-preflight blocker construction owned maintained safety-finding codes
and the static authority fallback subject as raw strings. The
execution-preflight text renderer owned title, field, section, and status labels
as raw strings.

Intended owner:
Deployment-truth execution-preflight validation owns field labels through
`DeploymentExecutionPreflightFieldLabel`. Execution-preflight blocker
construction owns safety-finding codes through
`DeploymentExecutionPreflightBlockerCode` and the static authority fallback
subject through `DeploymentExecutionPreflightSubjectLabel`. The
execution-preflight text renderer owns rendered text labels through
`ExecutionPreflightTextLabel`.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust deployment-truth internals only. No CLI or endpoint surface changes.

Serialized surfaces affected:
None. Validation error field strings, safety-finding code strings, fallback
subject string, and operator text output labels remain unchanged.

Validation:
- `cargo check --locked -p canic-host`
- `cargo test --locked -p canic-host execution_preflight`
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON field changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes
- no execution-preflight behavior changes
- no validation error field string changes
- no safety-finding code string changes
- no fallback subject string changes
- no operator text output label changes

## 0.83 Comparison Report Validation And Text Label Typing

Status:
completed in 0.83.18 for the accepted `CANIC-083-DEBT-024` scope.

Source findings:
- CANIC-083-DEBT-024

Boundary:
Deployment-truth comparison report validation field labels and text renderer
labels.

Previous owner:
Comparison validation helpers accepted raw field-name strings, target-side
strings, and target-field strings, including a fallback string path for target
field labels. The comparison text renderer owned title, field, section, count,
target, and fallback labels as raw strings.

Intended owner:
Deployment-truth comparison validation owns field labels through
`DeploymentComparisonFieldLabel` plus typed target-side/field enums. The
comparison text renderer owns rendered text labels through
`DeploymentComparisonTextLabel`.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust deployment-truth internals only. No CLI or endpoint surface changes.

Serialized surfaces affected:
None. Validation error field strings and operator text output labels remain
unchanged.

Validation:
- `cargo check --locked -p canic-host`
- `cargo test --locked -p canic-host comparison`
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON field changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes
- no comparison behavior changes
- no validation error field string changes
- no operator text output label changes

## 0.83 Authority Report Text Label Typing

Status:
completed in 0.83.19 for the accepted `CANIC-083-DEBT-025` scope.

Source findings:
- CANIC-083-DEBT-025

Boundary:
Deployment-truth authority report text renderer labels and report-owned shared
authority text helper labels.

Previous owner:
The authority report text renderer owned title, field, section, count,
fallback, and list labels as raw strings. Shared report helpers also owned
blocker, next-action, automatic-action, external-action, and apply-blocker
labels as raw strings.

Intended owner:
The authority report renderer owns rendered text labels through
`AuthorityReportTextLabel`. Report-owned shared helper labels are owned through
`AuthoritySharedTextLabel`.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust deployment-truth internals only. No CLI or endpoint surface changes.

Serialized surfaces affected:
None. Operator text output labels remain unchanged.

Validation:
- `cargo check --locked -p canic-host`
- `cargo test --locked -p canic-host authority`
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON field changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes
- no authority behavior changes
- no operator text output label changes

## 0.83 Delegated Auth Candid Surface Hard Cut

Status:
completed in 0.83.20 for the accepted `CANIC-083-DEBT-026` scope.

Source findings:
- CANIC-083-DEBT-026

Boundary:
Delegated-auth verifier policy and registry snapshot metadata versus active
Candid endpoint/token/proof payloads.

Previous owner:
`RootProofMode`, `RootKeyPolicyV1`, `DelegatedAuthRegistrySnapshotV1`, and
`DelegatedAuthIssuerPolicySnapshotV1` derived `CandidType` and were pinned by a
protocol-surface Candid round-trip test, even though they are local verifier
policy/canonical-hash metadata rather than active Candid endpoint payloads.

Intended owner:
Delegated-auth active token, root proof, issuer proof, proof install, and proof
status types own the Candid boundary. Verifier policy and registry snapshot
metadata own canonical hash/config semantics without deriving `CandidType`.

Behavior impact label:
behavior_change_declared for the Rust trait surface only.

Public surfaces affected:
Rust trait surface for the four metadata types. No CLI, endpoint, token, proof,
or canister method surface changes.

Serialized surfaces affected:
None for active Candid endpoint payloads, JSON schemas, deployment truth,
evidence/report schemas, or stable-state layout.

Validation:
- `cargo check --locked -p canic-core -p canic`
- `cargo test --locked -p canic --test protocol_surface`
- `cargo test --locked -p canic-core auth --lib`

Explicit non-scope:
- no command changes
- no endpoint changes
- no active delegated token Candid payload changes
- no active root proof Candid payload changes
- no active issuer proof Candid payload changes
- no proof install/status Candid payload changes
- no JSON field changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes

## 0.83 Local Metadata Candid Surface Hard Cut

Status:
completed in 0.83.21 for the accepted `CANIC-083-DEBT-027` scope.

Source findings:
- CANIC-083-DEBT-027

Boundary:
Local runtime/config/policy and bootstrap validation metadata versus active
Candid DTO payloads.

Previous owner:
`ids::BuildNetwork` derived `CandidType` even after the delegated-auth policy
metadata hard cut removed the only Candid-bearing consumer found in this slice.
`ValidationReport` and `ValidationIssue` also derived `CandidType` even though
they are root bootstrap validation metadata, not active endpoint DTOs.

Intended owner:
`BuildNetwork` remains local runtime/config/policy data with `serde` support and
stable `as_str()` labels. `ValidationReport` and `ValidationIssue` remain root
bootstrap validation metadata with serde support. Active Candid DTOs own any
Candid boundary explicitly.

Behavior impact label:
behavior_change_declared for the Rust trait surface only.

Public surfaces affected:
Rust trait surface for `BuildNetwork`, `ValidationReport`, and
`ValidationIssue`. No CLI, endpoint, token, proof, or canister method surface
changes.

Serialized surfaces affected:
None for active Candid endpoint payloads, JSON schemas, deployment truth,
evidence/report schemas, or stable-state layout.

Validation:
- `cargo check --locked -p canic-core -p canic -p canic-control-plane`
- `cargo test --locked -p canic-core auth --lib`
- `cargo test --locked -p canic-control-plane --lib`

Explicit non-scope:
- no command changes
- no endpoint changes
- no active Candid payload changes
- no bootstrap behavior changes
- no JSON field changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes

## 0.83 State Manifest And Audit Label Ownership

Status:
completed in 0.83.22 for the accepted `CANIC-083-DEBT-028` scope.

Source findings:
- CANIC-083-DEBT-028

Boundary:
State manifest storage and migration-policy schema labels used by state CLI
rendering and runtime state summaries, plus state-audit status labels used by
CLI audit text rendering and medic state-audit summaries.

Previous owner:
The state CLI text renderer and runtime state-summary builder each owned local
matches from `StateStorage`, `MigrationPolicy`, or `StateAuditStatus` variants
to stable schema/report labels. Medic also owned a local `StateAuditStatus`
label match for its state-audit project check.

Intended owner:
The state contract model owns the stable labels through `StateStorage::as_str()`
and `MigrationPolicy::as_str()`. The state-audit report model owns status
labels through `StateAuditStatus::label()`. Renderers and runtime summaries
consume those owner-defined labels, and medic consumes the same report-owned
status labels for its state-audit summary.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust internals only. No CLI command, endpoint, or canister method surface
changes.

Serialized surfaces affected:
None. State manifest JSON labels, state-audit JSON labels, text output labels,
runtime state summary strings, deployment truth schema, evidence/report schemas,
and stable-state layout remain unchanged.

Validation:
- `cargo check --locked -p canic-core -p canic-cli -p canic-host`
- `cargo test --locked -p canic-core state_contract --lib`
- `cargo test --locked -p canic-core runtime --lib`
- `cargo test --locked -p canic-cli state`
- `cargo test --locked -p canic-cli medic`
- `cargo test --locked -p canic-host state_manifest --lib`
- `cargo clippy --locked -p canic-core -p canic-cli -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON field changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes
- no audit status/check behavior changes

## 0.83 Runtime Introspection Enum Label Ownership

Status:
completed in 0.83.23 for the accepted `CANIC-083-DEBT-029` scope.

Source findings:
- CANIC-083-DEBT-029

Boundary:
Runtime introspection enum labels used by runtime DTO serde/Candid contracts and
`canic inspect` text rendering.

Previous owner:
The inspect CLI text renderer owned local matches from runtime status, timer
status, state-domain status, and failure-severity enum variants to stable
labels. Runtime DTO serde-label tests also carried a second copy of
representative runtime enum labels.

Intended owner:
`domain::runtime` owns all stable runtime enum labels through `label()` methods.
Inspect rendering and runtime DTO tests consume those owner-defined labels.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust internals only. No CLI command, endpoint, or canister method surface
changes.

Serialized surfaces affected:
None. Runtime JSON labels, Candid payload labels, inspect text output labels,
deployment truth schema, evidence/report schemas, and stable-state layout remain
unchanged.

Validation:
- `cargo check --locked -p canic-core -p canic-cli`
- `cargo test --locked -p canic-core runtime --lib`
- `cargo test --locked -p canic-cli inspect`
- `cargo test --locked -p canic --test protocol_surface`
- `cargo clippy --locked -p canic-core -p canic-cli --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid label changes
- no JSON label changes
- no inspect text output label changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes

## 0.83 Deployment Truth Status Label Ownership

Status:
completed in 0.83.24 for the accepted `CANIC-083-DEBT-030` scope.

Source findings:
- CANIC-083-DEBT-030

Boundary:
Deployment-truth model status enums versus deployment-truth text rendering and
medic receipt summaries.

Previous owner:
Deployment-truth text renderer modules owned local matches from safety,
execution-preflight, execution, promotion readiness, external lifecycle plan,
external upgrade completion, and verification-requirement status variants to
stable text labels. Medic consumed one of those renderer-owned helper functions
for authority receipt summaries.

Intended owner:
Deployment-truth model status enums own their stable labels through `label()`
methods. Text renderers and medic summaries consume those model-owned labels.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust internals only. No CLI command, endpoint, or canister method surface
changes.

Serialized surfaces affected:
None. Operator text output labels, medic text, JSON schemas, deployment truth
schema, evidence/report schemas, Candid, and stable-state layout remain
unchanged.

Validation:
- `cargo check --locked -p canic-host -p canic-cli`
- `cargo test --locked -p canic-host deployment_truth --lib`
- `cargo test --locked -p canic-cli medic`
- `cargo clippy --locked -p canic-host -p canic-cli --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON schema or label changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes
- no operator text output label changes

## 0.83 Deployment Root Verification Text Label Ownership

Status:
completed in 0.83.25 for the accepted `CANIC-083-DEBT-031` scope.

Source findings:
- CANIC-083-DEBT-031

Boundary:
Deployment-root verification and root-observation model labels versus
deployment-root verification report/receipt text rendering.

Previous owner:
Deployment-root verification report and receipt text renderers used `Debug`
formatting for root verification source, evidence status, state transition,
root verification state, and root observation source labels. The root
verification evidence-check builder also owned a local root-observation-source
label match.

Intended owner:
The deployment-truth root-verification and inventory model enums own the exact
operator text labels through `label()` methods. Report and receipt text
renderers plus root verification evidence-check construction consume those
owner-defined labels.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust internals only. No CLI command, endpoint, or canister method surface
changes.

Serialized surfaces affected:
None. Operator text output labels, JSON schemas, deployment truth schema,
evidence/report schemas, Candid, and stable-state layout remain unchanged.

Validation:
- `cargo check --locked -p canic-host`
- `cargo test --locked -p canic-host root_verification --lib`
- `cargo test --locked -p canic-host deployment_truth --lib`
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON schema or label changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes
- no operator text output label changes

## 0.83 Deployment Truth Control-Class Label Ownership

Status:
completed in 0.83.26 for the accepted `CANIC-083-DEBT-032` and
`CANIC-083-DEBT-033` scopes.

Source findings:
- CANIC-083-DEBT-032
- CANIC-083-DEBT-033

Boundary:
Deployment-truth inventory control-class labels versus report/diff builders,
external lifecycle text rendering, and external-upgrade verification summaries.
Deployment-truth external lifecycle model enum labels versus external lifecycle
text rendering.

Previous owner:
Report builders and lifecycle helpers used enum `Debug` formatting for
`CanisterControlClassV1` labels, and canister/pool diff builders duplicated
expected `DeploymentControlled` and `CanicManagedPool` label literals.
External lifecycle text helpers also owned local match blocks for lifecycle
mode, consent state, verification result, observation source, consent
subject/channel, and verification requirement labels.

Intended owner:
`CanisterControlClassV1` owns the exact control-class labels through `label()`.
Report, diff, lifecycle text, and external-upgrade verification helpers consume
those owner-defined labels.
External lifecycle model enums own their exact text labels through `label()`
methods, and lifecycle text renderers consume those owner-defined labels.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust internals only. No CLI command, endpoint, or canister method surface
changes.

Serialized surfaces affected:
None. Operator text output labels, diff values, JSON schemas, deployment truth
schema, evidence/report schemas, Candid, and stable-state layout remain
unchanged.

Validation:
- `cargo check --locked -p canic-host`
- `cargo test --locked -p canic-host deployment_truth --lib`
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON schema or label changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes
- no operator text output label changes
- no diff value changes

## 0.83 Promotion Artifact And Policy Label Ownership

Status:
completed in 0.83.27 for the accepted `CANIC-083-DEBT-034` scope.

Source findings:
- CANIC-083-DEBT-034

Boundary:
Deployment-truth promotion artifact/policy model enum labels versus promotion
text rendering and promotion artifact identity-key helpers.

Previous owner:
Promotion text renderers used enum `Debug` formatting for promotion artifact
level, role artifact source kind, artifact identity kind, policy requirement,
policy claim, artifact source, observation status, and role phase result
labels. Promotion artifact identity-key helpers also used `Debug` formatting
for role artifact source kind labels.

Intended owner:
Promotion, artifact, inventory, and execution model enums own their exact text
labels through `label()` methods. Promotion text renderers and promotion
identity-key helpers consume those owner-defined labels.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Rust internals only. No CLI command, endpoint, or canister method surface
changes.

Serialized surfaces affected:
None. Operator text output labels, identity-key strings, JSON schemas,
deployment truth schema, evidence/report schemas, Candid, and stable-state
layout remain unchanged.

Validation:
- `cargo check --locked -p canic-host`
- `cargo test --locked -p canic-host promotion --lib`
- `cargo test --locked -p canic-host deployment_truth --lib`
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON schema or label changes
- no deployment truth schema changes
- no evidence/report schema changes
- no stable-state layout changes
- no operator text output label changes
- no identity-key string changes
- no execution/status mixed-label cleanup

## 0.83 Receipt Resume Structural Evidence Comparison

Status:
completed for 0.83.28 for `CANIC-083-DEBT-035`.

Source findings:
- CANIC-083-DEBT-035

Boundary:
Typed deployment receipt evidence versus human-readable conflict diagnostics.

Previous owner:
Delimiter-joined diagnostic strings doubled as duplicate conflict identities.

Intended owner:
Typed phase and role-phase evidence keys decide conflicts. Formatted strings
only explain the resulting finding.

Behavior impact label:
behavior_change_declared. Valid receipts are unchanged; malformed or
conflicting receipts that previously collided now fail closed.

Public surfaces affected:
Passive resume-safety results may correctly change from warning to blocked for
structurally conflicting receipt evidence.

Serialized surfaces affected:
None. Receipt JSON, resume-safety JSON, deployment-truth schemas, Candid, and
stable-state layout are unchanged.

Validation:
- `cargo test --locked -p canic-host deployment_truth::tests::execution_receipts::resume --lib`
- `cargo test --locked -p canic-host deployment_truth --lib`
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no automated resume execution
- no receipt JSON schema changes
- no deployment-truth schema changes
- no command changes

## 0.83 Promotion Execution And Staging Label Ownership

Status:
completed for 0.83.28 for `CANIC-083-DEBT-036`.

Source findings:
- CANIC-083-DEBT-036

Boundary:
Deployment execution/promotion model variant labels versus promotion text and
staging evidence consumers.

Previous owner:
Promotion text and staging evidence relied on enum `Debug`; an unused public
previous-receipt-kind label method had no production consumer.

Intended owner:
Execution, promotion, observation-status, and artifact-transport enums own the
exact labels consumed by text and evidence builders.

Behavior impact label:
behavior_change_declared for the pre-1.0 removal of the unused public Rust
method; runtime behavior and emitted strings are unchanged.

Public surfaces affected:
The unused `PreviousArtifactReceiptKindV1::label()` Rust method is removed.
CLI commands, endpoints, and canister methods are unchanged.

Serialized surfaces affected:
None. Operator text, receipt evidence strings, JSON schemas, deployment truth,
evidence/report schemas, Candid, and stable-state layout are unchanged.

Validation:
- `cargo test --locked -p canic-host promotion --lib`
- `cargo test --locked -p canic-host deployment_truth --lib`
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`

Explicit non-scope:
- no command changes
- no endpoint changes
- no Candid changes
- no JSON or stable-state changes
- no emitted text/evidence label changes

## 0.83 Audit Closeout Metadata Reconciliation

Status:
completed for 0.83.28 for `CANIC-083-DEBT-037`.

Source findings:
- CANIC-083-DEBT-037

Boundary:
Canonical 0.83 audit status versus release/tag and handoff state.

Previous owner:
Ledger header/scope, recommended slices, and the compact handoff described
different completion points.

Intended owner:
The canonical ledger records complete finding state, and the handoff mirrors
the current package surface and 0.83.28 release-preparation batch.

Behavior impact label:
no_behavior_change.

Public surfaces affected:
Documentation only.

Serialized surfaces affected:
None.

Validation:
- `cargo test --locked -p canic --test changelog_governance`
- `git diff --check`

Explicit non-scope:
- no package version change
- no release command
- no git staging, commit, tag, or push
