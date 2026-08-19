# 0.105 B2 Canonical Authority Evidence

Date: 2026-08-19

## Outcome

B2 completes the model and pure-policy boundary without adding stable session
state:

- `ApplicationScope`, `ApplicationScopeRef` and
  `CanonicalApplicationScopes` have one model owner and enforce the accepted
  1-64 byte colon-segment grammar, count limits, aggregate session bound,
  sorting and exact-duplicate rejection;
- `canic::application_scope!` uses the same constant validator and has passing
  compile-pass and compile-fail documentation tests;
- delegated-token preparation has no caller-selected presenter or subject and
  derives both signed claims from the authenticated preparation caller;
- canonical claims bytes place presenter before subject, so issuer proof and
  cache identity cover the presenter-bearing hard-cut shape;
- cold and cached proof verification both require
  `presenter == subject == current caller` and return the one model-owned
  `VerifiedApplicationAuthority`; the old ops-local verified-token projection
  is absent;
- the model projection owns canonical local scopes, Fleet, role, proof times
  and replay fingerprint; target-local authority generation is attached only
  when B3 constructs a local session and is not falsely asserted by a remote
  proof;
- pure policy fixes authorization denial precedence, non-empty scope
  narrowing, exact 60-second proof eligibility, independent protected session
  expiry, exact retry, replay conflict, atomic replacement and target-local
  capacity denial; and
- the Candid contract contains required signed presenter, excludes both
  identities from preparation requests and rejects presenter-less predecessor
  claims.

B2 introduces no stable record, memory ID, session mutation, role protocol
variant or configuration table. Those remain in their named B3 and B4 batches.

## Focused Validation

| Command | Result |
| --- | --- |
| `cargo test --locked -p canic-core application_authorization --lib` | PASS, 13 tests |
| `cargo test --locked -p canic-core ops::auth::delegated:: --lib` | PASS, 98 tests |
| `cargo test --locked -p canic-core ops::auth::delegated::verify:: --lib` | PASS, 19 tests after final projection assertions |
| `cargo test --locked -p canic-core workflow::runtime::auth::prepare:: --lib` | PASS, 17 tests |
| `cargo test --locked -p canic-core access::auth:: --lib` | PASS, 17 tests |
| `cargo test --locked -p canic-core delegated_token_candid_hard_cuts_presenter_and_request_subject --lib` | PASS, 1 test |
| `cargo test --locked -p canic --doc application_scope` | PASS, valid and compile-fail examples |
| `cargo clippy --locked -p canic-core --lib --tests -- -D warnings` | PASS |
| `cargo clippy --locked -p canic --lib --tests -- -D warnings` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `git diff --check` | PASS |

The complete workspace, release matrix and PocketIC suites were not run; they
are outside this focused batch gate.
