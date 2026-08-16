# Skynet Fleet Demo

Skynet is a Terminator-themed demonstration of Canic's multi-Subnet Component
model. It is an application, not a special runtime mode: the same checked-in
Canic configuration defines one service Authority, seven initial Replicas,
dynamic T-800 workers, stateful memory-cell shards, limits, diagnostics,
metrics and the authority boundaries shown by the consoles.

## Topology

```text
Canic Coordinator
        |
        +-- Fleet Subnet Root + Store -- Skynet Authority -- T-800
        |                                      |           -- memory cell
        +-- Fleet Subnet Root + Store -- Skynet Replica  -- T-800
        |                                      |           -- memory cell
        +-- ... six more initial Replica roots ...
        +-- ... up to 24 prepared scale-out roots ...
```

The initial service occupies eight distinct physical Subnets. An operator may
prepare between 8 and 32 distinct roots, and the checked-in deployment permits
scale-out to one Skynet service member on every prepared root, up to 32 members.
Each service node starts with one T-800 scaling worker and one memory-cell
shard; both pools may grow independently to four children per node.

Every application Component and Fleet Subnet Root currently serves the themed
console. Scheduled Canic 0.106 extends one application-neutral observatory
renderer and data-only T2 profile to the canonical Coordinator and Wasm Stores
so every installed Canister serves the experience without pretending
infrastructure is an application role. Empty module-free estate assets remain
visible inventory and cannot serve HTTP.

## What the Consoles Show

Every console is code-native HTML with no external assets. It has a responsive
mobile layout and a machine-readable `/api/status.json` view. Depending on the
role, it displays:

- current Canister identity, role, version, cycles, readiness and bootstrap;
- physical Subnet, parent, Fleet root and Component Spec environment;
- protected deployment purpose, placement, labels, limits and authority;
- live Fleet Directory roots, service members and Registry revision;
- local scaling workers or shards and navigable parent/child links;
- all enabled Canic metric tiers and representative Candid endpoint
  highlights; and
- Canic capabilities such as Authority guards, scaling, sharding, ICRC-21,
  memory accounting and Fleet Directory convergence.

The global map is derived from the node's protected
`ComponentRuntimeApi::status()` view. It is not a hard-coded mock topology.
Controller-only values are not made public; the console lists protected
operations so the boundary itself is demonstrable. The generated role `.did`
artifact remains the exhaustive parameter and result contract.

## Generate the Fleet Input

The application configuration fixes eight initial placements, while concrete
Subnet principals remain operator-owned installation input. Generate that
input with one Coordinator Subnet followed by 8–32 distinct workload Subnets:

```sh
scripts/dev/skynet-fleet-input.sh \
  <coordinator-subnet> \
  <workload-subnet-1> <workload-subnet-2> ... <workload-subnet-8> \
  > /tmp/skynet-fleet.toml
```

The first workload Subnet receives the Authority and the next seven receive
Replicas. Additional supplied roots are admitted and pre-provisioned for later
scale-out. The helper only writes TOML to standard output; it never creates a
Canister or transfers cycles.

Review the generated funding and then use the normal Canic workflow from the
repository root:

```sh
canic build skynet root --profile release
canic build skynet skynet_node --profile release
canic build skynet t800 --profile release
canic build skynet memory_cell --profile release
canic install skynet skynet --fleet-input /tmp/skynet-fleet.toml --profile release
canic info list skynet
```

Open an application Canister as `https://<canister-id>.raw.icp0.io/`. The
machine view is `https://<canister-id>.raw.icp0.io/api/status.json`.

## Cost and Safety Boundary

This is deliberately not a low-cost playground topology. Installation creates
one Coordinator plus a root, Store and prepaid pool on every supplied workload
Subnet. The eight initial service nodes also create their configured T-800 and
memory-cell children, and pool maintenance can fund replacement standby
Canisters. Exact cost depends on network charging and later scaling.

No deployment is automated by this demo. Inspect the generated file, confirm
the identity and network, and fund the relevant Cycles Ledger accounts before
running installation. Use a disposable non-mainnet environment for functional
experiments unless the multi-Subnet mainnet display is explicitly intended.

Every Skynet Canister is intended to be repurposed eventually for a real
workload such as Toko. Canic 0.101 does not yet qualify grouped removal or
full-Fleet asset reprovisioning, so that is a future design invariant rather
than a current teardown command. Do not treat a label or Wasm replacement as
proof that Skynet state and authority were safely detached.

## Scheduled Design

[Canic 0.107](../../docs/design/0.107-skynet-fleet-observatory/0.107-design.md)
is the accepted post-estate design for an exceptionally informative T2-themed
topology and Fleet overview served by every installed Skynet Fleet Canister.
Implementation remains gated on accepted 0.106 closeout and explicit 0.107 B1
promotion.
