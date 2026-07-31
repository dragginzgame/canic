# Module Surface Hardening: `dead_code` allowances

## Run Metadata

| Field | Value |
| ---- | ---- |
| `method_version` | `MSH-2.0` |
| `surface_taxonomy` | `ST-1` |
| `authority_taxonomy` | `AT-1` |
| `deletion_confidence_model` | `DC-1` |
| `compatibility_policy` | `pre-1.0-hard-cut` |
| `wasm_signal_rule` | `raw-wasm-primary` |
| `hot_path_risk_model` | `HP-1` |
| `proof_policy` | `read-only-first` |
| `baseline_report` | `N/A` |
| `comparability_status` | `non-comparable`: first repository-wide allowance inventory |
| `code_snapshot` | `7a09657fa` (`v0.100.65`) plus the shared in-progress `0.100.66` worktree |
| `in_scope_roots` | Rust source under `crates/`, `canisters/`, and `apps/` |
| `excluded_roots` | historical docs, generated output, target artifacts, and unrelated unsuppressed warnings |
| `generated_code_inclusion` | focused `canic::finish!` macro boundary included |
| `test_surface_inclusion` | direct consumers and compile-only PocketIC test surface included |
| `audit_tier` | `Tier 2`: generated macro and stable-schema boundaries were inspected |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `2 / 10` after cleanup.
- Cleanup result: six of eight `dead_code` expectations were removed. Five
  test-only functions now compile only for tests, two blob-storage helpers now
  compile only with their owning feature, and the generated finish marker is a
  referenced constant instead of an otherwise-dead function.
- Retained surface: two expectations remain. The auth snapshot field is owned
  by focused round-trip validation while its type name is used by the
  unconditional state contract. The sharding expectation covers schemas that
  are live with the feature enabled and whose names remain in the unconditional
  descriptor registry.

## Step Status

| Step | Status | Evidence Artifact | Comparability Impact |
| ---- | ---- | ---- | ---- |
| STEP 0 | PASS | Run metadata above. | First run is non-comparable. |
| STEP 1 | PASS | Eight source expectations and their direct consumers inventoried. | None. |
| STEP 2 | PASS | Forced no-feature and enabled-feature compiler probes classified hidden warnings. | None. |
| STEP 3 | PASS | Stable descriptor and generated finish-marker owners inspected. | None. |
| STEP 4 | PASS | Test-only and feature-only production shapes identified. | None. |
| STEP 5 | PASS | `finish!` definition, requiring macro, generated consumers, and structural test inspected. | None. |
| STEP 6 | PASS | PocketIC-only helpers and feature-gated schemas classified. | None. |
| STEP 7 | PASS | Six safe removals patched; two retained with explicit owners. | None. |
| STEP 8 | PASS | Changes add no runtime allocation, dispatch, clone, formatting, or encode/decode work. | None. |
| STEP 9 | PASS | Residual risk scored below. | None. |

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| source inventory | `rg -n -C 8 'dead_code' crates canisters apps -g '*.rs'` | Eight expectations found before the patch; two remain afterward. | Terminal output and source diff. |
| consumer check | Focused `rg` searches for the three PocketIC helpers, `FleetActivationOps::snapshot`, `FleetActivation::export`, `AuthStateData`, blob schemas, sharding schemas, and the finish marker. | Test-only, feature-only, descriptor, and generated consumers classified. | Terminal output. |
| hidden warning probe | `RUSTFLAGS='--force-warn dead_code' cargo check -p canic-core --no-default-features` | Confirmed the snapshot/export seam, auth field, two blob helpers, and disabled-feature sharding fields/constants. | Terminal output. |
| enabled-feature probe | `RUSTFLAGS='--force-warn dead_code' cargo check -p canic-core --features blob-storage,blob-storage-billing,sharding --message-format short` | Blob and sharding warnings disappear when their owning features are enabled. | Terminal output. |
| core checks | `cargo check -p canic-core --no-default-features`; `cargo check -p canic-core --features blob-storage,blob-storage-billing,sharding` | PASS. | Terminal output. |
| core lint | `cargo clippy -p canic-core --no-default-features --lib -- -D warnings` | PASS. | Terminal output. |
| testing-support check | `cargo check -p canic-testing-internal` | PASS with test-only imports removed from normal builds. | Terminal output. |
| focused core tests | `cargo test -p canic-core ops::storage::fleet_activation::tests`; `cargo test -p canic-core --features blob-storage,blob-storage-billing,sharding storage::stable::blob_storage::tests` | PASS: 13 Fleet activation and 5 blob-storage tests. | Terminal output. |
| generated marker check | `cargo test -p canic --test protocol_surface missing_finish_marker_stays_actionable`; `cargo test -p canic --features fleet-coordinator-canister --test fleet_coordinator_surface --no-run` | PASS after the matching unit-constant correction. | Terminal output. |
| PocketIC test compile | `cargo test -p canic-testing-internal --lib --no-run` | PASS after the concurrent Fleet Registry rename settled. | Terminal output. |
| changelog governance | `cargo test -p canic --test changelog_governance` | PASS for the new open `0.100.66` draft. | Terminal output. |

## Reachable Surface And Retention Inventory

| Item | Kind | Visibility / cfg | Consumer Evidence | Authority Reason | Surface Class | Owner | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Three removal-journey helpers | private functions | normal build before patch; `cfg(test)` after patch | Called only by two local PocketIC tests. | Test journey only; no production testing-library caller. | `live-test-support` | Fleet Registry PocketIC tests | `MOVE TO TEST` | Low. |
| `FleetActivationOps::snapshot` and stable `export` | crate-private functions | normal build before patch; `cfg(test)` after patch | Called only by unit tests in the owning ops module. | Corruption and round-trip test support. | `live-test-support` | Fleet activation unit tests | `MOVE TO TEST` | Low. |
| Blob key projection and gateway-record constructor | public/internal helpers | available without blob feature before patch; feature-gated after patch | Live when `blob-storage` is enabled; absent-feature compiler was the only dead signal. | Blob stable-storage operations. | `live-authority` | Blob-storage stable schema | `NARROW NOW` | Low. |
| Generated finish marker | macro-generated private function | generated boundary | Required by lifecycle macros through an anonymous compile-time marker reference. | Actionable missing-`finish!` compiler failure and Candid ordering. | `live-generated-boundary` | `canic` lifecycle macros | `PATCH WITH PROOF` | Medium if marker resolution changes. |
| `AuthStateData.record` | public field inside internal stable module | normal build; used under `cfg(test)` | Type contract name is consumed by the unconditional descriptor registry; field is read by focused auth snapshot tests. | Stable-state schema inventory plus round-trip validation. | `live-test-support` and descriptor support | Auth stable storage/state contract | `RETAIN WITH OWNER` | Medium. |
| Sharding schema module | stable schema types and snapshots | normal type/name availability; runtime operations feature-gated | Descriptor registry consumes schema names without the feature; all warned fields/constants are consumed with `sharding`. | Unconditional memory/state inventory for an optional stable-storage feature. | `live-authority` | Sharding stable storage/state contract | `RETAIN WITH OWNER` | Medium. |

## Dead / Stale Candidate Table

| Candidate | Signal | Current Consumers | Confidence | Disposition | Result |
| ---- | ---- | ---- | ---- | ---- | ---- |
| PocketIC removal helpers | Three `cfg_attr(not(test), expect(dead_code))` attributes. | Local `#[test]` functions only. | High | `MOVE TO TEST` | Patched with `cfg(test)` and test-only imports. |
| Fleet activation snapshot seam | Non-test `dead_code` expectation plus forced warning on stable `export`. | Owning unit tests only. | High | `MOVE TO TEST` | Both functions now use `cfg(test)`. |
| Blob-storage module expectation | Module-wide expectation masked two helpers in no-feature builds. | Live blob-storage operations with the feature enabled. | High | `NARROW NOW` | Helpers follow `blob-storage`; broad expectation removed. |
| Finish marker function | Generated function required only as a name-resolution marker. | Lifecycle start macros. | High | `PATCH WITH PROOF` | Replaced with a unit constant and compiled through the generated consumer. |
| Auth snapshot field | Targeted expectation on a field used only in tests. | State descriptor type name plus focused round-trip tests. | Low | `RETAIN WITH OWNER` | Retained; splitting the descriptor name from its canonical type is not justified by this lint-only slice. |
| Sharding schema expectation | Disabled-feature fields/constants trigger warnings. | Enabled-feature stable storage and unconditional descriptors. | Low | `RETAIN WITH OWNER` | Retained; forced feature build proves current consumers. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- |
| Fleet activation | Protected stable record through `FleetActivation`; ops project and validate it. | No. | Test snapshots no longer widen the normal ops surface. | Low. |
| Blob storage | Feature-enabled stable stores and records. | No. | Helper cfgs now match the feature that owns their runtime consumers. | Low. |
| Sharding | Feature-enabled stable registry/lifecycle stores; unconditional state descriptors inventory reserved memory. | No. | Retained schemas are current authority, not compatibility residue. | Medium if split without state-contract proof. |
| Finish marker | Macro expansion name resolution. | No. | Unit constant preserves the same actionable missing-marker boundary without a dead function. | Low after generated compile proof. |

## Facade / Generated Boundary Findings

| Surface | Generated Consumer Evidence | Could Narrow? | Required Replacement | Confidence | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `finish!` marker | Every maintained lifecycle canister expands a start macro that references the marker; the fleet-Coordinator surface compiles after the patch. | Yes, from function to unit constant. | Matching unit-typed anonymous constant in `__canic_require_finish`. | High | `PATCH WITH PROOF` | Macro mismatch produces a compile failure, which the focused feature build caught during the patch. |

## Feature / Diagnostics / Test Surface Findings

| Surface | Feature / cfg | Production Consumer? | Test Consumer? | Action | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| Removal helpers | `cfg(test)` after patch | No | Yes | Exclude from normal testing-support library. | `MOVE TO TEST` |
| Fleet activation snapshot/export | `cfg(test)` after patch | No | Yes | Exclude from normal core library. | `MOVE TO TEST` |
| Blob helpers | `blob-storage` after patch | Yes, when enabled | Yes | Align availability with owning feature. | `NARROW NOW` |
| Auth snapshot field | normal type plus test-only reads | Descriptor name only | Yes | Keep pending a broader state-contract/type-name separation decision. | `RETAIN WITH OWNER` |
| Sharding schemas | schema names unconditional; runtime use under `sharding` | Yes, when enabled | Yes | Keep current descriptor contract. | `RETAIN WITH OWNER` |

## Removal Safety Plan

| Candidate | Action | Hotness | Required Proof | Focused Validation | Raw Wasm Relevant? | Follow-up Trigger |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Test-only helpers and snapshots | Compile only under tests. | `test-only` | Normal library checks plus test compilation. | Core, testing-support and PocketIC no-run checks passed. | No. | None. |
| Blob helpers | Gate with `blob-storage`. | `encode-decode-hot` stable-key support, shape unchanged when enabled | Both disabled- and enabled-feature core checks. | Both passed. | No runtime shape change in enabled builds. | None. |
| Finish marker | Replace function marker with unit constant. | `wasm-sensitive` generated boundary, but compile-time only | Structural marker test and one generated lifecycle consumer compile. | Both boundary checks passed across the two iterations. | No; marker carries no runtime work. | None. |
| Auth snapshot expectation | Keep. | Stable-storage adjacent | State-contract owner decision before separating descriptor strings from types. | Existing auth/state-contract tests if revisited. | Potentially. | Revisit when auth snapshots become a production export or descriptor names gain a dedicated owner. |
| Sharding expectation | Keep. | Stable-memory/wasm-sensitive | State-contract and sharding-feature proof before changing schema availability. | Forced enabled-feature check already proves liveness. | Potentially. | Revisit if state descriptors stop requiring schema types in feature-disabled builds. |

## Runtime Shape / Optimization Risk

| Candidate | Hotness | Runtime Shape Before | Runtime Shape After | Risk Signal | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Test-only helpers/snapshots | `test-only` | Present in normal libraries but unreachable. | Absent from normal libraries. | None. | Compile normal and test surfaces. | `MOVE TO TEST` |
| Blob helpers | `encode-decode-hot` when enabled | Direct methods. | Same direct methods when enabled; absent otherwise. | No allocation or dispatch change. | Enabled-feature check. | `NARROW NOW` |
| Finish marker | `wasm-sensitive` generated boundary | Empty private function used only for name resolution. | Unit constant used only for name resolution. | Compile-time type agreement only. | Generated consumer compile. | `PATCH WITH PROOF` |
| Auth and sharding schemas | stable-storage adjacent | Current schema/type coupling. | Unchanged. | Splitting names from types could weaken schema inventory. | Owner decision and focused stable-state proof. | `RETAIN WITH OWNER` |

## Risk Score

| Bucket | Count | Highest Risk | Notes |
| ---- | ----: | ---- | ---- |
| stale compatibility | 0 | None | No compatibility allowance found. |
| stale generated fallback | 0 | None | Finish marker is a current generated contract. |
| orphaned helper | 0 | None after patch | Test-only and feature-only items were narrowed instead of deleted. |
| overexposed internal | 0 | None after patch | Test seams no longer compile in normal libraries. |
| duplicate surface | 0 | None | No parallel API found. |
| unclear | 0 | None | Both retained expectations have owners and triggers. |
| optimization-risk cleanup | 0 | None | Enabled runtime shapes are unchanged. |

## Verification Readout

- `cargo fmt --all`: PASS after the patch.
- Targeted `rustfmt --edition 2024 --check` over all six changed Rust files:
  PASS. A later workspace-wide format check reported only the concurrent
  lifecycle session's in-progress files.
- `cargo check -p canic-core --no-default-features`: PASS.
- `cargo check -p canic-core --features blob-storage,blob-storage-billing,sharding`: PASS.
- `cargo clippy -p canic-core --no-default-features --lib -- -D warnings`: PASS.
- `cargo check -p canic-testing-internal`: PASS.
- `cargo test -p canic-core ops::storage::fleet_activation::tests`: PASS, 13
  focused tests.
- `cargo test -p canic-core --features blob-storage,blob-storage-billing,sharding storage::stable::blob_storage::tests`:
  PASS, 5 focused tests.
- `cargo test -p canic --test protocol_surface missing_finish_marker_stays_actionable`:
  PASS after the final matching unit-type correction.
- `cargo test -p canic --features fleet-coordinator-canister --test fleet_coordinator_surface --no-run`:
  PASS after the final marker correction. The command reports pre-existing
  control-plane dead-code warnings for a feature combination that excludes the
  root control-plane owner; those warnings are not hidden by allowances and
  are outside this report's candidate inventory.
- `cargo test -p canic-testing-internal --lib --no-run`: PASS after the
  concurrent Fleet Registry rename settled.
- `cargo test -p canic --test changelog_governance`: PASS.
- Raw wasm comparison: not required; enabled runtime logic and data shapes are
  unchanged, and the finish marker is compile-time-only.

## Disposition Summary

| Disposition | Count |
| ---- | ----: |
| `DELETE NOW` | 0 |
| `NARROW NOW` | 1 |
| `INLINE NOW` | 0 |
| `MOVE TO TEST` | 2 |
| `PATCH WITH PROOF` | 1 |
| `RETAIN WITH OWNER` | 2 |
| `MEASURE FIRST` | 0 |
| `BLOCKED` | 0 |

## Follow-up Actions

1. Revisit the two retained expectations only at their named state-contract
   triggers; neither is evidence of stale compatibility today.
