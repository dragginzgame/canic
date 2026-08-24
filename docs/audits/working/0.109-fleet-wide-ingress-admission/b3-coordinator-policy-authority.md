# Canic 0.109 B3 Coordinator Policy Authority

Date: 2026-08-23

## Revision and boundary

| Item | Value |
| --- | --- |
| Published predecessor | annotated `v0.108.2`, commit `dafc455339df92acb304072d3ec2b98c4069747d` |
| Candidate | uncommitted 0.109 B1-B3 working tree on `main`; `v0.108.2-dirty` |
| Release posture | reinstall-only hard cut; no earlier admission record is decoded, migrated or adopted |
| Effects | source, generated Coordinator Candid and documentation only; no remote, paid or canister effect |

The pre-existing 0.108 changelog compaction in `CHANGELOG.md` and
`docs/changelog/0.108.md` remains concurrent maintainer work and is not B3
evidence.

## Sole authority and retained state

B3 adds one Coordinator-owned stable record containing the installed policy,
at most one current transition and at most one terminal last result. The record
uses memory ID 64, the exact stable key
`canic.control_plane.fleet_admission.v1`, one map entry and the frozen 8 MiB
bound. Memory ID 65 and its exact Root key are reserved for the sequenced Root
distribution owner; B3 does not write that memory.

The first representative generated Coordinator build exposed that IDs 64 and
65 still intersected the prior generic core range beginning at 64. The
correction moves that range to 66, changes the control-plane authority range to
62 through 65 and admits only the two exact admission allocation keys at 64
and 65. Role-contract, memory-policy and generated-artifact checks then pass.
No alias, dynamic exception or second allocation authority was added.

The stored policy is independently validated against the exact Registry Fleet,
Coordinator identity, generation and digest before use. Genesis commit accepts
only an empty store or the exact identical installed record, so an install
retry is idempotent while conflicting authority fails closed.

## Mutation and replay invariants

Only a current Coordinator controller may call `MutateAdmission`; endpoint
authentication precedes workflow and state access. A request binds:

- exact installed Fleet and Coordinator authority;
- expected predecessor generation and digest;
- add or remove action, exact selector and Principal;
- operation identity; and
- the caller-supplied exact successor digest.

The sole pure policy implementation owns canonical add/remove semantics,
intersection, bounds and generation movement. Removing a Fleet Principal also
removes it from narrower rules and may not empty the Fleet. Removing a
narrower Principal creates an explicit restriction from inherited Fleet
authority. Adding it back removes a redundant all-Fleet narrowing rule.
Anonymous, widening, unknown, noncanonical and first-excess authority rejects.

The operation identity is checked across funding rotation, component
provisioning, Root removal and admission mutation. An exact current or terminal
retry returns its retained state; a different request using that identity
rejects; a second effective mutation rejects while one is current. An
idempotent semantic request returns a terminal result without increasing the
generation. An effective request retains one exact successor and stops at
`Planned`.

New effective mutations additionally require a nonempty Registry in which
every registered Root is `Active`. Joining Roots reject before retained
mutation. B3 intentionally performs no participant call, local projection
mutation or paid effect. B6 owns prepare/fence/activate/open convergence and
the participant-set lifecycle fence. Exact Component-instance resolution in
B3 is limited to current published service members under an Active Root; B6
owns the complete managed participant catalog.

## Public and generated surfaces

The maintained surface adds:

```text
CoordinatorCommand::MutateAdmission(FleetAdmissionMutationRequest)
CoordinatorStatusRequest::Admission(FleetAdmissionStatusRequest)
CoordinatorOperationStatusResponse::Admission(FleetAdmissionOperationStatusResponse)
```

Protected status provides a bounded Principal page plus current and last
operation summaries. Exact operation lookup exposes only retained current or
last identity. `MutateAdmission` is classified as replay-protected in the role
command manifest. The canonical generated Coordinator Candid and protocol
surface expectations contain the same types and variants.

## Focused evidence

The following targeted commands passed on the recorded working tree:

```text
cargo test --locked -p canic-core --lib fleet_admission
# 16 passed

cargo test --locked -p canic-control-plane --lib fleet_admission
# 7 passed

cargo test --locked -p canic-core --lib memory::policy::tests
# 6 passed

cargo test --locked -p canic-core --lib role_contract::tests
# 21 passed

cargo test --locked -p canic-control-plane --lib state_contract::tests
# 4 passed

cargo test --locked -p canic-control-plane --lib \
  protected_init_commits_exact_genesis_and_supports_exact_retry
# 1 passed

cargo test --locked -p canic --test protocol_surface \
  fleet_coordinator_protocol_surface_matches_declared_commands
# 4 protocol-surface tests passed in the selected binary

cargo test --locked -p canic-control-plane --lib role_status_authority
# 1 passed

cargo test --locked -p canic-core --lib replay_policy::tests::role_command
# 10 passed

make layering-gate

cargo clippy --locked -p canic-core -p canic-control-plane -p canic \
  -p canic-host -p canic-cli -p canic-testing-internal \
  --all-targets -- -D warnings
```

The canonical Coordinator interface and representative artifact were rebuilt
with:

```text
CARGO_INCREMENTAL=0 cargo run -q --profile fast -p canic-host \
  --example build_artifact --locked -- fleet_coordinator debug . . \
  apps/test/canic.toml --refresh-canonical-did
```

The final closeout also runs `cargo fmt --all -- --check`, the layering gate,
the changed-package warning-denied Clippy command and `git diff --check` after
this evidence is recorded.

No full workspace validation or PocketIC matrix was run. Repository policy
reserves the complete gate for explicit maintainer authorization, and B3 has no
participant or value-transfer effects. B4 and B6 own the applicable runtime
PocketIC evidence.

## Result

B3 is ready within its sequenced batch boundary. It establishes one bounded
Coordinator authority, canonical mutation semantics, exact cross-domain replay
identity and protected generated surfaces without creating a second policy or
participant journal. It is not a release boundary: B4 must replace the old
local whitelist record with the managed projection and B6 must converge the
retained `Planned` successor before 0.109 can ship.
