# Retained autonomous-management qualification

The profile application is retired as a hands-on demo. The selected consumer is
now the [ongoing demo](demo.md), with a persistent counter and explicit recovery.
The original profile manager/worker remain private qualification fixtures because
they exercise autonomous creation, callback loss, finite authority and state
preservation independently from the simple greeting journey.

## Local regression

After preparing the simulator explicitly, run from the repository root:

```sh
make growth-test
```

The script builds the CLI, the fleet Wasm Store and the profile fixtures. The manager admits an exact published release for each child creation and upgrade. It uses
synthetic cycles. The Storage 0.78 actors passed the installed growth and child
upgrade cases, including retained profile ownership after interrupted installs.

See the [qualification contract](../../qualification/README.md#application-owned-growth-journey)
for interruption, exact replay, expiry, reserve and maintenance assertions. Existing
custody/codec tests remain with `canic-management` and its passive protocol.

## Current mainnet demo

This heading preserves the historical link target. The original manager and
worker identities remain in the [retired profile declaration](../../qualification/fixtures/profile-mainnet.toml),
using the unchanged root journal. Explicit read-only inspection is:

```sh
canic fleet show --config qualification/fixtures/profile-mainnet.toml --manager rk6n2-wyaaa-aaaad-qmoua-cai
```

This is historical fleet inspection, not the new demo. No live controller,
canister, retained operation or spending allowance was changed by moving these
fixtures. Original observations remain in the [qualification history](../qualification/history.md).
