# Changelog

All notable, and occasionally less notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [0.6.2]
- added back build_network() that reads in option_env!(DFX_NETWORK), and added access policies
- refactored testkit::pic so it uses a static variable for all tests (we were running out of chunks)
- canic-macros weren't passing through clippy lints

## [0.6.0] - Aquafresh 3-in-1 Endpoint Protection

### Changed
- Major internal refactor: removed the old `ops/` and `model/` interface layer; wrappers were removed or split between crates.
- `canic-macros` endpoints now support three levels of endpoint security and automatically apply `perf_scope`.
- Reserve subsystem refactor: move reserve orchestration into `ops::reserve` + `ops::service` and consolidate state access via `ops::storage`.

### Added
- Split metrics queries into per-metric endpoints: `canic_metrics_system`, `canic_metrics_icc(page)`, `canic_metrics_http(page)`, `canic_metrics_timer(page)`, `canic_metrics_access(page)`.

### Removed
- Removed the aggregated `canic_metrics` endpoint and `MetricsReport` type.

## [0.5.22] - 2025-12-13
### Added
- CI now builds all canister `.wasm` artifacts (and deterministic `.wasm.gz` via `gzip -n`) into `.dfx/local/canisters/...` before running `fmt`, `clippy`, and tests.
- New `canic-macros` crate with `#[canic_query]` / `#[canic_update]` proc-macro attributes.
- Centralized endpoint dispatch wrappers (sync + async query/update) to unify perf instrumentation and future endpoint hooks.

### Changed
- Config loading is now unconditional in lifecycle; build scripts always provide `CANIC_CONFIG_PATH`, generating a minimal default config when the repo config file is missing.
- Perf instrumentation switched to call-context instruction counter (`ic0.performance_counter(1)`); perf aggregation is now keyed by kind (`Endpoint(name)` vs `Timer(label)`) to avoid label collisions.
- Whitelist enforcement now always consults `Config` (no longer gated behind `feature = "ic"`).
- Root canister embeds dependent canister `.wasm.gz` on `wasm32` builds (non-wasm builds use empty slices).

### Fixed
- `perf_scope!` now reliably records at scope exit (RAII guard lifetime/shadowing).
- Stable memory range initialization is idempotent when re-registering the same initial range (prevents upgrade traps).

### Removed
- `EnvError`; SNS principals now fail-fast on build if invalid.
- All custom cfg-based CI conditionals (notably `cfg(canic_github_ci)`) and related build-script cfg emissions.
- Dead `DFX_NETWORK` network helper.

## [0.5.21] - Perf & Types Consolidation
- Labeled timer metrics: `TimerMetrics` now records mode, delay, and a caller-provided label so scheduled tasks can be distinguished in metrics; interval timers increment on every tick.
- `canic_perf` diagnostic query and instruction aggregation for timer executions (labels + total instructions) to inspect timer cost without inflating main metrics.
- Added `timer!` and `timer_interval!` macros that auto-label timers with `module_path::function` and route through `TimerOps` for perf recording.
- bumped rust to 1.92.0

## [0.5.17] - 2025-12-11 - HTTP Metrics
### Added
- Ops-level `http_get` helper for JSON GETs that records HTTP outcall metrics alongside the system counters.
- Timer metrics wrapper to record scheduled timers (once + interval) and track their cadence alongside other system metrics.

### Changed
- Metrics reporting now distinguishes HTTP outcalls and the main metrics façade is called `SystemMetrics`.

## [0.5.16] - 2025-12-11 - O(n^2) -> O(n)
### Fixed
- Decode `notify_top_up` responses from the CMC and surface errors instead of treating any reply as success, so failed cycle top-ups no longer appear successful.

### Changed
- Topology sync bundles now carry only the parent chain and per-node direct children (no full subtree), removing the quadratic fanout cost and matching the stored parent/child snapshot.

## [0.5.15] - 2025-12-11 - Canister Lifecycle Orchestrator
- simplified the reserve-pool subsystem to make canister recycling more reliable and easier to maintain.
- A new internal utility (recycle_via_orchestrator) integrates cleanly with the orchestrator so that recycling automatically triggers topology/directory updates when required.
- changed (limit, offset) endpoint arguments to use a unified struct

## [0.5.14] - 2025-12-10 - Icc / System Metrics
- split Metrics into two types, System and Inter-canister calls
- Pagination queries now take a `PageRequest` (with defaults and a 1,000 item cap) instead of raw `offset`/`limit` pairs for logs, directories, cycle tracker, and topology children.

## [0.5.13] - 2025-12-10 - Canic Metrics
- Wrapped cross-canister call construction so `CanisterCall` metrics are recorded centrally without scattered increments.
- Targeted topology cascades now delegate to the first child (letting the branch fan out) to honor parent-only auth and cut hop count.
- Added PocketIC coverage for worker creation ensuring new workers register under `scale_hub` and appear in its child view.

## [0.5.12] - 2025-12-10
- Topology syncs are now branch-targeted when creating canisters: root cascades only the affected subtree, retries once per hop, and falls back to a full cascade on errors. Large cascades log warnings so noisy fan-outs are visible.

## [0.5.10]
- added a wrapper around performance_counter
- added more types to ICRC2 (Allowance, TransferFromArgs, etc.)

## [0.5.8] - 2025-12-09
- Reduced topology cascade complexity: subtree extraction now builds a parent→children index once and reuses it for all child bundles, and registry subtrees walk the stable map directly without repeated scans. This keeps syncs near linear even with hundreds of canisters.
- Added targeted topology cascade from root so creates only cascade the affected branch (root→child→…→leaf), with retries and a safe fallback to full cascade if any hop fails.

## [0.5.7] - 2025-12-08
- Added caller/parent context logs for create_canister_request and the root handler so bootstrap failures during repeated create calls surface clearly.

## [0.5.6] - 2025-12-07
### Added
- One timer service entry point to start all background jobs (logs, cycle tracker, reserve) per canister role.
- Info-level tick logs for retention and cycle tracking so you can see timers firing.

### Fixed
- Root init no longer traps if auto-creating canisters fails; it now logs the error and keeps running.
- Log retention moved to a timer instead of every write, keeping logging cheap while still cleaning up.
- Cycle tracker purge now runs on the timer loop instead of a modulus counter, aligning all cleanup on scheduled ticks.

## [0.5.4] - 2025-12-06
- Hardened reserve imports: uninstall first, reset controllers, then remove from registry and recascade before registering into the reserve pool.
- Added a management delete wrapper and explicit delete path separate from uninstall.
- `impl_storable_*` macros now panic with contextual messages on (de)serialization errors and ship basic round-trip/corrupt-data tests.
- Refreshed `canic-memory` README with simpler “why/how” guidance, boot log example, and clearer eager TLS rationale.

## [0.5.2] - 2025-12-06
- Split stable-memory plumbing into the new `canic-memory` crate (manager, registry, eager TLS, macros) and re-exported its macros/runtime from `canic`; added registry/eager-init tests and ops wrapper for initialization.

## [0.5.1] - 2025-12-05
- Moved general-purpose wrappers (Account, Cycles, BoundedString, WasmModule, ULID) into `canic-core::types` and slimmed `canic::types` to topology roles.

## [0.5.0] - canic-cdk breaking change - 2025-12-05
- Added the `canic-cdk` crate as a curated façade over `ic-cdk`, `candid`, timers, and management canister APIs.
- Introduced `canic-core` as the shared types/utils crate (perf macros, MiniCBOR serializers, bounded strings/ULID/cycles, wasm/time/hash helpers); re-exported via `canic::core` and replaces the old `canic-utils` crate.

## [0.4.12] - 2025-12-04
- Removed the auth-specific `verify_auth_token`; callers now pass the signing domain and seed into `ops::signature::verify` when validating tokens.
- Fixed `canic_subnet_canister_children` on root by rebuilding the view from the registry instead of the empty local snapshot.
- Register canisters in the subnet registry before install so init hooks can see themselves; roll back the entry on install failure to avoid phantom records.

## [0.4.8] - 2025-12-04
- made the memory data structures pub(crate), and removed unused code
- commented more public facing functions

## [0.4.7] - 2025-12-04
- Fixed canister signature verification panic on short (10-byte) canister principals by constructing the DER-encoded public key with the signing seed

## [0.4.6] - 2025-12-03 - e2e Tests
- AppDirectory now rebuilds from the registry on root (not just prime root) while children read their stable snapshot, keeping directory queries consistent everywhere.
- SubnetDirectory resolves from the registry on root and falls back to an empty view instead of erroring during early bootstrap/config gaps.
- Added PocketIC coverage that asserts app/subnet directory views match across root and all children after auto-create.
- fixed missing Ops passthrough functions

## [0.4.1] - 2025-12-01 - Bug Splatting
- Register new canisters in the subnet registry only after a successful install to avoid phantom entries on install failure.
- Post-upgrade now replays memory range/ID registrations so new stable-memory segments are validated after upgrades.
- Failed canister installs recycle the allocated canister into the reserve instead of leaving it orphaned.
- Fix ICP→cycles conversion to use ICP-per-XDR and add coverage for the buffered calculation.
- Sharding planner now skips full shards and requests creation when capacity is exhausted.
- Reserve imports reset controllers to the configured set, and registry records track upgraded module hashes.
- Narrowed internal sharding/pagination helpers to crate scope to shrink the public surface.
- Removed unused shard metrics helpers.

## [0.4.0] - 2025-12-01 - endpoints -> ops -> model
- Endpoints now call a slim ops façade; ops owns orchestration and DTOs; model stays pure storage/registries.
- ICRC helpers added to ops for supported standards and consent messages.
- Sharding, topology, directory, reserve, and env access now flow through ops (no direct model calls).
- State and topology sync now use ops DTOs and cascade helpers; logging writes routed through LogOps.
- Auth, request handling, and canister lifecycle updated to enforce layering while keeping behavior the same.

## [0.3.15] - 2025-11-29
- app and subnet_directory() now are on all canisters, use pagination and a proper DTO return type

## [0.3.0] - 2025-11-15
- Added paginated `canic_subnet_canister_children` via `CanisterChildrenOps::page` and `CanisterChildrenPage` DTO, mirroring CycleTracker paging.
- Introduced global log retention config (`max_entries` ring cap + optional `max_age_secs`) with second-level timestamps and enforced trimming.
- Documented the new log config block and refreshed README layout to match current modules.
- Added notes about the cross-filesystem compilation error for the LLM
- fixed logging so that the message is stored correctly, and made the log! macro more ergonomic and include topic
- moved all the mimic utils into canic-utils so they can be used independently
- added FromStr for Account
- added crate_name to the logs, plus filtering on the front end
- Scaling now uses plan_create_worker so there aren't two parallel paths for checking if a worker can be spawned
- lots of work going through the codebase and moving state and memory into model

## [0.2.24] - 2025-11-10
- added a test/ module that's gated by cfg(test) for pocket-ic helpers

## [0.2.21] - 2025-10-24
- fixed config validation, now its finding nested invalid canister types

## [0.2.17] - 2025-10-20
- removed icrc-ledger-types and implemented it manually

## [0.2.10] - 2025-10-20
- made the Sharding data structures use String not Principal so they're more flexible
- updated scaling to use HRW algo always, removed a lot of unused code that won't make sense going forward

## [0.2.9] - 2025-10-18
- gave config a better recursive validation.  Also now checking for invalid subnet directory entries

## [0.2.7] - 2025-10-16
- moved xxhash functions to canic as mimic can import them, and we also need them for sharding

## [0.2.6] - 2025-10-16
- moved more of the memory:: logic to Ops, and split things like CycleTracker vs. CycleTrackerOps
- moved the CanisterReserve config to be on a per-subnet basis

## [0.2.3] - 2025-10-15
- app_directory and subnet_directory are now calculated from the SubnetCanisterRegistry
- directories are now part of CanisterInitPayload, with the Env struct, sent to a canister as its created

## [0.2.2] - 2025-10-13
- removed all the delegation code
- added in ops::signature, a wrapper around creating and verifying canister signatures

## [0.2.1] - 2025-10-13
- bug fixes as expected

## [0.2.0] - 2025-10-13 - PRIME Subnet
- Added the SubnetRole, so we can have a Prime Subnet and others
- Added an Env cell so each canister remembers its root, subnet, parent, and type IDs.
- Split topology storage into dedicated directory modules and updated the ops helpers to use them.
- AppDirectory is now an App-level canister directory
- SyncBundle will sync both states and directories now
- Tons of little code improvements, especially splitting memory:: and ops::

## [0.1.7] - 2025-10-08
- with dfx 0.30 now the subnet's pid can be read, and stored in the root's SubnetContext

## [0.1.4] - 2025-10-07
- added ops::delegation::sync_session_with_source to stop repeated code in toko
- added debug! macro that always does Log::Debug and has a conditional first argument

## [0.1.3] - 2025-10-05
- new logo and README.  Got Codex to check all the documentation to make sure it's more up-to-date.
- removed a load of outdated documentation

## [0.1.0] - 2025-10-04 - Published!
- renamed to canic (like mechanic) because icu was taken by a unicode library on crates.io
- publishing to crates.io.  I wouldn't use it in its current form though muhaha!  Lots more to come.

############################ icu ######################################

## [0.12.0] - 2025-09-28 - Scaling Canisters
- so now in addition to Sharding you have Scaling which spins up and down a pool of canisters based
on available resources
- memory ranges nicely ordered

## [0.11.0] - 2025-09-25 - Memory Ranges
- now you can register a Memory Range for an application.  For instance, icu is limited between 0-4 for the Memory
Registry and 5-30 for icu-native memories.
- added BoundedString8 -> 256 as stable memory types
- AppState and CanisterState moved to memory::state.  Added SubnetState as the layer in between

## [0.10.5] - 2025-09-23
- split Topology and State syncs so they can be done independently, no point syncing state if topology
is wrong
- added the first pocket-ic test

## [0.10.4] - 2025-09-22
- big rewrite of memory:: with new CanisterView and CanisterEntry.  root is now authorative on
everything and only syncs what it needs to

## [0.9.15] - 2025-09-21
- made SubnetDirectory + co into zero sized handles so root can return different versions

## [0.9.11] - 2025-09-21
- added ICRC-103 to standards
- fixed a few nasty bugs in the canister pools

## [0.9.3] - 2025-09-17
- split off Subnet Views, fixed the bug where state wasn't cascading
- added find_by_type for parent
- added CreateCanisterParent::Directory
- added SubnetChildren::find_by_type and find_first_by_typeit ca

## [0.8.6] - 2025-09-17
- added icu_config endpoint for controllers

## [0.8.4] - 2025-09-17
- made initial_cycles default to 5T

## [0.8.2] - 2025-09-16
- fixed the broken candid/serde deps
- fixed the broken delegation macro code
- renamed the crates to what they actually are/do (blank, sharder, delegation)

## [0.8.0] - Delegation Layering Overhaul
- Changed: Rebuilt `state::delegation` as pure in-memory registries (`cache.rs`,
`registry.rs`) with focused unit tests.
- Added: `ops::delegation::DelegationRegistry` now owns session policy, cleanup cadence,
requester tracking, and exposes view/list helpers.
- Changed: Delegation endpoints route through the ops layer, returning proper `Result<…>` and logging policy decisions.
- Added: `DelegationRegistry::track` deduplicates requesting canisters and records them
with audit logs; new coverage test ensures idempotency.
- Docs: README notes the leaner `DelegationSessionView` (caller infers expiry from
`expires_at`).

## [0.7.3] - Partition Registry v2
- now you can configure multiple pools each with a different CanisterRole

## [0.7.0] - Partition Registry
- partition registry v1 added and tested

## [0.6.8] - 2025-09-05
- Docs: Reduce/streamline documentation.
- CI: Minor workflow tweaks.
- removed re-exports of ic types as the versions will mess up downstream deps

## [0.6.7] - 2025-09-05
- CI: Workflow updates and cleanup.

## [0.6.6] - 2025-09-04
- Changed: Move `utils::serialization` utilities into `core::serialize`; introduce `SerializeError` and update imports.

## [0.6.5] - 2025-09-04
- Maintenance: Version bump; no functional changes.

## [0.6.4] - 2025-09-04
- CI: Fix pipeline stability issues.
- Utils/Rand: Revert earlier RNG change; retain thread-safe tinyrand `StdRand` with `LazyLock<Mutex<...>>`.

## [0.6.3] - 2025-09-04
- Added: PartitionRegistry for item→partition assignment with capacities, retirement, audit/export.
- Added: Partition endpoints (cfg-gated): `icu_partition_registry`, `icu_partition_lookup`, `icu_partition_register`, `icu_partition_audit`.
- Added: Ops helpers for partitioning: `ensure_item_assignment`, `assign_with_config`, `assign_with_policy`, `plan_with_config`, and `PartitionPolicy`.
- Added: Auto-registration of non-root canisters from config `partition` block during init/upgrade.
- Changed: Config (`canic.toml`) supports per-canister `partition` block: `initial_capacity`, `max_partitions`, `growth_threshold_bps`.
- Added: Delegation revoke endpoint `icu_delegation_revoke` and registry method `revoke_session_or_wallet`.

## [0.6.2] - 2025-09-04
- State/Delegation: Fix session expiration boundary (now expired when `expires_at <= now`).
- State/Delegation: Add admin endpoints
  - `icu_delegation_list_all` (query): list all sessions (requires controller).
  - `icu_delegation_list_by_wallet` (query): list sessions for a wallet (requires controller).
  - `icu_delegation_cleanup` (update): remove expired sessions immediately (requires parent).
- State/Delegation: Expose `DelegationRegistry::cleanup()` publicly; add boundary unit test.

## [0.6.0] - 2025-08-31
- Added AGENTS.md with concise repository/contributor guidelines
- Added PR template `.github/pull_request_template.md`
- Introduced runnable examples under `crates/icu/examples/` and a doctest in `lib.rs`
- Makefile: new `examples` target to build examples (default and `ic` feature)
- CI: enforce `cargo fmt --check`, build examples, and run doctests
- README: linked guidelines and examples for easier discovery

## [0.6.1] - 2025-08-31
- Docs: Added CONFIG.md (schema + loading), improved rustdocs for auth/config
- Structure: Moved serialization to `utils/serialization.rs`; added `spec/` and `ops/` READMEs
- CI: Pin MSRV (1.89.0) in workflows; add clippy `--all-features`
- DX: `make install-canister-deps` for rustup target + candid-extractor
- Examples: Fixed compile warnings, clarified minimal root example notes

## [0.5.9] - 2025-08-27
- call and candid errors now go into the top level error struct, saving lots of boilerplate code

## [0.5.3] - 2025-08-25
- did a few patches to fix bugs
- added ICTS standards to endpoints

## [0.5.0] - Interface & Spec
- ok now we're really getting into the IC frame of mind, started wrapping as much as we could and
adding canister IDs to config

## [0.4.6] - CanisterPool Config
- now the pool will always be created, but you can specify the minimum size
- it will also create a maximum of 10 on any one check, spaced 30 mins apart

## [0.4.4] - Cycle Topup
- moved the canister attribute stuff to the config file
- CanisterCatalog is now WasmRegistry
- canisters now send an automatic topup request to root if they are configured to

## [0.4.0] - Canister Pool
- Rewrote a lot of the canister states, now we have CanisterChildren, CanisterDirectory, CanisterRegistry
- CanisterConfig -> CanisterCatalog(Type, Config)
- added icu_create_pool_canister and icu_move_canister_to_pool

## [0.3.8] - 2025-08-21
- fixed patch script

## [0.3.7] - 2025-08-21
- ic-stable-structures bumped to 0.7.0
- CanisterRole in prelude
- fixed auth race condition
- added the CanisterPool structure to root only
- added uninstall_canister to the interface::ic

## [0.3.4] - 2025-08-20
- relaxed the restriction that directory canisters can only be created under root
- changed CanisterRole to an enum

## [0.3.3] - 2025-08-19
- 💥CanisterUpgrade, Create, Cycles requests now all return their appropriate responses, not an enum

## [0.3.2] - 2025-08-19
- 💥SubnetIndex renamed to SubnetDirectory, and SubnetRegistry added to root

## [0.3.1] - It's a Bit Breaky!
- 💥icu_canister_upgrade_children now returns a Vec<Result>
- 💥create_canister_request now returns a Response::CreateCanisterResponse
- 💥added a root/child auth check to responses - will break stuff

## [0.3.0] - Cycle Tracker
- added the CycleTracker stable memory
- rewrote all stable memory wrappers so they can be tested properly
- removed wrapper for Cell and BTreeSet as they were redundant

## [0.2.32]
- Restructured the config model into typed subnet and canister sections with whitelist checks.
- Added an Env cell so each canister remembers its root, subnet, parent, and type IDs.
- Split topology storage into dedicated directory modules and updated the ops helpers to use them.
- Refreshed the lifecycle and start macros so cycle tracking and the reserve start with the new layout.
- Removed the CanisterParents memory wrapper because parent tracking now lives in Env.

## [0.2.31] - 2025-08-16
- changed Config to an Arc as it could get big and can potentially be requested many times

## [0.2.30] - 2025-08-15
- whitelist now just only works on mainnet, don't need bypass any more
- removed CandidType from Config, and removed endpoint to avoid unneccessary bloat and
possible security issues

## [0.2.29] - 2025-08-14
- moved icrc supported standards into Config
- config is now created by default, so no error variant when retrieving the config
- config now implements serde deny_unknown_fields
- icu_build!() macro so that config errors can be caught at compile time not on deploy
- added VERSION

## [0.2.19]
- added an icu_canister_status endpoint to all canisters
- fixed the error when sending a include_bytes!() to github actions

## [0.2.12]
- config toml now uses #[serde(default)]

## [0.2.9]
- now having no whitelist at all means that is_whitelisted() won't return an auth error

## [0.2.8] - CANISTERS
- now canisters are stored in a constant slice and made the import procedure much easier
- canic_setup() before canic_install() and canic_upgrade()

## [0.2.5] - icu_init + canic::startup
- split these functions, now post_upgrade calls canic::startup in addition to icu_init

## [0.2.3] - Toko Time Really
- use this for toko

## [0.2.1] - 2025-08-11
- changes to ergonomics on the CanisterConfig

## [0.2.0] - Toko Time
- fresh new minor release as we're gearing up for Toko now

## [0.1.29] - 2025-08-09
- new SubnetIndex, now you can store many canisters per type
- moved all the root canister registry to canister/ and cleaned up unused structs
- try_get_singleton() for SubnetIndex

## [0.1.26] - 2025-08-09
- overhaul of cascade/update state.  One function to transfer any number of states and
it's also sent via canister create args

## [0.1.25] - 2025-08-08
- redid cascade so it's just one endpoint and has a bundle of optional data types
- removed Serialize where it wasn't needed

## [0.1.24] - 2025-08-08
- fixed nasty cascade bug
- renamed canister methods to be consistent with ic cdk
- update to rust 1.89

## [0.1.23] - 2025-08-07
- removed the test.wasm because it wasnt building

## [0.1.21] - 2025-08-07
- fixed a bug in subnet_index, a race condition when adding the child index

## [0.1.19] - 2025-08-06
- removed the ability for custom controllers as it's all in the config file now

## [0.1.18] - 2025-08-06
- getting the hang of github tags

## [0.1.15] - 2025-08-06
- added config, with principals and whitelist.  icu_config("filename.toml")
- added is_whitelisted auth rule

## [0.1.14]
- now doing tagged releases

## [0.1.10]
- added a way to make ICRC-21 easy
- made ICRC-10 native
- added a DelegationCache for other canisters to query an Auth canister
- made all the CanisterState / Registry errors type 'Error' not the internal Error type
- rewrote the API for all states and memories to be consistent

## [0.1.9]
- added a whole state for session delegations
- added a utils/ module and moved rand, hash and time from mimic

## [0.1.8]
- complete refactor of stable structures, much better now!
- also updated to ic-stable-structures 0.7.0

## [0.1.7]
- now create_canister always sends args, Option<Vec<u8>>
- you can specify extra controllers when creating canisters

## [0.1.6]
- auth rewritten to be async, and to just use function names
- perf and perf_start got a big upgrade
- changed the underlying serialization method from ciborium to minicbor-serde
- only one init_async now, we don't have a race condition with the init_setup

## [0.1.5]
- added wrapper for BTreeSet from ic-stable-structures 0.6.9
- adding in ic-management-canister-types

## [0.1.4]
- refactored into two crates, just so I have a test crate to play with
- updated canic::start! so it takes another optional argument to pass to the init function
- added a timer for init_async so we dont call it from the macro
- auth rules working, now with support for custom auth rules

## [0.1.3]
- memory counter has now evolved into a Registry

## [0.1.2]
- changed the WasmManager to a CanisterConfig
- added a MemoryCounter to handle allocation of memory_ids

## [0.1.1]
- moved loads of IC/canister-specific, and shared code from Dragginz into icu
- have the old request/cascade/response code back and working

## [0.1.0]
- ITS ALIVE!11!1!!
