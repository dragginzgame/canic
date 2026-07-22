# 0.98.2 Consolidation Implementation Tracker

Date: 2026-07-22

Overall status: implementation-complete; immutable closeout waits for
`v0.98.2`

## Baseline guard

| Item | Status | Evidence |
| --- | --- | --- |
| Immutable source | complete | commit `e0dcd0cbb8f550e4c0366d9e1007ca32dceb2aa7`, tree `ae154b0deb862702d48fed4dd235caf76089f7a2` |
| Open patch scope | complete | original Slice C randomness hard cut plus the maintainer-approved consolidation amendment |
| Prior 0.98 ownership | complete | project-protocol and LocalIntent fixture cuts remain published in v0.98.0/v0.98.1 and are not counted again |
| Version guard | complete | no package, install URL, release-script default, tag, or lock package version changed |

## Slice tracker

| Slice | Status | Findings / outcome | Focused evidence |
| --- | --- | --- | --- |
| A — baseline and authority reconstruction | complete | 37 packages, target kinds, workspace edges, features, role configs, build scripts, DIDs, test fixtures, and major authorities inventoried | locked Cargo metadata; reverse-reference and conceptual searches; Git history |
| B — build/config truth | complete | C003, C004, C005, C006 fixed | 26 role-package tests; 21 state-manifest tests; config-guide parse test |
| C — runtime/state hard cuts | complete | C007, C009, C010, C011, C012 fixed | state-contract, CycleTracker, capability, management-op, and host checks |
| D — auth consolidation | complete | C013 fixed with seed/domain bytes unchanged | default and all-feature auth suites: 169 passed in each selected run |
| E — host/operator and dependency surface | complete | C014/C015 fixed; C016 tool evidence corrected | 20 ICP adapter tests; host/CLI/backup all-target all-feature check; Cargo Machete clean |
| F — repository closure | complete | all 42 dispositions, final searches, broad validation, governance, and closeout agree | see completion matrix below |

## Implemented finding details

### C003 — Syntax-aware role build contract

- Ledger/severity: `CANIC-098-CLOSE-BUILD-001`, P2, `SIMPLIFY`.
- Old responsibility: a line-oriented string matcher guessed whether a role
  build script called `canic::build!` exactly once.
- Surviving authority: the parsed Rust AST in
  `canic-host::role_contract::package`.
- Change: use Syn 3 `Visit` to count only exact non-leading-colon
  `canic::build!` macro paths.
- Regression proof: multiline invocation is accepted; raw-string and comment
  text do not count; omitted and duplicate invocations reject.
- Dependency result: direct host dependency selects existing Syn 3.0.3; Syn
  1/2/3 version count is unchanged.
- Files/symbols: `crates/canic-host/src/role_contract/package/{mod.rs,tests.rs}`
  replaces lexical counting with a Syn `Visit`; `crates/canic-host/Cargo.toml`
  and the host package row in `Cargo.lock` add the direct existing Syn edge.
- Validation: 26 package-contract tests pass, including the formerly failing
  omitted-role-feature case; the inverse tree selects Syn 3.0.3 and the
  duplicate tree still contains exactly Syn 1, 2, and 3.

### C004 — One parsed config snapshot

- Ledger/severity: `CANIC-098-CLOSE-CONFIG-001`, P2, `CONSOLIDATE`.
- Old responsibility: passive state-manifest resolution re-opened and reparsed
  the selected config while validating each declared role.
- Surviving authority: the `ConfigModel` parsed once by state-manifest
  resolution.
- Change: pass that model to `_from_config` role validation and contract
  resolution functions.
- Contract/persistence impact: none.
- Files/symbols: `crates/canic-host/src/state_manifest/resolution.rs` and
  `crates/canic-host/src/role_contract/package/mod.rs` carry `&ConfigModel`
  through `resolve_state_manifest` and the `_from_config` entry points;
  adjacent state-manifest/package tests prove the same selected model.
- Validation: 21 state-manifest tests and the workspace role-contract suites
  pass.

### C005/C006 — Current configuration and testing contracts

- Ledger/severity: `CANIC-098-CLOSE-CONFIG-002`, P1, `CONSOLIDATE`; and
  `CANIC-098-CLOSE-DOC-001`, P2, `REMOVE`.
- Old responsibility: root guides described retired role/config semantics and
  TESTING preserved a removed annotation compatibility breadcrumb.
- Surviving authority: strict `ConfigModel`, role contract, and current
  PocketIC harness.
- Change: rewrite the guide around roles/fleet, deny-all omission, current
  canister kinds, singleton placement, metrics, funding, root-only manual ICP
  refill, verifier, ICRC-21, directory, and sharding; parse its exact example
  in a test; delete the breadcrumb.
- Impact: documentation correction only; no accepted TOML shape added.
- Files/symbols: `CONFIG.md` is replaced with the current `ConfigModel`
  contract; `crates/canic/tests/config_guide.rs` parses its exact TOML block;
  `TESTING.md` drops the retired annotation compatibility text.
- Validation: the executable guide test passes, strict unknown-field tests
  reject the original Slice C randomness table, and protocol/root suites
  prove the documented manual refill surface.

### C007 — Dead duration parser

- Ledger/severity: `CANIC-098-CLOSE-HOST-001`, P2, `REMOVE`.
- Old responsibility: host parsed free-form duration strings for an earlier
  command surface.
- Surviving authority: CLI command-specific typed parsing, including `--since`.
- Change: delete `canic-host::duration` and its self-only tests.
- Impact: unused public Rust API hard cut; no CLI/JSON/Candid/stable impact.
- Files/symbols: delete `crates/canic-host/src/duration/{mod.rs,tests.rs}` and
  remove `pub mod duration` from `crates/canic-host/src/lib.rs`.
- Validation: host/CLI all-target checks, workspace tests, and workspace
  Clippy pass; the maintained CLI `--since` parser remains covered.

### C009 — Control-plane internal surface

- Ledger/severity: `CANIC-098-CLOSE-CP-001`, P2, `SIMPLIFY`.
- Old responsibility: `runtime` and `schema` were exported even though no
  external or macro consumer existed.
- Surviving authority: crate-local control-plane workflow and schema modules;
  semantic DTO/ID/state-contract surfaces remain public.
- Change: narrow both modules to `pub(crate)`.
- Files/symbols: `crates/canic-control-plane/src/lib.rs` narrows `runtime` and
  `schema`; public semantic `dto`, `ids`, and state-contract exports remain.
- Contract/persistence impact: no external Candid, stable, JSON, or CLI
  contract changes.
- Validation: the minimal/Wasm-store/host control-plane feature matrix and
  workspace Clippy pass.

### C010 — CycleTracker state truth

- Ledger/severity: `CANIC-098-CLOSE-STATE-001`, P2, `CONSOLIDATE`.
- Old responsibility: memory 29 was described as reserved although the live
  cycle tracker reads and writes it.
- Surviving authority: `CycleTrackerEntryRecord` plus canonical
  `CycleTrackerData` export/import.
- Change: add the snapshot conversion and declare the memory active in the
  core/host state manifests.
- Impact: metadata and tests only; memory ID and stored bytes unchanged.
- Files/symbols: `crates/canic-core/src/storage/stable/cycles.rs` adds
  `CycleTrackerEntryRecord`, `CycleTrackerData`, and exact `export`/`import`;
  `crates/canic-core/src/state_contract.rs` moves memory 29 from reservation to
  `cycle_tracker`; host state-manifest and CLI state-report tests project it.
- Validation: the stable snapshot test, 13 state-contract tests, 21 host
  state-manifest tests, eight CLI state tests, and the final workspace gate
  pass with state status `pass` and no reserved-memory warning.

### C011 — Capability proof simplification

- Ledger/severity: `CANIC-098-CLOSE-CAP-001`, P2, `SIMPLIFY`.
- Old responsibility: an internal root-proof enum, mode enum, async verifier
  trait, and router modeled multiple verifier implementations that no longer
  existed.
- Surviving authority: the one structural `CapabilityProof` wire contract and
  direct structural validation.
- Change: delete the internal multi-mode router/verifier and derive metrics
  from the wire proof.
- Impact: no Candid or authorization semantic change.
- Files/symbols: delete
  `crates/canic-core/src/workflow/rpc/capability/verifier.rs`; remove
  `RootCapabilityProof`, its mode/router/trait, and route
  `CapabilityProof::Structural` directly through `envelope.rs`, `root.rs`, and
  `nonroot.rs`; metric labels derive from the wire proof.
- Validation: 14 all-feature capability tests and all four role-attestation
  PocketIC cases pass, including structural authorization and denial metrics.

### C012 — Unreachable runtime snapshot adapter

- Ledger/severity: `CANIC-098-CLOSE-MGMT-001`, P2, `REMOVE`.
- Old responsibility: core management infra/ops exposed take/load canister
  snapshots and metric labels.
- Surviving authority: `canic-backup` orchestration using the typed host ICP
  CLI snapshot adapter.
- Change: delete infra/ops snapshot modules, args/results, ops snapshot model,
  conversion/re-exports, unreachable management metric variants, and the dead
  unscoped metric recorder.
- Impact: internal public Rust/metric-label hard cut; no production call,
  Candid contract, or stable state existed.
- Files/symbols: delete core infra/ops `mgmt/snapshots.rs`; remove snapshot
  args/results and `CanisterSnapshot` conversions/re-exports, management-call
  `TakeCanisterSnapshot`/`LoadCanisterSnapshot`, canister-op
  `Snapshot`/`Restore`, and `record_unscoped_canister_op` from the associated
  mgmt and metric modules.
- Validation: management-op metric tests, exact production/macro/wasm/script
  reachability searches, host backup/restore suites, and the full root/Wasm
  suites pass; no runtime management-snapshot symbol remains.

### C013 — Singleton auth payload families

- Ledger/severity: `CANIC-098-CLOSE-AUTH-001`, P2, `SIMPLIFY`.
- Old responsibility: `RootPayloadKind` and `IssuerPayloadKind` suggested
  selectable payload families after prior proof designs were removed.
- Surviving authority: fixed root role-attestation and issuer delegated-token
  seed/domain constants.
- Change: remove kind arguments and branches from prepare/get/verify,
  pending-key, token, delegation, registry, batch, and fixture paths.
- Impact: exact bytes unchanged; no Candid/stable/public endpoint change.
- Files/symbols: remove `RootPayloadKind` and `IssuerPayloadKind` plus their
  parameters from `ops/auth/{root_canister_sig.rs,issuer_canister_sig.rs}` and
  all attestation, delegation, token, registry, batch, pending-key, retention,
  and fixture call sites. Current auth docs use the canonical seed/domain
  constants. The recurring trust-chain method advances to fingerprinted v2
  because its search inventory changed.
- Validation: 169 all-feature auth tests, four role-attestation PocketIC
  cases, protocol surfaces, audit-method catalog, and exact retired-kind
  searches pass.

### C014/C015 — Host ICP surface

- Ledger/severity: `CANIC-098-CLOSE-ICP-001` and `CANIC-098-CLOSE-ICP-002`, both P2;
  dispositions `REMOVE` and `SIMPLIFY`.
- Old responsibility: convenience methods and public helpers supported old
  snapshot CLI/dry-run and internal command-building surfaces.
- Surviving authority: typed create/inventory/download, current Candid-aware
  call/query methods, lifecycle/top-up methods, and only the helpers imported
  by CLI/backup.
- Change: delete orphan call/start/stop/snapshot/version/display wrappers and
  their test-only coverage; make same-crate helpers private, parent-scoped, or
  crate-only re-exports through private modules.
- Impact: unused public Rust hard cut only; maintained CLI behavior unchanged.
- Files/symbols: `crates/canic-host/src/icp/{canister.rs,snapshot.rs,model.rs,
  version.rs,command.rs,run.rs,mod.rs,tests.rs}` remove the orphan methods and
  narrow command/response/run/version helpers. The surviving `*_with_candid`
  methods remain because cycles conversion and blob-storage commands consume
  them.
- Validation: 20 host ICP tests, host/CLI/backup all-target checks, workspace
  Clippy, and exact word-boundary searches pass.

### C016 — Intentional role fixture dependencies

- Ledger/severity: `CANIC-098-CLOSE-DEPS-001`, Note, `RETAIN`.
- Suspected issue: Cargo Machete reported three unused normal dependencies.
- Evidence: these isolated fixtures deliberately prove that role features live
  on the normal `canic` dependency while `canic::build!` uses a featureless
  build dependency; one also proves a renamed dependency.
- Change: add exact fixture-local Cargo Machete ignores for `canic` or
  `framework`.
- Result: repository-wide Cargo Machete reports no unused dependencies.
- Files/symbols: exact `[package.metadata.cargo-machete]` ignores live only in
  the `supported`, `renamed_canic`, and `protected_sibling` role fixture
  manifests; no production manifest is suppressed.
- Validation: Cargo Machete is clean and all 26 role-package tests pass.

## Focused validation completed

| Command / scope | Outcome |
| --- | --- |
| `cargo test --locked -p canic-host role_contract::package::tests --lib` | pass, 26 tests |
| `cargo test --locked -p canic-host state_manifest --lib` | pass, 21 tests |
| `cargo test --locked -p canic --test config_guide` | pass, 1 test |
| `cargo test --locked -p canic-core state_contract --lib` | pass, 13 tests in final rerun |
| CycleTracker stable snapshot filter | pass, 1 test |
| Capability workflow filter | pass, 14 tests |
| Management canister-op metric filter | pass, 1 test |
| `cargo test --locked -p canic-core --all-features ops::auth --lib` | pass, 169 tests |
| `cargo test --locked -p canic-host icp:: --lib` | pass, 20 tests |
| `cargo check --locked -p canic-host -p canic-cli -p canic-backup --all-targets --all-features` | pass |
| `cargo machete --skip-target-dir` | pass, no unused dependencies |
| Syn inverse/duplicate tree | pass; host selects existing Syn 3.0.3 and adds no version |

## Completion validation matrix

| Gate | Status | Result |
| --- | --- | --- |
| Exact retired-symbol search | pass | no active exact symbol, retired module, package, refill-timer, or runtime snapshot-adapter hit; 0.98 randomness hits are strict rejection tests only |
| Conceptual legacy/fallback/duplicate-owner search | pass | every candidate has a ledger disposition; no deferred/unresolved row |
| Formatting | pass | `make fmt-check` passes Cargo Sort, derive sort, and Rustfmt |
| Layering guard | pass | production guard and detector self-test pass |
| Role/control-plane feature matrix | pass | minimal, Wasm-store, and host consumer combinations compile |
| Affected package/all-target tests | pass | focused core/host/config/auth/state/capability tests and workspace library/binary run pass |
| Workspace tests | pass | `make test-unit` full mode passes every selected workspace and PocketIC suite |
| Relevant wasm/protocol checks | pass | CI Wasm builders, 22 protocol tests, 28 root cases, and 10 Wasm-store reconciliation cases pass |
| Strict Clippy | pass | `make clippy` passes workspace/all-target/all-feature `-D warnings` |
| Dependency checks | pass | dependency-risk guard and Cargo Machete pass; Syn versions remain 1/2/3 with existing 3.0.3 reused |
| Changelog/governance | pass | changelog, audit-method catalog, release-validation, and release-integrity guards pass |
| Diff hygiene | pass | formatting and `git diff --check` pass; package versions are unchanged and the complete patch is compared to immutable `v0.98.1` |

## External validation limitation

No live funded-IC ICP-to-cycles conversion is executed by this source audit.
That journey requires maintainer-controlled identity, ledger funds, network,
and deployment state. Its absence does not hide a second architecture: source,
config, Candid, CLI, replay, stable memory, metrics, and timer inventories all
show one root-only manual workflow. Local and unit validation must not be
reported as a real mainnet conversion.
