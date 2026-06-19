# MSH Module Cleanup: canic-cli support candid

## Preamble

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
| `comparability_status` | `non-comparable`: first targeted MSH run for this module |
| `code_snapshot` | current working tree |
| `in_scope_roots` | `crates/canic-cli/src/support/candid.rs` |
| `excluded_roots` | host ICP path construction, registry parsing, CLI consumers, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused support candid unit tests |
| `audit_tier` | `Tier 0` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `1 / 10`.
- Tier: `Tier 0`, because this module is a cold CLI adapter over host Candid
  sidecar path resolution with no mutation, generated-boundary, storage,
  recovery, or runtime authority.
- Cleanup result: retained the production helper surface and added focused unit
  tests for the adapter contract.

The module remains a passive adapter between CLI registry/project inputs and
`canic-host` local Candid sidecar discovery. It does not own path layout
authority, registry parsing, ICP command invocation, file mutation, or
deployment truth.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,200p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules reviewed for this cleanup slice | terminal output |
| target inventory | `wc -l crates/canic-cli/src/support/*.rs` | PASS: `support/candid.rs` is the smallest support helper and has two production functions | terminal output |
| consumer scan | `rg -n "role_candid_path|registry_entry_candid_path" crates/canic-cli/src -g '*.rs'` | PASS: helpers are used by backup/create, cycles, list, medic, metrics, and snapshot flows | terminal output |
| owner lookup | `rg -n "fn existing_local_canister_candid_path|struct RegistryEntry" crates -g '*.rs'` | PASS: path layout and registry record shape are owned by `canic-host` | terminal output |
| focused tests | `cargo test --locked -p canic-cli support::candid -- --nocapture` | PASS: 3 focused tests passed | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| Import order | `NARROW STYLE` | Moved external `canic_host` import below `std` import to match source ordering policy. | fmt check; clippy |
| Candid sidecar adapter tests | `ADD PROOF` | Tests pin the intended `None` behavior for absent roots and missing registry roles, and the existing-sidecar behavior for a local role. | focused tests; package check; clippy |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `role_candid_path` | `canic-cli::support::candid` | CLI consumers need a shared optional-project-root adapter over host sidecar lookup. | Revisit if CLI callers stop accepting optional local ICP roots. |
| `registry_entry_candid_path` | `canic-cli::support::candid` | CLI consumers need a shared registry-entry adapter that requires an explicit role before looking up a sidecar. | Revisit if registry entries gain a stronger typed role model. |
| Host path layout | `canic-host::icp` | `local_canister_candid_path` and `existing_local_canister_candid_path` own the `.icp/<network>/canisters/<role>/<role>.did` contract. | Revisit only with a host ICP artifact layout change. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Inlining helper calls into consumers | Multiple CLI flows share the same optional-root and missing-role semantics. Inlining would duplicate the same adapter behavior across command modules. | Dedicated CLI consumer pass showing all call sites can own those semantics locally. |

## Verification

- `cargo test --locked -p canic-cli support::candid -- --nocapture`: PASS, 3
  focused tests passed.
- `cargo fmt --all -- --check`: PASS.
- `git diff --check`: PASS.
- `cargo check --locked -p canic-cli`: PASS.
- `cargo clippy --locked -p canic-cli --all-targets --all-features -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; CLI-only adapter and tests.
