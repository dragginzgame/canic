# Canic Blob service adapter changelog

Covers `canic-blob-service`. Independent releases continue its existing 0.114.0
package history; earlier coordinated releases remain in [shared history](../history.md).

## [0.115.0]

### ⚠️ Breaking

- Require Rust 1.99.1 and explicit Storage namespace grants with the supplying
  framework family. Rebuild and clean reinstall dedicated and embedded consumers;
  same-release interruption recovery retains its existing authority.

### 🔧 Changed

- Align the adapter and both private consumer graphs to the framework's Timers
  0.40.3, Metrics 0.29.1/Storage 0.68.0, Logging 0.24.6 and Platform 0.22.0
  selections. Keep one timer provider, Runtime owner and execution guard;
  adopt Blob 0.71.1 and its 0.71.1 contracts, removing the older Metrics 0.28 edge.

- Keep the adapter and both managed consumers on the pinned development/release
  compiler with separate managed Wasm qualification.

- Version and publish the adapter independently from the framework using the
  existing workspace and catalog. Retain its framework requirement and both
  dedicated/embedded consumer locks. CLI admission retains its exact linked
  framework identity.
- Adopt Blob engine and Contracts 0.71.1 with Storage 0.66.2,
  Metrics 0.29.1, Wire 0.16.2 and the matching framework family. Align both
  private consumer locks and explicit Storage namespace grants. Requalify
  installation, recovery and generated clients for dedicated and embedded
  consumers. Earlier five-profile funding evidence keeps its recorded graph;
  deployed provider and application acceptance remain separate.
- Check the service configuration's Engine release against the locked engine,
  keeping the Contracts release independent in installed qualification.
- Open checked Blob keys through the existing host runtime and
  use the sole committed Storage directory. Fresh framework and fixture collections
  explicitly create; retained collections strictly load and enforce their record bounds.
  Preserve lazy allocation and exact chunk-store lease custody during deliberate
  clearing. Add explicit gateway-refresh supersession, stale cancellation
  and reversed held-reply cases for the new contract. The recorded 0.66 installed
  qualification covers dedicated and embedded installation/recovery, all five
  embedded funding profiles and generated tenant clients against both shapes
  ([#622](https://github.com/dragginzgame/canic/issues/622),
  [#494](https://github.com/dragginzgame/canic/issues/494)).
