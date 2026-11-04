# canic-tests

Private, unpublished Canic test package. Its library owns the shared harness:
Root baselines, topology and lifecycle fixtures, embedded allocation peers,
audit probes and reference-canister setup. Integration suites remain separate
Cargo test targets under `tests/`; maintenance tools remain separate examples.
Downstream applications use the reusable `ic-testkit` instead of this
repository-specific harness.

Ordinary native tests use normal library and integration-target discovery.
`governed-pocketic-tests` adds the stateful library catalogue for the governed
PocketIC runner. Source-bound activation recovery still precedes its isolated
workers; exact single-case selection remains serial.

Select an ordinary integration target with:

```sh
cargo test --locked -p canic-tests --test fixture_payloads
```

Select an installed integration target with:

```sh
make test-pocketic-case CASE=timer_authority
```

Individual registered library cases remain selectable by their exact `pic::…`
path through the same governed command. The `verify_embedded_root` and
`refresh_embedded_root` examples retain their separate Cargo selections, as does
the native `wire_composition` codec/schema qualification example.

- [Testing rules and selectors](../../TESTING.md)
- [Test canisters](../../canisters/README.md)
- [Documentation](../../docs/README.md)
