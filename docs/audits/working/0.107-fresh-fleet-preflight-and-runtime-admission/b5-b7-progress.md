# Canic 0.107 B5-B7 Implementation Progress

Date: 2026-08-21

This is implementation evidence for the accepted 0.107 design. It is not the
human closeout audit and does not accept the minor.

## Historical Development Snapshot

The following table records the bounded source state at which the original
B5-B7 evidence was captured. The word "dirty" describes that past development
snapshot only; it is not a claim about the current candidate or worktree.

| Item | Identity |
| --- | --- |
| captured branch/source | `main` / published checkpoint `v0.107.1` at `5487e4e765a46371af69e09ea1e6e347444b6be6`, plus the then-dirty `0.107.2` B5 correction |
| published direct predecessor | annotated `v0.106.0`, peeled commit `63c80c21fd5d67a70d1a2173afcdd4ad0f33fc30` |
| published 0.107 checkpoint | annotated `v0.107.0`, peeled commit `448c5f2d19c41b9a71dd2b69affa32d8cf4868df` |
| published typed-provenance checkpoint | annotated `v0.107.1`, peeled commit `5487e4e765a46371af69e09ea1e6e347444b6be6` |
| accepted B1 production baseline | annotated `v0.105.0`, peeled commit `b6c46ca1d307e0a3fed6f7bfddfba7d9f1922811` |
| working `Cargo.lock` SHA-256 | `fb51f9a86fc867251bf1e7c3e22e5a58ae4a5d44f21f9bbaa9ddb5180127113f` |
| Rust/Cargo | `rustc 1.97.1 (8bab26f4f 2026-07-14)` / `cargo 1.97.1 (c980f4866 2026-06-30)` |

At capture, the source was a dirty 0.107 correction worktree over published
checkpoint `v0.107.1`. No Canic package version, tag, commit or external
Canister operation was created by that B5 correction batch.

## B5 Typed-Upstream Result

`cargo tree --locked --offline -p canic-host -i ic-query` resolves exactly
`ic-query v0.41.2 -> canic-host`; the CLI reaches the same package only through
`canic-host`. The crates.io lock checksum is
`a9c7486d35030ca36b45636599da5f142b92bf51c548cf238d8567750376fded`.
The relevant published source identities are:

| Published `ic-query 0.41.2` source | SHA-256 |
| --- | --- |
| `subnet_catalog/host/failure.rs` | `bf61ada78a28897e7c4d20efe5a375bce6cf529bf80ec380d8bfc7a61522a16c` |
| `subnet_catalog/host/error.rs` | `fbc133374a67d8d449be6b18b092591ae25f489b50c641703c2664faed397584` |
| `subnet_catalog/host/cache.rs` | `e66ec094fda4262ec790b7b992634319792c103f3cdd8c3c416aa150f1291097` |
| `ic_registry/catalog.rs` | `9fc8803ee6cd38d0993a3994d96b3b89dc9346a4568dfdcd6eb82cd6e62a287c` |
| `ic_registry/source/subnet_catalog.rs` | `64d08cab25bcdab6e369bc9af2c659c64e719f05958ec7da746a6bba28bcc56f` |
| `subnet_catalog/model/types.rs` | `c1f360ea9159f34b7cfd5db261c87419b012c1133afa1dc3925ac73969e7b26c` |

The published collector reconstructs the complete pinned `canister_ranges_*`
family and reads legacy `routing_table` only when that family is genuinely
empty. It never falls back after a modern-family error. Its detailed APIs
return `SubnetCatalogLoadFailure`, which retains request network/source/
assurance, exact load stage, failure-side cache disposition, pinned and
returned Registry-value versions, exact failing endpoint/assurance, completed
record reads, typed subject, stable code/category and
`SubnetCatalogRetryability::Unknown(SubnetCatalogUnknownRetryReason)`.

The portable API additionally exposes canonical Registry-key constants, typed
Subnet-list, legacy-routing, routing-shard and Subnet-record subject builders,
and `SubnetCatalogRegistryRecordEvidence::uncertified_query`. Canic's valid
Root-subnet cache fixture consumes those builders instead of duplicating key
formatting; this is the same downstream fixture boundary required by Toko
Miner. No Toko file was changed.

Canic switches both cached and live wrappers to those detailed APIs. One
exhaustive typed host projection carries every upstream-known field—including
each completed record's key/schema subject, shard lower bound, requested and
returned versions, timestamp, endpoint, assurance and inline/chunked
encoding—into the deployment-plan JSON/text report and adds the three local
false effect facts. The wrapping path reads no prose to derive identity,
version, subject, cache state or retryability. A real missing-cache journey
remains read-only and is reported as cache absence; a pinned-version Registry-
record fixture remains unknown with its typed reason rather than becoming
transient. Old cache shapes fail closed and require refresh without a reader or
migration. B5 is complete.

### Stable Authority Follow-Up

Published `ic-query 0.42.0`, locked with crates.io checksum
`311b60543bc5c09c961abe9612d2bf3e26e99ba8bcadb3c01d043056c544a318`,
hard-cuts reproducible snapshot identity from transient acquisition provenance.
Canic replaces the removed combined authority accessor with
`CatalogLoadOutcome::snapshot_authority()` and keeps only Registry version,
catalog digest, assurance and source endpoints in the canonical Fleet
decision. Cache path, collection time and disposition are rendered separately
as `catalog_acquisition`; no alias or compatibility adapter was added.

A focused host test gives one validated catalog both `refreshed_missing` and
`cache_hit` acquisition outcomes and proves identical canonical authority with
truthfully different acquisition provenance. Direct plan refresh now compiles
from the live-capable load outcome without the former extra cache-only reload.
Install retains its existing exact second compilation gate, whose cache hit is
now naturally equal to the first load's stable authority.

## Closeout-Feedback Correction

The downstream rerun found one remaining operational gap: a fresh checkout
could not produce an authoritative mainnet plan until another command had
created the catalog cache. The corrected CLI now exposes
`canic deploy plan --refresh-catalog`. Ordinary planning remains cache-only;
the explicit mode selects the existing `RefreshMissingOrInvalid` policy, may
issue public NNS Registry query calls and may update only the private
`.canic/ic-query` cache.

This acquisition step still cannot build Wasm, mutate deployment state or
perform an IC update call. Its validated Registry version, catalog digest,
assurance and source endpoints enter the canonical decision used by
plan/install parity. Cache path, collection time and disposition remain
separate report provenance, so the request and transient refresh result cannot
change the digest. Fresh installation uses the same acquisition policy
automatically when the cache is missing or invalid. No live mainnet call was
needed or made while testing this correction.

The same downstream feedback exposed CANIC-009: install could discover an
anonymous, locked or wrong effective ICP identity only after expensive build
preparation. The corrected install resolves that identity after exact plan
recompilation and before release-build allocation or Wasm preparation. It must
be usable, non-anonymous and exactly equal to the Fleet-input operator;
encrypted non-interactive failure points directly to
`CANIC_ICP_IDENTITY_PASSWORD_FILE`. The creation-time controller observations
remain as later time-of-check/time-of-use defenses. Cache-only catalog blockers
now recommend `--refresh-catalog` instead of Fleet-input repair.

## B6 Durable Runtime Whitelist

The complete implementation owns:

- memory ID 61, stable key `canic.core.runtime.whitelist.v1` and one current
  schema-1 record;
- 256 canonical principals, 128-entry pages and one retained operation;
- domain-separated membership and operation hashes frozen by B1;
- fresh compiled-seed bootstrap and same-release validate-only restoration;
- atomic complete-record mutation with exact replay and no rejected-state
  commit;
- controller-first or exact stable-Root administration beneath the existing
  managed role methods; and
- config-independent runtime access evaluation, separate from 0.105
  application sessions.

The production schema itself freezes exact maxima of 8,417 stable bytes,
4,072 status Candid bytes and 101 mutation-request Candid bytes. Restore
validation rejects unsupported schema, non-canonical membership, a bad digest,
an inconsistent retained outcome and a retained request-hash mismatch.

## B7 Independent Proof

The generic delegation managed fixture now exposes one ordinary
`caller::is_whitelisted()` consumer and seeds one principal. Its bounded
PocketIC journey proves:

1. fresh managed bootstrap reads the compiled seed;
2. an independent current controller and the exact stable Root can inspect and
   mutate, while an unrelated caller cannot inspect;
3. removal denies the next access evaluation;
4. a response-loss retry returns the exact accepted removal;
5. operation-ID and revision conflicts leave revision unchanged;
6. same-release upgrade restores the removed state without reseeding;
7. the retained exact operation survives restoration;
8. Root can re-add without rebuilding; and
9. membership creates no local application session.

Artifact extraction proves `RuntimeWhitelist` appears under the existing
managed `canic_command`/`canic_status` pair and nowhere in the inspected Root,
Coordinator, Store or standalone-local specialized artifacts:

| Profile | Wasm SHA-256 | Candid SHA-256 | Runtime-whitelist surface |
| --- | --- | --- | --- |
| managed delegation fixture | `8ce163e4511408ced8855cf76911bd0b4742f39227fffdb51d099a94835a51b9` | `9d3b6ba03d766c6ff7c34b675646bf961d1c478f55d2fb24e82cede4a808980b` | command/status variants plus fixture consumer |
| Root fixture | `1151a314c5a0aed68b6cb28332cb0191f5b0d7a1a61113845476a21fd088be2c` | `07a9af5760ac5d7b758cc40516a2f146df03c2cda4bd2b03d11be28b3a3f2352` | absent |
| Wasm Store fixture | `e5ce902b087b3b0609ecbfa9c2db6999bb627a889162eacdcd3fc035eaca0aab` | `6d0f224a7d7a755939454ad898d2068d2054e4c1b2028422c6f1617af104291b` | absent |
| Coordinator fixture | `41be3cfb4701685aaec5b91e7cd6b517df99d0a660fe761ec33634567180409e` | `5c6e374b3462289023b67ff698997a29f0496ec3b438eacfbc4d6ea084cac0a8` | absent |
| standalone-local runtime probe | `f730e49fe4e6695bdeaa3801bc18a0962095c7b1e5a2a9f6dad5704bce2d9c2f` | `f2a79cc00ee6fc8dfa56e3a939f4522e3dcc04b6620dff75aa37ff900df0af51` | absent |

The original read-only Toko snapshot was clean `main` at
`bf14a5d3d89be4335d3da2601e8a60128fde04df` and had no Canic integration or
feedback identifier. A later read-only inspection at current Toko HEAD
`2af2182f97cb21e220081d49169d6a006eff1adb` preserved its existing dirty user
work and supplied concrete CANIC-009/CANIC-012 deployment feedback. The Canic
correction owns those changes; no Toko file was changed. Acceptance criterion
13 retains that read-only downstream boundary.

## Focused Validation

The following focused commands pass:

```text
cargo test -p canic-core runtime_whitelist -- --nocapture
  9 passed
cargo test --locked -p canic-core replay_policy::tests::role_command -- --nocapture
  10 passed
cargo test --locked -p canic-core role_contract::tests::canonical_allocations_form_packed_owner_ledgers -- --exact --nocapture
  1 passed
cargo test --locked -p canic-core role_contract::tests::surplus_state_feature_allocates_normally -- --exact --nocapture
  1 passed
cargo test -p canic-core state_contract::tests::descriptors_exactly_cover_declared_core_memory_ids -- --nocapture
  1 passed
cargo test -p canic-core state_contract::tests::runtime_bindings_and_fleet_state_descriptors_reference_canonical_data_types -- --nocapture
  1 passed
cargo test -p canic --test managed_endpoint_gate runtime_whitelist_is_managed_only_and_authenticates_before_state_access -- --nocapture
  1 passed
cargo test -p canic --test protocol_surface runtime_whitelist_candid_uses_the_bounded_managed_role_contract -- --nocapture
  1 passed
cargo check -p delegation_issuer_stub -p canic_icydb_lifecycle_probe
  passed
cargo test -p canic-tests --test native_agent_delegation runtime_whitelist_is_durable_bounded_and_separate_from_application_sessions -- --nocapture
  1 passed against the pinned local PocketIC 15.0.0 server
cargo test --locked -p canic-tests --test pic_ingress_payload_limits -- --nocapture
  2 passed against the pinned local PocketIC 15.0.0 server
cargo test --locked -p canic-testing-internal --lib pic::fleet_registry::baseline::tests::qualification_reset_preflight_keeps_1_8_16_32_lanes_independent -- --exact --nocapture
  1 passed against the pinned local PocketIC 15.0.0 server
cargo clippy -p canic-core -p canic --all-targets -- -D warnings
  passed
cargo clippy -p delegation_issuer_stub --all-targets -- -D warnings
  passed
cargo clippy -p canic-tests --test native_agent_delegation -- -D warnings
  passed
cargo clippy --locked -p canic-core -p canic-testing-internal --lib -- -D warnings
  passed
cargo clippy --locked -p canic-tests --test pic_ingress_payload_limits -- -D warnings
  passed
cargo test --locked -p canic-host subnet_catalog -- --nocapture
  5 passed
cargo test --locked -p canic-host deployment_truth::report::root_subnet::tests::subnet_catalog_source_resolves_cached_canister_without_icq_process -- --exact --nocapture
  1 passed without a live refresh
cargo test --locked -p canic-cli deploy::tests::plan -- --nocapture
  23 passed
cargo test --locked -p canic-cli medic::tests::fleet -- --nocapture
  4 passed
cargo test -p canic-host install_identity_admission -- --nocapture
  3 passed
cargo test -p canic-host current_install_records_gates_before_activation_mutation -- --nocapture
  1 passed
cargo test -p canic-host install_recompiles_the_exact_plan_digest_and_rejects_changed_balance_evidence -- --nocapture
  1 passed
cargo test -p canic-host before_release_build_allocation -- --nocapture
  3 passed
cargo test -p canic-cli deploy_plan_ -- --nocapture
  24 passed
cargo test -p canic-cli install_usage_explains_app_config -- --nocapture
  1 passed
cargo test --locked -p canic-cli catalog_failure_rendering_preserves_typed_unknown_provenance -- --nocapture
  1 passed
cargo clippy --locked -p canic-host -p canic-cli --all-targets -- -D warnings
  passed
cargo clippy --locked --no-deps -p canic -p canic-macros --all-targets -- -D warnings
  passed with dead code, stale expectations and unreachable public items denied
cargo check --locked -p sharding_root_stub
  passed with the final workspace member inheriting the shared lint table
cargo check --locked -p saltz_preview -p canister_test -p canic_icydb_lifecycle_probe -p canic-icydb-lifecycle-schema -p delegation_issuer_stub -p root_funding_probe -p cycles_ledger_stub -p canic-testing-internal
  passed after exact generated-code dependency classification
cargo clippy --locked --no-deps -p saltz_preview -p canister_test -p canic_icydb_lifecycle_probe -p canic-icydb-lifecycle-schema -p delegation_issuer_stub -p root_funding_probe -p cycles_ledger_stub -p canic-testing-internal -p sharding_root_stub --all-targets -- -D warnings
  passed
cargo machete --skip-target-dir .
  passed with no unused dependency finding
cargo tree --locked --offline -p canic-host -i ic-query
  one `ic-query v0.41.2` package directly beneath `canic-host`
cargo tree --locked --offline -p canic-cli -i ic-query
  the same package reaches `canic-cli` only through `canic-host`
bash scripts/ci/run-layering-guards.sh
  passed
cargo test --locked -p canic --test changelog_governance -- --nocapture
  1 passed
cargo test --locked -p canic-core --test policy_purity_boundary_guard -- --nocapture
  2 passed
cargo test --locked -p canic-core --test passive_dto_boundary_guard -- --nocapture
  2 passed
cargo test --locked -p canic-core --test lifecycle_boundary_guard -- --nocapture
  7 passed
cargo test --locked -p canic-core --test stable_memory_abi_guard -- --nocapture
  2 passed
cargo fmt --all -- --check
  passed
bash scripts/ci/check-current-document-semantics.sh
  passed
git diff --check
  passed
```

The published `ic-query 0.42.0` stable-authority follow-up adds these focused
results without rerunning the broad release gate:

```text
cargo test --locked -p canic-host fleet_install_input -- --nocapture
  19 passed
cargo test --locked -p canic-host fleet_install_plan::tests -- --nocapture
  13 passed
cargo test --locked -p canic-host subnet_catalog -- --nocapture
  5 passed without a live refresh
cargo test --locked -p canic-cli deploy_plan_ -- --nocapture
  24 passed
cargo test --locked -p canic-cli catalog_acquisition_rendering_preserves_transient_provenance -- --nocapture
  1 passed
cargo test --locked -p canic-cli catalog_failure_rendering_preserves_typed_unknown_provenance -- --nocapture
  1 passed
cargo test --locked -p canic-host install_recompiles_the_exact_plan_digest -- --nocapture
  1 passed
cargo test --locked -p canic-host install_identity_admission -- --nocapture
  3 passed
cargo test --locked -p canic-host current_install_records_gates_before_activation_mutation -- --nocapture
  1 passed
cargo clippy --locked -p canic-host -p canic-cli --all-targets -- -D warnings
  passed
cargo tree --locked --offline -p canic-host -i ic-query
  one `ic-query v0.42.0` package directly beneath `canic-host`
cargo tree --locked --offline -p canic-cli -i ic-query
  the same package reaches `canic-cli` only through `canic-host`
cargo test --locked -p canic --test changelog_governance -- --nocapture
  1 passed
bash scripts/ci/check-current-document-semantics.sh
  passed
```

The first filtered host run exposed that the pre-`0.41.1` Root-subnet fixture
was no longer valid and therefore entered the production refresh path. It made
one read-only mainnet catalog refresh, returned a real route and failed the
fixture assertion. It performed no IC mutation; its invocation-owned temporary
cache was removed. The fixture now carries complete schema-1 Registry-record
evidence and uses an injected observation time. The exact rerun above completed
in 0.00 seconds without a live refresh.

The direct Cargo Wasm build correctly rejected as unsupported. The
standalone-local artifact was then produced with the canonical `canic build`
path before Candid extraction. No broad workspace, full PocketIC or release
validation suite was run.

## Maintainer Validation Follow-Up

A later maintainer-owned complete-validation run passed the Fleet deployment-
restore proof and every serial PocketIC lane. Its ordinary workspace unit lane
found two independent fixture races outside deployment behavior:

- placement recovery returned and persisted `bound_at = 1787314241`, while the
  assertion resampled the wall clock across the next second and expected
  `1787314242`; that first panic poisoned the shared seam-test mutex and caused
  14 follow-on failures;
- the successful-shrink fixture received Linux `ETXTBSY` while inspecting its
  just-written fake `ic-wasm` executable.

The placement test now compares the operation-returned timestamp with the
exact durable bound row. The artifact helper writes and closes a staged fake
tool, marks it executable and atomically publishes it before invocation.
Focused correction evidence passes:

```text
cargo test --locked -p canic-core workflow::placement::index::tests -- --nocapture
  12 passed
cargo test --locked -p canic-host artifact_io::tests -- --nocapture
  7 passed
cargo clippy --locked -p canic-core -p canic-host --all-targets -- -D warnings
  passed
```

The automated correction did not rerun the complete validation gate; that
remains part of the maintainer-owned release flow.

## Remaining Boundary

B5-B7 implementation evidence is complete after the `0.107.2` correction and
the closeout-feedback follow-up. Default planning is cache-only; explicit
`--refresh-catalog` enables the existing query-only live acquisition policy
and private-cache repair without deployment or IC update mutation. Install
uses that policy automatically and both paths compile from stable snapshot
authority while reporting acquisition provenance separately. Install then
admits the exact effective Fleet-operator identity before build preparation.
B5
otherwise changes only the host dependency, host/CLI report path and
deterministic host fixture. The accompanying compile-time hygiene narrows
internal facade/macro
visibility and makes dead code and stale lint expectations hard failures; it
also deletes two redundant dependencies from the test-only IcyDB composition
fixture and records only proven generated-code dependency exceptions. It does
not alter generated macro output. Neither part alters the B7 managed/Root/
Coordinator/Store/runtime-probe Wasm or Candid boundary recorded above. After
the maintainer reruns the complete gate, the line is **ready for human 0.107
closeout audit**. Do not begin 0.108 production before the maintainer requests
and accepts that exact audit.
