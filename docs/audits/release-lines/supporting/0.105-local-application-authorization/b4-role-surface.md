# 0.105 B4 Role-Surface Evidence

Date: 2026-08-19

This working evidence records B4's protected enablement and standard managed
role variants. It is not a release artifact or a substitute for the
maintainer-owned complete validation gate.

## Authority and surface

`CanisterAuthConfig::local_application_authorization` is the single protected
configuration owner. Its presence requires delegated-token verification, a
non-empty canonical scope allowlist with at most 32 entries, positive default
and maximum session lifetimes, `default <= maximum`, and a maximum no greater
than 1,800 seconds. Component and component-child roles use the same table.
Infrastructure role schemas cannot contain it.

The validated table derives one internal
`LocalApplicationAuthorization` role capability. The generated role cfg
controls all eight declaration, authorization and dispatch sites for:

- `CanisterCommand::ApplicationSession` with exact `Establish` and `Clear`
  commands;
- `CanisterCommandResponse::ApplicationSession`;
- `CanisterStatusRequest::ApplicationSession`; and
- `CanisterStatusResponse::ApplicationSession`.

The capability is deliberately not added to the shared public
`RoleCapability` enum: doing so would place its referenced type into disabled
and infrastructure Candid. Exact generated Candid and the role-profile digest
prove this protocol-only capability instead. The established public method
identities remain exactly `canic_command` and `canic_status`.

The endpoint adapter obtains caller and time from the IC, Fleet, role and
generation from protected local state, and issuer, subject and scopes from the
verified proof. The request can only narrow configured scopes and TTL. It
cannot nominate caller, subject, issuer, Fleet, role, generation or an absolute
expiry. Exact retry resolves against the retained canonical request hash before
proof expiry is reconsidered, but only while the exact current session remains
active. Clear is caller-scoped and retains the replay tombstone.

Caller-self status authorizes before lookup and uses this closed precedence:
`Missing`, `Expired`, `StaleFleet`, `StaleRole`, `StaleGeneration`,
`InadmissibleSubject`, then `Active`. The public error remains the existing
compact `record { code : nat16 }` contract.

## Generated artifact proof

Both artifacts below were built from the same current B4 working tree through
the repository's version-enforcing `canic build` command with the `fast`
profile. Direct `cargo build --target wasm32-unknown-unknown` was rejected by
the canonical builder guard, as intended.

| Role | Configuration | Raw Wasm SHA-256 | Gzip Wasm SHA-256 | Candid SHA-256 | Result |
| --- | --- | --- | --- | --- | --- |
| `delegation_root_stub/issuer` | verifier plus explicit local application authorization | `b7ff6f0f5fcb5ec71464e3a02e58d393ba6ece32c0a25cdd0776929382f7d46a` | `9edee1222ebb59790200aa4e4f9e9d1679b7006c4b43d854198b1ed42bff20c2` | `cd59ff029bc0bef7b9b5f02ad1234b62b6a85ec334e4d2783dbaa19d72362f32` | exact application-session types and nested command/status variants present; no new method |
| `test/user_shard` | verifier-only, capability omitted | `bb85c47813b7e33c03aee4378dff20a8b307b7ffc7b2e4478e6510637f5f8556` | `fe1b82cf8186a799dece44eade4ba693bba5315ec55bcfaf2d0e374a2c0f6477` | `5a0cd29add1a66aa4f1312b5c8ebcfbe83db123467eea5fac3c190ee7b89e602` | `canic_command` and `canic_status` remain; every application-session type and variant is absent |

The focused PEM-backed native-agent PocketIC journey uses the configured
issuer role and passes the exact application boundary: prepare, retrieve and
present one delegated proof, establish a 1,800-second local session, read
caller-self active status, return a byte-identical exact retry, clear it,
observe `Missing`, and reject reuse of the tombstoned proof. The
repository-pinned PocketIC 15.0.0 run completed with one passed test in 24.15
seconds.

## Focused validation

- `cargo test --locked -p canic-core application_session --lib`: 14 passed.
- `cargo test --locked -p canic-core local_application_authorization --lib`:
  four passed.
- `cargo test --locked -p canic --test protocol_surface role_capability_surfaces_are_pruned_at_the_destination_macro -- --exact`:
  one passed.
- `cargo test --locked -p canic-core config::schema::tests::every_checked_in_canic_config_parses_and_validates --lib -- --exact`:
  one passed.
- `cargo test --locked -p canic-tests --test native_agent_delegation pem_backed_native_agent_prepares_retrieves_and_presents_delegated_token -- --test-threads=1 --nocapture`
  against the governed PocketIC server: one passed.
- Canonical focused builds of the configured issuer and verifier-only managed
  roles passed; the same PocketIC build also completed the canonical Root and
  Wasm Store artifacts without Candid drift.

B4 changes runtime and generated role Candid only for explicitly enabled
managed roles. It does not add a method, stable-state generation, lifecycle
owner, timer, dependency or infrastructure protocol. B5 owns the public
synchronous facade and generic consumer convergence.
