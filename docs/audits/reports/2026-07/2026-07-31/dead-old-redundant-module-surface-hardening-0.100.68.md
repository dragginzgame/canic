# Module Surface Hardening: exact-body duplication follow-up

## Run Metadata

| Field | Value |
| ---- | ---- |
| `method_version` | `MSH-2.0` |
| `surface_taxonomy` | `ST-1` |
| `authority_taxonomy` | `AT-1` |
| `deletion_confidence_model` | `DC-1` |
| `compatibility_policy` | `pre-1.0-hard-cut` |
| `code_snapshot` | `8720cb7a2` (`v0.100.67`) plus the open `0.100.68` worktree |
| `in_scope_roots` | `canic-host`, `canic-cli`, and `canic-backup` Rust source |
| `excluded_roots` | generated output, target artifacts, canister runtime and stable-state code |
| `audit_tier` | `Tier 1`: host/operator implementation only; no boundary or persistence shape changed |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS` after focused validation.
- Risk score: `1 / 10` after cleanup.
- Cleanup result: ten identical Fleet-install `IcpCli` constructors and
  forwarding wrappers were replaced by one install-boundary constructor,
  removing nine functions and 87 net production lines.
- Contract result: the exact executable, environment, project root and
  optional direct-replica target are unchanged at every caller. No command,
  endpoint, Candid, stable schema, serialized shape or public Rust API changed.

## Evidence Log

| Evidence | Inspection | Result |
| ---- | ---- | ---- |
| declaration reachability | Counted exact identifier occurrences across repository Rust source and inspected zero- and one-consumer declarations. | No zero-consumer host function remained. Ten one-consumer forwarding helpers with no independent invariant were removed in the settled `0.100.67` release tail. |
| exact-body duplication | Parsed non-test functions under host, CLI and backup source and compared whitespace-normalized bodies. | Nine install modules repeated the same three-step `IcpCli` construction; Registry synchronization added a tenth wrapper around its local copy. |
| call-site authority | Traced every constructor consumer through Coordinator installation, Registry join/activation/synchronization, root installation, Store bootstrap, Component Registry preparation, mirror activation and runtime activation. | Canister principals, methods, typed arguments, journals and verification remain at their existing role-specific call sites. Only transport-context construction moved. |
| dependency inventory | Re-ran direct-dependency analysis after `0.100.67`. | The only findings remain `candid` and `ic-cdk` in the Fleet Coordinator stub, already proven required by `start_fleet_coordinator!` and `finish!` macro expansion. |
| retained duplicate bodies | Reviewed exact-body groups for timestamps, typed journal transitions, command parsing/rendering, error conversion and test support. | Retained where separate type/domain ownership is meaningful or consolidation would couple unrelated command/package boundaries without deleting behavior. |

## Candidate Dispositions

| Candidate | Surface Class | Authority / Consumer Result | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- |
| nine role-local install ICP constructors | `duplicate-surface` | Byte-for-byte identical context construction; role authority is supplied after construction. | `DELETE NOW` in favor of one install owner | Low |
| Registry-sync Coordinator wrapper | `duplicate-surface` | Forwards directly to the identical root constructor without adding policy or evidence. | `DELETE NOW` | Low |
| generated-service direct dependencies | `live-generated-boundary` | Required by macro expansion despite no ordinary source reference. | `RETAIN WITH OWNER` | High if removed |
| distinct timestamp helpers | `domain-local-utility` | Output/error semantics differ by CLI report, backup marker and operation-identity owner. | `RETAIN WITH OWNER` | Low |
| typed journal transition helpers | `domain-local-invariant` | Identical mechanics operate on distinct durable journal types and validation owners. | `RETAIN WITH OWNER` | Medium if merged |

## Verification Readout

- `cargo clippy -p canic-host --all-targets --no-deps -- -D warnings`: PASS.
- `cargo test -p canic-host install_root::coordinator_install::tests --lib`:
  PASS, 3 tests.
- `cargo test -p canic-host install_root::tests::commands --lib`: PASS, 4
  tests.
- Targeted formatting and `git diff --check`: PASS.

The settled `0.100.67` tail was also revalidated after the release target
stopped changing: backup durable-JSON tests and focused cycles, metrics,
endpoint-rendering and deploy-resume CLI tests all pass.
