# canic-wasm-store Code Hygiene Audit

- Definition path: `docs/audits/modular/code-hygiene.md`
- Scope: `crates/canic-wasm-store/**`, plus direct macro, build, protocol, host wrapper, and test call sites needed to classify behavior.
- Compared baseline report path: N/A
- Code snapshot identifier: `1366604d3b653ed3695046ed1689f141b18edbbd` (`7f5b989c9e78a445e00258a597972e16caa48c1d` tree)
- Method tag/version: `modular-code-hygiene-current`
- Comparability status: non-comparable: first targeted modular code-hygiene audit for this crate
- Exclusions applied: historical changelog/design mentions except where they identify active ownership or validation paths
- Notable methodology changes vs baseline: none

## Verdict

- Is this module bloated? No. The Rust source is a two-macro canonical role wrapper plus an 8-line build script.
- Boundary cleanliness rating: 9/10.
- Current non-test Rust LoC: 10.
- Current package/artifact surface LoC: 412.
- Realistic target non-test Rust LoC: 10.
- Highest-value deletion: none in `crates/canic-wasm-store`; deletion would remove externally meaningful wrapper behavior.
- Highest-risk deletion: either `canic::finish!()` in `src/lib.rs` or `wasm_store.did`.
- First safe cleanup pass: no code deletion. Keep this crate intentionally frozen and use validation-only checks after runtime/protocol changes.

The only hygiene pressure is not local code bloat. It is the intentional broad dependency on the `canic` facade with `control-plane` enabled in `Cargo.toml`, plus the checked-in `wasm_store.did` being a noisy protocol artifact. Those are ownership tradeoffs, not deletion candidates in this crate.

## Size Map

| File | Raw LoC | Non-test LoC | Test LoC | Main responsibility | Initial verdict |
| --- | ---: | ---: | ---: | --- | --- |
| `crates/canic-wasm-store/src/lib.rs` | 2 | 2 | 0 | endpoint/lifecycle integration for canonical store role | KEEP |
| `crates/canic-wasm-store/build.rs` | 8 | 8 | 0 | build-time Canic config embedding | KEEP |
| `crates/canic-wasm-store/Cargo.toml` | 32 | 32 | 0 | package/dependency/public crate metadata | KEEP |
| `crates/canic-wasm-store/README.md` | 41 | 41 | 0 | published package role and DID ownership docs | KEEP |
| `crates/canic-wasm-store/canic.toml` | 16 | 16 | 0 | default build config for canonical role crate | KEEP |
| `crates/canic-wasm-store/wasm_store.did` | 313 | 313 | 0 | checked-in canonical Candid contract | KEEP |

| Category | LoC |
| --- | ---: |
| Non-test Rust code | 10 |
| Inline tests | 0 |
| Total package files inspected | 412 |

## Behavior Contract

- `src/lib.rs` must invoke `canic::start_wasm_store!()` so the published crate exports the canonical subnet-local `wasm_store` lifecycle and endpoint bundle.
- `src/lib.rs` must invoke `canic::finish!()` after the start macro so Candid export/finalization remains valid.
- `build.rs` must run `canic::build!(config_path)` and allow `CANIC_CONFIG_PATH` override while defaulting to `canic.toml`.
- `Cargo.toml` must publish a `cdylib`/`rlib` crate named `canister_wasm_store` with the `canic` control-plane dependency needed by the start macro.
- `wasm_store.did` must stay the checked-in canonical interface, including protected update methods with `CanicInternalCallEnvelopeV1`, structural query exceptions, cycle tracker, memory ledger diagnostic, and no removed compatibility cycles-accept methods.
- `README.md` must continue to state that this is a narrow canonical role crate, not a general application facade.

## Rent Table

| Item | Classification | Rationale | Recommended action |
| --- | --- | --- | --- |
| `src/lib.rs` module | KEEP | Sole published source for the canonical role crate; delegates behavior to `canic::start_wasm_store!()` and finalizes Candid export. | Keep unchanged. |
| `canic::start_wasm_store!()` invocation | KEEP | Binds the crate to `CanisterRole::WASM_STORE`, lifecycle hooks, inspect-message hook, and the canonical wasm-store endpoint bundle owned by `canic`. | Keep unchanged. |
| `canic::finish!()` invocation | KEEP | Required by the lifecycle macro contract and debug Candid export flow in `crates/canic/src/macros/start.rs`. | Keep unchanged. |
| `build.rs::main` | KEEP | Embeds Canic config and supports `CANIC_CONFIG_PATH`; this is the only build-time behavior in the crate. | Keep unchanged. |
| `Cargo.toml` package metadata | KEEP | Public package identity and role-specific crate posture are part of crates.io behavior. | Keep unchanged. |
| `Cargo.toml` dependencies `canic`, `candid`, `ic-cdk` | KEEP | Existing manifest explicitly ignores `candid`/`ic-cdk` for machete, and macro-expanded canister surfaces rely on the published facade/toolchain shape. | Keep unless a separate dependency audit proves removable. |
| `canic.toml` | KEEP | Default config used when `CANIC_CONFIG_PATH` is unset. | Keep unchanged. |
| `README.md` | KEEP | Prevents this published crate from reading like a general Canic entry surface. | Keep unchanged. |
| `wasm_store.did` | KEEP | Canonical checked-in protocol artifact tested by `crates/canic/tests/protocol_surface.rs` and `crates/canic-core/tests/protected_internal_call_guard.rs`. | Keep; refresh only intentionally with `CANIC_REFRESH_WASM_STORE_DID=1`. |

## Duplications

| Duplicated logic | Locations | Risk | Recommended action |
| --- | --- | --- | --- |
| Two-line wrapper source | `crates/canic-wasm-store/src/lib.rs`; generated fallback wrapper in `crates/canic-host/src/bootstrap_store.rs` | Low. One is the canonical published crate; the other is a downstream fallback when only `canic` is available. | Keep duplication because contexts differ. |
| Build script shape | `crates/canic-wasm-store/build.rs`; generated fallback `build.rs` in `crates/canic-host/src/bootstrap_store.rs` | Low. Canonical crate has default `canic.toml`; generated fallback requires explicit `CANIC_CONFIG_PATH`. | Keep duplication because behavior differs. |
| DID contract checks | `crates/canic/tests/protocol_surface.rs`; `crates/canic-core/tests/protected_internal_call_guard.rs` | Low. Tests cover different surfaces: public protocol re-export and protected internal-call guardrails. | Keep both. |

## Dead / Low-Value Code

No dead or low-value Rust code was found under `crates/canic-wasm-store`.

Searches for public items, stale markers, wrapper residue, registry/state structures, and local helper functions found only `build.rs::main` in the target crate. There are no local DTOs, errors, storage helpers, endpoint functions, tests, compatibility shims, or one-use wrappers to delete.

## Error Model

This crate defines no error types. Build-time failures are delegated to `canic::build!`; runtime endpoint errors are generated by the `canic` facade and lower runtime crates.

No error cleanup is recommended inside `crates/canic-wasm-store`.

## Test Cleanup

| Test | What it asserts | Keep/rewrite/delete | Reason |
| --- | --- | --- | --- |
| `crates/canic/tests/protocol_surface.rs::removed_cycles_accept_surface_stays_absent` | Removed cycles-accept compatibility methods remain absent from checked-in DID. | KEEP | Externally meaningful protocol regression guard. |
| `crates/canic/tests/protocol_surface.rs::wasm_store_exposes_standard_cycle_tracker` | Canonical DID includes cycle tracker queries. | KEEP | Role contract behavior. |
| `crates/canic/tests/protocol_surface.rs::wasm_store_exposes_ledger_but_not_registry_memory_diagnostics` | Canonical DID includes ledger diagnostic and excludes removed live registry. | KEEP | Role contract behavior. |
| `crates/canic/tests/protocol_surface.rs::wasm_store_canonical_did_parses` | Checked-in DID is syntactically valid Candid and contains the ledger query. | KEEP | Public artifact validity. |
| `crates/canic/tests/protocol_surface.rs::public_protocol_reexports_wasm_store_protection_manifest` | Public `canic::protocol` manifest matches `canic-core`. | KEEP | Public facade contract. |
| `crates/canic-core/tests/protected_internal_call_guard.rs::wasm_store_macro_declarations_match_protected_method_manifest` | Macro-declared protected and structural query methods match protocol manifests. | KEEP | Protects generated endpoint shape. |
| `crates/canic-core/tests/protected_internal_call_guard.rs::wasm_store_did_matches_protected_method_manifest` | Checked-in DID matches protected-method/query manifests and ABI shape. | KEEP | Protects canonical external contract. |
| `scripts/ci/verify-packaged-downstream-wasm-store.sh` probe | Downstream wrapper can build store artifacts without packaged `canic-wasm-store`. | KEEP | Protects published downstream build behavior. |

No test currently preserves removable `canic-wasm-store` scaffolding.

## Boundary Recommendations

This crate should own:

- the published source identity for the canonical `wasm_store` canister;
- the minimal build script needed to embed config;
- the checked-in canonical `wasm_store.did`;
- README guidance for the role-specific package.

This crate should not own:

- wasm-store endpoint implementations;
- protected internal-call protocol policy;
- root/store publication workflow;
- storage schemas, DTOs, or error mapping;
- downstream wrapper synthesis.

Code that belongs elsewhere already lives elsewhere:

- endpoint generation and lifecycle behavior in `crates/canic/src/macros/start.rs` and endpoint macros;
- protected method/query manifests in `canic-core::protocol`;
- downstream wrapper fallback in `crates/canic-host/src/bootstrap_store.rs`;
- behavior/integration tests in `crates/canic` and `crates/canic-core`.

## Footprint Target

| Area | Current non-test LoC | Realistic target | How to get there |
| --- | ---: | ---: | --- |
| Rust wrapper source | 2 | 2 | No cleanup available without deleting role behavior. |
| Build script | 8 | 8 | No cleanup available; the default/override config path behavior is minimal. |
| Package metadata | 32 | 32 | Keep explicit package/dependency posture. |
| Canonical DID | 313 | 313 | Do not hand-shrink; only generated/protocol-driven refreshes should change it. |

## Proposed Cleanup Plan

- Phase 1: No code deletion. Record this crate as intentionally minimal and use targeted validation after runtime/protocol changes.
- Phase 2: If dependency hygiene later proves `candid` or `ic-cdk` removable from the manifest despite macro expansion and Candid export, remove only that manifest entry and rerun package/downstream checks.
- Phase 3: If the broad `canic` facade dependency becomes a real packaging problem, evaluate a separate facade/build-support split outside this crate. Do not start that from `canic-wasm-store`.

## Implementation Prompt for Phase 1

```text
You are working in:

    crates/canic-wasm-store

Implement Phase 1 from the module code-hygiene audit.

This is a validation-only cleanup pass. Do not edit Rust code unless validation
finds a concrete stale artifact.

Scope:

- crates/canic-wasm-store/src/lib.rs
- crates/canic-wasm-store/build.rs
- crates/canic-wasm-store/Cargo.toml
- crates/canic-wasm-store/canic.toml
- crates/canic-wasm-store/README.md
- crates/canic-wasm-store/wasm_store.did
- crates/canic/tests/protocol_surface.rs
- crates/canic-core/tests/protected_internal_call_guard.rs

Goal:

- confirm the canonical wasm_store wrapper remains minimal and aligned with the
  protected-method/query manifest;
- do not delete the two macro calls, build script, default config, or DID.

Preserve this behavior contract:

- published crate builds the canonical wasm_store canister;
- start_wasm_store and finish stay paired in src/lib.rs;
- build.rs supports CANIC_CONFIG_PATH and defaults to canic.toml;
- wasm_store.did remains the checked-in canonical Candid contract.

Do not touch in this pass:

- canic macro implementations;
- control-plane workflow;
- host generated wrapper synthesis;
- protocol manifests unless a failing test proves drift.

Expected cleanup:

- none by default;
- only refresh wasm_store.did if the manifest/DID guard fails and the protocol
  change is intentional.

Validation:

- cargo fmt --all --check
- cargo check -p canic-wasm-store
- cargo test -p canic protocol_surface --test protocol_surface
- cargo test -p canic-core --test protected_internal_call_guard wasm_store
- cargo clippy -p canic-wasm-store --all-targets -- -D warnings
- git diff --check

Output:

1. Validation results.
2. Whether any artifact drift was found.
3. Any deferred dependency-hygiene question.
```

## Validation Commands for a Future Implementation Pass

```bash
cargo fmt --all --check
cargo check -p canic-wasm-store
cargo test -p canic protocol_surface --test protocol_surface
cargo test -p canic-core --test protected_internal_call_guard wasm_store
cargo clippy -p canic-wasm-store --all-targets -- -D warnings
git diff --check
```

## Validation Run

| Command | Result | Notes |
| --- | --- | --- |
| `cargo check -p canic-wasm-store` | PASS | Finished `dev` profile in 16.05s. |
