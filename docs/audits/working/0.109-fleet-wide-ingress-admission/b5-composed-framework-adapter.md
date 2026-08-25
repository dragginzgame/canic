# Canic 0.109 B5 Composed-Framework Adapter

Date: 2026-08-23

## Revision and boundary

| Item | Value |
| --- | --- |
| Published predecessor | annotated `v0.108.2`, commit `dafc455339df92acb304072d3ec2b98c4069747d` |
| Candidate | uncommitted 0.109 B1-B5 working tree on `main`; `v0.108.2-dirty` |
| Release posture | reinstall-only hard cut; no earlier whitelist record or adapter is decoded, migrated or aliased |
| Effects | repository source, local build artifacts and local PocketIC only; no remote, paid, canister, Ledger or network effect |

The pre-existing 0.108 changelog compaction in `CHANGELOG.md` and
`docs/changelog/0.108.md` remains concurrent maintainer work and is not B5
evidence.

## One synchronous guard

The maintained public surface is exactly:

```rust
canic::fleet_admission::require_caller()
    -> Result<candid::Principal, canic::access::AccessError>
```

The facade acquires `msg_caller()` itself and immediately delegates to the
same synchronous projection-membership decision used by
`caller::is_fleet_admitted()`. The shared decision returns only that observed
Principal. A caller cannot supply a Principal, Fleet, target, generation or
digest.

The facade contains no `await`, allocation of stable state, mutation, timer,
lifecycle registration, log, cleanup, spawn or remote call. Canic's B4
projection workflow and memory-61 record remain the only local authority and
restore owner. The composed framework receives no storage trait, lifecycle
participant or policy cache.

Missing, invalid or fenced projection state fails closed. A valid open
projection returns `FleetAdmissionRequired` for an unlisted caller; an invalid
local authority remains an internal typed access failure. Consuming endpoint
code may map the typed access error into its own Candid response, but admission
does not replace application membership or resource ownership.

## Generic composed-framework fixture

The existing managed `canic_icydb_lifecycle_probe` remains the generic
composition fixture. The accepted B5 candidate used exact published
`icydb = 0.230.2`; the current 0.109.7 draft advances that dependency boundary
to exact published IcyDB 0.240.1 without changing the existing synchronous
Canic/IcyDB lifecycle participant. B5 adds three direct IC-CDK methods rather
than a second endpoint framework or storage owner:

- a deliberately public caller probe;
- a guarded update that calls `require_caller()` before incrementing one
  application-work counter; and
- a read-only counter query.

A Canic-macro endpoint using `caller::is_fleet_admitted()` is present in the
same artifact for parity. The fixture counter is application test evidence,
not authority or durable runtime state.

The PocketIC journey proves:

1. an unlisted caller still reaches the deliberately public method directly;
2. while fresh activation is fenced, both protected paths reject and the
   composed workflow counter stays zero;
3. after exact Root activation, the admitted caller receives its observed
   Principal through both paths;
4. the composed workflow runs exactly once for that admitted call; and
5. a later unlisted caller receives typed denial through both paths without
   increasing the counter.

No Core proxy, forwarded identity or remote policy lookup participates.

## Targeted qualification

The final B5 candidate passed:

```text
cargo test --locked -p canic-core fleet_admission --lib
# 24 passed, including the shared native decision and synchronous facade guards

cargo test --locked -p canic --test protocol_surface \
  composed_framework_fleet_admission_facade_is_typed_and_synchronous
# 1 passed

cargo check --locked -p canic_icydb_lifecycle_probe
cargo test --locked -p canic-testing-internal --lib \
  pic::lifecycle::tests::composed_framework_guard_matches_canic_endpoint_on_direct_ingress \
  --no-run
cargo test --locked -p canic-tests --test icydb_lifecycle_composition --no-run
# all passed

bash scripts/ci/run-with-test-scratch.sh \
  bash scripts/ci/run-workspace-tests.sh targeted-pocketic \
  pic::lifecycle::tests::composed_framework_guard_matches_canic_endpoint_on_direct_ingress
# 1 passed in 11s; 274,740 kB final RSS, 326,084 kB high-water mark,
# 19 PocketIC threads; current Wasm fingerprint reused after its immediately
# preceding local build

make layering-gate

cargo clippy --locked -p canic-core -p canic -p canic-testing-internal \
  -p canic_icydb_lifecycle_probe --all-targets -- -D warnings

cargo fmt --all -- --check
git diff --check
# all passed
```

The first ad hoc Cargo invocation was rejected before the journey because it
did not supply the repository-owned PocketIC server. The first canonical
runner attempt then failed before testing because the filesystem/network
sandbox denied its loopback bind. Re-running the unchanged canonical command
with loopback permission executed the case and passed; neither preliminary
failure identified a product defect or caused an external effect.

The ordered internal PocketIC inventory now contains 23 cases, with the B5
direct-ingress journey included in the complete maintainer gate. That complete
gate, the broad PocketIC matrix and the external Toko adoption review were not
run. Repository policy reserves the complete gate for explicit maintainer
authorization, and Toko remains read-only from this repository.

## Current published-IcyDB refresh

Published 0.109.7 resolves all six IcyDB packages at exact published
0.240.1. The existing fixture required no source adaptation. Its focused
refresh qualification passed:

```text
cargo check --locked -p canic_icydb_lifecycle_probe \
  -p canic-icydb-lifecycle-schema

cargo test --locked -p canic-core --test timer_inventory_guard \
  timer_provider_graph_and_manifest_consumers_are_closed \
  -- --exact --nocapture

cargo test --locked -p canic-tests --test icydb_lifecycle_composition --no-run

cargo clippy --locked -p canic_icydb_lifecycle_probe \
  -p canic-icydb-lifecycle-schema --all-targets -- -D warnings

CANIC_POCKET_IC_SERVER_URL=http://127.0.0.1:41337/ \
  cargo test --locked -p canic-tests --test icydb_lifecycle_composition \
  managed_canic_and_published_icydb_share_lifecycle_and_timer_custody \
  -- --exact --nocapture --test-threads=1
# all passed; the PocketIC journey passed 1/1 in 9.35 seconds with cached Wasms
```

This refresh was repository-local. It did not mutate IcyDB, another workspace,
an identity, a canister or a network.

## Result

At B5 acceptance, a composed endpoint could enforce the same exact caller and
managed projection as a Canic endpoint without a second authority, storage
record, lifecycle owner, timer or remote lookup. B6 then owned runtime
prepare/fence/activate/open convergence and forward recovery. Current release
readiness is tracked in the active 0.109 status rather than this historical B5
result.
