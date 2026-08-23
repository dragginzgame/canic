# Fleet Installation Input

`canic install` accepts App topology and concrete Fleet deployment policy from
separate authorities:

- `apps/<app>/canic.toml` declares reusable roles and Component Specs; and
- `--fleet-input <path>` names an operator-owned TOML document containing
concrete Subnet placement, root-local admissions and limits, and separate
cycle-funded initial balances for each host-created infrastructure Canister.

The Fleet input is required. A relative path is resolved from the ICP project
root. It is read as a bounded regular no-follow file, rejects unknown fields,
and currently uses `schema_version = 1`.

## Document Shape

```toml
schema_version = 1
funding_profile = "single_subnet"
operator = "<operator-principal>"

[coordinator.subnet]
kind = "explicit"
subnet = "<subnet-principal>"

[coordinator.creation_funding]
kind = "cycles"
cycles = "100T"

[coordinator.root_funding]
minimum_reserve_cycles = "30T"
window_secs = 7776000
maximum_cycles = "30T"
maximum_automatic_grants = 4
maximum_automatic_cycles = "120T"

[[fleet_subnet_roots]]
placement_subnet = "<subnet-principal>"

[fleet_subnet_roots.component_admissions]
project_hub = 10
database = 3

# Exact initial deployment ordinals assigned to this root's Subnet.
[fleet_subnet_roots.component_group_placements]
project_cells = [0, 1]

[fleet_subnet_roots.canister_pool]
minimum_size = 3
maximum_size = 10
canister_cycles = "5T"

[fleet_subnet_roots.root_funding]
request_threshold = "10T"
target_balance = "30T"
cooldown_secs = 2592000
window_secs = 7776000
maximum_cycles = "30T"
maximum_automatic_grants = 4
maximum_automatic_cycles = "120T"

[fleet_subnet_roots.limits]
maximum_component_instances = 13
maximum_registry_bytes = 16777216
maximum_wasm_store_bytes = 40000000
maximum_group_placements = 16

[fleet_subnet_roots.limits.cycles_funding]
window_secs = 3600
maximum_cycles = "10T"

[fleet_subnet_roots.root_creation_funding]
kind = "cycles"
cycles = "30T"

[fleet_subnet_roots.wasm_store_creation_funding]
kind = "cycles"
cycles = "10T"
```

`operator` is the only authored operator-funding identity. Canic requires it
to be one canonical non-anonymous Principal. `canic deploy plan` and
`canic install` require the active ICP CLI identity to match it, derive that
identity's Cycles Ledger account, and query its live cycles balance.
The former `funding_account`, `source`, observation timestamps and `balance`
input fields are rejected; they were observations, not operator policy.

`funding_profile` remains explicit because cross-Subnet topology alone cannot
choose between the bounded `preview_multi_subnet` staging authority and the
professional `multi_subnet` authority. `single_subnet` must resolve wholly on
the Coordinator Subnet. Profile validation applies the current node-count-
scaled minimums, while the concrete creation, reserve, window and lifetime
caps remain explicit protected policy.

Repeat `[[fleet_subnet_roots]]` once for each physical Subnet occupied by the
Fleet. Each root admission key is an exact `ComponentSpecId`; its positive
value is that root's immutable top-level Component-instance ceiling. Every
configured Spec must be admitted somewhere, and the sum of its root-local
ceilings cannot exceed the Spec's Fleet-wide `maximum_instances`.

An admission value is not an initial deployment count. For every checked-in
Component Group deployment with nonzero `initial_placements`, the complete
Fleet input must assign each ordinal from zero through
`initial_placements - 1` exactly once under
`component_group_placements`. The table key is the exact
`ComponentGroupDeploymentId`; its array assigns those placement ordinals to
this root's explicit `placement_subnet`. Root rows and deployment keys are
canonicalized, ordinal arrays must already be strictly increasing, and the
resulting assignments become immutable install-plan authority before any
Canister effect.

Assignments must satisfy the deployment and Fleet-service density/spread
policies, root admissions, `maximum_component_instances` and
`maximum_group_placements`. They never infer a root from labels, apparent
capacity or iteration order. A deployment with `initial_placements = 0` has no
initial assignment; ordinary post-install Component creation remains a
separate lifecycle.

Fresh installation accepts each root's complete initial Component batch
before claiming its members. That batch must therefore fit the Ready-asset
target established by immutable input: the greater of the pool
`minimum_size` and imported asset count. This bounds the one atomic initial
transaction; it is not a lifetime workload or physical-Subnet Canister
ceiling.

`maximum_group_placements` is the immutable aggregate ceiling for accepted or
committed Component Group placements on that root. Ordinary Components do not
consume it. Set it to zero when the root must remain ineligible for grouped
placement; every grouped Component still consumes the existing Component,
Registry, Store, prepaid-asset and cycles limits independently.

Every root has one required prepaid empty-Canister policy. `minimum_size` is
the automatic Ready-asset target, `maximum_size` bounds standby and
operator-imported pool assets, and `canister_cycles` is the minimum retained
balance required before an asset becomes Ready. On IC mainnet, maintenance
uses the canonical Cycles Ledger to create one exact-Subnet, root-controlled
asset at a time when the Ready count is low. The root's default Cycles Ledger
account must already contain the creation amount plus the ledger fee; refill
does not infer or move funds into that account. An insufficient-funds result
reports the available balance and frozen creation amount without pretending
the separately charged fee is part of the request. `imports` accepts exact
existing Canister principals that the root will take under sole control,
uninstall and validate before use. Imported
principals must be non-reserved, unique within the root and unique across the
complete Fleet input. On IC mainnet, every import must also have trusted
routing-catalog evidence for the exact Subnet occupied by that root; missing
or different placement fails before installation. Recycled workload
Canisters remain tracked even if their return temporarily exceeds
`maximum_size`.

The same placement rule applies to controller-requested runtime imports on IC:
the root requeries the NNS Registry before taking control. Non-mainnet imports
remain explicit operator authority because those networks do not provide the
trusted IC routing catalog.

All root limits and funding amounts are explicit. Each root row funds its Fleet
Subnet Root and sibling Wasm Store independently; the former ambiguous
`fleet_subnet_roots.creation_funding` field is rejected. Cycle amounts accept
exact integers or Canic cycle suffixes such as `"2T"`. Infrastructure
creation is cycle-only. `kind = "icp"` rejects before profile admission
because an ICP amount cannot satisfy a cycle-denominated profile floor without
separate live conversion-rate, fee and margin authority. That deferred
automation is recorded as the
[operator funding conversion authority idea](../design/ideas/operator-funding-conversion-authority/design.md).

Zero funding, zero limits, invalid pool ranges, over-limit, duplicate or
wrong-Subnet pool imports, unknown Specs, duplicate Subnets, incomplete
admission or initial-placement coverage, or a root that cannot fit one
admitted Component tree fails before Canister creation.

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
kind = "profile"
profile = "fiduciary"
```

`profile` resolves an exact unique eligible application Subnet by its trusted
catalog label. The former Fiduciary-backed `recommended` selector is rejected;
Fiduciary placement must be explicit and set the adjacent
`acknowledge_fiduciary_cost = true`. Explicit IC mainnet Subnets must also
exist in the trusted catalog.

IC mainnet application Subnets require `cycles` creation funding. Restricted
system Subnets cannot host fresh-Fleet infrastructure while creation funding is
cycle-only. Cloud Engine and unknown Subnets are also not eligible. Non-mainnet
networks require explicit Subnets and cycles funding; Canic does not invent an
ICP amount, silently convert value, or fall back to another Subnet.

## Funding-Profile Scaffold

`canic --environment ic scaffold fleet-input <profile>
--coordinator-subnet <subnet> --root-subnet <subnet>` is the primary authoring
surface. Repeat `--root-subnet` once per Root. It resolves exact node counts
from Canic's validated Registry catalog without reading a funded identity or
ledger balance. By default the command is cache-only;
`--refresh-catalog` may refresh missing or invalid evidence.

For fully offline authoring, replace the Subnet-ID options with
`--coordinator-node-count <count>` and one `--root-node-count <count>` per
Root. Those explicit counts remain operator evidence and must come from the
current Registry; the mode exists so a missing local catalog never forces
authoring through a funded live identity.

Both modes display every scaling, rounding, cap, subtotal, creation-fee and
maximum-debit formula, then emits exact integer TOML for all protected funding
fields. It does not read an ICP identity, query a balance, write a file or claim
install admission. Merge the fragment with exact operator, Subnet, admission,
pool and limit authority, then run `canic deploy plan` on the complete input.
The live plan remains the sole no-effect install-admission boundary.

## Durable Result

After the exact application artifact union is finalized, Canic resolves the
document and immutably publishes one `FleetInstallPlan` plus the exact
release-set manifest for every planned root. The durable plan contains exact
Subnets and positive funding—not unresolved selectors—and is published before
any Canister creation effect.

## Installation Boundary

Before allocating a release build or preparing Wasm, installation may acquire
missing or invalid IC-mainnet catalog evidence through public NNS Registry
queries and compiles its authoritative decision from the validated snapshot's
stable Registry version, catalog digest, assurance and source endpoints. This
is the same snapshot authority compiled by `canic deploy plan
--refresh-catalog`, so cache path, collection time, cache disposition and the
refresh request do not change the plan digest. Those transient facts remain
available as report acquisition provenance.

Planning and installation also resolve the effective ICP CLI identity at this
boundary. They reject an anonymous or unusable identity, require the observed
Principal to equal top-level `operator`, derive its Cycles Ledger account and
require its live balance to cover creation amounts plus one exact Cycles Ledger
fee for every host-created Coordinator, Root and Store. Transient balance,
source and timestamp evidence is rendered in the plan but excluded from the
canonical digest; the operator Principal and derived account remain digest
authority. Installation observes the balance again immediately before effects
and rejects equality without fees as well as any newly insufficient account.
For an encrypted identity in non-interactive execution, set
`CANIC_ICP_IDENTITY_PASSWORD_FILE` to an absolute operator-owned password file.

The installer uses that immutable authority to create, install, and
independently verify the Coordinator, every planned Fleet Subnet Root, one
exact topology-admitted local Store per root, and every Registry row. It then
commits the complete active Registry, installs each root's exact Mirror and
Registry-derived Directories, provisions the configured initial Components
through root-local journals, activates their runtime and Registry membership,
seals initial inventory, activates every selected root, and publishes the
terminal Coordinator-anchored Fleet catalog.

Repeating an exact input is same-release journal recovery. Changing placement,
admissions, limits, funding, topology, or release-build authority after
publication is a conflict; the installer does not fall back to a single-root
path. Every pre-1.0 release transition starts from empty Fleet state.

Successful terminal publication is Coordinator-anchored.
`canic info subnets <fleet> [--json]` resolves that terminal authority and
reports exact Fleet-owned Canister counts by occupied physical Subnet. It
fails closed before the terminal catalog boundary or when current
Coordinator/root evidence is incomplete. Root rows expose
pooled Canisters separately and include them in their exact totals.

Each root's controller-only `canic_pool_list` query supplies the detailed,
paginated asset ledger plus any exact pending Cycles Ledger creation. The
controller can request an explicit retry only for a known terminal ledger/CMC
rejection; an uncertain request that outlives ledger deduplication is
permanently fenced. During root draining, `canic_pool_admin` can hand ready or
failed assets to explicit replacement authority one at a time; final root
inventory remains fenced until the tracked pool and refill journal are empty.
