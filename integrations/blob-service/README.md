# Canic Blob Service

Endpoint, lifecycle and metrics composition for the independent IC Blob service.
Applications own their Canic App configuration and canister artifacts. IC Blob
owns storage, tenant policy, accounting and provider behavior.

This package belongs to the main Canic workspace and inherits its package
version, Canic dependency and published Blob runtime/contracts requirement from
the [root catalog](../../Cargo.toml). See the [current handoff](../../docs/status/current.md)
for qualified versions; pending checkout changes do not describe older published adapters.
The upstream contracts macro owns service method signatures; the adapter supplies
Canic authentication, bounded decoding, caller context and lifecycle composition.
Local paths support qualification in this checkout; Cargo removes them from the
packaged manifest. Application installation and provider acceptance remain tracked
under [#444](https://github.com/dragginzgame/canic/issues/444).

The workspace publisher publishes and observes the matching Canic dependency
before publishing the adapter in the same family:

```sh
make publish
```

Run this from the Canic repository root after the governed release checks.
Cargo's normal package verification must remain enabled.
Consumers can select the published adapter and matching Canic family from
crates.io. Application acceptance follows in the consuming repository.

The minimum supported Rust version is 1.91.0, including the checked-in dedicated
and embedded managed consumers. Checkout maintenance uses the pinned Rust 1.99
toolchain. Minimum-version qualification also needs the
`wasm32-unknown-unknown` target for 1.91.0 and Canic's managed build tools; build
consumer canister artifacts through `canic build`. The complete managed 1.91
qualification includes Canic's current role-specific descriptor-gating fix
([#495](https://github.com/dragginzgame/canic/issues/495)); older published
generated tooling is not retroactively qualified by it.

For a dedicated canister, a consumer-owned `cdylib` shell declares its own
`package.metadata.canic` App and role and invokes:

```rust
canic_blob_service::canister!();
```

For an embedded service, grant `MEMORY_AUTHORITY` / `MEMORY_KEY_PREFIX` in
the artifact’s explicit `canic::memory::MemoryBootstrap`, extend its requests
with `canic_blob_service::memory_requests()`, invoke `mount!()`, and
compose the synchronous lifecycle participant and metrics sampler into the
application's sole Canic lifecycle. See the
[composition guide](https://github.com/dragginzgame/canic/blob/main/docs/features/blob-storage/README.md).

Construct `dto::configuration::ServiceInstallationInput` for the actual allocated
service Principal and encode it with `ic_blob_storage_contracts::wire::encode_request("blob_install", input)`.
The dedicated nested payload is a method-bound Wire frame; an embedded application owns its outer
envelope and carries the same Blob bytes. Bind the exact encoded bytes through Canic's production Root
initializer before installation; retries retain those bytes and the member
operation. A dependency change does not supply controller or provider authority.

The governed framework runner qualifies the checked-in dedicated and embedded
consumer workspaces through their production CLI builds and finalized artifacts.
Run the owning case with `make test-pocketic-case` and its exact registered Rust
path under `pic::blob_service::tests`. They are private fixtures and are excluded from the registry
package. Application installation, same-release recovery and provider acceptance
remain the application's responsibility.

Applications may opt into `funding::{prepare, dispatch, renew}`
using upstream `ic-blob-storage` types. These functions bind the shared workflows
to the installed journal and actual IC caller/service; they mount no endpoint or
automatic schedule. The caller must be the installed operator. Applications own
trusted current provider/account/activity/liability evidence; unknown fields
block payment. Never accept that evidence as ingress assertions. Lost replies
require inspecting the original operation, not resending payment. The adapter
does not expose independent-credit reconciliation. Uncertain accepted payments
retain their original obligations and renewal refusals. See the
[funding integration guide](https://github.com/dragginzgame/canic/blob/main/docs/features/blob-storage/README.md#opt-in-application-funding)
for responsibilities and the remaining paid-provider qualification.

## Wire adapter ownership

This crate supplies Canic mounting, access/lifecycle integration and compiled
artifact declarations. Blob owns endpoint schemas, codecs and scalar semantics.
Its native schema API also supplies the browser decimal-validation adapter;
Canic must not copy that validator or infer schemas from raw handler signatures.

For combined native client generation, Canic Contracts supplies
`ids::wire_schema::TYPESCRIPT_ADAPTER` alongside its identity validators.
Blob's `wire::schema::runtime_adapter("./adapter.js")` can delegate to that
adapter, which preserves Blob's supplied checks and adds Canic's identity checks.
Attempts to replace the Canic checks refuse. Both use the same published Wire
runtime; neither adapter performs transport or authorization.

The `canic-testing-internal` example `wire_composition` qualifies a combined
catalog using actual Canic and Blob types, Rust frame vectors, strict generated
TypeScript and Node checks. It is native test tooling, not the unfinished managed
schema/release/frontend pipeline. Normal canister graphs exclude generation.

Enable this adapter's `wire-schema` feature only in a native schema producer.
`mount!` then registers Blob's published request/reply projections through Canic's
shared provider registry, alongside typed application methods. Snapshot admission
checks the combined registry against declarations read from finalized Wasm;
duplicate names/IDs, invalid schema references and changed budgets refuse.
The feature is rejected for Wasm and is absent from normal deployed graphs.
This registration does not yet write a managed sidecar or bind its release hash.

The dedicated consumer's native `mounted_supplier_schema_matches_final_wasm`
test accepts `CANIC_BLOB_SCHEMA_WASM` pointing at the managed final artifact. It
also compares every supplier payload projection with Blob's own schema output.

The dedicated consumer's native `wire_schema` example links its mounted library
and accepts one finalized Wasm path, emitting schema JSON on stdout. Its parser
and JSON dependencies are native development dependencies; normal Wasm graphs do
not acquire them. `canic_host::canister_build::produce_wire_schema` runs a selected
native producer with bounded capture, checks final exports/contracts, and retains
the exact output through IC Build's transaction. A failed producer leaves the
previous materialized sidecar intact. Raw producer output alone is not release
admission. The explicit Host `build_workspace_wire_schema` entrypoint can now compile the
configured role's example with its runtime profile/features and a native-only
`wire-schema` addition, then acquire the output with a Cargo source snapshot.
It selects the actual executable from Cargo records and owns the build lock
through acquisition. Normal artifact-build invocation and release hash binding
remain unfinished; this example is a private framework qualification consumer.
