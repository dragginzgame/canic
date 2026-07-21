# Canic Architectural Sediment Audit

Audit date: 2026-07-21

Audited baseline: clean main commit ee58f6ef3 (workspace version 0.96.3)

Scope: all 40 workspace packages, their normal/build/dev dependency edges,
runtime and host source, proc macros, generated-code boundaries, current
contracts, tests, fixtures, operations documents, active designs, prior audits,
and selective history.

The worktree was clean when this baseline was frozen. During validation,
uncommitted receipt-reclamation work appeared in canic-core files. Those
user-owned, incomplete changes were preserved, were not treated as findings,
and are outside this report. Where they prevented compilation, the affected
check is reported as inconclusive rather than attributed to the audited commit.

## Executive verdict

Canic has a recognisable and mostly coherent current architecture. Runtime
layering, stable-memory allocation, lifecycle ordering, timer ownership,
management-call routing, topology state, and backup/restore recovery each have
a credible canonical owner. The recent hard-cut work has removed many genuine
parallel paths rather than hiding them behind compatibility layers.

The repository is not yet the simplest coherent version of that architecture.
Six bounded sediment or proof gaps remain:

- the role-package dependency validator protects only the canic package, while
  a current root fixture also selects canic-control-plane directly;
- workspace defaults and dependency renaming mean the direct role declaration
  is not yet the complete feature/source-level authority it claims to be;
- the public canic::cdk facade is simultaneously a human API and macro plumbing;
- a fully parsed, validated, generated, and displayed randomness configuration
  has no runtime consumer, alongside an unreachable raw_rand adapter;
- an old LocalIntent external-call fixture demonstrates rollback on transport
  failure even though the current receipt-backed contract explicitly forbids
  that inference; and
- one shared protocol test package survived the hard cut that deleted the
  descriptor architecture it was created to support.

In addition, replay receipts correctly preserve uncertain external effects,
but the operator runbook asks operators to inspect exact receipt state without
providing a maintained per-operation observation path. This is a liveness and
operability gap, not permission to reset state or repeat an unknown effect.

No P0 was proven at the audited baseline. Multiple independently bounded
hard-cut lines are justified: the already designed 0.97 dependency/CDK line,
a randomness-contract deletion, a test-fixture deletion, and a narrowly scoped
replay-evidence/reconciliation line.

## Reconstructed architecture

### Workspace crate map

#### Product and support crates

| Workspace crate | Intended current responsibility | Surface class |
| --- | --- | --- |
| canic | Downstream facade; role build/start macros; curated runtime APIs, DTOs, IDs, protocols, and public endpoint macros | Intended downstream API plus hidden macro/build plumbing |
| canic-core | Framework runtime implementation inside a role Wasm: model, ops, policy, workflow, lifecycle, storage, config, auth, topology, replay, timers, and metrics | Mostly cross-crate/internal framework API; selected facade-backed downstream API |
| canic-macros | Procedural expansion of authenticated query/update endpoints | Proc-macro API; generated-code requirements |
| canic-control-plane | Root and built-in Wasm-store lifecycle, template publication, pool/control-plane state, and root orchestration | Cross-crate runtime API, normally reached through canic |
| canic-wasm-store | Built-in Wasm-store role/package and its canonical Candid artifact | Runtime role plus external Candid contract |
| canic-host | Non-CLI operator/build library: discovery, Cargo role validation, release materialisation, deployment truth, install/upgrade planning, ICP command execution, and state-contract audits | Cross-crate host API consumed by CLI and test support |
| canic-cli | User command parsing, presentation, and delegation into host/backup authorities | CLI contract |
| canic-backup | Backup and restore domain, plans, durable journals, artifact integrity, locking, execution, recovery, and verification | Host library and JSON/artifact contract |
| canic-testing-internal | Shared Canic-specific Wasm builders, PocketIC setup, fixtures, and attestation helpers | Test-only workspace API |
| canic-tests | Integration and PocketIC system proofs over real built role Wasm | Test package; no downstream product API |

#### Audit, sandbox, test, and fleet role packages

Every package below is a workspace crate even when its Rust surface is only a
cdylib. Its intended public boundary is the generated/exported Candid interface
or a test/build artifact, not a reusable Rust library.

| Packages | Intended responsibility |
| --- | --- |
| leaf_probe, root_probe, scaling_probe | Audit probes for role/build/dependency and generated runtime surfaces |
| canister_minimal, canister_minimal_metrics | Minimal role profiles proving explicit no-default and metrics-enabled builds |
| canister_sandbox_blank | Local sandbox lifecycle/profile fixture |
| blob_storage_cashier_mock, blob_storage_probe | Blob-storage billing/cashier and endpoint integration fixtures |
| delegation_issuer_stub, delegation_root_stub | Delegated-auth issuer/root trust-chain and root bootstrap fixtures |
| payload_limit_probe | Endpoint payload-limit behaviour fixture |
| project_hub_stub, project_instance_stub | Project placement/delegated endpoint fixtures |
| runtime_probe | Runtime API, intent, lifecycle, timer, and diagnostic integration fixture |
| sharding_root_stub | Deliberately small non-Canic lifecycle/root peer used by sharding bootstrap tests |
| intent_authority, intent_client, intent_external | Intent conformance plus an older external-call race fixture; the latter two are sediment as detailed below |
| project-protocol-stub | Surviving package from a deleted protected-descriptor architecture; proven dead below |
| demo_fleet_app, demo_fleet_root, demo_fleet_user_hub, demo_fleet_user_shard | Maintained demo fleet roles |
| canister_app, canister_root, canister_scale, canister_scale_hub, canister_test, canister_user_hub, canister_user_shard | Maintained test fleet roles and end-to-end role combinations |

### Permitted dependency directions

The intended runtime shape is:

    role package
      -> canic facade
           -> canic-core
           -> canic-macros
           -> canic-control-plane only when the role selects a control-plane feature

A role may separately have one build-dependency edge to canic for build.rs.
Normal runtime edges and build/proc-macro edges are different evidence classes.
Resolver 2 is set at Cargo.toml:44.

Shared application/domain libraries are intended to depend on ordinary
upstream crates such as candid or ic-cdk, not Canic. The facade states this
directly at crates/canic/src/lib.rs:9-13. A final role package owns Canic
lifecycle, endpoint semantics, framework feature selection, and Canic-specific
adapters.

Host direction is:

    canic-cli -> canic-host
              -> canic-backup
    canic-host -> canic-core / canic-control-plane contract types
    canic-testing-internal -> product crates for test construction
    canic-tests -> test support plus built role artifacts

Within runtime crates the normative direction is endpoints -> workflow ->
policy -> ops -> model. DTOs are passive boundary data; stable records remain
inside storage; views are internal projections; lifecycle adapters restore
synchronous invariants and schedule workflow.

### Runtime, host, build, and proc-macro ownership

- Runtime ownership is canic-core, with root/Wasm-store additions in
  canic-control-plane and the curated facade in canic.
- Host mutation and observation are owned by canic-host; backup/restore
  mutation and durable local artifacts are owned by canic-backup.
- build.rs entry points are exposed through canic::__build and must validate
  the same role evidence the later build consumes.
- canic-macros owns endpoint expansion. Generated code may reach public,
  doc-hidden paths because expansion executes in the downstream crate, but that
  does not justify a human-facing dependency facade.
- The repository currently has one intended direct normal canic edge per role
  and a separately allowed build edge. The executable checker does not yet
  fully enforce the Canic-owned package closure.

### Authority inventory

| Concept | Canonical owner | Representation and validation | Execution/persistence/public projection | Competing path or result |
| --- | --- | --- | --- | --- |
| Role/package identity | canic-core role contract; host package validator | CanisterRole, package.metadata.canic fleet/role, compiled config; host validates exact manifest package | Build marker and runtime Env state; CanisterRole facade | No zero-owner case; package identity is coherent |
| Dependency graph | canic-host role_contract::package | Cargo metadata normal edges, selected role package, direct canic edge | Pre-build/medic validation; no persistence | Validator protects canic only; direct control-plane bypass exists |
| Framework features | Final role manifest, interpreted by canic-core role policy | Direct dependency features plus defaults; role feature catalog | Cargo feature union drives Wasm | Workspace canic default metrics and accepted aliases dilute sole authority |
| Endpoint declaration | canic-macros, facade attributes | canic_query/canic_update parse and validate access/payload clauses | Generated ic-cdk endpoint delegates to access then handler | Raw CDK endpoints exist only in purpose-built fixtures; public CDK facade remains broad |
| Caller/authentication proof | canic-core access/auth ops | Raw caller or validated delegated identity; token/proof schema and binding checks | Macro access stage resolves identity before handler | No production auth bypass proven |
| Authorization/grants | Endpoint access expression and auth policy | AccessContext, endpoint guard expressions, issuer/root/local bindings | Endpoint rejects before workflow | No second mutation-layer auth authority proven |
| Lifecycle | canic start macros -> core lifecycle adapters -> workflow | Init payload, compiled config, restored Env/state-contract checks | Synchronous restore, zero-delay async bootstrap, then user hooks | No alternate production lifecycle path proven |
| Configuration | canic-core schema/Validate/parse; host consumes same model | canic.toml -> typed ConfigModel -> generated compiled model | ConfigOps/runtime and host diagnostics | Randomness subtree is accepted and projected but not executed |
| Stable memory IDs | canic-core role_contract::allocation | Closed allocation definitions and core/control-plane ranges | ic-memory/StableCell/StableBTreeMap records; state manifest and ledger diagnostics | No duplicate allocator proven |
| Stable schema | Owning runtime storage module plus state_contract | Named Record types, schema versions, Storable bounds, allocation descriptors | Stable memory; controller diagnostics and state audit | No dual decoder/dual write in current supported schema proven |
| Local intent | canic-core intent workflow/ops/storage | Locally decidable expirable reservation | Stable intent records and totals; LocalIntentApi | Old test uses it across an externally uncertain call, outside its boundary |
| Application receipt-backed intent | canic-core generic reservation; downstream adapter owns domain receipt | OperationId, PayloadBinding, pending/terminal evidence, CAS revision | Stable receipt primary and replay adjunct; public Rust facade | Current in-repo conformance is valid; older buy fixture teaches a conflicting path |
| Root replay/command journal | canic-core replay model/workflow/ops | ReplayReceipt, status, actor/payload binding, effect descriptor, staged response | Stable replay receipt; replay decisions/errors | Exact operator observation/reconciliation is incomplete for some unknown outcomes |
| Topology/registry | core topology storage/ops; root control-plane workflow mutates it | Canister/subnet/app registries and role bindings | Stable registry records; root propagation and query DTOs | No duplicate runtime registry owner proven |
| Creation/install/upgrade/stop/start | Core/control-plane workflow, then ops, then management infra | Typed request/capability/replay/cost evidence | Management call after durable marker; response/receipt commit | No direct production management call bypass proven |
| Deployment truth | canic-host | Prepared artifacts, plans, observations, receipts, authority reports | Local durable reports and CLI projections | Observations are evidence, not runtime truth; no competing mutator proven |
| Backup plan/download/verification | canic-backup | BackupPlan, execution/download journals, operation receipts, manifest | Durable JSON, snapshots, verified artifacts | No second backup runner proven |
| Restore | canic-backup | Restore plan/apply journal, checksum binding, command and operation receipts | Upload/stop/load/start/verify with restart reconciliation | No host/CLI alternate restore authority proven |
| Snapshot identity/artifact durability | canic-backup, using host executor boundary | Exact snapshot IDs, inventory deltas, hashes, private stages, fsync/rename | Backup layout and restore journal | No text-ID or unverified source fallback remains |
| Timers | canic-core TimerWorkflow; TimerOps is sole raw IC adapter | Closed TimerKey plus opaque application IDs and directives | In-memory scheduling reconstructed from durable work on lifecycle | Inventory guard proves one raw timer owner |
| Metrics/operational evidence | core metric IDs/recorders; host/CLI report projections | Closed metric enums and typed evidence/report structs | Prometheus/query/CLI/JSON projections | RawRand metric variants have no producer reachable from a supported runtime |
| Generated code/macro plumbing | canic-macros plus canic hidden internals | Token expansion into canic paths | Downstream crate compilation | Human cdk facade and hidden plumbing are conflated |
| Error taxonomy | Typed error at owning layer, projected at facade/CLI/Candid boundary | InternalError origin/class and owner-specific errors | Canic Error, CLI structured/text/JSON errors | Some external strings are legitimate test fixtures; no broad duplicate taxonomy proven |

The only ownership rule not confidently derivable is the long-term public
status of icp-refill: the design declares it an opt-in downstream capability,
but no current workspace role enables it. That is recorded as an observation,
not presumed dead code.

### Public-surface and reachability classification

| Crate/group | Actual public surface | Classification |
| --- | --- | --- |
| canic | access, api, dto, ids, prelude, protocol, memory, Error, endpoint/storable/build/start macros, constants, cdk | Intended downstream facade except cdk, which is superseded human surface plus macro plumbing |
| canic-core | Selected api/cdk/dto/ids/log/memory/perf/protocol/replay-policy exports and doc-hidden facade plumbing; most implementation modules are crate-private | Cross-crate framework API; cdk broad facade should shrink |
| canic-macros | canic_query and canic_update | Intended proc-macro API |
| canic-control-plane | api, dto, ids, runtime, schema, state_contract | Cross-crate framework surface normally mediated by canic; delegation_root_stub bypass is not justified |
| canic-wasm-store | Exported canister endpoints and checked-in wasm_store.did | Generated/external contract |
| canic-host | Public host models, planners, validators, executors, reports, and typed errors | Workspace API consumed by CLI/testing; not runtime |
| canic-backup | Public models, layouts, planners, runners, reports, and typed errors | Workspace host API plus durable JSON/artifact contract |
| canic-cli | Command options, top-level run/version/render/error entry points | Binary/CLI integration surface |
| canic-testing-internal, canic-tests | Builders/helpers and integration tests | Test-only exposure |
| All audit/demo/fleet role crates | Candid endpoints and build artifacts; occasional constants/helpers internal to fixture | Generated/external or test-only |
| project-protocol-stub | Two public CanisterRole constants with no consumer | Accidental obsolete compatibility/test surface |

The 0.92 module-surface audits already removed several accidental exports.
This audit found no reason to reopen those cuts. Public reachability alone was
not treated as retention evidence.

### Main end-to-end journeys

| Journey | Entry and ownership transitions | Durable/external boundary and restart behaviour | Bypass, obsolete stage, and strongest coverage |
| --- | --- | --- | --- |
| 1. Role discovery -> graph -> features -> build | CLI/build discovers exact workspace/package -> host Cargo metadata validator -> core pure role contract -> canic build macro | Cargo.lock/metadata are evidence; compiled config and role marker feed the build | Graph protects canic but not its owned closure; workspace defaults and aliases weaken authority. Unit graph fixtures are strong but miss the live direct control-plane edge |
| 2. Endpoint -> auth -> authorization -> handler | canic macro emits CDK endpoint -> captures caller/resolves delegated identity -> evaluates access expression -> invokes handler | Auth state/proofs may be stable; rejected calls do not enter workflow | Macro hard-codes ::canic while validator accepts rename. Macro tests and auth PocketIC tests cover maintained path |
| 3. Install -> memory -> lifecycle | Generated init -> synchronous lifecycle adapter -> role allocation/state restore -> zero-delay bootstrap -> user hooks | Stable memory is opened under canonical IDs before async work; upgrade reconstructs timers/indexes | No alternate product lifecycle path. lifecycle_boundary_guard and v0.91.6 upgrade evidence are strongest |
| 4. Config -> validation -> runtime/diagnostics | TOML -> core typed schema/Validate -> host/build projection -> generated compiled ConfigModel -> runtime ConfigOps and diagnostics | Source and compiled config are build evidence; no durable migration format | Randomness stops after projection. Config tests validate a promise runtime does not implement |
| 5. Management command -> intent/effect/receipt | Endpoint/capability -> replay and cost guard -> effect marked in flight -> workflow/ops -> management infra -> staged response/settlement/commit | Replay receipt and cost intents are written before/after the side effect | Normal path is single and typed; inventory guards cover costed operations |
| 6. Interruption around external mutation | Pre-effect failures can settle safely; post-marker failures preserve in-flight/recovery-required receipt; same request may recover only proven accounting/response gaps | Unknown external status is retained indefinitely and fails closed | Cost/response recovery is automatic; unknown effect/state projection lacks exact operator observation/reconciliation. This is safe but incomplete |
| 7. Topology mutation -> registry -> completion | Root capability/workflow -> creation/install -> topology ops/stable registry -> propagation -> replay completion | Registry and replay evidence bracket mutation; partial cascades keep typed evidence | No competing registry writer proven. Root/PocketIC placement and reconcile tests are strongest |
| 8. Backup prepare -> snapshot/download -> verify/commit | CLI -> backup planner/preflight -> execution journal -> host executor -> snapshot/download -> checksum/artifact verification -> manifest | Pending claim is durable before effects; receipts and fsync/rename publish after; exact inventory/artifact evidence may reconcile | No alternate runner. The 0.94 106-case matrix and real crash proofs are strong |
| 9. Backup interruption -> resume | Runner locks journal/command tree -> reads pending operation -> reconciles lifecycle, snapshot delta, download stage, or artifact -> records receipt | Unknown command outcome is rejected; only exact evidence is adopted | No blind reset. Process-death and publication-barrier tests cover maintained entrypoints |
| 10. Restore prepare -> upload/stop/load/start/verify | CLI -> plan/integrity -> apply journal -> private verified upload stage -> stop/load/start -> member/deployment verification | Each mutation is claimed pending and terminally receipted; checksum and snapshot ID are fixed | No host restore duplicate. Restore runner and closeout matrix cover all stages |
| 11. Restore interruption after external commit | Command-lifetime lock contains live descendants; restart observes status or snapshot inventory; load repeats only after stopped proof; verification is read-only and repeatable | Committed stop/start/upload are adopted; ambiguous inventory halts; load unknown is bounded by stopped proof and command-tree quiescence | 106/106 frozen cases pass; docs/audits/reports/2026-07/2026-07-20/0.94-restore-recovery-matrix-closeout.md:46-83 |
| 12. Upgrade/schema transition | post_upgrade synchronous restore -> state-contract/allocation validation -> derived-index rebuild -> runtime bootstrap -> user hook | Unsupported stable bytes/schema fail closed; supported v0.91.6 state upgraded in PocketIC with preserved state | No legacy decoder or dual write found. Stable ABI guard and compatibility accounting are strongest |

## Findings

No P0 finding was proven.

### P1 — Role dependency and feature authority is narrower than the Canic-owned runtime closure

1. **Current behaviour.** The validator requires exactly one direct normal
   dependency resolving to package canic and rejects another subtree that
   reaches that same package. It does not protect canic-core,
   canic-control-plane, or canic-macros as framework-owned packages. The
   delegation_root_stub role consequently has both the canonical canic edge
   and a direct normal canic-control-plane edge. In addition, the workspace
   canic dependency leaves defaults enabled, so 23 of 26 metadata-declared
   roles acquire metrics without spelling it in their own declaration.
2. **Evidence.** Cargo.toml:44 enables resolver 2, while Cargo.toml:63 inherits
   canic without default-features = false. The facade default is metrics at
   crates/canic/Cargo.toml:19-21. The validator accepts aliases at
   crates/canic-host/src/role_contract/package/mod.rs:318-324, reads defaults
   and direct features at :371-400, selects only normal package canic at
   :463-491, and searches sibling paths only for that selected package at
   :513-552. Its tests explicitly accept framework as an alias at
   crates/canic-host/src/role_contract/package/tests.rs:5-22.
   canisters/test/delegation_root_stub/Cargo.toml:14-25 declares both normal
   edges; its source imports the direct package at
   canisters/test/delegation_root_stub/src/lib.rs:12-18 and :64-115 even though
   the same template API/DTO surface is re-exported through
   crates/canic/src/api/mod.rs:81-89, crates/canic/src/dto/mod.rs:3-6, and
   crates/canic/src/ids/mod.rs:6-10.
   Cargo tree confirms canic-core is reached through both canic and the direct
   control-plane subtree.
3. **Historical origin.** The current validator was a useful first boundary
   for the direct canic package. The proposed 0.97 design now records the
   stronger role-owned closure and frozen-evidence contract at
   docs/design/0.97-role-owned-runtime-dependencies-and-cdk-surface/0.97-design.md:20-46,
   :73-110, and :350-427.
4. **Why this is sediment.** A final role is documented as the sole framework
   capability owner, yet the current checker permits a sibling framework crate
   to add the same core/runtime capabilities. The implicit default also means
   the role line is not the complete feature declaration. This is executable
   architectural drift, not dependency-style preference.
5. **Canonical surviving owner/path.** One direct normal dependency named
   canic in each role package, with all Canic capability features written
   there; a separate direct build edge may remain for build.rs.
6. **Smallest hard-cut remediation.** Implement the bounded 0.97 graph evidence
   and protected-package catalog; require resolver 2, exact dependency key
   canic, workspace defaults disabled/no workspace features, exact target and
   role feature selection, and reject every protected package reachable
   outside the canonical canic subtree.
7. **Deletion list.** The direct delegation_root_stub control-plane dependency
   and imports; alias-acceptance test/logic; workspace-inherited Canic defaults
   and any role manifests relying on them; independently chosen metadata
   invocation paths once the canonical evidence object exists.
8. **Contract/migration implications.** Cargo dependency keys and features are
   breaking source/build contracts. Toko and other downstream role repositories
   must split shared Canic coupling before upgrading. No Candid or stable-state
   change is implied.
9. **Tests required.** Real metadata fixtures for a direct protected sibling,
   shared-crate transitive Canic, renamed canonical edge, defaults/features
   inherited from workspace, inactive target-specific edge, permitted build
   edge, locked wasm32 graph, and every workspace role under its exact build
   features.
10. **Confidence.** High.

### P1 — Public canic::cdk is an implementation facade retained for macro plumbing

1. **Current behaviour.** canic publicly re-exports canic_core::cdk, which in
   turn publicly re-exports candid, broad ic-cdk attributes/API/call/futures,
   stable-structure types, serialization, specs, types, and utilities.
   Declarative and procedural macros also expand through this path.
2. **Evidence.** The facade advertises cdk for lower-level use at
   crates/canic/src/lib.rs:9-13 and exports it at :54-58. The broad surface is
   crates/canic-core/src/cdk/mod.rs:1-17. Storable macros refer to
   $crate::cdk at :25-54 and :64-89. Procedural endpoint attributes hard-code
   ::canic::cdk at crates/canic-macros/src/endpoint/expand/mod.rs:285-301 and
   caller lookup at crates/canic-macros/src/endpoint/expand/access.rs:54-95.
   Lifecycle macros likewise use ::canic::cdk at
   crates/canic/src/macros/start.rs:22-23, :51-52, and :99-100.
3. **Historical origin.** Canic previously concentrated generic CDK
   conveniences in its facade. Later cuts already introduced
   canic::__internal specifically for macro expansion
   (crates/canic/src/lib.rs:26-38), leaving two plumbing models.
4. **Why this is sediment.** A major upstream dependency is a public framework
   commitment without a Canic semantic boundary. Macro reachability is a real
   requirement, but it does not require a documented human facade. The
   hard-coded source name also contradicts the validator's accepted rename.
5. **Canonical surviving owner/path.** Role code imports candid, ic-cdk, and
   ic-stable-structures directly for generic operations; Canic semantic APIs
   remain in canic. Macro-only references use one doc-hidden
   canic::__internal path.
6. **Smallest hard-cut remediation.** Move only the exact macro-required
   upstream items into doc-hidden plumbing, change proc/declarative expansion
   to it, migrate maintained human call sites to direct dependencies or Canic
   semantic APIs, then delete canic::cdk in one cut.
7. **Deletion list.** crates/canic-core/src/cdk as a public facade after moving
   owned serialization/helpers to their real modules; canic pub use; public
   facade docs; macro references to canic::cdk; cargo-machete ignores that
   exist only because of the facade.
8. **Contract/migration implications.** Breaking Rust source and generated-code
   path change; downstream manifests may add candid/ic-cdk/
   ic-stable-structures directly. No stable/Candid change. Macro output must be
   changed atomically with the facade cut.
9. **Tests required.** Compile fixtures for query/update/start/storable macros
   with no canic::cdk path, source inventory proving the public facade absent,
   maintained roles with explicit upstream dependencies, and rejection of a
   renamed canic dependency if that is the selected hard-cut contract.
10. **Confidence.** High.

### P1 — Randomness configuration and raw_rand expose a capability that no runtime executes

1. **Current behaviour.** Per-role randomness is accepted, defaulted enabled,
   validated, rendered into compiled config, and displayed by host role
   details. Documentation promises initial PRNG seeding and periodic reseeding
   from IC raw_rand or time. Runtime startup never reads the compiled
   randomness field. A complete raw_rand infra/ops/metric path has no caller.
2. **Evidence.** The promise is CONFIG.md:199-202 and :248-261. Schema comments
   claim runtime consumption at
   crates/canic-core/src/config/schema/subnet/mod.rs:470-508. Validation is
   crates/canic-core/src/config/validation/subnet.rs:26-33; generation is
   crates/canic-core/src/bootstrap/render.rs:448-477 and :654-678; host
   projection is
   crates/canic-host/src/release_set/config/projection/details.rs:35-62.
   Runtime start owners at crates/canic-core/src/workflow/runtime/mod.rs:37-64
   start logs, cycle top-up, intent cleanup, pool reset, and auth renewal, but
   not randomness. Repository-wide reference analysis finds randomness only in
   schema/validation/render/projection/tests and no ConfigOps/runtime read.
   The unused adapter is
   crates/canic-core/src/infra/ic/mgmt/randomness.rs:1-30 and
   crates/canic-core/src/ops/ic/mgmt/lifecycle.rs:241-251, with dead metric
   variants at crates/canic-core/src/domain/metrics.rs:383-409 and
   crates/canic-core/src/ids/metrics.rs:43-65.
3. **Historical origin.** Commit 52b2413c6, titled “new randomness!”, introduced
   the schema, raw_rand, and PRNG plan. Later policy/runtime changes removed the
   consuming system but retained its contract and adapters.
4. **Why this is sediment.** This is not future extensibility: users can
   configure and observe a claimed security-relevant capability whose values
   do nothing. Tests validate the abandoned representation, not behaviour.
5. **Canonical surviving owner/path.** None is needed unless a current
   consumer is first proven. Applications needing generic entropy should own
   their own explicit adapter; a future Canic capability would require a new,
   tested semantic design.
6. **Smallest hard-cut remediation.** Delete the accepted config subtree,
   compiled representation, validation, generation, role-detail projection,
   raw_rand infra/ops/errors/metrics, docs, and tests together.
7. **Deletion list.** RandomnessConfig, RandomnessSource, their CanisterConfig
   field/defaults/renderers/validators/projections; raw_rand infra and ops
   functions; RawRand error and metric variants; randomness-only tests/docs.
8. **Contract/migration implications.** Breaking canic.toml and generated
   ConfigModel/Rust contracts; role-detail text/JSON and metric label inventory
   may shrink. No current workspace config sets the field. No stable state or
   Candid change was found.
9. **Tests required.** Config unknown-field rejection, generated-model and
   role-detail inventory updates, metric inventory update, and repository
   source/dependency proof that no raw_rand adapter remains. Do not add an
   anti-resurrection compatibility test.
10. **Confidence.** High.

### P1 — Recovery-required replay state is safe but lacks the operator evidence path promised by the runbook

1. **Current behaviour.** Replay receipts mark an effect in flight before an
   external mutation and retain ExternalEffectStatusUnknown,
   StateProjectionFailed, and other recovery-required states. Exact retries
   automatically finish only staged response/cost settlement failures.
   Other reasons fail closed. Operations guidance tells an operator to inspect
   the exact replay receipt and external evidence, but no controller-facing
   exact OperationId receipt lookup or CLI path was found.
2. **Evidence.** The durable receipt/status/reason/effect representation is
   crates/canic-core/src/model/replay/mod.rs:212-295. Automatic retry is limited
   to CostSettlementFailed and ResponseCommitFailed at
   crates/canic-core/src/workflow/rpc/request/handler/replay.rs:111-168.
   Root provisioning marks before creation and preserves unknown effect at
   crates/canic-core/src/workflow/rpc/request/handler/execute.rs:288-342.
   The safety rule is docs/operations/recovery-retry-runbooks.md:43-67; the ICP
   refill procedure requires receipt inspection at :188-200 and the upgrade
   procedure at :230-242. The 0.93.12 release explicitly leaves unknown/state
   projection manual at docs/changelog/0.93.md:1864-1871. Repository-wide API,
   endpoint, and CLI searches found no exact receipt observation command.
3. **Historical origin.** The original replay design deliberately deferred
   orphaned-created-canister discovery while guaranteeing no repeated creation
   (docs/design/archive/0.61-replay-protection/0.61-design.md:836-845). Later
   lines added durable cost/response recovery but retained that bounded
   deferral.
4. **Why this is sediment/gap.** The fail-closed state is current and correct;
   it is not sediment to delete. The sediment is the operational promise that
   an operator can inspect/reconcile an authority that is only internally
   persisted. This leaves durable evidence without a supported consumption
   path and can permanently strand capacity/state.
5. **Canonical surviving owner/path.** ReplayReceipt in canic-core remains the
   sole durable authority. External systems remain authoritative for their own
   effect evidence.
6. **Smallest hard-cut remediation.** Add one controller-only typed exact
   receipt projection keyed by OperationId and expose it through the existing
   diagnostic/CLI ownership chain. Add operation-specific reconciliation only
   where authoritative external evidence proves applied or no-effect. Keep
   create-canister unknown outcomes manual if identity discovery is not exact.
7. **Deletion list.** No receipt/reset deletion. Remove or rewrite runbook
   instructions that name evidence users cannot obtain; delete any generic
   duplicate status projection if the exact projection replaces it.
8. **Contract/migration implications.** Likely additive controller-only Candid,
   CLI/JSON, error-code, and possibly metrics surface. Reuse the current stable
   receipt; avoid a new journal or state field unless a specific reconciliation
   proof requires it.
9. **Tests required.** Exact authorized lookup, unauthorized rejection,
   actor/payload/effect projection, upgrade persistence, every recovery reason,
   operation-specific applied/no-effect/ambiguous reconciliation, and proof
   that ambiguous create/value-transfer outcomes never reset or repeat.
10. **Confidence.** High for the observation gap; medium for which effect
    families can safely support automatic reconciliation.

### P2 — The old LocalIntent race fixture teaches unsafe external-effect semantics

1. **Current behaviour.** intent_authority::buy reserves a LocalIntent, awaits
   an external call, commits on response success, and rolls back on any call
   error. pic_intent_race still builds intent_client and intent_external and
   asserts the capacity race before running the newer receipt-backed
   conformance in the same test.
2. **Evidence.** canisters/test/intent_authority/src/lib.rs:111-145 contains
   the external call and unconditional error rollback. LocalIntent is described
   as locally decidable at
   crates/canic-core/src/workflow/runtime/intent.rs:33-38. The current adapter
   contract states that transport/decode/callback/settlement failure does not
   prove no effect and must remain pending at
   docs/operations/receipt-backed-intent-adapter.md:15-36 and :64-66.
   crates/canic-tests/tests/pic_intent_race.rs:96-183 installs three canisters
   and exercises buy; receipt-backed conformance begins at :183-214.
3. **Historical origin.** The race fixture predates the receipt-backed intent
   API. The maintained conformance was appended rather than replacing the old
   half.
4. **Why this is sediment.** The test is executable documentation for a call
   shape that the current architecture says is unsafe. Passing coverage does
   not justify retaining a contradictory model.
5. **Canonical surviving owner/path.** Keep LocalIntent for truly local,
   synchronously decidable reservations. Keep receipt-backed intent plus a
   domain receipt adapter for external effects.
6. **Smallest hard-cut remediation.** Delete buy/call_buy/perform and the two
   auxiliary canisters; make pic_intent_race a focused receipt-backed
   conformance test using only intent_authority.
7. **Deletion list.** intent_client package, intent_external package,
   intent_authority EXTERNAL/init dependency and buy endpoint, first half of
   pic_intent_race, corresponding build roster/lock members.
8. **Contract/migration implications.** Test-only Candid and package/artifact
   names disappear. No product Candid, stable state, or downstream API change.
9. **Tests required.** Preserve receipt Created/pending/binding/capacity,
   committed/rolled-back CAS, upgrade persistence, and transport-uncertainty
   behaviour. Existing legitimate LocalIntent tests must remain.
10. **Confidence.** High.

### P2 — project-protocol-stub survived the architecture it represented

1. **Current behaviour.** project-protocol-stub is a workspace library whose
   entire public surface is two CanisterRole constants. No package, source,
   current design, operation document, or test consumes it.
2. **Evidence.** Its manifest is
   canisters/test/project_protocol_stub/Cargo.toml:1-12 and its complete source
   is canisters/test/project_protocol_stub/src/lib.rs:1-6. Reverse dependency
   analysis reports only the package itself. Current project hub/instance
   packages do not depend on it.
3. **Historical origin.** It was introduced in 0.40.9 as the shared owner of a
   protected endpoint descriptor and generated client
   (docs/changelog/0.40.md:200-216). Commit 6d8310ef0 / 0.65.17 deleted
   canic_protected_endpoint, descriptors, generated protected-internal calls,
   and their tests (docs/changelog/0.65.md:583-615), but left this emptied
   package.
4. **Why this is sediment.** The package exists solely because a hard-cut
   architecture once needed it. Its name and module comment still describe
   shared protocol ownership that no current protocol uses.
5. **Canonical surviving owner/path.** Role constants are owned at the final
   role/root fixture where they are used; current DTO/protocol contracts live
   in canic/canic-core owners.
6. **Smallest hard-cut remediation.** Remove the member and directory.
7. **Deletion list.** The package manifest/source, workspace member entry,
   Cargo.lock package row, and any package roster mention.
8. **Contract/migration implications.** Test-only Cargo package removal. It is
   unpublished and has no consumer; no Candid/stable/CLI/JSON change.
9. **Tests required.** Cargo metadata/workspace manifest validation and the
   existing project hub/instance integration test. No anti-resurrection test.
10. **Confidence.** High.

### Observation — icp-refill has no current workspace role consumer

The canic icp-refill feature and facade macro/API are current and tested at
macro level, while no current role manifest enables the feature or emits the
endpoint. This is insufficient evidence for deletion: the archived 0.58 design
explicitly defines an opt-in downstream endpoint/funding integration and notes
downstream adoption (docs/design/archive/0.58-convert-icp/0.58-design.md:128-218
and :254-280). The missing proof is one production-realistic role/PocketIC test
that enables the feature, exports the guarded endpoint, and exercises auth,
value-transfer evidence, retry, and recovery. Confidence: medium.

## Authority conflicts

| Concept | Intended owner | Competing owner/path | Production reachability | Required decision |
| --- | --- | --- | --- | --- |
| Canic framework package closure | Final role's direct canic edge | delegation_root_stub direct canic-control-plane edge; validator protects only canic | Reachable in a built root fixture | Implement protected catalog and remove direct edge |
| Canic feature selection | Role dependency declaration | Workspace inherited default metrics | Reachable in 23 metadata-declared roles | Disable defaults at workspace declaration and list every role feature |
| Dependency source name | Canonical key canic used by proc macro output | Validator explicitly accepts rename | Rename fixture passes validator but generated endpoint cannot compile | Hard-cut rename support or implement alias-aware proc expansion; 0.97 should choose exact key |
| CDK/public upstream API | Direct upstream dependencies and Canic semantic APIs | canic::cdk broad facade | Reachable throughout roles/macros | Move macro plumbing hidden and delete human facade |
| Randomness capability | No current owner/consumer | Config/build/host representations plus raw_rand ops | Config path reachable; operation path uncalled | Delete whole claimed capability |
| External intent uncertainty | Receipt-backed adapter and domain receipt | LocalIntent buy fixture rolls back transport failure | Test-only but production-realistic example | Delete old fixture path |
| Project protocol constants | Using role fixture/current protocol owner | project-protocol-stub | Unreachable | Delete package |
| Recovery-required operation status | ReplayReceipt | Runbook/log-only indirect observation | Persisted state reachable; no exact supported operator query | Add narrow typed observation and proven reconciliation |

## Deletion candidates

### Proven dead

- project-protocol-stub: one package, one six-line module, two unreferenced
  constants, its workspace/lock roster entries.
- raw_rand runtime branch: one infra module/function, one ops function, its
  invalid-length error variant, and RawRand operation/system metric variants;
  no supported caller was found.

### Compatibility-only

- No active product compatibility shim was proven.
- The false actor-extension marker retained in replay payload hashing is stable
  identity compatibility, not a deletion candidate.

### Test-anchored sediment

- intent_client and intent_external packages.
- intent_authority's EXTERNAL state/init argument and buy endpoint.
- The pre-conformance capacity-race half of pic_intent_race.
- Randomness schema/render/projection tests that prove only the abandoned
  representation.

### Superseded but still production-reachable

- canic::cdk and the broad canic-core cdk facade.
- RandomnessConfig/RandomnessSource in parsed and compiled config.
- delegation_root_stub's direct control-plane edge/import path.
- Alias support in role validation, because proc expansion assumes canic.

### Uncertain and requiring further evidence

- icp-refill feature/API: no in-workspace role consumer, but a current explicit
  downstream capability contract exists.
- Automatic reconciliation for ExternalEffectStatusUnknown: safe only for
  operation families with authoritative, identity-bound external evidence.
- Generic IC/CDK helpers currently under cdk may have valid direct consumers;
  classify each as move-to-owner or direct-upstream migration before deleting
  the module.

## Contract-risk register

| Proposed cut | Stable state | Candid | CLI/JSON/artifacts | Cargo/Rust/config/metrics risk |
| --- | --- | --- | --- | --- |
| 0.97 role graph enforcement | None | None | Build/medic diagnostics and structured finding evidence may change | Breaking dependency key/default/features; downstream shared crates must remove Canic |
| canic::cdk hard cut | None | Generated Candid should remain semantically identical | Generated source paths change | Breaking Rust imports/macro expansion; downstream adds direct upstream deps |
| Randomness deletion | None found | None found | Host role-detail output loses randomness text | Breaking TOML keys and compiled ConfigModel; RawRand metric label removed |
| Old intent fixture deletion | Test stable state only | Test-only buy/call_buy/perform endpoints disappear | Test Wasm/package roster shrinks | No product contract |
| project-protocol-stub deletion | None | None | Workspace/lock roster only | Unpublished test package removed |
| Replay observation/reconciliation | Reuse existing receipt if possible; new fields would need explicit schema review | Additive controller-only endpoint likely | Additive CLI/JSON/error/metrics surface likely | Rust DTO/API addition; no reset/migration implied |
| icp-refill proof | Existing ICP/refill/replay durable contracts are affected only if a defect is found | Existing optional endpoint | CLI convert output and pending-send evidence | Feature contract remains opt-in |

Current durable-contract evidence is strong: the v0.91.6 compatibility
accounting reports byte-identical root/Wasm-store interfaces and identical
stable trees, plus a successful old-state PocketIC upgrade
(docs/audits/reports/2026-07/2026-07-16/0.92-v0916-compatibility-accounting.md:5-18,
:58-72, and :82-117). The 0.94 recovery closeout reports 106/106 backup/restore
cases passing. Remediation must not generalise those hard-cut proofs into an
assumption that any future stable or wire change is safe.

## Test architecture audit

The suite mostly proves current authorities rather than mocking them away.
Stable-storage unit tests deliberately construct corrupt/impossible records to
prove fail-closed decoding and index validation; those are valid negative
proofs, not evidence that production can construct the states. PocketIC tests
build real role Wasm through canic-testing-internal, while backup/restore pairs
injected executors with real process-death, file-durability, lock, and selected
ICP integration evidence.

| Test pattern investigated | Assessment |
| --- | --- |
| Direct control-plane imports in delegation_root_stub | A real validator-blind framework path, not merely test access; finding P1 |
| Raw CDK lifecycle/endpoints in sharding_root_stub and purpose-built probes | Deliberate external/protocol fixtures with production consumers; not a second Canic endpoint authority |
| intent_authority buy plus intent_client/intent_external | Test-anchored obsolete external-effect model; finding P2 |
| project-protocol-stub | No test or production consumer remains; proven dead |
| Randomness schema/render tests | They anchor a representation without runtime behaviour; delete with the contract |
| Cargo role graph tests | Correctly reject a second path to canic, but encode obsolete rename support and omit protected sibling packages |
| Stable corruption/index tests | Necessary fail-closed proofs; retain current-schema cases |
| Backup/restore injected executor tests | Necessary effect boundary backed by real crash/filesystem/ICP cases; not sediment |
| Replay RecoveryRequired tests | Strong safety proof against duplicate effects; missing operator observation and effect-specific liveness proof |
| ICP refill macro/core/CLI tests | Unit/component coverage exists, but no configured role/PocketIC journey proves the public opt-in contract |
| Timer and lifecycle inventory guards | Strong executable architecture guards over production source |
| Anti-resurrection tests for old hard cuts | No material active set found that should be retained; do not add new ones during deletion |

For each major authority, the strongest current test and missing proof are
listed below. Test names and comments referring to removed descriptor/client
architecture are confined to history except for the dead project package.
No dead Cargo feature combination was proven solely from CI; icp-refill remains
unproven rather than classified as obsolete.

## Missing architectural proofs

| Authority | Strongest existing proof | Most important missing proof |
| --- | --- | --- |
| Role dependency/features | Unit metadata graph cases at role_contract/package/tests.rs:63-159 | One canonical wasm32 locked graph evidence object and protected Canic-owned closure over every real role/feature set |
| Endpoint/auth ordering | Proc-macro expansion and delegated-auth PocketIC tests | Compile proof after cdk facade removal and exact canonical dependency key |
| Lifecycle | lifecycle_boundary_guard plus v0.91.6 upgrade | Current active 0.96 receipt-reclamation state once that separate work is complete |
| Stable memory allocation | stable_memory_abi_guard and allocation catalog | Downstream generic adapter rule that proves no shared crate selects MemoryIds independently |
| Topology/registry | Root placement/reconciliation PocketIC suites | Exact operator evidence for orphaned create-canister outcomes |
| Root replay | Durable receipt and cost/response recovery tests | Controller-visible exact receipt lookup and evidence-driven unknown-effect reconciliation |
| Application receipt intent | pic_intent_race receipt-backed conformance | A domain adapter proving external transport uncertainty remains pending and later reconciles |
| Backup/restore | 106-case crash matrix and real process-death/ICP cases | No material missing proof for maintained entrypoints found |
| Timers | timer_inventory_guard exact source inventory | None material; continue updating the closed inventory when a current timer is added |
| Randomness | Schema/render tests | No behavioural proof exists; delete rather than add a framework solely to justify it |
| ICP refill | Macro unit test, core workflow tests, CLI decoding tests | One role/PocketIC end-to-end guarded endpoint with retry/recovery |
| Public surface | Prior 0.92 module-surface audits | Source/compile inventory for the exact surviving hidden macro plumbing after 0.97 |
| Error projection | Typed owner errors and boundary tests | Exact recovery receipt projection without string parsing |

Documentation-only/convention-only invariants that need executable enforcement
are:

- all Canic-owned runtime packages enter only through the canonical role edge;
- workspace Canic declarations select no features/defaults;
- the canonical dependency key is canic if proc macros continue to emit that
  name;
- metadata target/package/features/lock context is identical for checker and
  build;
- a shared stable-structures adapter may use types/algorithms but may not choose
  MemoryIds or open persistent memory independently; and
- every runbook diagnostic named as an operator action is reachable through a
  maintained typed command or endpoint.

## Recommended remediation line

The findings should not become one general cleanup release. Use the following
independently bounded lines.

### Line A — 0.97 role-owned runtime dependencies and hidden macro plumbing

#### Slice A1 — Freeze role Cargo evidence

- **Invariant established:** one selected role, wasm32-unknown-unknown, resolver
  2, exact default/feature arguments, normal edges only, locked resolution.
- **Canonical authority retained:** canic-host produces one structured
  RoleCargoGraphEvidence consumed by build, medic, deploy, and release.
- **Obsolete authority deleted:** caller-specific metadata invocation choices.
- **Affected crates:** canic-host, canic-core role contract, canic build support.
- **Contract impact:** diagnostic evidence schema; no runtime/stable/Candid.
- **Validation:** target/default/feature/build-edge/inactive-target fixtures and
  real role metadata.
- **Exclusions:** no Toko or sibling-repository edits.

#### Slice A2 — Enforce complete Canic-owned closure

- **Invariant established:** exactly one direct normal canic edge; no protected
  Canic-owned package outside its subtree; separately permitted build edge.
- **Canonical authority retained:** role manifest's exact canic dependency.
- **Obsolete authority deleted:** delegation_root_stub direct control-plane
  path, rename support, workspace feature/default selection.
- **Affected crates:** root Cargo.toml, role manifests/fixtures, canic-host.
- **Contract impact:** breaking Cargo manifest contract only.
- **Validation:** protected catalog fixtures, downstream packaged fixture, all
  workspace roles under exact build features.
- **Exclusions:** ordinary shared upstream packages such as ic-cdk are not
  protected solely because Canic also uses them.

#### Slice A3 — Hard-cut canic::cdk

- **Invariant established:** generic upstream APIs are direct dependencies;
  macros use one doc-hidden path.
- **Canonical authority retained:** canic semantic APIs and
  canic::__internal macro plumbing.
- **Obsolete authority deleted:** public canic::cdk/canic-core cdk facade and
  macro references to it.
- **Affected crates:** canic, canic-core, canic-macros, maintained role
  packages/scaffolds/docs.
- **Contract impact:** breaking Rust/Cargo source contract; generated Candid
  must remain unchanged.
- **Validation:** macro compile fixtures, scaffold builds, source/public-surface
  inventory, Candid comparison.
- **Exclusions:** no compatibility alias, deprecation, forwarding facade, or
  legacy import.

### Line B — Delete the unimplemented randomness contract

#### Slice B1 — Contract and implementation deletion

- **Invariant established:** every accepted config capability has a current
  runtime consumer.
- **Canonical authority retained:** ordinary application-owned entropy or a
  future separately designed Canic capability.
- **Obsolete authority deleted:** config schema/default/validation/compiled
  projection, host display, raw_rand adapter/errors/metrics, tests/docs.
- **Affected crates:** canic-core, canic-host, config documentation.
- **Contract impact:** hard-cut TOML/compiled Rust/role-detail/metric inventory;
  no stable/Candid.
- **Validation:** config unknown-field, generation/public inventory, focused
  host config tests, source proof of removal.
- **Exclusions:** do not implement a PRNG/reseed framework merely to retain the
  old fields.

### Line C — Delete obsolete test architecture

#### Slice C1 — Remove project-protocol-stub

- **Invariant established:** every workspace package has a current consumer or
  necessary boundary.
- **Canonical authority retained:** current role/protocol owners.
- **Obsolete authority deleted:** the entire project-protocol-stub package.
- **Affected crates:** workspace manifest/lock and test package roster.
- **Contract impact:** unpublished test package only.
- **Validation:** metadata/workspace manifest and project hub/instance tests.
- **Exclusions:** no replacement common crate.

#### Slice C2 — Remove the unsafe LocalIntent external fixture

- **Invariant established:** LocalIntent examples remain locally decidable;
  external uncertainty uses receipts.
- **Canonical authority retained:** receipt-backed conformance in
  intent_authority/pic_intent_race.
- **Obsolete authority deleted:** intent_client, intent_external, buy path and
  old test half.
- **Affected crates:** three test canisters, canic-tests, workspace/lock/test
  builder roster.
- **Contract impact:** test-only Candid/artifacts.
- **Validation:** focused receipt-backed PocketIC conformance and LocalIntent
  unit/runtime probe tests.
- **Exclusions:** do not delete LocalIntent itself.

### Line D — Make recovery-required evidence consumable

#### Slice D1 — Exact read-only receipt evidence

- **Invariant established:** every runbook instruction to inspect a replay
  receipt has a typed controller-only path.
- **Canonical authority retained:** existing ReplayReceipt.
- **Obsolete authority deleted:** log/string-only operator guidance where it is
  the sole path.
- **Affected crates:** canic-core facade/DTO/endpoint, canic-host, canic-cli,
  operations docs.
- **Contract impact:** additive Candid/CLI/JSON/error surface; avoid stable
  changes.
- **Validation:** auth, exact ID/binding/status/effect projection, upgrade, and
  structured CLI tests.
- **Exclusions:** no mutation/reset/retry command.

#### Slice D2 — Operation-specific reconciliation only where proof exists

- **Invariant established:** an unknown external outcome becomes terminal only
  from identity-bound authoritative evidence.
- **Canonical authority retained:** ReplayReceipt plus the external system's
  own receipt/status authority.
- **Obsolete authority deleted:** only permanently manual branches for which a
  complete deterministic proof is implemented.
- **Affected crates:** one operation family at a time; core/control-plane and
  focused tests.
- **Contract impact:** explicitly reviewed per operation; stable fields only if
  unavoidable.
- **Validation:** interruption before effect, after effect/before local receipt,
  applied, durable no-effect, contradictory, absent, and ambiguous outcomes.
- **Exclusions:** no generic recovery framework, no blind create/value-transfer
  replay, no destructive state edit, and no assumption that absence proves
  no-effect.

## Explicit non-findings

- **Stable-memory allocation is not duplicated.** The canonical ranges and
  allocation definitions are in
  crates/canic-core/src/role_contract/allocation.rs:1-84 and are guarded by the
  stable-memory ABI test. Runtime storage imports these IDs; no shared adapter
  opening its own Canic range was found.
- **Backup and restore do not retain parallel runners.** canic-backup creates or
  reads one execution journal before work
  (crates/canic-backup/src/runner/mod.rs:33-84), reconciles pending operations
  at :270-310, and durably publishes JSON through
  crates/canic-backup/src/persistence/json.rs:21-119. Restore claims operations
  before command execution and reconciles exact lifecycle/upload evidence at
  crates/canic-backup/src/restore/runner/execute.rs:142-233 and :459-577.
- **Backup/restore unknown outcomes are not reset-and-retry sediment.** The
  command-lifetime lock and exact status/inventory evidence deliberately halt
  ambiguous mutations; the 0.94 closeout proves the finite matrix.
- **Timer authority is singular.** TimerWorkflow owns identities/arbitration
  (crates/canic-core/src/workflow/runtime/timer/mod.rs:1-73), TimerOps is the
  sole raw IC adapter, and
  crates/canic-core/tests/timer_inventory_guard.rs:9-85 checks the exact
  repository inventory.
- **Lifecycle sequencing is not duplicated.** Generated start hooks restore
  synchronously and defer bootstrap/user work
  (crates/canic/src/macros/start.rs:5-77); lifecycle module contracts explicitly
  forbid orchestration at
  crates/canic-core/src/lifecycle/init/mod.rs:1-20 and
  crates/canic-core/src/lifecycle/upgrade/mod.rs:1-15.
- **Raw management calls are not bypassing ops in product runtime.**
  Management Call construction is confined to infra; workflows reach it
  through ops. Direct raw endpoints/calls found in test canisters are deliberate
  protocol fixtures.
- **The internal canic -> core and canic -> control-plane -> core paths are not
  the forbidden role sibling path.** They are one canonical framework subtree;
  Cargo feature union outside that subtree is the concern.
- **Host TOML Value projections are not a second semantic config parser.** They
  preserve/edit source documents; canic-core's typed parse/Validate model is
  the semantic authority.
- **Invalid delegated-session fallback is not an auth bypass.** Invalid session
  state is cleared and identity falls back to the raw transport caller, not a
  privileged subject.
- **Retired control-plane bindings and stable payload-hash markers are not
  proven compatibility sediment.** They participate in current lifecycle/GC
  or stable identity validation.
- **Executor traits in build/backup/restore are justified boundaries.** They
  have real host implementations and allow deterministic effect/interruption
  tests; they are not one-implementation domain abstractions.
- **sharding_root_stub is not a duplicate Canic lifecycle role.** It is an
  intentional minimal external root peer in a sharding bootstrap test. Its use
  of facade DTO/CDK imports should migrate in 0.97, but the fixture itself has a
  current consumer.
- **icp-refill is not proven dead.** Lack of a workspace role is a missing
  integration proof, not enough to override the explicit downstream contract.
- **No current shared/domain workspace library with a legitimate business
  responsibility was found depending on Canic.** The only suspicious shared
  package is project-protocol-stub, and it is dead rather than a current domain
  layer.

## Final recommendation

**Multiple independently bounded lines.**

Proceed with the already proposed 0.97 role-owned dependency/CDK hard cut after
incorporating its Cargo-contract feedback. Independently delete the randomness
contract and obsolete test packages/paths. Treat exact replay observation and
effect-specific reconciliation as a separate safety-reviewed operational line.
Do not couple recovery work to dependency cleanup, and do not retain dead
surface as a compatibility concession before 1.0.

The repository is coherent enough that no ownership redesign or general
rewrite is needed.

## Validation

### Completed against the audited repository/baseline

- cargo metadata --locked --no-deps --format-version 1: pass; 40 workspace
  packages classified.
- cargo metadata --locked --format-version 1: pass; complete locked graph
  captured. Reverse-dependency analysis included normal/build/dev kind
  separation and role feature/default extraction.
- cargo tree --locked -p delegation_root_stub --target
  wasm32-unknown-unknown -e normal -i canic-core: confirmed both the canonical
  canic subtree and the direct control-plane subtree.
- Repository-wide structural passes: manifests/dependencies/features, pub use
  chains, doc-hidden plumbing, raw endpoints/management/timers, shared/common/
  compat/legacy/adapter/bridge names, package reverse references, generated
  output assumptions, optional features, and fixture architecture.
- Repository-wide behavioural passes: validation/reparse boundaries,
  intent/replay/receipt state machines, pending/retry/reset/reconciliation,
  stable fields/indexes, lifecycle hooks, raw effects, typed/string errors,
  tests that bypass production, and docs/runbook claims.
- bash scripts/ci/run-layering-guards.sh: pass.
- bash scripts/ci/check-recovery-runbooks.sh: pass.
- Selective history: randomness introduction, protected-descriptor
  introduction/removal, replay recovery evolution, and recent hard-cut/audit
  lines.

### Inconclusive because user-owned edits appeared after baseline freeze

- cargo test --locked -p canic-host role_contract::package --lib --
  --nocapture.
- cargo test --locked -p canic-core --test stable_memory_abi_guard.

Both compilation attempts encountered an out-of-scope, uncommitted
receipt-reclamation edit in
crates/canic-core/src/ops/storage/intent/mod.rs. Git blame/diff confirmed the
failing lines were not in ee58f6ef3. No files were reverted or modified.

### Deliberately skipped

- Full workspace tests, workspace Clippy, release matrices, deployment,
  packaging, publishing, and broad PocketIC suites, per repository policy.
- Re-running the 106-case backup/restore matrix and v0.91.6 upgrade proof; their
  current immutable reports were inspected instead.
- Live network/deployment mutation.

The audit itself changed only this report. The concurrent user-owned changes
listed in the scope note remain untouched.
