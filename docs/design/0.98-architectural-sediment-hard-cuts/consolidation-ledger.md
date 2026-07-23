# 0.98.2 Consolidation Disposition Ledger

Date: 2026-07-22

Status: all investigated candidates resolved; validation evidence is recorded
in the supporting consolidation report and canonical 0.98 closeout audit.

Post-closeout note: the resolved status and totals below apply to the original
42 candidates published in `v0.98.2`. Fifteen later findings are recorded in the
[post-closeout supplement](#post-closeout-supplement). They are accepted design
scope; Slices D and E are published in `v0.98.10` and `v0.98.11`; Slices F
through J are published in `v0.98.12`; Slice K is published in `v0.98.13`;
Slice L is published in `v0.98.14`; Slice M is published in `v0.98.15`; Slice
N is published in `v0.98.16`; Slice O is published in `v0.98.17`; Slice P is
published in `v0.98.19`; Slice Q is published in `v0.98.20`; Slice R and the
call-builder correction are published in `v0.98.21`. Direct typed call
completion is published in `v0.98.22`; the related IC error-authority
refinement is in implementation for `0.98.23`. These refinements are not new
rows in the immutable fifteen-finding supplement.
They are not included in the immutable 0.98.2 totals.

## Counting rules

The disposition totals count the 42 unique `CANIC-098-CLOSE-*` candidate rows below.
Package and subsystem coverage tables map those candidates to the complete
workspace and are not counted again.

| Severity | Count |
| --- | ---: |
| P0 | 0 |
| P1 | 1 |
| P2 | 11 |
| Note | 30 |
| Total | 42 |

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

## Candidate dispositions

| ID | Severity | Candidate and authority evidence | Disposition |
| --- | --- | --- | --- |
| CANIC-098-CLOSE-BASE-001 | Note | Suspected consolidation ownership of randomness/raw_rand, project-protocol-stub, or LocalIntent race fixtures. The immutable baseline assigns the first to original Slice C and the others to released v0.98.0/v0.98.1 work. | REJECTED — no duplicate finding attribution inside the combined 0.98 line |
| CANIC-098-CLOSE-PKG-001 | Note | A workspace package might have no surviving purpose. Locked metadata finds 37 packages; reverse build/test/config/deployment consumers are recorded below. | RETAIN |
| CANIC-098-CLOSE-BUILD-001 | P2 | Line matching for `canic::build!` rejected valid multiline syntax and could count inert text. Role contract is the owner. | SIMPLIFY — parse and visit Rust syntax |
| CANIC-098-CLOSE-CONFIG-001 | P2 | State-manifest passive validation repeatedly reparsed the same selected config. The first `ConfigModel` is canonical. | CONSOLIDATE — pass one model through validation |
| CANIC-098-CLOSE-CONFIG-002 | P1 | Root configuration guidance described superseded fields/semantics and could direct operators to an invalid or unsafe contract. Strict `ConfigModel` is canonical. | CONSOLIDATE — current guide plus executable example |
| CANIC-098-CLOSE-DOC-001 | P2 | TESTING retained a compatibility breadcrumb for a removed annotation form. Current PocketIC helpers own the contract. | REMOVE |
| CANIC-098-CLOSE-HOST-001 | P2 | `canic-host::duration` had only its own tests after command parsing moved to CLI-specific owners. | REMOVE |
| CANIC-098-CLOSE-FACADE-001 | Note | Suspected removable `canic-core::cdk`. Macros, facade internals, control plane, role packages, and test canisters still require it. | RETAIN — hidden plumbing, not documented facade |
| CANIC-098-CLOSE-CP-001 | P2 | Control-plane `runtime` and `schema` modules were public without external or macro consumers. | SIMPLIFY — `pub(crate)`; semantic DTO/ID/state contracts stay public |
| CANIC-098-CLOSE-STATE-001 | P2 | Active CycleTracker memory 29 was declared reserved, producing duplicate/inaccurate state authority. | CONSOLIDATE — active record/data descriptor and snapshot conversion |
| CANIC-098-CLOSE-CAP-001 | P2 | Internal root-proof kind/mode/router/async verifier modeled multiple implementations beside one structural wire proof. | SIMPLIFY — direct structural verifier; wire proof retained |
| CANIC-098-CLOSE-MGMT-001 | P2 | Core take/load snapshot infra/ops/types/metrics had no production, macro, wasm, CLI, test, or script caller. Host backup owns snapshots. | REMOVE |
| CANIC-098-CLOSE-AUTH-001 | P2 | Root and issuer payload-kind enums had one variant after earlier auth paths disappeared. Fixed seed/domain bytes own the payload family. | SIMPLIFY — remove kind taxonomies and arguments; advance the corrected trust-chain inventory to fingerprinted v2 |
| CANIC-098-CLOSE-ICP-001 | P2 | Host call/start/stop/snapshot/version/display convenience methods were definition-only or test-only remnants of removed operator flows. | REMOVE |
| CANIC-098-CLOSE-ICP-002 | P2 | ICP parsing/run helpers were publicly re-exported despite only same-crate consumers. CLI/backup imports identify the true public boundary. | SIMPLIFY — private modules with exact crate/parent re-exports |
| CANIC-098-CLOSE-DEPS-001 | Note | Cargo Machete flagged three role-fixture normal `canic` edges. Those edges intentionally carry role features separately from featureless build dependencies. | RETAIN — exact fixture-local Machete metadata |
| CANIC-098-CLOSE-DEPS-002 | Note | Direct host Syn might add a compiled version. The lock already contains Syn 3.0.3 through active dependencies; inverse tree shows the host selects it. | RETAIN — no added version |
| CANIC-098-CLOSE-ABI-001 | Note | Suspected `repr(u16)` enums reducible to `u8`. There is no `repr(u16)`; the sole `repr(u8)` is a cryptographic canonical-domain byte. | REJECTED |
| CANIC-098-CLOSE-CONFIG-003 | Note | Suspected accepted-but-ignored current config. Schema-to-render/runtime/host searches cover every field; removed randomness is strict 0.98 rejection input only. | RETAIN |
| CANIC-098-CLOSE-FEAT-001 | Note | Suspected stale Cargo features. All non-metrics features gate code/dependencies; empty `metrics` is an intentional role build-selection marker consumed by build support/contracts. | RETAIN |
| CANIC-098-CLOSE-DEPS-003 | Note | Suspected workspace dependency inversion. Metadata shows core/macro bottoms, control-plane -> core, facade -> product internals, host/CLI above runtime, and backup independent. | RETAIN |
| CANIC-098-CLOSE-AUTH-002 | Note | Suspected competing auth models. Role attestation and delegated chain-key/issuer-token flows have distinct current duties and one endpoint authorization boundary. | RETAIN |
| CANIC-098-CLOSE-REPLAY-001 | Note | Suspected competing LocalIntent/replay/receipt authorities. Local intents and receipt-backed intents serve distinct current contracts; Canic-owned namespaces are isolated in the one receipt store. | RETAIN |
| CANIC-098-CLOSE-REPLAY-002 | Note | Suspected defect because no raw exact-receipt dump endpoint exists. Same-ID replay, typed responses/errors, workflow records, capacity, and controller logs are the supported observation contract. | REJECTED — a dump endpoint would be new security-sensitive surface, not sediment removal |
| CANIC-098-CLOSE-BACKUP-001 | Note | Suspected alternate backup/restore machinery. Plans, journals, receipts, artifact publication, and reconciliation all converge on `canic-backup` plus one host executor. | RETAIN |
| CANIC-098-CLOSE-POOL-001 | Note | Suspected retired `pending reset` shortcut. The only current form is generation-bound pool recycling with durable exclusion and bounded recovery. | RETAIN |
| CANIC-098-CLOSE-RECOVERY-001 | Note | Terms rollback/staging/unknown-outcome suggested old compatibility paths. Tracing finds transactional config rollback, private checksum-bound artifact staging, and fail-closed unknown-effect containment. | REJECTED — current safety invariants, not superseded architecture |
| CANIC-098-CLOSE-TIMER-001 | Note | Suspected parallel timer authorities. All CDK scheduling routes through `TimerWorkflow`; subsystem schedulers provide work/deadlines only. | RETAIN |
| CANIC-098-CLOSE-TIMER-002 | Note | Suspected timed ICP refill. No refill timer key/callback exists; conversion is controller-triggered root-only manual workflow. | REJECTED |
| CANIC-098-CLOSE-ICP-003 | Note | No funded live-IC refill journey ran in this audit. Source/Candid/CLI/state/replay/metrics inventories still prove one current workflow. | RETAIN with explicit external-validation limitation |
| CANIC-098-CLOSE-TOPO-001 | Note | Suspected duplicate topology/registry/pool owners. Records live in storage/model; ops project/mutate; placement is the shared creation authority; host registry is operator evidence only. | RETAIN |
| CANIC-098-CLOSE-STABLE-001 | Note | Suspected abandoned stable fields/variants. State contracts, record conversions, capacity/index tests, and lifecycle rebuilds justify current shapes; no ID is renumbered. | RETAIN |
| CANIC-098-CLOSE-VERSION-001 | Note | `V1` names suggested compatibility sediment. Remaining names identify current persisted JSON/Candid/evidence schemas with strict version-1 validation and no fallback reader. | RETAIN |
| CANIC-098-CLOSE-GEN-001 | Note | Suspected stale generated bindings/descriptors. Only canonical Wasm-store DID and two external blob fixtures are tracked; protocol tests consume them; `.icp` output is generated. | RETAIN |
| CANIC-098-CLOSE-FIXTURE-001 | Note | Suspected obsolete test canisters. Each test canister maps to a current PocketIC behavior or build contract; the 0.98 race auxiliaries are already absent. | RETAIN |
| CANIC-098-CLOSE-PROBE-001 | Note | Suspected dead audit/sandbox packages. Minimal, metrics, root, leaf, scaling probes measure role/wasm contracts; blank sandbox is documented manual compile-drift support. | RETAIN |
| CANIC-098-CLOSE-CLI-001 | Note | Suspected dead CLI commands/operational tools. Top-level dispatch, help, tests, and host/backup consumers cover each command family; no snapshot command survives. | RETAIN |
| CANIC-098-CLOSE-ERROR-001 | Note | Suspected obsolete wrapper taxonomies/string loss. Current layers preserve typed config, host process, backup/restore, auth, replay, and platform causes until text/JSON rendering boundaries. | RETAIN |
| CANIC-098-CLOSE-MGMT-002 | Note | Suspected unreachable remaining production adapters. After snapshot removal, lifecycle/settings/install/delete/status/cycles/HTTP/chain-key calls all have workflow or feature-gated consumers. | RETAIN |
| CANIC-098-CLOSE-TOOL-001 | Note | Suspected CI/build/deployment tooling for removed structures. All current scripts map to Make/CI/docs/release guards or intentional maintainer helpers; stale 0.98 symbols are absent. | RETAIN |
| CANIC-098-CLOSE-HISTORY-001 | Note | Historical reports mention removed snapshot/randomness/auth models. Their dates/snapshot preambles or explicit superseded labels make them evidence, not current instructions. | REJECTED — do not rewrite immutable history |
| CANIC-098-CLOSE-VIS-001 | Note | `unreachable_pub` reported roughly 1,700 existing items across deliberate cross-crate/facade/test surfaces. Raw lint volume does not prove obsolete responsibility. Exact consumer tracing produced C009/C015 instead. | REJECTED |

## Post-closeout supplement

Date: 2026-07-22

These rows record findings from a later source-and-history pass. They narrow
specific original retention claims without rewriting the evidence or totals
for the published 42-candidate audit. `STABLE-001`, `CONFIG-003`, and
`CLI-001` remain accurate for the shapes examined at the original closeout but
were not exhaustive proofs against these newly traced tails.

| ID | Severity | Candidate and authority evidence | Disposition |
| --- | --- | --- | --- |
| CANIC-098-POST-STATE-001 | P2 | The final core subnet-auth field was removed in `0.65.17`, but an empty stable record/cell, DTO, mapper, query, root endpoint, cascade slot, Candid shape, probe, and false restore invariant remained. The control-plane subnet state is separate and meaningful. | FIXED in `v0.98.10` — complete core surface deleted; ID 17 is excluded from active allocations and permanently rejected by the canonical allocation validator; control-plane ID 84 retained |
| CANIC-098-POST-AUTH-001 | P2 | `root_proof_mode` accepts only `chain_key_batch`; `RootProofMode` has one variant; verification rejects an impossible alternative; canonical encoding still emits its obsolete tag at three hash positions. The removed alternative disappeared in `0.76.6`. | FIXED in `v0.98.11` — selector and three constant bytes removed from the sole V1 format; configured proof/registry floors advance root counters and reject restored stale issuer material; no V2 or compatibility verifier added |
| CANIC-098-POST-AUTH-002 | P2 | Delegated-token access still forwards endpoint call kind after update-token consumption was removed in `0.61.0`; the terminal verifier parameter is unused. Metrics and performance still consume the shared call-kind type. | FIXED in `v0.98.12` — call kind removed only from delegated verification; endpoint identity, performance, and metrics retain it |
| CANIC-098-POST-LIFECYCLE-001 | P2 | Non-root init bytes are cloned through API/lifecycle/timer/workflow layers although internal bootstrap documents them as unused; the application hook independently consumes the original args. | FIXED in `v0.98.12` — internal bootstrap is argument-free while both generated application hooks retain original init bytes |
| CANIC-098-POST-COST-001 | Note | `CostGuardPermit` stores cost class, quota key, payer, and a private sentinel that no settlement or capability path reads; only the quota and reservation intent IDs are authoritative. | FIXED in `v0.98.12` — permit stores only the two durable intent identities and remains privately constructed |
| CANIC-098-POST-ADOPTION-001 | P2 | The top-level adoption-report warnings vector has been empty at its sole production constructor since introduction, yet remains in JSON and evidence-summary projection. Finding-local warning producers are distinct and active. | FIXED in `v0.98.12` — dead top-level JSON field and envelope mapping removed; role and observed-canister warning producers retained |
| CANIC-098-POST-CLI-001 | Note | Private auth CLI parsing wraps its sole renewal-status options in a one-variant enum and immediately unwraps it through one match arm. | FIXED in `v0.98.12` — parser returns renewal-status options directly with unchanged command behavior |
| CANIC-098-POST-CDK-001 | P2 | Internal XRC wire bindings have no consumer after Canic stopped wrapping that protocol. The 8-, 16-, 32-, and 256-byte bounded-string aliases also have no consumer; the generic type and its 64-/128-byte aliases still enforce active state limits. | FIXED in `v0.98.13` — XRC module/export and unused aliases deleted; generic Serde/Candid/stable validation and active aliases retained |
| CANIC-098-POST-TEST-001 | P2 | Repo-only test support retains a never-consumed upgrade wrapper, an attestation verifier-cache alternative whose final test disappeared in `0.65.0`, unused fixture projections, and a role constant kept alive only by its own test. | FIXED in `v0.98.14` — orphan helpers and alternative deleted; sole issuer baseline and all behaviorally consumed role constants retained |
| CANIC-098-POST-ICRC21-001 | P2 | `Icrc21Dispatcher::register_static_with` has no repository caller or active documentation and constructs one fixed English generic-display response beside the complete typed `register` contract. | FIXED in `v0.98.15` — convenience deleted; typed registration and `consent_message` dispatch retained |
| CANIC-098-POST-ICRC2-001 | P2 | The generic ICRC-2 API, workflow allowance rule, ops adapter, infra calls, wire bindings, known-ledger metadata, and platform metric dimension form one self-contained call chain with no facade, endpoint, canister, CLI, test, or script entrypoint. | FIXED in `v0.98.16` — complete chain and dead metric dimension deleted; independent root ICP-refill ledger/CMC path retained |
| CANIC-098-POST-IC-HELPERS-001 | P2 | The facade grouped a useful instrumented call builder with a consumerless HTTP stack and a forwarding-only build-network facade after 0.97 removed generic CDK ownership. Repository-only caller counts did not establish whether the builder itself was intended downstream developer experience. | CORRECTED in `v0.98.21` — HTTP/build-network facades and generic CDK relays remain deleted; one canonical `canic::api::call` builder is restored over the single retained call transport and metrics authority |
| CANIC-098-POST-CDK-RELAY-001 | P2 | `canic-core::cdk` still relays upstream IC runtime APIs to Canic runtime and macro code after the public facade hard cut; its separate time module only forwards that clock into `IcOps`, and its stable-structures inventory exports many types with no repository consumer. | FIXED in `v0.98.19` — runtime and hidden macro plumbing import upstream `ic-cdk` directly, time is owned by `IcOps`, and unused stable exports are removed without an alias |
| CANIC-098-POST-ICP-ACCOUNT-001 | P2 | Generic CDK `Account`/`Subaccount` helpers are consumed only by manual ICP refill; custom ordering/hash/default-subaccount semantics have no production caller, and transfer resume rebuilds the CMC account instead of consuming the exact identity already persisted in the refill record. | FIXED in `v0.98.20` — the wire account is adapter-local, unused semantics and aliases are removed, and resume consumes the persisted CMC owner/subaccount |
| CANIC-098-POST-ICRC103-001 | P2 | Global config can advertise ICRC-103 through the supported-standards query even though Canic has no allowance-list endpoint, required ledger metadata, or surviving generic ICRC-2 allowance authority. | FIXED in `v0.98.21` — unsupported config/render/query branches removed; the result lists only ICRC-10 and configured ICRC-21 |

## Package coverage

All workspace members from locked Cargo metadata are listed exactly once.

| Package | Current responsibility / consumer proof | Result |
| --- | --- | --- |
| `canic` | Public facade, build/start/api macros; every role package and protocol tests | RETAIN; C005 documentation corrected |
| `canic-core` | Runtime layers, DTO/model/state, platform ops, auth/replay/timers; facade/control-plane/host consumers | RETAIN after C010-C013/CANIC-098-CLOSE-MGMT-001 remediation |
| `canic-macros` | Procedural endpoint parsing/expansion; direct facade dependency and macro tests | RETAIN |
| `canic-control-plane` | Root/Wasm-store policy, storage, workflows, state contract; facade feature consumers | RETAIN after C009 narrowing |
| `canic-wasm-store` | Built-in store canister, canonical DID, packaging/build probes | RETAIN |
| `canic-host` | Config/build/install/deployment truth/ICP adapters; CLI and testing consumers | RETAIN after C003/C004/C007/C014/C015 |
| `canic-cli` | Operator binary and rendering; top-level dispatch plus command tests | RETAIN |
| `canic-backup` | Sole backup/restore durable recovery library; CLI executor consumer | RETAIN |
| `canic-testing-internal` | Shared PocketIC build/install/query harness; `canic-tests` consumer | RETAIN |
| `canic-tests` | Cross-canister, lifecycle, replay, auth, topology, payload, blob suites | RETAIN |
| `blob_storage_cashier_mock` | Cashier boundary for `pic_blob_storage` | RETAIN |
| `blob_storage_probe` | Blob billing/gateway behavior for `pic_blob_storage`; state-manifest fixture | RETAIN |
| `delegation_issuer_stub` | Issuer proof/token canister embedded by delegation root and attestation harness | RETAIN |
| `delegation_root_stub` | Root auth/attestation integration canister and embedded project roles | RETAIN |
| `intent_authority` | Sole receipt-backed intent PocketIC authority after 0.98 fixture deletion | RETAIN |
| `payload_limit_probe` | Ingress payload-limit PocketIC contract | RETAIN |
| `project_hub_stub` | Delegated project-hub audience/topology integration role | RETAIN |
| `project_instance_stub` | Delegated project-instance audience integration role | RETAIN |
| `runtime_probe` | Lifecycle/runtime status/timer integration canister | RETAIN |
| `sharding_root_stub` | Root side of sharding bootstrap PocketIC proof | RETAIN |
| `leaf_probe` | Standalone leaf role/wasm audit profile | RETAIN |
| `root_probe` | Standalone root/control-plane audit and shared test-config root | RETAIN |
| `scaling_probe` | Standalone scaling role/wasm audit profile | RETAIN |
| `canister_minimal` | No-default-feature wasm floor | RETAIN |
| `canister_minimal_metrics` | Metrics-enabled wasm delta floor | RETAIN |
| `canister_sandbox_blank` | Documented manual compile-drift sandbox outside production tests | RETAIN |
| `demo_fleet_app` | Demo application leaf role and deploy/build documentation | RETAIN |
| `demo_fleet_root` | Demo root/control-plane role | RETAIN |
| `demo_fleet_user_hub` | Demo sharding hub role | RETAIN |
| `demo_fleet_user_shard` | Demo user shard role | RETAIN |
| `canister_app` | Test-fleet application role used by root/PocketIC suites | RETAIN |
| `canister_root` | Full test-fleet root used by cumulative root/PocketIC suites | RETAIN |
| `canister_scale` | Test-fleet scale child role | RETAIN |
| `canister_scale_hub` | Test-fleet scaling hub role | RETAIN |
| `canister_test` | General test-fleet role for runtime/root integration | RETAIN |
| `canister_user_hub` | Test-fleet sharding hub and sharding bootstrap consumer | RETAIN |
| `canister_user_shard` | Test-fleet sharding leaf and backup/restore journey role | RETAIN |

## Major subsystem coverage

| Subsystem | Canonical owner and evidence | Candidate links / result |
| --- | --- | --- |
| Public facade/macros | `canic`, `canic-macros`, hidden core plumbing; protocol/reference/macro tests | FACADE-001 RETAIN |
| Config ownership/projection | strict core schema -> one parsed host model -> build/runtime projection | CONFIG-001/002 fixed; CONFIG-003 RETAIN |
| Role/package graph | metadata plus exact normal/build dependency contract and syntax-aware build macro validation | BUILD-001 fixed; DEPS-001 RETAIN |
| Authentication/authorization | endpoint auth; attestation and delegated chain-key/token authorities | AUTH-001 fixed; AUTH-002 RETAIN |
| Intent/replay/receipts | one durable receipt store, reserved namespace, pure decision + ops mutation + workflow effects | REPLAY-001/002 resolved |
| ICP refill | root config, stable record/index, manual endpoint/CLI, receipt workflow, no timer | TIMER-002 REJECTED; ICP-003 limitation |
| Backup/restore | `canic-backup` durable model plus host executor and CLI commands | BACKUP-001 RETAIN; RECOVERY-001 REJECTED |
| Management/platform calls | infra raw calls -> ops single effects -> workflows | MGMT-001 removed; MGMT-002 RETAIN |
| Timers/lifecycle | one `TimerWorkflow`; synchronous restore and deferred hooks | TIMER-001 RETAIN |
| Topology/registry/placement | storage/model authority, ops projections, shared placement workflow | TOPO-001 RETAIN |
| Pool reset/recovery | generation-bound pending reset and bounded scheduler | POOL-001 RETAIN |
| Wasm-store/template lifecycle | root binding state plus store-local one-way GC | TOPO-001 RETAIN |
| DTO/domain/view/record boundaries | passive DTO, authoritative model, record persistence, internal views, `*Data` snapshots | STABLE-001 RETAIN |
| Stable memory/state manifests | core/control-plane descriptors plus host role applicability | STATE-001 fixed; STABLE-001 RETAIN |
| Candid/generated clients | canonical checked-in DID/fixtures plus build-generated `.icp` artifacts | GEN-001 RETAIN |
| JSON/evidence versions | strict current version-1 backup/restore/deployment/provenance contracts | VERSION-001 RETAIN |
| Host/operator adapters | typed ICP CLI, deployment truth, status/medic/inspect/state evidence | ICP-001/002 fixed; CLI-001 RETAIN |
| Errors/diagnostics | typed causes through boundary renderers | ERROR-001 RETAIN |
| PocketIC/test canisters | testing-internal harness plus one current fixture per behavior | FIXTURE-001 RETAIN |
| Audit probes/sandbox | current wasm/role measurements and documented manual sandbox | PROBE-001 RETAIN |
| CI/release/deployment scripts | Make/Actions/guard reverse consumers and maintainer helper policy | TOOL-001 RETAIN |

## Contract and persistence impact

| Surface | 0.98.2 result |
| --- | --- |
| Stable memory | no ID, record encoding, or migration change; CycleTracker metadata corrected only |
| Candid | no method or type change |
| JSON | no maintained key/version/output change |
| CLI | no command, flag, or output change |
| Config TOML | breaking hard cut: explicit per-role randomness input is rejected; guide corrected to the surviving strict schema |
| Public Rust | randomness shapes and unconsumed core snapshot/host helper surfaces hard-cut; control-plane/core plumbing hidden/narrowed |
| Dependencies | host-only `serde_path_to_error 0.1.20` plus one direct host edge to existing Syn 3.0.3; no added Syn version; Machete clean |
| Package versions | unchanged by policy |
