# Asset Library

Alice and Bob each own a catalog entry for the same content commitment. The
application decides who may inspect, publish or release an entry. Blob tracks
the shared object and the two references. The example registers manifests and
declared sizes locally. It does not upload or download content from a provider.

## Run the two-user walkthrough

From the Canic repository root, prepare the simulator once and run the example:

```sh
make qualification-setup
make asset-library-test
```

The walkthrough builds the Wasm selected by this fleet's `canic.toml` and
installs it in a disposable simulator with two distinct user principals and a
separate manager. Alice publishes reference 1. Bob cannot inspect or remove
Alice’s entry, but can publish his own reference 2 to the same object. After
Alice releases reference 1, Bob still sees one live reference. Replaying Alice’s
original request leaves hers released.

To build the same artifact without starting a simulator:

```sh
canic app build --config apps/examples/asset-library/canic.toml
```

The example uses the [existing catalog actor](../../../qualification/canic-asset-catalog/src/canister/mod.rs)
and its single Storage bootstrap. The catalog admits two users and four lifetime
entries. Released entries continue to occupy a slot. For the conflict,
transaction rollback, drain and upgrade cases, run `make blob-test`. That suite
builds separate qualification Wasms with an explicit fault-injection feature.
The walkthrough checks the example Wasm exports no fault selector.
