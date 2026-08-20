# Canic 0.107 B5-B7 Implementation Progress

Date: 2026-08-20

This is implementation evidence for the accepted 0.107 design. It is not the
human closeout audit and does not accept the minor.

## Source Boundary

| Item | Identity |
| --- | --- |
| current branch/HEAD | `main` / published checkpoint `v0.107.0` at `448c5f2d19c41b9a71dd2b69affa32d8cf4868df`, plus the dirty B5 closeout batch |
| published direct predecessor | annotated `v0.106.0`, peeled commit `63c80c21fd5d67a70d1a2173afcdd4ad0f33fc30` |
| published 0.107 checkpoint | annotated `v0.107.0`, peeled commit `448c5f2d19c41b9a71dd2b69affa32d8cf4868df` |
| accepted B1 production baseline | annotated `v0.105.0`, peeled commit `b6c46ca1d307e0a3fed6f7bfddfba7d9f1922811` |
| working `Cargo.lock` SHA-256 | `5d98823414e6b0a8957e41d5d96f87e8c0b680757855040085d02e39394d5217` |
| Rust/Cargo | `rustc 1.97.1 (8bab26f4f 2026-07-14)` / `cargo 1.97.1 (c980f4866 2026-06-30)` |

The source is a dirty 0.107 implementation worktree over published checkpoint
`v0.107.0`. No version, tag, commit or external Canister operation was created
by this B5 closeout batch.

## B5 Typed-Upstream Result

`cargo tree --locked --offline -p canic-host -i ic-query` resolves exactly
`ic-query v0.41.0 -> canic-host`; the CLI reaches the same package only through
`canic-host`. The crates.io lock checksum is
`2d9f80d344dab9cb5ad21029f66c5aad23317c330d61c8fce06cc6424101d7d6`.
The relevant published source identities are:

| Published `ic-query 0.41.0` source | SHA-256 |
| --- | --- |
| `subnet_catalog/host/failure.rs` | `db027c7b82a8f23db564e9d975ef4e0e2852feba09ef1aa1e78368bbf9911666` |
| `subnet_catalog/host/error.rs` | `f790ea60f974b903599473d48953d9cfda1fd2b71bed1450975a6e615a66f1d8` |
| `subnet_catalog/host/cache.rs` | `e66ec094fda4262ec790b7b992634319792c103f3cdd8c3c416aa150f1291097` |
| `ic_registry/catalog.rs` | `a7ea0a511ab341c58acbbf332b6703265c06ab44ead0c32cbcd0a22fcc09cab2` |

The published detailed APIs now return `SubnetCatalogLoadFailure`, which
retains request network/source/assurance, exact load stage, failure-side cache
disposition, a Registry version known before later failure, typed subject,
stable code/category and
`SubnetCatalogRetryability::Unknown(SubnetCatalogUnknownRetryReason)`.

Canic switches both cached and live wrappers to those detailed APIs. One
exhaustive typed host projection carries every upstream-known field into the
deployment-plan JSON/text report and adds the three local false effect facts.
The wrapping path reads no prose to derive identity, version, subject, cache
state or retryability. A real missing-cache journey remains read-only and is
reported as cache absence; a pinned-version Registry-record fixture remains
unknown with its typed reason rather than becoming transient. B5 is complete.

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

The read-only Toko checkout is clean `main` at
`bf14a5d3d89be4335d3da2601e8a60128fde04df`. It has no Canic integration and
no `CANIC-011`, `CANIC-012` or `CANIC-013` identifier. That is the exact B7
downstream evidence blocker permitted by acceptance criterion 10; no Toko file
was changed to manufacture acceptance.

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
cargo test --locked -p canic-cli deploy::tests::plan -- --nocapture
  22 passed
cargo test --locked -p canic-cli catalog_failure_rendering_preserves_typed_unknown_provenance -- --nocapture
  1 passed
cargo clippy --locked -p canic-host -p canic-cli --all-targets -- -D warnings
  passed
cargo tree --locked --offline -p canic-host -i ic-query
  one `ic-query v0.41.0` package directly beneath `canic-host`
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

The direct Cargo Wasm build correctly rejected as unsupported. The
standalone-local artifact was then produced with the canonical `canic build`
path before Candid extraction. No broad workspace, full PocketIC or release
validation suite was run.

## Remaining Boundary

B5-B7 implementation evidence is complete. B5 changes only the host dependency
and host/CLI report path, so it does not alter the B7 managed/Root/Coordinator/
Store/runtime-probe Wasm or Candid boundary recorded above. The line is
**ready for human 0.107 closeout audit**. Do not begin 0.108 production before
the maintainer requests and accepts that exact audit.
