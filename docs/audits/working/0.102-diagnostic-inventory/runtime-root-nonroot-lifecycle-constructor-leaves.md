# Canic 0.102 Root And Non-Root Runtime Lifecycle Constructor Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger classifies all eleven production `InternalError`
constructor references in root and non-root runtime lifecycle orchestration. It
assigns no number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/runtime/root.rs` | 6 |
| `workflow/runtime/nonroot.rs` | 5 |
| **Total** | **11** |

## Fleet Subnet Root Lifecycle

The six root lifecycle sites add no exact meaning:

| Exact candidate or disposition | Sites | Producer function/branch | Required hard cut |
| --- | ---: | --- | --- |
| transparent typed memory-bootstrap cause | 1 | `init_root_canister`; stable-memory registry bootstrap | Preserve the exact `ic-memory` adapter diagnostic |
| reuse `ACCESS_BUILD_NETWORK_UNAVAILABLE` | 1 | `init_root_canister`; embedded build-network identity | Rebuild with exact IC/local network configuration |
| reuse `ENV_REQUIRED_FIELDS_MISSING` | 1 | `init_root_canister`; root environment policy | Return the existing environment identity without the missing-field list |
| transparent typed environment-import cause | 1 | `init_root_canister`; protected root environment import | Preserve the exact environment diagnostic and approved projection |
| transparent typed configuration cause | 1 | `init_root_canister`; application init-mode lookup | Preserve the exact configuration diagnostic |
| transparent typed runtime-startup cause | 1 | `post_upgrade_root_canister_after_memory_init`; Active root service startup | Preserve the exact failing service diagnostic without a root-startup wrapper |

Lifecycle phase does not create another error meaning. Memory bootstrap occurs
before protected state initialization; environment and configuration admission
occur before root service startup; and startup runs only after re-observing an
Active Fleet activation phase. Each typed cause retains its own authority and
retry boundary.

The root build-network check reuses the qualified access identity because it
reads the same immutable build fact and has the same rebuild action. Missing
environment fields similarly reuse the exact policy identity already shared by
`EnvPolicyError` and environment accessors.

## Non-Root Lifecycle

All five non-root constructors are transparent typed edges:

| Exact candidate or disposition | Sites | Producer function/branch | Required hard cut |
| --- | ---: | --- | --- |
| transparent typed environment workflow cause | 2 | `init_wasm_store_canister` and `init_local_nonroot_canister`; environment initialization | Preserve exact environment admission/import cause |
| transparent typed memory-bootstrap cause | 1 | `initialize_nonroot_base`; stable-memory registry bootstrap | Preserve the exact `ic-memory` adapter diagnostic |
| transparent typed configuration cause | 2 | `register_nonroot_runtime_contract` and `restore_nonroot_after_upgrade`; current-role configuration | Preserve the exact configuration diagnostic |

Managed Component/Component Child authority, sibling Store authority and
standalone-local initialization remain distinct lifecycle routes, but wrapping
their typed environment or configuration error with route prose does not add a
semantic leaf. Prepared managed Canisters still do not start timers or
application hooks; post-upgrade startup remains conditional on re-observed
Active state.

## Dynamic Public Context

Ten formatted typed values are classified as `DPC-254` through `DPC-263` in
[dynamic-public-context.md](dynamic-public-context.md). They are memory,
environment, configuration or nested runtime causes. Each must retain its
registered identity and approved projection; missing-field lists and nested
error prose are discarded.

## Reconciliation

All eleven direct sites now have one disposition. They add no exact meaning,
reuse two existing identities and retain nine transparent typed edges. The
effective constructor frontier moves from 2,299 to 2,310 classified sites and
from 200 to 189 open sites. The qualified semantic set remains 2,504 exact
candidates plus 31 safe projections: 2,535 current symbolic identities.

## Required Tests

- root and non-root memory bootstrap preserve exact adapter diagnostics;
- missing build-network and environment-field failures reuse their established
  identities;
- environment and configuration wrappers preserve source class/projection;
- root service startup returns the exact nested service diagnostic;
- prepared managed non-roots still schedule no runtime/application work;
- post-upgrade startup remains fenced by re-observed Active state; and
- no dynamic missing-field, configuration or nested-error prose remains in the
  compact diagnostic.

## Next Slice

Continue with runtime module coordination, authority-restore orchestration and
Fleet activation lifecycle gates.
