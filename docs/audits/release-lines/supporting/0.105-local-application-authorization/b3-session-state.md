# 0.105 B3 Canonical Session-State Evidence

Date: 2026-08-19

## Outcome

B3 replaces the old subject-only delegated session in memory ID 34 with one
current-format local application authorization state:

- `LocalApplicationSessionRecord` binds transport caller, authenticated
  subject, issuer, Fleet, role, canonical scopes, local authority generation,
  establishment/session times, proof fingerprint and establishment-request
  hash;
- `LocalApplicationReplayRecord` binds proof fingerprint, exact caller/subject,
  authority generation and strict proof-expiry removal time;
- `application_authority_generation` is independent of the root issuer
  registry and proof epochs; and
- the existing memory ID 34 and `canic.core.auth.state.v1` key remain the sole
  current encoding. There is no old reader, alternate record generation,
  migration decoder or dual write.

The old `AuthApi` delegated-session methods, wallet/delegated-subject storage
views, bootstrap-binding state, expiry clamp and corresponding test-canister
endpoints are deleted rather than adapted.

## State And Recovery Contract

The ops boundary reconstructs four derived `BTreeMap` indexes from canonical
records: exact caller, exact proof fingerprint, active count by subject and
replay count by subject. Indexes are heap-only and unavailable until a
successful synchronous restore. Restore rejects over-capacity vectors,
duplicate callers or fingerprints, non-canonical scopes, invalid or overlong
session windows, future generations, missing/mismatched replay authority,
oversize records and over-limit stable/index footprints.

Session/replay replacement is prepared and validated before one auth-cell
commit. Clear removes only the exact caller session and retains replay state.
Cleanup removes no more than 128 strictly expired records. A replay whose
proof has expired is removed on that independent clock even while its session
remains active. The active record's own proof fingerprint and request hash
continue to resolve exact retry, and same-release restore accepts that current
shape while still rejecting duplicate session fingerprints or mismatched
overlapping replay authority. Read and authorization paths do not clean,
repair or mutate state.

The workflow resolves an exact proof/request retry before proof eligibility.
It returns only the still-active, unexpired, current-generation session and
does not change its expiry. Conflicting reuse denies. A new verified proof
then passes the accepted proof-lifetime, scope-narrowing and capacity policy
before atomic commit.

Lifecycle restoration calls the application-session reconstruction from the
shared derived-index restore path used by managed, Root and local init/upgrade
flows. This remains synchronous and therefore completes before the existing
0.104 lifecycle participant and deferred work.

## Bounded Resource Record

| Quantity | Current result |
| --- | ---: |
| One representative active-session record | 519 CBOR bytes |
| One replay record | 198 CBOR bytes |
| Maximum-format state with maximum authority binding | 4,025,450 bytes |
| Maximum restore fixture with its exact binding | 4,025,139 bytes |
| PocketIC injected maximum state before binding reconciliation | 3,995,192 bytes |
| Stable-state ceiling | 8,388,608 bytes |
| Conservative maximum derived-index estimate | 2,039,808 bytes |
| Derived-index ceiling | 4,194,304 bytes |
| Maximum cleanup removals | 128 records |

Each maximum state measurement contains 2,048 active sessions, each with 16
64-byte canonical scopes, and 4,096 replay records. The first two distinguish
the maximal test binding from the exact restore fixture binding. The PocketIC
injection starts without a binding so same-release lifecycle reconciliation
constructs it from current protected authority. These are encoding facts, not
traffic-capacity claims.

## Focused Validation

| Command | Result |
| --- | --- |
| `cargo test --locked -p canic-core application_sessions --lib` | PASS, 15 tests |
| `cargo test --locked -p canic-core application_authorization --lib` | PASS, 14 tests |
| `cargo test --locked -p canic-core access::auth:: --lib` | PASS, 12 tests |
| `cargo test --locked -p canic-core current_session_and_replay_cbor_footprint_stays_bounded --lib` | PASS, one exact resource-contract test |
| `cargo check --locked -p canister_test -p delegation_issuer_stub -p delegation_root_stub` | PASS |
| `cargo clippy --locked -p canic-core --lib --tests -- -D warnings` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `git diff --check` | PASS |

The complete workspace, release build matrix and PocketIC suites were not run;
they are outside this focused implementation-batch gate.
