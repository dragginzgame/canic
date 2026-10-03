# Fleet Orchestration

<p align="center">
  <img src="../../../assets/1400x600/canic-deployment.jpg" alt="The Canic mechanic tending a connected group of canisters" width="700" />
</p>

A **Fleet** is one deployed copy of a Canic application on one IC network. Fleet
orchestration is the process of creating its canisters, installing the intended
code, supplying cycles, and keeping the deployed result aligned with an
operator-approved plan.

Canic first shows the operator what it intends to do. Only an explicitly
approved `canic fleet ensure` plan may make those changes. If an operation is
interrupted or its result is unclear, Canic records enough information to check
what happened before trying again.

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
an idempotent treasury drain before Canic may replace or delete it. Historical
install and recovery state is not a current authority.

## Start Here

- [Installing Canic](../../../INSTALLING.md)
- [Fleet ensure](../operations/fleet-ensure.md)
- [Build artifact architecture](../../architecture/build-artifacts.md)
- [Host library guide](../../../crates/canic-host/README.md)
- [Current implementation status](../../status/current.md)
