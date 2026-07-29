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

An admission value is not an initial deployment count. Fresh 0.100
installation activates each root with an empty sealed Component inventory;
Components may then be created through the active-root lifecycle. Exact
nonempty initial placement is a separate 0.101 Component Group deployment
authority.

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

IC mainnet additionally supports:

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

`recommended` resolves to the unique trusted IC mainnet Fiduciary application
Subnet. `profile` resolves an exact unique eligible application Subnet by its
trusted catalog label. Explicit IC mainnet Subnets must also exist in the
trusted catalog.

IC mainnet application Subnets require `cycles` creation funding. Restricted
system Subnets require `icp` funding. Cloud Engine and unknown Subnets are not
eligible for Fleet infrastructure. Non-mainnet networks currently require
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
`RegistrySyncVerified` only long enough to atomically commit and independently
verify the complete Coordinator Registry as `Active`, with an exact private
snapshot candidate and Coordinator acknowledgement retained at every root.
It then atomically replaces each private candidate with the exact all-`Active`
Registry Mirror and Registry-derived Fleet Directory and independently
reverifies every result. Every root remains runtime-`Prepared`; Component
creation, runtime activation and terminal Fleet-catalog publication remain
fenced.

Repeating an exact input is same-release journal recovery. Changing placement,
admissions, limits, funding, topology, or release-build authority after
publication is a conflict; the installer does not fall back to a single-root
path. Every pre-1.0 release transition starts from empty Fleet state.

Successful terminal publication is Coordinator-anchored.
`canic info subnets <fleet> [--json]` resolves that terminal authority and
reports exact Fleet-owned Canister counts by occupied physical Subnet. It
fails closed while installation remains before the terminal catalog boundary
or when current Coordinator/root evidence is incomplete.
