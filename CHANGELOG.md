# Changelog

All notable, and occasionally less notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

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