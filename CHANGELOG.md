# Changelog

All notable, and occasionally less notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [0.1.20] - 2025-08-06

## [Unreleased]

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