# Fleet Installation Input

`canic install` accepts App topology and concrete Fleet deployment policy from
separate authorities:

- `apps/<app>/canic.toml` declares reusable roles and Component Specs; and
- `--fleet-input <path>` names an operator-owned TOML document containing
  concrete Subnet placement, root-local admissions and limits, and initial
  creation funding.

The Fleet input is required. A relative path is resolved from the ICP project
root. It is read as a bounded regular no-follow file, rejects unknown fields,
and currently uses `schema_version = 1`.

## Document Shape

```toml
schema_version = 1

[coordinator.subnet]
kind = "explicit"
subnet = "<coordinator-subnet-principal>"

[coordinator.creation_funding]
kind = "cycles"
cycles = "2T"

[[fleet_subnet_roots]]
placement_subnet = "<workload-subnet-principal>"

[fleet_subnet_roots.component_admissions]
project_hub = 10
database = 3

[fleet_subnet_roots.limits]
maximum_component_instances = 13
maximum_managed_canisters = 20000
maximum_registry_bytes = 16777216
maximum_wasm_store_bytes = 40000000

[fleet_subnet_roots.limits.cycles_funding]
window_secs = 3600
maximum_cycles = "100T"

[fleet_subnet_roots.creation_funding]
kind = "cycles"
cycles = "2T"
```

Repeat `[[fleet_subnet_roots]]` once for each physical Subnet occupied by the
Fleet. Each root admission key is an exact `ComponentSpecId`; its positive
value is that root's immutable top-level Component-instance ceiling. Every
configured Spec must be admitted somewhere, and the sum of its root-local
ceilings cannot exceed the Spec's Fleet-wide `maximum_instances`.

All root limits and funding amounts are explicit. Cycle amounts accept exact
integers or Canic cycle suffixes such as `"2T"`. ICP creation funding uses:

```toml
[coordinator.creation_funding]
kind = "icp"
e8s = 100000000
```

Zero funding, zero limits, unknown Specs, duplicate Subnets, incomplete
admission coverage, or a root that cannot fit one admitted Component tree
fails before Canister creation.

## Coordinator Subnet Selection

An explicit selector works on any enrolled network:

```toml
[coordinator.subnet]
kind = "explicit"
subnet = "<subnet-principal>"
```

The public IC additionally supports:

```toml
[coordinator.subnet]
kind = "recommended"
```

and:

```toml
[coordinator.subnet]
kind = "profile"
profile = "fiduciary"
```

`recommended` resolves to the unique trusted public-IC Fiduciary application
Subnet. `profile` resolves an exact unique eligible application Subnet by its
trusted catalog label. Explicit public-IC Subnets must also exist in the
trusted catalog.

Public-IC application Subnets require `cycles` creation funding. Restricted
system Subnets require `icp` funding. Cloud Engine and unknown Subnets are not
eligible for Fleet infrastructure. Non-public networks currently require
explicit Subnets and cycles funding; Canic does not invent an ICP amount,
silently change the funding method, or fall back to another Subnet.

## Durable Result

After the exact application artifact union is finalized, Canic resolves the
document and immutably publishes one `FleetInstallPlan` plus the exact
release-set manifest for every planned root. The durable plan contains exact
Subnets and positive funding—not unresolved selectors—and is published before
any Canister creation effect.

## Current Implementation Boundary

The in-progress 0.100 installer uses that immutable authority to create,
install, and independently verify the Coordinator, every planned Fleet Subnet
Root, one exact topology-admitted local Store per root, and every exact
Registry `Joining` row. It currently stops after all roots reach
`RegistrySyncVerified`, with an exact private snapshot candidate and
Coordinator acknowledgement at every root, before Registry `Active`, final
mirror/Directory activation, Component creation, or terminal Fleet-catalog
publication.

Repeating an exact input is same-release journal recovery. Changing placement,
admissions, limits, funding, topology, or release-build authority after
publication is a conflict; the installer does not fall back to a single-root
path. Every pre-1.0 release transition starts from empty Fleet state.

At 0.100 closeout, successful terminal publication will be
Coordinator-anchored. The planned `canic info subnets <fleet> [--json]`
command will resolve that terminal authority and report exact Fleet-owned
Canister counts by occupied physical Subnet; it is not available at the
current Registry-join boundary.
