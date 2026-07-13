# Canic Environment-Input Audit - 2026-07-13

## Scope

This audit covers `CANIC_*` environment variables read or written by shipped
Rust product code and its canonical canister-build path. Protocol constants,
generated `CANIC_ROOT`-style shell output, standard Cargo/ICP variables, and
CI-only tool-version inputs are not product configuration and are excluded.

The default is removal. A variable is retained only when it crosses a real
process boundary that cannot consume an existing typed argument.

## Decision Ledger

| Variable | Current role | Decision |
| --- | --- | --- |
| `CANIC_WASM_PROFILE` | Public fallback for build/install/deploy profile selection and maintainer builds. | REMOVED. Commands use typed `--profile`; the low-level builder and artifact script take explicit profile arguments. |
| `CANIC_KEEP_WASM_BUILD_CACHE` | Public switch that disables install-exit cache cleanup. | REMOVED. Install and workspace-test cleanup is deterministic; no retention alias or replacement switch remains. |
| `CANIC_WASM_TARGET_DIR` | Canic-specific Cargo target override. | REMOVED. Default Canic targets remain bounded, while advanced Cargo callers use standard `CARGO_TARGET_DIR`. |
| `CANIC_WORKSPACE_ROOT` | Public workspace discovery override and child-build handoff. | REMOVED. Explicit command inputs and canonical Cargo discovery own the workspace; Cargo children did not need this value. |
| `CANIC_ICP_ROOT` | Public ICP project override and child-build/bootstrap handoff. | REMOVED. Project discovery owns operator selection; required Cargo handoff is renamed `CANIC_INTERNAL_BUILD_ICP_ROOT`. |
| `CANIC_CONFIG_PATH` | Public config-selection override, build-script input, and emitted compile-time config path. | REMOVED. Commands and low-level builders pass config paths explicitly; Cargo handoff is `CANIC_INTERNAL_BUILD_CONFIG_PATH`, and compiled origin metadata is `CANIC_CONFIG_ORIGIN_PATH`. |
| `CANIC_CANISTERS_ROOT` | Public override for canister package search. | REMOVED. The selected config and Cargo role metadata identify packages. |
| `CANIC_WORKSPACE_MANIFEST_PATH` | Public workspace-manifest discovery hint. | REMOVED. Canonical Cargo discovery owns the workspace manifest. |
| `CANIC_ROOT_MANIFEST_PATH` | Public root-package manifest override. | REMOVED. Root role metadata resolution owns package selection. |
| `CANIC_REFRESH_WASM_STORE_DID` | Maintainer-only switch for refreshing generated Wasm-store Candid. | REMOVED. The low-level artifact builder accepts the explicit `--refresh-wasm-store-did` argument only for `wasm_store`. |
| `CANIC_ICP_LOCAL_NETWORK_URL` | Child-build environment write for local replica context. | REMOVED. No production reader existed; typed local-replica context remains available to the install operations that consume it. |
| `CANIC_ICP_LOCAL_ROOT_KEY` | Child-build environment write for local replica root key. | REMOVED. No production reader existed; typed local-replica context remains available to the install operations that consume it. |
| `CANIC_ICP_BUILD_ENVIRONMENT` | Value removed from child commands but never read or set by maintained production code. | REMOVED. The stale cleanup and its test assertion are deleted. |
| `CANIC_ROLE_CONTRACT_VALIDATED` | Private marker proving the canonical role-package validator ran before a Wasm Cargo build. | KEEP INTERNAL. Cargo build scripts require a process-boundary marker; this is not an operator shortcut. |
| `CANIC_INTERNAL_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS` | Private child-build requirement for canonical embedded release artifacts. | KEEP INTERNAL. One core constant owns the name and Canic-owned child commands set it only across the required Cargo build boundary. |
| `CANIC_INTERNAL_TEST_ENDPOINTS` | Test-only build switch for fixture endpoint generation. | KEEP TEST-INTERNAL. It is not a shipped operator surface. |
| `CANIC_CONFIG_ORIGIN_PATH`, `CANIC_CONFIG_MODEL_PATH`, `CANIC_CONFIG_SOURCE_PATH`, `CANIC_CANISTER_ROLE`, `CANIC_ROOT_WASM_STORE_BOOTSTRAP_RELEASE_SET_PATH` | Compile-time values emitted by build scripts and consumed by macros through `env!`. | KEEP INTERNAL. These are generated compiler inputs, not ambient runtime configuration. |

## Hard-Cut Rules

- Removed variables receive no alias, deprecated fallback, compatibility
  reader, or hidden precedence path.
- Documentation, help, Make targets, scripts, examples, and tests move to the
  maintained explicit input in the same batch.
- Internal variables are set only by Canic-owned child commands or Cargo build
  scripts and are not documented as operator controls.
- If implementation finds no reader for an internal write, the write is
  deleted rather than documented.
- Removing a shortcut must not create a generic configuration object or a
  replacement environment variable with a different name.

## Implementation Order

1. Delete the three environment writes/cleanup paths with no production reader. COMPLETE.
2. Remove public profile selection in favor of existing typed inputs. COMPLETE.
3. Remove public path and manifest overrides, then replace unsafe environment
   tests with direct discovery inputs. COMPLETE.
4. Remove cache-retention, target-directory, and Candid-refresh shortcuts in
   favor of existing typed inputs or explicit maintainer commands. COMPLETE.
5. Internalize only the canonical Cargo/build-script handoff values that remain
   necessary after the removals. COMPLETE.
6. Re-scan source, help, scripts, examples, and active documentation before
   closing Slice C. COMPLETE.
