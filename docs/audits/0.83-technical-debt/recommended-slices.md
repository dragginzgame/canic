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
