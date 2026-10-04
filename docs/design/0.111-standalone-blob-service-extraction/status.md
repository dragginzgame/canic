# Blob Extraction Implementation Status

Date: 2026-10-04

The maintainer selected this extraction after publishing 0.110.52. The current
scope is the [in-repository hard cut and isolated adapter](0.111-design.md).
This selection does not close FR1 or certify the preceding minor. No release
version has been assigned or changed.

The embedded implementation, command group, billing Medic option, stable
allocations and dedicated main-workspace test/CI lane are removed in the working
tree. Generic feature/descriptor coverage now uses maintained Canic fixtures.
Active guides point to the independent service. Published historical links to
removed sources retain their immutable 0.110.52 snapshot.

The Canic-owned adapter lives in `integrations/blob-service`, outside the normal
workspace. It selects published `ic-blob-storage = 0.14.1`, composes Canic's sole
memory runtime and preserves the independent service's platform-version fence.
The upstream repository source is unchanged. At the maintainer's request, the
remaining Canic adapter composition tests, PocketIC runner, local test mode and
test-only dependencies are removed. The unused generic fixture-helper addition
was also withdrawn; the existing generic framework implementation/tests remain.
Earlier native Clippy passed before this removal. Managed Wasm build qualification
awaits a stable-tree CLI rebuild; the prior attempt crossed concurrent Host/CLI
API edits. No build is active in this session.

The upstream service already owns certificate, authority, restoration, snapshot,
expired-history and release-readback tests. The missing decoder-budget regression
request is [ic-blob-storage#7](https://github.com/dragginzgame/ic-blob-storage/issues/7).
No upstream framework dependency or duplicate issue is requested.

The extraction still requires the remaining targeted source-cut checks, managed
adapter build, dependency/document checks and embedded allocation-peer
qualification. Provider physical deletion, billing cessation, paid uploads and
live retirement are outside this source task. Keep their retained evidence and
obligations intact. This is not yet a push-ready replacement.
