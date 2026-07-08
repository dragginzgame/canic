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

Source findings:
- CANIC-083-DEBT-002

Boundary:
Default-JSON advanced deploy report and request-inspection output selection.

Current owner:
Each advanced deploy subfamily owns a local `--format json|text` parser.

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
