# Canic 0.109 B4 Managed-Role Projections

Date: 2026-08-23

## Revision and boundary

| Item | Value |
| --- | --- |
| Published predecessor | annotated `v0.108.2`, commit `dafc455339df92acb304072d3ec2b98c4069747d` |
| Candidate | uncommitted 0.109 B1-B4 working tree on `main`; `v0.108.2-dirty` |
| Release posture | reinstall-only hard cut; no 0.107/0.108 whitelist record is decoded, migrated or adopted |
| Effects | repository source, generated local artifacts, canonical Coordinator Candid and documentation only; no remote, paid or canister effect |

The pre-existing 0.108 changelog compaction in `CHANGELOG.md` and
`docs/changelog/0.108.md` remains concurrent maintainer work and is not B4
evidence.

## Ownership and fresh distribution

The Root remains the only Component-install lifecycle owner. Immediately before
building a Component or child init payload, it validates its exact active Fleet
Registry mirror and exact Active Root entry, reads the Registry-owned admission
policy and compiles the complete effective projection for the new target. The
workflow layer applies the sole intersection policy and delegates deterministic
materialization and hashing to ops; the layering gate rejects any ops-to-policy
dependency.

`CanisterInitPayload` carries that complete optional projection beside the
existing protected deployment and installed authority. An enrolled managed
canister requires it; a non-enrolled role rejects it. The managed canister validates
the exact target, Coordinator/Fleet binding, generation, policy digest,
projection digest and canonical bounded Principal set before retaining it.
Target substitution and projection tampering fail closed.

The projection is a named `FleetAdmissionProjection` role capability.
`[roles.<role>] fleet_admission = true` is the sole declaration that derives
the capability, owns the memory-61 allocation and participates in protocol-
profile hashing and immutable role overview status. Omitted roles, declared
Roots and built-in Coordinator/Store roles do not receive that allocation or
capability. There is no second policy, Root journal or projection provider.

## Local state and activation

Memory ID 61 now stores exactly one `FleetAdmissionProjectionRecord` under
`canic.core.fleet_admission.projection.v1`. The record contains one active
projection, at most one B6-prepared successor and one retained participant
receipt. Its frozen bound is 32 KiB; the exact maximum active-plus-prepared-plus-
receipt fixture passes below that bound.

Fresh installation of an enrolled role retains generation one as `Fenced`. The existing Root-only
`ConfigureRuntime` path opens it only after the existing Fleet activation record
is durably `Active`. Opening is monotonic and exact replay is harmless. Because
the open call runs even when `ConfigureRuntime` returns its retained replay,
response loss between activation and opening converges forward on retry. The
application hook is scheduled only by the existing activation transition.

Same-release restart validates the complete stable record against the restored
managed binding before starting runtime work. Missing, malformed, target-
substituted or digest-invalid state returns unavailable/invariant failure. No
restart, reinstall compatibility path or implicit reseed opens authority.

## Local enforcement and public surface

`caller::is_fleet_admitted()` reads the observed transport caller and the one
local open projection synchronously. It performs no remote lookup, mutation or
timer work. Missing or fenced authority denies. The capability-pruned protected
managed `Admission(PageRequest)` status authenticates controller-or-exact-Root before
reading state and returns bounded membership plus target/generation/digest/phase
evidence without transition identities.

The following 0.107 surfaces are hard-deleted rather than aliased:

- `caller::is_whitelisted()`;
- runtime-whitelist DTO, model, policy, ops, workflow, API and stable store;
- `RuntimeWhitelist` managed command/status and replay-manifest variants; and
- `[app.whitelist]` configuration and bootstrap seed.

The application fixtures, procedural-macro parser/validator, managed role
endpoint bundle and representative generated Candid all use the new exact
spelling. The only remaining old names in executable-source searches are
negative absence assertions.

## Targeted qualification

The final B4 candidate passed:

```text
cargo test --locked -p canic-core fleet_admission --lib
# 22 passed

cargo test --locked -p canic-core role_contract --lib
# 21 passed

cargo test --locked -p canic-core state_contract --lib
# 11 passed

cargo test --locked -p canic-control-plane fleet_admission --lib
# 7 passed

cargo test --locked -p canic-control-plane state_contract --lib
# 4 passed

cargo test --locked -p canic-host fleet_install_plan::tests --lib
# 15 passed

cargo test --locked -p canic-host release_set::tests --lib
# 41 passed

cargo test --locked -p canic --test protocol_surface \
  fleet_admission_projection_candid_uses_the_bounded_managed_role_contract
cargo test --locked -p canic --test protocol_surface \
  role_capability_surfaces_are_pruned_at_the_destination_macro
cargo test --locked -p canic --test protocol_surface \
  fleet_coordinator_canonical_did_parses
cargo test --locked -p canic --test protocol_surface \
  fleet_coordinator_candid_contains_protected_admission_and_funding_protocol_types
cargo test --locked -p canic --test managed_endpoint_gate \
  fleet_admission_projection_is_managed_only_and_authenticates_before_state_access
cargo test --locked -p canic --test endpoint_macro
# all selected tests passed

bash scripts/ci/run-with-test-scratch.sh \
  bash scripts/ci/run-workspace-tests.sh targeted-pocketic \
  pic::lifecycle::tests::managed_projection_fences_then_opens_and_restores
# 1 passed on freshly rebuilt Wasm in 117s; 298,324 kB final RSS,
# 310,044 kB high-water mark and 19 PocketIC threads

make layering-gate
cargo fmt --all -- --check
git diff --check

cargo clippy --locked -p canic-core -p canic-control-plane -p canic \
  -p canic-macros -p canic-host -p canic-cli \
  -p canic-testing-internal -p canic-tests --all-targets -- -D warnings
```

The governed PocketIC case installs two managed targets under distinct exact
Root bindings. Both start fenced; their installed policy digests match while
their target-bound projection digests differ. Each exact Root activates only
its target, the admitted Principal succeeds locally on both, an unlisted
Principal fails, and same-release upgrade restores the first target's exact
open generation and digests.

Representative artifacts were built with:

```text
CARGO_INCREMENTAL=0 cargo run -q --profile fast -p canic-host \
  --example build_artifact --locked -- test debug . . apps/test/canic.toml

CARGO_INCREMENTAL=0 cargo run -q --profile fast -p canic-host \
  --example build_artifact --locked -- root debug . . apps/test/canic.toml

CARGO_INCREMENTAL=0 cargo run -q --profile fast -p canic-host \
  --example build_artifact --locked -- fleet_coordinator debug . . \
  apps/test/canic.toml --refresh-canonical-did
```

The final governed PocketIC run freshly rebuilt the managed test Wasm after the
layering correction; the final Root artifact also rebuilt successfully. The
canonical Coordinator Candid was refreshed and its closed `RoleCapability`
type now includes `FleetAdmissionProjection`.

No full workspace validation, broad PocketIC matrix or native-agent suite was
run. Repository policy reserves the complete gate for explicit maintainer
authorization. The updated native-agent projection journey compiles under the
changed-package all-targets Clippy gate; B5 owns its composed-framework direct-
ingress qualification.

## Result

B4 is ready within its sequenced boundary. Fresh explicitly enrolled managed
targets receive one exact local projection, remain fenced until existing Root
activation completes, enforce the observed caller locally and restore the same
authority after a same-release interruption. Omitted roles receive no
projection state or admission surface. The independent whitelist implementation
and public vocabulary are gone. B5 still owns the synchronous composed-
framework adapter, and B6 still owns runtime policy-mutation convergence; 0.109 is not yet ready
for closeout or publication.
