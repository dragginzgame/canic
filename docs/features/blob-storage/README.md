# Blob Service Composition

Application blob storage belongs to [ic-blob-storage](https://github.com/dragginzgame/ic-blob-storage).
It owns content, tenant authority, provider access, references and accounting.
Canic owns Fleet lifecycle and deployment of ordinary application Components.

The Canic-owned adapter lives in a separate Cargo workspace at
`integrations/blob-service`. Its service dependency and qualification are outside
the main Canic workspace and ordinary release test lane. The upstream library has
no Canic dependency. See the [extraction design](../../design/0.111-standalone-blob-service-extraction/0.111-design.md)
and [current handoff](../../status/current.md) for implementation evidence.

The adapter pins the published `ic-blob-storage` library and uses public Canic
endpoints, bounded decoders and synchronous lifecycle participants. It registers
its service memory grants before Canic bootstraps the sole memory runtime. Its
managed lifecycle retains Fleet admission.

Blob-specific tests live upstream. This workspace contains no blob test harness,
PocketIC runner or standalone test mode. Canic retains its generic endpoint,
Fleet, lifecycle and memory tests. The upstream service suite owns blob authority,
certificate replies, restoration, provider behavior and accounting; those results
do not establish deployment qualification for an arbitrary wrapper.

Build the adapter explicitly through the current Canic CLI, outside ordinary
Canic release validation:

```sh
cargo build --locked -p canic-cli --bin canic
cd integrations/blob-service
../../target/debug/canic build blob-service blob --workspace . --config canic.toml --icp-root . --profile fast --json
```

Installation supplies the library's Candid `ServiceInstallationInput` as Canic's
nested application argument bytes. Its configured service Principal must be the
actual canister ID. This source adapter has not yet completed managed Wasm build
qualification; source removal alone does not qualify a live installation.

The source cut removes Canic's embedded blob runtime, feature flags, billing
commands and passive Medic inspection. Use the service's own operator tools for
its maintained API. Its status response does not promise the former readiness
exit code, and there is no qualified replacement for Canic's direct funding
command. Neither limitation requires retaining the old runtime.

Reinstalling or deleting a live service is a separate operation. Preserve records
of outstanding provider liabilities; logical reference release does not prove
physical deletion or billing cessation. The source extraction performs no paid
upload, provider cleanup, deployment, or reset.
