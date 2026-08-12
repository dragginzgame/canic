# Canic 0.101 Topology Qualification

Date: 2026-08-12

Baseline: `v0.101.51` (`c20ed1a57148e860e46742c991de872de9edefc8`), plus the open Q4 worktree.

## Result

The disposable Q4 journey passes for the first supported Toko-shaped topology.
It qualifies Canic's topology, placement, protected policy, lifecycle and
discovery contracts. It does not qualify application-level database
replication, failover, health or readiness.

The Toko topology has its own checked-in root configuration and Wasm build
target. The smaller delegation-root fixture retains its original topology, so
the qualification cannot silently broaden unrelated grouped, cross-root or
restart journeys through shared build-time configuration.

The journey uses three distinct PocketIC application Subnets for one Fleet:

- root 0 owns database A, B and C Authority Components;
- roots 1 and 2 each own same-Spec database A, B and C Replica Components and
  one Project Hub PoolMember from the reused nested project-data-cell group;
- the same Project Hub group is also reused by an ActivePool deployment that
  packs two stable placements on root 1, retains one differently configured
  placement, and atomically scales one further placement onto root 2;
- the four ActivePool members finish split two per application root;
- the 10,000-child deployment retains the unreduced Spec grant, while all
  three packed-deployment runtimes retain the exact 2,000-child reduction;
- the two project-data-cell Hubs create three Project Instances split across
  the application roots, every Instance creates one Ledger, and exactly one creates
  a Machine;
- all seven dynamic descendants remain on their owning root's physical
  Subnet, preserve their exact immediate parent and do not mutate the Fleet
  Registry; and
- a second Fleet installs its own Coordinator, root, Store, pool and Registry
  on the first Fleet's root-1 Subnet without entering either authority.

Both scale-out operations stop and restart the Coordinator after every durable
advance. Exact replay preserves the original status, placement, service member
and Canister identities. The journey also exposed and fixed one real replay
defect: after the first Component activation response was durably recorded,
replaying the original request was incorrectly rejected because the current
cursor had advanced to that first Component. Replay now recognizes only that
exact first-progress state; arbitrary cursor mismatches still conflict.

## Measured executed envelope

| Quantity | Measured value |
| --- | ---: |
| Main-Fleet roots | 3 |
| Configured deployments used | 4 |
| Materialized group placements | 7 |
| Top-level Components | 15 |
| Fleet services | 5 |
| Fleet-service members | 15 |
| Dynamic descendants | 7 |
| Initial canonical provisioning plan | 4,464 bytes |
| Final canonical Fleet Registry | 4,769 bytes |
| Largest root Fleet Directory Candid value | 2,854 bytes |

The checked-in policy permits at most ten project-data-cell placements and
fifteen Project Hub Components for this fixture, with the normal deployment,
root-admission and service-density checks applying before effects. The
executed evidence above materializes two project-data cells. Ten cells are a
bounded configuration envelope, not a claim that ten physical cells or ten
Subnets were exercised here.

The existing hard protocol ceilings remain the admission boundary: 4,096 plan
root batches, Directory-confirmation roots, placements, Component entries,
configured deployments and Fleet-service targets; 8 MiB canonical plan and
root-batch documents; 2 MiB canonical Fleet Registry bytes; and 64 KiB compact
root publication and activation payloads. Existing first-excess tests reject
the structural, placement and admission bounds before effects, while endpoint
payload quotas reject oversized requests before handler execution. This report
does not increase any of them.

## Wasm footprint

These are the exact optimized test artifacts used by the passing journey.

| Role | Raw bytes | Gzip bytes |
| --- | ---: | ---: |
| Fleet Coordinator | 4,201,595 | 1,047,435 |
| Fleet Subnet Root | 9,963,759 | 2,314,066 |
| Wasm Store | 845,038 | 843,765 |
| database A | 3,636,636 | 869,142 |
| Project Hub | 4,422,476 | 1,041,051 |
| Project Instance | 4,186,536 | 995,816 |
| Project Ledger | 3,639,920 | 869,307 |
| Project Machine | 3,639,920 | 869,302 |

Database B and C are separate qualified packages and were installed throughout
the journey. Their behavior and dependency shape are intentionally identical
to the representative database A fixture, so only database A is retained in
the footprint table.

## Fresh validation

The qualifying command was:

```text
cargo test --locked -p canic-testing-internal pic::fleet_registry::baseline::tests::toko_topology_qualifies_scale_out_descendants_packing_and_fleet_isolation --lib -- --exact --nocapture
```

Result: `1 passed; 0 failed; 24 filtered out`, completed in 106.49 seconds.

## Scope limits

- PocketIC proves routing across three distinct synthetic application Subnets;
  it is not live-mainnet placement or failure-domain evidence.
- `Authority` and `Replica` remain protected topology purposes only. No data
  synchronization, consistency, promotion or failover is inferred.
- `PoolMember` remains configured membership only. No health, load-balancer
  eligibility or readiness is inferred.
- The 10,000 value is a per-parent Project Hub spawn ceiling, not a Subnet
  count. No ten-thousand-Subnet claim is made.
- Q5 still owns whole-program hard-cut, sediment, generated-surface,
  stable-memory and responsibility/size closeout.
