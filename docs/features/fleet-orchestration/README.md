# Fleet Orchestration

A **Fleet** is one deployed copy of a Canic application on one IC network. Fleet
orchestration is the process of creating its canisters, installing the intended
code, supplying cycles, and keeping the deployed result aligned with an
operator-approved plan.

Canic first shows the operator what it intends to do. Ordinary deployment
convergence requires an explicitly approved `canic fleet ensure` plan;
bootstrap and capacity import have their own reviewed operations. If an operation is
interrupted or its result is unclear, Canic records enough information to check
what happened before trying again.

<p align="center">
  <a href="../../../assets/canic-ic-fleet.jpg">
    <img src="../../../assets/canic-ic-fleet.jpg" alt="A Canic Fleet with one Coordinator and a Root, Wasm Store, and application Components on each occupied IC Subnet" width="650" />
  </a>
</p>

The CLI and local files retain operator authority. The Coordinator plans for the
whole Fleet, while each Root performs approved effects for Components on its own
Subnet. Application Components do not receive local files or operator credentials.

## What It Provides

- explicit enrollment of each trusted network
- a deployment description, owned by the operator and kept separate from App
  source configuration
- exact decisions to create, reuse, reinstall, replace, or delete each canister
- intent-before-effect creation, funding, transfer and management operations
- bounded fees, funding, observation/update burn and cycle conservation
- a repeat run that makes no changes once the Fleet matches the approved plan

The host current-generation journal owns sequencing. Ledger and configured
drain effects additionally retain exact replay identities.

## Boundary

Application canisters never receive filesystem, repository, identity-key, or
operator configuration authority. A material canister must explicitly expose
an idempotent treasury drain before Canic may physically replace or delete it.
ID-preserving clean reinstall retains native cycles on the selected IDs while
clearing application and framework state. Historical install and recovery state
is not a current authority.

## Continue From Here

- [Installing Canic](../../../INSTALLING.md)
- [Fleet ensure](../operations/fleet-ensure.md)
- [Build artifact architecture](../../architecture/build-artifacts.md)
- [Host library guide](../../../crates/canic-host/README.md)
- [Current implementation status](../../status/current.md)
- [Choose another feature](../README.md)
- [Browse all documentation](../../README.md)
