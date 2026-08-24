# Canic 0.109 B1 Authority And Baseline

Date: 2026-08-23

## 1. Exact source boundary

| Item | Captured identity |
| --- | --- |
| Canic predecessor | annotated `v0.108.2`, commit `dafc455339df92acb304072d3ec2b98c4069747d` |
| Canic branch and tree | `main`, clean before B1 documentation began |
| Release posture | 0.109 is reinstall-only; no 0.108 whitelist record is decoded or migrated |
| Toko Miner source | read-only `main` at `e61c15b54afd04744611724408dcceeae65dab7d`, described as `v0.1.6-dirty` |
| Toko working tree | ten pre-existing tracked changes; none in the four traced source/config files or the retained run-02 report |
| Rust used for sizing | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |

The Toko checkout was inspected only as a downstream acceptance source. No
Toko file, build output, canister, network, identity or deployment state was
changed.

## 2. Current Toko direct-ingress trace

The accepted design premise that Toko's IcyDB App is standalone is obsolete.
Current production source declares both `core` and `app` as ordinary Canic
Components. The App uses `canic::start!` with IcyDB's synchronous lifecycle
participant; only its explicit local-development feature uses
`canic::start_local!`.

The exact relevant source identities are:

| File | SHA-256 | Current fact |
| --- | --- | --- |
| `apps/toko_miner/canic.toml` | `7a0459f2ff0067c5fffa972a91fa2dadad746266006b429890c798fb33f3d9b4` | seven-member App-wide whitelist seed; singleton managed Core and App Components |
| `apps/toko_miner/core/src/lib.rs` | `f4b6da983dff3276f5e18d367edf79b2616d688d248322156555d78a576068d5` | browser login uses `caller::is_whitelisted()`; the manifest query is public |
| `apps/toko_miner/app/src/lib.rs` | `8b6aacc77d19c0e497bfdc629dbf9b863b3aff596ed9ca07a8ec0232afdac5e9` | Canic owns lifecycle and restores IcyDB synchronously |
| `apps/toko_miner/app/src/robot/mod.rs` | `3d71e637e1f675962cf5de9b264a011f53b95ad30a9d8b54b70900ed167af5ac` | `enroll_my_user` accepts every non-anonymous direct caller before caller-derived User/Robot ownership |
| run-02 demo-readiness report | `5dec5ebedf296f0c38ff90f106c96a1ba78e2036d9575e2ff8e042cdd4a39866` | `DEMO-003` requires inherited admission on the managed App before mainnet publication |

The security gap is therefore not lifecycle integration. It is that the App's
direct IcyDB/IC-CDK endpoints do not evaluate the Canic-managed local whitelist
record that already exists in their artifact. A second passive storage
protocol or standalone-consumer lifecycle would duplicate ownership and would
not address the actual current source. The required seam is one synchronous
framework adapter over the same Canic-owned local projection used by Canic
endpoint access expressions.

No configured Principal value is copied into this evidence. Counts and file
digests are sufficient for traceability.

## 3. Released 0.108.2 whitelist inventory

The current authority is one independent record in every eligible managed
non-Root canister:

- model, policy, storage, ops, workflow and API owners live under the matching
  `runtime_whitelist` modules in `canic-core`;
- memory ID 61 and stable key `canic.core.runtime.whitelist.v1` hold one
  schema-1 record bounded to 32 KiB;
- the record contains at most 256 sorted unique Principals, revision, digest
  and one retained accepted operation/result;
- status pages contain at most 128 Principals;
- fresh bootstrap seeds from `[app.whitelist]`, while same-release restore
  validates memory ID 61 without reseeding;
- controller or exact stable Root may administer the local record;
- `CanisterCommand::RuntimeWhitelist` and
  `CanisterStatusRequest::RuntimeWhitelist` expose the managed surface; and
- `caller::is_whitelisted()` performs the only endpoint access evaluation.

The surface is absent from Root, Coordinator, Store and standalone-local role
contracts. It has no host or CLI Fleet mutation owner. Twelve checked-in
`canic.toml` files contain `[app.whitelist]`; only the delegation issuer test
fixture contains a maintained product-like endpoint use in this repository.

The 0.109 hard cut removes, rather than aliases:

1. `AppConfig::whitelist`, `Whitelist` and `[app.whitelist]`;
2. the compiled whitelist seed and bootstrap renderer;
3. all `RuntimeWhitelist*` DTO/model/policy/ops/workflow/API/storage names;
4. `RuntimeWhitelist` role command/status/replay-manifest variants;
5. `caller::is_whitelisted()`, `CallerIsWhitelisted` and
   `WhitelistRequired`;
6. memory-ID-61's old key and decoder; and
7. fixtures, generated expectations and active documentation for the removed
   surface.

0.109 reuses memory ID 61 for the one current Fleet admission projection under
a new schema-1 key. Reinstall-only release policy means there is no old-record
reader, importer, migration or compatibility alias.

## 4. Frozen authority and participant ownership

The maintained flow is:

```text
protected Fleet input
  -> host compiles generation-one canonical policy and Root projections
  -> Coordinator owns the only mutable policy and Fleet transition journal
  -> each exact registered Root owns one subtree distribution journal
  -> each explicitly enrolled managed non-Root target owns one local projection and local receipt
  -> endpoint reads the local projection and observed transport caller
```

No Root chooses policy, adds membership, widens a selector or becomes a second
Fleet journal. A Root accepts only its exact installed Coordinator, derives
target projections from the retained canonical policy plus its protected
Component Registry, and reports one aggregate phase receipt. The Coordinator
tracks Root convergence, not a duplicate per-Component journal.

Infrastructure, service, controller, Root and application-resource authority
remain separate. Admission returns only the observed caller; Toko must still
resolve `Principal -> UserPrincipal -> UserId -> Robot` before domain work.

The participant set contains managed non-Root canisters whose exact
`[roles.<role>]` declaration sets `fleet_admission = true`. That declaration
selects the `FleetAdmissionProjection` role capability and is the sole
enrollment authority; omission selects no projection state or generated
admission surface. Root canisters are bounded
distribution targets but are not user-admission participants. New Component
creation and retirement are fenced against an active transition. A new target
receives the converged projection before its protected ingress opens.

## 5. Frozen input, selectors and public names

Generation one moves to the strict Fleet-input document:

```toml
[admission]
principals = ["<principal>"]

[[admission.rules]]
selector = { kind = "component_spec", component_spec = "core" }
principals = ["<narrower-principal>"]
```

`admission.principals` is the required Fleet set. `admission.rules` is sorted,
unique and optional. Generation-one rules may select a `component_spec` or
`fleet_subnet_root`. The latter is authored with the Root's unique
`placement_subnet` from the same exact Fleet input but semantically selects
only that protected Root binding and descendants, never arbitrary canisters on
the physical Subnet.

The runtime selector is frozen as:

```text
FleetAdmissionSelector::Fleet
FleetAdmissionSelector::ComponentSpec(ComponentSpecId)
FleetAdmissionSelector::ComponentInstance(ComponentInstanceId)
FleetAdmissionSelector::FleetSubnetRoot(SubnetId)
```

An exact Component-instance selector is admitted only after that durable
instance ID exists. It is not accepted in generation-one fresh input. Every
narrower Principal must exist in the Fleet set; matching rules intersect.
Unknown selectors, duplicate selectors, duplicate Principals, anonymous
Principals, noncanonical order and widening all reject before retention.

The exact maintained role additions are:

```text
CoordinatorCommand::MutateAdmission(FleetAdmissionMutationRequest)
CoordinatorStatusRequest::Admission(FleetAdmissionStatusRequest)
CoordinatorOperationStatusResponse::Admission(FleetAdmissionOperationStatusResponse)

RootCommand::PrepareFleetAdmission(FleetAdmissionPrepareRootRequest)
RootCommand::ActivateFleetAdmission(FleetAdmissionActivateRootRequest)
RootCommand::OpenFleetAdmission(FleetAdmissionOpenRootRequest)
RootStatusRequest::Admission(PageRequest)

CanisterCommand::PrepareFleetAdmission(FleetAdmissionPrepareTargetRequest)
CanisterCommand::ActivateFleetAdmission(FleetAdmissionActivateTargetRequest)
CanisterCommand::OpenFleetAdmission(FleetAdmissionOpenTargetRequest)
CanisterStatusRequest::Admission(PageRequest)
```

Only current Coordinator controllers may start `MutateAdmission`. Root phase
commands accept only the exact installed Coordinator. Managed-target phase
commands accept only the exact stable Root binding. Every endpoint checks
caller authority before policy, operation or projection state.

Operator spelling is frozen as the ASCII-ordered top-level family:

```text
canic admission apply <fleet> <plan-file>
canic admission plan <fleet> --add|--remove <principal> <selector> --out <path>
canic admission status <fleet>
```

The plan is read-only and binds installed Fleet/Coordinator, Registry version,
participant set, predecessor generation/digest, exact mutation, successor
digest and operation ID. Apply accepts only that exact plan. The selector is
one of `--fleet`, `--component-spec <id>`, `--component-instance <id>` or
`--fleet-subnet-root <subnet-id>`.

Managed Canic endpoints use `caller::is_fleet_admitted()`. A composed framework
endpoint uses synchronous `canic::fleet_admission::require_caller()`, which
reads `msg_caller()` and the same Canic-owned projection and returns the caller
or a typed fail-closed error before application work. It owns no storage,
lifecycle hook, timer, remote lookup or caller-supplied Principal.

## 6. Frozen bounds and sizing evidence

| Bound | Value |
| --- | ---: |
| Fleet Principals | 256 |
| narrower rules | 32 |
| total Principal references across narrower rules | 128 |
| registered Roots/distribution targets | 4,096 |
| admission participants per Fleet | 4,096 |
| admission participants per Root | 4,096, additionally constrained by the Fleet total |
| participant progress rows per status page | 32 |
| Principal inspection page | 128 |
| retained progress rows in Coordinator current-plus-last state | 8,192 |
| retained progress rows in Root current-plus-last state | 8,192 |
| Coordinator admission cell | memory ID 64, 8 MiB |
| Root admission cell | memory ID 65, 8 MiB |
| managed projection cell | reused memory ID 61, 32 KiB |

The checked-in synthetic fixture models maximum-length IDs and Principals,
complete current and prepared policies, maximum current and last-result
progress, and active plus prepared participant projections. Reproduce it
without adding another workspace package:

```text
tmp=$(mktemp -d)
mkdir -p "$tmp/src"
cp docs/audits/working/0.109-fleet-wide-ingress-admission/bounds-fixture/manifest.toml "$tmp/Cargo.toml"
cp docs/audits/working/0.109-fleet-wide-ingress-admission/bounds-fixture/src/main.rs "$tmp/src/main.rs"
cargo run --offline --manifest-path "$tmp/Cargo.toml"
```

Captured output:

```text
policy_candid_bytes=14295
policy_cbor_bytes=16234
root_prepare_command_candid_bytes=14394
participant_prepare_command_candid_bytes=8408
participant_status_page_candid_bytes=7623
coordinator_state_cbor_bytes=2973600
root_state_cbor_bytes=3981231
participant_state_cbor_bytes=17130
```

The Root prepare fixture leaves 1,990 bytes beneath the existing 16 KiB role
command limit before the production outer variant is frozen. B2-B4 must encode
the final outer envelopes and reject first excess; an outer command that does
not fit must reduce a bound rather than introduce an unreviewed paging
protocol. The stable fixtures remain below half of their Coordinator/Root
allocations and the participant fixture remains below 32 KiB.

Membership remains a sorted vector. A maximum local lookup performs at most
eight Principal comparisons. Restore decodes at most one 32 KiB record,
validates at most one active and one prepared 256-member projection and checks
their complete digests and bindings synchronously. B4 owns the executable
PocketIC instruction ceiling for those already-frozen structural limits.

## 7. B1 result and review boundary

| Required output | Result |
| --- | --- |
| released whitelist/config/Candid inventory | Complete |
| exact Toko direct-ingress trace | Complete; current App is managed, not standalone |
| sole authority and Root/participant ownership | Frozen |
| selector and generation-one contract | Frozen |
| hard-cut list | Frozen |
| command/status/CLI names | Frozen |
| counts, payloads, stable allocations and restore work | Frozen with reproducible conservative sizing fixture |
| runtime/config/Candid mutation | Not begun in B1 |

The smallest correct direction is the managed-only contract above. Keeping the
old standalone-consumer protocol would add a second persistence/lifecycle
integration surface without a current acceptance source. B2 may begin only
after the maintainer reviews and accepts this B1 correction.
