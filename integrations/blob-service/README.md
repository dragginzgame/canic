# Canic Blob Service

Endpoint, lifecycle and metrics composition for the independent IC Blob service.
Applications own their Canic App configuration and canister artifacts. IC Blob
owns storage, tenant policy, accounting and provider behavior.

This package is prepared for its first registry publication. The registry form
depends on `canic 0.110.53` and published Blob `0.17.2`. Local paths support
qualification in this checkout; Cargo removes them from the packaged manifest. Package preparation is not registry publication.

For a dedicated canister, a consumer-owned `cdylib` shell declares its own
`package.metadata.canic` App and role and invokes:

```rust
canic_blob_service::canister!();
```

For an embedded service, mount the endpoints in a disjoint memory range and
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
