# 0.105 B6 Operator, Security And Resource Evidence

Date: 2026-08-20

## Outcome

B6 completes the protected operator view, bounded aggregate metrics,
authority-generation reconciliation and the accepted `E/A/D/B/H/R/C/M`
resource record. It adds no method identity, framework dependency, timer owner,
lifecycle owner or stable-state generation.

`CanisterStatusRequest::ApplicationSessionAudit(PageRequest)` exists only in an
explicitly enabled managed-role Candid. The existing status dispatcher requires
the exact Root authority before the API reads protected policy or session
state. The response separates the current policy/binding from a caller-ordered,
bounded page of retained sessions. It includes the operator-required caller,
subject, issuer, scopes, timestamps, generation and active/inactive
classification, but contains no token, proof bytes, proof fingerprint or
establishment-request hash. Caller-self status remains independently
authorized and cannot select the operator view.

The one durable local authority generation remains in memory ID 34. Lifecycle
reconciliation derives one binding from current protected Fleet, role,
verifier root, minimum accepted registry epoch, canonical scopes and maximum
session TTL. The pure transition table advances generation only for the frozen
invalidating changes; default-TTL and authority additions remain future-only,
while disabled/unavailable authority and subject inadmissibility deny on the
read path. A generation advance records one bounded aggregate metric and does
not erase or re-key replay evidence.

## Security And Metrics

Application-session metrics use closed surface, operation, outcome and reason
enums. They cover establishment start/create/replace/idempotent/rejection,
clear, expired observation, bounded cleanup and generation invalidation. Their
public labels contain no caller, subject, issuer, Fleet, role, scope, resource
identifier, proof fingerprint, token bytes or payload. The pure local decision
records no metric; the Canic-owned command/status/cleanup boundaries record the
aggregate events.

The `internal-test-fixtures` Cargo feature exposes only the pure denial-branch
measurement helper to the delegation test canister. It is declared in the
canonical feature catalogue, is not selected by any product-role build and
does not grant runtime state or protocol capability. Its three measurement
queries are test-fixture Candid only.

## Resource Results

All instruction values below are single deterministic observations from the
current test artifact on repository-pinned PocketIC 15.0.0. They are not
percentages, medians or general throughput claims.

| Symbol | Current observation | Accepted conclusion |
| --- | ---: | --- |
| E, warm establishment | 4,584,113 instructions | Existing verified-proof cache is used; session commit remains bounded. |
| E, cold establishment | 476,135,709 instructions | Includes complete embedded-proof verification. |
| E, cold proof component | 471,154,912 instructions | Reported separately from the surrounding establishment work. |
| A, 1 active session / 16 maximum-width scopes | 180,797 instructions | Below the 1,000,000 ceiling. |
| A, 1,024 active sessions / 16 maximum-width scopes | 186,749 instructions | Below the ceiling at the admitted midpoint. |
| A, 2,048 active sessions / 16 maximum-width scopes | 187,112 instructions | Below the ceiling at maximum admitted `G` and `S`. |
| B, one active record | 519 CBOR bytes | Below the 2,048-byte record ceiling. |
| B, one replay record | 198 CBOR bytes | Bounded canonical replay authority. |
| B, maximum-format state/binding | 4,025,450 CBOR bytes | Below the 8 MiB ceiling. |
| H, maximum reconstructed indexes | 2,039,808 estimated bytes | Four exact derived indexes remain below the 4 MiB ceiling. |
| R, maximum same-release restore | 1,168,165,228 instructions | One synchronous restore checkpoint reconstructs all indexes before ingress. |
| C, one maximum cleanup delivery | 128 removals / 1,483,321,373 instructions | The exact work-count ceiling is enforced. |

The PocketIC maximum fixture injects 2,048 active sessions and 4,096 replay
records with no stored authority binding, then upgrades the same-release Wasm.
Lifecycle restore and binding reconciliation reconstruct the current state.
The injected auth-state payload is 3,995,192 bytes. Physical stable memory is
218,169,344 bytes (3,329 pages) both before and at maximum state, so the
observation has zero stable-page growth. Cleanup also leaves that physical
extent unchanged. The larger 4,025,450-byte unit value deliberately includes a
maximal encoded authority binding; these values describe different exact
fixtures and are not contradictory.

Every closed local denial branch also remains below the 1,000,000-instruction
ceiling:

| Denial | Instructions |
| --- | ---: |
| Anonymous | 59,455 |
| Caller mismatch | 58,742 |
| Disabled | 59,496 |
| Authority unavailable | 59,517 |
| Missing session | 59,513 |
| Expired | 59,511 |
| Stale authority | 62,433 |
| Inadmissible subject | 62,442 |
| Missing scope | 62,931 |

## Controlled Wasm And Candid Comparison

The comparison uses exact annotated predecessor `v0.104.2` at peeled commit
`0811b7d3ea3e0ebae5b522faa1f0f18d4dca1220` and the current working tree. The
predecessor `Cargo.lock` SHA-256 is
`94b1338b8fcd9b1ebfe2271ff42e63942eacc988c3f1d3490eb05861200a4de3`;
the current lock SHA-256 is
`e5823fc93e63323600fe1c0997636df7d46e3258c84909f059ef6dead6b8b616`.
Each source uses its own locked graph. Both builds use equal-length temporary
source/target roots, Rust/Cargo 1.97.1, `wasm32-unknown-unknown`, the repository
`fast` profile, `apps/test/canic.toml`, `ICP_ENVIRONMENT=local`, offline Cargo
and the canonical `canic-host` `build_artifact` example. No release-build ID is
supplied, so the comparison does not reuse the unpreserved 0.104 release
identity input.

| Role | Predecessor raw | Current raw | Raw delta | Predecessor gzip | Current gzip | Gzip delta |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Component app | 3,559,180 | 3,627,127 | +67,947 | 941,979 | 961,050 | +19,071 |
| Fleet Subnet Root | 8,372,728 | 8,420,249 | +47,521 | 2,173,115 | 2,185,700 | +12,585 |
| Fleet Coordinator | 3,816,760 | 3,836,668 | +19,908 | 950,367 | 955,208 | +4,841 |
| Wasm Store | 3,306,010 | 3,352,083 | +46,073 | 875,421 | 886,098 | +10,677 |
| **Total** | **19,054,678** | **19,236,127** | **+181,449** | **4,940,882** | **4,988,056** | **+47,174** |

The raw total remains 80,695 bytes below the accepted 262,144-byte growth
ceiling. A repeated current build in a differently named equal-length source
root reproduced every raw role size, Candid hash and lifecycle export set. Its
gzip total moved from 4,987,851 to the final 4,988,056 because the embedded
equal-length path bytes differ; gzip is secondary context, not a claim of
byte-for-byte artifact reproducibility.

| Role | Predecessor/current Candid SHA-256 | Methods | Lifecycle exports |
| --- | --- | --- | --- |
| Component app | `fd72b06353a7f3e10d7bd59cad88ba3f8ee592c1dd783f320143135e6a758a9f` | `canic_command`, `canic_status`, `icrc10_supported_standards`, `icrc21_canister_call_consent_message` | `canister_init`, `canister_post_upgrade` |
| Fleet Subnet Root | `f132139527f7283e7b255fbcc438f0cb6d24048171a88ea47c9b9a58019d8917` | `canic_command`, `canic_status`, `icrc10_supported_standards` and three test-only Root helpers | `canister_init`, `canister_post_upgrade` |
| Fleet Coordinator | `5c6e374b3462289023b67ff698997a29f0496ec3b438eacfbc4d6ea084cac0a8` | `canic_command`, `canic_status` | `canister_init` |
| Wasm Store | `6d0f224a7d7a755939454ad898d2068d2054e4c1b2028422c6f1617af104291b` | `canic_command`, `canic_status`, two chunk methods, `icrc10_supported_standards` | `canister_init`, `canister_post_upgrade` |

Every exact Candid hash and lifecycle export set is unchanged between the two
controlled product artifacts. The enabled delegation fixture separately proves
that application-session variants are nested beneath the existing methods and
are absent from capability-disabled roles.

## Focused Validation

| Command | Result |
| --- | --- |
| `cargo test --locked -p canic-core --test timer_inventory_guard` | PASS, 16 tests; the 0.105 changes introduce no unclassified timer authority. |
| `cargo test --locked -p canic-core application_sessions --lib` | PASS, 15 tests including bounded operator page, generation transition, restore, cleanup and maximum state. |
| `cargo test --locked -p canic-core ops::runtime::metrics::auth --lib` | PASS, 2 tests. |
| `cargo test --locked -p canic-core auth_metrics_are_exposed_with_stable_labels --lib` | PASS, 2 matching focused tests. |
| `cargo test --locked -p canic --test protocol_surface application_session_audit_is_bounded_protected_and_secret_free -- --exact` | PASS, one test. |
| `cargo test --locked -p canic-tests --test native_agent_delegation pem_backed_native_agent_prepares_retrieves_and_presents_delegated_token -- --exact --test-threads=1 --nocapture` | PASS, warm/cold establishment, protected audit, exact retry, clear and token-free consumer. |
| `cargo test --locked -p canic-tests --test native_agent_delegation local_application_authorization_lookup_is_bounded_at_one_and_median_state -- --exact --test-threads=1 --nocapture` | PASS, one and midpoint observations. |
| `cargo test --locked -p canic-tests --test native_agent_delegation maximum_application_session_resource_contract_is_bounded -- --exact --test-threads=1 --nocapture` | PASS, maximum restore/authorization/cleanup and zero page growth. |
| `cargo test --locked -p canic-tests --test native_agent_delegation closed_local_authorization_denial_partition_is_bounded -- --exact --test-threads=1 --nocapture` | PASS, all nine denial branches. |
| `cargo test --locked -p canic-tests --test native_agent_delegation -- --test-threads=1 --nocapture` | PASS, all five focused B6/B7 native-agent, instruction, memory and recovery tests. |
| `cargo test --locked -p canic-tests --test native_agent_delegation multi_target_sessions_preserve_controller_separation_and_same_release_recovery -- --nocapture --test-threads=1` | PASS against final source after lint cleanup. |
| `cargo clippy --locked -p canic-core -p canic -p canic-macros -p canic-host -p canic-testing-internal --all-targets -- -D warnings` | PASS. |
| `cargo clippy --locked -p canic-tests --test native_agent_delegation -- -D warnings` | PASS. |
| `cargo fmt --all -- --check` | PASS. |
| `bash scripts/ci/check-current-document-semantics.sh` | PASS after advancing the maintained 0.105 state to closed B2-B7. |

The four product artifacts were built independently for predecessor and
current source; `candid-extractor 0.1.6`, `ic-wasm 0.11.1` and `wasm-objdump`
confirmed Candid identities and lifecycle exports. Full workspace and release
validation remain maintainer-owned and were not run.

## Complexity Disposition

B6 adds one required operator status variant and passive DTO projection, one
closed metric family extension, one lifecycle binding reconciliation and
test-only measurement instrumentation. It adds no durable state beyond B3,
removes no additional state, and leaves product methods, lifecycle exports,
timer ownership and framework dependencies unchanged. The maintained runtime
becomes more explicit at the operator boundary while the authorization hot
path remains the single B5 facade; overall runtime authority stays neutral.
