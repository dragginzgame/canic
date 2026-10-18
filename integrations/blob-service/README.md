# Canic Blob Service

Endpoint, lifecycle and metrics composition for the independent IC Blob service.
Applications own their Canic App configuration and canister artifacts. IC Blob
owns storage, tenant policy, accounting and provider behavior.

This package is published at 0.111.0 with the coherent Canic family
([#33](https://github.com/dragginzgame/canic/issues/33)). It belongs to the main
Canic workspace and inherits its package version, Canic dependency and published
Blob runtime/contracts `0.34` from the root catalog. The upstream
contracts macro owns service method signatures; the adapter supplies Canic
authentication, bounded decoding, caller context and lifecycle composition.
Local paths support
qualification in this checkout; Cargo removes them from the packaged manifest.
Application installation and provider acceptance remain tracked under
[#444](https://github.com/dragginzgame/canic/issues/444).

The coordinated release is owned by
[#459](https://github.com/dragginzgame/canic/issues/459) and
[#444](https://github.com/dragginzgame/canic/issues/444). The adapter shares the
approved Canic release version. The workspace publisher publishes and observes
its Canic dependency before publishing the adapter in the same family:

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

Owned methods use their Contracts-declared Wire or CBOR codecs through explicit
raw Canic entrypoints; only the external provider certificate is Candid. The
shared `ServiceWorkflow` owns dispatch and continuity observations; Canic owns
installation borrowing, lifecycle and memory bootstrap. This pending adoption
selects published Blob 0.34.0, Memory 0.48.2 and Storage 0.5.3 in all
three maintained lockfiles. Installed qualification still requires Canic Core's
Memory caller adoption; dependency resolution alone is not runtime acceptance.

Construct `dto::configuration::ServiceInstallationInput` for the actual allocated
service Principal. Encode the Blob installation as exact CBOR. Embedded applications carry those
bytes in their own envelope. Bind the exact encoded bytes through Canic's production Root
initializer before installation; retries retain those bytes and the member
operation. A dependency change does not supply controller or provider authority.

The governed framework runner qualifies the checked-in dedicated and embedded
consumer workspaces through their production CLI builds and finalized artifacts.
Run the owning case with `make test-pocketic-case` and its exact registered Rust
path under `pic::blob_service::tests`. They are private fixtures and are excluded from the registry
package. Application installation, same-release recovery and provider acceptance
remain the application's responsibility.
