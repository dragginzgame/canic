# Module Surface Hardening: canic-host workspace_discovery

## Verdict

- Status: `PASS`.
- Risk score: `2 / 10`.
- Tier: `Tier 1`.
- Patch mode: `implementation-requested`.
- Cleanup result: no final source patch; attempted helper visibility narrowing
  was rejected by existing clippy policy because `workspace_discovery` is
  already a private module.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| target inventory | `find crates/canic-host/src/workspace_discovery -type f -name '*.rs'`; `wc -l crates/canic-host/src/workspace_discovery/mod.rs` | PASS: single 105-line private host module | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/workspace_discovery -g '*.rs'` | PASS: four private-module public helpers identified; no stale markers or lint suppressions found | terminal output |
| consumer check | focused `rg` searches for `discover_workspace_root_from`, `discover_icp_root_from`, `normalize_workspace_path`, and `resolve_canister_manifest_from_metadata_under` | PASS: all helpers have current same-crate consumers in release-set paths, ICP config, install-root config selection, or canister manifest resolution | terminal output |
| rejected cleanup attempt | changed the four helpers to `pub(super)` and ran clippy | REJECTED: `cargo clippy --locked -p canic-host --all-targets -- -D warnings` flagged `clippy::redundant_pub_crate`; patch was reverted | terminal output |
| focused tests | `cargo test --locked -p canic-host release_set::tests::paths -- --nocapture`; `cargo test --locked -p canic-host icp_config -- --nocapture`; `cargo test --locked -p canic-host workspace -- --nocapture` | PASS: release-set path, ICP config, and workspace-filtered callers passed | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings` after revert | PASS | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| None | N/A | No safe final source cleanup remained after clippy rejected explicit `pub(super)` narrowing in a private module. | Final host clippy passed after reverting the attempted narrowing. |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `discover_workspace_root_from` | `workspace_discovery` | Release-set path resolution uses it to find the downstream Cargo workspace from env/config/current-directory hints. | If workspace-root resolution moves fully into release-set path ownership. |
| `discover_icp_root_from` | `workspace_discovery` | ICP config and release-set path resolution use it to find the nearest ICP CLI project root. | If ICP root discovery is centralized elsewhere. |
| `normalize_workspace_path` | `workspace_discovery` | Release-set and install-root config selection normalize env/option paths against the chosen workspace root. | If all callers switch to a path newtype or owner-specific normalizer. |
| `resolve_canister_manifest_from_metadata_under` | `workspace_discovery` | Release-set manifest paths use Cargo metadata to resolve exactly one role manifest below the canister root. | If canister manifest discovery moves into a dedicated Cargo metadata owner. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Replacing `pub` with `pub(super)` | Clippy treats `pub(super)`/crate-level visibility inside this private module as redundant and requires `pub`. | A broader module visibility refactor or clippy policy change. |
| Moving manifest discovery out of `workspace_discovery` | It is live release-set path authority and depends on cached Cargo metadata. | Owner decision plus release-set path and build/install validation. |

## Verification

- `cargo fmt --all`: PASS.
- `cargo test --locked -p canic-host release_set::tests::paths -- --nocapture`: PASS, 9 release-set path tests passed.
- `cargo test --locked -p canic-host icp_config -- --nocapture`: PASS, 10 ICP config tests passed.
- `cargo test --locked -p canic-host workspace -- --nocapture`: PASS, 4 workspace-filtered tests passed.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS after reverting the rejected visibility-only patch.
- wasm/raw-size check: not applicable; no final production source change and no runtime wasm payload change.
