# 0.103 B3 Profile-Pruning Evidence

Date: 2026-08-17

This evidence covers the B3 compile-time protocol boundary only. It does not
authorize B4/B5 command work or pre-implement 0.104's private timer-provider
hard cut.

## Canonical Builds

Each profile was built through the repository-owned `canic-host`
`build_artifact` example with the `fast` profile. The generated DID was read
from the builder-owned `.icp/local/canisters/<role>/<role>.did` output; ignored
build output is not a checked-in authority.

| Profile | Config | DID SHA-256 | Types | Methods | Status selectors |
| --- | --- | --- | ---: | ---: | ---: |
| Runtime-only managed `app` | `apps/demo/canic.toml` | `868247d0aeb1257702e9ee1612407b71d1eefbf6d575cbca61872f81734db69c` | 126 | 19 | 10 |
| Sharding + AutomaticTopup managed `user_hub` | `apps/demo/canic.toml` | `95b8b7eeddfbf75d1e7bff48a261286909a5da6283ce7e8d2bf74b2551961e97` | 138 | 26 | 12 |
| non-signer Fleet Subnet Root | `apps/demo/canic.toml` | `a713a264e2cc1aebbea728c46722a7dec70f9915e8d7bc1814180e11606b82af` | 356 | 112 | 21 |
| signer Fleet Subnet Root | `apps/test/canic.toml` | `da51fde56508c0b54eec0be3704d2a11553c0e32cdb452661f0087939832c04f` | 388 | 121 | 22 |
| built-in Wasm Store | `crates/canic-wasm-store/canic.toml` | `acdea4b02eff9891d031ef36c2f073017dc016d1802b3c430ac2f9aa0137d93c` | 76 | 26 | 7 |

The method totals describe the transitional unreleased artifact, not the final
B7 method ceiling. Old methods with maintained callers remain until their B4
or B5 atomic caller cut; they gain no authority from this report.

## Differential Results

- The Runtime-only app contains neither `Children` nor `CycleTopups` status
  selectors and contains no `CanisterInfo`, `CycleTopupEvent`,
  `canic_canister_children` or `canic_cycle_topups` Candid surface.
- The `user_hub` adds exactly `Children` and `CycleTopups`, their referenced
  Candid types and the still-transitional old methods. This proves the
  positive side of the same two config-derived capability decisions.
- The non-signer Root contains no `RoleAttestation` status selector,
  `RoleAttestationGetRequest`, signed-attestation response type or old
  prepare/get methods. The signer Root contains the request and response
  selector, referenced types and transition methods.
- The built-in Store contains its seven fixed status selectors, cycle balance
  history and no `CycleTopupEvent`, `CycleTopupEventStatus` or
  `canic_cycle_topups` surface.
- Root status envelopes are declared in the destination macro, so Rust cfg
  removes the unavailable request variant, response variant, authorization
  arm and dispatch arm together. The dependency-owned Root DTO module retains
  only operation-detail projections.

The 0.103 boundary freezes the `AutomaticTopup` capability decision and prunes
its Candid DTO and public handler reachability. Existing private timer dispatch
remains config-inert for profiles without a top-up policy; compile-time removal
of that registration, callback and private workflow is the explicit 0.104 B4
owner and is not duplicated here.

## Failure And Composition Evidence

- `cargo test -p canic-core
  role_contract::tests::missing_required_feature_rejects_without_a_contract
  --lib` passed: an unavailable required Cargo feature produces no role
  contract.
- A disposable standalone managed-canister fixture defined
  `async fn canic_status() {}` beside `canic::start!()`. `cargo check` failed
  with Rust `E0428`, ``canic_status` is defined multiple times``. The native
  value-namespace collision occurs before Candid export and is independent of
  macro order; no source scanner or endpoint registry is required.
- `managed_start_remains_a_thin_profile_surface_composer` proves `start!`
  composes lifecycle, ingress and role emitters without owning a status/command
  function, workflow call or inline `await`.
- `role_capability_surfaces_are_pruned_at_the_destination_macro` pins the exact
  cfg ownership for top-up, children and Root attestation transition/public
  surfaces.

## Commands

~~~text
cargo run -p canic-host --example build_artifact -- app fast /home/adam/projects/canic /home/adam/projects/canic apps/demo/canic.toml
cargo run -p canic-host --example build_artifact -- user_hub fast /home/adam/projects/canic /home/adam/projects/canic apps/demo/canic.toml
cargo run -p canic-host --example build_artifact -- root fast /home/adam/projects/canic /home/adam/projects/canic apps/demo/canic.toml
cargo run -p canic-host --example build_artifact -- root fast /home/adam/projects/canic /home/adam/projects/canic apps/test/canic.toml
cargo run -p canic-host --example build_artifact -- wasm_store fast /home/adam/projects/canic /home/adam/projects/canic crates/canic-wasm-store/canic.toml --refresh-canonical-did
cargo check -p canister_root
cargo test -p canic-core role_contract::tests::missing_required_feature_rejects_without_a_contract --lib
cargo test -p canic-core role_contract::tests::automatic_topup_is_derived_only_from_the_exact_configured_role --lib
cargo test -p canic-core role_contract::tests::capability_derivation_is_centralized_for_auth_and_sharding --lib
cargo test -p canic --test protocol_surface wasm_store_exposes_cycle_history_without_automatic_topup_surface
cargo test -p canic --test protocol_surface role_capability_surfaces_are_pruned_at_the_destination_macro
cargo test -p canic --test protocol_surface role_status_dispatchers_keep_variant_specific_authority
cargo test -p canic --test managed_endpoint_gate managed_start_remains_a_thin_profile_surface_composer
~~~

The broad workspace, release matrix and PocketIC suites were deliberately not
run. B3 changes compile-time emission and Candid reachability; the canonical
role builders and focused role-contract/source guards are its direct owners.

