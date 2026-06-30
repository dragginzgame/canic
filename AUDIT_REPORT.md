# Canic Repository Audit Report

Date of audit: 2026-06-30

## Executive Summary

This is a running repo-wide correctness and cleanup audit for the Canic
workspace. The audit started from a clean working tree at `0.76.6` after the
delegated-auth hard-cut cleanup. The current priority is correctness,
maintainability, stale-code removal, consistency, and confidence-building, not
new product features or broad optimization.

Baseline and final non-PocketIC gates are healthy. Workspace clippy passed with
all targets/features, the root protocol-surface test passed, and the core auth
unit subset passed. A concurrent maintainer-owned `make publish` /
`cargo publish` process was observed in another terminal while baseline checks
ran; heavy Cargo gates were staged carefully to avoid interfering with publish
locks.

## Project Summary

Canic is a Rust workspace for Internet Computer canister orchestration. It
contains:

- public facade and endpoint macro crates;
- shared canister runtime/core logic;
- host/operator CLI and install/build libraries;
- backup/restore libraries;
- control-plane and wasm-store canister crates;
- PocketIC/integration test harnesses;
- audit/test canisters and local fleet fixtures;
- operations, architecture, contract, governance, and release docs.

## Tooling Detected

| Area | Tooling |
| --- | --- |
| Language | Rust 2024 |
| Toolchain | `rust-toolchain.toml` pins Rust `1.96.0` with `rustfmt` and `clippy` |
| Package/build | Cargo workspace, `Makefile` wrappers |
| Runtime target | Internet Computer canisters, `ic-cdk`, `ic-cdk-management-canister` |
| Integration tests | PocketIC via `canic-tests` / `canic-testing-internal` |
| CLI | `clap`, published `canic` binary from `canic-cli` |
| Serialization | Candid, CBOR/serde, stable record DTOs |
| Crypto/auth | IC canister signatures, IC threshold ECDSA, `k256`, IC signature verification |
| Formatting | `cargo fmt --all`; Makefile also expects `cargo sort` and `cargo sort-derives` |
| Linting | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| Docs/governance | `AGENTS.md`, `docs/governance/ci-deployment.md`, `docs/governance/changelog.md` |
| Release/deploy | Human-owned Make targets and `scripts/ci/publish-workspace.sh` |

## Generated / Cache / Vendor Exclusions

The following are generated, cache, build, local runtime, backup, or external
state and should not be manually audited except for high-level hygiene:

- `.git/`
- `target/`
- `artifacts/`
- `.tmp/`
- `.icp/`
- `.canic/`
- `backups/`
- local VS Code / editor state under `.cursor/` and `.agents/`

Repo-wide file count outside common generated/cache paths at audit start:
`4525` files.

## Commands Run

| Phase | Command | Result | Notes |
| --- | --- | --- | --- |
| Recon | `sed -n '1,240p' docs/status/current.md` | Pass | Read current handoff first. |
| Recon | `git status --short` | Pass | Clean working tree at audit start. |
| Recon | `find . -maxdepth 2 ...` | Pass | Identified root config/docs/tooling files. |
| Recon | `find . -maxdepth 2 -type d \| sort` | Pass | Identified top-level source/generated areas. |
| Recon | `sed -n '1,220p' README.md` | Pass | Read project overview and repository layout. |
| Recon | `sed -n '1,240p' Cargo.toml` | Pass | Read workspace members, versions, dependencies, lints. |
| Recon | `sed -n '1,280p' Makefile` | Pass | Read quality, test, release, and install targets. |
| Recon | `sed -n '1,220p' docs/governance/ci-deployment.md` | Pass | Confirmed command and git/release boundaries. |
| Recon | `find crates canisters fleets -maxdepth 3 -name Cargo.toml \| sort` | Pass | Found 42 workspace manifests. |
| Recon | `find docs -maxdepth 2 -type f \| sort` | Pass | Mapped active docs/audits/changelogs. |
| Recon | `find scripts -maxdepth 2 -type f \| sort` | Pass | Mapped CI/dev scripts. |
| Baseline | `cargo fmt --all -- --check` | Pass | Formatting clean before edits. |
| Baseline | `cargo check --locked --workspace` | Pass | Workspace compile succeeded in about 1m39s. Build initially waited on another Cargo lock. |
| Baseline support | `ps -eo pid,ppid,stat,etime,cmd` | Pass | Found concurrent maintainer `make publish` / `cargo publish` process. |
| Audit | `rg -n "RootProof|IcCanisterSignatureV1|IcChainKeyBatchSignatureV1|sign_with_ecdsa|canic_prepare_delegated_token|canic_get_delegated_token|canic_get_or_create_chain_key_delegation_proof|RoleAttestationRootProof" crates/canic-core/src crates/canic/src crates/canic-cli/src -g '*.rs'` | Pass | Delegated `RootProof` is chain-key-only; role-attestation still uses its separate canister-signature proof enum. |
| Audit | `rg -n "canic auth renewal|renewal run-once|provisioner|canic_get_delegation_renewal_proof_batch|canic_install_delegation_proof_batch|delegation_renewal_work|bridge" crates scripts docs README.md -g '!target/**'` | Pass | Active hits were docs/tests proving removed bridge surfaces stay absent, archive/changelog history, or non-auth "bridge" wording. |
| Audit | `rg -n "canic_prepare_delegation_proof_batch|canic_get_delegation_proof_batch|canic_install_delegation_proof_batch|canic_upsert_delegation_renewal_provisioner|canic_delegation_renewal_provisioners|canic_delegation_renewal_work|canic_get_delegation_renewal_proof_batch|is_delegation_renewal_provisioner|auth renewal run-once|auth renewal provisioner" README.md docs/status docs/architecture docs/contracts docs/operations docs/audits/recurring crates/canic/src crates/canic-core/src crates/canic-cli/src scripts/ci -g '!**/archive/**' -g '!**/reports/**' -g '!docs/changelog/**'` | Pass | Remaining hits are deleted-command status notes and tests that assert old endpoints are absent. |
| Audit | `rg -n "RootProof::IcCanisterSignatureV1|RootProofMode::IcCanisterSignature|RootProofRecord::IcCanisterSignatureV1|IcCanisterSignatureProofRecord|LegacyRootProofRejected|Root prepares canister-signature|stable snapshot decode records" README.md docs/status docs/architecture docs/contracts docs/operations docs/audits/recurring crates/canic-core/src crates/canic/src crates/canic-cli/src -g '!**/archive/**' -g '!**/reports/**' -g '!docs/changelog/**'` | Pass | Remaining code hits are `RoleAttestationRootProof::IcCanisterSignatureV1`, not delegated `RootProof`. |
| Audit | `rg -n "#\\[expect\\(dead_code\\)|#\\[allow\\(dead_code\\)|#\\[expect\\(unused_imports\\)|#\\[allow\\(unused_imports\\)|todo!\\(|unimplemented!\\(|TODO|FIXME|HACK|XXX|temporary|workaround|deprecated|legacy" crates canisters fleets -g '*.rs'` | Pass | No high-confidence production cleanup found; remaining hits are intentional marker/re-export/reserved stable ID, tests, user-facing legacy warnings, or sandbox/test wording. |
| Audit | `rg -n "\\.only\\(|\\.skip\\(|#\\[ignore\\]|console\\.log|debugger|dbg!\\(|println!\\(" crates canisters fleets scripts -g '!target/**'` | Pass | No focused/skipped Rust tests or debug macros found; hits are build-script/user-facing output, test harness progress logs, and iterator `.skip`. |
| Final | `git diff --check` | Pass | No whitespace errors. |
| Final | `cargo fmt --all -- --check` | Pass | Formatting still clean after docs edits. |
| Final | `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | Pass | Completed in about 2m06s; only build-script artifact warnings. |
| Final | `cargo test --locked -p canic --test protocol_surface` | Pass | 17 passed; includes root delegation surface and removed bridge endpoint assertions. |
| Final | `cargo test --locked -p canic-core auth --lib` | Pass | 250 passed, 0 failed, 0 ignored; 468 filtered out. |
| Final | `git diff -U0 | rg -n "(?i)(api[_-]?key|secret|password|token|private[_-]?key|BEGIN [A-Z ]*PRIVATE KEY)"` | Pass | Hits are auth/token terminology in doc diffs; no secrets/private keys. |
| Final | `find . -maxdepth 2 -name '*tmp*' -o -name '*temp*' -o -name '*.bak' -o -name '*.orig'` | Pass | Only existing `.tmp` and `target/tmp`. |

## Current Baseline Status

- Working tree was clean at audit start.
- Baseline formatting and workspace compile passed before edits.
- Final workspace clippy, targeted protocol-surface tests, and targeted core
  auth tests passed after edits.
- No production Rust code was changed in this pass.
- The prior maintainer instruction not to run `make test` was preserved. Heavy
  PocketIC suites were not started.

## Initial Warnings

- A maintainer-owned `make publish` process was running during early baseline
  verification. It was not killed or disrupted.
- `make test` is PocketIC-heavy and was previously called out as timing out in
  this workspace.
- Auth is in a sensitive hard-cut state. Changes must preserve the chain-key
  delegated root proof invariant and keep role-attestation canister-signature
  proof material separate from delegated-token `RootProof`.

## Module Inventory

| Area | Purpose | Key files / dirs | External deps | Internal deps | Public interfaces | Health | Actions taken | Remaining concerns |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Root workspace | Cargo workspace, Make targets, governance | `Cargo.toml`, `Makefile`, `rust-toolchain.toml`, `AGENTS.md` | Cargo, Rust 1.96, Make | all crates | workspace members, lint profile, quality targets | OK | Read and recorded; workspace check/clippy passed | Full PocketIC suite intentionally deferred. |
| `crates/canic` | Public facade crate, endpoint bundles, macros, protocol constants | `src/`, `src/macros/endpoints/`, `tests/` | `canic-core`, macros | core DTO/API/runtime | public crate exports, endpoint macro bundles | OK | Verified root/nonroot auth endpoint wiring and protocol-surface tests | Keep protocol-surface tests as release gates for auth endpoint removals. |
| `crates/canic-core/src/dto` | Boundary DTO contracts | `dto/auth.rs`, `dto/*` | Candid, serde | passive data only | `RootProof`, `DelegationProof`, token/session/attestation DTOs | OK | Confirmed delegated `RootProof` has only `IcChainKeyBatchSignatureV1` at `crates/canic-core/src/dto/auth.rs:31`; role attestation uses separate `RoleAttestationRootProof` at `crates/canic-core/src/dto/auth.rs:652` | None found. |
| `crates/canic-core/src/access` | Endpoint caller/auth predicates and delegated-token first-arg verifier | `access/auth/predicates.rs`, `access/auth/token.rs`, `access/expr` | IC CDK Candid decode | config, storage sessions, ops auth | access predicates used by macros | OK | Confirmed registered-subnet root predicate fails closed outside root (`crates/canic-core/src/access/auth/predicates.rs:109`) and endpoint token path verifies/binds/scopes in order (`crates/canic-core/src/access/auth/token.rs:60`) | No production cleanup found. |
| `crates/canic-core/src/api/auth` | Endpoint API adapter for delegated auth/session/role attestation | `api/auth/mod.rs`, `api/auth/session/mod.rs` | core DTOs | ops/workflow/config/env | `AuthApi::*` called by macros | OK | Confirmed token prepare/get are issuer-local and active-proof install verifies through ops (`crates/canic-core/src/api/auth/mod.rs:101`, `crates/canic-core/src/api/auth/mod.rs:120`); lazy repair uses caller as issuer (`crates/canic-core/src/api/auth/mod.rs:169`) | None found. |
| `crates/canic-core/src/config` | Config schema and validation | `config/schema/mod.rs`, `config/validation/auth.rs` | serde, k256 | domain auth | `DelegatedTokenConfig`, validation | OK | Confirmed default `root_proof_mode` is `chain_key_batch` (`crates/canic-core/src/config/schema/mod.rs:474`) and validation rejects non-chain-key plus mainnet `test_key_1` (`crates/canic-core/src/config/validation/auth.rs:99`, `crates/canic-core/src/config/validation/auth.rs:174`) | None found. |
| `crates/canic-core/src/storage` | Stable state records and memory IDs | `storage/stable/auth/*`, other stable modules | IC stable memory | DTO/ops mappers | stable schema records | OK | Confirmed stable `RootProofRecord` only has chain-key batch (`crates/canic-core/src/storage/stable/auth/records.rs:245`) and mapper has no legacy root proof arm (`crates/canic-core/src/ops/storage/auth/mapper.rs:362`) | Reserved `ROOT_REPLAY_ID` dead-code expect is intentional stable-memory reservation. |
| `crates/canic-core/src/ops/auth/delegated` | Delegated token canonicalization, verifier, issuer proof, root proof checks | `canonical.rs`, `chain_key.rs`, `verify.rs`, `prepare.rs`, `active_proof.rs` | `sha2`, `k256`, IC crypto verification | DTO/config/policy/storage | pure verification/preparation helpers | OK | Confirmed chain-key verifier binds policy/header/cert/leaf/signature/witness (`crates/canic-core/src/ops/auth/delegated/chain_key.rs:162`), high-s rejection (`:464`), and token subset checks after root proof (`crates/canic-core/src/ops/auth/delegated/verify.rs:110`) | No unsafe auth refactor made. |
| `crates/canic-core/src/ops/auth/delegation` | Root issuer policy, renewal templates, chain-key batch prepare/sign/install | `chain_key_batch/mod.rs`, `chain_key_registry.rs`, `root_issuer_renewal/*` | `sha2`, management signer through auth signer trait | storage/config/policy | root auth ops facade | OK | Confirmed one batch covers sorted due issuers, duplicate issuer rejection, Merkle witness generation, persisted retry states, and install retries (`crates/canic-core/src/ops/auth/delegation/chain_key_batch/mod.rs:113`, `:998`, `:1011`, `:195`) | Large file remains complex; first directory-module move is complete, extraction remains deferred. |
| `crates/canic-core/src/ops/auth/token.rs` | Runtime delegated-token verifier config and public ops | `ops/auth/token.rs` | IC sig verification, k256 | delegated verifier, config, metrics | `AuthOps::verify_token`, prepare/get active proof ops | OK | Confirmed verifier requires `RootProofMode::ChainKeyBatch` and configured chain-key root policy (`crates/canic-core/src/ops/auth/token.rs:388`); legacy mode rejection covered by tests | None found. |
| `crates/canic-core/src/ops/auth/root_canister_sig.rs` | Role-attestation root canister-signature proof helper | `root_canister_sig.rs` | IC canister signature verification | role attestation DTOs | `RoleAttestationRootProof` only | OK | Confirmed remaining root canister-signature path is role-attestation enum, not delegated `RootProof` (`crates/canic-core/src/ops/auth/root_canister_sig.rs:292`) | Keep docs clear to avoid future conflation. |
| `crates/canic-core/src/ops/ic` and `src/infra/ic` | Management-canister wrappers | `ops/ic/mgmt/signing.rs`, `infra/ic/mgmt/signing.rs` | IC management canister | auth signing ops | `sign_with_ecdsa`, `ecdsa_public_key` wrappers | OK | Confirmed `sign_with_ecdsa` call sites are chain-key batch signer only (`crates/canic-core/src/ops/auth/delegated/chain_key_signing.rs:221`) | None found. |
| `crates/canic-core/src/workflow/runtime/auth` | Runtime auth orchestration, renewal timer, lazy repair, public prepare flow | `renewal.rs`, `provisioning/mod.rs`, `prepare/mod.rs` | IC timers/calls | ops/auth/config/env | lifecycle/auth workflow methods | OK | Confirmed root timer prepare/sign/install requires chain-key mode (`crates/canic-core/src/workflow/runtime/auth/renewal.rs:43`, `:93`, `:108`, `:115`) and lazy repair is internal/update based | Full PocketIC liveness not rerun here. |
| `crates/canic-core/src/replay_policy` | Endpoint replay policy manifest | `endpoint_manifest.rs`, tests | none significant | protocol constants | manifest rows | OK | Confirmed deleted bridge endpoints are tested absent (`crates/canic-core/src/replay_policy/tests/endpoint.rs:10`) | None found. |
| `crates/canic-core` non-auth modules | Runtime foundation: lifecycle, bootstrap, metrics, RPC, view, memory, IDs | `src/lifecycle`, `src/workflow`, `src/ops`, `src/view`, `src/memory`, `src/ids` | IC CDK, Candid, serde | core storage/config | runtime APIs and DTOs | OK | Workspace check/clippy covered all targets; targeted searches did not find focused tests/debug leftovers | Deeper non-auth behavior review can be a follow-up if desired. |
| `crates/canic-macros` | Proc macros for endpoint/lifecycle/build support | `src/endpoint/{parse,validate,expand}` | `syn`, `quote`, `proc-macro2` | facade/core contracts | proc macros | OK | Workspace clippy passed all targets/features; no skipped/focused tests found | No safe cleanup identified. |
| `crates/canic-cli` | Operator CLI | `src/auth`, `src/backup`, `src/blob_storage`, `src/install`, `src/restore`, etc. | `clap`, host/core crates | host, backup, core DTOs | `canic` binary subcommands | OK | Confirmed auth renewal CLI exposes status only (`crates/canic-cli/src/auth/mod.rs:70`) and CI proof asserts removed `run-once`/`provisioner` commands stay absent (`scripts/ci/auth-renewal-cli-proof-lib.sh:81`) | No CLI code cleanup needed. |
| `crates/canic-host` | Host-side build/install/fleet/truth/adoption libraries | `src/install_root`, `src/deployment_truth`, `src/policy_gate`, `src/replica_query` | filesystem, IC CLI/query deps | core/backup/CLI consumers | host APIs consumed by CLI | OK | Workspace check/clippy covered it; broad churn scan found intentional legacy fleet-state warnings only | A separate host-focused audit could inspect operator UX more deeply. |
| `crates/canic-backup` | Backup/restore domain library | `src/manifest`, `src/plan`, `src/restore`, `src/persistence` | serde/filesystem | CLI restore/backup | backup/restore APIs | OK | Workspace check/clippy covered it; "legacy" hits are compatibility tests/manifest parsing | No safe cleanup found. |
| `crates/canic-control-plane` | Root/control-plane canister support | `src/api`, `src/runtime`, `src/storage`, `src/workflow` | canic-core | canic-core | control-plane APIs | OK | Workspace check/clippy covered it; churn scan found expected deprecate-approved-release logic | None found. |
| `crates/canic-wasm-store` | Implicit bootstrap wasm store canister | `src/`, `wasm_store.did` | canic/core | root bootstrap flow | wasm store canister API | OK | Workspace check/clippy covered it; protocol-surface test covers wasm-store endpoint invariants | None found. |
| `crates/canic-testing-internal` | PocketIC test harness helpers | `src/pic` | PocketIC, canic crates | integration tests | test helper APIs | OK | Workspace check/clippy covered it; progress logging hits are test-harness output | Full PocketIC suites deferred. |
| `crates/canic-tests` | Integration and PocketIC tests | `tests/` | PocketIC harness | all crates | integration test binaries | OK | Workspace clippy covered test binaries; full execution deferred; auth `0.76` case names assert legacy bridge surface absence | Full PocketIC execution remains a maintainer/local CI task. |
| `canisters/audit` | Audit/probe canisters | `minimal`, `root_probe`, `leaf_probe`, etc. | canic facade | workspace crates | canister fixtures | OK | Workspace check/clippy covered all targets; no focused tests/debug macros found | None found. |
| `canisters/test` | Test/stub canisters | delegation, sharding, blob, intent, runtime stubs | canic facade | test harness | canister fixtures | OK | Workspace check/clippy covered all targets; prints are intentional test-canister diagnostics | None found. |
| `fleets/test`, `fleets/demo` | Reference local fleet canisters/configs | `Cargo.toml`, `canic.toml`, source dirs | canic facade | host/config validation | local fleet fixtures | OK | Workspace check/clippy covered fleet crates | None found. |
| `scripts/ci` | CI, release, validation scripts | `run-workspace-tests.sh`, `publish-workspace.sh`, inventory gates | shell, ICP, Cargo | workspace | CI commands | OK | Auth renewal proof script checked; removed bridge command assertions retained | Shellcheck not run; not configured as a standard gate found in Makefile. |
| `scripts/dev` | Maintainer/dev helpers | `install_dev.sh`, `cloc.sh`, `gh-ci.sh` | shell tools | workspace | local helpers | OK | AGENTS says these are intentional maintainer helpers, not stale CLI leftovers | No cleanup made. |
| `docs` active | Architecture, contracts, operations, recurring audits, status | `README.md`, `docs/architecture`, `docs/contracts`, `docs/operations`, `docs/audits/recurring`, `docs/status` | Markdown | source contracts | user/dev docs | Needs small cleanup: fixed | Updated stale delegated-auth README/current-status wording and recurring audit templates | Historical archive/changelog entries intentionally left unchanged. |
| `docs` archive/changelog/reports | Historical records | `docs/design/archive`, `docs/changelog`, `docs/audits/reports` | Markdown | release history | history only | OK | Left historical bridge references intact; added baseline note to 0.76 closeout audit | Older reports contain stale line evidence by design. |

## Auth And Access-Control Audit

Initial map:

- Delegated auth public contract is in
  `docs/contracts/AUTH_DELEGATED_SIGNATURES.md`.
- Runtime architecture is in `docs/architecture/authentication.md`.
- Core auth DTOs are in `crates/canic-core/src/dto/auth.rs`.
- Endpoint auth access code is under `crates/canic-core/src/access/auth`.
- Auth ops are under `crates/canic-core/src/ops/auth`.
- Runtime auth workflows are under
  `crates/canic-core/src/workflow/runtime/auth`.
- Endpoint macro bundles live under `crates/canic/src/macros/endpoints`.
- CLI auth operator surface is under `crates/canic-cli/src/auth`.

Current invariant to preserve:

```text
delegated RootProof == IcChainKeyBatchSignatureV1 only
role attestation root canister signatures == RoleAttestationRootProof only
issuer-local token proof == IssuerProof::IcCanisterSignatureV1
```

Detailed auth findings:

- Delegated root proof is hard-cut to `RootProof::IcChainKeyBatchSignatureV1`
  only (`crates/canic-core/src/dto/auth.rs:31`).
- Stable delegated root proof records are also chain-key-only
  (`crates/canic-core/src/storage/stable/auth/records.rs:245`), and mapper
  match arms have no delegated legacy branch
  (`crates/canic-core/src/ops/storage/auth/mapper.rs:362`).
- Verifier config accepts only `root_proof_mode = "chain_key_batch"`
  (`crates/canic-core/src/ops/auth/token.rs:388`,
  `crates/canic-core/src/config/validation/auth.rs:99`).
- Token hot path verifies stored proof material and issuer proof; it does not
  call root threshold signing (`crates/canic-core/src/ops/auth/delegated/verify.rs:110`,
  `crates/canic-core/src/access/auth/token.rs:60`).
- Root threshold signing is reached through chain-key batch signing only
  (`crates/canic-core/src/ops/auth/delegated/chain_key_signing.rs:221`) and is
  invoked by root timer renewal or root internal lazy repair
  (`crates/canic-core/src/workflow/runtime/auth/renewal.rs:108`,
  `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs:36`).
- Lazy repair is an internal root update gated to registered subnet callers in
  the facade macro (`crates/canic/src/macros/endpoints/root.rs:97`), and the API
  passes `msg_caller()` as the issuer (`crates/canic-core/src/api/auth/mod.rs:169`).
- Remaining canister-signature root proof code is role-attestation-specific
  through `RoleAttestationRootProof` (`crates/canic-core/src/dto/auth.rs:652`),
  while issuer-local delegated token proofs still use
  `IssuerProof::IcCanisterSignatureV1` (`crates/canic-core/src/dto/auth.rs:49`).

No P0/P1 delegated-auth cleanup findings remain from this pass.

## Bug Hunt / Cleanup Log

Safe fixes made:

- Updated `README.md` so delegated auth is described as root chain-key batch
  proof renewal, not root canister-signature delegation proof preparation.
- Updated `docs/status/current.md` to state old bridge-backed delegated-auth
  renewal remains historical documentation only, not active stable state.
- Updated an old status-history role-attestation line to use
  `RoleAttestationRootProof::IcCanisterSignatureV1`, avoiding delegated
  `RootProof` conflation.
- Updated
  `docs/audits/recurring/invariants/token-trust-chain.md` so recurring trust
  chain audits inspect the chain-key batch delegated root proof path and the
  separate role-attestation canister-signature path.
- Updated `docs/audits/recurring/system/capability-surface.md` so recurring
  endpoint audits scan current root issuer renewal/lazy-repair endpoints rather
  than deleted bridge proof endpoints.
- Added a baseline note to
  `docs/design/0.76-auth/0.76-closeout-audit.md` clarifying that old line-number
  evidence predates the 0.76.6 hard cut.

No high-confidence production Rust cleanup was identified beyond already
completed hard-cut auth removals.

## Baseline Command Results

- `cargo fmt --all -- --check`: pass.
- `cargo check --locked --workspace`: pass.

## Final Command Results

- `git diff --check`: pass.
- `cargo fmt --all -- --check`: pass.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`:
  pass.
- `cargo test --locked -p canic --test protocol_surface`: pass, 17 tests.
- `cargo test --locked -p canic-core auth --lib`: pass, 250 tests, 0 ignored.
- Focused removed-bridge and legacy-root-proof `rg` scans: pass; remaining hits
  are tests/status notes or role-attestation-specific code.

## Bugs Fixed

- Fixed stale active docs that still described delegated root proof renewal or
  recurring audit commands using pre-hard-cut canister-signature/bridge terms.
- Fixed ambiguous status-history wording that used `RootProof` for
  role-attestation proof material after the delegated-auth hard cut.

## Stale / Dead Code Removed

- No production code removed.
- Removed stale active-doc references to old delegated-auth proof surfaces by
  replacing them with current chain-key batch terminology.
- Removed a stale deleted-auth-command string from a generic `cost_guard` unit
  test fixture. The test now uses a neutral management-deployment command kind
  because it is testing cost bucket behavior, not delegated-auth endpoint names.

## Duplication Reduced

- No code-level DRY changes were made. No obvious, low-risk duplicate auth
  helper surfaced that should be consolidated during this correctness pass.

## Tests Added Or Updated

- Updated a `crates/canic-core/src/ops/cost_guard.rs` unit-test fixture to stop
  using the removed `auth.prepare_delegation_proof_batch.v1` command kind.
- Existing tests now verified in this pass:
  - `crates/canic/tests/protocol_surface.rs` pins current root delegation
    endpoints and absence of old bridge-backed proof methods.
  - `crates/canic-core` auth unit tests cover chain-key verifier negatives,
    canonical fixtures, high-s rejection, lazy repair caching/singleflight,
    signing-volume behavior, partial install retry, legacy mode rejection,
    session binding, and endpoint auth ordering.

## Docs Updated

- `README.md`
- `docs/status/current.md`
- `docs/audits/recurring/invariants/token-trust-chain.md`
- `docs/audits/recurring/invariants/audience-target-binding.md`
- `docs/audits/recurring/invariants/expiry-replay-single-use.md`
- `docs/audits/recurring/system/capability-surface.md`
- `docs/audits/recurring/system/dry-consolidation.md`
- `docs/audits/recurring/system/ops-purity.md`
- `docs/audits/recurring/system/security-boundary-ordering.md`
- `docs/audits/recurring/system/workflow-purity.md`
- `docs/design/0.76-auth/0.76-closeout-audit.md`

## Risky Items Intentionally Deferred

- Full `make test` was not rerun from this agent session because earlier
  instructions said not to run it here and the maintainer reported it passed
  locally after this audit started.
- Full `cargo test --locked --workspace` and PocketIC `canic-tests` execution:
  deferred for the same runtime/timeout reason. Targeted non-PocketIC auth and
  protocol-surface tests passed.
- Large structural split of
  `crates/canic-core/src/ops/auth/delegation/chain_key_batch/mod.rs`: this remains
  a complex but tested file, and splitting it now would be a rewrite-style
  cleanup outside this pass.
- Host/backup/operator UX deep audit: workspace gates passed and broad scans did
  not find obvious bugs, but a dedicated pass would be better than incidental
  changes.

## Recommended Next Steps Before Optimization

1. Run the full PocketIC suite in the maintainer environment:
   `cargo test --locked -p canic-tests` or the repo-approved CI wrapper, not
   from this constrained session.
2. Keep `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
   and `cargo test --locked -p canic-core auth --lib` as the immediate auth
   hard-cut confidence gates.
3. Do a separate, narrow maintainability slice for
  `ops/auth/delegation/chain_key_batch/mod.rs` if it keeps growing, preserving
   current tests before any extraction.
4. Archive or clearly mark older audit reports only if the project wants
   historical reports to stop showing old endpoint names in broad search; this
   pass intentionally avoided rewriting release history.

## Pre-1.0 Holistic Follow-Up

Maintainer reported `make test` completed with no errors after the first audit
pass. This follow-up therefore focused on cleanup that is useful before 1.0 but
low risk for a feature-complete auth line:

- re-scanned active code, operations docs, architecture docs, contracts,
  recurring audit templates, CLI code, and CI scripts for deleted delegated-auth
  endpoint names and delegated `RootProof::IcCanisterSignatureV1` references;
- confirmed remaining active-code hits for deleted delegated-auth endpoint names
  are negative replay-manifest tests asserting those endpoints stay absent;
- confirmed remaining `IcCanisterSignatureV1` active-code hits are
  `RoleAttestationRootProof` uses, not delegated-token `RootProof` acceptance;
- inspected the public `RootDataCertificateUnavailable` mapping and left it as
  a documented P2 naming wart because it belongs to the role-attestation
  canister-signature path and changing public error text/code needs a narrower
  compatibility decision;
- inspected the workspace/public MSRV split and left it unchanged:
  `Cargo.toml` declares public crate MSRV while `rust-toolchain.toml` pins the
  internal development toolchain, and `AGENTS.md` forbids version edits unless
  explicitly requested.

Additional commands run in the follow-up:

| Command | Result | Notes |
| --- | --- | --- |
| `rg -n "prepare_delegation_proof_batch|get_delegation_proof_batch|install_delegation_proof_batch|preflight_delegation_proof_batch|pending_delegation_proof_batch|mark_delegation_proof_batch_installed|RootProof::IcCanisterSignatureV1|RootProofRecord::IcCanisterSignatureV1|RootProofMode::IcCanisterSignature" ...` | Pass | Remaining hits are negative tests or role-attestation-specific canister-signature code. |
| `rg -n "TODO|FIXME|HACK|XXX|temporary|workaround|deprecated|legacy|unused|stale|dead code|console\\.log|debugger|dbg!\\(|#\\[ignore\\]|\\.only\\(|\\.skip\\(" ...` | Pass | Hits are active docs, intentional test wording, scaffold lint expectations, user-facing output, or domain concepts; no high-confidence production cleanup found. |
| `git diff --stat` | Pass | Diff remains scoped to docs plus one generic test fixture string. |
| `cargo fmt --all -- --check` | Pass | Formatting clean after the follow-up Rust fixture edit. |
| `cargo test --locked -p canic-core cost_guard --lib` | Pass | 10 passed, 0 failed, 0 ignored. |
| `git diff --check` | Pass | No whitespace errors. |

Recommended next cleanup slices:

1. A narrow non-auth host/operator UX audit of `canic-cli` and `canic-host`
   command flows, especially stale operator messages and error consistency.
2. A maintainability-only extraction plan for
  `crates/canic-core/src/ops/auth/delegation/chain_key_batch/mod.rs`, with no
   behavior change and tests pinned before movement.
3. A public-error naming review for role-attestation canister-signature errors,
   including whether `RootDataCertificateUnavailable` should remain stable as
   an error code or be aliased by a clearer role-attestation-specific message.

## Pre-1.0 Cleanup Slices Handled

This follow-up handled the three cleanup slices from the prior recommendation.

### Host / Operator UX Wording

Scope inspected:

- `crates/canic-cli/src/auth`
- `crates/canic-cli/src/medic`
- `crates/canic-host/src`
- `docs/operations/root-proof-provisioning.md`

Findings:

- No active CLI bridge renewal command, `run-once`, or provisioner command was
  found.
- `canic auth renewal` is status-only today, but help text still used
  "Run root-managed delegation proof renewal workflows", which could read like
  an operator-driven bridge action.
- Medic auth-renewal help used generic delegated-auth wording instead of the
  current chain-key renewal wording.

Actions taken:

- Updated `canic auth renewal` help to say it inspects root-managed chain-key
  delegation proof renewal.
- Updated `canic auth renewal status` help to say it shows chain-key renewal
  state for one issuer.
- Updated medic `--auth-renewal` help to say chain-key auth renewal drift
  diagnostics.
- Updated auth-renewal medic next-action text to avoid implying an operator
  manual repair path; it now points to waiting for root chain-key renewal or
  retrying an issuer login/update so lazy repair can run.
- Added a CLI unit test that asserts renewal help names the chain-key status
  surface and does not mention `run-once` or `provisioner`.

### Role-Attestation Error Wording

Scope inspected:

- `crates/canic-core/src/dto/error.rs`
- `crates/canic-core/src/error.rs`
- `crates/canic-core/src/ops/auth/error.rs`
- `crates/canic-core/src/ops/auth/root_canister_sig.rs`

Findings:

- `ErrorCode::RootDataCertificateUnavailable` is still a stable public error
  code used by the role-attestation root canister-signature path.
- The public message said "delegation proof retrieval", which is stale after
  delegated-token root proofs moved to chain-key batches.

Actions taken:

- Kept `ErrorCode::RootDataCertificateUnavailable` stable.
- Updated the public message to "root data certificate unavailable for
  role-attestation proof retrieval".
- Added a regression assertion that the public message contains
  "role-attestation proof retrieval".

### Chain-Key Batch Maintainability Plan

Scope inspected:

- `crates/canic-core/src/ops/auth/delegation/chain_key_batch/mod.rs`
- `crates/canic-core/src/ops/auth/delegation/mod.rs`
- `crates/canic-tests/tests/root_cases/auth_076.rs`

Findings:

- `chain_key_batch/mod.rs` is 2353 lines and combines public ops facade, persisted
  state selection, due template selection, batch building, Merkle construction,
  install materialization, local batch-id encoding, and tests.
- The file has broad focused coverage, so a split should be planned and
  test-gated rather than mixed into behavior cleanup.

Actions taken:

- Added `docs/design/0.76-auth/chain-key-batch-maintainability-plan.md`.
- The plan records current function clusters with line evidence, required test
  gates, extraction order, no-change boundaries, and stop conditions.
- Follow-up `.7` cleanup completed the planned no-behavior-change module
  extraction.

Additional commands for this follow-up:

| Command | Result | Notes |
| --- | --- | --- |
| `rg -n "delegation proof|delegation_proof|auth renewal|renewal provisioner|bridge|direct root query|RootProof|canister-signature|reprovision auth proof|root data certificate|data certificate|legacy|deprecated|old auth|root proof" crates/canic-cli/src crates/canic-host/src docs/operations docs/architecture docs/contracts README.md -g '*.rs' -g '*.md'` | Pass | No live bridge/provisioner command path found; hits were current docs, role-attestation, or non-auth legacy deployment state. |
| `rg -n "RootDataCertificateUnavailable|root_data_certificate_unavailable|delegation proof retrieval|root data certificate unavailable" crates/canic-core/src crates/canic-core/tests crates/canic/src crates/canic-cli/src -g '*.rs'` | Pass | Isolated the stale public message to `dto/error.rs`. |
| `wc -l crates/canic-core/src/ops/auth/delegation/chain_key_batch.rs` | Pass | 2353 lines before the directory-module move. |
| `rg -n "^(pub\\(super\\)|pub|fn|struct|enum|impl|mod tests|    fn chain_key_batch_|    fn .*chain_key)" crates/canic-core/src/ops/auth/delegation/chain_key_batch.rs` | Pass | Produced line evidence for the maintainability plan before the directory-module move. |

### Chain-Key Batch Plan Slice 1

Action taken:

- Moved `crates/canic-core/src/ops/auth/delegation/chain_key_batch.rs` to
  `crates/canic-core/src/ops/auth/delegation/chain_key_batch/mod.rs` with
  `git mv`.
- Left `mod chain_key_batch;` unchanged in
  `crates/canic-core/src/ops/auth/delegation/mod.rs`; Rust's normal module
  discovery now resolves the directory module.
- Did not extract submodules or change behavior.
- Updated
  `docs/design/0.76-auth/chain-key-batch-maintainability-plan.md` so future
  extraction work starts at Slice 2.

### Chain-Key Batch Plan Slices 2-5

Action taken:

- Extracted local deterministic batch-id encoding into
  `crates/canic-core/src/ops/auth/delegation/chain_key_batch/batch_id.rs`.
- Extracted signed batch to issuer proof materialization into
  `crates/canic-core/src/ops/auth/delegation/chain_key_batch/install.rs`.
- Extracted duplicate-issuer checks, Merkle root construction, witness
  construction, and node hashing into
  `crates/canic-core/src/ops/auth/delegation/chain_key_batch/merkle.rs`.
- Extracted due template selection, cap enforcement, pending batch quota
  counting, and retry-window due checks into
  `crates/canic-core/src/ops/auth/delegation/chain_key_batch/selection.rs`.
- Kept the public ops boundary, state-machine transitions, signing
  orchestration, and tests in `chain_key_batch/mod.rs`.
- Did not change DTO shapes, stable records, endpoint names, signature
  material, Merkle witness format, retry semantics, or lazy repair behavior.
- Added `0.76.7` root and detailed changelog entries for this cleanup batch.

Verification covering this follow-up and the final Slice 1 move:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all` | Pass | Formatted the Rust test/help-string edits. |
| `cargo test --locked -p canic-cli auth --lib` | Pass | 25 passed, 0 failed, 0 ignored. |
| `cargo test --locked -p canic-core root_data_certificate_unavailable_maps_to_public_code --lib` | Pass | 1 passed, 0 failed, 0 ignored. |
| `cargo fmt --all -- --check` | Pass | Formatting check passed after the full chain-key batch split. |
| `cargo test --locked -p canic-core chain_key_batch --lib` | Pass | 43 passed, 0 failed, 0 ignored after the full chain-key batch split. |
| `cargo test --locked -p canic-core chain_key_lazy_repair --lib` | Pass | 4 passed, 0 failed, 0 ignored after the full chain-key batch split. |
| `cargo test --locked -p canic-core workflow::runtime::auth --lib` | Pass | 24 passed, 0 failed, 0 ignored after the full chain-key batch split. |
| `cargo test --locked -p canic-core auth --lib` | Pass | 250 passed, 0 failed, 0 ignored after the full chain-key batch split. |
| `cargo test --locked -p canic-cli auth --lib` | Pass | 25 passed, 0 failed, 0 ignored after operator wording cleanup. |
| `cargo test --locked -p canic --test protocol_surface` | Pass | 17 passed, 0 failed, 0 ignored after the full chain-key batch split. |
| `cargo test --locked -p canic --test changelog_governance` | Pass | 1 passed, 0 failed, 0 ignored after adding the `0.76.7` notes. |
| `cargo clippy --locked -p canic-core -p canic-cli --all-targets --all-features -- -D warnings` | Pass | Core/CLI clippy clean after the split and help-text tests. |
| `git diff --check` | Pass | No whitespace errors after the directory-module move. |
| `rg -n "delegation proof renewal workflows|Run root-managed delegation proof|delegation proof retrieval|auth\\.prepare_delegation_proof_batch" ...` | Pass | No active-code/doc matches remain outside intentionally historical design/audit text. |
