# Shared Tooling adoption

The [manifest](../../.shared-tooling.snapshot) records 68 files from reviewed
committed revision `25e7ce83149e081e4dcc52c55c33724e44153f2a` (0.1.14).
[AGENTS.md](../../AGENTS.md) retains the product overlay. Refresh only through
the upstream distribution helper; verify exact snapshot bytes and executable
modes before release validation. Dirty sibling source is never inherited. The
repository description is reviewed against the README during baseline adoption.
Snapshot verification hashes the inspected files independently, without executing
the checksum helper being verified. The committed governance file list is selected
with its linked guides. Canic's governance adapter first verifies snapshot bytes
and exact file-list membership, then runs the shared link checker on a private
export containing only those files. It enforces file/link closure, not prose.
The canonical distribution fixture now isolates its own manifest while preserving
the checkout's existing snapshot. Its focused consumer qualification passes;
the earlier failure remains historical evidence in
[Shared Tooling#28](https://github.com/dragginzgame/shared-tooling/issues/28#issuecomment-6032229331).

The maintainer approved retaining Canic's framework fixtures and independent
blob root-package workspaces as scoped exceptions in [AGENTS.md](../../AGENTS.md).
Application packages already satisfy the standard `apps/` layout. Snapshot
adoption does not move packages or merge independently selected managed graphs;
[#473](https://github.com/dragginzgame/canic/issues/473) records this boundary.

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

## CI and verification helpers

Release dispatch, the Canic validation logger, formatting hooks and hook installation
use the canonical Make-execution guard. It qualifies actual recipe execution and
failure propagation, refusing ignore-errors, dry-run, question, touch and
version-only modes before consumer effects. Canic's logger keeps retained success
logs, timing rows and structured failure events pending
[shared convergence](https://github.com/dragginzgame/shared-tooling/issues/37).

`make cloc` uses the canonical metadata-derived workspace-member reporter through
a thin Canic JSON-tool projection that preserves `JQ_BIN` and prepared-tool selection;
`apps/`, `crates/` and framework fixtures share that inventory. Cargo's selected
output directory is excluded from both LOC and test-function counts, including
custom paths. The unchanged shared fixture passes from a neutral directory;
invoking it within Canic inherits Canic's Cargo configuration and exposes the
[fixture isolation limitation](https://github.com/dragginzgame/shared-tooling/issues/31#issuecomment-6033151305).
The reporter's actual Canic member roster is qualified independently. Reporting
needs explicit prepared `cargo` and `cloc`; direct canonical calls also need `jq`
on PATH. Adoption does not install cloc into the user's environment.

Canic's pin adapter selects existing actionlint, ShellCheck and CI sccache
versions, archive digests and destinations; the shared installer owns download,
verification and installation mechanics. CI and developer setup use that adapter.
Linux installations pass in isolated evidence directories. Native macOS execution
remains upstream CI evidence; local fixtures qualify the four host pin selections.
The shared ripgrep installer is not selected; Canic's Cargo-installed pin remains.

The read-only `scripts/dev/gh-ci.sh` helper now comes from the snapshot. Explicit
`--commit HEAD --all-workflows --limit 100` inspects the exact committed source;
listing is bounded evidence, not a complete CI verdict. `--failed` intentionally
searches historical failures. Stub fixtures qualify argument selection, invalid
combinations and failed observations without contacting GitHub.

macOS durability CI selects Canic's maintained publication/recovery tests through
the shared nonempty Cargo-test wrapper, which rejects zero passing tests and
retains failed output. Native Linux execution qualifies the selector; actual
macOS execution remains for the configured CI cells.

`fmt` and `fmt-check` use the shared pinned formatter prerequisite check before
running Canic's existing formatter sequence. Dependency invariants enable shared
Cargo inheritance checks; Canic's Rust manifest tests retain only product role and
artifact policy. The shared Cargo version reader replaces cargo-get. Canic's
adapter selects current source or an archived committed view; release candidate
checks select their own exact commit view.

The portable checksum helper owns current file digest calculation. RustSec
preparation makes a private commit-bound database view, then Canic audits with
`--no-fetch`; Canic retains classification policy and failed evidence. This does
not relabel historical audit evidence or qualify a new advisory result. Tag
maintenance uses the shared Perl utility with an explicit cutoff, exact object
identities and retained retry evidence; see [tag maintenance](../tag-maintenance.md).
No tag deletion is part of adoption qualification.

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
payload, destination, unknown remote history and concurrency conflicts. Atomic
push dispatch uses the captured push URL, with destination checks before intent
and dispatch, rather than resolving the remote name again when pushing.
Focused stub fixtures cover destination drift and lost-response recovery.

Late Canic checks read archived metadata and structured validation evidence from
`RELEASE_COMMIT`, which may precede HEAD. Prepared checks retain their current
worktree/index boundary. The exact annotated tag must identify that selected
commit. `make release-resume VERSION=X.Y.Z` remains an explicit recovery entry.
Cargo metadata selects Canic's owned package roster; the shared lock transformer
rewrites only those local selections into a private candidate. Canic owns the
transaction, source authority, validation receipt, publication boundary and
rollback. Focused toy-workspace execution uses real Cargo and fake Git authority:
receipt creation/replacement, retained undated history, failed synchronization,
partial transformer output and exact rollback all pass. No adoption check creates
real commits, tags, pushes or deployment effects.

## Hooks and audits

The Canic-owned hook adapts the reviewed isolated formatter because the shared
hook rejects this checkout's historical audit symlinks. It exports only regular
stage-zero blobs, leaves unrelated symlinks in the real index and rejects selected
symlinks, submodules and unresolved entries. It rejects partial staging and
checks concurrent changes before refreshing selected index entries. The unchanged
shared installer preserves an existing hook configuration. Adoption does not
activate Git configuration; `make install-hooks` remains explicit. Its fixture
stays Canic-owned as well: the newer shared whole-checkout hook checker rejects
historical symlinks before reaching the product adapter. Canic's isolated-index
regressions pass; adoption does not claim that shared checker passed here.
[Shared Tooling#33](https://github.com/dragginzgame/shared-tooling/issues/33) owns
the consumer checker limitation; [Canic#461](https://github.com/dragginzgame/canic/issues/461)
owns qualification.

Generic reviews use the [shared methods](../../audits/README.md) with the
[local catalog](../audits/METHODS.md) supplying Canic obligations. Superseded
scored definitions remain historical; changed comparisons are non-comparable.
Adoption is not a fresh product audit or a broad validation gate.

The [adoption walk](../audits/reports/2026-10/2026-10-06/shared-method-adoption.md)
maps the retired obligations and checks a historical report against the new
shared methods without relabeling its evidence.
