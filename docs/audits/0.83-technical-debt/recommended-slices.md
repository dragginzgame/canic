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
None.

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
