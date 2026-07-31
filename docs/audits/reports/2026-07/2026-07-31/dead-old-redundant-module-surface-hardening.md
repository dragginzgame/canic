# Module Surface Hardening: dead, old, and redundant code follow-up

## Run Metadata

| Field | Value |
| ---- | ---- |
| `method_version` | `MSH-2.0` |
| `surface_taxonomy` | `ST-1` |
| `authority_taxonomy` | `AT-1` |
| `deletion_confidence_model` | `DC-1` |
| `compatibility_policy` | `pre-1.0-hard-cut` |
| `code_snapshot` | `7a09657fa` (`v0.100.65`) plus the shared in-progress `0.100.66` worktree |
| `in_scope_roots` | Rust source under `crates/`, `canisters/`, and `apps/` |
| `excluded_roots` | archived documentation, generated output, target artifacts, and concurrent lifecycle files |
| `test_surface_inclusion` | workspace-only test library and all repository Rust consumers |
| `audit_tier` | `Tier 2`: public Rust DTO, serialized enum, host snapshot, and generated/facade boundaries were inspected; persistence and Candid owners were traced |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS` after focused validation.
- Risk score: `2 / 10` after cleanup.
- Cleanup result: 866 net lines removed. The uncalled 626-line
  `canic_tests::root::assertions` module, its module export, one isolated
  `count_workers` helper, four zero-consumer crate-private methods, an obsolete
  host root-funding estimator/cache, and two abandoned DTO islands were deleted.
- Coverage result: no maintained behavior coverage was lost. The assertion
  functions never executed; two removed tests exercised only the deleted
  root-funding surface, while topology compilation remains validated on load.
- Retained surface: public protocol constants and asynchronous access/lifecycle
  signatures remain because they have facade, generated, or downstream-facing
  contract owners rather than repository-only reachability owners.

## Step Status

| Step | Status | Evidence |
| ---- | ---- | ---- |
| Inventory suppressions and stale markers | PASS | The companion `dead_code` report owns the two retained expectations; no additional `allow(dead_code)` surface exists. |
| Inventory zero-consumer functions | PASS | Identifier-count scan across all repository Rust source, followed by exact `rg` consumer checks. |
| Trace test-support ownership | PASS | `canic-tests` is `publish = false`; no integration test imports `root::assertions`, and only two worker helpers are called. |
| Trace crate-private wrappers | PASS | Exact-name scans found no call, documentation, macro, or generated consumer for the four methods. |
| Trace public DTO and serialized owners | PASS | Declaration/re-export scans proved the validation DTO module and host activation-record types had no source consumer; authority reconciliation never issued the removed import-confirmation action. |
| Patch high-confidence cuts | PASS | Twenty-three code files changed across the two passes, including deletion of two obsolete modules and retargeting one unit test to live topology projections. |
| Focused verification | PASS | Formatting, package checks, Clippy, and the focused instruction-audit test passed. |

## Evidence Log

| Evidence | Inspection | Result |
| ---- | ---- | ---- |
| zero-consumer public function scan | Compared public function declarations under `crates/` with identifier occurrence counts across every repository `.rs` file. | Initially identified the dead `canic-tests` helpers and three internal methods; no candidate remains after the patch. |
| exact assertion-module scan | `rg` for `root::assertions`, `assertions::`, and every exported helper name. | No consumer outside the module; private helpers formed one closed unreachable island beneath the public helpers. |
| test-package ownership | Inspected `crates/canic-tests/Cargo.toml`, `src/lib.rs`, `root/mod.rs`, and integration-test imports. | Package is workspace-only and unpublished. The assertion module was not an external contract. |
| worker-helper scan | Exact search for `root::workers` and each public worker helper. | `create_worker` and `prepare_worker_for_explicit_parent_funding` are live; `count_workers` had no caller. |
| internal method scan | Exact search for `role_bearing_child_roles`, `missing_required_fields`, `SubnetRegistryOps::has_role`, and `attached_app_roles`, including a recursive declaration-plus-test-only pass. | The first three were declaration-only. `attached_app_roles` was called only by a test of that otherwise-unused projection. Their owner modules are `pub(crate)`, and live underlying projections remain at direct call sites. |
| public type island scan | Compared public struct/enum/type declarations with exact identifier occurrence counts, then traced re-exports and return/field owners. | `FleetActivationHostRecord`, its canister evidence, host-size limit, and `dto::validation` were closed zero-consumer islands. No public type remains with only declaration/comment occurrences. |
| host snapshot projection scan | Traced every `AppConfigSnapshot` field and projection into host, CLI, and test-support consumers. | Local-root funding was test-only and had no production consumer. The cached `component_topology` getter is consumed by PocketIC support and was retained. |
| stale compatibility scan | Searched current source for legacy, obsolete, fallback, deprecated, import/adoption vocabulary, TODO, FIXME, and dead/unused suppressions. | Removed the never-issued `RequiresDestructiveImportConfirmation` action. Remaining matches describe current validation, recovery, observation, or generated contracts. |

## Candidate Dispositions

| Candidate | Surface Class | Authority / Consumer Result | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- |
| `canic_tests::root::assertions` | `orphaned-helper` test-support island | No repository consumer; unpublished package; no test function in the file. | `DELETE NOW` | Low |
| `root::workers::count_workers` | `orphaned-helper` | No caller; adjacent worker creation/preparation helpers remain live. | `DELETE NOW` | Low |
| `CanisterConfig::role_bearing_child_roles` | `overexposed-internal` | Declaration-only method in crate-private config schema; no validator or topology compiler calls it. | `DELETE NOW` | Low |
| `ConfigModel::attached_app_roles` | `orphaned-helper` | Its only caller tested the otherwise-unused projection; the test now directly covers live attached/deployable role projections. | `DELETE NOW` | Low |
| `EnvOps::missing_required_fields` | `duplicate-surface` | Declaration-only wrapper; import validation still calls `required_fields_missing` directly. | `DELETE NOW` | Low |
| `SubnetRegistryOps::has_role` | `duplicate-surface` | Declaration-only wrapper over the still-live role lookup. | `DELETE NOW` | Low |
| local-root create-cycle estimator and readiness constant | `stale-compatibility` | Method, projection, constant, and both tests had no production consumer; current Fleet install plans bind `PlannedCanisterCreationFunding`, which the create commands consume. | `DELETE NOW` | Low |
| cached `AppConfigSnapshot.component_topology` and getter | `live-test-boundary` | PocketIC canister, lifecycle, and Fleet-registry fixture setup consume the validated topology snapshot. | `RETAIN WITH OWNER` | Low |
| `FleetActivationHostRecord` DTO island | `orphaned-helper` | Host journal/runtime code uses current install-journal and `FleetActivationStatusResponse` shapes; the record, canister evidence, and byte bound had no consumer. | `DELETE NOW` | Low |
| `dto::validation` | `orphaned-helper` | `ValidationReport` and `ValidationIssue` had no endpoint, facade, host, macro, test, or generated consumer. | `DELETE NOW` | Low |
| duplicate `CRATE_NAME` constants | `duplicate-surface` | Neither facade nor core constant had a repository/documentation consumer; `VERSION` remains the owned runtime metadata value. | `DELETE NOW` | Low |
| `RequiresDestructiveImportConfirmation` | `stale-compatibility` | Serialized authority action was never constructed or matched and conflicts with the current reinstall-only/no-import contract. | `DELETE NOW` | Low |
| `TopologyHasher::canonical_input` visibility | `overexposed-internal` | Only `hash` and its child unit test consume the canonical byte builder. | `NARROW NOW` | Low |
| async access predicates | `live-generated-boundary` | Access-expression expansion awaits a uniform async predicate surface. | `RETAIN WITH OWNER` | Medium if signature changes |
| generated lifecycle async functions | `live-generated-boundary` | IC lifecycle macro expansion owns the async entrypoint signatures. | `RETAIN WITH OWNER` | Medium if signature changes |
| protocol/default constants with no in-repository caller | `live-facade` | Explicit public contract constants may be consumed downstream and are not internal reachability candidates. | `RETAIN WITH OWNER` | Low |

## Removal Safety And Runtime Shape

| Cut | Runtime Shape Before | Runtime Shape After | Required Proof | Result |
| ---- | ---- | ---- | ---- | ---- |
| dead assertion module | Unreachable host test-library code | Absent | `canic-tests` all-target compile and focused live-consumer test | PASS |
| dead worker counter | Unreachable host test-library code | Absent | `canic-tests` all-target compile and focused live-consumer test | PASS |
| four internal methods | Uncalled native/wasm library functions | Absent; direct underlying owners unchanged | core check, focused config test, and Clippy | PASS |
| host root-funding residue | Uncalled estimator API beside a live validated topology snapshot | Topology snapshot/getter remain intact; only the obsolete funding estimate is absent | host and `canic-tests` all-target checks plus release-set tests | PASS |
| abandoned DTO/type islands | Public Rust vocabulary without a consumer | Absent | core/facade checks and declaration scan | PASS |
| authority action variant | Unissued JSON vocabulary | Absent under the pre-1.0 hard cut | host all-target check and authority tests | PASS |
| topology canonical-input helper | Public method | Private implementation detail; hash bytes unchanged | backup all-target check and topology tests | PASS |

The public Rust surface and one never-issued JSON enum vocabulary were narrowed.
No Candid endpoint, stable record, stable-memory ID, feature gate, runtime
authority branch, or generated macro shape changed. Raw wasm comparison is not
required: deleted runtime items were unreachable and host-only cache removal
does not affect canister wasm.

## Verification Readout

- Targeted `rustfmt --edition 2024 --check` over every retained Rust file changed
  by the two cleanup reports: PASS.
- `cargo check -p canic-core --no-default-features`: PASS.
- `cargo check -p canic-host --all-targets`: PASS.
- `cargo check -p canic-backup --all-targets`: PASS.
- `cargo check -p canic --all-targets`: PASS.
- `cargo clippy -p canic-core --no-default-features --lib -- -D warnings`:
  PASS.
- `cargo clippy -p canic-host --all-targets -- -D warnings`: PASS.
- `cargo clippy -p canic-backup --lib -- -D warnings`: PASS.
- `cargo clippy -p canic --all-targets -- -D warnings`: PASS.
- `cargo test -p canic-core attached_and_deployable_roles_follow_structural_ownership --lib`:
  PASS, 1 focused test.
- `cargo test -p canic-core dto::fleet_activation::tests --lib`: PASS, 1
  focused test.
- `cargo test -p canic-host release_set::config::tests --lib`: PASS, 8 focused
  tests.
- `cargo test -p canic-host release_set::tests::roles::bootstrap --lib`: PASS,
  3 focused tests.
- `cargo test -p canic-host authority --lib`: PASS, 24 focused tests.
- `cargo test -p canic-backup topology::tests --lib`: PASS, 3 focused tests.
- `cargo check -p canic-tests --all-targets`: PASS. A pre-finalization run
  exposed the live `component_topology` test-support consumer; retaining that
  surface made the clean rerun pass.
- `cargo test -p canic-tests instruction_audit --no-run`: PASS.
- Targeted `git diff --check`: PASS.
- `cargo test -p canic --test changelog_governance`: PASS.

## Disposition Summary

| Disposition | Count |
| ---- | ----: |
| `DELETE NOW` | 11 |
| `NARROW NOW` | 1 |
| `MOVE TO TEST` | 0 |
| `RETAIN WITH OWNER` | 4 classes |
| `BLOCKED` | 0 |

## Follow-up Actions

1. Keep the two remaining `dead_code` expectations tied to the state-contract
   triggers recorded in the companion allowance report.
2. Re-run the declaration-only reachability inventory when workspace-only test
   helpers are added or reorganized; public visibility inside an unpublished
   test package otherwise hides unreachable helper islands from `dead_code`.
