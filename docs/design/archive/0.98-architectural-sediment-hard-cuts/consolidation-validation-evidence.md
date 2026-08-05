# 0.98.2 Consolidation Validation Evidence

Date: 2026-07-22

Status: finalized at immutable `v0.98.2`

Verdict: `PASS`

## 1. Baseline and scope

This supporting report covers the complete Canic repository at immutable commit
`e0dcd0cbb8f550e4c0366d9e1007ca32dceb2aa7`, tree
`ae154b0deb862702d48fed4dd235caf76089f7a2`, plus the complete 0.98.2 release
candidate that became immutable `v0.98.2`. It inspected all 37 workspace
packages, their modules, features, build scripts, macro paths, role packages,
generated/deployment surfaces, current and historical documentation, test
canisters, fixtures, scripts, durable contracts, and operator paths.

The committed `Cargo.lock` baseline hash was
`801ad42f9b2a733e925d3c4b0b66cae1922b60b3b7b2cc0166a9a52cfd2092e2`.
The validated release-candidate lock hash is
`7c952ab09618af9f936837205918c4092ad066ed0a943a6e1b663c5649966046`.
Its new rows both belong to the combined 0.98.2 patch:

- host-only `serde_path_to_error` provides typed rejection evidence for the
  removed randomness table; and
- the direct `canic-host` Syn edge reuses the already present Syn 3.0.3
  package.

The maintainer approved combining the original Slice C randomness work and
the consolidation findings before publication. The released
`project-protocol-stub` and LocalIntent fixture cuts remain attributed to
v0.98.0 and v0.98.1 respectively and are not counted again.

## 2. Reconstructed current architecture

The complete authority map is in [consolidation design amendment](consolidation-design-amendment.md). Its
closeout result is:

| Responsibility | Sole current authority |
| --- | --- |
| Public canister facade and lifecycle/build macros | `canic`, with procedural expansion in `canic-macros` |
| Runtime DTO/model/policy/ops/workflow/stable state | `canic-core` |
| Root and Wasm-store control plane | `canic-control-plane`; built-in store package in `canic-wasm-store` |
| Configuration schema | strict `canic-core::config::schema::ConfigModel`; one parsed host projection |
| Role/package/build contract | config role declarations plus `package.metadata.canic`, checked by `canic-host` |
| Endpoint authentication/authorization | endpoint guard boundary; one role-attestation chain and one delegated chain-key/token chain |
| Intent admission, execution, receipt replay, and recovery | policy decisions plus `ReceiptBackedIntentOps` storage and workflow orchestration |
| Root ICP-to-cycles conversion | controller-triggered root workflow; manual only, with no timer key or callback |
| Management-canister effects | core infra raw calls -> ops checked effects -> workflows; no runtime snapshot adapter |
| Timers | one generation-safe `TimerWorkflow` authority |
| Topology, registry, placement, pool, and template state | core/control-plane records and ops; placement owns child creation |
| Backup/restore | `canic-backup` durable plan/journal/receipt model plus one CLI/host ICP executor |
| Protocol descriptors | semantic DTOs plus the canonical Wasm-store DID and two external blob fixtures |
| Stable-state declarations | core/control-plane state contracts combined by the host state manifest |
| Operator evidence | `canic-cli` over typed `canic-host`, `canic-backup`, and core boundary types |
| PocketIC infrastructure | `canic-testing-internal` harness and `canic-tests` behavior suites |

The validated dependency direction is:

```text
role canisters -> canic -> {canic-core, canic-control-plane, canic-macros}
canic-control-plane -> canic-core

canic-cli -> {canic-host, canic-backup, canic-core}
canic-host -> {canic-control-plane, canic-core}

canic-tests -> canic-testing-internal -> product crates
```

No shared runtime crate depends on a canister-role or host/operator facade.
DTOs remain passive, records own persisted encodings, views remain internal,
and workflow does not bypass ops into stable storage.

## 3. Coverage

The [consolidation disposition ledger](consolidation-ledger.md) lists every workspace package once
and covers every major subsystem. Package coverage is complete across:

- 10 product/support packages: `canic`, `canic-core`, `canic-macros`,
  `canic-control-plane`, `canic-wasm-store`, `canic-host`, `canic-cli`,
  `canic-backup`, `canic-testing-internal`, and `canic-tests`;
- 6 audit/sandbox packages: `leaf_probe`, `root_probe`, `scaling_probe`,
  `canister_minimal`, `canister_minimal_metrics`, and
  `canister_sandbox_blank`;
- 10 dedicated test canisters: blob cashier/probe, delegation issuer/root,
  intent authority, payload limit, project hub/instance, runtime, and sharding
  root;
- 4 demo fleet role packages; and
- 7 full test-fleet role packages.

Subsystem coverage includes facade/macro expansion, configuration and role
projection, authentication, intent/replay, refill, backup/restore,
management calls, timers/lifecycle, topology/registry/pool, templates and
Wasm-store, DTO/model/view/record ownership, stable memory, Candid/generated
artifacts, JSON evidence, host/CLI adapters, error propagation, PocketIC
fixtures, audit probes, and CI/release/deployment tools.

Every accepted configuration field was traced into validation, rendering,
runtime behavior, or a deliberate build/diagnostic contract. Every remaining
production platform adapter has a production or feature-selected consumer.
Every package has a current build, test, deployment, probe, or operator
consumer. All Cargo features either gate code/dependencies or, for `metrics`,
serve the intentional role-build selection contract.

## 4. Counts and dispositions

| Severity | Count | Closeout |
| --- | ---: | --- |
| P0 | 0 | none found |
| P1 | 1 | fixed |
| P2 | 11 | fixed |
| Note | 30 | proved |
| Total | 42 | all resolved |

| Disposition | Count |
| --- | ---: |
| REMOVE | 4 |
| CONSOLIDATE | 3 |
| REHOME | 0 |
| SIMPLIFY | 5 |
| RETAIN | 23 |
| REJECTED | 7 |
| DEFERRED | 0 |
| UNRESOLVED | 0 |
| Total | 42 |

`RETAIN` was used only with producer/consumer, state, feature, macro, test, or
operator evidence. `REJECTED` rows record why a suspected duplicate or dead
path was disproved. No difficult candidate was defaulted to either category.

## 5. Corrections by implementation slice

### Slice A — Baseline and authority reconstruction

Frozen the immutable `v0.98.1` baseline and complete release scope;
enumerated all packages, target kinds,
features, normal/build dependencies, role configs, build scripts, checked-in
DIDs, generated artifact paths, test fixtures, and production entry points.
This prevented the 0.98 hard cuts from being duplicated or silently absorbed.

### Slice B — Build and configuration truth

- Replaced lexical `canic::build!` matching with a Syn visitor that recognizes
  the exact macro path. Multiline syntax is supported; strings/comments do
  not count.
- Passed one parsed `ConfigModel` through passive state-manifest and role
  validation instead of reopening the selected file per role.
- Rebuilt `CONFIG.md` around the current strict schema and added a test that
  parses the exact documented example.
- Removed the retired TESTING annotation compatibility breadcrumb.

This slice fixed the sole P1: current operator documentation had described a
superseded and potentially unsafe configuration contract.

### Slice C — Runtime and state hard cuts

- Deleted the consumerless `canic-host::duration` module.
- Narrowed control-plane `runtime` and `schema` to crate ownership.
- Corrected CycleTracker memory 29 from false reservation to active
  `CycleTrackerEntryRecord`/`CycleTrackerData` state with exact export/import.
- Deleted the internal capability proof kind/mode/router/async verifier and
  directly validated the surviving structural wire proof.
- Deleted the unreachable core management snapshot infra/ops/types and their
  impossible runtime metric variants.

### Slice D — Authentication consolidation

- Deleted singleton `RootPayloadKind` and `IssuerPayloadKind` taxonomies and
  their parameters across attestation, delegation, token, registry, batch,
  pending-key, retention, and fixture paths.
- Preserved the exact root role-attestation and issuer delegated-token
  seed/domain bytes and wire behavior.
- Advanced `CANIC-AUTH-TRUST-001` to fingerprinted method v2 so the recurring
  audit names the surviving constants; retained v1 as superseded evidence.

### Slice E — Host/operator and dependency surface

- Deleted orphan host ICP call/start/stop/snapshot/version/display wrappers
  and their self-only tests.
- Kept only the Candid-aware call/query methods and typed snapshot operations
  that current cycles, blob-storage, backup, and restore consumers use.
- Narrowed internal command/response/run/version helpers behind private
  modules and exact crate/parent re-exports.
- Recorded three manifest-only role fixture dependencies as intentional Cargo
  Machete edges; no production dependency is ignored.

### Slice F — Closure

Repeated exact and conceptual searches, resolved all 42 ledger rows, ran the
full validation matrix, synchronized changelog/status/design/tracker/ledger,
and recorded contract and release impact without changing versions or
external state.

The [consolidation implementation tracker](consolidation-implementation-tracker.md) records severity, old
owner, surviving authority, exact files/symbols, contract impact, regression
proof, and validation for every implemented finding.

## 6. Deleted and narrowed surfaces

Deleted modules/files:

- `canic-host/src/duration/{mod.rs,tests.rs}`;
- core infra and ops `ic/mgmt/snapshots.rs`; and
- `workflow/rpc/capability/verifier.rs`.

Deleted types/variants/functions include:

- `RootCapabilityProof`, its mode/router/async verifier machinery;
- `RootPayloadKind` and `IssuerPayloadKind`;
- core snapshot args/results, `CanisterSnapshot`, take/load operations,
  `ManagementCall` snapshot labels, `CanisterOps` snapshot/restore labels, and
  `record_unscoped_canister_op`;
- host call/start/stop/snapshot/version/display convenience wrappers; and
- the host duration parser.

Narrowed surfaces:

- control-plane `runtime` and `schema` are crate-owned;
- core `cdk` remains doc-hidden because facade, macros, control plane, and role
  builds still require it; and
- host ICP internals are no longer public re-exports unless CLI/backup has a
  proven external consumer.

The consolidation amendment deletes no crate, workspace member, Cargo
feature, binary, build script, Candid method, JSON contract, CLI command,
stable-memory ID, or test canister. The complete 0.98.2 patch additionally
deletes the original Slice C randomness config and public Rust surface. The
package deletions remain owned by the already released v0.98.0/v0.98.1 work.

## 7. Validation results

Focused correction gates:

| Command/scope | Outcome |
| --- | --- |
| Host role-package contract tests | 26 passed; omitted-feature CI regression included |
| Host state-manifest tests | 21 passed |
| Exact CONFIG guide test | 1 passed |
| Core state-contract tests | 13 passed |
| CycleTracker snapshot test | 1 passed |
| All-feature capability tests | 14 passed |
| All-feature auth tests | 169 passed |
| Host ICP adapter tests | 20 passed |
| CLI state-report tests | 8 passed |
| Host/CLI/backup all-target/all-feature check | passed |
| Cargo Machete | no unused dependencies |

Completion gates:

| Command | Outcome |
| --- | --- |
| `make test-unit` | passed in full mode |
| `make clippy` | workspace, all targets, all features, `-D warnings` passed |
| `make fmt-check` | Cargo Sort, derive sort, and Rustfmt passed |
| `bash scripts/ci/run-layering-guards.sh` | passed |
| `bash scripts/ci/run-layering-guards.sh --self-test` | detector fixtures passed |
| `bash scripts/ci/check-control-plane-feature-matrix.sh` | minimal, Wasm-store, and host consumers passed |
| `bash scripts/ci/check-dependency-risk-inventory.sh` | zero vulnerabilities; four exact inventoried transitive advisories |
| `bash scripts/ci/check-audit-method-catalog.sh` | passed with trust-chain v2 fingerprint |
| `bash scripts/ci/check-release-validation-matrix.sh` | passed |
| `bash scripts/ci/check-release-integrity-contract.sh` | passed; 13 immutable Actions |
| `cargo test --locked -p canic --test changelog_governance -- --nocapture` | passed |
| `git diff --check` | passed |

The full workspace gate included:

- all workspace library/binary tests across 37 packages;
- 22 protocol-surface, 1 install-script, 3 reference-surface, 7
  workspace-manifest, and 1 trap-guard tests;
- PocketIC suites: receipt-backed intent 1/1, sharding bootstrap 2/2,
  role attestation/capability 4/4, timer authority 2/2, lifecycle boundary
  3/3, root suite 28/28, and root/Wasm-store reconciliation 10/10; and
- instruction audit 38 passed with its one explicitly ignored external case.

Those PocketIC suites invoke the canonical CI Wasm builders for the affected
root, role-attestation, capability, role, and Wasm-store artifacts. Protocol
tests parse the checked-in Wasm-store DID and pin the root manual refill Candid
surface. No stale generated declaration failed its owner test.

The first broad unit run exposed two stale CLI state-report assertions that
still expected memory 29 to be reserved; they were corrected to the active
CycleTracker contract before the passing rerun. The first Clippy runs exposed
one needless test collection, two obsolete semicolons, and redundant private-
module visibility; all were corrected before the passing final gate.

Final exact searches found no removed symbol, retired module declaration,
retired package, runtime management-snapshot call, or refill timer. The only
active-source `randomness` hits are strict unknown-field rejection tests owned
by 0.98. The retained `*_with_candid` host call helpers have real cycles and
blob-storage consumers and are not aliases for the removed wrappers.

## 8. Contract, persistence, dependency, and version impact

| Surface | Impact |
| --- | --- |
| Stable schema | no memory ID, key/value encoding, record version, or migration change; CycleTracker metadata now tells the truth about existing bytes |
| Candid | no method or type change |
| JSON | no maintained key, version, or output change |
| CLI | no command, flag, or output change |
| Config TOML | breaking hard cut: explicit per-role randomness input is rejected; the guide describes the surviving strict schema |
| Public Rust | randomness config/metric shapes and unconsumed snapshot/duration/ICP helpers are hard-cut; control-plane/internal helpers are narrowed |
| Dependencies | host-only `serde_path_to_error 0.1.20` plus one direct host edge to existing Syn 3.0.3; compiled Syn versions remain 1.0.109, 2.0.119, and 3.0.3 |
| Workspace/package set | unchanged at 37 packages |
| Package versions | unchanged; no release-script or install URL mutation |

Backup/restore continues to use the current durable recovery model and typed
host ICP snapshot adapter. Current generation-bound pool pending reset,
checksum-bound staging, and fail-closed unknown-effect reconciliation were
retained because they are active safety invariants, not compatibility paths.

## 9. Deferred, unresolved, and external limitations

Deferred findings: none.

Unresolved findings: none.

No funded live-IC ICP-to-cycles conversion was executed. That journey requires
maintainer-controlled identity, ledger funds, network, and deployment state.
This is an execution-environment limitation, not deferred design work: source,
strict config, Candid, CLI, replay, stable state, metrics, and timer inventories
all converge on one controller-operated root-only manual workflow. Local test
evidence is not represented as a real mainnet conversion.

## 10. Final verdict

`PASS`.

Closure is based on repository-wide authority and reachability evidence, not
test success alone: every package and major subsystem is covered, every
candidate has an explicit disposition, all confirmed P1/P2 sediment was
remediated, removed concepts are unreachable through code/features/macros/
tests/scripts, no alternate auth/intent/recovery authority survives, and the
practical completion matrix passes.

The combined change set is published at immutable `v0.98.2`. Its exact
release identity and final `CLOSED` verdict are recorded in the canonical
0.98 closeout audit and status documents.
