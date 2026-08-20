# Canic 0.107 B1 Working Evidence

Date: 2026-08-20

## Authority And Scope

This bundle records the completed repository-local B1 contract capture for the
[0.107 design](../../../design/0.107-fresh-fleet-preflight-and-runtime-admission/0.107-design.md).
The maintainer accepted it on 2026-08-20 as the production and stable-state
implementation authority for the sequenced B2-B7 batches.

The immutable direct release baseline is annotated tag `v0.105.0`, peeled
commit `b6c46ca1d307e0a3fed6f7bfddfba7d9f1922811`. The planner,
installer, whitelist access path, stable-allocation registry and managed role
macros inspected by B1 were byte-for-byte that released source at capture.
The retained hashes remain the accepted predecessor baseline after subsequent
0.107 implementation. The working tree also contains accepted 0.106 evidence
and a separately classified 0.108 test probe; neither changes that baseline.

Toko was inspected read-only at commit
`bf14a5d3d89be4335d3da2601e8a60128fde04df`. No Toko file was modified.
The 0.106 B2 external-effect gate remains closed and no remote or IC-mainnet
operation was performed for this batch.

## Contents

- [B1 baseline and frozen contract](b1-baseline-and-contract.md) records the
  exact current source behavior, command and Candid spelling, pure-plan and
  digest contract, whitelist schema and bounds, authorization predicate, and
  smallest upstream diagnostic requirement.
- [Source baseline](source-baseline.tsv) freezes the exact local and read-only
  external file identities used by the capture.
- [`stable_memory_abi_guard.rs`](../../../../crates/canic-core/tests/stable_memory_abi_guard.rs)
  contains a test-only schema-1 fixture proving the chosen whitelist bounds.

## B1 Result

| Area | Frozen result | Remaining boundary |
| --- | --- | --- |
| CANIC-011 | Managed non-root `canic_command`/`canic_status` variants, Root-or-controller administration, memory ID 61, schema 1, 256 principals, 128-entry pages, one retained operation, and exact digest/replay rules are frozen. The maximum fixture encodes to 8,417 stable bytes; maximum status and mutation Candid are 4,072 and 101 bytes. | Production DTO/model/ops/workflow/macro and restoration work is scheduled for B6. |
| CANIC-012 | Exact `deploy plan`/`install` option parity, pre-effect ordering, one pure compiler, canonical payload and SHA-256 domain are frozen. | B2 target forwarding is complete; B3-B4 own complete input loading, plan/install parity, receipts and funding evidence. |
| CANIC-013 | `ic-query 0.40.1` is exact. Its current failure API loses known Registry-version and cache-attempt context and has no `Unknown(reason)` retry result. The smallest additive upstream result is frozen without a fork or string parsing. | B5 may not claim complete provenance until a committed or published upstream API supplies the missing typed fields. |
| Downstream sizing | Toko's current compiled whitelist contains 175 principals, leaving 81 entries beneath the frozen hard maximum. | The B7 read-only acceptance rerun needs a downstream source that actually exercises or reports CANIC-011/012/013; the inspected 2025 source has no Canic integration or feedback identifiers. |
| Production impact | None. All B1 executable evidence is test-only. | B1 was accepted on 2026-08-20; sequenced B2-B7 implementation may proceed. |

## Effect Ledger

- Production source changed by B1: none.
- Stable-state implementation changed by B1: none.
- Candid or CLI changed by B1: none.
- External Canisters created: zero.
- Cycles transferred or consumed: zero.
- Remote or IC-mainnet calls: zero.
- Toko or another sibling repository changed: zero.

## Acceptance Boundary

The maintainer's 2026-08-20 acceptance freezes the contracts in
[B1 baseline and frozen contract](b1-baseline-and-contract.md), including the
memory allocation and public variant names and authorizes the already
sequenced B2-B7 implementation within the 0.107 design. B2 subsequently
changed only the target-correct CLI planning boundary.
