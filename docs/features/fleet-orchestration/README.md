# Fleet Orchestration

A Fleet is one live desired-state instance on one network. Canic qualifies its
artifacts, observes exact configured canisters, and reconciles only the effects
in one reviewed `canic fleet ensure` plan.

## What It Provides

- explicit network trust enrollment and canonical network identity
- operator-owned desired Fleet state separate from App configuration
- exact create/reuse/reinstall/replace/delete dispositions
- intent-before-effect creation, funding, transfer and management operations
- bounded fees, funding, observation/update burn and cycle conservation
- effect-free immediate replay after convergence

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
