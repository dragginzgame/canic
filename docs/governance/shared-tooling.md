# Shared Tooling adoption

The [manifest](../../.shared-tooling.snapshot) records 41 files from reviewed
committed revision `a37771f1b6b5fc9a88ed6ab3b705bdda35cd8fa3`.
[AGENTS.md](../../AGENTS.md) retains the product overlay. Refresh only through
the upstream distribution helper; verify exact snapshot bytes and executable
modes before release validation. Dirty sibling source is never inherited.

## Repository setup

`make install-tools` explicitly provisions pinned jq/yq and the complete common
IC toolset under `.tools/host/bin` and `.tools/ic/bin`. `make tools-check` verifies
bytes and versions offline. Make and CI select these paths; direct shell use
needs the export in [INSTALLING.md](../../INSTALLING.md). Ordinary validation
and PocketIC test entrypoints check prepared tools without downloading them.

`ci/ic-tools.tsv` owns common versions and archive digests. Root
`tool-versions.env` retains Canic Cargo/lint pins, Binaryen executable/runtime-library
digests and PocketIC executable digests. The shell projection reads the matrix; it defines no independent
version. PocketIC lock/server alignment, Host exact tool admission and packaged
`canic toolchain install` remain product-owned. Shared setup is qualified on
Linux; native macOS CI owns both architectures' actual execution evidence.
The declaration checker runs in CI and release invariants. Maintained independent
workspaces retain their own lockfiles; negative role-contract templates are
materialized only by their owner tests. Existing exact requirements have
[scoped exceptions](../../ci/dependency-pinning-exceptions.json), governed by
[CI policy](ci-deployment.md).

The existing standalone Linux ARM64 ic-wasm archive pin stays Canic-owned because
the common complete toolset excludes that host. It remains install-capable, without
establishing Canic release support or complete-toolset availability there.

## Release recovery

Standard `make release-patch`, `release-minor` and `release-major` use the
[common release contract](../releases.md), with `RELEASE_REMOTE=origin` and
`RELEASE_BRANCH=main`. They are maintainer-owned because they create commits.
Every standard increment uses complete validation; package publication and
artifact cleanup remain separate. Canic retains its validation evidence,
successful/failed logs and tested compiler-cache failure fallback.

Rerunning a normal target selects and reconciles unfinished intent before
computing another increment. Preparation-free failures restart current-source
preflight and validation; earlier evidence is retained. Prepared intent keeps
its exact candidate, source, UTC date and destination. When newer descendant
fixes or another increment are selected, the runner reconciles the older release
first, then validates the next candidate separately. It stops on identity,
payload, destination, unknown remote history and concurrency conflicts.

Late Canic checks read archived metadata and structured validation evidence from
`RELEASE_COMMIT`, which may precede HEAD. Prepared checks retain their current
worktree/index boundary. The exact annotated tag must identify that selected
commit. `make release-resume VERSION=X.Y.Z` remains an explicit recovery entry.
No adoption check creates real commits, tags, pushes or deployment effects.

## Hooks and audits

The Canic-owned hook adapts the reviewed isolated formatter because the shared
hook rejects this checkout's historical audit symlinks. It exports only regular
stage-zero blobs, leaves unrelated symlinks in the real index and rejects selected
symlinks, submodules and unresolved entries. It rejects partial staging and
checks concurrent changes before refreshing selected index entries. The unchanged
shared installer preserves an existing hook configuration. Adoption does not
activate Git configuration; `make install-hooks` remains explicit.

Generic reviews use the [shared methods](../../audits/README.md) with the
[local catalog](../audits/METHODS.md) supplying Canic obligations. Superseded
scored definitions remain historical; changed comparisons are non-comparable.
Adoption is not a fresh product audit or a broad validation gate.

The [adoption walk](../audits/reports/2026-10/2026-10-06/shared-method-adoption.md)
maps the retired obligations and checks a historical report against the new
shared methods without relabeling its evidence.
