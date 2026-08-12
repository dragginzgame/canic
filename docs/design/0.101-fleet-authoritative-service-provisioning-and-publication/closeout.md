# Canic 0.101 Closeout And Responsibility Report

Date: 2026-08-12

## Baseline And Scope

This report closes the mandatory 0.101 sediment-removal pass against released
baseline `v0.100.103` (`3c73a8b1c507b9cb3544f441ce1557c8c7fff89c`) on branch
`main`. The inspected release head is `v0.101.52`
(`997e0109fae575f84d30b009a1240ef795c719d3`) plus the open 0.101.53 working
batch. The release remains reinstall-only.

The production-source diff from `v0.100.103` contains 288 files, 49,895
insertions and 4,491 deletions across the runtime, control plane, host, CLI,
Coordinator and Store source roots. This includes the two new open-0.101.53
ownership modules, which are not yet in Git's tracked-file count. Those gross
numbers include the new
configuration compiler, protected plan and Directory contracts, Coordinator
and root state machines, recovery evidence and current application fixtures;
they are not a claim that all lines belong to one authority.

## Responsibility Accounting

| Current owner | Lines | Maintained responsibility | Closeout disposition |
| --- | ---: | --- | --- |
| `workflow/component_registry/mod.rs` | 8,832 | Root-local top-level Component and arbitrary-depth descendant lifecycle orchestration, including creation, activation, Directory synchronization, draining and recycling | Retained as one lifecycle workflow owner. Stable mutation remains in Component Registry ops/storage, pure admission remains in policy, and endpoints only authenticate/delegate. Further mechanical splitting would create forwarding namespaces across shared await/revalidation boundaries without removing authority. |
| `ops/fleet_coordinator/mod.rs` | 7,980 | Coordinator-owned Fleet Registry and Component-provisioning state transitions | Root physical-deletion authority was separated into `root_deletion`; deployment-ledger authority was already separate. The remaining functions share the single Coordinator registry/provisioning record and its atomic commit boundary. |
| `ops/component_provisioning.rs` | 3,848 | Root-owned aggregate Component Group provisioning state, cursors, protected result validation and terminal receipts | Retained as the one deterministic root provisioning state owner. Stable records are separate, and external effects remain in workflow. Splitting individual cursor phases would duplicate whole-record validation and weaken one-record atomicity. |
| `workflow/component_provisioning.rs` | 1,333 | Bounded root provisioning orchestration over existing Component identity, pool, Store, Registry and runtime journals | Retained as one workflow owner; it contains no stable schema or direct filesystem authority. |
| `ops/component_provisioning_plan/mod.rs` | 1,107 | Semantic validation of fresh and scale-out plans against compiled configuration and Fleet Registry authority | Canonical byte encoding was separated into the 256-line `canonical` owner; scale-out authority remains in its existing focused module. |
| `ops/fleet_coordinator/root_deletion/mod.rs` | 765 | Coordinator root-deletion readiness, execution identity, exact retry, cycle target, hashing and history validation | Extracted during Q5 from the accumulated Coordinator module without changing DTOs, stable records, domains or interruption behavior. |

Tests and disposable-environment fixtures are intentionally large because they
retain independent adversarial and restart journeys. In particular,
`workflow/fleet_coordinator/tests.rs` and the PocketIC Fleet Registry baseline
are evidence owners, not runtime authority. They are not folded into generic
helpers where doing so would hide different interruption boundaries.

The fresh-install host journal is no longer one accumulating owner. Its
directory module delegates to separate `model`, `validation`, `transition` and
`persistence` owners. Mechanically identical bounded canonical encoding,
no-follow reads, atomic replacement and exact replacement reconciliation are
shared through `canic-host::durable_io`; file names, locks, schemas, authority
checks and phase machines remain domain-specific.

## Deleted And Replaced Paths

- The standalone host `fleet_subnet_root_runtime_activation` module is deleted.
  Empty and populated roots now cross the same Coordinator-owned Directory and
  runtime transaction.
- Fresh-install Component provisioning uses the same protected plan, per-root
  batch, Directory barrier and runtime receipt model as later scale-out. There
  is no maintained alternate singleton-Spec, sole-admission or pre-service
  publication path.
- Initial placement semantics moved out of Fleet-plan persistence into the pure
  `fleet_install_plan::initial_placement_policy` owner.
- Root deletion is no longer mixed into the main Coordinator Registry and
  provisioning implementation file.
- Canonical provisioning-plan encoding is no longer interleaved with semantic
  validation.
- Coordinator and Wasm Store canonical Candid contracts are checked-in source
  authority. Ordinary builds copy and embed them; generated sidecars and debug
  builds are not fallback contract authorities.

## Hard-Cut And Stale-Occurrence Inventory

An active executable/configuration scan of `crates`, `apps`, `canisters` and
`scripts` has no occurrences of `TreeId`, `TreeSpecId`, `TreeGroupId`,
`TreeBinding`, `TreeRegistry`, `TreeDirectory`, `SubnetSlotId`, `subnet_slot`,
`SubnetRegistry`, `SubnetDirectory`, `fleet_root_pid`, `tree_root_pid`,
`is_fleet_root`, `--tree-spec`, `GroupCanister`, `group_canister`,
`group_parent` or `parent_group`.

The following nearby words remain intentionally and are not compatibility
surface:

- `selected_root` identifies roots selected by the current bounded placement
  plan. It is not a separate root authority, journal or protocol.
- `empty_root` identifies a current root with an exact zero-placement batch or
  zero-member initial inventory. It does not select the deleted direct host
  activation path.
- sibling Wasm Store `adoption` is the current fresh-install transition from
  installer-plus-root controllers to the separately installed Store's exact
  root-only controller set. No application Component, previous release or
  external installation is adopted.
- generic backup/restore `create_or_adopt` means exact same-release durable-file
  replay. It does not import a prior schema or installation.

Historical design, archived status, audit and changelog records retain old
terms so the development history remains intact. The active 0.101 design also
names forbidden concepts when stating non-goals and hard-cut requirements.
Those occurrences are descriptive prohibitions, not maintained interfaces.

No migration list, previous-layout decoder, legacy alias or compatibility
fallback exists in the 0.101 Component Group, provisioning-plan,
Coordinator-provisioning or root-provisioning owners. Stable state uses only
schema version 1 and `MigrationPolicy::NewDomain` with empty migration lists.

## Stable Memory And Dependency Isolation

The canonical stable-memory registry remains packed and collision-free:

- `0–9`: `ic-memory`;
- `10–29`: `canic-control-plane`, with template `10–13`, Store GC `14`,
  Coordinator `15`, root Store/Mirror/Registry `16–23`, prepaid Canister pool
  `24–26`, and root aggregate provisioning `27–29`;
- `30–59`: active `canic-core` runtime, Fleet, auth, replay, cycles, log,
  intent, receipt, placement, sharding, blob-storage and restore-fence state;
- `60–99`: reserved Canic core range;
- `100–254`: application allocations;
- `255`: invalid sentinel.

The allocation registry and control-plane descriptors agree on every active
ID, owner and consecutive provisioning domain. Managed runtime source cannot
bypass the explicit-key memory ABI.

The control-plane feature gate now compiles five exact graphs: no feature,
Fleet Coordinator only, Fleet Subnet Root only, Wasm Store only and the host
consumer. This makes the root isolation edge explicit instead of inferring it
from the combined host build. Cargo Machete finds no unused control-plane
dependency. Host filesystem/planning code is not a runtime dependency, and
role-gated control-plane modules keep Coordinator orchestration out of root and
application builds.

## Generated, Configuration And Public Surfaces

- Both canonical infrastructure DIDs parse and retain their current endpoint
  contracts. Ordinary host builds fail closed when a canonical DID is missing.
- The public protocol suite pins the Coordinator, root and Store endpoint
  guards, provisioning plan, Directory synchronization, pool, draining and
  physical-deletion surfaces.
- The checked-in ICP test Canisters match the configured Component topology,
  and the topology still derives an exact release set.
- The canonical configuration guide and every checked-in `canic.toml` parse and
  validate through the current schema.
- Complete build output classifies Fleet Coordinator, Fleet Subnet Root and
  Wasm Store as infrastructure while retaining one shared configured Cargo
  batch for root and Component compilation. Artifact rows report exact package
  versions and never fabricate per-role compilation time.
- The exact nine-role Toko fixture produced eight Component artifacts plus
  root, Coordinator and Store in 186.98s on first fixture-specific
  materialization and 60.54s on an identical retained-cache run. Configured
  artifacts fell from 126.58s to 52.27s; dedicated infrastructure work fell
  from 60.40s to 8.27s. These are artifact-build measurements, not a claim that
  a network-mutating operator install completed in that wall time.

## Fresh Closeout Evidence

The following focused commands were executed against the open 0.101.53 working
batch and passed:

- `bash scripts/ci/run-layering-guards.sh`;
- `bash scripts/ci/check-control-plane-feature-matrix.sh`;
- `cargo machete --with-metadata --skip-target-dir crates/canic-control-plane`;
- `cargo test --locked -p canic-core --test stable_memory_abi_guard`;
- `cargo test --locked -p canic-core --test candid_serde_boundary_guard`;
- `cargo test --locked -p canic-core role_contract::tests --lib` (16 tests);
- `cargo test --locked -p canic-control-plane state_contract::tests --lib`
  (4 tests);
- `cargo test --locked -p canic --test reference_surface` (3 tests);
- `cargo test --locked -p canic --test protocol_surface` (43 tests);
- `cargo test --locked -p canic --test config_guide`;
- `cargo test --locked -p canic-core
  config::schema::tests::every_checked_in_canic_config_parses_and_validates
  --lib`;
- `cargo test --locked -p canic-host candid --lib` (17 focused matches);
- `cargo test --locked -p canic-core
  ops::component_provisioning_plan::tests --lib` (13 tests, including frozen
  plan and root-batch hash vectors);
- `cargo clippy --locked -p canic-core --lib --tests -- -D warnings`;
- `cargo clippy --locked -p canic-control-plane --lib --tests -- -D warnings`;
- the complete focused Coordinator root-removal/deletion workflow test;
- the focused `canic-cli` build suite and `canic-host` artifact tests;
- `cargo fmt --all -- --check` and `git diff --check`.

The Toko artifact measurement used the exact same command twice:

```text
target/debug/canic build delegation_root_stub --workspace /home/adam/projects/canic --icp-root /tmp/canic-toko-q5-retained --config /home/adam/projects/canic/canisters/test/toko_topology/canic.toml --profile fast
```

The Q4 qualification report separately records the freshly executed
three-Subnet topology, shared-Subnet second Fleet, dynamic descendant and
interrupted scale-out evidence plus measured Wasm and canonical payload sizes.
This report does not relabel that recorded evidence as a new Q5 execution.

The maintainer-owned complete release validation and publish journey are not
run by this cleanup pass. Repository governance explicitly assigns automated
agents targeted validation only; absence of a second broad run does not replace
or weaken the focused evidence above.

## Closeout Disposition

No unexplained current-runtime, schema, Candid, CLI/configuration, generated,
fixture or active-document residue remains for the superseded 0.101 concepts.
The remaining large production files have one documented authority or workflow
responsibility, with mechanically separable root-deletion and canonical-byte
owners extracted. No stable allocation, feature edge or dependency was added
to preserve an earlier slice.

Application-data replication/readiness, health-based ActivePool eligibility,
promotion/failover, scale-in, autoscaling, cross-Fleet prepaid-Canister estate
transfer and the standalone blob-service extraction remain later designs.
They are not hidden 0.101 topology behavior and do not block this release's
closeout.
