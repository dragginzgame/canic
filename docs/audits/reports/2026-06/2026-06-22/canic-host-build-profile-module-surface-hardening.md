# Module Surface Hardening: canic-host build_profile

## Verdict

- Status: `PASS`.
- Risk score: `2 / 10`.
- Tier: `Tier 1`.
- Patch mode: `implementation-requested`.
- Cleanup result: no source changes; all exposed build-profile surface is live
  for canister build/install/deploy selection.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| target inventory | `wc -l crates/canic-host/src/build_profile.rs`; `sed -n '1,220p' crates/canic-host/src/build_profile.rs` | PASS: single 59-line file module | terminal output |
| surface scan | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/build_profile.rs` | PASS: public enum, public `current`, crate-visible `cargo_args`, public `target_dir_name`, and `FromStr` identified; no stale markers or lint suppressions found | terminal output |
| consumer check | `rg` for `CanisterBuildProfile::current`, `cargo_args`, `target_dir_name`, and `CanisterBuildProfile` across host/CLI/backup | PASS: CLI build/install/deploy parse and render the public type; host build/bootstrap paths consume cargo args and target profile names | terminal output |
| focused tests | `cargo test --locked -p canic-cli profile -- --nocapture` | PASS: 9 profile-filtered CLI tests passed | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | PASS | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| None | N/A | No dead, stale, or overexposed build-profile surface was found. | Focused profile tests and host clippy passed. |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `CanisterBuildProfile` | `canic-host::canister_build` facade | Public CLI build/install/deploy options parse this type and pass it into host build/install paths. | If build profile parsing moves fully into `canic-cli` and host accepts a narrower internal enum. |
| `CanisterBuildProfile::current` | `build_profile` | Resolves `CANIC_WASM_PROFILE` defaulting for CLI and install-root build paths. | If profile defaulting becomes command-specific. |
| `CanisterBuildProfile::cargo_args` | `build_profile` | Host build/bootstrap code maps Canic profiles to Cargo profile flags. | If Cargo build invocation is centralized behind a different profile adapter. |
| `CanisterBuildProfile::target_dir_name` | `build_profile` | CLI provenance/argv rendering and host artifact path resolution need the target directory profile label. | If target artifact layout stops depending on Cargo profile directory names. |
| `FromStr for CanisterBuildProfile` | `build_profile` | Clap value parsing uses the public `FromStr` implementation for `--profile`. | If command parsing switches to a CLI-local parser. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Narrowing the public enum or methods | `CanisterBuildProfile` is re-exported through `canister_build` and consumed by `canic-cli` options, examples, and host install/build paths. | CLI parser migration plus build/install/deploy profile validation. |
| Removing `fast` profile support | `fast` is part of the current operator-facing profile set and artifact path/provenance behavior. | Owner decision and release/build compatibility proof. |

## Verification

- `cargo test --locked -p canic-cli profile -- --nocapture`: PASS, 9 profile-filtered tests passed.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS.
- `git diff --check`: PASS.
- trailing whitespace scan over touched build-profile report and summary files: PASS.
- wasm/raw-size check: not applicable; audit retained existing host build-profile surface with no source change.
