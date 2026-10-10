# Canic Blob Service

Endpoint, lifecycle and metrics composition for the independent IC Blob service.
Applications own their Canic App configuration and canister artifacts. IC Blob
owns storage, tenant policy, accounting and provider behavior.

This package is prepared for its first registry publication. It belongs to the
main Canic workspace and inherits its package version, Canic dependency and
published Blob runtime/contracts `0.22` from the root catalog. Local paths
support qualification in this checkout; Cargo removes them from the packaged
manifest. Package preparation is not registry publication. The local facade uses
Memory 0.35; an aligned published Canic graph is still required under
[#33](https://github.com/dragginzgame/canic/issues/33) before registry-only managed
qualification or adapter delivery.

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
Observe the adapter version on crates.io before asking consumers to remove their
Git dependency. Application acceptance follows in the consuming repository.

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
the artifact’s `canic::memory::memory_allocation_pool!`, invoke `mount!()`, and
compose the synchronous lifecycle participant and metrics sampler into the
application's sole Canic lifecycle. See the
[composition guide](https://github.com/dragginzgame/canic/blob/main/docs/features/blob-storage/README.md).

Construct `dto::configuration::ServiceInstallationInput` for the actual allocated
service Principal. Bind the exact encoded bytes through Canic's production Root
initializer before installation; retries retain those bytes and the member
operation. A dependency change does not supply controller or provider authority.

The checked-in dedicated and embedded consumer workspaces qualify this adapter's
managed composition. They are private fixtures and are excluded from the registry
package. Application installation, same-release recovery and provider acceptance
remain the application's responsibility.
