# 0.103 B6 Representative Surface Report

Date: 2026-08-17

This report records the post-B5 generated Candid surface. It supersedes the
transitional method totals in `b3-profile-pruning.md`; the B3 report remains
the compile-time pruning proof and is not a current endpoint authority.

## Canonical Profiles

Configured profiles were built with the repository-owned `canic-host`
`build_artifact` example and its `fast` profile. Coordinator and Store use
their checked-in canonical declarations; Store was refreshed through the same
builder. Generated `.icp` files are ignored build evidence, not independent
authorities.

| Profile | DID SHA-256 | Bytes | Types | Total methods | Canic methods | Commands | Status selectors |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Runtime-only managed `app` | `e3dda55a2df68edbb1bda11dd78fe10c62e8d60c6896c82213dea471ee561150` | 25,395 | 117 | 3 | 2 | 2 | 10 |
| Sharding/top-up managed `user_hub` | `c9d46eb48861c5591dab5de02d066044550383fe8c45f6b5b76b3296c3271523` | 26,300 | 123 | 6 | 2 | 2 | 12 |
| Delegated-token managed `issuer` | `0d6ec4c5d8e2954bfd43a74baf89bfde8993b46104d279f71ecc7b25d96d5987` | 31,576 | 151 | 13 | 2 | 4 | 12 |
| Non-signer Fleet Subnet Root | `04210595c89afc33a69919d9695e67c21455a1590439b54f0414adc326d0eccd` | 74,763 | 282 | 3 | 2 | 31 | 21 |
| Signer Fleet Subnet Root | `0fa86d62bb335ee6289aa9836ab9a2a9d15d3cb254855c0ee6fc8a4fc00af892` | 76,205 | 291 | 6 | 2 | 32 | 22 |
| Fleet Coordinator | `84f147a87bda5c5a359a575de57a7584af070f5a7784f982122f698ebff9f98f` | 29,233 | 97 | 2 | 2 | 9 | 7 |
| Wasm Store | `fbf6250a7bbf5fb9d064e36127b4e373f6fef6cc409d3e507fddde8875abdad4` | 11,957 | 69 | 5 | 4 | 10 | 7 |

`Total methods` includes fixture/application methods and separately mandated
standards. The Canic architectural count is two for every ordinary role and
four for Store: command, status and the two admitted byte lanes. ICRC-10 is an
external standard and does not consume a Canic method slot.

## Exact Service Result

- Runtime-only, sharding/top-up and issuer profiles all emit exactly
  `canic_command` and `canic_status`; only their compiled variants and
  referenced types differ.
- Root and Coordinator emit exactly `canic_command` and `canic_status`.
- Store emits `canic_command`, `canic_status`, `canic_wasm_store_chunk` and
  `canic_wasm_store_publish_chunk` plus external ICRC-10.
- Command and status requests use one flat top-level selector. `Operation`
  responses may select one role-local durable operation detail, but no former
  endpoint family or workflow phase is nested under the command selector.
- Every current generated service is free of the removed standalone Canic
  methods. Fixture methods remain classified as application-owned.

## Commands Run

~~~text
cargo run -p canic-host --example build_artifact -- app fast /home/adam/projects/canic /home/adam/projects/canic apps/demo/canic.toml
cargo run -p canic-host --example build_artifact -- user_hub fast /home/adam/projects/canic /home/adam/projects/canic apps/demo/canic.toml
cargo run -p canic-host --example build_artifact -- issuer fast /home/adam/projects/canic /home/adam/projects/canic canisters/test/delegation_issuer_stub/canic.toml
cargo run -p canic-host --example build_artifact -- root fast /home/adam/projects/canic /home/adam/projects/canic apps/demo/canic.toml
cargo run -p canic-host --example build_artifact -- root fast /home/adam/projects/canic /home/adam/projects/canic apps/test/canic.toml
cargo run -p canic-host --example build_artifact -- wasm_store fast /home/adam/projects/canic /home/adam/projects/canic crates/canic-wasm-store/canic.toml --refresh-canonical-did
~~~

The broad workspace, release matrix and PocketIC suites were not run. Focused
profile builds, exact service inspection and the single Store bootstrap/
reverification journey are the direct B5/B6 owners.
