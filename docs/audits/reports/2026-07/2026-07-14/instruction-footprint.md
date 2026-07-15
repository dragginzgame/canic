# Instruction Footprint Audit - 2026-07-14

## Verdict

- Run result: `blocked`.
- Result validity: `invalid` under the post-freeze method-defect protocol.
- Comparability: `non-comparable: first frozen CANIC-INSTRUCTION-001/v1
  attempt produced no instruction rows`.
- Authoritative risk score: `blocked`; minimum observed evidence risk 4 / 10.

The canonical PocketIC runner cannot measure the published `v0.92.0`
snapshot. After the pinned PocketIC 14.0.0 server started, the first root-probe
scenario called the test harness's direct Cargo Wasm builder. Canic's current
build boundary rejected that path at `crates/canic/src/build_support/config.rs:25`:
authoritative Wasm must be built through `canic build <fleet> <role>`. No perf
row or checkpoint delta was produced, so this result says nothing about a
runtime instruction regression.

The executable method also fails its frozen identity and coverage contracts.
The frozen relative-path composite is `c79f7027...`, while the runner hashes
absolute filenames and emitted `a5fa45ef...`; its exact documented checkpoint
scan finds zero namespaced calls even though 57 product `perf!` calls exist;
and the retained 11-scenario manifest omits required root-proof,
delegated-token, sharding, and bootstrap flows. These are one canonical
P1 audit-method finding, `CANIC-092-AUDIT-015`. The frozen method and product
were not edited.

## Report Preamble

- Scope: frozen PocketIC instruction-audit roster and current product
  checkpoint coverage.
- Definition path:
  `docs/audits/recurring/system/instruction-footprint.md`.
- Compared baseline report path: `N/A`; first run of the UTC day.
- Code snapshot: `v0.92.0` /
  `91736337fc1cfeb891f17d7d62affb5e671348e2`; source tree
  `fd31bb8289365a38f2bea7f8ebd6973908ee959f`; product tree
  `c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e`.
- Method: `CANIC-INSTRUCTION-001/v1`, definition fingerprint
  `f90bbd1443ac5acdcc69ad256eaef8877955a9219025f65c6255c6fdd7bf2805`;
  expected executable composite `c79f7027f3629bcbe4dbf4680005d3a9b37104c7ba6d4956a5a3c789c5b5cfab`.
- Runner-emitted composite:
  `a5fa45efe6b9582a53a52fc7ed37500c7ba5d1388f3c5a0b498343f31d7d13fb`.
- Auditor: Codex.
- Run timestamp: `2026-07-14T17:49:17Z` to `2026-07-14T17:50:43Z`.
- Branch: `main`.
- Worktree: dirty with Phase C report/status changes only; product tree
  unchanged.
- Execution environment: isolated Cargo target/TMPDIR plus pinned PocketIC
  14.0.0 in a disposable local process; no authoritative IC environment.
- Target canisters: planned `leaf_probe`, `root_probe`, `scaling_probe`,
  `test`, and `root`.
- Target flows: 11 retained scenario identities covering query probes, one
  update floor, root capability cycles, and three publication stages.
- Host-side commands and timing: explicitly out of scope.

## Run Identity

```text
release_anchor: v0.92.0
source_commit_full: 91736337fc1cfeb891f17d7d62affb5e671348e2
source_tree_hash: fd31bb8289365a38f2bea7f8ebd6973908ee959f
product_tree_hash: c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e
clean_worktree: false; report/status-only Phase C changes, product tree unchanged
cargo_lock_hash: 6cd75f146077bbf3f254fda608f1265531d1065ce0cd9c1bb56d67118f3de5cc
rust_toolchain: rustc 1.97.0; cargo 1.97.0
target_triple: x86_64-unknown-linux-gnu plus wasm32-unknown-unknown fixture attempt
feature_set: canonical instruction-audit test target; offline locked Cargo graph
audit_method_id: CANIC-INSTRUCTION-001
audit_method_version: 1
audit_method_fingerprint: f90bbd1443ac5acdcc69ad256eaef8877955a9219025f65c6255c6fdd7bf2805
expected_executable_composite: c79f7027f3629bcbe4dbf4680005d3a9b37104c7ba6d4956a5a3c789c5b5cfab
runner_executable_composite: a5fa45efe6b9582a53a52fc7ed37500c7ba5d1388f3c5a0b498343f31d7d13fb
fixture_or_seed: retained 11-entry scenario-manifest.json
environment_class: disposable local PocketIC execution trace
started_at: 2026-07-14T17:49:17Z
completed_at: 2026-07-14T17:50:43Z
```

## Execution Attempts

1. The canonical runner first failed closed because isolated `TMPDIR` had no
   cached PocketIC server and network download was disabled.
2. A retry permitting the pinned download failed under the restricted network
   sandbox.
3. The authorized retry downloaded and started pinned PocketIC 14.0.0,
   installed three non-root leaf probes, then failed when
   `install_audit_root_probe` called `ensure_probe_wasm_ready` at
   `crates/canic-testing-internal/src/pic/audit.rs:46-77`.
4. That helper delegates to `build_wasm_canisters`, which invokes the direct
   Cargo Wasm path rejected by the current build contract.

The runner returned exit 101. Its post-run mutation guard completed and
retained only the scenario manifest and evidence manifest.

## Endpoint Matrix

No endpoint instruction count is available. The planned identities remain
useful for method repair and are retained without fabricated totals.

| Canister | Endpoint/flow | Scenario | Origin | Count | Total | Average | Delta |
| --- | --- | --- | --- | ---: | ---: | ---: | --- |
| `leaf_probe` | `audit_time_probe` | minimal valid | query | N/A | N/A | N/A | N/A |
| `leaf_probe` | `audit_env_probe` | minimal valid | query | N/A | N/A | N/A | N/A |
| `leaf_probe` | `audit_log_probe` | empty page | query | N/A | N/A | N/A | N/A |
| `root_probe` | `audit_subnet_registry_probe` | representative valid | query | N/A | N/A | N/A | N/A |
| `root_probe` | `audit_subnet_state_probe` | minimal valid | query | N/A | N/A | N/A | N/A |
| `scaling_probe` | `audit_plan_create_worker_probe` | empty pool | query | N/A | N/A | N/A | N/A |
| `test` | `test` | minimal valid | update | N/A | N/A | N/A | N/A |
| `root` | `canic_response_capability_v1` | fresh cycles request | update | N/A | N/A | N/A | N/A |
| `root` | `canic_template_stage_manifest_admin` | single chunk | update | N/A | N/A | N/A | N/A |
| `root` | `canic_template_prepare_admin` | single chunk | update | N/A | N/A | N/A | N/A |
| `root` | `canic_template_publish_chunk_admin` | single chunk | update | N/A | N/A | N/A | N/A |

All tuples declare a fresh standalone or topology-per-scenario model. No timer
scenario is present, so endpoint/timer mixing did not occur.

## Flow Checkpoints

The method's exact command
`rg -n '^[[:space:]]*perf!\(' crates` returns zero because every maintained
call is namespaced as `crate::perf!` or `canic_core::perf!`. A broader literal
scan finds 57 product callsites across 11 files.

| Flow | Static checkpoint order/groups | Runtime deltas |
| --- | --- | --- |
| root capability/replay | `extract_context`, `map_request`, `preflight`, replay stages, `execute_capability`, `commit_replay` | unavailable |
| scaling | `observe_state`, `plan_spawn`, `create_canister`, `register_worker` | unavailable |
| sharding bootstrap | empty/registry/select/allocate/assign stages | unavailable |
| sharding assignment | load/collect/plan/assigned/allocate/create stages | unavailable |
| template stage/store | validate, project/enforce capacity, upsert, accounting | unavailable |
| release publication | prepare store, push chunk, promote manifest | unavailable |
| root bootstrap | import/create/rebuild/validate/store publication stages | unavailable |

No `Topic::Perf` delta is retained. Static callsites are coverage evidence, not
substitutes for execution measurements.

## Checkpoint Coverage Gaps

| Critical flow | Current coverage | Gap / first candidate boundary |
| --- | --- | --- |
| root capability dispatch | named static checkpoints | execution blocked |
| replay/cached response | named static checkpoints | replayed scenario absent from roster |
| scaling | named static checkpoints plus planned dry-run query | execution blocked |
| sharding | named static checkpoints | no sharding scenario in manifest |
| bootstrap/publication | named static checkpoints and publication scenarios | full bootstrap scenario absent |
| root proof prepare/get/install | no product checkpoints found | prepare, sign/get, install boundaries |
| delegated-token prepare/get/verify | no product checkpoints found | material load, proof verify, projection boundaries |

V1 requires at least one flow from every active class, but the retained
manifest has no root-proof, issuer-local delegated-token, sharding, or full
bootstrap flow. This would keep the run partial even after the build path is
repaired unless the scenario contract is versioned.

## Structural Hotspots

Because no instruction rows exist, these are static attribution candidates,
not measured hottest paths.

| Module | Flow | Evidence | Pressure |
| --- | --- | --- | --- |
| `canic-core/workflow/rpc/request/handler` | capability/replay | 15 primary/replay checkpoints and 33 passing focused tests elsewhere in Phase C | high shared-runtime |
| `canic-core/workflow/placement/scaling` | scaling | 6 checkpoints across observe/plan/create/register | medium-high |
| `canic-core/workflow/placement/sharding` | sharding | 11 checkpoints across bootstrap and assignment | high flow complexity |
| `canic-control-plane/workflow/bootstrap/root.rs` | bootstrap | 12 checkpoints across import/create/index/store stages | high root-only |
| `canic-control-plane/ops/storage/template/chunked.rs` | publication storage | 6 checkpoints across validation/capacity/upsert | high root/store |

## Hub Module Pressure

| Module | Crossed concerns | Layers | Pressure |
| --- | --- | ---: | --- |
| RPC request handler | request, capability, replay, policy, cost, execution | 4 | high |
| root bootstrap | topology, state, canister creation, store, publication | 4 | high |
| sharding assignment | registry, policy, allocation, persistence, metrics | 4 | high |
| template chunk store | validation, capacity, stable storage, metrics | 3 | medium-high |

## Dependency Fan-In Pressure

Current static fan-in evidence from the complexity/change-friction runs applies:

- `InternalError` appears in 154 non-test core files.
- `IcOps` appears in 63 files and `ConfigOps` in 34.
- `SubnetRegistryOps` appears in 32 files.
- The RPC and auth hubs cross at least four semantic domains.

These are investigation priorities only after measured rows exist. They do not
establish instruction cost.

## Early Warning Signals

- The frozen executable composite does not equal the runner's emitted
  fingerprint because absolute path text enters the hash.
- The exact coverage command silently reports zero despite 57 product
  checkpoints.
- Four required flow classes are absent from the frozen scenario roster.
- The canonical runner uses an obsolete direct Cargo Wasm build path and
  cannot reach the first root-probe measurement.
- Rejection, replayed, and cache-warm costs cannot be compared because no perf
  row exists and the current manifest includes only one fresh replay scenario.

No endpoint regression, rejection-path inflation, or improvement is claimed.

## Risk Score

Authoritative score: `blocked`.

| Rubric component | Score | Evidence |
| --- | ---: | --- |
| shared-runtime regression severity | N/A | no measured rows |
| hotspot concentration | N/A | no measured ranking |
| critical-flow checkpoint/roster gaps | 2 / 2 | four required classes absent; two trust flows uncheckpointed |
| comparability/method drift | 2 / 2 | identity mismatch and runner/build incompatibility |
| rejection/replay/cache sensitivity | N/A | no comparable scenarios |

The minimum observed evidence risk is 4 / 10. Missing components are not
treated as zero, so 4 / 10 is not the release risk score.

## Finding

### P1 - frozen instruction method is neither identity-stable nor executable

Canonical finding: `CANIC-092-AUDIT-015` (`audit_method_defect`, P1,
confirmed, open).

Required correction:

1. Mark this result invalid and preserve its blocked evidence.
2. Make the composite root-independent and prove it matches the frozen
   executable identity before execution.
3. Route every audit fixture through the current authoritative Canic Wasm
   builder without adding a direct-build compatibility bypass.
4. Fix the checkpoint scan to recognize maintained namespaced calls.
5. Version and complete the scenario roster for every required flow class.
6. Rerun the corrected method against immutable `v0.92.0` before accepting an
   instruction baseline.

The unchecked PocketIC download/checksum posture also supports existing
`CANIC-092-RELEASE-002`; no duplicate release finding is created.

## Verification Readout

| Check | Result | Evidence |
| --- | --- | --- |
| frozen definition SHA-256 | `PASS` | `f90bbd14...` |
| relative executable composite | `PASS` | equals frozen `c79f7027...` |
| runner-emitted executable composite | `FAIL` | `a5fa45ef...`, absolute-root dependent |
| source mutation guard | `PASS` | only report artifact paths changed |
| pinned PocketIC start | `PASS` | 14.0.0 listened in disposable environment |
| endpoint enumeration | `PASS` | 11 stable scenario tuples retained |
| exact documented checkpoint scan | `FAIL` | zero matches |
| broader product checkpoint scan | `PASS` | 57 calls across 11 product files |
| authoritative fixture build | `FAIL` | direct Cargo Wasm path rejected at build support |
| normalized perf rows | `BLOCKED` | none produced |
| checkpoint deltas / logs | `BLOCKED` | none produced |
| timer isolation | `PASS` | no timer scenario present |
| baseline deltas | `BLOCKED` | first invalid attempt, no rows |
| canonical runner | `FAIL` | exit 101 |

## Retained Evidence

- [scenario manifest](artifacts/instruction-footprint/scenario-manifest.json)
- [evidence manifest](artifacts/instruction-footprint/evidence-manifest.yml)

No product or frozen-method change was made during this run.
