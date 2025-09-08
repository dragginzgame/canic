# Changelog

All notable, and occasionally less notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]
- Changed: Shard helpers now use `CanisterType` instead of pool name.
  - `assign_in_pool(hub_type, canister_type, item)`
  - `plan_pool(hub_type, canister_type, item)`
  - `ensure_item_assignment_from_pool(hub_type, canister_type, item)`
  - Pools are resolved by matching `canister_type` in config; Player Hub call sites remain valid.
- DX: Pre-commit hook auto-formats (`cargo fmt`) and sorts (`cargo sort`, `cargo sort-derives`), then stages changes.
- CI: Add workflow `permissions` and `concurrency`; use `make fmt-check` and `make clippy` for consistency.
- Makefile: `install-dev` installs `cargo-sort` and `cargo-sort-derives` to support hooks locally.
- Scripts: Fix `scripts/app/version.sh` usage to remove non-implemented `release` subcommand.

## [0.7.3] - Partition Registry v2
- now you can configure multiple pools each with a different CanisterType

## [0.7.0] - Partition Registry
- partition registry v1 added and tested

## [0.6.8] - 2025-09-05
- Docs: Reduce/streamline documentation.
- CI: Minor workflow tweaks.
- removed re-exports of ic types as the versions will mess up downstream deps

## [0.6.7] - 2025-09-05
- CI: Workflow updates and cleanup.

## [0.6.6] - 2025-09-04
- Changed: Move `utils::serialization` to `utils::cbor`; introduce `SerializeError` and update imports.

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
- Changed: Config (`icu.toml`) supports per-canister `partition` block: `initial_capacity`, `max_partitions`, `growth_threshold_bps`.
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
- CanisterType in prelude
- fixed auth race condition
- added the CanisterPool structure to root only
- added uninstall_canister to the interface::ic

## [0.3.4] - 2025-08-20
- relaxed the restriction that directory canisters can only be created under root
- changed CanisterType to an enum

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
- icu_setup() before icu_install() and icu_upgrade()

## [0.2.5] - icu_init + icu_startup
- split these functions, now post_upgrade calls icu_startup in addition to icu_init

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
- updated icu_start! so it takes another optional argument to pass to the init function
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
