# Canic IcyDB adapter changelog

Covers `canic-icydb`. Independent releases continue its existing 0.114.0 package
history; earlier coordinated releases remain in [shared history](../history.md).

## [0.115.0]

### ⚠️ Breaking

- Require Rust 1.99.1 and the supplying framework's Storage namespace-grant
  contract. Rebuild consumers with explicit AllocationGrants, one host lifecycle
  and one Storage owner; actual database composition stays downstream.

### 🔧 Changed

- Select Timers 0.40.3 and Platform 0.22.0. Qualify adapter Wire mounts with
  framework-owned report DTOs rather than serializing Metrics recorder types;
  preserve wide values and independent overflow flags. Actual database
  composition remains owned by downstream consumers.

- Follow the framework's current Timers family and pinned development/release
  compiler.

- Version and publish the adapter independently from the framework using the
  existing workspace and catalog. Keep its framework macro dependency explicit;
  CLI/runtime matching remains coordinated.
- Qualify its native startup watchdog and composed lifecycle exports through
  framework-owned PocketIC fixtures. Actual database startup remains downstream
  owned ([#590](https://github.com/dragginzgame/canic/issues/590)).
