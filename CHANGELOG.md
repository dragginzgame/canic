# Changelog

All notable, and occasionally less notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

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
- changes to ergonomics on the CanisterRegistry

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
- not using candid Principal any more, switching to ic_principal and ic_ledger_types
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
- changed the WasmManager to a CanisterRegistry
- added a MemoryCounter to handle allocation of memory_ids

## [0.1.1]
- moved loads of IC/canister-specific, and shared code from Dragginz into icu
- have the old request/cascade/response code back and working

## [0.1.0]
- ITS ALIVE!11!1!!