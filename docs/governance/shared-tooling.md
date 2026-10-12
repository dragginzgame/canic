# Shared Tooling adoption

The [manifest](../../.shared-tooling.snapshot) records reviewed files from
committed revision `c118062a070a2e238f1eb3989b6fd700f369ce1e` (0.9.2),
with 110 selected files. Explicit entrypoints record the maintained selection;
the exporter derives required companions and verifies ownership before retiring
unused payloads. Refreshes reuse that selection without repeating file arguments.
[AGENTS.md](../../AGENTS.md) retains the product overlay. Refresh only through
the upstream distribution helper; verify exact snapshot bytes and executable
modes before release validation. Dirty sibling source is never inherited. The
repository description is reviewed against the README during baseline adoption.
Snapshot verification hashes the inspected files independently, without executing
the checksum helper being verified. The committed governance file list is selected
with its linked guides. Canic's governance adapter first verifies snapshot bytes
and exact file-list membership, then runs the shared link checker on a private
export containing only those files. It enforces file/link closure, not prose.
Exporter integration tests run in Shared Tooling. Canic does not vendor that
upstream test roster; its adoption checks cover actual refresh, snapshot integrity,
governance closure and affected local adapters.

The maintainer approved retaining Canic's framework fixtures and independent
private Blob consumer workspaces as scoped exceptions in [AGENTS.md](../../AGENTS.md).
The Blob adapter belongs to the main workspace.
Application packages already satisfy the standard `apps/` layout. Snapshot
adoption does not move packages or merge independently selected managed graphs;
[#473](https://github.com/dragginzgame/canic/issues/473) records this boundary.

## Repository setup

`make install-tools` explicitly provisions the complete pinned host, IC and common
Rust toolsets through `make/tools.mk`, under `.tools/host/bin`, `.tools/ic/bin`
and `.tools/rust/common/bin`. Host and IC payloads now use the canonical per-user
store shared across checkouts. Repository references bind exact verified bundle
generations; matching pins reuse existing bundles without downloads. Contributors
must rerun `make install-host-tools` and `make install-ic-tools`, then their
offline checks. Prior physical checkout bundles remain retained. Use the same
`SHARED_TOOLING_STORE` setting for setup, checks and failure evidence; see
[local setup](../local-setup.md#shared-executable-storage).
Cargo-built tools now share verified generations keyed by exact compiler, native
environment, package and packaged-lock identities. The canonical build helper
uses a standard tool environment; application build flags and wrappers remain
local. The common Cargo set uses receipts and byte admission; selection activates
only after all three tools pass. Interrupted setup retains completed generations
and the previous complete selection. Contributors must rerun `make install-rust-tools`
and `make rust-tools-check`, then `make install-testkit-server` and
`make testkit-server-check`. Keep `.tools/rust/common/bin` on PATH and use the
selected Testkit installer's returned path. Offline admission requires the managed
source cache. Old local slots and failed attempts remain retained.
`make tools-check` verifies bytes and versions offline.
Make and CI select these paths; direct shell use
needs the export in [INSTALLING.md](../../INSTALLING.md). Ordinary validation
and PocketIC test entrypoints check prepared tools without downloading them.

`ci/ic-tools.tsv` owns common versions and archive digests. Root
`tool-versions.env` retains Canic Cargo/lint pins, Binaryen executable/runtime-library
digests. The shell projection reads the matrix; it defines no independent
version. Testkit owns PocketIC provisioning, offline byte/receipt admission, the
16.x protocol policy and command-scoped server custody. `make install-testkit-server`
explicitly installs the registry CLI through the shared Cargo installer and calls
its setup; `make testkit-server-check` checks both retained selections offline.
The shared Cargo installer selects the published Testkit package directly from
the root lock, validates its source and exact version, and refuses selection
drift before admitting an installation. Canic supplies the package, binary,
profile and lock path without a second parser or pin.
Shared setup propagates lock, preparation, receipt, reference and activation
failures even inside conditional command substitutions. A failed setup returns
nonzero without publishing a success path or removing another owner's lock.
Mandatory installer and fixture assertions explicitly stop on failure, including
under Bash 3.2; failed checks retain their original evidence. Canic's custom hook
and Make-recipe fixtures follow the same rule.
Ordinary validation never installs these tools. The five-tool IC bundle must be
prepared again through `make install-ic-tools`; old bundles and receipts are retained.
Standard release preflight prepares the admitted locked cache, then invokes
`make install-tools` and the offline
`make tools-check` admission before version preparation or
complete validation. Standalone `make validate` also starts with those offline
checks. Testkit selection follows the held root lock; setup preserves older installations.
Host exact tool admission and packaged
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

Routine CI uses the workflow/ref concurrency group and cancels superseded runs,
retaining all required native hosts on the newest run. Cancelled runs do not
qualify their source; publication and deployment retain their own effect scope.

The canonical `run-sccache.sh` owns binary selection, protected persistent runtime
paths and the server socket. Canic's `run-canic-sccache.sh` projects its explicit
binary override and retains quiet compiler fallback for classified cache failures;
cache-management commands preserve the shared launcher's result. Make, CI and
direct test entrypoints select that adapter. The canonical disk-space checker
owns filesystem admission and explicitly selected diagnostic paths; CI supplies
the workspace and Cargo cache paths without a second local scanner.


Canic includes `make/consumer.mk` once, binding its runtime root to this checkout.
The common owner supplies setup, formatting, hook activation, snapshot verification
and release routing. Paired formatter commands retain Canic's derive ordering and
both private Blob consumer roots without replacing shared recipes. Canic's custom
hook remains product-owned; its unused copy of the upstream Cargo installer test
is retired, while the owning hook and adapter fixtures remain. The selected
`cargo-sort-derives` 0.14.1 is installed and admitted offline.
Make admission retains independent `MFLAGS`
evidence when callers replace `MAKEFLAGS`. The canonical formatting reporter wraps
Canic's formatter order and existing workspace catalog, printing one success line
or a failure with its retained log. The CI evidence action retains those logs.
The updated CI inspector reports failed-log retrieval errors rather than treating
an unavailable log as a successful failure review.

Release dispatch, the Canic validation logger, formatting hooks and hook installation
use the canonical Make-execution guard. It qualifies actual recipe execution and
failure propagation, refusing ignore-errors, dry-run, question, touch and
version-only modes before consumer effects. The canonical validation logger now
owns retained success/failure logs, timing rows, literal structured-event matching
and nested snapshot routing. The thin `run-canic-validation-targets.sh` adapter
selects Canic's `[CANIC-TEST:E001]` identity, retained-log default and terminal colors;
configuration uses the shared `VALIDATION_*` fields. Its actual Make wiring has
one product-specific fixture. Generic logger regressions belong to Shared Tooling;
the overlapping local suite is removed.

`make cloc` uses the canonical metadata-derived workspace-member reporter through
a thin Canic JSON-tool projection that preserves `JQ_BIN` and prepared-tool selection;
`apps/`, `crates/` and framework fixtures share that inventory. Cargo's selected
output directory is excluded from both LOC and test-function counts, including
custom paths and symlink aliases. The canonical fixture now isolates its Cargo
configuration and manifest, including when invoked from the Canic checkout.
The reporter's actual Canic member roster is qualified independently. Reporting
needs explicit prepared `cargo` and `cloc`; direct canonical calls also need `jq`
on PATH. Adoption does not install cloc into the user's environment.

Canic's pin adapter selects existing actionlint, ShellCheck and CI sccache
versions, archive digests and destinations; the shared installer owns download,
verification and installation mechanics. CI and developer setup use that adapter.
Linux installations pass in isolated evidence directories. Native macOS execution
remains upstream CI evidence; local fixtures qualify the four host pin selections.
Shared host setup now owns ripgrep with PCRE2 and cloc; their duplicate Cargo/
package-manager selections are removed. Shared Rust setup owns cargo-sort,
cargo-sort-derives and candid-extractor; Canic's other Cargo/lint tools and
packaged operator-tool admission remain product-owned.

Sibling LOC and tooling inventories run from Shared Tooling. Canic does not
select the optional fleet reporters or their dedicated regression; `make cloc`
retains the local workspace report. The unchanged shared Make include explains
the owner when `cloc-tooling` is requested without that optional selection
([#500](https://github.com/dragginzgame/canic/issues/500)).

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
preflight and validation; earlier evidence is retained. Direct preparation before
staging can restart from a newer clean descendant commit with committed metadata
at the saved previous or candidate version. The runner archives the exact old
intent, selects the requested increment from current metadata and requires fresh
preflight and complete validation. Canic's validation logger retains unique run
directories; prior committed receipts and notes remain in Git. Explicit resume
keeps its saved source and candidate. Staged or delivered intent still requires
exact reconciliation. Identity, payload, destination, unknown remote history and
concurrency conflicts stop the operation. Atomic
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
