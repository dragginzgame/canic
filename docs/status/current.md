# Current handoff — 2026-10-07

The six reported Clippy E0308 errors are repaired under
[#450](https://github.com/dragginzgame/canic/issues/450). Selected IC Testkit 0.21.0
returns a fallible Cargo-discovered workspace root. All affected callers in
Internal Testing and Canic Tests now explicitly require successful discovery,
including governed/test-only cases missed by ordinary library compilation.
No path fallback, compatibility wrapper or dependency change was added.

Warning-denied Clippy passes for both affected packages with all targets and all
features, including governed case compilation. Offline Cargo discovery resolves
both packages to the Canic root; scoped rustfmt and diff hygiene pass. This is
compile/lint evidence, not execution of PocketIC journeys or broad validation.
Logs remain under `target/review-validation/testkit-workspace-result-20261007/`.
Fixture propagation and this handoff remain uncommitted on maintainer-owned base
`900ef517695a14423764910e22ae7c22af4a41a3`. Routine fixture-only changes add no
changelog entry. Manifest SHA-256 remains
`0113b7c6deb72aea98f80dabbe8d244b46b19db894f352b5076969cdeb5c1962`;
lock SHA-256 remains
`8f44021d2c12aa741bb952d663f605bd6ef3093c7786364106b770a2cef9cdd5`.
No Git commit, version, release or publication ran. Whole-batch push/publication
readiness remains subject to the recorded managed and minor-boundary findings.

## Earlier nested-Make fixture qualification — 2026-10-07

The maintainer's nested release-integrity failure is repaired under
[#450](https://github.com/dragginzgame/canic/issues/450). The new preflight fixture
inherited GNU Make release assignments, which overrode its private version and
caused correct source admission to refuse before draft/Cargo events. It now clears
the parent Make control variables before private invocations. Its owning gate
deliberately supplies conflicting release values to retain this regression.

The complete focused release-integrity gate passes with inherited
`RELEASE_PREVIOUS=0.110.52`, `RELEASE_KIND=patch`, `RELEASE_VERSION=0.110.53` and
`CARGO_NET_OFFLINE=true`. Separate conflicting-value and dry-run/injected-Makefile
checks, scoped ShellCheck, Bash syntax and diff hygiene pass. Logs remain under
`target/review-validation/preflight-make-context-20261007/`; the original failed
fixture is preserved. Only the fixture, its gate invocation and this handoff
change, uncommitted on maintainer-owned base
`6a4020a72002c857614d7198877df4c17c628a55`. Routine fixture-only work adds no
changelog entry. Production cache preparation and caller offline policy are
unchanged; no broad validation, actual release, Git commit or publication ran.
This repair does not establish whole-batch push/publication readiness under the
existing managed-alignment and minor-boundary findings below. Earlier Host
qualification remains bound to its recorded graph, rather than the newer graph
committed by the maintainer.

## Earlier bounded-capture qualification — 2026-10-07

The release preflight cache defect is implemented and closed in
[#486](https://github.com/dragginzgame/canic/issues/486). The adapter prepares the
selected cache with `cargo fetch --locked` after source/draft/tool admission,
preserves explicit Cargo offline policy, and leaves validation offline. The new
private regression is registered in the release-integrity gate. That complete
focused gate passes, including cold/warm cache, refusal, phase-order and unchanged
manifest/lock fixture evidence. No actual release or registry publication ran.

Further Host cleanup under [#458](https://github.com/dragginzgame/canic/issues/458)
delegates process-evidence collection to the bounded stream reader, preserving
procfs opens and unknown/UTF-8 policy. Compiled Candid now uses shared bounded
capture: 16 MiB stdout, 64 KiB diagnostics and a 120-second deadline. Original
typed extraction errors and cache/environment/source checks remain. All 11
selected build-lock tests and 12 Candid/cache tests pass. The separately selected
installed-extractor case also passes: fresh, reused and parallel declarations
match. It uses one synthetic Wasm declaration, not a managed canister journey.
Host all-target/all-feature warning-denied Clippy, scoped rustfmt, ShellCheck,
Bash syntax and diff hygiene pass. Logs remain under
`target/review-validation/cache-reader-followup-20261007/`.

These changes are uncommitted on maintainer-owned base
`b420704efd60835583871e1b98d072c5f5e45c92`, which includes the preceding named-output
cleanup. Both 0.110.53 draft changelog views are updated; workspace version remains
0.110.52. Manifest SHA-256 is
`4bb3ebb4f2dd3f78a7bd9d64996526c15a7b4f6ed898bb4ba4eef124390caa47`;
lock SHA-256 is
`be7245a7fea9cb6a869252062bac9f0673c8d0bb1cdc64f3848f37c73b952bf9`.
Neither selection changed. No broad validation, Git commit, version, publication
or deployment ran. Existing [#444](https://github.com/dragginzgame/canic/issues/444)
managed alignment and [#459](https://github.com/dragginzgame/canic/issues/459)
human-owned minor boundary still prevent whole-batch push/publication readiness.

## Earlier named-output qualification — 2026-10-07

Host 0.4.1 cleanup is implemented under
[#458](https://github.com/dragginzgame/canic/issues/458). Shrink and optimization
delegate staging, cleanup and durable replacement to `write_named_with`, with
bounded Wasm validation before success and original producer/publication errors
retained. Canic still owns optimizer contract checks and artifact-set
qualification. Observatory rendering uses the shared bounded writer; its private
writer is removed. The fs dependency minimum is 0.4.1, and Cargo.lock is unchanged.
Both open changelog views include the cleanup; package versions remain unchanged.

All 26 selected artifact tests and 36 Observatory tests pass, including missing,
malformed and oversized producer output, original-artifact preservation, contract
drift, exact rendering budgets and byte identity. Host all-target/all-feature
warning-denied Clippy, scoped rustfmt, dependency declarations/inheritance and
diff hygiene pass. A real installed ic-wasm 0.11.1/Binaryen 132 probe confirms
precreated-inode writes and Wasm output at spaced paths. This is focused Linux
qualification; process fixtures are substitutes and no native macOS, managed
journey or broad gate ran. Logs remain under
`target/review-validation/host-named-output-20261007/`.

The preceding fixture repairs were committed by the maintainer in
`ab7227f0acb216fdabdceb76fd134c7a369d2def` during this cleanup. Current cleanup
changes remain uncommitted. Manifest SHA-256 is
`4bb3ebb4f2dd3f78a7bd9d64996526c15a7b4f6ed898bb4ba4eef124390caa47`;
lock SHA-256 is
`be7245a7fea9cb6a869252062bac9f0673c8d0bb1cdc64f3848f37c73b952bf9`.
Read-only sibling reuse feedback and qualification limits are recorded in
[Host #8](https://github.com/dragginzgame/ic-host-tooling/issues/8#issuecomment-6037635473).
The complete release batch remains unready for push/publication under the existing
managed-stack and minor-boundary findings below; no release effects ran.

## Earlier release-fixture qualification — 2026-10-07

The maintainer's release-integrity failure is repaired under
[#450](https://github.com/dragginzgame/canic/issues/450). Development and standard
release-entry fixtures now include the actual Makefile and its shared records;
the development ripgrep stub uses the current repository-local path. The
authority corruption fixture checks a maintained Canic-owned version pin.
The complete focused `make release-integrity-contract-gate`, scoped ShellCheck,
syntax checks and diff hygiene pass. Final evidence remains at
`target/review-validation/release-fixture-20261007/gate-final.log`.
Only local test scripts and this handoff change; routine fixture-only work adds
no changelog entry. Changes remain uncommitted. No broad validation or actual
release/publication effects ran, and the inherited dirty lock is preserved.
The separate cold-cache preflight defect remains owned by
[#486](https://github.com/dragginzgame/canic/issues/486).

A read-only review of Host 0.4.1 records named-output writer reuse candidates in
[#458](https://github.com/dragginzgame/canic/issues/458#issuecomment-6037236922).
The current lock already selects its four direct packages; this review does not
qualify runtime behavior on that graph. Manifest SHA-256 remains
`bda220b51c2aa13a7e683311c7b3ad1fb06bb100a2580402ea3ac83377609963`;
lock SHA-256 remains
`852aab7a13f5fc9c6a6a2c5b7f031f586745b80e77d8d2d7a873044ec2596728`.
Earlier managed-stack and minor-boundary findings below still prevent declaring
the complete batch push/publication ready.

## Earlier membership qualification — 2026-10-07

Both remaining initial-membership regressions in
[#457](https://github.com/dragginzgame/canic/issues/457) now pass. The inherited
fixture change uses normal Host observation pacing while establishing the source
Fleet, allowing scheduled lifecycle retries to run before the finite stalled-
observation budget expires. Production retry policy and observation limits are
unchanged. Retained traces show temporary ComponentMembership/E140 responses
followed by successful membership and working-Fleet preparation.

The exact governed PocketIC cases
`pic::fleet_registry::baseline::tests::generated_reinstall_recovers_lost_install_and_reaches_working_fleet`
and
`pic::fleet_registry::baseline::tests::completed_reset::completed_estate_reset_recovers_and_replays`
pass end to end in 436.54s and 232.93s. They qualify real installation, lost replies,
interrupted reset recovery, terminal conservation and effect-free replay. Both
inventory preflights and normal invocation-owned server/scratch cleanup pass.
Logs remain under `target/test-runs/20261007T104828Z-48662.ZdEuZq/` and
`target/test-runs/20261007T105819Z-118038.5FJlmQ/`. Warning-denied Clippy passes
for Internal Testing's all-target/all-feature selection; scoped rustfmt passes.
Its lint log remains under `target/review-validation/membership-pacing-20261007/`.

This qualification uses the later retained graph: Memory 0.31.0, Metrics 0.2.7,
Timers 0.14.8, Query 0.47.9, Testkit 0.20.0 and direct Host 0.4.0. Manifest SHA-256
is `bda220b51c2aa13a7e683311c7b3ad1fb06bb100a2580402ea3ac83377609963`;
lock SHA-256 is `79bbf0dc318297f3a28a449e8d54f5dbc8cd05e2a88aa565767ce4eab85873bb`.
Those inputs and the affected fixture source stayed unchanged through both cases
and lint. These two managed journeys do not requalify the embedded peer, independent
blob consumers, native macOS or the complete workspace. Routine fixture-only
qualification adds no changelog entry. Existing dirty work remains uncommitted;
no broad gate, version, publication or deployment ran. The managed Blob alignment
in [#444](https://github.com/dragginzgame/canic/issues/444) and the human-owned minor
boundary in [#459](https://github.com/dragginzgame/canic/issues/459) still prevent
declaring the complete batch push/publication ready; the selected 0.110.53 draft
stays. Published Blob 0.16.0 is now available for Memory 0.30, but its `^0.30`
requirement cannot unify with this checkout's Memory 0.31. The independent adapter
still selects Blob 0.15.2 and Memory 0.28; this continuation did not change it or
its locks. The [upstream alignment record](https://github.com/dragginzgame/ic-blob-storage/issues/20)
and #444 own that remaining composition boundary.

## Earlier native and embedded-peer qualification — 2026-10-07

The latest ordinary native validation failures are repaired under
[#450](https://github.com/dragginzgame/canic/issues/450). Three release-receipt
tests had obsolete Cargo stubs and invalid empty metadata/comment-only lock
inputs; their private fixture now uses a valid inherited-version package and
real offline Cargo discovery/metadata. Eight downstream golden values had missed
the current `application_init_required` Component Spec identity hard cut in
`335f7b02b8999723e33e3f823db9b3256ca31af6`; the configuration, provisioning,
registry, receipt and artifact vectors now bind the maintained contract. Existing
rejection, canonical ordering and rollback assertions remain, and configuration
digest sensitivity directly covers the initializer requirement.

All 11 reported failures pass within 75 selected native regressions (three receipt,
63 Core and nine Host tests). Warning-denied Clippy passes for Core/Host all
targets/features and Canic's affected release-flow target with all features.
Independent embedded-peer verification also passes without rewriting the peer or
its provenance. The manifest and lock hashes remain unchanged. Logs are retained
under `target/review-validation/validation-fixture-propagation-20261007/`.
Only tests and this handoff change; routine test-only fixes do not add a changelog
entry. These results are focused native qualification; no new managed run or broad
gate ran. Changes remain uncommitted. Existing managed-stack and minor-boundary
blockers still prevent declaring the complete batch push/publication ready.

The repeated embedded allocation-peer preflight failure is repaired for the
maintainer's new registry stack: Memory 0.30.0, Metrics 0.2.5, Timers 0.14.6,
Host 0.3.2, Query 0.47.7 and Testkit 0.20.0. Explicit regeneration with the new
Testkit producer and independent `verify_embedded_root` pass. Primary lock
SHA-256 is `104254fce8363bd75103194fdffd55c10ad3c5fa872131c22c8baab667c52518`;
the refreshed peer SHA-256 is
`1cfcad34cd22ce59e1b73953823c30031ffeb28ba07a3c917917266666069c15`.
The peer and provenance are updated together; verification remains read-only.
The exact governed PocketIC case
`pic::lifecycle::tests::published_managed_component_group_support_drives_child_lifecycle`
passes: real installation, lifecycle restoration and authorization refusals.
The case took 163.60s; the targeted runner completed in 376s, including fresh
framework canister builds and normal PocketIC cleanup. Logs remain under
`target/review-validation/embedded-peer-current-20261007/`.
[#450](https://github.com/dragginzgame/canic/issues/450) owns the result. Staged
dependency changes are preserved. No broad validation or release effects ran;
this focused repair does not establish complete-batch push/publication readiness.

## Earlier qualification — 2026-10-07

Host's stale Wasm code-section ceiling is corrected from 10 MiB to the current
documented 12 MiB, including Local builds. Pre-publication refusal still protects
the previous artifact set; the warning now starts at 11.25 MiB. All 25 targeted
`artifact_io::` native tests pass, including explicit 12 MiB acceptance and
one-byte-over refusal with previous Wasm/Candid/gzip preservation. Scoped rustfmt,
diff hygiene and the changed documentation's local links pass. This result uses
the current Memory 0.30.0 / Metrics 0.2.4 / Timers 0.14.6 root graph below, whose
manifest and lock hashes remained unchanged during qualification. Synthetic
Wasm/tool fixtures qualify Host admission, not managed installation at the new
ceiling. [#476](https://github.com/dragginzgame/canic/issues/476) owns the correction;
logs remain under `target/review-validation/wasm-code-limit-20261007/`.
Earlier footprint evidence retains its dated limits. The complete batch's
managed-stack and minor-boundary blockers remain; no release effects ran.

After the focused adoption qualification below, a concurrent dependency update
selected Memory 0.30.0, Metrics 0.2.4 and Timers 0.14.6. It is preserved. Targeted
Host `cargo check --locked --offline` passes with root lock SHA-256
`7932e45632560aa4eeefe30349410e4c280c7cfec58567cdc7e4a84cc266578f`;
this later selection has compile-only evidence, not the earlier native/managed
qualification. All three independent blob locks refuse locked Wasm metadata.
Published Blob Storage 0.15.3 and the current adapter still require Memory 0.28,
which cannot collapse into the new 0.30 identity. Published service alignment,
adapter/lock updates and renewed managed qualification are owned by
[Blob Storage#20](https://github.com/dragginzgame/ic-blob-storage/issues/20) and
[Canic#444](https://github.com/dragginzgame/canic/issues/444).

Shared Tooling now selects reviewed committed 0.1.14,
`25e7ce83149e081e4dcc52c55c33724e44153f2a`; all 68 canonical files and governance
link closure verify. The maintainer explicitly retained `apps/`, the framework
fixtures under `canisters/{audit,sandbox,test}` and the three independent blob
root-package workspaces. [AGENTS.md](../../AGENTS.md) records the scoped exceptions;
no package moved or dependency graph merged
([#473](https://github.com/dragginzgame/canic/issues/473#issuecomment-6033246970)).
The shared Make-execution guard protects release dispatch, the validation runner
and isolated hook. The canonical LOC reporter inventories all Cargo members;
only the JSON-tool projection remains Canic-owned. Hook, validation runner, release
recovery and snapshot-distribution fixtures, actual metadata/LOC roster equality,
explicit JSON-tool admission/refusal and scoped ShellCheck pass. The unchanged
LOC fixture passes from a neutral directory; its Canic configuration-isolation
failure remains reported upstream. Exact upstream 0.1.14 CI passes its configured
matrix. These are native/tooling results, not Canic macOS or managed qualification.
Results and remaining shared reuse boundaries belong to
[#461](https://github.com/dragginzgame/canic/issues/461#issuecomment-6033246207),
[Shared Tooling#31](https://github.com/dragginzgame/shared-tooling/issues/31#issuecomment-6033151305),
[#33](https://github.com/dragginzgame/shared-tooling/issues/33) and
[#37](https://github.com/dragginzgame/shared-tooling/issues/37).

Published IC Host Tooling 0.3.1 now owns ICP JSON/hex envelopes, Backup descriptor
hashing/copy, advisory missing-path observations, bounded lock/Observatory reads
and streamed file evidence. Canic retains Candid/rejection policy, private tree
framing, publication recovery and paid-effect custody. Public response-error
callers adopt `IcpJsonResponseError::Envelope(ResponseError)`; the three local
JSON/hex variants and private parser are removed without a shim. Focused Linux
results: 31 Host boundary tests, 21 CLI projections, eight Backup artifact tests
and five native publication/checksum recovery cases pass. Package-scoped strict
Clippy for Host/CLI/Backup all targets/features passes. These do not qualify live
paid commands or native macOS. This adoption added one inherited Backup dependency
edge without changing external selections; the later concurrent update is separate.
Results and retained process/archive
boundaries are owned by [#458](https://github.com/dragginzgame/canic/issues/458) and
[IC Host Tooling#5](https://github.com/dragginzgame/ic-host-tooling/issues/5#issuecomment-6033152454).

The embedded allocation peer was explicitly refreshed and independently verified
against the preceding qualified lock SHA-256
`1bd26dadd972057ce8b5ac6fdf9bd79b8f07662d10fdd1cfa59808f6c5720545`.
Its Wasm remains byte-identical to the already managed-qualified peer, SHA-256
`01f9170c9a10292e2ccbe6ea45ea8fa3e425a820fa884038ce6643db55139b94`.
The exact governed PocketIC case
`pic::lifecycle::tests::published_managed_component_group_support_drives_child_lifecycle`
passes against that Host/lock graph: real installation, lifecycle restoration
and authorization refusals. The case took 27.22s; the runner completed in 166s,
including native inventory compilation and an inherited output pipe held until
an invocation-owned detached PocketIC server exited. No manual termination was
performed. This is managed lifecycle evidence, not production deployment or a
complete PocketIC suite; [#450](https://github.com/dragginzgame/canic/issues/450#issuecomment-6033333375)
records the final proof. Native/tooling logs remain under
`target/review-validation/host-reuse-20261007/` and
`target/review-validation/shared-tooling-014-refresh/`.

Changes remain uncommitted and the maintainer-selected `0.110.53` draft stays.
No broad gate, version bump, publication or deployment ran. Public hard cuts and
the existing #457/#459 minor-boundary conflicts still prevent declaring the
complete batch push/publication ready. Existing staged entries were not changed;
Git may refresh its index stat cache during observation.

## Earlier Host reuse result — 2026-10-07

Host reuse follow-up delegates gzip publication, bounded frontend file hashing
and strict lowercase network digests to published Host 0.3.1
([Canic #458](https://github.com/dragginzgame/canic/issues/458)).
On Linux, 14 network tests, two payload inventory tests and 16 artifact tests
pass. The latter filters used the already-built native executable while another
workspace Cargo build was active. Tool invocations in artifact fixtures are
substitutes. These checks retain the existing lock selections and do not qualify
managed execution, full CI or native macOS. The maintainer committed the source
replacement separately as d7698e1f0541cb52ad8d762bc1549a700b412e88 during review;
this agent created no commit. Existing minor-boundary conflicts remain.

This file records implementation and validation handoffs. Track bugs, review
findings and follow-up work in [GitHub issues](https://github.com/dragginzgame/canic/issues).

## Shared Tooling 0.1.12 refresh — 2026-10-07

The 63-file snapshot selects reviewed committed revision
`33c2a6f0018a94915f819ff219e270500ed5b73b`; exact bytes and modes verify.
The later dirty sibling Rust-workspace rules stay excluded. Independent snapshot
hashing no longer executes the inspected checksum helper. Release intent and
atomic push remain bound to the captured destination URL. Shared exact-commit CI
inspection replaces the copied helper, with bounded listings and explicit
historical failure search. The [owner guide](../governance/shared-tooling.md) and
existing open changelog describe the maintained callers;
[Canic#461](https://github.com/dragginzgame/canic/issues/461#issuecomment-6032274059)
owns results.

Focused release destination-drift/recovery, CI inspection, pinned installer
projection and canonical installer fixtures pass. Canic's governance adapter
qualifies exact canonical file-list membership and link closure in an isolated
export; missing membership, broken links and tampered helper/payload refusals
pass. Scoped ShellCheck, CI actionlint and document guards pass. macOS durability
CI now selects maintained publication/recovery tests through the shared nonempty
wrapper. All three execute successfully on Linux; empty/ignored-only selection
refusal and failed Cargo status/evidence retention pass separately. This is native
Linux evidence, not managed execution or native macOS qualification. Upstream
[0.1.12 CI](https://github.com/dragginzgame/shared-tooling/actions/runs/37511845192)
passes its configured matrix. Logs remain under
`target/review-validation/shared-tooling-012-refresh/`.

The native result used Host 0.3.0 and Timers 0.14.4. A concurrent lock update now
selects Host 0.3.1 and Timers 0.14.5; it is preserved and this result does not
qualify that later graph. No dependency selection was changed by this refresh;
the real index remains unchanged. No broad gate, managed execution, commit,
version bump, publication or deployment ran. Existing #457/#459 minor-boundary
conflicts still prevent declaring the complete batch push/publication ready.

Shared Tooling#33 remains open for historical-symlink hook qualification. The
upstream distribution fixture also assumes its source history lacks a committed
snapshot; its retained Canic failure is reported in
[Shared Tooling#28](https://github.com/dragginzgame/shared-tooling/issues/28#issuecomment-6032229331).
That fixture is not selected or patched in Canic. The local governance adapter
uses the canonical list and shared verifier/link checker instead.

## Shared Tooling follow-up adoption — 2026-10-06

The 57-file snapshot now selects reviewed committed revision
`46c02774a8335cb3949d6f04284c4f53375353c1` (0.1.11); exact bytes and executable
modes verify. Dirty sibling prerequisite/CI-inspection work stays excluded.
The empty GitHub repository description is corrected to match the README.
Canic delegates pinned actionlint/ShellCheck/CI sccache installation, formatter
prerequisites, Cargo inheritance/version reading, local lock transformation,
portable current file digests, private RustSec database preparation and
explicit-cutoff Perl tag maintenance. Existing executable pins/destinations stay;
cargo-get, copied installers/lock parsing/tag maintenance and redundant Rust
inheritance tests are removed. Cargo metadata still selects Canic's package roster;
Canic owns transaction rollback, source authority, receipts, audit classification,
logging and compiler fallback. See the [owner guide](../governance/shared-tooling.md)
and the [owning result](https://github.com/dragginzgame/canic/issues/461#issuecomment-6023216324).

Focused shared-helper and Canic caller fixtures pass, including negative Cargo
inheritance/version cases, four host pin projections, digest/backend failures,
local RustSec isolation, exact tag retry identities, release runner recovery,
tool setup and developer recipes, and release candidate/commit views. Real Cargo
toy-workspace execution with fake Git authority passes receipt
creation/replacement, retained undated history, failed synchronization, partial
transformer output and exact rollback, including previous receipt absence.
Actual Linux actionlint, ShellCheck and sccache installations pass in private
evidence directories. Canic's three remaining product manifest tests pass;
strict Clippy for both changed native guard targets, scoped rustfmt, ShellCheck,
Bash syntax and CI actionlint pass. Evidence remains under
`target/review-validation/shared-tooling-followup/`. No actual tag deletion or
full advisory-policy audit ran. Native macOS execution is
[upstream CI evidence](https://github.com/dragginzgame/shared-tooling/actions/runs/37500153922),
not Canic qualification.

The shared whole-checkout hook checker rejects Canic's intentional historical
symlinks before invoking the product adapter. The actual isolated-index hook and
its fixture stay Canic-owned and pass their focused regressions; shared checker
adoption remains blocked by
[Shared Tooling#33](https://github.com/dragginzgame/shared-tooling/issues/33).
No broad gate, managed execution, commit, version bump,
publication or deployment ran in this follow-up. Staged concurrent work and real
lockfiles are preserved. Package versions and the selected 0.110.53 draft stay;
the complete release batch retains the existing #457/#459 minor-boundary conflict
and is not declared push/publication ready.

## IC Host package split adoption — 2026-10-06

Canic now uses all four published IC Host Tooling 0.3.0 owners directly:
`ic-host-artifacts` for Wasm/gzip mechanics, `ic-host-fs` for reads, hashing,
durable publication and descriptor locks, `ic-host-process` for executable
resolution, and `ic-host-tools` for Candid formats. Host selects the artifact
features explicitly; CLI and internal fixtures inherit their own filesystem
dependency. Existing unrelated dependency identities are preserved, including
the maintainer's Memory 0.28.4 and Query 0.47.6 selections. Earlier qualification
below retains its recorded graph rather than qualifying these later updates.

The public `canic_host::durable_io` surface, copied 631-line engine and 488 lines
of duplicate primitive tests are deleted. Callers, examples and the embedded-peer
producer use `ic_host_fs::durable`; no compatibility module or copied replacement
remains. Canic retains domain errors, journal schemas, authority, custody,
intent-before-effect ordering and same-operation recovery. The former failing
split-import check remains retained as superseded evidence.

Qualification passes 108 selected native tests: artifact/gzip boundaries, tool
resolution/install, private key admission, interrupted enrollment/publication,
Component-operation recovery, CLI output/scaffolding, peer admission and build-lock
recovery. Strict Host/CLI/internal-test all-feature/all-target Clippy and scoped
formatting pass. Final import grouping follows the hygiene policy; strict lint
and independent embedded-peer verification pass again on that final source. The
stale embedded allocation peer was refreshed and its bytes reproduced. The exact governed managed-child lifecycle PocketIC case passes,
including independent peer freshness, real installation, authorization refusals
and same-release restoration; its native inventory preflight passes separately.
One completed-case PocketIC child retained the log pipe; stopping only that
verified owned child released the runner, which completed successfully. This is
focused Linux/native and managed execution, not full workspace or macOS proof.
Evidence remains under `target/review-validation/host-split-adoption/`.

Streaming awaits upstream publication: `encode_gzip`, `MatchingWriter` and
`durable::write_with` are absent from published 0.3.0 and exist only in the
read-only sibling's pending work. Process delegation needs trusted executable
digest/environment admission; paid ICP commands additionally need inherited
lock-descriptor support. Archive/response delegation must preserve current
admission and detailed typed errors. Backup's stronger directory publication
remains with its owner. The [owning result](https://github.com/dragginzgame/canic/issues/458#issuecomment-6022225599)
records these remaining contracts and downstream acceptance.

Read-only Toko review finds its native qualification caller still uses removed
`ic_host_tools::artifact::read_file`; it needs a direct inherited `ic-host-fs`
native dev dependency and `ic_host_fs::read::read_file`
([Toko#30](https://github.com/dragginzgame/toko-miner/issues/30)). No downstream
compile, mutation or managed acceptance ran. This public Rust surface hard cut
adds to the recorded [#457](https://github.com/dragginzgame/canic/issues/457)/
[#459](https://github.com/dragginzgame/canic/issues/459) minor-boundary conflict;
the complete release batch is not declared push/publication ready. Package
versions and the maintainer-selected 0.110.53 draft remain unchanged. No commit,
version bump, publication, live deployment or broad gate ran; existing staged
and unrelated work is preserved.

## Blob adapter stack and production initialization — 2026-10-06

The Canic-owned adapter and its independent dedicated/embedded workspaces now
select published Blob Storage 0.15.2, Memory 0.28.3, Timers 0.14.3 and
arithmetic-only Metrics 0.2.2. Timers re-exports the shared Metrics summary type;
there is no cross-version conversion, application override or copied adapter.
Concurrent Host Tools 0.2, Query 0.47.4 and Testkit 0.19.2 selections are retained.
Follow-up package review confirms these manifests and locks match the qualified
snapshot. Host Tools 0.2's owned parser error and typed process-I/O categories
need no Canic adaptation; all nine focused Wasm-inspection regressions pass
against its updated parser. Query 0.47.4 has identical Rust source to 0.47.3.
Testkit still selects Host Tools 0.1.14 in its explicit native-only dependency
table; that additional host identity does not enter managed Wasm graphs.
The new native result is retained in
`target/review-validation/package-update-host-wasm.log`; no new managed execution
or broad validation ran during this follow-up.
The qualified root lock SHA-256 is
`83a050270dccde55dc1b660f68dd29d42833c12ad4daef308ca7411b0f8d2628`;
manifest SHA-256 is
`fc640c2fe4d463e16b3146ed6a90ab86cdcc4cbfa81918857afe720f55e90250`.
[Canic#444](https://github.com/dragginzgame/canic/issues/444#issuecomment-6021528575) owns acceptance and
remaining downstream work; the [owner status](../design/0.111-standalone-blob-service-extraction/status.md)
and [operating guide](../features/blob-storage/README.md) describe this contract.

Root now holds Components whose Spec requires application initialization. A
controller binds 1–16,384 opaque bytes to the actual allocated Principal and
member operation. The atomic retained binding, exact install-intent hash and
Host command encoder preserve Canic lifecycle ownership and deliver exact inner
bytes through ordinary and Component Group production installation. The protected
outer envelope is generated from current authority. Changed targets/bytes refuse;
identical binding replay is effect-free. Controllers can inspect Component Group
allocation status; noncontrollers remain refused. This is a pre-1.0 configuration,
wire and stable-record hard cut requiring cross-release clean reinstall.

Complete release-bound dedicated and embedded Fast builds pass. Their eight
complete normal Wasm graphs each contain one Memory, Timers and Metrics identity.
Structural blob Candid parity and actual PocketIC installation, caller/tenant
refusals, lost mutation response, aggregate reporting, same-release restoration
and repeated current-instance recovery pass for both forms. A temporary ignored
service proof qualifies these artifacts without restoring blob dependencies to
Canic's default test graph. Single-role compilation lacks the release-build
identity and does not qualify managed installation.

The maintained generic production initialization PocketIC case passes with real
Root/Coordinator/Store/probe artifacts: held allocation, exact-target refusals,
controller inspection, Root interruption before binding, discarded binding-command
ingress reply, full-bound 16,384-byte delivery, effect-free replay and synchronous
same-release application restoration. Root upgrade during installation quiesces
pending IC management callbacks. It does not inject a lost install callback;
uncertain install-intent persistence and renewal have separate native evidence.
Strict focused Core/Control Plane/Host/probe/internal Clippy and the native binding
regressions pass. The refreshed allocation peer reproduces SHA-256
`020c5880db1a585e725d0f07b16b50a6a138dd965369275956fac00c1cd54c62`.
Logs remain under `target/review-validation/blob-stack-*`; failed earlier attempts
retain their original scope alongside final passing logs.

Read-only Toko inspection finds one current Blob/Memory/Timers/Metrics identity
in its locked normal blob Wasm graph. Its Spec still lacks the required-init flag,
and no application-owned target-bound input producer/binding caller was found.
Its own managed installation and acceptance remain downstream work. No Toko
mutation, compilation, installation or live provider qualification ran.

This in-repository implementation is qualified for review. The complete planned
release batch is not declared push/publication ready: the recorded human-owned
minor/release boundaries in [#457](https://github.com/dragginzgame/canic/issues/457)
and [#459](https://github.com/dragginzgame/canic/issues/459) remain, and the selected
0.110.53 draft was not renumbered. No commit, version bump, publication, deployment
or broad workspace gate ran. Existing staged and unrelated work is preserved.

## Earlier dependency and native-guard qualification — 2026-10-06

The maintainer's lock at this earlier qualification selected Host Tools 0.1.14, Memory 0.28.2,
arithmetic-only Metrics 0.2.0 and Timers 0.14.0. Canic already owns its CDK
instruction reader; the selected timer platform adapter owns its reader too.
The late timer requirement change was resolved without changing other packages,
removing the remaining Metrics 0.1.9 selection. A later maintainer lock update
selects Testkit 0.19.2.
The newer published Host/Memory patches do not change their library source.
These selections retain Query 0.47.3 and management types 0.11.0.
That qualification's lock SHA-256 is
`d656d673dcaf807cf496ffee1d2710d69f330784e6431b987f8c9dd306bfa304`.

The latest retained validation log failed two native guards. Their corrections
admit only exact unpublished performance-observation queries and exclude Host's
isolated role-contract templates from the maintained package inventory.
Both guard targets pass, including negative export-option checks. Initial
qualification passed eight Core performance tests, 15 memory adapter tests,
seven Host installation tests and the isolated role-template resolver regression.
After the late timer/Testkit updates, both guard targets, all 29 selected Core
timer/performance/memory tests and strict targeted guard Clippy pass again.
Initial qualification corrected one test-literal lint. Current manifest/lock/
performance-source hashes stayed unchanged through the final native checks.
Logs and hashes remain
under `target/review-validation/dependency-update-*`; earlier qualification keeps
its original graph identity, including lock
`1ca6746ab0ab4c2e95838fa7394663bd4e497f0f2b58e6bd877a72ffecb530cd`
([#450](https://github.com/dragginzgame/canic/issues/450#issuecomment-6018058712)).

Independent fixture verification found stale embedded allocation-peer bytes on
the updated graph. The refresh reproduces exact replacement bytes; the owning
child-lifecycle PocketIC case passes, including independent freshness admission,
real canister builds and same-release restoration. The final artifact SHA-256 is
`5b3a9597b5551ad0fb15b051fbf8fbd0b9de830f847541ff0f5c2d7a002aaeff`,
with source input digest
`22a6bfbf03f8b735f1d1014c5d81a7b1796002272d3beabe496af0dabb0c2c55`.
The current lock/manifest/performance-source hashes remained unchanged throughout
refresh and the complete exact-case runner, including server cleanup. The
freshness failure stays retained alongside the successful current refresh/proof.
The [final qualification](https://github.com/dragginzgame/canic/issues/450#issuecomment-6018418811)
records this graph separately from earlier results.

The retained 41-file Shared Tooling snapshot verifies. Sibling committed revision
`47cd2ccaf0e8b428f06e6db0262df76cfc1581de` was inspected read-only; its later dirty
hook and lock helpers were not inherited. The observed remote main still matches
the local tracking reference; this checkout is 15 commits ahead. There is no
active pre-push hook or outgoing blob over 10 MB. The available log's warnings
are unsupported canister cdylib doctests, separate from the failed assertions;
no actual push-warning transcript was available. Changes remain uncommitted and
extend the existing notes. No broad gate, version mutation, Git publication or
deployment ran; the complete batch retains its recorded #457/#459 boundaries.

## Management-canister types adoption — 2026-10-06

The direct workspace requirement now selects registry `ic-management-canister-types
0.11`, locked at `0.11.0`. Its only direct consumer is the runtime probe; HTTP
requests explicitly retain pricing version 1 to match their existing cycle quote.
`ic-agent` keeps its required transitive `0.8.0`. The initial types update
preserved other selections; later concurrent maintainer updates are retained.

Strict native runtime-probe Clippy and the exact governed interleaving PocketIC
case pass. A freshness check found the retained allocation peer differed from
the current checkout. Its refresh confirms reproducible bytes; the owning
managed-component-group child-lifecycle PocketIC case then passes, including
independent embedded-peer verification and same-release restoration
([#450](https://github.com/dragginzgame/canic/issues/450)). Evidence is retained under
`target/review-validation/management-types-*`. The initial direct Wasm lint
attempt hit the canonical role-build guard; governed fixture builds qualify the
actual Wasm. The initial sandboxed PocketIC attempt could not bind localhost;
the permitted exact retry passes. Earlier failures remain retained.

After the first qualification, concurrent updates selected Host Tools 0.1.13,
Memory 0.28.1, Query 0.47.3 and Testkit 0.19.1. Explicit locked cache preparation,
peer refresh, strict native fixture Clippy and both exact PocketIC cases pass
again on that graph. Final logs use `management-types-current-*`; earlier logs
and the intermediate peer receipt retain their original graph identity.
The final lock SHA-256 is
`7362388043d5c6d5eeae08f02c0c19c11cf712f0f4159181669916768501beef`;
peer source digest is `b364527eb41a5a38fe25d202d74cee205fdfa56d04d29ac3ebb2002f2ec05be6`
and artifact SHA-256 is `00c23d513a6729c4de613c6dc2ed4093962bfda670cb928fbb9b79c62925c1e9`.
Manifest, lock and probe hashes remained unchanged through final qualification.
Changes are uncommitted and extend both pending 0.110.53 notes. This compatible
dependency/fixture update does not resolve the complete batch's #457 or #459
boundary; no broad validation, release, Git publication or deployment ran.

## Consumer-owned instruction reader — 2026-10-06

The performance adapter now calls the existing CDK counter-1 API directly and
removes ic-metrics' `ic` feature selection. Published 0.1 arithmetic stays selected
until the shared library's arithmetic-only 0.2 publication. Native zero, exclusive
nesting, async invocation/checkpoint identity and report fields are unchanged.
[ic-metrics #10](https://github.com/dragginzgame/ic-metrics/issues/10) coordinates the cut and later registry adoption.
This compatible adapter edit extends both pending 0.110.53 notes; independent
breaking changes and the human-owned minor boundary remain as recorded below.
On the current concurrent locked graph (ic-memory 0.28.0, ic-testkit 0.19.0),
strict Core library/tests and Wasm library Clippy pass, along with eight named
performance tests. The single governed PocketIC
`interleaved_endpoint_and_checkpoint_metrics_preserve_call_contexts` case passes
against actual IC counters in both completion orders, after its inventory
preflight. Manifest/lock/performance-source hashes stayed unchanged. Inputs,
source hashes and logs are retained in ic-metrics'
`target/evidence/arithmetic-cut-020/canic/`. This is focused Linux/IC proof; older
hosted results do not qualify this worktree and no full local gate ran. The
prepared shared adapter still needs its own release and coordinated adoption;
a later concurrent fixture/PocketIC graph edit adding management types 0.11 was
rechecked with strict native runtime-probe lint and the same exact governed IC
case. Five new input hashes match; the runner used its admitted Wasm cache.
An attempted direct fixture Wasm check was rejected by the canonical build guard
and retained in the separate recheck log; the guard was not bypassed. No
commit, real staging, version change or publication ran.

## Shared Tooling and Host SDK delegation — 2026-10-06

The maintainer accepted working through the delegation audit. Canic now adopts
41 exact files from committed Shared Tooling
`a37771f1b6b5fc9a88ed6ab3b705bdda35cd8fa3`; dirty sibling work was excluded.
Explicit repository tool setup/offline checking and CI use the common matrix,
retiring duplicate IC installers and implicit PocketIC downloads. The declaration
checker preserves scoped existing exact requirements. Eight negative role-contract
manifests become `.fixture` templates materialized by their owning tests.
The checker passes against an isolated reviewed index/object view; the real index
is unchanged, so its normal file inventory reflects these renames only after the
maintainer records them. This delegation work preserves dependency selections
and package versions; concurrent maintainer dependency updates were retained.

Host delegates executable filesystem resolution to the existing registry
`ic-host-tools 0.1.12`, retaining preferred installation order, literal paths,
npm distribution policy and selected-candidate admission. Rust Binaryen
installation retains the macOS executable/library bundle and atomically selects
it only after qualification. Archive-derived library hashes join executable
admission before version execution. The
[library authority evidence](../audits/reports/2026-10/2026-10-06/binaryen-library-authority.md)
records both reviewed macOS archives without claiming native execution.

Late release checks validate the selected commit's archived metadata, source-bound
receipt, version/date, exact Cargo transaction and annotated tag independently
of newer HEAD/worktree metadata. The shared runner retains unfinished-intent
reconciliation. Canic's isolated formatting adapter omits unrelated historical
symlinks from its scratch view while preserving their real index entries; the
unchanged shared installer refuses to replace existing hook configuration.
Generic audit definitions now use shared methods with local product overlays;
frozen reports/definitions remain historical and changed comparisons are
non-comparable. The [adoption walk](../audits/reports/2026-10/2026-10-06/shared-method-adoption.md)
maps retained obligations without rerunning a product audit.

Twenty-seven focused Host tests pass, covering installation/admission, executable
resolution and role-template materialization. Final qualification preserves the
concurrently selected Host Tools 0.1.12, Memory 0.28.0, Metrics 0.1.9, Query 0.47.2,
Testkit 0.19.0 and Timers 0.13.5. The manifest and lock remained byte-identical
through all final tests and strict Host library/test Clippy, all features and
locked/offline. Evidence is retained in
`target/review-validation/host-delegation-final.log` and
`host-delegation-graph.sha256`; lock SHA-256 is
`e874292b9366fe9aa7ff87f237edcf3aa805c68afed804d98fd5d8f5a7a606b5`.
Earlier attempts were stopped when dependency inputs changed; one template
attempt lacked offline child resolution. Actual Linux
shared setup/offline verification, installer fixtures, hook preservation/setup,
release recovery/recipe/lane fixtures, declaration checks, snapshot/catalog/link
checks, scoped shell/workflow lint and formatting pass. Native macOS CI now owns
the actual Rust installer, resolver and hook proofs on both architectures;
those executions remain outstanding. No full workspace or deployment gate ran.

The ready in-repository delegation work is reviewable and uncommitted:
[#461](https://github.com/dragginzgame/canic/issues/461),
[#458](https://github.com/dragginzgame/canic/issues/458),
[#464](https://github.com/dragginzgame/canic/issues/464),
[#453](https://github.com/dragginzgame/canic/issues/453),
[#454](https://github.com/dragginzgame/canic/issues/454) and
[#460](https://github.com/dragginzgame/canic/issues/460) own the respective work.
Complete bounded Candid/process/Git delegation still needs trusted executable
authority and explicit source/output/deadline/environment bounds; current Candid
normalization remains delegated. Whole release-batch push/publication readiness
is still unestablished because #457 remains unresolved and the memory/performance
hard cuts require the human-owned minor boundary in #459. Both changelog views
extend the existing pending 0.110.53 entry and preserve that version conflict.
No commit, real staging, tag, push, repository version transaction, live deployment or sibling
mutation occurred; retained release/build artifacts remain intact.

## Invocation-owned performance accounting — 2026-10-06

[#99](https://github.com/dragginzgame/canic/issues/99) is implemented and qualified
in the uncommitted checkout. Each async endpoint owns its frames and checkpoint
baseline, activated only during its future polls. Synchronous nested work retains
exclusive accounting. Pending polls, cancellation and native unwind restore the
enclosing context. Lifecycle and issuer-renewal futures establish their own
checkpoint contexts; application futures can use `api::ops::with_async_perf_context`
within one IC call context. Unscoped checkpoints produce no sample.

All eight focused Core perf tests, fifteen endpoint expansion tests and the
public checkpoint-macro test pass. Strict native Core/Macros/Facade/Internal
target Clippy and runtime-probe Wasm Clippy pass, locked/offline. The real IC
interleaving proof passes with both completion orders: endpoint totals and first
and resumed checkpoints remain within independent call-context counter brackets.
Final evidence is under `target/test-runs/20261006T104815Z-577601.ww6EVz/`.
The refreshed public embedded peer also passes the child lifecycle and same-release
restoration case at `target/test-runs/20261006T105343Z-643154.mFIi0e/`.
Refresh reproduces the exact bytes and independent fixture verification passes.

Final qualification preserves the concurrent lock selections `ic-metrics 0.1.8`,
`ic-host-tools 0.1.10`, `ic-query 0.47.1`, `ic-memory 0.27.1` and `ic-timers 0.13.5`.
The receipt binds lock `2614292dc4b16c44727a8c19e87e25076142c274e648181e49df3b5a55383771`,
source digest `be653be572dd1b49a050258add0f3b8bb9bdb1c7d61dbebc637b481686cb2be8`
and artifact `a3c17fb4d944b0a0123ed3614b3eb72c30ad06e4191d6251c6bfbe491a548d2b`.
Earlier lookup cost figures bind their pre-repair sources, not these wrappers.
No complete workspace, macOS, release or deployment gate ran for this repair.

Direct Core dispatch enter/exit callers must adopt scoped measurement, and
background checkpoint callers must establish a context. The instrumentation
hard cut joins the memory hard cut in requiring a minor release; the selected
pending `0.110.53` patch remains incompatible. Both changelog views record this.
Package versions remain 0.110.52. Full-batch push/publication readiness remains
unestablished while [#457](https://github.com/dragginzgame/canic/issues/457) and the
human-owned minor boundary described in [#459](https://github.com/dragginzgame/canic/issues/459)
remain unresolved.

## Host tools adoption — 2026-10-06

The initial Host adoption selected published registry `ic-host-tools 0.1.9` for Wasm
structural inspection, Candid normalization, bounded admitted-descriptor reads,
tool-file SHA-256 and gzip decoding. Canic retains admission, install limits,
exact transform contracts and artifact publication. Trailing compressed bytes
and extra gzip members are rejected; original file I/O error kinds are retained.
Seven superseded local readers/normalizers are removed. Public ICP response
decoding retains its detailed Canic error payloads; that API is not changed here.

All 44 selected Host tests pass with default features against `ic-memory 0.27.1`,
`ic-query 0.47.0`, `ic-metrics 0.1.7` and `ic-timers 0.13.5` in the primary
checkout, after an isolated 0.26.2 diagnostic run. The installed
real-extractor test remains intentionally ignored; native extractor fixtures
cover normalization/cache behavior. Warning-denied Clippy passes for all
Core/Host/Internal targets/features on the final Host tools 0.1.9 graph and for
the changed Canic protocol target. Core Wasm library Clippy also passes; the Host
SDK is excluded from that runtime graph. No full workspace or macOS gate ran.

The [adoption](https://github.com/dragginzgame/canic/issues/458) is uncommitted.
Host tools 0.1.8's exact `tar 0.4.40` constraint initially failed the dependency
gate. Published 0.1.9 permits selecting patched `tar 0.4.46`; the prior rustix
1.1.5 selection is restored. Current RustSec audit and Canic's dependency-risk
gate pass with zero vulnerabilities and the two previously reviewed transitive
warnings. [Upstream #3](https://github.com/dragginzgame/ic-host-tools/issues/3)
records the remaining minimum-version recommendation. No Cargo override,
advisory exception or sibling repository edit is introduced.

## Current memory ledger adoption — 2026-10-06

The maintainer explicitly requested completing the concurrent `ic-memory 0.27`
selection. The lock now resolves published `0.27.1`. Core removes upstream
history projections and reports the exact ledger anchor, checked commit counter,
recovery slots, retained ownership/state and latest declared schema version.
The memory DTO module moves to `dto/memory/mod.rs`; its ordinary module identity
is unchanged. Current retirement preserves claimed IDs. No history is fabricated.

All 15 focused memory adapter tests, four stable-memory ABI guards and the
selected public memory Candid-shape test pass with all features, locked/offline
on the final graph. The memory DTO enum round-trip and Core Wasm Clippy also pass.
The
[migration](https://github.com/dragginzgame/canic/issues/459) changes public
diagnostic DTOs and persisted ledger bytes, requiring updated consumers and
clean reinstall. Shared Tooling therefore requires a minor release; the selected
`0.110.53` patch conflicts with this hard cut. Both draft changelog views flag
that conflict. Package versions remain 0.110.52; no minor closeout verdict,
version transaction, Git publication or deployment is implied.

The two optional blob consumer workspaces still select `ic-memory 0.25` with
`ic-blob-storage 0.14.9`; their earlier composition proof is not current for
Core 0.27. [#444](https://github.com/dragginzgame/canic/issues/444) owns that
independent integration boundary. Full-batch push/publication readiness is not
established.

## Focused PocketIC fixture correction — 2026-10-06

The retained governed run is
`target/test-runs/20261006T073404Z-1450918.6mpRHX/10.log`. Synthetic lifecycle
fixtures now complete receiver opening and startup release before admission
checks. The imported-pool fixture allows only observed, time-bounded IC idle
debit for its unclaimed empty asset. These changes are uncommitted and extend
the pending 0.110.53 notes; formatting and whitespace checks pass.

The first exact direct-ingress regression stopped in locked dependency prefetch,
before compilation or PocketIC: the concurrently edited manifest requires
`ic-memory 0.27`, while that attempt's lockfile retained `0.26.2`. The memory
adoption above now aligns source and lock. All four corrected cases pass through
the exact governed runner: the three managed-admission journeys and the imported
pool refresh/claim cycle test. Their retained evidence is respectively under
`target/test-runs/20261006T093356Z-3530354.CKWSCe/`,
`20261006T093753Z-3580448.IoXrQi/`, `20261006T093933Z-3613230.RLnY5O/`
and `20261006T094310Z-3695898.XNCKeX/`. The refreshed embedded peer's child lifecycle
proof also passes at `20261006T093937Z-3614558.5bMOM6/`. These PocketIC proofs used
Host tools 0.1.8 and the current memory/metrics/query/timer selections before the
compatible Host dependency correction. Dependency/performance edits are preserved.
[#457](https://github.com/dragginzgame/canic/issues/457) retains the two initial
working-Fleet membership stalls; their retry policy is unchanged. Full-batch
push/publication readiness is not established;
no broad gate, version transaction or Git publication ran for this correction.

## Fixture lock preservation — 2026-10-06

Final Host tools qualification exposed an embedded-peer producer defect:
`cargo update --workspace --offline` could reselect external packages while
normalizing Canic's private fixture version. The producer now derives workspace
ownership from Cargo metadata, rewrites only owned manifest/lock identities and
exact local references, and requires Cargo to accept the snapshot with
`--locked --offline`. External registry/Git selections and unrelated path
requirements remain unchanged, including version-coincident dependencies and
registry-qualified references.

Four focused normalizer/evidence tests and the explicit real-Wasm reproduction
across paths and fixture versions pass; the latter also rejects changed producer
code and verifies primary manifest/lock bytes remain intact. Internal strict
Clippy passes after the change. Final refresh reproduces the artifact bytes
and independent verification passes. The receipt binds primary lock SHA-256
`ab80dd031ceeb0edb3fedc2524f75fa07930b6fa9bf577b5446345547b5e3727`,
producer lock `5603ed4a9b66186678d199a31286366fd90778f8c50a804b72155fade933f244`,
source digest `a6e01e9e432f74fff102f31c483b6730003401a0f3b384eb0e7bfa59e5a9af74`
and artifact `65fd3923632b950b0da11c5e7634609f1d4edd8e829ee01d37a53f2c9c53c34a`.
The compatible SDK correction leaves the previously qualified PocketIC peer
bytes unchanged. [#450](https://github.com/dragginzgame/canic/issues/450) owns
this producer correction; #458/#459 own the integrations. Changes remain
uncommitted and the full-batch blockers above remain open.

## Performance lookup work — 2026-10-06

The compatible pending 0.110.53 batch borrows repeated endpoint/checkpoint keys
and collects performance entries in the map's existing order. Public keys,
report ordering, zero/saturation/reset contracts and attribution functions are
preserved. The [evidence record](../audits/reports/2026-10/2026-10-06/metrics-lookup.md)
binds sources, dependencies, artifacts and focused commands.

In actual local IC fixtures, repeated endpoint/checkpoint instructions fall
about 53%/60%, and raw Wasm shrinks 3,698 bytes. New-key instructions rise about
85% because a borrowed miss is followed by owned insertion. These changes favor
stable keys; they are not whole-managed-canister or universal savings.

Native/Wasm strict Core Clippy, all five selected perf tests and the Rust 1.91
Wasm library check pass after formatting. Package versions and dependency
selections are unchanged; the implementation and notes remain uncommitted.
No broad gate, macOS execution, release or publication ran for this batch.
A real async interleaving probe confirms both endpoint totals are outside their
own IC intervals. [#99](https://github.com/dragginzgame/canic/issues/99#issuecomment-6012668977)
records that earlier evidence; the repair and current qualification appear above.

## Embedded allocation peer refresh — 2026-10-06

The maintainer's retained run
`target/test-runs/20261006T064250Z-592136.6UP0Yt/1.log` failed embedded-peer
preflight because the checked-in allocation Wasm differed from a fresh build.
No ordinary tests or PocketIC cases started in that run. The
[repair](https://github.com/dragginzgame/canic/issues/450) regenerates the peer
and structured provenance using the maintained `refresh_embedded_root` owner.
Its second build reproduces the exact bytes after embedding them.

The maintainer-selected lock graph now contains `ic-memory 0.26.2`,
`ic-timers 0.13.2`, `ic-testkit 0.18.2` and `ic-metrics 0.1.5`. Missing locked
packages were fetched without changing selections. Testkit changed during the
first qualification attempt; refresh and independent verification then passed
against the final unchanged lockfile. The generated receipt binds lock SHA-256
`f0d49d1b6fc8c370322cb0661fa88ec677351098fc24e7dc61c668660dc7a052`
and artifact SHA-256
`52f7b413539024b13ac5f3e751942f0d658745b243a1d62b1ca37689544d7f33`.

The exact owning PocketIC case
`pic::lifecycle::tests::published_managed_component_group_support_drives_child_lifecycle`
passes, including peer installation, child allocation, admission and same-release
recovery. Its inventory preflight also passes. Complete evidence is retained in
`target/test-runs/20261006T071135Z-907379.TPz9b7/`; the governed runner exits 0
and clears only its invocation-owned scratch. An earlier sandbox attempt could
not bind the local simulation server; the same focused command passed with
localhost access.

The peer repair and pending 0.110.53 notes are complete, review-ready and
uncommitted. Package versions remain 0.110.52 and the concurrent lock selection
is preserved. Entire-batch push/publication readiness was not reassessed. No
broad gate, version transaction, Git publication, live deployment or retained
release-artifact cleanup ran.

## IC Metrics reader adoption — 2026-10-05

The current 0.110.53 batch selects published registry `ic-metrics 0.1.5` with
feature `ic` and delegates `perf_counter`'s Wasm read to the shared call-context
reader. Native zero, exclusive nesting, measured-zero semantics and public
reports remain unchanged. Only the metrics lock record changed; all other
dependency selections and package versions are preserved.

Locked offline native and Wasm Core library Clippy pass with warnings denied,
and all four selected endpoint-accounting tests pass. `make fmt` ran before
validation and preserved the concurrent release-flow fixture bytes. These are
focused Linux compilation/contract results, not new consumer IC instruction
measurements, native macOS qualification or complete-batch release evidence.
The pending root and detailed notes extend
[the metrics adoption](https://github.com/dragginzgame/canic/issues/447); no agent
commit, package-version bump or broad gate ran. Concurrent release-script,
fixture and handoff edits remain preserved.

## Release-fixture identity isolation — 2026-10-05

The maintainer's retained ordinary run
`target/test-runs/20261005T201157Z-3909842.t1bwSD/2.log` reports only the two
receipt success cases failing. The standard runner passes the real candidate
`RELEASE_VERSION=0.110.53` through Make into validation; the fixtures inherited
it while their temporary workspace derives `0.92.8`. The previous focused
qualification omitted that inherited environment. Both failures reproduce with
the outer candidate present and disappear without it.

The [correction](https://github.com/dragginzgame/canic/issues/450) binds the
fixtures' version/date explicitly. Existing rollback cases now retain the newly
written receipt at the injected tag observation and verify its structured fields
after rollback, preventing an earlier refusal from satisfying that proof.
The production bump still rejects mismatched identity before mutation and now
reports requested/planned versions.

All four selected draft/receipt success/rollback cases pass together with default
features through the governed scratch wrapper under the outer release version
and date. The changed integration target passes locked offline warning-denied
Clippy. Scoped ShellCheck, Bash syntax, formatting and whitespace pass; a command
stub check confirms version mismatch exits 1 without changing fixture files.
Only command-stub release fixtures were executed; they create no Git commits.
Concurrent metrics dependency/performance edits are preserved and remain outside
this correction's behavior qualification.

The existing 0.110.53 notes include this test-tooling and diagnostic correction;
package versions remain 0.110.52. These fixes are review-ready and uncommitted.
Complete-batch push/publication readiness was not reassessed. No broad gate,
version transaction, Git publication or artifact cleanup ran for this correction.

## Native validation contract propagation — 2026-10-05

The retained ordinary/internal native run
`target/test-runs/20261005T175731Z-3219217.QIQXvv` exposed stale fixtures and
missing propagation from the current accepted release batch. The
[repair](https://github.com/dragginzgame/canic/issues/450) aligns cache fixtures
with Cargo's release profile, release fixtures with the shared helpers and
numbered pending ledger, and Host/CLI inputs with current package/report/help
contracts. Lifecycle guards select exact function identifiers and check
restoration and synchronous participants before the current startup schedulers.
Replay manifests now cover caller-authority publication, startup release and
read-only Root membership; canonical capability Candid and timer custody match
their Rust owners. The locked graph guards pass after the separately recorded
registry metrics adoption below.

Locked offline focused qualification passes:

- Internal cache immutability: the exact reported unit test with
  `governed-pocketic-tests`.
- Canic: changelog, managed-endpoint and protocol-surface targets with all
  features; four selected release-flow success/rollback cases use command stubs
  and create no Git commits.
- Core: lifecycle, memory ABI and timer inventory targets plus all 32 focused
  replay-policy tests, with all features.
- Host and CLI: the two affected unit tests per package with default features.

Warning-denied Clippy passes for the changed Canic guard targets, Core library
and tests with all features, Host/CLI libraries and tests with default features,
and Internal library/tests with `governed-pocketic-tests`. Scoped formatting,
whitespace and the embedded peer verifier pass; its checked-in Wasm and
provenance remain unchanged. No PocketIC or broad suite ran for this repair.

The existing pending 0.110.53 notes cover the corrected native contracts;
package versions remain 0.110.52. The native repair is complete and review-ready,
uncommitted. The entire concurrent release batch's push/publication readiness
was not reassessed. No version transaction, Git publication or artifact cleanup
ran.

## IC Metrics registry adoption — 2026-10-05

Canic now resolves published registry `ic-metrics 0.1.4` without a sibling
checkout, retaining the maintainer's compatible `0.1` requirement, all other
lock records and consumer package metadata. The previous local path changed
its lock selection from 0.1.3 to 0.1.4 during maintainer validation; this adoption
preserves that selection and adds the verified registry source/checksum.
The primary release command stopped before the patch was applied. Source and
lock identities were rechecked against the captured inputs before mutation.

An isolated worktree first passed locked offline Linux metadata, manifest
sorting, warning-denied Core library Clippy and all four endpoint tests, using
its own target directory. Primary locked offline Linux metadata, manifest sorting, strict Core library
Clippy and all four endpoint tests also pass using this checkout's target.
A first primary test attempt was stopped after another validator acquired the
build lock; the completed rerun followed active-process and free-lock checks. Attribution, measured-zero
semantics and public reports are unchanged by adoption. Later concurrent Core
replay-policy/guard, CLI and host fixture edits are outside these passing results;
they require their owning validation rather than reusing the earlier evidence.
The pending 0.110.53 notes include
[registry adoption](https://github.com/dragginzgame/canic/issues/447); source edits
are uncommitted. No broad gate, IC measurement, native macOS qualification,
agent commit, tag, push, package publication or cleanup occurred.

## Selected testkit lockfile alignment — 2026-10-05

The maintainer's latest commit `2241e9896` selects `ic-testkit = "0.18"` but
retains registry `0.17.3` in Cargo.lock. Standard release preflight correctly
refused its locked offline fetch before validation or preparation. A targeted
offline update now selects the already cached `0.18.0`; only that package's
version and registry checksum changed. Every other lock entry and the testkit
dependency list are preserved. The published .17.3 and .18.0 library source
trees are byte-identical, so no further caller migration was needed.

`cargo fetch --locked --offline` now passes. Locked offline, warning-denied
Clippy also passes for the exact ingress-payload integration target with all
features. Whitespace checks pass. The alignment extends the existing
[testkit adoption](https://github.com/dragginzgame/canic/issues/449) and .53 notes;
changes are uncommitted and ready for review. The maintainer must commit the
lockfile correction before normal release preflight admits the source again.
The full release batch was not reassessed for push/publication, and no broad
gate, version preparation, Git publication, package publication or cleanup ran.

## Ingress fixture testkit API alignment — 2026-10-05

The maintainer's 17:28 UTC Clippy run found the ingress probe still using the
removed const-generic standalone fixture pool API. The
[correction](https://github.com/dragginzgame/canic/issues/449) uses the selected
registry `ic-testkit 0.17.3` constructor with a nonzero runtime capacity of one
and its function-pointer builder type. The snapshot restore funding policy is
preserved. Other baseline pool callers already use runtime capacities; no
dependency, lockfile or production runtime source changed.

Locked offline, warning-denied Clippy passes for the exact
`canic-tests/pic_ingress_payload_limits` integration target with all features.
Scoped formatting and whitespace checks pass. This constructor-only adaptation
did not rerun PocketIC or broad validation. The repair and existing .53 notes
are ready for review, uncommitted; whole-batch push/publication readiness was
not reassessed. No version transaction, Git publication or artifact cleanup ran.

## Release-note and crypto gate repair — 2026-10-05

The maintainer's 17:20 UTC validation at `abeb37ad9` reported missing release-note
fixture helpers and a false crypto-closure mismatch under English collation.
The [repair](https://github.com/dragginzgame/canic/issues/448) carries both common
helpers and current numbered ledgers into the isolated fixture. The production
preflight again refuses a missing required detailed-note file before validation
or version mutation. Crypto identities use C collation and normalized sets;
exact profile membership, distinct-version refusal and SHA-256 remain enforced.

The release-lane fixture passes its rejection, source-drift and receipt-recovery
cases. The actual locked offline Wasm crypto gate passes for all twelve canonical
roles under `en_US.UTF-8`, alongside new Cargo-stub fixtures covering unordered
and repeated inputs, missing/extra signature providers, duplicate versions and
missing SHA-256. Actual .53 notes preflight, scoped ShellCheck, Bash syntax,
snapshot integrity and whitespace checks pass. No broad validation was rerun.

These reported gate repairs are complete and ready for review, uncommitted, with
notes in the existing .53 draft. They do not establish the entire concurrent
release batch's push or publication readiness; package versions remain .52.
No version transaction, commit, tag, push, publication or artifact cleanup ran.

## Standard release validation retry — 2026-10-05

The maintainer's `release-patch` refused the retained `0.110.53` plan because
Canic still used Shared Tooling revision `b8537873`. That plan is at `validate`,
with no saved index tree or preparation file set; package versions remain .52
and neither local nor remote has the candidate tag. The snapshot now records
eighteen files from committed upstream `c0206f1943238e21bd00fbe01658e6a0864c24fa`,
exported through its distribution helper from a clean temporary checkout.
Uncommitted sibling changes were excluded and the sibling was not modified.

The common runner now permits a fresh normal-target retry after source fixes,
retaining earlier preparation-free plans unchanged before repeating preflight
and complete validation. Intent is persisted only before preparation. Exact
resume remains required after preparation begins. Snapshot verification,
command-stub runner recovery and the consumer's standard-entry/recipe fixtures
pass. The real .53 plan and failed-validation artifacts remain untouched.

The new upstream hook was evaluated but not installed: its regular-file-only
index rule rejects Canic's historical audit symlinks. The existing hook and
installer remain unchanged. Snapshot integrity does not qualify full baseline
adoption or native macOS behavior. The GitHub description is currently empty;
no external repository metadata or issue was written during this inspection.

The release retry correction and .53 notes are ready for review, uncommitted.
The complete concurrent release batch has not been reassessed for push or
publication. No broad validation, version mutation, commit, tag, push,
publication or cleanup ran. After the maintainer commits the correction, the
same `make release-patch` can restart the validation-only attempt.

## IC Metrics tagged-package alignment — 2026-10-05

The maintainer tagged ic-metrics 0.1.1 while the consumer still required local
0.1.0. The workspace dependency and only its lock entry now select 0.1.1; Canic
package versions and other dependency selections are preserved. Locked offline
metadata resolves one local ic-metrics package, and the selected Core library
build passes. This qualifies the extraction dependency graph, not the complete
concurrent release batch. The path remains temporary pending registry publication
and [adoption](https://github.com/dragginzgame/canic/issues/447). No broad gate,
Git effects, publication or native macOS qualification ran here.

## Release-recipe ShellCheck correction — 2026-10-05

The maintainer committed the preceding validation and release-tooling work in
`cb596fc72`. The next validation run reported SC2043 in the release-recipe fixture:
its remaining `patch-fast` case was wrapped in a loop containing one literal.
The fixture now invokes that case directly, preserving success ordering and
failure-boundary coverage. Scoped ShellCheck, Bash syntax, the complete recording
fixture and whitespace checks pass. The fixture executes command stubs and has
no Git effects. This structural test correction changes no maintained release
behavior and remains uncommitted. No broad validation or deployment ran; the
single corrected gate does not establish whole-release push readiness.

## Shared jq discovery and timer-provider integration — 2026-10-05

The reported publish-manifest guard now shares `jq` discovery with validation,
release and reporting helpers: explicit `JQ_BIN`, PATH, then the user-local
installation. The resolver exports an absolute executable path, preserves the
calling shell's PATH and refuses invalid explicit selections. Fixture copies
carry the resolver alongside their consuming scripts.

At the maintainer's request, framework jobs, lifecycle adapters, pool maintenance
and the runtime probe now use the selected ic-timers 0.12 policy-specific callback
results. Active examples match that API. The embedded peer's temporary workspace
now preserves external local dependency locations, including the selected
ic-metrics extraction, without changing those repositories. The peer Wasm and
structured provenance are refreshed and verification passes.

Qualification passes 37 Core timer/domain regressions, one pool outcome test,
three embedded-fixture tests, all ten timer-authority PocketIC cases and the exact
[Canic#38](https://github.com/dragginzgame/canic/issues/38) initial-Shard bootstrap
and recovery journey. This completes the previously interrupted installed rerun
on the current metrics/timer graph. Scoped all-target/all-feature warning-denied
Clippy passes for Core, Control Plane, facade, runtime probe and internal testing.
Evidence is under `target/review-validation/timer-api-*`; installed logs are
`target/test-runs/20261005T154734Z-1860318.Ac85TZ/1.log` and
`target/test-runs/20261005T155727Z-2012475.kzkp2H/2.log`.

The actual publish-manifest guard and focused jq, manifest-boundary,
dependency-classification, release-candidate, fast-patch and Wasm-classifier
fixtures pass with user-local tools absent from PATH. The optional historical
ablation inventory check still reports missing experiment rows; its inventory
was not changed here. Release fixtures that create Git commits were not run.
Release-fixture evidence predates the concurrent release-tooling edits below;
the actual publish-manifest guard and scoped ShellCheck were rechecked afterward.

These corrections are ready for review and included in the existing .53 notes.
Whole-worktree push/publication readiness still requires completion of the
concurrent shared release-tooling adoption and its qualification. Its edits and
the maintainer's ic-testkit 0.17.1 lock selection are preserved. Package versions
remain .52; this work performed no broad gate, version transaction, commit, push,
publication, live deployment or sibling-repository edit. Changes are uncommitted.

## Development-tool path correction — 2026-10-05

The maintainer's `update-dev` completed tool installation but failed at the final
bare `wasm-opt --version`: the installer changed only its child-shell PATH.
Make now passes explicit Binaryen and IC Wasm installation directories to both
setup routes and uses the installed paths for its ICP/IC Wasm/Binaryen probes.
The existing installed binaries report ICP 1.6.0, ic-wasm 0.11.1 and Binaryen 132.
The focused recipe fixture qualifies paths containing spaces, stale executables
on PATH, directory propagation and stopping after a failed tool probe; ShellCheck
and whitespace checks pass. The fixture invokes no installer, Git or network
effect. Default probes also pass with user-local directories absent from PATH.
The open .53 changelog includes this correction. Changes remain uncommitted;
the complete updater, broad validation and deployment were not rerun. The
installed caller-authority regression on the concurrent metrics graph remains
outstanding as described below.

## Validation preflight corrections — 2026-10-05

The maintainer requested correction of the 15:00 UTC validation failure following
[Canic#38](https://github.com/dragginzgame/canic/issues/38). Caller admission now
composes indexed ops projections with pure policy in workflow; startup errors
belong to the model, and Root publication workflow consumes read-only journal
views. The three derive-order failures are corrected without weakening the guards.
Dependency-risk checks resolve an explicit `JQ_BIN`, then PATH, then the existing
user-local executable. The maintainer's terminal PATH omitted `.local/bin`.

Focused qualification passes 20 caller-authority and 48 activation tests, including
refusal of undeclared sources before any reservation or row write. Warning-denied
Core/Control Plane/facade library-and-test Clippy passes. Layering, scoped formatting,
derive ordering, ShellCheck and validation-runner checks pass. The actual dependency
risk gate passes with the maintainer's PATH and current advisory data: zero
vulnerabilities and two reviewed transitive warnings. The original silent ShellCheck
failure did not reproduce through the host validation runner. Evidence is under
`target/review-validation/caller-authority-layering-*`, `dependency-risk-*-path*`,
`dependency-risk-path-regression.log` and `shellcheck-host-path.log`.

The embedded allocation peer and provenance were refreshed for these corrections.
The exact installed bootstrap/recovery rerun then encountered concurrent local
`ic-metrics` integration changes in the workspace and Core manifests and `perf.rs`.
Its post-build locked metadata check refused the changed dependency graph:
`target/test-runs/20261005T151415Z-1579004.GuGUzz/2.log`. Those concurrent edits are
preserved; further builds await their manifest/lockfile completion. The refreshed
fixture evidence predates that integration and needs requalification with it.
The earlier native, lint and dependency results also predate those concurrent edits.

The maintainer subsequently committed the corrections and `ic-metrics` lockfile
in `481e94d0e`. The next formatting check found the new dependency at the start of
both dependency tables. Cargo sorting now passes for the workspace and Core
manifests, with identical parsed TOML contents. The formatting-only pre-commit hook
was present but inactive: local `core.hooksPath` was unset and no conventional hook
was installed. The existing hook installer restored `core.hooksPath = .githooks`.
These formatting corrections remain uncommitted; current-graph installed
qualification is still outstanding.

The original preflight corrections are ready for review; complete-worktree push
readiness remains pending the concurrent integration and installed regression.
The existing .53 notes include these fixes; versions remain .52. Changes remain
uncommitted, and no broad validation, version transaction or deployment ran.
For agent follow-up, use dependency-risk fixtures' `--classification-only` mode:
their default mode and the release-tool fixture include temporary Git commits.
Those fixture modes were initially invoked during diagnosis before this was noticed;
Canic's repository history was not changed.

## Caller-authority integration — 2026-10-05

The maintainer explicitly requested [Canic#38](https://github.com/dragginzgame/canic/issues/38).
The deleted draft has been rebuilt in this checkout against the current owners.
Compiled permission policy, receiver projections and tickets, protected delivery,
installation bindings and publication-bound application startup are implemented.
Root activation and denial use indexed original-operation evidence within the
existing Component Registry store. Application endpoint startup admission is
independent of the endpoint's Boolean access expression.

Targeted qualification is complete. Native evidence includes 16 receiver/policy,
three Root publication, 21 activation/startup and 52 endpoint-macro cases, plus
the current-child allocation regression, role/memory manifests, public identifiers
and explicit Fleet-service/peer authority checks. The current-child lookup now
uses the registered allocation ID, so historical allocations retained for a
recycled Principal do not obstruct its current runtime authority. Initial members
retain their bounded bootstrap funding while Root remains Prepared.

Four exact PocketIC journeys pass on the selected locked graph:

- Initial-Shard bootstrap, Root-outage local admission, caller/target direction,
  receiver upgrade, unavailable-recipient denial, Root restart with frozen progress,
  competing enrollment, post-await effect fencing and two recycling/removal cycles:
  `target/test-runs/20261005T142640Z-1154820.asBm32/2.log`.
- Public Component Group fixture, generated protected Candid, discarded replies,
  receipt replay, foreign-Root refusal, two upgrades during publication,
  Component-wide denial and delayed-grant refusal:
  `target/test-runs/20261005T143533Z-1244831.Q603Tl/2.log`.
- Public Managed App and standalone fixture activation/upgrade:
  `target/test-runs/20261005T144326Z-1316622.0kDEun/2.log`.
- Fixture-bearing Root retirement, descendant cleanup and cycle conservation:
  `target/test-runs/20261005T144437Z-1325177.kfPVG2/2.log`.

The embedded allocation peer and structured provenance are refreshed. Warning-denied
Clippy passes for Core, Control Plane, facade, macros, host, internal simulator,
User Hub and runtime probe with all features and library/test targets. The actual
governed simulator catalogue is included. Native and lint logs are under
`target/review-validation/caller-authority-*`. The graph retains ic-memory 0.25.10,
ic-timers 0.11.8 and the maintainer-selected ic-testkit 0.16.0, including concurrent
lockfile updates. Retained-record budgets do not claim dense-graph performance or
account for stable-tree overhead.

The complete #38 implementation batch is ready for review and push through the
maintainer's commit flow. Issue #38 owns acceptance tracking. The .53 changelog is
ready for the release flow; package versions remain .52 and require the governed
version transaction before publication. Changes remain uncommitted. No broad gate,
release transaction, commit, push, deployment or sibling-repository edit ran.
The maintainer-approved [implementation status comment](https://github.com/dragginzgame/canic/issues/38#issuecomment-5997008520)
is posted to #38. The issue remains open for maintainer review.

## Repository payload cleanup — 2026-10-05

The maintainer requested repository-size cleanup under
[Canic#445](https://github.com/dragginzgame/canic/issues/445). Root ignores Cargo
`target/` directories at every depth; 4,506 accidental consumer build files are
removed from the index while their local bytes remain. Four superseded raw
qualification archives are removed, alongside the redundant browser-review
collection. Structured results retain their original measurements and hashes,
with explicit removal metadata; GitHub retains every original review record.
Reports link to retained summaries instead of removed payloads.

Focused ignore, JSON-preservation, document-link and whitespace checks pass.
The maintainer committed the checkout cleanup, then explicitly overrode the
agent commit/history restrictions for removal from all local history and tags.
All 291 local references were filtered; every other file at each reference tip
retains its original Git object ID. The maintainer's cleanup commit and current
working files are preserved. The selected payloads are absent from reachable
history, and full Git integrity verification passes after object reclamation.

The tracked tree is approximately 56 MiB; `.git` shrank from 534 MiB to 71 MiB,
with packed objects falling from 514 MiB to 59.6 MiB. An external recovery copy
and commit/ref maps are retained at `/tmp/canic-history-cleanup-20261005T083005Z`.
No push, release transaction, build-cache deletion or recovery-evidence cleanup
ran. Local commit IDs and tags have changed; GitHub still has the original
history. Publishing this rewrite requires a coordinated forced branch/tag update;
an ordinary fetch or pull can reintroduce the original historical objects.
This storage cleanup does not qualify the complete runtime release batch.

## Root membership discovery — 2026-10-05

The maintainer prioritized [Canic#39](https://github.com/dragginzgame/canic/issues/39)
and [Canic#38](https://github.com/dragginzgame/canic/issues/38). The independent
lookup is implemented in this checkout: `canic_root_membership` is a read-only
update admitting Root controllers or active local managed members through normal
Fleet guards. Public request/response DTOs and the protocol constant are exposed
through Canic. The existing Root owner resolves the exact binding, preserves
descendants' own roles, returns `None` for ordinary negative membership, and
retains typed authority failures. It grants no lasting application permission.

Focused qualification is complete on the current locked graph. Both native
error/negative tests and both public control-plane facade tests pass. The exact
active-Registry PocketIC journey passes through fresh and restored fixtures;
the generated-Root initial-Shard journey passes exact descendant roles,
removed-child negatives, loss of removed-caller access and pre-activation
refusal. The latter also compares the generated Candid function structurally
against the public request/result DTOs and update mode. A Prepared Root rejects
through the normal Fleet fence before the handler; admitted lookup failures
retain their typed application errors.

Warning-denied library/test Clippy passes for the internal simulator package
and explicitly for Core, Control Plane and the facade with all features. Scoped
formatting and whitespace checks pass. Final simulator logs are
`target/test-runs/20261005T112409Z-272983.2bltO7/2.log` (36.70s) and
`target/test-runs/20261005T111725Z-202163.94mUW6/2.log` (49.30s). Native/facade
and lint evidence is under `target/review-validation/root-membership-*.log`.
The test-only Candid parser dependency inherits the workspace declaration;
the concurrent `powerfmt` lockfile update is preserved. The earlier public-ID
inventory and testkit reset-policy corrections remain. The existing .53 notes
and authentication documentation describe the lookup and its guard boundary.
The lookup outcome is ready for review; no broad gate or release transaction ran.

The maintainer confirmed on 2026-10-05 that the isolated caller-authority draft
at `.canic/local-work/caller-authority-20261003/source` was deleted during file-count
cleanup. Earlier draft and native-test descriptions below are historical; the
source and its local validation artifacts are unavailable in this checkout.
[Canic#38](https://github.com/dragginzgame/canic/issues/38) records the corrected
implementation status. The subsequent rebuild and qualification are described
in the current caller-authority section above. Neither issue is being claimed
as published; this membership section alone is not whole-worktree push readiness.

## Published blob dependency update — 2026-10-05

The adapter now pins `ic-blob-storage = "=0.14.9"`, confirmed as the latest stable,
non-yanked release through crates.io metadata. The adapter and both consumer
lockfiles select that published crate; unrelated selected versions are retained.
The sole declaration is in `integrations/blob-service/Cargo.toml` under
`workspace.dependencies`. The separate adapter workspace owns this dependency;
neither the main Canic Cargo workspace nor `canic.toml` controls its version.
The composition guide explains this boundary and the .53 draft records the pin.

Strict library Clippy passes for the adapter and both consumer examples. Both
managed Fast Wasm builds and dedicated/embedded PocketIC proofs pass with 0.14.9:
exact certificate replies, refusal without unwanted effects, application/blob
state separation, combined metrics, same-release restoration and operator-only
recovery. Evidence: `target/review-validation/blob-0149-*.log` and
`blob-0149-results.jsonl`. Final artifact SHA-256 values are
`bf58243855c5f461102beb431eadeed75f48fb45f5da8354ffca39c01a54d542`
(dedicated) and
`e27583684607b12cba9d0a445966e5aae5e26df83bc42806602f71afddff3be3`
(embedded). These supersede the preceding 0.14.6 artifact results below.

This dependency adoption is qualified. The broader runtime-argument delivery
work under [Canic#444](https://github.com/dragginzgame/canic/issues/444) remains
open; no live deployment or whole-worktree release qualification is claimed.
Package versions, sibling repositories and release authority are unchanged.

## Optional embedded blob management — 2026-10-05

The maintainer selected both embedded and dedicated placement under
[Canic#444](https://github.com/dragginzgame/canic/issues/444). The adapter exposes
`mount!(memory = 150..=166)`, synchronous `lifecycle::install` / `restore` and
`metrics::sample`. The application retains its single lifecycle, Candid export,
memory bootstrap and sampler. Mounting contributes endpoints and seventeen
memory requests in a host-selected range; typed installation and restoration
do not replace the host's sampler. The dedicated `canister!` macro assembles
these same parts with its existing default range and blob-only sampler.

The new isolated `integrations/blob-service/embedded-consumer` example has its
own durable counter, endpoints and combined metrics. Its counter occupies memory
120, with blob grants at 150–166. Both examples remain outside Canic's default
test graph. No service dependency or production blob mechanism was added to Core.
The [composition guide](../features/blob-storage/README.md) explains both choices
and the shared pattern for other optional adapters; those other adapters are not
implemented by these blob-specific macros.

Adapter and both consumer libraries pass warning-denied Clippy. Both managed
Fast Wasm builds pass after the concurrent timer adoption: the isolated graphs
select service 0.14.6, memory 0.25.0 and timers 0.11.4. Dedicated artifact SHA-256:
`85924410e8d7631a260dd633953f9ab1069ee2cd10d7c30a69afe4b070a01a1a`;
embedded artifact SHA-256:
`64db8896b6f14c68536d86576110e9650d54a23c667d339e7055c8787141e517`.
Both focused PocketIC cases pass with the native driver using query 0.46.1,
testkit 0.15.4, timers 0.11.4 and memory 0.25.1. The embedded case additionally
proves independent application state and endpoints, both metric families,
noninterference from refused and accepted blob mutations, and restoration of
both owners before subsequent application and service updates. Both forms retain
exact plain certificate replies, access/decoder refusal and operator-only recovery.

Qualification corrected an invalid hyphenated stable key in the new example;
the native bootstrap probe exposed the typed key-validation failure behind
`init: E137`. The corrected `embedded_app.counter.v1` commits separately at 120,
with blob grants at 150–166. Final Clippy, managed build and PocketIC checks pass.
Evidence is under `target/review-validation/blob-embedding-*`; the results JSONL
retains command outcomes and timings. Earlier native results below retain their
original dependency scope. The embedding implementation and its focused
qualification are complete; this is not a whole-worktree push-readiness claim.

Applications may construct typed blob configuration from their compiled settings
and actual hosting Principal, or decode bounded external installation input.
Root's production argument-delivery gap remains for runtime-supplied arguments,
including the contracts chosen by both examples. This work does not qualify a
Toko deployment, paid uploads, provider capacity or arbitrary terabyte workloads.
The .53 draft includes the composition change; package versions remain .52.
No broad suite, sibling edit, release transaction, deployment or Git publication ran.

## Combined .53 qualification and production delivery boundary — 2026-10-04

The current combined checkout passes 112 selected native regressions and all
12 facade doctests: Core activation/attestation, Host configuration/evidence/
provenance/transport, CLI inspection/configuration/funding/help, and public
identifier/protocol/performance surfaces. Warning-denied Clippy passes for Core,
Host, CLI and facade library/test targets. This supersedes the pending native
qualification statements in the independent issue-review checkpoint below.
One stale CLI text assertion still expected the removed explicit-input source;
it now checks the maintained current-inventory source, matching the existing
fixture and JSON contract. Logs are `target/review-validation/053-*.log`;
`053-targeted-results.jsonl` records commands, outcomes and timings. Compilation
used one Cargo job and command-local Host profile overrides; repository build
profiles were unchanged. The embedded peer was stale after the dependency patch;
the governed refresh and final read-only verification now pass on memory 0.25.1.
Its artifact SHA-256 is
`9a17edf8b847f749a8b48603c91d21de8c720baab72465030a0703391585d47d`.
Evidence: `053-fixture-refresh.log` and `053-fixture-final-verify.log`.

The focused blob consumer PocketIC proof also passes on the rebuilt service
0.14.6 / memory 0.25.0 Wasm, with memory 0.25.1 in the native driver. It covers
actual-Principal-bound installation, exact plain certificate replies, access
and decoding refusal without tenant effects, usage metrics, same-release restore
and operator-only recovery. Evidence:
`target/review-validation/blob-consumer-pocketic-025.log` and the successful
post-refresh rerun `blob-consumer-pocketic-025-final.log`. The final simulator
run needed local loopback networking after the sandbox refused PocketIC's port.
This supersedes the earlier absence of installed evidence on the new dependency
graph. Targeted qualification of the existing repairs and extraction is complete;
the full production blob integration remains incomplete for the reason below.

The production trace nevertheless found a real integration gap, tracked in
[Canic#444](https://github.com/dragginzgame/canic/issues/444): Root's top-level
Component installer, including Component Group members, passes no application
arguments. `ManagedApplicationInit::ForCanister` is fixture injection only.
Production needs bounded post-allocation argument delivery, exact operation and
canister binding, durable bytes before installation, immutable retries and
Root/Host integration. Changing `app.init_mode` does not supply arguments; it
selects the initial Fleet operating mode and defaults to Enabled. The blob and
qualification guides now expose this limitation. Do not treat the fixture proof
as a successful production Toko deployment or the complete blob rollout as ready.

The 0.110.53 changelog remains the accumulated draft; package versions remain
0.110.52. No broad suite, sibling edit, paid provider call, deployment, version
transaction, commit or Git publication ran.

## Checked memory slots — 2026-10-04

Core adopts published ic-memory 0.25.0: allocation policies consume
`MemoryManagerSlot`, diagnostic projection reads its infallible ID, and the
slot-error adapter is removed. The main manifest and lockfile already selected
this release before the source adoption. Canic retains namespace, range,
reservation and admission decisions; a checked ID is not allocation authority.

The exact source adoption passes 32 focused policy, runtime-memory, public
memory-metrics and stable-memory ABI tests against the published crate, strict
Core library/test Clippy and default-feature Wasm compilation with Rust 1.99.0.
Checks ran sequentially in an isolated source snapshot with an isolated target;
the live adopted Rust files and main manifest/lockfile match the qualified inputs.
Logs are `/tmp/canic-published-0.25-{policy,runtime,metrics,abi,clippy,wasm}.log`.
The producer retains the qualification record in
`docs/consumer-qualification-0.25.0-canic.md`. The generated allocation peer was
refused as stale, then refreshed through its governed helper. Final read-only
verification passes; its artifact SHA-256 is
`24cd00206abf8c5249e9ea9aa905fe8d34883d7c6d3ff53743281a25bf1f28bf`.
Logs are `/tmp/canic-0.25-embedded-{verify,refresh,final-verify}.log`.

The independent blob adapter now selects published ic-blob-storage 0.14.6 and
ic-memory 0.25.0. Both isolated lockfiles are aligned, and the consumer's Wasm
normal-dependency graph contains one ic-memory package identity. Earlier
installed/PocketIC and Toko-copy evidence retains its original 0.24 graph;
dependency alignment alone does not requalify installed or provider behavior.

On the current graph, the adapter and consumer libraries pass Rust 1.99.0
Clippy with `--locked --offline --lib -- -D warnings`. The consumer also passes
the governed Fast-profile `canic build consumer-app blob`: declaration
extraction, role admission, runtime compilation and artifact finalization all
complete. Its Wasm SHA-256 is
`d9be333370c94a07bf60fd4f3fab5dc68968c1ad4dc1341dcb3daf1e02104666`.
This uses the existing CLI reporting 0.110.52, not a freshly rebuilt CLI.
Logs are `/tmp/canic-blob-0.25-{library-clippy,consumer-clippy,managed-build}.log`;
the completed build result is `/tmp/canic-blob-0.25-managed-build.json`.
At that checkpoint no installed lifecycle proof, live deployment or paid provider
effect ran; the combined qualification above adds the subsequent simulator proof.

## Reusable blob composition — 2026-10-04

The reusable-library portion of
[Canic#444](https://github.com/dragginzgame/canic/issues/444) is implemented in
this working tree; production argument delivery remains open as described above.
`integrations/blob-service` is a reusable library;
`canic_blob_service::canister!()` emits the managed endpoints, sole lifecycle and
service memory registration into a consumer-owned shell. The shell owns exact
App/role metadata and compiled topology. Upstream DTO imports alone register no
service memory. Host admits sharing the exact selected public Canic facade while
retaining App/role, feature-closure, protected-internal and memory-identity checks.
The adapter remains unpublished and must select the same checkout's Canic facade
as the consumer. Its upstream dependency stays outside the main workspace.

Before the concurrent 0.25 adoption, targeted evidence passed: 36 Host
package-contract tests, four generic managed
Component fixture tests, warning-denied Host/facade and isolated library/consumer
Clippy, and managed Fast-profile builds for the checked-in consumer and a
Canic-owned copy of Toko's actual topology with a blob Component. The final Toko
copy reports 4.38 MiB of Wasm code. Toko's repository was not modified or deployed.
The generic fixture now accepts `ManagedApplicationInit::ForCanister` to encode
nested arguments after the actual Principal is allocated; Root-provided child
arguments retain precedence and protected framework payloads remain unchanged.

On that 0.24 graph, a focused PocketIC consumer proof passed actual-Principal
installation, exact
plain certificate Candid reply shape, authorized/denied reads and mutation,
oversized/malformed input refusal without tenant effects, aggregate per-canister
metrics, same-release restoration and authenticated current-instance recovery.
Restoration preserves configuration/enrollment, keeps the upstream fence, and
rebuilds sampling; only the operator can resume before a subsequent mutation.
Framework setup can advance the platform version: the next service mutation
proves continuity through the existing upstream path. No service fence was
weakened. This temporary proof remains under `target/review-validation/`; no blob
suite, runner or development dependency was restored to the default test graph.

Evidence logs are `blob-consumer-pocketic.log`,
`blob-composition-{graph-tests,fixture-tests,framework-clippy}.log`,
`blob-consumer-{clippy,managed-build}.log` and `blob-toko-managed-build.log` under
`target/review-validation/`. Embedded-peer refresh passed on the earlier graph.
At that checkpoint, read-only verification failed after its source snapshot
caught the concurrent 0.25 manifest change before the Core API adaptation.
Published service 0.14.5 still required 0.24. The checked-slot handoff above
records the later Core adoption, fixture refresh and published 0.14.6 alignment.
The earlier installed/PocketIC and Toko-copy results retain their preceding
0.24 graph and do not qualify the new combined tree.

The maintainer selected 0.110.53 for the accumulated changelog draft; package
versions remain unchanged. Toko adoption, live provider behavior and paid uploads
remain unqualified. Independent issue-review work elsewhere in this handoff is
outside this blob batch. No broad suite, live deployment, release transaction,
commit or Git publication ran.

## Allocation registration cleanup — 2026-10-04

Canic's key and range macros now delegate registration to ic-memory. The two
hidden registration constructors are removed; Canic retains its authority
constants, per-thread native readiness hook and separate admission registration.
Admission policy selection still precedes ic-memory declaration sealing. Invalid
registrations now surface as bootstrap errors before allocation authority is
published. The stable-memory guide reflects the current 0.24 dependency line.
Root Unreleased records this change without assigning a release version.

With the locked ic-memory 0.24.7 graph, all 14 Core runtime-memory tests, the
control-plane lazy-store bootstrap regression and all four stable-memory ABI
guards pass. Validation used an isolated target at
`/tmp/canic-memory-consolidation-target` to avoid the concurrent shared build.
An isolated macro probe also verifies the declaration snapshot, literal and
constant authority forms, range modes, nested type labels, per-thread readiness,
invalid-range refusal without committed authority and Wasm compilation.
Evidence logs are `/tmp/canic-memory-consolidation-*.log`. These are scoped memory
results; the remaining blob and independent issue-review qualification below
still belongs to those batches. No dependency, package version or generated
binding changed in this cleanup.

## Blob extraction — 2026-10-04

The maintainer explicitly selected blob removal after publishing 0.110.52.
The embedded runtime, provider client, CLI group, Medic billing option, stable
allocations, two fixture canisters and dedicated default test/CI lane are removed
from the working tree. The independent published service is composed only under
`integrations/blob-service`, with its own Cargo workspace, lockfile and target.
No sibling repository, live service, retained operation or release artifact was
modified. This selection supersedes the earlier future-work ordering below;
it does not close FR1, accept minor closeout or assign a release version.

At the maintainer's request, all remaining blob-specific adapter tests and their
runner, standalone test mode and test-only dependencies are removed. The unused
managed-fixture argument helper added solely for that suite is withdrawn; existing
Canic-owned generic tests remain. Upstream already covers service authority,
certificate replies, restoration, snapshots, expired history and release readback.
The missing decoder-budget regression request is
[ic-blob-storage#7](https://github.com/dragginzgame/ic-blob-storage/issues/7).
At that extraction checkpoint the service dependency was exactly
`ic-blob-storage = 0.14.1`.
After removal, formatting, whitespace, test inventory and locked offline adapter
metadata checks pass. The adapter has no test targets, development dependencies
or test features; its refreshed lockfile no longer includes PocketIC or ic-testkit.
No compilation or simulator was started for this removal.

Earlier targeted Core/facade/Host/CLI Clippy, 30 Core role-contract tests and 48
Host role-contract tests passed. Host fixture subprocesses needed
`CARGO_NET_OFFLINE=true` in the sandbox. Runner barriers, test inventory, scoped
shell lint and document semantics passed. Adapter native Clippy passed before
its test scaffolding was removed. These are scoped results, not a full gate.
Evidence is under `target/review-validation/blob-*`.

The preceding standalone CLI rebuild passed with one Cargo job (10m 41s). That adapter
also passes the documented managed Fast-profile build, including declaration,
runtime, Candid extraction and finalization. Qualification caught and fixed the
isolated workspace's missing `profile.fast.inherits` declaration. The resulting
artifact reports 4.36 MiB of Wasm code, with 5.64 MiB of headroom beneath the
10 MiB limit. Logs and the structured build result are
`target/review-validation/blob-cli-build.log` and `blob-managed-build.{log,json}`.
Artifacts remain under `integrations/blob-service/.icp/local/canisters/blob/`.
No canister was installed and no service/provider transaction ran.

There is no blob-specific test command to resume. All 12 selected facade
regressions now pass: workspace/package metadata, current exports, metrics and
control-plane APIs. Evidence:
`target/review-validation/blob-facade-targeted-tests.log`. This replaces the earlier
SIGKILLed facade compile; the concurrent Host/CLI request-field mismatch is also
resolved. Embedded-peer refresh and read-only verification now both pass. The
checked-in Wasm and `scripts/dev/managed-root-fixture.json` bind the current
source; the artifact SHA-256 is
`c6cb76063412c09787f9bc9bddd4ba5b7218ce4c7349be1a526befd0dee6fe29`.

The initial refresh attempt received SIGKILL while compiling Host, without a
Rust diagnostic. The successful helper build used one Cargo job and command-local
`profile.dev.package.canic-host` overrides: `opt-level=0`, `codegen-units=64`,
`debug=0`. The fixture itself still used the ordinary Fast Wasm profile. Logs:
`target/review-validation/blob-embedded-root-refresh-low-memory.log` and
`blob-embedded-root-verify.log`; the failed attempt is retained separately.
No repository profile or dependency changed for that workaround.

The preceding extraction/reporting batch's targeted qualification was complete. The combined
checkout still has the independent issue-review validation described below; this
is not a whole-worktree push-readiness claim. Package versions and published .52
notes are unchanged; the current 0.110.53 draft now includes the extraction.
No full suite, deployment, release transaction or Git publication ran.

## Application usage reporting — 2026-10-04

The maintainer selected usage reporting after blob extraction. The isolated
adapter now registers a bounded synchronous application sampler after install
and restore, reading upstream-maintained global upload counters. Its config
explicitly publishes the application family. Byte gauges distinguish logical,
physical, liability and reserved accounting; reservation bytes are already
included in the first three. No scans, paid calls, shadow ledger, new timer or
blob-specific tests are added. This is per-storage-canister reporting, not a
public tenant breakdown or provider qualification.

Observatory collects one bounded application-cache page per retained role and
includes generic application metrics in private JSON and public JSON/HTML.
Public views remove Principal dimensions and omit rows attributed to another
canister, marking that page partial. Names that collide after dimension redaction
are also omitted publicly; the private source rows remain intact.
Source age, disabled/unavailable states,
exact values and truncation survive projection. Shared metric views replace
cost-only names without changing cost JSON fields. Source timestamp validation
now accepts rows newer than the family's oldest sample, matching Core's cache.

Validation passes: all 34 Host `observatory::` tests, Host library/test Clippy and
isolated adapter library Clippy, both with warnings denied. Scoped formatting,
whitespace, documentation links and document semantics also pass with zero
advisory warnings. Logs are retained at
`target/review-validation/application-metrics-{host-tests,host-clippy,adapter-clippy}.log`.
The adapter check also removed an obsolete lint expectation; Host lint corrected
redundant visibility in the existing package-path helper without widening its
effective crate-only exposure. No dependencies or lockfiles changed in this slice.

The reporting slice and remaining extraction qualification above are complete.
Independent issue-review work still needs its recorded validation. No full suite,
simulator, deployment,
paid call, version transaction or Git publication ran. The blob composition and
Observatory guides and root Unreleased are updated; package versions and
published .52 notes are unchanged.

## Backup package direction — 2026-10-04

Delegated authentication uses the existing Canic implementation. No delegated-auth
extraction or external dependency adoption is pending.

Generic backup artifacts, manifests, execution journals, capture/restore runners
and retention belong upstream. Canic supplies Fleet inventory, release/controller
authority, application quiescence and operator command/report integration on the
host. The current upstream package covers local artifacts, persistence, locks,
restore references and command custody; runners, transport and terminal release/
prune remain incomplete. Canic's live Component Registry preflight also still
rejects. Extraction must resolve that integration gap rather than copy it.

Replace backup mechanisms only as the upstream executor contracts become usable
and qualified; each replacement removes the corresponding duplicate
implementation. Upstream owns product tests, while Canic retains focused Fleet
authority, lifecycle and recovery integration evidence. Remaining blob
qualification and existing release/minor boundaries are unchanged.

The backup contract covers live membership paging, controller/read evidence,
capture consistency, fresh nonterminal preflight, bounded execution, custody and
same-release restore. The current upstream runners remain unavailable. A scoped
CLI correction for [#394](https://github.com/dragginzgame/canic/issues/394)
now returns `LiveCreateUnavailable` before workspace discovery, layout creation
or ICP invocation. It removes the unusable executor branch and the unobserved
`Proven` authority constructors; dry-run plans retain declared authority.
All 62 focused CLI backup tests pass, including refusal before new output or
retained evidence can change. Log:
`target/review-validation/backup-create-boundary-tests.log`. CLI library/test
Clippy also passes with warnings denied; evidence is in
`target/review-validation/backup-create-boundary-clippy.log`. This correction
does not implement live preflight or close the underlying backup availability
issue.

## Independent issue-review implementation — 2026-10-04

The independent CLI repair for
[#299](https://github.com/dragginzgame/canic/issues/299) deletes the nonfunctional
explicit Canic-inspection command, its options/parser/target and help branches,
and obsolete fixtures. Existing JSON assertions now use the maintained Fleet
identity, replacing the duplicate Fleet fixture test. Fleet and management
inspection remain the maintained surface; no replacement path or helper was
added. The inspection module shrinks by 112 lines, including 58 production
lines. The active ICP guide and root Unreleased reflect this hard cut.
Source review, scoped formatting and whitespace checks pass; no build or native
test ran. Required qualification: existing CLI `inspect::tests::`, management
inspection tests and the recursive help ordering test. This repair remains open
pending the build owner's targeted checks with the issue-review batch.

The independent documentation correction for
[#373](https://github.com/dragginzgame/canic/issues/373) makes all four network
enrollment guides obtain the expected fingerprint from an authenticated operator
publication or independent trusted channel. Examples retain that trusted value
separately from the local file's comparison digest and pass it to enrollment.
Current enrollment validates the supplied digest before writing trust authority;
the guides now describe this boundary accurately. Documentation links, example
shell syntax with substituted placeholders and scoped whitespace checks pass.
No runtime source changed and no build or enrollment command ran.
Root Unreleased includes the guidance correction.

The independent facade repair for
[#324](https://github.com/dragginzgame/canic/issues/324) exposes the Core identifier
types needed by public fields and constructors, including App/release identities,
admission authority, funding policy and typed parsing failures. The new
`ids_facade` integration target constructs and Candid-roundtrips a managed init
payload using supported facade paths only and checks typed release-ID failures.
Existing protocol ID contracts now use `canic::ids` instead of Core imports.
Required facade targets: `ids_facade` and the existing `protocol_surface` ID/wire
regressions. Formatting and scoped whitespace checks pass; no build or native
test ran. Targeted native qualification remains required before the combined
issue-review batch is ready. Root Unreleased records this public API repair.

The independent Core repair for
[#134](https://github.com/dragginzgame/canic/issues/134) rejects an all-zero
installation operation ID in Root and ordinary application preparation with a
typed `InstallIdZero` error, before activation state is constructed or persisted.
New boundary regressions cover zero refusal for all three installation owners
and exact preservation of first-byte, last-byte and all-one nonzero IDs. Required
Core selector: `model::fleet_activation::tests::`; existing storage activation
regressions remain relevant to the caller boundary. Formatting and scoped
whitespace checks pass; no build or native test ran. This repair still requires
targeted native qualification with the independent issue-review batch.
Root Unreleased records the behavior without assigning a release version.

Documentation corrections for
[#326](https://github.com/dragginzgame/canic/issues/326) and
[#328](https://github.com/dragginzgame/canic/issues/328) complete the facade's
application lifecycle example with all three async hooks and `finish!`, and
describe optional init-block execution on initial activation/local installation
and active same-release upgrades. The application auth-feature table now leaves
Root feature derivation to the host; existing attestation and local-authorization
requirements match the capability catalogue. README, crate and macro docs were
verified against current macro expansion and role requirements; formatting,
local links and scoped whitespace checks pass. No build or doctest ran, and
executable behavior is unchanged. Root Unreleased includes this guidance.

Documentation corrections for
[#343](https://github.com/dragginzgame/canic/issues/343) and
[#370](https://github.com/dragginzgame/canic/issues/370) align TESTING.md with the
internal artifact harness's command-scoped, cache-bound build configuration
selection and make the onboarding and macro README endpoints explicitly public.
The README uses the facade and a fallible reply under the default Fleet guard.
Current parser/expansion, fixture builders, cache inputs and documentation links
were checked from source; no build or doctest ran. Root Unreleased records these
documentation corrections; runtime and fixture source are unchanged.

The independent CLI repair for
[#296](https://github.com/dragginzgame/canic/issues/296) resolves missing-role
diagnostics from the verified Fleet Registry's App identity instead of the Fleet
name. Configuration discovery, selection and loading failures now emit a warning
while preserving the inventory table. New regressions cover differently named
Fleet/App identities, multiple App configs, missing roles, unknown Apps and invalid
selected configuration. Required CLI selector: `list::config::tests::`.
Formatting and scoped whitespace checks pass; no build or native test ran.
The combined issue-review batch still requires its targeted native qualification;
root Unreleased includes the repaired diagnostic without assigning a version.

The independent CLI repair for
[#305](https://github.com/dragginzgame/canic/issues/305) checks the observed operator
balance before requesting a retained-operation conversion quote. Covered debits
return a typed outcome carrying the original plan or supplementary funding
resume digest. No conversion review is retained on this path. New boundary
regressions cover equal/excess balances, exact resume identity, positive
shortfalls and zero-rate/overflow quote arithmetic. Required CLI selector:
`fleet::operator_mint::tests::`. Formatting and scoped whitespace checks pass;
no build, Cargo check or native test ran. The operating guide and root Unreleased
describe the corrected outcome.

Additional independent repairs are implemented for
[#217](https://github.com/dragginzgame/canic/issues/217),
[#220](https://github.com/dragginzgame/canic/issues/220) and
[#221](https://github.com/dragginzgame/canic/issues/221). Role rename shares the
canonical package-path resolver and propagates read/parse failures before
configuration publication. Policy requests carry exact envelope source;
single-envelope and workspace-manifest gates fingerprint the consumed source
bytes, with byte-derived sizes and no invented modification time. File-backed
fingerprints obtain metadata from the opened file. Unreadable or malformed
manifest evidence yields individual required failures or optional warnings,
preserving every other entry's report.

New Host regressions cover relative/absolute directory and Cargo.toml selectors,
unchanged configuration after missing/malformed package failure, replacement
files after input consumption, and complete mixed valid/invalid entry reports.
Formatting and scoped whitespace checks pass; no builds or native tests ran.
Required Host selectors: `release_set::config::tests::role_rename_`,
`policy_gate::tests::envelope::` and `policy_gate::tests::manifest::`, plus existing
`evidence_envelope::tests::` and CLI `evidence::tests::gate_` for the changed input
boundary. The policy guide documents fingerprints and per-entry outcomes.
The native-timers guide now restricts application lifecycle participants to the
macros that support them; this documentation correction is verified against the
current macro declarations without compilation.

Further independent repairs are implemented for
[#71](https://github.com/dragginzgame/canic/issues/71),
[#239](https://github.com/dragginzgame/canic/issues/239) and
[#242](https://github.com/dragginzgame/canic/issues/242). Application attestation
arguments can only raise the configured role epoch floor. Fleet status accepts
only the known Running, Stopped and Stopping states; unknown text retains its
value in a typed refusal. All four operator balance observation paths share an
observation error preserving the lower ICP balance failure.

The new Core boundary-matrix regression and Host process fixtures cover epoch
floor tightening, known/unknown status values, successful balance preparation,
malformed balance/JSON and invocation failure without payment commands.
Formatting and scoped whitespace checks pass; these regressions are unrun here
under the maintainer's no-build instruction. Required selectors:
`workflow::runtime::auth::tests::application_epoch_floor_can_only_tighten_configured_revocation`
in Core, and
`fleet_ensure::ops::platform::tests::status_observation_rejects_unknown_runtime_states`
and
`fleet_ensure::ops::platform::tests::estate_preparation_preserves_typed_balance_observation_failures`
in Host. The API and configuration guide document the epoch-floor semantics.

Additional Host repairs are implemented for
[#215](https://github.com/dragginzgame/canic/issues/215),
[#218](https://github.com/dragginzgame/canic/issues/218) and
[#222](https://github.com/dragginzgame/canic/issues/222). Source provenance
explicitly observes untracked files and submodules, disables filesystem-monitor
shortcuts and removes injected Git configuration. Generated family patches use
TOML serialization and reject non-UTF-8 paths rather than losing path bytes.
Every enabled build-provenance rule rejects Failed and NotRecorded payloads even
when the envelope reports Success. Envelope-only policies retain their scope.

New regressions exercise actual Git status under a hiding local configuration,
injected Git environment, all three generated infrastructure package manifests
under a quoted/backslashed/control-character path, and each enabled policy rule's
success and failure cases. Formatting and scoped whitespace checks pass. No build,
Cargo check, Clippy or native regression was run for these additions; the other
session owns compilation and targeted validation. Required Host selectors:
`build_provenance::source::tests::`,
`fleet_package::tests::generated_packages_preserve_special_characters_in_dependency_patch_paths`
and `policy_gate::tests::build_provenance::`. These checks and the transport
regressions below must pass before the combined issue-review batch is ready.
Root Unreleased and the active provenance/policy guides reflect these changes.

The public performance macro repair for [#320](https://github.com/dragginzgame/canic/issues/320)
qualifies the Perf topic, corrects the log arguments and directs the prelude and
operational macro paths to the facade implementation. Its exact native checkpoint
regression and all 12 Canic doctests passed in an isolated copy of published
0.110.52 plus this repair. Logs: `/tmp/canic-perf-review-{unit,doc}.log`.
This evidence does not qualify the concurrently changing blob-removal graph.

The transport repair for [#273](https://github.com/dragginzgame/canic/issues/273)
makes Candid argument cleanup non-fatal in typed ICP, Canister Protocol and
Observatory adapters. Non-missing cleanup failures emit a local warning; original
replies, transport failures and response-validation failures retain their outcomes.
The added real-process native regressions cover missing/replaced scratch with
success, invocation failure and invalid responses. Their Host compilation was
interrupted before execution when the maintainer assigned all builds and targeted
checkout checks to the blob-removal session. Host verification remains required:
`canister_protocol::tests::cleanup_failure_preserves_typed_update_and_query_outcomes`
and `icp::candid::tests::child_reads_complete_arguments_and_cleanup_preserves_transport_outcomes`.
Affected-package Clippy also remains unrun here. This session has no active build
and will start no further builds or Cargo checks.

Changes remain uncommitted and the root Unreleased notes include these repairs.
The issue-review batch awaits its owning checks; no combined push-readiness or
publication claim is made. Existing published changelog entries, dependency
selection and concurrent blob-removal edits are preserved.

## Embedded allocation-peer refresh — 2026-10-04

The release test preflight stopped before ordinary tests or PocketIC because
its freshly built allocation peer differed from the checked-in Wasm. Refresh
and the exact `verify_embedded_root` command now both pass on the selected
memory 0.24.2, query 0.45.6, timers 0.10.6 and testkit 0.14.1 graph. The Wasm and
`scripts/dev/managed-root-fixture.json` are updated together. Artifact SHA-256:
`89303d639958976cdffd9685d330f14d4b970b4df0c0a3319e6bb7dc6c8284d6`.
The manifest and lockfile stayed byte-identical throughout the repair. Release
and Cargo build artifacts are retained. The .52 changelog reflects the refresh
and current memory minor.

This fixes the reported preflight blocker; the maintainer can retry the release
flow. It is not a full-suite result or FR1 completion. No broad gate, version
bump, commit, push or deployment ran. This session has finished its builds.
Logs: `target/review-validation/canic-embedded-{refresh,verify}-20261004.log`.

## Cargo workspace inheritance — 2026-10-04

All 49 checked-in Cargo manifests now source package versions and dependency
specifications from their owning workspaces. The main workspace already supplied
package versions; three audit/sandbox canisters now also inherit their Canic
runtime/build dependencies. Five isolated role-fixture packages inherit from
three fixture workspace roots. The standalone historical audit harness inherits
its original exact pins from its own workspace tables; no dependency version or
package identity was changed by this cleanup. Generated build artifacts are not
edited.

The existing version-inheritance test now covers example Apps and no longer
allows local path exceptions. AGENTS records the ownership rule and .52 includes
it. All five workspace-manifest tests, locked metadata, structural checks across
all manifests and current-document semantics pass. All ten Host isolated-fixture
regressions pass, including resolver selection, protected sibling detection and
renamed dependency rejection. The batch and its .52 changelog are complete;
FR1 remains unfinished. This session has finished its builds. Logs are retained
under `target/review-validation/canic-workspace-versions-*`. No broad suite,
version bump, commit, push or deployment was performed.

## IcyDB removal — 2026-10-04

Follow-up removal audit: 5,247 Rust lines deleted and 174 added, a net reduction
of 5,073 lines including comments and blank lines, excluding the separate Cargo
fixture-lock fixes. All 2,279 maintained non-document files were searched; current
source, tests, build configuration and CI have no IcyDB references. Locked metadata
and the lockfile have no IcyDB package, target or dependency edge. Call tracing
found one orphaned `fetch_in_flight` qualification hook; its API, workflow, ops
and model accessors are now removed. Generic application fixture-import contracts
remain framework-owned. Historical documentation and build caches are retained.
All eight fixture-importer regressions pass after hook removal, including lease
serialization, stale cleanup fencing and exact receipt checks. This narrow rerun
used the concurrently selected memory 0.24.1 / timers 0.10.5 graph; earlier
PocketIC evidence below retains its original dependency selection. Scoped
formatting and whitespace checks pass.

The maintainer explicitly requested removal of all IcyDB testing. Canic's
workspace and lockfile no longer contain IcyDB, its schema, its lifecycle actor
or its local Cargo override. The independent audit probe and database-specific
integration suite are removed. The three mandatory managed lifecycle/admission
journeys now use `managed_lifecycle_probe`, a small Canic-only actor preserving
explicit guard parity, denied-dispatch accounting, transition replay and public
managed-App support. Framework Store response-barrier support remains because a
Fleet recovery journey still uses it. No sibling repository was modified.

Default workspace selection now includes every maintained package and feature;
the external-composition lane and two-pass Clippy exception are removed. The
framework no longer owns database row/checkpoint and consumer reinstall proof;
those are downstream qualification obligations. Active docs and AGENTS describe
that boundary. Historical reports remain evidence, with links to deleted fixture
sources pinned to the prior immutable Git snapshot. .52 includes the removal.

Locked metadata contains zero IcyDB packages, edges or sibling paths. Resolution
removed 20 package identities and added only the Canic-owned probe; retained
package versions did not change. Shell runner regressions, inventory, scoped
ShellCheck, 15 manifest/endpoint tests, 15 timer inventory tests, Host fixture
ownership and warning-denied replacement-probe/internal harness/integration-package
Clippy pass. All
three formerly database-backed mandatory PocketIC journeys pass with the Canic-only
actor. The retained Root/Store reply-recovery PocketIC proof also passes (four
targeted simulator cases total). Embedded allocation-peer verification passes
without changing the checked-in artifact. Logs use
`target/review-validation/canic-no-icydb-*`.
Previous IcyDB qualification/override blockers below are superseded by this cut.
The IcyDB-removal batch and its .52 changelog are complete; FR1 whole-Fleet
execution remains unfinished. This session has finished its builds. No broad
suite, version bump, commit, push or deployment was performed.

## .52 release error review and changelog consolidation — 2026-10-04

The latest retained release run (20261003T185713Z-55976) passed Clippy but
failed six native Cargo-fixture tests across `canic/build_cfg_surface` and Host.
Their manually written lockfiles omitted inherited unused local patches; Cargo
refused the locked operations when scratch lived beneath the workstation IcyDB
override. The build-macro failure reproduces under the real scratch runner.
All six fixtures now generate their initial lockfiles offline before exercising
unchanged locked operations. Reuse mutation coverage modifies the resolved lock
instead of discarding its patch records. No dependency selection, sibling source,
production behavior or paid operation changed.

All six failed tests now pass under that same scratch runner and local override,
along with the companion build-cfg check and changelog structure test. Scoped
formatting, whitespace and document semantics pass. Warning-denied Host
library/test Clippy with all features and focused build-cfg/changelog Clippy pass.
Evidence is retained under `target/review-validation/canic052-*.log`.
The .52 root summary and detailed notes now own all pending changelog content,
including clear limitations for unfinished FR1 execution and receiver-local
authorization. Earlier published entries and package versions are unchanged.

This is targeted qualification of the reported failures. The release run stopped
at its native-test barrier before PocketIC; no new full-suite result is claimed.
The .52 changelog is prepared for the selected checkpoint, while complete FR1
remains unfinished. Prior optional-composition qualification limits remain as
recorded below and do not establish a production dependency defect. No commit,
version bump, push or deployment was performed. This session has finished its builds.

## ic-query 0.45.4 catalog fixture correction — 2026-10-03

The release Clippy failure came from ic-query removing the catalog collection
constructor and evidence builder. Both Canic callers now construct its public
fields directly: Host source tests and the governed growth catalog fixture.
Registry evidence, routing source and endpoint agreement behavior are unchanged.
All ten Host catalog tests pass. Warning-denied Clippy for both affected packages,
including tests and all features, passes with the selected ic-timers 0.10.2 patch;
that concurrent lockfile update is preserved. The Host test run preceded the timer
patch selection. The exact native growth-catalog regression also passes with
timers 0.10.2, preserving Host agreement assurance and route resolution. Formatting,
whitespace and current-document semantics pass. Logs are retained under
`target/review-validation/canic-query0454-*.log`. Both .52 changelog views are updated.

This corrects the reported compiler blocker, not the complete release batch.
The prior three IcyDB-backed cases remain unqualified, the workstation override
still needs portable CI configuration, and FR1 whole-Fleet execution remains
unfinished. No broad suite, version bump, commit, push or deployment ran.

## Testkit 0.14 fixture pool migration — 2026-10-03

The prepared `/tmp/ic-testkit-0.14-migrations/canic.patch` is now applied to
`pic_ingress_payload_limits`: the pool owns its builder at construction, and
acquisition uses `acquire()` without a replacement builder. Its dependency hunk
was already present. Targeted Clippy passes with warnings denied. All six
payload-limit PocketIC integration tests pass (51.18s tests, 118s runner),
including one pool construction and five snapshot restorations without rebuilds.
Embedded peer refresh and the runner's verification preflight pass; artifact
SHA-256 is `44e0bce396ab1d224ce86ccf0df315dfb7e72f41e719e5c4fc42073e8c3a74d0`.
Formatting, whitespace and current-document semantics pass. Manifest and lockfile
remained byte-identical during final validation. Evidence is retained under
`target/review-validation/canic-testkit014-*.log`. The .52 changelog records the
migration and current dependency selection. This session has finished its builds.

Current selection is ic-memory 0.23.0, ic-query 0.45.4, ic-timers 0.10.1 and
ic-testkit 0.14.0. Local IcyDB 0.264.7 has adopted memory 0.23 and timers 0.10;
selected Wasm metadata now has one identity for each, superseding the dual-runtime
observation below. This metadata result is not yet qualification of the three
previously failing IcyDB-backed cases. The workstation Cargo override remains
local-only, and this session did not mutate the sibling repository. This focused
migration is complete; the complete release batch still needs the previously
failing composition cases qualified and a portable dependency configuration.
No broad suite, version bump, commit, push or deployment was performed.

## Local IcyDB test selection — 2026-10-03

The maintainer requested local IcyDB and confirmed its dependency update is still
underway. `.cargo/config.toml` now has a workstation-only Cargo patch selecting
`/home/adam/projects/icydb/crates/icydb`. Cargo resolves IcyDB 0.264.7 and
its supporting crates from that read-only sibling checkout. `Cargo.lock` is
refreshed and locked/offline Wasm-target metadata resolution passes for this
snapshot. No sibling files were modified.

This does not yet qualify composition: local IcyDB still selects ic-memory 0.22
and ic-timers 0.9, while current Canic selects 0.23 and 0.10. The earlier three
normal governed cases still reach the IcyDB fixture. Their dual-runtime refusal
remains expected until the upstream update finishes; no expensive test rerun was
started. Canic-owned production graphs remain distinct from optional composition.
The source switch alone does not fix the default-suite ownership defect.

The absolute Cargo override and local-source lock entries are local development
state, not portable release configuration. Before a portable CI/release run,
remove the workstation patch and resolve the intended published graph, or
separately provide explicit local-source CI setup. Do not commit the absolute
patch table. After upstream edits finish, refresh the selected lock graph and
rerun the affected exact cases. Earlier push-readiness statements below predate
this newly exposed composition blocker and the current dependency edits.

## ic-memory 0.23 reader adoption — 2026-10-03

The manifest now requests ic-memory 0.23. Apply its diagnostic hard cut directly:
the memory ledger adapter consumes `GenerationRecord` and no longer imports or
unwraps `DiagnosticGeneration`. The focused Candid regression preserves all five
public generation fields. This exact source patch previously passed 14 Core
memory tests and strict Core all-target/all-feature Clippy in an isolated copy
against the local release candidate; it is not published-graph qualification.

Concurrent dependency edits are preserved. At this handoff the manifest also
requests ic-timers 0.10 and ic-testkit 0.14, while the lockfile still selects
ic-memory 0.22.0, ic-timers 0.9.5 and ic-testkit 0.13.0. The dependency-update owner
must finish the selected lock graph, qualify the reader against that graph, and
refresh/verify the embedded allocation peer. No actual-checkout build or fixture
refresh ran here. Earlier 0.22 qualification below applies to its recorded graph.
No version bump, commit, push, deployment or optional IcyDB alignment ran.

## Test reliability and ic-memory 0.22 qualification — 2026-10-03

The maintainer authorized finishing the 0.22 adoption in this checkout. The
published adapter and matching lockfile are present: ic-memory 0.22.0,
ic-query 0.45.3, ic-timers 0.9.4 and ic-testkit 0.13.0. Earlier dependency
snapshots below are historical; qualification here uses the current selection.
Optional IcyDB remains independent and nonblocking.

The release-reliability correction replaces incidental Candid text assertions
with structural wire-contract checks and exact receipt-consumer file lists with
production layer boundaries. Storage IDs and ownership remain checked. Targeted
internal PocketIC runs now check compiled registration before starting the server
or building journey fixtures; missing registrations and zero executed tests fail
there. The prior lifecycle registration omission remains fixed.

Final-graph qualification passes: 49 protocol tests, four receipt ownership/
allocation checks and 13 native memory regressions. Focused Core and protocol
Clippy pass with warnings denied. Embedded peer refresh and verification pass;
SHA-256 is `b2fa1c7c014ad99530857c8ad6faeaa3eaa02251b46754fb8cf681ff7335e13d`.
The new real-runner registration preflight and exact standalone memory PocketIC
journey both pass, including two identical-Wasm restoration rounds (128.35s case,
217s runner including compilation). Runner shell regressions, scoped ShellCheck,
formatting, whitespace and current-document semantics also pass. Manifest and
lockfile stayed byte-identical throughout final qualification. Retained evidence:
`target/review-validation/canic-test-reliability-*.log`.

These checks address the reported release path; they are not an exhaustive audit
of every test or a full-suite result. The maintainer-selected .52 checkpoint is
ready for the normal release flow, and both changelog views are prepared. Packages
remain .51 for the governed release bump. FR1 whole-Fleet execution remains
unfinished. No broad suite, version, commit, push, deployment or sibling mutation
was performed. This session has finished its builds; retained artifacts remain
available for reuse.

## Published upstream memory diagnostic hard cut — 2026-10-03

Canic now selects published ic-memory 0.22.0 and reads its direct optional
measured size. The obsolete measurement-outcome conversion is removed, and
current measured/unmeasured fixtures preserve Canic's public response shape.
The manifest already selected 0.22; the lockfile was resolved from 0.21.0 to
0.22.0 without changing the independent optional IcyDB dependency schedule.

All 13 native Core memory tests and warning-denied Core all-target/all-feature
Clippy pass in a frozen source copy using the published registry dependency.
The changed reader, tests, manifest and lockfile still match that tested copy.
The locked production Canic Wasm graph selects only ic-memory 0.22.0.
Concurrent Canic work refreshed and verified the embedded allocation peer on
this same lockfile; its artifact SHA-256 is
`b2fa1c7c014ad99530857c8ad6faeaa3eaa02251b46754fb8cf681ff7335e13d`.
Refresh/verification logs are `/tmp/canic-test-reliability-peer-refresh.log`
and `/tmp/canic-test-reliability-peer-verify.log`. Existing open .52 release
notes describe the current reader and dependency selection.

This qualifies the upstream adoption, not the complete .52 batch or optional
IcyDB composition. The lifecycle PocketIC check pending at that handoff has
since passed; final combined qualification is recorded above. Preserve the
protocol-test, fixture, runner and release work. Edits remain
unstaged and uncommitted; no version, commit, push or deployment was performed.

## Governed lifecycle inventory correction — 2026-10-03

The maintainer's release test found that the new standalone memory restoration
journey had a compiled `#[test]` but no governed runner registration. The
inventory guard correctly refused its missing owner; the earlier individual
checkpoint proofs did not check complete compiled membership. Register it in
the runtime lifecycle group so normal execution and worker partitioning include
both identical-Wasm restoration rounds. Preserve the discovered-membership,
unique-owner and recovery-order checks; no guard is relaxed.

The original failure reproduces with the exact native inventory selector.
After correction, all four native runner/inventory/partition checks pass, as
does warning-denied internal library/test Clippy. The exact lifecycle PocketIC
journey passes through `make test-pocketic-case` (162.89s including fixture builds,
170s runner). Formatting, whitespace and document semantics also pass. Evidence
is retained under `target/review-validation/canic-inventory-*.log`, including the
original reproduction. The .52 changelog records its normal-runner coverage.
This confirmed inventory blocker is fixed. The final status check found concurrent
manifest/lock changes to ic-memory 0.21.0 and ic-query 0.45.3 plus an active Canic
build. Preserve those edits; this session's results do not qualify that later
dependency selection. Its owner must finish dependency/fixture qualification
before the combined checkpoint is called ready. No broad suite, version, commit,
push or deployment was run; earlier checkpoint evidence remains scoped as recorded.

## .52 checkpoint qualification — 2026-10-03

The maintainer confirmed dependency edits are finished and needs an early push.
The open .52 draft now includes the implemented FR1 discovery/assessment APIs,
pool-creation uncertainty and release-preflight corrections, alongside the
existing Host/Backup and documentation work. The whole-Fleet release command
remains unfinished; this checkpoint does not close FR1. The earlier waiting and
whole-batch readiness statements below describe their earlier source states.

Current selection: ic-memory 0.20.0, ic-query 0.45.2, ic-timers 0.9.2 and ic-testkit
0.13.0. The manifest and lockfile already agreed; no additional dependency update
was necessary. Adapted both test baseline recipes to testkit's explicit cycle
policy constructor, preserving mandatory snapshots, minimum-cycle top-up and
current time. This fixes the actual compile failure from removed reset variants.

Affected-package Clippy passes with warnings denied. Selected native tests pass
(11 Core, four control-plane, 73 Host; three simulator cases excluded), as do
two receipt-storage and 15 timer inventory checks and the canonical Coordinator
Candid contract. Embedded peer refresh and verification pass; its SHA-256 is
`368dc211042138128fcf3413ac49167189def2e2c951594721f3938aebcc09d7`.
Four exact PocketIC proofs pass on this graph: signed Host ownership/obligation
collection (3.78s case, 134s runner), real Root/Coordinator grant accounting and
canonical intent queries (388.56s case including Wasm builds, 488s runner),
uncertain creation across five retry refusals (145.79s case, 149s runner), and
baseline reacquisition plus sealed restored-Root inventory/allocation refusal
(95.29s case, 97s runner). Formatting, whitespace and document semantics pass.
Logs are retained under `target/review-validation/canic-fr1-push-*.log`.
The manifest and lockfile remained byte-identical throughout qualification.

The maintainer-selected .52 checkpoint is ready for the normal release flow;
no known blocker remains in this checkpoint. Its changelog views are prepared.
The complete FR1 outcome remains open: remaining owner/application obligations,
continuation quiescence, custody handoff, account recovery, journaled execution /
resume / CLI, whole-Fleet conservation proof and retirement contraction.
Do not describe the checkpoint as completed FR1 or complete workspace validation.

Packages remain .51 with the existing .52 changelog draft. No broad suite,
versioning, commit, push, deployment or sibling mutation was performed. Preserve
the isolated caller-authority work and optional IcyDB independence. All compiled
artifacts remain available for reuse. The maintainer-selected complete release
gate owns broad validation and version advancement; this runtime/fixture batch
is not eligible for the documentation-only fast lane.

## FR1 canonical intent census — 2026-10-03

Controller-only `IntentRelease` now discovers canonical local accounting and
receipt-backed reservations on Roots and Coordinator independently of replay links
and cleanup indexes. One bounded primary row per query retains original identities,
resources, quantities, state, payload binding, revision, application replay deadline
and terminal evidence. Expiry does not hide rows or establish payment outcome;
contradictory terminal evidence refuses observation. Host collects signed pages
inside certified custody/Registry brackets with byte, count, decoder and time
bounds; late refusal returns no partial inventory. This does not settle work or
supply reset authority. Pages are not an atomic multi-query snapshot.

The initial ic-memory 0.17.1 selection passed eight Core accounting/replay tests,
73 Host native tests (three simulator cases ignored), both receipt-storage ownership
checks, affected-package Clippy, generated Coordinator Candid equality, embedded
peer refresh/verification, formatting and document semantics. Before PocketIC,
another session changed the manifest to ic-memory 0.19 and ic-query resolved to
0.45.1. The maintainer confirmed the update was finished and authorized refreshing
the lockfile. That refresh now selects ic-memory 0.19.0, preserving the optional
IcyDB consumer's pre-refresh selection. Its separate dependencies remain outside
Canic qualification; no consumer source or integration was investigated or aligned.

On ic-memory 0.19.0 / ic-query 0.45.1, all eight Core, four control-plane and
73 Host selected native tests pass; affected-package Clippy also passes. The
signed Host PocketIC census proof passes (2.57s case, 130s runner). Embedded peer
refresh passed. However, dependencies changed again during the production
Root/Coordinator proof: ic-memory 0.20.0, ic-testkit 0.13.0 and ic-timers 0.9.2.
Artifact acquisition refused changed inputs after 70.26s; query assertions never
ran. The runner took 174s including native compilation. This is input instability,
not a demonstrated census assertion failure. None of the earlier results qualifies
the latest graph. Await a stable checkout, then run affected lint/native checks,
refresh/verify the peer, verify canonical Candid and rerun the two exact PocketIC
cases. Do not repeatedly rebuild underneath dependency edits.

Logs are retained under `target/review-validation/canic-fr1-intents-*.log`.
The runtime failure is `canic-fr1-intents-runtime-pocketic.log`; complete runner
output is under `target/test-runs/20261003T131244Z-13971.Jlp1EI/1.log`.
Preserve all dirty work and the isolated caller-authority session. No broad suite,
version, commit, push, deployment or sibling edit was performed. Compiled artifacts
remain cached.

FR1 remains unfinished and not push-ready. Next integrate canonical accounting
with original owner assessments and application observations, complete remaining
paid-owner/continuation quiescence, then custody handoff, account-recovery artifact
qualification, journaled execution/resume/CLI, whole-Fleet conservation/interruption
proof and retirement contraction. Root/Coordinator discovery does not prove the
absence of arbitrary application debts or child-owned stores. Keep the accepted
outcome in the same Unreleased batch.

## FR1 Host provisioning assessment — 2026-10-03

Host now assesses the existing authenticated provisioning census alongside its
complete original pages. Provisioning and Directory synchronization keep separate
owner-qualified identities, even when they share the same operation ID. Explicit
in-flight delivery retains its exact recipient for reconciliation. Other
unfinished aggregate work remains with its original owner to account for
lower-level effects; release does not require completing a disposable installation.
Terminal history stays completed despite stale failures, exhausted failure
counters, retry deadlines or active pointers. Active pointers with no matching
owner-qualified record remain explicit observations, including empty journals.
None of these assessments issues dispatch, replacement spending or reset authority.

All 70 selected Host native tests pass (three simulator cases explicitly ignored),
as does Host library/test Clippy with warnings denied. The exact signed ownership
query PocketIC case passes with the new assessment, preserving delivery targets,
owner identity, unmatched pointers, typed collection refusals, pagination bounds
and effect-free repeated reads. It uses the existing wire fixture to qualify Host
collection/assessment, not Root effect execution. The case took 2.48s; the runner
took 93s including the Host rebuild. Scoped formatting, whitespace and document
semantics pass. Evidence is retained under
`target/review-validation/canic-fr1-provisioning-assessment-*.log`.

Cargo.toml and Cargo.lock remained byte-identical throughout these checks. This
step changes Host only: canonical Candid, stable schemas, issued budgets and the
qualified embedded peer remain unchanged. Preserve all earlier dirty work and
the isolated caller-authority work. Optional IcyDB composition was not investigated
or aligned. No broad suite, version change, commit, push, deployment or sibling
edit was performed; compiled artifacts remain cached.

FR1 remains unfinished and not push-ready. Funding, pool, replay and provisioning
now have Host assessments, but they are not a complete destructive admission.
Next integrate orphan accounting/application obligations and the remaining paid
owners with continuation quiescence; then complete custody handoff, account-recovery
artifact qualification, journaled execution/resume/CLI, whole-Fleet conservation /
interruption proof and retirement contraction. Keep this accepted outcome in the
same Unreleased batch; these library steps are not separate patch releases.

## FR1 Host pool assessment — 2026-10-03

Host now combines bounded authenticated pool collection with pure assessment of
retained owner work. Import recovery, Root settlement, Host publication and
recorded completion remain distinct; exhausted original call/debit ceilings are
separate observations and never replacement authority. Creation assessment keeps
known unissued/refused work, uncertain Ledger effects, exact created identities
and unresolved expiry distinct. Cancellation still requires the original owner's
accounting checks. Bootstrap, import source/receipt, creation and handoff IDs
remain custody candidates, with original recipients and full evidence retained.
Historical references do not prove current custody, and no assessment grants
reset, spending or producer-quiescence authority.

Validation passes: 66 selected Host native tests (three simulator cases explicitly
ignored), Host library/test Clippy with warnings denied, and the exact signed
ownership-query PocketIC case. The simulator proof includes retained exhausted
imports and uncertain creation, complete two-Root collection, empty-owner
assessment, late refusal without partial results and effect-free repeated reads.
It uses the existing wire fixture to qualify Host collection/assessment; it does
not claim to execute Root import recovery or whole-Fleet release. The case took
2.42s; its runner took 190s including a dependency rebuild. Scoped formatting,
whitespace and document semantics pass. Logs are retained under
`target/review-validation/canic-fr1-pool-assessment-*.log`.

Concurrent dependency updates advanced the deployed selection to ic-memory
0.17.1 and ic-timers 0.9.1, with ic-query 0.45.0 and ic-testkit 0.12.0. The memory
lock entry had already been refreshed when inspected; this session refreshed only
the stale testkit entry after maintainer authorization. The final native, Clippy
and simulator checks above use this selection. Optional IcyDB skew was neither
investigated nor aligned. Embedded Root refresh and read-only verification pass
with artifact SHA-256
`e53ffcd2656dbcde822ac406cafe84f92efabcdbaba382301254d4a9f9230517`.
The timer guide explains selecting the actual provider version for inverse lookup.
Retain compiled artifacts; dependency edits during validation caused the repeated
rebuilds, not test execution failures.

FR1 remains unfinished and not push-ready. Continue the remaining paid-owner and
application/orphan-obligation integration, then complete continuation quiescence,
custody handoff, account-recovery artifact qualification, journaled execution /
resume / CLI, whole-Fleet conservation/recovery proof and retirement contraction.
This Host assessment changes no RPC, stable schema or issued budget. Changelog
stays Unreleased. No broad suite, version change, commit, push, deployment or
sibling edit was performed; preserve the isolated caller-authority work.

## Code-review issue migration — 2026-10-03

The [review catalogue](https://github.com/dragginzgame/canic/issues/40) owns the
complete September 29 review transfer. Finding issues retain the full original
record, verification/reproduction evidence and merged duplicate records; refuted
source reports remain in the catalogue. Every issue body was read back from GitHub
and matches the prepared source exactly. Qualified-fix closures were independently
checked against GitHub's closed-issue results.

The original review export and implementation/validation evidence remain available.
Local finding lists and counters now point to the catalogue, and finding references
link to their owning issues. Record future triage and completion decisions in GitHub.
This migration does not qualify the unfinished FR1 runtime batch. No runtime,
version, commit, push or deployment action was performed.

## FR1 pool-creation uncertainty correction — 2026-10-03

Root pool refill now retains earlier creation uncertainty across later Ledger
refusals. Previously insufficient funds, a future timestamp, temporary
unavailability, creation failure or generic refusal could clear uncertainty or
make the retained operation cancelable. A rejected retry does not prove an
earlier creation absent. The correction preserves the original operation,
timestamp, amount, fee and authority bindings until an exact principal resolves
custody; fresh refusal and unresolved-expiry behavior remain unchanged. No stable
schema or spending-authority change is added by this correction. The touched
refill workflow now uses a directory module; its active inventory link is updated.

The registered PocketIC case passes on the mainnet Root refill path. A test Ledger
retains one debit, first withholds the created principal, then injects each of the
five refusal variants before returning the exact original identity. The proof
checks retained creation identity, one recorded debit, Ready inventory and
terminal replay. It took 185.02s including fresh Wasm builds; the runner took 276s
including native compilation. The ordinary mainnet refill regression also passes,
reusing artifacts in a 21s runner. Test Ledger fault controls are controller-only.

The first simulator attempt exposed a production Wasm dead-code error left by
the prior timer preflight change: the exact issuer-timer identity wrapper has only
test callers. Its wrapper is now test-only; the production claimed-identity path
is unchanged. The subsequent mainnet Wasm build passes. An intermediate rerun was
stopped to yield the shared target to an editor check; no other build was stopped.

The maintainer confirmed the concurrent dependency edits were finished. Preserve
the selected ic-memory 0.16, ic-query 0.45 and ic-timers 0.9 graph. Core, Control
Plane, Ledger fixture and internal-test library/test Clippy passes with warnings
denied on that graph, as do both simulator cases above. Embedded Root refresh
and read-only verification pass with artifact SHA-256
`228dfd0786707de5dfe88e44fe10ff69008ee755b1f9945807967f955bdd3bcf`.
Both refill native regressions also pass on the updated graph; the three Ledger
fixture native tests passed earlier and that fixture's dependency graph is
unchanged. Scoped formatting, whitespace and document semantics pass.
The timer guide now matches the selected 0.9 provider. Evidence is retained under
`target/review-validation/canic-fr1-pool-uncertainty-*.log`; compiled artifacts
remain cached. Optional IcyDB skew was not investigated or aligned.

FR1 remains unfinished and not push-ready. This correction protects an existing
paid owner; it does not add Host pool assessment or a whole-Fleet release command.
Paid-owner integration, continuation quiescence, custody handoff, journaled
execution/resume/CLI, recovery/conservation proof and retirement contraction
still belong to the accepted batch. No broad validation, version change, commit,
push, deployment or sibling edit was performed. Changelog remains Unreleased.

## FR1 release preflight before producer cancellation — 2026-10-03

Core's internal Root/Coordinator release hooks now check native timer custody and
durable async-job attempts before cancelling role producers. Busy preflight and
unsettled paid-owner refusals leave producers intact; failures after cancellation
still trap for atomic rollback. Exact terminal seal replay still skips producer
checks. Both authority roles inspect the shared job-owner catalogue, including
fixture import, and return a typed active-job observation. Lease expiry and stale
completion never clear the current attempt; terminal job history does not block.
Moved the touched authority workflow to its directory module and updated its
owning inventory reference.

The targeted timer inventory test exposed an obsolete exact-version pin check:
the maintained manifest uses `ic-timers = "0.8"`. Removed that syntax restriction
and its unused parser. The guard still validates a locked deployed graph with
one shared timer provider and one raw provider identity. Duplicate identity
coverage remains; no dependency version or IcyDB integration was changed. The
native-timer guide and Unreleased changelog reflect the maintained contract.

Validation passes: 90 selected runtime tests, all 15 timer inventory checks,
Core/Control Plane library-and-test Clippy with warnings denied, scoped formatting,
whitespace and document semantics. Embedded Root refresh and read-only verification
pass with SHA-256
`3f8bde513c9a7d2e5d5c3bb299d9295a54ab751e04d8f0ce4bd4f1f6d5519159`.
The exact Coordinator joining/replay PocketIC case passes, including real snapshot
sealing, resume and restored-authority refusal (62.33s including Coordinator Wasm
build; runner 154s with native compilation). Evidence is retained under
`target/review-validation/fr1-quiescence-*.log`; compiled artifacts remain cached.

This is release-hook preflight qualification plus a shared snapshot-path
regression, not whole-Fleet release proof or a new release endpoint. Registered
jobs and timers do not cover every direct endpoint continuation or retained paid
obligation. Remaining paid-owner integration, orphan/application obligations,
custody handoff, account-recovery artifact qualification, journaled execution /
resume / CLI, whole-Fleet conservation/recovery and retirement contraction remain
in FR1. The batch is unfinished and not push-ready; its changelog stays Unreleased.
No broad validation, version change, commit, push or deployment was performed.

## FR1 Host replay assessment — 2026-10-03

Host now combines the authenticated replay census with pure assessment of the
remaining operation-owner work. Recorded terminal responses, reserved operations,
uncertain effects, child-lifecycle recovery, response recovery and cost settlement
remain distinct. Pending and missing accounting IDs are separate observations:
released reservations never clear payment uncertainty, and missing bookkeeping
never invalidates an already recorded terminal response. Original pages, owners,
slots and operation identities remain available; no settlement or spending
authority is issued. This does not expose a new CLI or establish release readiness.

Validation: 61 selected Host native tests pass (three explicit simulator cases
ignored), including every replay phase across all linked accounting states and
bounded wire-to-assessment preservation. Host library/tests Clippy passes with
warnings denied. The exact signed-query ownership PocketIC case passes, including
assessment after both owners' collection, late refusals with no partial result,
empty-owner evidence and effect-free repetition (2.64s test, 205s runner including
native compilation). Scoped formatting, whitespace and document semantics pass.
Evidence: `target/review-validation/fr1-assessment-*.log`.

This step changes Host only; the qualified embedded Root and canonical Candid
from the preceding step remain intact. For subsequent direct Cargo checks, match
Make's `ICP_ENVIRONMENT=local`, `CARGO_INCREMENTAL=0` and repository sccache wrapper
settings when reusing its compiled graph; this run switched from direct Cargo's
incremental build to Make's compiler-cache configuration and rebuilt native code.

Next connect the remaining paid owners and producer shutdown to the existing
Core release-fence hooks, accounting for callbacks already in flight. Orphan
intents/application obligations, custody handoff, account-recovery artifact
qualification, journaled execution/resume/CLI, whole-Fleet conservation/recovery
proof and retirement contraction still belong to the accepted FR1 batch. FR1
remains unfinished and not push-ready. Changelog stays in Unreleased; no version,
commit, push, deployment or broad validation was performed. Preserve the other
session's isolated membership/authentication work and the earlier dirty changes.

## Documentation refresh verification — 2026-10-03

Reviewed the 30 Markdown files changed by the merged documentation refresh
against current source owners, CLI declarations, manifests and governance.
Corrected reset prerequisites, managed initialization bindings, role funding
policy, compiler-cache behavior, platform support and standalone-local scope.
Clarified that the minimal Fleet example permits a child without creating it,
and updated its Wasm crate configuration and the native timer dependency example.
Updated the open .52 documentation notes. Historical measurements remain dated
evidence; this documentation pass does not qualify FR1 or close its release batch.
Validation: all 365 local references and 43 section fragments resolve; current
document semantics and whitespace checks pass. No Cargo, PocketIC or broad
release validation was run for these documentation-only corrections.

## FR1 linked replay accounting — 2026-10-03

Root and Coordinator replay discovery now reads each receipt's original quota
and cycle-reservation records in the same query. It retains exact resource keys,
quantities, stored pending/committed/aborted state, creation times and TTLs.
Missing records stay explicit; expiry and released bookkeeping do not prove an
external payment absent. Collection adds at most two bounded intent-record reads
per receipt and performs no settlement, cleanup, IC call or new reservation.
Host preserves these fields through its existing bounded decoder and collector.

Four Core regressions, five Host receipt regressions and four Coordinator
contract checks pass. Core/Host/governed-journey Clippy passes with warnings denied.
Coordinator Candid was regenerated through the owning artifact builder and the
ordinary build without refresh also passes. Canonical bytes retain the generator's
trailing blank line; other source whitespace and scoped formatting pass.
Embedded Root refresh and read-only verification pass with artifact SHA-256
`e232883593618c6a688e5a6760cefb32e393d3382b1726fb5824f38da37018b2`.
The real paid child-grant PocketIC case passes through the production Host decoder,
checking committed accounting, controller denial and effect-free repeated reads.
Its runner took 545 seconds including native and Wasm rebuilds; artifacts remain
available for reuse. Evidence: `target/review-validation/fr1-accounting-*.log`.

The first locked check found the existing manifest already requiring ic-testkit
0.11 while Cargo.lock retained 0.10.4. Offline resolution updated only that package
to 0.11.0; IcyDB dependencies were not changed. Preserve the separate documentation
corrections and isolated membership/authentication work.

These are linked accounting observations, not an orphan-intent census or a release
safety verdict. FR1 still needs the remaining paid-owner integration, producer
quiescence, custody handoff, account-recovery artifact qualification, journaled
execution/resume/CLI, whole-Fleet recovery/conservation proof and retirement
contraction. It remains unfinished and not push-ready. No broad suite, version
change, commit, push or deployment was performed.

## FR1 shared replay discovery — 2026-10-03

Continue the accepted FR1 batch with controller-only `ReplayRelease` observations
on Root status and Coordinator observability. This reads one stable replay record
per page with key-only lookahead, retaining expired uncertainty, original actor /
authentication / payload bindings, effect targets and cost-guard intent IDs.
Cached application responses are not returned. Encoded stable receipts now have
a 32 MiB write/read limit, without changing their CBOR layout or allocation;
projection also bounds command/method identities. This is paid-owner discovery,
not accounting settlement or a release-ready predicate.

Source and PocketIC discovery regressions pass targeted qualification. The real
child-funding journey now inspects its committed paid receipt, and the provisioning
journey checks Coordinator query authorization/replay. Core/affected-package and
governed-journey Clippy pass with warnings denied. All three discovery tests and
150 selected replay regressions pass, including refusal of an oversized stable
replacement before any bytes change. Fixture refresh and read-only verification
pass with artifact hash `3f767b151d2a1ff2ead015f435ba86b423b1d6993c0440031dfa4704953a512f`.
The real paid-grant journey passes (325.14s, 414s runner), as does the
Coordinator/provisioning journey (142.87s, 143s runner). These runs rebuilt missing
fixture artifacts and retain them for reuse.
The Host provisioning decoder's 128-type limit was too small for Root's expanded
reply union (even an older generated Root has 145 types). It now uses the existing
release readers' 512-type / 16 KiB header limits. All 28 selected Host release
tests and affected Host/journey Clippy pass. The Root journey now decodes real
wire replies through that production Host decoder.

Host receipt collection is now implemented for Coordinator and every selected
Root, preserving original pages under certified custody/Registry brackets with
bounded signed reads and no partial result on a late refusal. Its new native and
signed-wire regressions pass: 33 selected Host release tests (three simulator
cases ignored), Host/journey Clippy and the exact signed-query PocketIC case
(2.78s, 3s runner). The real paid-grant case passes through the production Host
receipt decoder (36.09s, 123s runner; cached Wasm). Coordinator's public request
DTO and canonical Candid now include the selector; five exact contract tests and
affected Control Plane/journey lint pass. DTO round trips in package isolation,
final read-only fixture verification and the public-request Coordinator journey
also pass (98.39s, 222s runner; it rebuilt affected native/Wasm artifacts).
Scoped formatting, whitespace and document semantics checks pass.
Evidence: `target/review-validation/fr1-host-receipts-*` and
`target/review-validation/fr1-replay-canonical-*`. Keep FR1 in
`Unreleased`, preserve the parallel .52 and membership work, and do not claim
push readiness. The completed provisioning/Host results
below describe earlier discovery steps. No broad workspace/release gate was run.

Next complete the maintained paid-owner integration and real producer quiescence,
custody handoff, account-recovery artifact qualification, existing-journal
execution/resume/CLI, whole-Fleet interruption/conservation proof and retirement
contraction. Discovery is not a safety verdict: completed history and incidental
accounting drift must not independently prevent an explicitly reviewed reset.
Keep unfinished paid effects under their original authority and account for
callbacks already in flight when establishing the live fence. FR1 remains
unfinished and not push-ready; no commits, version changes or deployment occurred.

## FR1 provisioning journal discovery — 2026-10-03

Root's controller-only `ProvisioningRelease` query discovers retained aggregate
provisioning and Directory synchronization operations independently of active
pointers and new-work admission. Each page reads one bounded stable value with
key-only lookahead. It preserves original operation/plan identities, exact stage,
outstanding publication/Directory delivery and failure evidence without mutation.
This is discovery, not a complete paid-obligation census or settlement predicate.

Host now collects these original pages for every reviewed Root with bounded
signed reads, exact key/phase checks, advancing cursors and unchanged active-pointer
headers, bracketed by certified custody and Registry evidence. Refusal returns no
partial result. The two journal kinds may retain the same operation ID.

All 24 provisioning and three Directory synchronization native tests pass.
The 27 selected Host release tests pass (three simulator cases ignored), including
five new provisioning regressions. Control Plane/facade, Host and governed-journey
library/test Clippy pass with warnings denied. Embedded fixture refresh and
read-only verification pass with unchanged artifact hash
`ce51c5a228f8c08eab5cf6f5e2bc7ea8ba1e6bbead9f0201482dfbc3ce0f6704`.
The extended Host signed-query PocketIC proof passes in 2.17s (4s runner).
The exact interrupted-to-terminal production Root journey passes in 30.46s
(51s runner), proving original identity/failure evidence, controller denial,
terminal discovery, replay and unchanged balances. Its first run exposed a
test-only E30/E31 expectation error; correcting the assertion reused the retained
Wasm/build artifacts. Runtime authorization was unchanged.
Logs: `target/review-validation/fr1-provisioning-census-*` and
`target/review-validation/fr1-host-provisioning-*`.
Remaining paid owners, quiescence/handoff, whole-Fleet
execution/recovery/CLI and retirement contraction remain in FR1 before push
readiness. Document semantics and whitespace checks pass. Preserve the separate
membership/authentication session's work.

## FR1 Host pool collection — 2026-10-03

Host now collects `PoolRelease` evidence for every reviewed Registry Root using
bounded signed queries, bracketed by certified Coordinator/Root custody and
matching Registry observations. It preserves exact exhausted import budgets,
historical operators, bootstrap holds, uncertain creation and handoff records.
Root/subnet mismatches and malformed/over-budget replies refuse the complete
result. This does not settle effects or establish custody of historical sources.
Root membership/authentication implementation belongs to the other session;
this step changes only Host collection and its signed-query fixture coverage.

After the maintainer freed space, all 22 selected release-ops native tests passed
(three simulator cases ignored), including four new pool regressions. Host
library/test Clippy passes with warnings denied. The exact signed-query PocketIC
proof passes in 1.77s (4s runner), including two Roots, late refusal, replay and
unchanged balances. Current logs are `target/review-validation/fr1-host-pool-*.log`.
External cleanup had removed the earlier build cache and logs; this session did
not perform cleanup. Membership/authentication uses its isolated source/target copy.
Provisioning/Directory discovery follows in the current section above; it does
not yet establish paid-effect settlement.
Keep FR1 in root `Unreleased`, the parallel .52 fixes intact and
the original-review count at 31/401. FR1 remains unfinished and not push-ready.

## Documentation accuracy baseline — 2026-10-03

The requested documentation-wide refresh begins with a source-backed accuracy
pass over the maintained landing pages, README files, configuration reference,
feature guides and onboarding flow. The canonical configuration guide now
describes the live host-compiled configuration boundary, role observability,
offline chain-key derivation, local application authorization, peer Component
provisioning, Component Groups, independent Group deployments, reduction-only
member limits and Fleet-service targets. Its canonical example includes a real
Group deployment and active-pool service and passes the strict current parser.

Correct the Core layering diagram so workflow calls pure policy and ops as
independent branches, complete the facade's config-to-feature requirements,
and align current timer and memory guide versions with `ic-timers 0.8.1` and
`ic-memory 0.15.3`. The timer guide no longer presents optional IcyDB version
alignment as a Canic requirement. The minimal Fleet and reference-App guides
now distinguish declaring a reusable Spec from selecting a concrete Group
deployment or desired Fleet occurrence. Root vocabulary and scaling navigation
include Groups, deployments and logical services.

The exact `config_guide` integration test passes. The maintained README and
non-archived local-link scans find no missing targets; the CLI command catalogue
and facade feature table match source; document semantics, scoped whitespace
and diff checks pass. Archived designs, dated audits and release notes remain
historical evidence and were not rewritten as current guidance. The first
scanability pass adds task-oriented tables to the root landing page,
documentation index, feature index and configuration map without changing
their authority. This documentation-only batch changes no runtime, package
version, release readiness, Git publication or deployment state. FR1 remains
unfinished and not push-ready.

The newcomer pass now defines IC and Canic vocabulary before using it across
the root README, documentation and feature indexes, installation, configuration,
the minimal managed Fleet walkthrough, reference Apps, public crate/CLI guides,
operations and architecture entry points. The root now presents Canic as the
Kubernetes-like orchestration and operations layer for multi-canister IC Apps,
then links to a focused model guide that traces source and configuration through
build evidence, reviewed planning and per-Subnet management. The shorter root
landing page retains only orientation, task navigation, a capability map and
current pre-1.0 status. Compact diagrams replace full-width character banners;
one combined logo/welcome hero anchors the landing page, and small supplied
callouts retain descriptive alternative text. Detailed contracts, active
designs, audits and historical records retain their precise technical language
and evidence role.

The next design pass carries that visual language through the focused model,
installation, first managed App, configuration and documentation-index pages.
It reuses the split build/deploy and Fleet-structure diagrams, and adds compact
operator/runtime, installation-path, minimal-topology, milestone,
configuration-ownership, App-hierarchy and audience-route maps. Small 256-pixel
callouts are integrated beside orientation, checklist and warning text rather
than used as full-width section art.

## Cross-Component caller authority assessment — 2026-10-03

At the maintainer's request, assess the missing .51 membership flow as a whole
rather than adding a Root lookup endpoint. The
[authentication proposal](../architecture/authentication.md#receiver-local-caller-authority--design-proposal-2026-10-03)
selects receiver-specific managed binding projections maintained by protected
Directory publication during activation and removal. Ordinary endpoint guards
combine IC caller identity with local policy. Strict revocation completes only
after every affected receiver has a durable denial fence; outages block that
completion, while previously committed ordinary calls remain available.
The maintainer has authorized continuing the implementation alongside FR1.
Implementation uses strict completion as the stated baseline; no bounded-expiry
authorization contract has been selected.

Existing role attestations require a caller-bound direct-query certificate and
cannot be autonomously obtained through the normal canister update flow. Their
request-supplied epoch is also not a Registry-backed revocation protocol. Correct
the active command/status names and separate verifier identity checks from
endpoint-owned allowed-role policy. The design distinguishes metrics target
selection from incoming caller admission and covers dynamic children, receiver
enrollment, in-flight effects, bounded state, lost replies, same-release recovery
and required cross-Root evidence. The Root membership lookup remains a separate
API gap for inspection/discovery; it is not the proposed ordinary authorization
path. This work does not schedule a later minor or introduce a runtime contract.

The receiver state machine, pure admission/ticket policy and exact Root
recipient census draft lives in an isolated source copy under
`.canic/local-work/caller-authority-20261003/source`, with its own target directory.
The draft covers exact Component Spec/role pairs, dynamic-child roles,
ordered prepare/commit/complete phases,
denial fencing, generation high-water retention, unchanged state on conflicts,
exact replay and complete recipient acknowledgement. Receiver storage now uses
a bounded fixed header and independently indexed source rows. An exclusive
model plan validates one source delta without cloning unrelated grants; ops
encodes before synchronous row/header writes and exposes receipts afterward.
Ordinary policy borrows a validated local cache. Restoration checks exact
receiver/Root installations, policy, capacity, row keys/counts, grant revisions
and retained phase/generation evidence. Root census construction also checks the
current issuing installation, including empty coverage. The isolated Root persistence draft now indexes
fixed publication headers and recipient progress by the original operation and
receiver. Each acknowledgement replaces one row; earlier operations remain
retained. An immutable census commitment binds issuing/source authority, the
complete receiver set, count and encoded byte reservation. Cold restoration
refuses altered/missing recipients, changed operation keys and impossible receipt
phases without rewriting the retained evidence.

All 33 selected native tests pass, including retained-memory reopening after
every phase, lost-reply replay with unchanged bytes, unavailable revocation
receivers, malformed restoration, irreversible retirement, pre-fence encoding
bounds, exact aggregate byte refusal, isolated acknowledgements and retained
terminal history retained before the next same-receiver operation. Separate
20,000-source and 20,000-recipient cases pass;
they do not qualify a dense source/receiver product graph. These are native
storage/model proofs,
not canister lifecycle or IC rollback qualification. The draft is not integrated
into live lifecycle operations. Production allocation and policy/build admission,
protected publication, complete receiver enrollment and capacity reservation,
lifecycle ordering, metrics, cross-Root qualification and PocketIC recovery remain
in the same unfinished batch. Source formatting and patch application checks
pass. Production Clippy refuses the draft's unwired types/functions under the
existing dead-code rule; actual style findings were corrected, with no lint
suppression or visibility change to bypass production integration.
The existing runtime activation adapter currently schedules framework bootstrap
and application hooks together, before Root membership activation. Making its
current readiness barrier wait for publication would create a circular wait.
The proposal now requires an internal framework-bootstrap observation followed
by a protected application-startup release under the original membership
operation. Init arguments remain retained until release; same-release recovery
must restore the startup decision and caller fences before deferred hooks. This
is the next lifecycle integration boundary, not an implemented startup change.
Component draining also needs complete descendant-aware publication before its
current Active-to-Draining Registry commit. The proposal selects a Component-wide
installation-bound denial fence and bounded cleanup; nested subtree removal
needs exact descendant coverage. Individual-source native proofs do not qualify
these group fences. Preserve FR1's original paid-owner reconciliation throughout.
Do not integrate that unwired patch or call this runtime qualification.
The scoped `implementation.patch`, source hashes, native/Clippy logs and
validation manifest are retained beside the isolated source. The failed
disk-space build log is retained; only its invocation-owned target was removed.
Keep concurrent FR1 source and validation intact. The caller-authority batch
is unfinished and not push-ready; its runtime changelog/publication surfaces
must wait for the coherent end-to-end outcome.

The separate exact PocketIC memory regression now passes: fresh standalone
installation, a retained TTL-free local intent reservation and two upgrades of
the exact same Wasm preserve ownership, geometry, reservation denial and the
intent counter. The private existing-runtime selection uses Canic 0.110.51 and
ic-memory 0.15.3; it does not qualify Toko's Generator, optional IcyDB composition,
ReceiptBackedIntentRecord at ID 45 or live caller-authority publication. The
exact selected case passed in 18.83s (21s runner); owning library/test and fixture
Wasm Clippy pass with warnings denied. A controller-only test-fixture probe reads
committed memory allocations through the existing Core query facade.

Only those two qualified fixture/test deltas were propagated to the main tree
after checking build ownership and byte-exact baselines. Preserve them beside FR1;
no production runtime or endpoint changed. The scoped patch, file hashes and
logs are recorded in `standalone-memory-regression-manifest.json` beside the
isolated source. The deep source-copy path exceeded PocketIC's Unix socket limit;
a server-only short temporary path allowed the checksum-verified pinned binary
to run. The original private runner and all four exact caller-authority module
registrations are restored; the two selection manifests record that restoration.
Earlier failed invocation logs remain retained. This is same-release repeated
restoration coverage, not cross-release upgrade support.

The [memory guide](../features/runtime/stable-memory-layout.md#native-composed-tests)
now documents host-first bootstrap on every native test thread, composed
admission/grants and repeated-upgrade qualification beyond package alignment.
The inspected Toko generator fixture asserts the role-validation build marker;
that marker alone is not evidence that its selected Wasm graph was checked.
Document the E137 cause-loss boundary and a bounded ic-memory cold-reopen
qualification request; no library defect is established. Downstream source was
read-only, and no failing Toko journey was reproduced. Keep all concurrent FR1
and .52 work intact. Runtime, version, Git and deployment surfaces are unchanged.
Document semantics and scoped whitespace checks pass. This documentation/design
assessment adds no runtime qualification; the complete FR1 batch remains
unfinished and not push-ready.

## FR1 Root pool evidence — 2026-10-03

Controller-only `canic_root_status::PoolRelease` now projects the bounded pool
singleton without new-work admission. It preserves bootstrap Store/source holds,
retained import reservations/progress and exhausted budgets, creation uncertainty
and pending handoff. Released import history remains visible; queries do not
resume effects, clear records or replenish authority. Foreign Root bindings
refuse the complete result. Host integration and provisioning/child-funding
evidence remain unfinished, so this is not a complete settled predicate.

All 62 selected pool native tests and affected-package library/test Clippy pass
with warnings denied. Scoped formatting, document semantics and whitespace checks
pass. Embedded fixture refresh and read-only verification pass, retaining hash
`ce51c5a228f8c08eab5cf6f5e2bc7ea8ba1e6bbead9f0201482dfbc3ce0f6704`.
The extended exact import PocketIC journey passes in 88.42s (170s runner),
covering operator- and Root-controlled sources, exact retained progress through
issued/ready/released phases, controller denial, replay and unchanged balances.
Evidence is retained under `target/review-validation/fr1-pool-census-*.log`.
Keep FR1 in root `Unreleased`, preserve the parallel .52 corrections and keep the
original-review count at 31/401. Host pool collection, provision/child-funding
obligations, quiescence/handoff, account recovery, existing-journal execution/CLI,
whole-Fleet interruption/conservation proof and retirement contraction remain.
The complete FR1 batch is not push-ready. No broad gate, version/Git publication,
deployment or sibling mutation ran.

## FR1 Coordinator funding evidence — 2026-10-03

The Host funding collector now retains the existing controller-only Coordinator
funding status alongside every Root's pages. It shares the signed/certified
custody and Registry bracket, per-reply/aggregate byte bounds, bounded decoding
and query/census deadlines. Every reviewed Root must appear exactly once with
matching policy and lifecycle state; ordering is irrelevant. Missing, duplicate,
mismatched or malformed evidence refuses the complete result.

The assessment includes pending Coordinator grants even when Root has no pending
request, deduplicates operation IDs across both sides and reports Coordinator
rotation separately. It retains cycle balances, reservation windows and terminal
decisions exactly. Terminal history alone creates no pending operation; a Root
still awaiting a terminal result stays visible. This reuses current endpoints
and adds no canister runtime/schema change, paid effect or settlement authority.

All 44 selected Host release tests and Host library/test Clippy with warnings
denied pass. The extended signed-query PocketIC case passes in 1.29s (72s runner),
covering exact combined evidence, pending grants, malformed/missing/mismatched
Coordinator replies, replay and unchanged Root/Coordinator balances. Scoped
formatting, document semantics and whitespace checks pass. This step changes
Host collection and its wire fixture, with no canister runtime change. No embedded
fixture refresh or broad validation was run. Evidence is retained under
`target/review-validation/fr1-coordinator-evidence-*.log`.

FR1 remains incomplete and not push-ready: provision/import/child-funding
obligations, quiescence and handoff, account recovery, existing-journal execution
and CLI, whole-Fleet interruption/conservation proof and retirement contraction
remain. The current evidence is time-local; no cross-role atomic snapshot or
complete settled predicate is claimed. Keep incomplete work in root `Unreleased`,
preserve the parallel open .52 corrections and the 31/401 original-ID count.

## FR1 funding assessment and transfer uncertainty — 2026-10-02

Host now projects the retained funding pages into receipt assessments while
keeping original accounts, pending Coordinator operations and policy rotation.
Exact conversions/refunds are historical evidence; missing refund blocks need
residual review, malformed receipts are distinct, and uncertain transfers retain
Ledger/CMC reconciliation. Known unissued or explicitly refused transfers are
not made automatic reconciliation blockers.

That distinction exposed a runtime defect: expiry/rejection could clear a
reservation after an earlier lost Ledger reply, and BadFee could change the
original transfer identity. A required current-record `transfer_uncertain` fact
is now persisted before dispatch, cleared by success/duplicate or a first
explicit refusal, and preserved after a prior lost reply. Uncertain refusal or
expiry retains the reserved allowance; BadFee cannot rewrite its original fee.
This current-v1 schema change follows pre-1.0 reinstall-only policy.

Core's 86 selected refill tests, 42 Host release tests and 17 selected Root
funding tests pass. Affected-package library/test Clippy passes with warnings
denied, including governed test code. Embedded fixture refresh passes with hash
`ce51c5a228f8c08eab5cf6f5e2bc7ea8ba1e6bbead9f0201482dfbc3ce0f6704`.
The exact real Ledger/CMC PocketIC case passes in 229.71s (307s runner), including
completed transfer uncertainty, authorization, retained evidence and balance-safe
query replay. The exact Host signed-query/assessment PocketIC case also passes
in 0.87s (144s runner), preserving exact account/page evidence and distinguishing
completed, uncertain and known-refused transfers. Final read-only fixture
verification, scoped formatting, document semantics and whitespace checks pass. Logs:
`target/review-validation/fr1-transfer-uncertainty-native.log` and
`target/review-validation/fr1-funding-assessment-*.log`.

FR1 remains unfinished and not push-ready. Complete paid-obligation collection,
quiescence/handoff, account recovery, existing-journal execution/CLI and whole-Fleet
interruption/conservation evidence still precede retirement contraction. Preserve
parallel Host/CLI/Backup changes and the original-review count of 31/401. No
broad gate, version/Git publication, deployment or sibling mutation is authorized.

## FR1 Host funding collection — 2026-10-02

Continued the accepted FR1 batch after qualification of Root's funding census.
Host now collects and retains exact funding pages for every selected Registry
Root. It verifies reviewed signer/network and Registry selection, brackets reads
with certified Coordinator/Root custody and unchanged Registry observations,
and rejects policy/header/participant drift, duplicate operation IDs or invalid
cursors without returning a partial result. Bounded signed queries issue no
management updates and transfer no funds. Historical/exhausted records remain
evidence rather than being interpreted as either settlement or automatic blockers.

Five native collector tests pass, including sparse/max cursors, full record
capacity, exact exhausted evidence and a tiny wire payload with excessive
skipping work. Host library/test Clippy passes with warnings denied. The existing
certified ownership PocketIC case now also covers funding collection, replay,
unchanged balances and network/custody/Registry/policy/pagination/decode refusals;
it passes in 0.88s (70s runner, mostly native compilation). Evidence:
`target/review-validation/fr1-host-funding-{native-final,clippy-final,pocketic,docs}.log`.
Scoped formatting and document semantics pass. Runtime/Wasm source is unchanged
by this step, so the prior embedded fixture qualification remains applicable.

The collector is a Host library boundary, not yet a durable release executor or
CLI. It does not establish a cross-page snapshot, quiesce producers, classify
settlement, resolve default Ledger IDs, discover arbitrary application accounts
or qualify account-recovery artifacts. Those owners and complete Coordinator,
provision/import/child-funding obligations remain before destructive execution.
FR1 remains unfinished and not push-ready. Changelog notes stay under root
`Unreleased`; completed parallel fixes retain the open .52 draft. No broad
suite, version/Git publication, deployment or sibling mutation ran.

## Low code-review corrections — 2026-10-02

At the maintainer's request, fix `cli-core-5` and `host-icp-network-9` against
current source. `info env` now returns typed `BindingCollision` before shell or
JSON output when a numbered duplicate role collides with another role's variable.
Normal numbered exports retain their existing names and deterministic order.
Exact-session local reset no longer applies load traversal limits to the tree
being discarded; its root must be a real directory. Interior symlink targets
remain untouched, interruption resumes and terminal replay keeps the same receipt.

Seven CLI export tests and three native reset regressions pass. Host/CLI
all-feature library/test Clippy passes with `--no-deps` and warnings denied;
scoped formatting and whitespace checks pass. Evidence:
`target/review-validation/low-review-*.log`.
The counted original-ID minimum is now 31/401. Both changelog views extend the
existing .52 draft; active frontend/local-operation guides describe the fixes.
These fixes are complete for review; the complete FR1 batch remains unfinished
and not push-ready. No broad gate, simulator journey, versioning, Git publication,
deployment or sibling mutation ran. Pre-existing funding work is preserved.

## FR1 Root funding evidence — active after .51, 2026-10-02

The maintainer reports .51 live and authorizes continued FR1 work. Packages
remain .51; completed parallel Host/Backup corrections belong in the open .52
changelog draft, with published .51 notes preserved. FR1 remains in root
`Unreleased`; no whole-Fleet release command or destructive executor is exposed.

Root now exposes controller-only `FundingRelease : opt nat64` status. Its stable
census returns at most 32 refills per page, preserves exhausted and historical
records independently of resumable indexes, and includes exact funding requests,
accepted grants, pending rotations, accounts, refunds and CMC expiry evidence.
Key/record and Root participant conflicts refuse without mutation. This query
does not settle effects, fence producers, resolve default Ledgers or observe
balances; pages are time-local until the release owner establishes quiescence.

Core's 84 selected refill tests and all 10 Root funding tests pass. Affected Core,
Control Plane, facade and Internal Testing library/test Clippy passes with all
features and warnings denied. Embedded peer refresh and final read-only
verification pass, retaining SHA-256
`6d4126c235ef483d5fdfd44cc740fb1b5ecaef28cf25a167a4d2b79fec2d895a`.
The exact real Ledger/CMC fallback PocketIC case passes in 226.87s (243s runner),
including controller denial, exact retained transfer evidence, pagination,
repeat-query stability and unchanged cycle balances. Evidence:
`target/review-validation/fr1-funding-census-*`, with the successful PocketIC
run in `fr1-funding-census-pocketic-verified.log`.

Earlier attempts were blocked by local-listener sandbox restrictions or stopped
while waiting for the other session's build lock. The first compiled PocketIC
attempt exposed a test import relying on the facade's optional Control Plane
feature; the helper now uses its existing direct Control Plane dependency.
The corrected case and final governed-feature lint pass. All four Core census
regressions also pass again after the panic-free cursor cleanup. Preserve parallel
Host/CLI/Backup edits. Scoped formatting and document semantics pass.

The complete FR1 batch remains unfinished and is not push-ready. Next work is
complete role-owned paid/account evidence, quiescence and bounded handoff,
account recovery, existing-journal execution and CLI, whole-Fleet interruption
proof and obsolete retirement contraction. No broad gate, version transaction,
commit, publication, deployment or sibling mutation ran.

## Simple code-review corrections — 2026-10-02

At the maintainer's request, verify small original-review findings against current
source and fix `host-icp-network-10` and `backup-persistence-12`. ICP CLI balance,
snapshot inventory and known visibility output accept additive informational
fields while retaining required-field/type checks and unknown-variant refusal.
Backup artifact verification and restore-preview projections compare valid
SHA-256 hex independent of letter case. Malformed expected hashes return typed
`InvalidHash`; different artifact bytes still return `ChecksumMismatch`.

Seven Backup artifact tests, 17 restore-preview tests and 42 selected Host tests
pass. The initial wider ICP selection passed 72 cases but two unrelated local
HTTP listener cases failed with sandbox `PermissionDenied`; affected parsing
selections pass. Backup library/test Clippy passes with warnings denied. Host's
dependency-inclusive lint stops on a pre-existing missing-panic-doc warning in
the in-progress Core `IcpRefillOps::release_page`; its owning source is untouched.
Host library/test Clippy then passes with `--no-deps` and warnings denied,
checking the affected package without linting those separate dependency edits.
Scoped formatting and whitespace checks pass.
Evidence: `target/review-validation/simple-review-*.log`.

The original-ID tracker now conservatively counts 29 of 401 findings. Both
changelog views record the corrections in the .52 draft and active operation guides describe
the corrected inputs. These bounded fixes are complete for review; the complete
accepted FR1 batch remains unfinished and is not push-ready. No broad gate,
version change, Git publication, deployment or sibling mutation ran. Preserve
the pre-existing funding edits independently.

## Service-authority denial wording — 2026-10-02

At the maintainer's request, replace the ambiguous Fleet-service Authority denial
with `access denied: this canister is not the active authority for Fleet service
'<service-id>'`. The typed denial retains the validated required service ID;
the active deployment predicate and `AUTHORITY_UNAVAILABLE` diagnostic identity
remain unchanged. Caller admission has its separate denial. Downstream diagnosis
of the reported target/service binding remains with the maintainer.

All 28 Core access tests and Core all-feature library Clippy with warnings denied
pass. Refresh and confirmation of embedded peer provenance also pass for the
changed Core source. Evidence: `target/review-validation/service-authority-message-{native,clippy,fixture}.log`.
The existing .51 changelog draft includes the message and typed-error change.
No deployment, broad suite, version change or Git publication ran. The message
fix is complete; the complete FR1 batch remains unfinished.

## Opt-in inventory registration and .51 notes — 2026-10-02

The maintainer's validation found that the manual embedded-peer reproduction test
was omitted from the explicit ignored-test inventory. Register its exact compiled
identity in `pic::cases::EXPLICIT_SELECTIONS`; keep discovery, unique ownership,
recovery ordering and worker partition checks intact. The four native
`pic::governed_suite` checks pass (0.01s, after 16.87s compilation); the two
explicit PocketIC runners remain ignored. Evidence:
`target/review-validation/ci-embedded-inventory-registration.log`.
No PocketIC journey or broad gate was repeated for this registration correction.

At the maintainer's request, consolidate completed .51 work into one root summary
and the existing detailed draft: fresh-shard auth and issuer setup, pool-import
diagnostics, smaller `fast` artifacts, macOS Binaryen/CI prerequisites, and embedded
fixture qualification plus registration. Incomplete FR1 remains in root
`Unreleased`; it still prevents declaring the complete accepted batch ready.
No version change, commit, push or publication ran.

## Fresh-shard authentication AF1 — complete for open .51, 2026-10-02

Restore automatic missing-proof fetching during delegated-token preparation.
An explicit internal missing-proof result joins the existing stale/expired repair
path; unrelated E10 failures do not trigger fetching. Preparation makes one
Root fetch, revalidates replay ownership after the await, and retries once.
Caller/issuer, Fleet, grant and proof-verification bindings remain enforced.

Replace split issuer policy/template setup with controller-owned
`canic_root_command::ConfigureIssuer` and Root-local
`AuthApi::configure_issuer_root`. One explicit audience, grant set, certificate TTL
and refresh ratio derive both records. Admit the complete configuration before
mutation, normalize role/scope ordering, reject malformed/duplicate/over-capacity
grants, and preserve epoch, renewal state and usable proofs on identical retries.
Missing Root configuration returns `CONFIGURATION_INCOMPLETE`; disabled renewal
returns `SECURITY_INACTIVE`, both before paid signing. Existing in-flight signing
can still return transient E10; the same request can retry against that batch.

Qualification: 49 Core auth-workflow tests, 35 delegation ops tests and two facade
protocol tests pass. Core/facade runtime Clippy, Core library/test Clippy and the
three affected integration-target lint checks pass with warnings denied. The
exact `issuer_proof_bootstrap` PocketIC target passes (233.51s case, 384s runner):
missing configuration creates no batch, one setup call enables automatic proof
delivery to a fresh issuer, status reaches `Valid`, the receiver accepts the
token, identical configuration preserves proof state, and request replay and
subsequent tokens reuse the proof. The fixture retries transient E10 only after
observing a typed active Root batch. Its initial run stopped at the first E10;
the final test qualifies any pending state explicitly and requires recovery.

Affected DTOs, macros, replay identifiers, maintained fixtures, test inventory,
active auth docs and the existing .51 changelog draft are updated. The embedded
peer's stale-byte guard correctly refused the combined source; refresh and final
read-only verification pass with artifact SHA-256
`446fd160ca174747fb3a87509d1710057858818afb559110685e4dca481e67b9`.
Evidence: `target/review-validation/issuer-config-{workflow-native,ops-native,runtime-clippy,core-test-clippy,fixture-clippy,embedded-refresh,embedded-verify-final}.log`
and `issuer-proof-bootstrap-pocketic.log` in that directory. Scoped formatting,
whitespace, layering, document semantics and inventory checks pass. No broad gate
or repeat public embedded-peer lifecycle journey ran for this auth batch.

AF1 is complete and ready for maintainer review; its changelog is ready for the
open .51 publication batch. The complete worktree remains not push-ready because
FR1 is unfinished. Packages remain .50; no versioning, commits, push, publication
or deployment ran. No Toko files or staging state were changed. Toko must adopt
the single configuration call on its shard-ensure path when taking .51, then
verify a live shard's `Valid` status and sign-in. Deployment completion alone is
not authentication acceptance.

## Embedded allocation-peer CI repair — 2026-10-02

The third failure from published .50 CI is corrected in source. The embedded
peer now builds from an invocation-owned copy of current Git-listed files,
including untracked source additions. Only that copy uses synthetic workspace
version `0.0.0` and matching local dependency requirements; exact external locked
packages must stay unchanged. Compiler source-path remapping removes local
workspace, Cargo-cache and sysroot prefixes. The producer recipe is watched, and
provenance records original and normalized producer locks. Verification still
rejects corrupt bytes and qualifies changed inputs through an exact Wasm build;
ordinary release-version transactions no longer invalidate the fixture afterward.
No real workspace versions, lockfile, release transaction or production build
profiles are changed by this repair.

Three focused tests pass, including real byte-for-byte reproduction from different
source directories after a private Cargo release-version transaction, and refusal
of a deliberately broken source (77.88s). Internal Testing library/test/example
Clippy passes with all features and warnings denied (63s). The final refresh
confirms artifact SHA-256
`da1bbf14cc448dddcd2d7ecb85e6959ddbd926d74ba541aa2cd33df2691162ff`.
The exact public managed-component lifecycle PocketIC proof passes (229.76s;
382s runner including native compilation and cleanup). Two native evidence tests
also pass using that qualified binary. Evidence is under
`target/review-validation/ci-embedded-{reproduction,clippy,refresh,pocketic,native-qualified-binary}.log`.
No actual GitHub rerun, cross-host-architecture reproduction or native macOS run
is claimed; earlier Binaryen/CI-helper corrections remain uncommitted.

**Combined-tree fixture qualification is resolved by AF1 above.** The earlier
attempted native rebuild encountered in-progress Root issuer configuration files
(`ci-embedded-native-final.log`). Those changes are now complete; the peer is
refreshed and its standalone verifier passes against the settled combined tree.
After the maintainer confirmed completion, both native fixture evidence tests
were rebuilt and passed (57.59s compile; the explicit reproduction test remains
unselected). The read-only standalone verifier also exits successfully using the
current artifact above. Evidence: `ci-embedded-native-settled.log` and
`ci-embedded-verify-settled.log` under `target/review-validation/`.
The earlier lifecycle and byte-reproduction evidence belongs to the preceding
fixture bytes; those expensive journeys were not repeated. No fixture refresh
was necessary after AF1's verified refresh, and the staged changes were preserved.
FR1 remains unfinished, so the complete batch is not push-ready. Changelog/status
updates are retained; no commits, push, versioning, deployment or broad gate ran.

## Smaller fast-profile Wasm — 2026-10-02

At the maintainer's request, change the maintained Cargo `fast` profile from
local-only LTO / 16 code-generation units to ThinLTO / 8 units. Release keeps its
existing fat LTO / single-unit settings. The workspace, generated infrastructure
manifests and infrastructure command overrides agree; the maintained application
example is updated. Existing downstream application workspaces must update their
own `[profile.fast]`, because Cargo does not inherit dependency profiles. No
sibling repository was edited. Historical ablation inputs remain frozen evidence.

Two real `canic build test root --profile fast` builds on the same source/config
measure code-section bytes **9,748,043 → 9,415,049** (3.42% smaller), final Wasm
**10,423,944 → 10,079,226**, and gzip **2,687,229 → 2,621,450**. Code headroom below
the 10 MiB threshold rises from 0.70 to 1.02 MiB. Candid is byte-identical. This
is one representative Root, not proof that every oversized consumer fits. Wall
times are not comparable: the second invocation also rebuilt the native CLI and
used a different cache state. The initial direct `root_probe` Cargo build was
refused by the maintained build boundary; both measured builds use the supported
Canic builder. Evidence: `target/review-validation/fast-root-{before,after}.*` and
`fast-profile-comparison.json` in that directory.

Twelve selected Host profile tests pass, including the new structured comparison
between workspace and generated profiles; Host all-feature library/test Clippy,
scoped formatting and whitespace checks pass. Refresh the embedded peer and
provenance for the new profile; its confirmed artifact SHA-256 is
`6bbce41eb79f80f35abb7d089bd7163460fc932bea4f5b56f5edea964caf47a4`.
Evidence: `target/review-validation/fast-profile-{native,clippy,embedded-refresh}.log`.
No broad suite or new PocketIC journey ran for this build-profile change. Separate
timing/progress edits arriving from the other session were preserved.

This size-tuning change is complete and uncommitted. The complete open batch is
still not release-ready: FR1 remains unfinished and the earlier cross-runner /
release-boundary embedded-Wasm CI qualification issue remains open. Packages stay
.50 and root `Unreleased` includes this change; no next version was allocated,
committed, pushed or deployed.

## Published .50 CI diagnosis — 2026-10-02

[Run 37004277856](https://github.com/dragginzgame/canic/actions/runs/37004277856)
tested release commit `718531020`, not the uncommitted FR1 changes. Linux checks,
MSRV, security, preflight and release build passed. Four jobs failed for three
distinct reasons:

- Both macOS architectures aborted during Binaryen installation (exit 134).
  The pinned archive's Mach-O executable loads `@rpath/libbinaryen.dylib` using
  `@loader_path/../lib`, but the installer extracted only `bin/wasm-opt`.
  Preserve the verified bundle's library in a private versioned install directory
  and expose its executable by symlink; report captured startup errors.
- Ordinary tests failed only `release_flow_guard`'s release-candidate fixture:
  `cargo set-version` was absent. Install/check pinned `cargo-edit` in that CI job.
  Do not weaken the real Cargo release-surface comparison.
- PocketIC never started: embedded allocation-peer qualification rebuilt after
  input drift and rejected different Wasm bytes. The published artifact contains
  absolute `/home/adam/.cargo` and `.rustup` paths; the .49-to-.50 version transaction
  also changes the selected Cargo graph without refreshing this artifact.
  Exact contribution of each difference is not isolated from the retained job log.
  **Still open:** portable fixture builds and release-boundary qualification.
  The local FR1 refresh does not establish byte reproducibility on GitHub runners.

The first two corrections are uncommitted. Targeted Binaryen installer fixtures
pass for simulated arm64/x86_64 Darwin and x86_64 Linux, including repeat install,
library retention, startup diagnostics and checksum rejection. The existing real
Cargo/fake-Git release-candidate fixture passes without commits; Bash syntax,
ShellCheck, workflow actionlint and whitespace checks pass. Actual macOS execution
and corrected CI remain unqualified here. No broad gate, remote rerun, release,
commit or deployment ran; FR1 remains incomplete below.

## FR1 declared-account observation — 2026-10-02

Continue the accepted FR1 batch with authenticated `icrc1_balance_of` queries
for declared Ledger accounts. The Host reader binds the reviewed signer and
network, requires known Root/Coordinator Cycles Ledger accounts even at zero,
and checks declared operator-held custody before querying. It bounds account
count, transport/reply bytes, Candid work/type/header complexity, each query and
the whole collection. Failure returns no partial inventory; queries transfer no
funds and do not reserve paid management calls. Live balances replace the input
balance claims. Missing and explicit all-zero subaccounts normalize to the same
account and sealed review digest; duplicate representations refuse admission.

Qualification: 30 selected Host native tests pass, including reply/header bounds,
overflow, custody, mandatory accounts and normalization. The exact signed-query
PocketIC proof passes in 0.72s (87s runner including Host compilation), covering
exact owner/subaccount arguments, zero/full-width balances, multiple Ledgers,
malformed replies, overflow and wrong signer/network. Its first run failed only
because the fixture omitted the NNS trust anchor; the final run fixes that setup.
Host all-feature library/test Clippy passes with warnings denied.
Evidence: `target/review-validation/fr1-accounts-{native,pocketic,clippy}.log`.

This is declared-account observation, not automatic discovery of all accounts or
proof of recoverability. Recovery hashes remain declarations until artifact
qualification; balances are time-local and still require the release fence.
Next collect configured ICP accounts and role-owned paid obligations, then wire
quiescence and bounded handoff. Do not reuse snapshot resumability as settlement:
ICP refill `NotifyMaxAttempts` is non-resumable but can retain an unresolved CMC
effect; conversely unused reservations and completed history are not release
blockers. Inspect effect evidence at its existing owner rather than adding
another retry or migration path.

FR1 remains unfinished and not release-ready: account recovery, operation
creation/execution, CLI, whole-Fleet interruption/reuse proof and retirement
contraction remain. Packages stay .50 with one incomplete root `Unreleased`
entry. No broad gate, release, deployment, commits or artifact cleanup ran.

## FR1 resumed after published 0.110.50 — 2026-10-02

The maintainer reports `.50` live; main retains its release commit, tag and
complete validation receipt. Generic continuation now resumes the already accepted
FR1 Fleet-release batch. The parked source patch is integrated into this checkout,
with the operation-selection conflict reconciled against `.50` and the new typed
operator CLI journal fixture updated. Do not apply the restoration patch again.
Preserve `.canic/local-work/fr1-separated-20261001T200203Z/` as recovery evidence.
Blob work remains in its separate worktree; no sibling edits ran here.

Restored foundations include durable non-refundable read reservations under the
existing Fleet lock, authenticated complete registered-ownership collection and
release-specific Core fencing that snapshot recovery cannot reopen. These are
internal boundaries, not a release command or proof of whole-Fleet quiescence.
Restoration exposed an obsolete blanket reset refusal for any release record.
Remove it: spent observation allowances alone do not represent uncertain mutation.
The new regression uses an actual retained reservation, preserves its bytes, permits
reset admission, and still refuses unresolved `Intent`/`Issued` effect envelopes.
Applied effects and explicit physical reset authority keep their existing owners.

Qualification passes 74 native tests: 27 release-related Host tests, six shared
inventory/decoder tests, 16 Core fence tests, one canonical Candid test and 24
operation-selection/reset tests. All-feature library/test Clippy passes for Core,
facade, Host, CLI and internal Testing; the later reset correction has a final
Host lint pass. Scoped formatting, Bash syntax, ShellCheck with maintained
exclusions and document semantics also pass.

Four exact PocketIC proofs pass: authenticated ownership inventory (0.66s),
reserved physical/custody observation (1.47s), public managed-component lifecycle
(178.82s) and operator Component CLI completion/export/recovery/replay (272.20s).
The embedded peer and provenance are refreshed; lifecycle verifies their current
source binding. The lifecycle runner completes in 326s, including native compilation
and roughly a minute between libtest completion and runner progress; the retained
log does not establish that delay's cause. It exits successfully without intervention.
The CLI runner completes in 274s, dominated by fresh Root/Coordinator/Store/Component
artifact builds. Prior testkit teardown concerns remain open, not fixed by these passes.
Evidence: `target/review-validation/fr1-resumed-*.log`.

FR1 remains unfinished and is **not ready for another release**. Next connect
role-owned outstanding-effect/account collection, producer quiescence and bounded
handoff to the existing journal; then complete account recovery, operation creation,
the executor/CLI, whole-Fleet reuse/recovery proof and retirement contraction.
Do not turn completed historical state or observation bookkeeping into reset
prerequisites. Root `Unreleased` tracks this incomplete batch; packages stay `.50`
and no next patch is allocated. No broad gate, version change, commit, push,
publication, live deployment or build-artifact cleanup ran here.

Earlier sections retain the preceding qualification checkpoints; this section
supersedes their `.50` draft and parked-FR1 next-action statements.

## Integrated Toko import diagnostics: CANIC-183 — 2026-10-02

The maintainer requested separate Toko feedback work while the other session
owns Fleet release/reservation and authority-restoration changes. This bounded
batch was implemented and qualified in `.canic/local-work/toko-feedback-20261002/`,
an archive of published .50 at `718531020dd0311b682d2539c3178627549b2486`, using
independent source, target, server and scratch. At the maintainer's request,
the patch is now applied to the primary worktree after its active build finished.
Implementation files do not overlap the other session's edits; shared handoff
and changelog entries are preserved.

Direct Root/Coordinator queries, signed controller handoff, certified ingress
reconciliation and Root commands now retain paired timings through the existing
Fleet receipt. Root advances and handoffs retain the imported child separately
from the carrying endpoint. Prepared clients and observer clones retain the
diagnostic context across restart. Capacity-limit diagnostic reads start after
the failed update ends. Async lifetimes do not retain thread-local parents;
cancellation and panic release worker counts and leave incomplete evidence.
Existing typed failures, durable authority, consumed allowances, conservation,
reconciliation and effect-free terminal replay remain with their current owners.

Isolated-source qualification passes 88 Host import regressions, seven timing tests
and 15 CLI receipt tests. Host/CLI/internal Testing library and test Clippy passes
with all features and warnings denied. The exact signed-handoff PocketIC journey
passes in 306.33 seconds, including fresh fixture Wasms, failed admission,
Host reopen, certified retirement, reviewed continuation, completion and replay.
The governed invocation completes in 527 seconds including cold compilation and
owned scratch cleanup. Scoped formatting and current-document semantics pass.

Final logs are retained beside the source bundle as `validation-{host-import,
timing,cli-receipt,clippy}-final.log`, `validation-pocketic.log`,
`validation-format.log` and `validation-docs.log`; the complete PocketIC log is
under its private `target/test-runs/20261002T125129Z-42415.SyxSdE/1.log`.
`changes.patch` and `README.txt` retain the review/integration boundary.

This diagnostic batch and its .51 changelog draft are integrated and ready for
review. Patch integrity and document checks pass; qualification above belongs
to the isolated source, not the combined worktree. This integration changes no
package versions or dependencies. Combined primary-tree push readiness belongs
to the other session; no full gate or combined compile/test result is claimed.
CANIC-183's complete attribution and CANIC-160's matched full-estate performance
requirements remain open, alongside downstream operational acceptance. No
commit, push, version transaction, publication, sibling edit or live effect ran.

## Operator Component CLI fixture correction — 2026-10-02

The maintainer's 2697.49-second governed suite failed in the operator Component
public CLI case: its JSON-built terminal journal omitted the required
`bootstrap_registration_recovery` field. The case failed during fixture setup,
not during a live deployment. After the maintainer stopped validation, replace
the journal, state and topology construction with current Rust record types.
Added fields now cause compilation failures instead of late deserialization
panics; production record validation remains unchanged.

The exact public CLI PocketIC case passes on final source in 23.18 seconds
(39-second runner including compilation, then successful cleanup). It exercises
real ICP calls, interruption recovery, completion, export and terminal replay.
Internal library/test Clippy with the governed feature and warnings denied,
scoped formatting, release-notes preflight and document checks pass. Evidence:
`target/review-validation/operator-cli-fixture-{pocketic,clippy}-final.log` and
`target/test-runs/20261002T112658Z-14899.b3FayL/1.log`.

This reported release blocker is fixed; the bounded 0.110.50 corrective batch
and its notes are ready for maintainer review and the selected release flow.
No complete-suite pass is claimed: the prior run failed and was stopped, and
only this exact case was rerun. No broad gate, version change, commit, push,
deployment or artifact cleanup ran here. Remaining review work stays sequenced
separately below.

## Dependency lock and embedded-fixture qualification — 2026-10-02

The reported crypto-closure and dependency-risk failures shared one cause:
the manifest requested ic-query 0.44.2 while the lockfile retained 0.44.1.
Reconcile only that package with `cargo update -p ic-query --precise 0.44.2`;
preserve the maintainer's testkit 0.10.4 and optional IcyDB selections. Both
reported gates now pass: 12 canonical Wasm roles have valid crypto closure and
the dependency audit reports zero vulnerabilities with two reviewed warnings.

Compiling the new query version exposed its added `HistoryCache` progress
variant. Host now preserves its path, disposition, watermark and optional reason
as the `history_cache` JSON phase; CLI progress renders those diagnostics.
Canic's caller-owned source still does not opt into disk history reuse. The
root and detailed 0.110.50 notes reflect query 0.44.2 and testkit 0.10.4.

Qualification passes 10 Host catalog tests, three CLI catalog tests and
warning-denied Host/CLI library/binary Clippy. The embedded peer refreshed in
the prior turn verifies successfully against the final graph. The exact public
managed-component lifecycle PocketIC case passes in 166.24 seconds; the governed
runner exits successfully after 309 seconds including build and cleanup. This
qualifies that journey with memory 0.15.3, query 0.44.2, testkit 0.10.4 and timers
0.8.1. Formatting, document semantics and whitespace checks pass.

Evidence: `target/review-validation/lock-reconcile-*` and
`target/test-runs/20261002T101700Z-4862.niWBgk/1.log`. The earlier offline-cache
and missing-variant failures are superseded by these final passes. The CLI binary
selector contained no tests; the subsequent library selector ran all three
catalog tests. The corrective dependency/fixture work is complete for review;
no broad validation, version transaction, commit, push or deployment ran here.

## Gitleaks flow removed — 2026-10-02

At the maintainer's explicit request, remove the Gitleaks target from local and
CI validation, developer installation/update paths, CI tool setup, version and
checksum pins, dedicated release-tool fixtures, installer, scanner script and
fingerprint exclusion file. Active setup, platform, validation and CS1 installer
documentation now matches the maintained tools. Release-integrity audit revision
2 removes mandatory scanner evidence while retaining credential-handling review;
the catalog preserves the historical revision and existing scan reports.

Targeted shell syntax, ShellCheck, actionlint, release-integrity authority,
audit-method catalog, validation-matrix, document semantics and whitespace checks
pass. Make dispatch with a print-only runner confirms local/CI validation retains
the remaining gates. Release-tool fixtures pass in an isolated copy, excluding
the unrelated tag-deletion fixture because it creates Git commits. Evidence:
`target/review-validation/gitleaks-removal-{validation-dispatch,release-tools}.log`.
The removal is complete and included in the open 0.110.50 notes. No Rust build,
broad validation, version transaction, commit, push or deployment ran.

## Testkit follow-up — 2026-10-02

Canic's manifest permits ic-testkit 0.10.2 and its lockfile now selects registry
0.10.3. The upstream release changes only the teardown proposal/probe and related
documentation; production crate source is unchanged from the classifier fix in
0.10.2. No dependency edits were made by this check.

The exact locked 0.10.3 library builds successfully in 6.57 seconds. Three fresh
public-API probe tests pass: application/quoted text and bare I/O refusal do not
trigger dead-transport recovery; maintained refused/incomplete/channel-closed
instance request shapes still qualify; consumer error wrappers preserve both
positive and negative classification. Logs and source are retained under
`target/review-validation/testkit-0103-*`. Direct Cargo dependency unit testing
was unavailable because the package is not a workspace member, so the successful
regression probe links the freshly built registry library instead.

0.10.3 does not fix production PocketIC teardown. Its candidate upstream patch
adds bounded fallible shutdown and has seven synthetic parent tests reported
upstream; Canic still uses unmodified registry PocketIC 16. Persistent-state
handoff after unconfirmed deletion requires further review, and the original
Busy/tick cause remains unproven. No live PocketIC or full Canic suite ran here.
The classifier feedback is resolved; teardown must remain open in the tracker.

## Upstream feedback recheck — 2026-10-02

Read-only sibling review confirms Canic now selects registry releases ic-memory
0.15.3, ic-query 0.44.1, ic-testkit 0.10.2 and ic-timers 0.8.1. This supersedes
the feedback dispositions in the earlier 0.15.2 checkpoint below, not its exact
qualification record. No dependency or upstream repository edits ran here.

- Memory 0.15.3 fixes diagnostic-triggered default runtime construction. A fresh
  public-API probe against Canic's existing exact-version rlib confirms export,
  commit-recovery and both doctor helpers return typed `NotBootstrapped` before
  bootstrap, then allow configured 16-page bootstrap. All four cases and the
  fresh-thread control pass. Source and log:
  `target/review-validation/upstream-feedback-recheck-memory.{rs,log}`.
- Query 0.44.1 still writes an export before its dry-run cache-write branch and
  opens that output with `File::create`; an alias can overwrite the cache. The
  upstream working tree has the alias fix and records successful focused/full
  tests, but that fix is uncommitted and absent from the selected release.
  Canic's loader does not supply an output path, so the triggering combination
  is not exposed there. Persistent acquisition checkpoints across CLI processes
  remain a performance opportunity; no new live timing claim is made.
- Testkit 0.10.2 fixes the generic-text dead-transport false positives. Source
  and upstream tests cover negative application/quoted messages and positive
  transport evidence. PocketIC 16 synchronous teardown waiting is now reproduced
  in isolation, not fixed; the original Busy/tick cause is still unproven.
- Timers 0.8.1 changes tooling, lints and docs without scheduling logic changes.
  No new runtime issue was found. Its handoff's claim that 0.8.1 is unpublished
  is stale relative to the release commit and registry selection.

The [GitHub review catalogue](https://github.com/dragginzgame/canic/issues/40)
tracks these dispositions. This was source review plus the small memory probe,
not combined Canic lifecycle qualification of the new dependency graph. Prior
memory 0.15.2 / timers 0.8.1 lifecycle evidence remains correctly scoped below.

## ic-memory 0.15.2 and upstream feedback — 2026-10-02

The maintainer selected published ic-memory 0.15.2 and feedback notes here,
without upstream issue publication. The workspace dependency, lockfile, runtime
guides and existing 0.110.50 changelog draft now select 0.15.2. Package versions
remain 0.110.49. This supersedes the earlier 0.15.0 adoption checkpoint below;
the upstream maintenance patches require no Canic API or allocation-policy change.

Qualification passes 35 focused memory tests, 70 receipt tests, four stable-memory
ABI/identity guards and Core library/test all-feature Clippy with warnings denied.
These native checks used timers 0.8.0. The refreshed embedded peer and its structured
provenance qualify the final graph with the concurrent timers 0.8.1 selection;
the exact public managed-component lifecycle PocketIC case passes in 201.44 seconds,
including six freshly built Wasm fixtures; the governed runner completes in
285 seconds with cleanup. The selected production/peer Wasm graph
contains one ic-memory identity, 0.15.2. Documentation semantics, scoped diff checks
and the 0.110.50 draft preflight pass.

Earlier lifecycle attempts stopped at sandbox loopback restrictions or concurrent
manifest/lockfile changes. The final permitted run uses consistent current inputs.
Locked packages were fetched for offline metadata after another session changed
optional IcyDB entries; this work did not change those versions, build or repair
IcyDB composition, or mutate sibling repositories. Optional composition remains
separately unqualified and does not block Canic-owned adoption.

Updated upstream feedback remains reproducible through public APIs:

- Memory 0.15.2: default export/recovery/doctor diagnostics before bootstrap
  construct 128-page buckets, then configured 16-page bootstrap rejects with
  `BucketSizeMismatch`. Export first returns `NotBootstrapped` despite this effect.
  A fresh-thread control bootstraps 16 pages successfully. Request nonconstructing
  inspection and explicit configuration ownership for prebootstrap diagnostics.
  This is an upstream API hazard, not an observed failure in Canic's normal startup.
- Testkit 0.10.1: dead-transport matching accepts unrelated application channel
  closure and quoted `ConnectionRefused` text. Narrow recognition to transport
  evidence; the existing upstream classifier issue already owns this feedback.
- Query 0.44.0: a dry run whose output path is the managed catalog overwrites that
  valid catalog while reporting `wrote_catalog=false`. A distinct-output control
  preserves it. Reject managed-path aliases before writing.
- Timers: four focused registration/deadline/ownership tests pass. The 0.8.1
  production-source changes are lint annotations; no additional runtime defect
  was found in this review.

Evidence: `target/review-validation/ic-memory-0152-*`, including the feedback log,
retained probe source, timer review and final PocketIC log. Complete lifecycle output
is in `target/test-runs/20261002T083700Z-61887.pWYK00/1.log`.
The dependency-adoption batch is ready for maintainer review/push and its changelog
surfaces are complete. Publication still requires the selected version/release
transaction. Preserve concurrent corrective batches. No broad gate, Canic version
transaction, commit, push, upstream posting or live deployment ran.

## Deployment and release guard cleanup — 2026-10-02

The maintainer authorized correcting trivial deployment refusals after the
guard audit. This cleanup batch is complete in the open 0.110.50 draft; package
versions remain .49 and all changes are uncommitted.

Retained-operation selection now recognizes owned records and evidence instead
of treating arbitrary directory entries as an installation. Metadata, notes,
backups, temporary writes and empty evidence directories no longer require a
reset. Incomplete owned records and paid-effect evidence retain their recovery
and explicit-inventory requirements. Root Ledger-account surplus is accepted
at admission and terminal conservation, including no-creation operations;
observed net credit never expands reviewed authority or replaces exact receipts.
Unexplained deficits, wrong custody, excess creations and uncertain payments
still refuse or require their existing recovery owner.

Release integrity now checks authority records and executable behavior instead
of freezing shell/Make source spelling. CI authority is parsed as YAML; product
generations are checked through Rust syntax, excluding comments, rustdoc and
test-only input. Audit-method fingerprints remain an explicit audit lane rather
than a deployment/release blocker. Passing release-tool fixtures capture their
expected rejection diagnostics.

All 17 selected Rust regressions passed, including exact creation receipts,
transfer-loss recovery, surplus accounting and metadata selection. Focused Host
and Canic Clippy passed with warnings denied. The complete targeted
release-integrity-contract gate, ShellCheck, formatting, document semantics and
test-inventory checks passed. Evidence: `target/review-validation/deployment-guards-*`.
No broad validation, version transaction, Git publication or live deployment ran.

This guard-cleanup batch is ready to push. The combined 0.110.50 draft also
contains concurrent batches whose qualification is tracked below; preserve them.
Root and detailed changelog notes are updated; package publication still requires
the maintainer-selected version/release transaction.

## R2 import efficiency and CANIC-190 — 2026-10-02

The maintainer explicitly selected R2 import-call reduction and adjacent CANIC-190.
Both implementations and their direct qualification are complete in the open
0.110.50 corrective draft. Package versions remain .49 and changes are uncommitted.

Root continues stop/controller confirmation into the next mutation from the
same final status sample, with no intervening await before durable intent. Fresh
mainnet placement is still required on every resumed advance and controller/
uninstall history remains exact. Admission derives 13 calls per running source
plus one terminal Root read; recommended retries derive 26 per source plus 16.
Eight sources require 105/224 minimum/recommended, down from 137/272. A 72-call
envelope still refuses before handoff. No issued Root authority is expanded.

`fleet recover-attempts` reviews exact exhausted Host owners without IC calls and
grants two attempts per resource only after exact-digest approval. Surveys,
submissions, inspections and retired handoff envelopes retain consumption,
original approvals, ingress and balance baselines. Changed owners refuse before
publication; partial publication and approval replay retain each grant once.
Current v1 owner records require `attempt_recoveries` arrays, through the existing
reinstall-only hard cut. Root cap exhaustion still requires cycle-safe reset.

Qualification passes three Core budget tests, 32 Control Plane import tests,
88 Host import tests, three Host recovery tests, bootstrap qualification,
121 focused CLI tests and recursive help checks. Warning-denied all-target/
all-feature Clippy passes for the five affected packages. Native continuation
proofs retain spent counters, Root caps and retired-envelope history, reject
changed owner files, and resume partially published grants exactly once.

All three focused PocketIC cases pass: mainnet-shaped eight-source call/debit
measurement with placement drift, retained running/stopped IDs and reset receipts,
and signed HTTP handoff recovery after Host submission exhaustion. The clean
eight-source run uses 105 paid calls in 2,087 ms with 44,571,742,321 observed Root
debit cycles. The placement-drift run refuses the mutation and completes with
106 calls in 2,026 ms and 44,616,376,394 debit cycles. These are individual
PocketIC measurements, not a comparative latency benchmark. The previous
137-call baseline derives from the original workflow. Applied journal effect
counts are unchanged; the reduction removes observations.

Evidence: `target/review-validation/r2-*`. The active R2 tracker records this
bounded outcome without closing unrelated R2 findings or conditional placement
caching. R2 plus CANIC-190 is ready for maintainer review/push, with its changelog
surfaces complete; package publication still needs the governed version/release
transaction. Combined 0.110.50 readiness also depends on the concurrent batches
tracked here, including CANIC-192 qualification. Preserve those edits and serialize
target validation. No broad gate, version transaction, Git publication, sibling
edit or live effect ran.

## Toko input continuity: CANIC-192 — 2026-10-02

CANIC-192 is implemented and qualified. Ensure's argument arrays and human
next-command retain desired, policy, seed and identity through apply/resume/import/
successor review. Policy/seed flags are accepted on apply without requesting a new
reset. Omitted retry paths inherit the retained selection. Explicit destination
changes reject before paid work with both paths in the typed error; equivalent
existing path spellings remain valid. The Host checks path continuity when
reviewing the next phase. Typed successor errors use the resolved reset selection.

Final checks pass: 82 Fleet CLI tests, 23 Host operation-selection tests (one
existing manual test ignored), and warning-denied all-target/all-feature Clippy
for CLI, Host and Testing. Scoped formatting, document semantics and the 0.110.50
release-draft preflight pass. The extended public CLI PocketIC journey passes in
481.48 seconds (484-second governed invocation). It covers non-default paths,
changed-input refusal without mutations, interrupted infrastructure and import,
Fleet completion, retained accounting and effect-free offline replay. Fixture
artifact preparation accounts for roughly 214 seconds of the journey.

Evidence: `target/review-validation/canic192-{cli-final,host,clippy,pocketic,docs}.log`
and `target/test-runs/20261002T080829Z-48987.qOe5vd/1.log`. The earlier Host attempt
failed during concurrent, incomplete CANIC-190 edits; the final Host run above
replaces that incomplete-build result. The existing 0.110.50 draft and Fleet guide
include this correction; package versions remain .49. No broad gate, deployment,
version transaction, commit, sibling edit or Cargo cleanup ran.

This closes CANIC-192's implementation and qualification for the corrective
release batch. CANIC-190/R2 and guard-cleanup qualification are recorded above.
A separate session changed ic-memory from 0.15.0 to 0.15.2 while the PocketIC
journey was running; this journey's already-built binaries and artifacts qualify
0.15.0. The dependency update owns its separate final qualification and combined
push-readiness handoff; preserve its edits and evidence. Its 35 memory and 70
receipt native checks already pass in `ic-memory-0152-{native,receipts}.log`.
Do not represent this journey as exact-source validation of that later update.
Toko's reported .49 operation remains untouched; local Canic qualification does
not establish downstream adoption or live acceptance.

## Ordinary test guard follow-up — 2026-10-02

The maintainer's release run reported seven real failures across the workspace
manifest guard, receipt inventory guard and Host library. The runner deliberately
collects the remaining native results before its ordinary-test barrier rejects
PocketIC execution; these failures are not swallowed or expected negative cases.

Corrected the test fixtures and inventories without changing runtime behavior:
role declarations are discovered only under maintained package trees and skip
`.canic` state; the receipt inventory names the memory-reservation regression;
the IcyDB guard verifies the optional dependency, feature and absent default
harness edge; activation handoff fixtures retain reviewed desired input before
sealing their plan hash. Production plan-integrity checks remain intact.

All 15 targeted tests pass (six manifest, two receipt and seven Host tests), as
does all-target/all-feature warning-denied Clippy for the three affected packages.
Evidence: `target/review-validation/ordinary-fallout-{guards,host,clippy}.log`.
The 0.110.50 draft includes these corrections. The corrective batch is ready for
maintainer review and another selected release attempt; no full gate, version
transaction, commit or deployment was run for this follow-up.

## Rust 1.99 qualified — 2026-10-02

The maintainer-requested Rust 1.99 update and full Canic-owned Clippy check are
complete. Repository, CI/security, developer installer and README pins select
1.99.0; the published MSRV remains 1.91.0. Package versions remain 0.110.49 and
the existing 0.110.50 draft includes this work. No commit, version transaction,
publication, deployment, cargo clean or full test suite ran.

Removed redundant `must_use` annotations on the two Host diagnostic iterator
accessors and unnecessary closure borrows in Backup and Host import-journal
lookup. Workspace policy allows the new `assert_is_empty` style lint: existing
emptiness predicates remain valid assertions without typed empty-array casts.

Default build/check/test graphs now exclude optional IcyDB fixture packages.
The cross-crate integration package gates its IcyDB dependency and target behind
`external-composition`; the existing explicit PocketIC selector enables it.
Full Clippy checks all targets/features in other maintained packages and all
default targets in that integration package. A locked dependency-tree check
confirmed IcyDB is absent from the default integration graph. AGENTS explicitly
requires expected upstream/API/type drift to remain a separately reported,
nonblocking local-consumer limitation. This work does not qualify IcyDB or
synchronize its upstream versions.

Qualification passed:

- Full `make clippy` with Rust 1.99 and warnings denied; final passes took
  7.83 seconds and 2.46 seconds with the warmed cache.
- Focused `external_composition_qualification_is_explicit` regression; its new
  assertion was corrected to accept an omitted Cargo `default` feature.
- Embedded allocation peer refresh, including Wasm bytes and structured Rust
  1.99 provenance, followed by the single public managed-component lifecycle
  PocketIC proof: 1 passed in 214.38 seconds, 383-second runner invocation.
- Scope-helper failure-propagation tests, governed runner regressions including
  explicit external selection, ShellCheck, release-integrity contract,
  scoped Rust formatting, document semantics and 0.110.50 draft preflight.
- Follow-up: the maintainer's release gate exposed a missing `workspace-scope.sh`
  copy in the isolated PocketIC worker fixture. Added that dependency, checked
  the other fixture-copy paths, and passed the complete targeted
  `make validation-runner-gate`, including worker interruption/cleanup and native
  ICP selection. ShellCheck and Bash syntax checks also pass. Evidence:
  `target/review-validation/rust199-validation-runner-gate.log`.

Evidence is retained under `target/review-validation/rust199-*`; complete
lifecycle output is in `target/test-runs/20261002T061230Z-63634.D7uBxi/1.log`.
No owned validation process remains running. The corrective batch is ready for
maintainer review and the selected release flow, with the earlier corrective
qualification below and these Rust 1.99 checks. No additional broad gate was
pre-run. Preserve the separated FR1 work described below.

## Corrective release separation — 2026-10-01

The maintainer selected a bounded corrective release instead of waiting for FR1.
The active tree retains CANIC-191, native-credit/controller-order corrections,
ic-memory 0.15.0, current Host hard cuts and the frozen-repair tooling removal.
The root and detailed changelogs now open the `0.110.50` draft; package versions
remain `0.110.49`. No version transaction, Git publication or live reset ran.

Newer FR1 reservations, registered-inventory collection and Core release fences
are set aside in `.canic/local-work/fr1-separated-20261001T200203Z/`.
Its `README.txt` explains restoration. `resume-fr1.patch` contains only the
separated source changes; disposable-copy application with zero fuzz reproduced
all 42 original file hashes. `before/` also preserves all 122 original changed,
new or deleted paths, including documentation. This ignored local bundle is
outside build/test cleanup and is not included in Git publication; retain it
until FR1 is restored or backed up. Earlier committed admission, snapshot-deletion
and physical-observation helpers remain, with no whole-Fleet release command.

The FR1 checkpoints below describe preserved development evidence, not features
included in this corrective release. FR1/CS1 and conditional CANIC-190 work remain
follow-ups and do not block this corrective release.

The separated corrective tree is ready for maintainer review/commit and the
chosen release flow. All 208 selected native tests pass (one existing manual
inspection remains ignored), along with Core/Host/CLI/Testing library/test
all-feature Clippy with warnings denied, scoped formatting, document semantics
and the `0.110.50` release-draft preflight. The public-CLI CANIC-191 PocketIC
journey passes in 509.76 seconds (590-second invocation); roughly four minutes
rebuilt its changed canister artifacts. The refreshed embedded Component Group
peer passes its lifecycle journey in 121.83 seconds (184-second invocation).

The separation check found an embedded peer built from the removed fence code;
its Wasm and structured provenance were refreshed. Two test-only lint fixes in
the preserved FR1 edits were retained in the corrective tree. The restoration
patch was regenerated and again reproduced all original source hashes with zero
fuzz. No unresolved separation failure remains. Existing builds are retained;
only invocation-owned scratch was cleared by the test runner.

Evidence: `target/review-validation/release-split-{native,format,clippy,embedded-refresh,pocketic,lifecycle,docs}.log`.
Complete PocketIC logs:
`target/test-runs/20261001T201943Z-55381.eT7IL5/1.log` and
`target/test-runs/20261001T202814Z-17421.LKP0m7/1.log`.
The earlier embedded verification refusal and first Clippy findings remain in
`release-split-embedded.log` and `release-split-clippy-first.log`. No broad gate,
package version change, staging, commit, push, deployment or sibling edit ran.
Full release validation remains owned by the maintainer-selected release flow;
its not having run during coding is not an implementation blocker.

## Toko feedback and corrective priority — 2026-10-01

CANIC-191 / [RD1](../design/0.110-fleet-runtime-contraction/0.110-design.md#rd1-remove-repair-before-reset-admission--prioritized-2026-10-01)
has a locally qualified admission correction through the existing clean-reinstall owner. Generation and
readiness now select explicit physical inventory without requiring predecessor
completion. Explicit reset qualifies current artifacts and certified physical
custody before archiving opaque predecessor bytes and binding the new selection
under the same Fleet lock. An unfinished Root import alone no longer blocks reset;
old executable plans and application state are not reset authority. Current retries
retain their spent allowances, including unpaid successors of completed phases.
Unresolved Host paid-effect envelopes still refuse with a typed, located error;
this does not require completing an old application or restoring its binary.

Focused checks pass: 21 operation-selection/archive/retirement tests, 28 generation
checks, seven readiness tests, three cancellation tests and 76 Fleet CLI tests.
Host/CLI/Testing library/test all-feature warning-denied Clippy, final fixture lint,
scoped formatting and document semantics pass. The final public CLI PocketIC
journey passes in 249.78 seconds (268-second governed invocation). Its three-source
estate covers interrupted infrastructure, Coordinator activation before Root mirror
activation, mixed cleared/stopped/untouched import sources, malformed predecessor
documents, changed-controller refusal, lost-response reset recovery, bounded debit,
cycle conservation and effect-free offline replay. Exact same-selection retry retains
allowances; a damaged matching current record does not force application repair.

Evidence: `target/review-validation/canic191-{native,generation,readiness-cancellation,cli,clippy,fixture-clippy,pocketic,docs}.log`
and `target/test-runs/20261001T185224Z-34716.H7C65V/1.log`. The corrected fixture uses
the dynamically expanded bootstrap activation phase, not a presumed action in the
later workload review. The stopped disposable run's own scratch was removed; shared
build artifacts and real operation evidence remain intact.

The reported CANIC-191 admission blocker is corrected and qualified in Canic; this
is not live Toko acceptance. The public explicit-reset CLI no longer selects the
source-bound activation preparation/adoption route. Retained in-flight provisioning
reconciliation is not replaced by this slice: genuinely uncertain Host calls and
unreadable paid-effect envelopes still require cycle-safe reconciliation, and the
older preparation machinery must remain until those obligations are covered. Its
remaining contraction is not a requirement to finish an exhausted Root import.
The correction is included in the `0.110.50` draft. Newer incomplete FR1 work
has been preserved outside the corrective source tree as described above.

Toko's current .18 handoff reports its 24-source .48 installation blocked. The
separately reported eight-source recovery is a different operation and does not
establish recovery of Toko. No live reset, sibling mutation, broad validation,
version or Git publication ran. RD1 takes priority over FR1/CS1; their unrelated
remaining work is not a prerequisite for delivering this deployment correction.

CANIC-190's conditional attempt-exhaustion gap is also confirmed in the named
Host survey/submission owners: two attempts can remain spent without a usable
resolution. It is not an observed additional Toko failure. Bounded reviewed
continuation or a concrete cycle-safe reset follows RD1 unless needed by that path;
do not silently refresh counters. Maintained native-credit corrections below
already cover the reported positive-balance refusals locally, not in published .49.
Toko reports its JSON next-action adapter locally implemented with passing offline
contracts; its older audit's missing-adapter finding is superseded. Those downstream
tests were not rerun here; live acceptance remains downstream-owned.

The controller-order false refusals in import admission/destination now compare
sorted temporary copies. Retained Registry bytes, review and authority hashes stay
unchanged; duplicate and changed membership remain invalid. All eleven owning
native tests and Host library/test all-feature Clippy with warnings denied pass
against the concurrently updated `ic-memory` dependency. Scoped formatting, diff
hygiene and document semantics pass. Evidence:
`target/review-validation/toko-controller-order-{native,clippy}.log` and
`toko-feedback-docs.log`. The transient manifest/lock mismatch and missing cached
crate during the concurrent update were resolved by its owner and normal locked
dependency download; no dependency source or lock edits were made for this fix.
AGENTS.md now explicitly rejects historical completion and incidental
representation drift as independent safety requirements. No sibling mutation,
live reset, broad validation, version or Git publication is authorized/performed.
The controller comparison and CANIC-191 admission corrections are qualified; FR1
remains unfinished and is set aside. The correction is included in the `0.110.50`
draft and does not depend on FR1 completion.

## ic-memory 0.15.0 adoption

Canic resolves published ic-memory 0.15.0 and hard-cuts direct runtime growth to
typed failures. Receipt-capacity reservation retains `RuntimeGrowError` in its
ops error and rejects before inserting a receipt. Anonymous allocation metrics
use numeric summaries, preserving gauge names, independent conservation checks,
cache failure timestamps and the 34,848-byte metadata-read bound. Committed ID
and authority helpers replace manual resolution in the IcyDB admission fixture;
early default-runtime access is qualified without choosing bucket configuration.
No compatibility shim, product schema generation or Canic version bump is added.

Thirty-five focused memory tests, seventy receipt tests and Core library/test
all-feature warning-denied Clippy pass. The embedded allocation peer and its
provenance are refreshed, and its exact Component Group lifecycle PocketIC proof
passes in 214.44 seconds, including fixture Wasm acquisition. The governed runner
completes successfully in 372 seconds including native compilation and cleanup.
Evidence:
`target/review-validation/ic-memory-015-{native,receipts,clippy,embedded-refresh,embedded-pocketic,wasm-dependency}.log`.
The selected peer/managed-role Wasm graph contains only ic-memory 0.15.0.

The maintainer clarified that IcyDB is exclusively an optional local test
consumer with an independent upstream dependency schedule. AGENTS.md and CI
governance now prohibit synchronizing its dependencies with Canic, chasing or
waiting for matching releases, or treating upstream skew as a Canic upgrade,
push or publication blocker. IcyDB 0.262.2's composition against the new runtime
is unqualified; no matching-release follow-up is required for this Canic batch.
The earlier alignment-blocker verdict is withdrawn. Canic's ic-memory adoption
is qualified by the focused evidence above and included in the `0.110.50`
corrective draft. FR1 remains a separate unfinished batch. No broad gate, Git
publication, Canic version transaction or deployment ran.

## Hard cut and reinstall policy — 2026-10-01

The maintainer withdrew the CANIC-188 frozen `.48` repair exception and requires
hard cut plus reinstall for every pre-1.0 release transition, including malformed
or incomplete installations. Discard predecessor application/framework state.
Select qualified current-build artifacts, explicit physical inventory and current
controllers for reset authority. Reconcile unfinished paid effects only to
prevent duplicate spending and establish cycle-safe disposition; do not repair
old state, restore an old Root or maintain an old CLI to continue it. Current
same-release interruption recovery, retry, idempotency and conservation remain.

AGENTS.md, active 0.110 design/status, operator guidance and the corrective draft now
carry this decision. Dedicated incident preparation/build/qualification helpers,
frozen patches/fixtures and the Host qualifier example are removed. The
[withdrawn decision and tooling record](../audits/release-lines/supporting/0.110-fleet-runtime-contraction/canic188-issued-import-recovery.md)
and existing incident bundles remain historical evidence only. Their `.48`
terminal-surplus limitation is no longer active work or a readiness condition.
The remaining Host examples compile with all features after removal of the
qualifier; Cargo metadata lists only maintained example targets. Current document
semantics pass without advisories, relative file links resolve in all ten changed
policy/history documents, and scoped diff hygiene passes. Both retained incident
bundle checksum inventories remain unchanged and pass. Evidence:
`target/review-validation/reinstall-only-policy-*`. This policy/tooling cut is
complete and included in the `0.110.50` draft; FR1 remains unfinished. No live reset, deployment, sibling edit,
version or Git publication ran.

## Maintained native-credit corrections

Current import admission, handoff/lost-response recovery, Root callbacks, reset
and net-debit receipts accept native credits while preserving spent debit,
consumed calls and uncertain reserves. Custody, floors and original caps remain
binding. The maintained PocketIC import journey passes real Root/source credits,
settlement and effect-free replay. Native evidence passes 27 Control Plane and
83 Host import tests; affected library/test and qualifier lint passed before the
now-withdrawn frozen repair tooling was retired. Evidence remains in
`target/review-validation/canic188-credit-*`.

The three Host follow-ups also pass: infrastructure bootstrap, unfinished
activation preparation and Fleet-release custody/held-capacity checks admit
increases and bound only net debit. Reviewed source records, debit/funding
ceilings, native floors, custody/content bindings and exact Ledger accounts stay
binding. A new activation preparation records its current balance while retaining
its original source. All thirteen focused native tests pass, including the
bootstrap/activation generator journey and twelve release policy tests. Host
library/test all-feature Clippy with warnings denied, scoped formatting and diff
hygiene pass. Evidence is `target/review-validation/balance-credit-followup-*`.
These current-code fixes remain maintained after withdrawal of the incident
repair and are included in the `0.110.50` corrective draft. FR1 is set aside
and does not block these completed corrections.

## Preserved FR1 development checkpoints (excluded from corrective release)

Core now distinguishes irreversible Fleet-release sealing from resumable snapshot
sealing. The current `v1` fence retains exact operation, review digest and recipient;
exact replay preserves its timestamp, changed bindings refuse, and snapshot
prepare/resume cannot replace or reopen it. The internal synchronous workflow hook
checks role-owned settlement, suspends producers and commits in one message.
It exposes no endpoint and admits no existing command through a release seal.
Actual role paid-obligation checks, bounded handoffs and operator wiring remain
unimplemented; this is not yet proof of whole-Fleet quiescence.

Seventeen focused Core/facade tests pass, including existing snapshot behavior,
typed release refusals, native retained-state reopening, maximum-width record
encoding and canonical Candid equality. Core/facade library/test all-feature
Clippy with warnings denied, scoped formatting and diff hygiene also pass.
Evidence: `target/review-validation/fleet-release-fence-{native,clippy}.log`.
Native store reopening
does not prove IC snapshot restoration. The first compile found an unused future
handoff helper; it was removed rather than suppressing dead-code checks. Root
Unreleased records the potentially breaking current record/status change and
reinstall-only boundary. FR1 remains unfinished and not push-ready; no release
command, broad gate, version or Git action ran.

The maintainer reports `.49` pushed and selected the Root/Coordinator/Wasm Store
FR1 slice next. The first Host library slice now seals an effect-free `v1` review
and validates complete ownership, quiescence/pending-effect evidence, explicit
destructive disposition, snapshot IDs, exact controllers, budgets and native/
reserved-cycle conservation. Known Root/Coordinator Ledger accounts must remain
explicitly recoverable. Independent destination pools require matching network,
operator and subnet, distinct surviving infrastructure and aggregate capacity.
Before-reset checks require every source stopped under operator custody without
changing reviewed code/snapshots; held-capacity checks require empty code and
snapshots with retained balances. Review integrity is checked at both boundaries.

Ten focused Host tests pass (`target/review-validation/fleet-release-admission-native.log`).
Warning-denied Host library/test Clippy, scoped formatting and diff hygiene also
pass (`target/review-validation/fleet-release-admission-clippy.log`). The two
redundant test clones found by the first lint pass are corrected. Existing dependency and test-cleanup
edits from another session were preserved. Shared-target validation owners were
allowed to finish before editing or continuing checks. No deployment, versioning,
Git action, broad gate or Cargo cleanup was performed for this work.

FR1 remains an unfinished batch: no release CLI or whole-Fleet executor is exposed,
no retirement code is removed, and complete role/account observations still need
authenticated collection. Next wire complete inventory/quiescence, qualified
account recovery/settlement and existing-journal handoff/reset execution; then
prove interruptions, fresh bootstrap and replay in PocketIC before contraction.
The root Unreleased notes hold this incomplete batch; no patch number is assigned.
Toko now reports its separate eight-source import recovered, backend Ensure
converged and the original Root restored. Its Root `5lnwm-ziaaa-aaaae-agtqa-cai`
is distinct from the historical CANIC-188 incident; the downstream bundle is not available
here and no independent live verification was performed. The maintainer retained
all report feedback and call-reduction work in the
[0.110 future-work tracker](../design/0.110-fleet-runtime-contraction/status.md#toko-staging-follow-up--accepted-future-work-2026-10-01):
bounded workflow-derived budgets, fewer repeated subnet/status calls with validity
proof, controller-order fixes, observation diagnostics, reproducible incident/raw/
gzip evidence, quiescence and effect-free replay, plus downstream application
acceptance and dependency provenance. RD1 now precedes FR1; no patch or external
effect is authorized by this planning update.

The next execution primitive is now implemented in the existing Host executor:
`DeleteSnapshot` binds exact target, module and snapshot inventory, requires
stopped sole-operator custody, and reconciles only the exact before/after sets.
It carries normal journal identity, debit observation and CLI progress reporting;
it introduces no second journal or whole-Fleet command. The production-adapter
PocketIC case passes unsafe-custody/inventory refusal, exact deletion, a discarded
response with disk intent still at `Intent`, and replay without a second deletion.
The source ID, subnet, code and controllers remain intact, with bounded observed
native debit. Test execution took 3.01 seconds; compilation dominated the 84-second
invocation. The initial fixture omitted the NNS trust anchor and failed before
deletion; that setup is corrected. Evidence:
`target/review-validation/fleet-release-snapshots-pocketic.log`.

Final focused validation also passes fourteen native tests (ten admission,
three snapshot policies and the shared signer-helper regression), Host/CLI
library/test all-feature Clippy with warnings denied, scoped formatting, runner
regressions, shell syntax/lint and diff hygiene. Logs are
`target/review-validation/fleet-release-snapshots-{native,clippy,runner}.log`.
No full gate or release action ran. The first lint pass's duplicate match arm,
redundant test clone and long journey annotation are resolved.

`ops/platform.rs` moved to `ops/platform/mod.rs` to host the focused PocketIC
module using normal Rust directory discovery. Historical audit paths still name
their pinned source revision. The existing authority snapshot fence is not a
drop-in release fence: it blocks normal handoffs and permits resumable retained
work. Explicit release quiescence and reconciliation remain necessary. The whole
FR1 batch is not push-ready; its root Unreleased entry stays open.

Authenticated physical observation after operator handoff is now implemented.
It reuses certified import status reads and shares bounded snapshot decoding with
the deletion adapter. Free preparation binds the signer, network and certified
custody; consuming it performs at most four management reads without automatic
retry. Changed code/version, controllers, subnet, running state or snapshot
inventory refuses the sample. Current balances are returned for conservation
checks. This is neither role quiescence nor account recovery. The following
reservation slice connects its read allowance to the existing operation journal.

The new PocketIC observer case passes actual stopped-canister/snapshot sampling,
wrong signer/network/subnet refusal and changed-custody refusal after preparation
(1.47 seconds). The shared deletion/lost-reply regression also passes (2.97
seconds); recompilation dominated its 169-second invocation. Logs are
`target/review-validation/fleet-release-observation-pocketic.log` and
`fleet-release-observation-snapshot-regression.log`. All seventeen focused native
tests and Host library/test all-feature Clippy with warnings denied pass
(`fleet-release-observation-{native,clippy}.log`), alongside runner regressions,
scoped formatting, shell syntax/lint and diff hygiene. The lint check found a
single-case loop in concurrent import-test edits; it is now the same direct
assertion, with import behavior unchanged by this correction. Only targeted checks ran; shared-target
ownership was checked before each compile. Another session advanced HEAD and
continued import-budget edits; those changes remain preserved.
Use `ICP_ENVIRONMENT=local CARGO_INCREMENTAL=0` with direct Cargo checks to
match local Make runs. Core watches the environment selector, so switching
between unset and explicit `local` also invalidates otherwise reusable artifacts.

The release observer now requires an opaque, non-cloneable reservation borrowing
the ordinary Fleet journal lock. The review freezes each source's per-call quote;
the existing journal retains its review, separate executable-plan identity and
monotonic reserved-call counters. Four calls and their worst-case debit must fit
before persistence can issue the token. Lost results remain charged, uncertain
persistence forces reopening, and ordinary Ensure/import rejects takeover of an
unfinished release. This attaches to an existing matching operation envelope;
operation creation, full quiescence/account collection and execution are still
pending. Current journal/review fields change through a hard cut with no fallback.
Source edits waited for the other session's governed import journey to finish
successfully. Later checks were also serialized with the other validation owners.

Final reservation qualification passes 31 native tests (29 Host, two CLI), the
exact reserved-observation PocketIC case and Host/CLI/Testing library/test
all-feature Clippy with warnings denied. Logs:
`target/review-validation/fleet-release-reservation-{native,pocketic,clippy}.log`.
The PocketIC test takes 1.59 seconds; its isolated Host test binary rebuild dominates
the 92-second invocation. Combining Host and CLI test packages selects a different
Host feature graph; prefer separate native selectors when reusing the isolated
PocketIC binary. The initial test-only redundant clone is corrected. Scoped
formatting and diff hygiene pass; the unrelated incident repair patch retains its
own owner's changes, including blank patch-context whitespace diagnostics.
No broad gate, Git action, release mutation, deployment or Cargo cleanup ran.
FR1 remains unfinished and not push-ready: complete live role inventory/quiescence,
account recovery, operation creation, whole-Fleet execution/reuse proof and
retirement contraction still need implementation and qualification. CS1 follows FR1.

The registered-ownership collector now reuses bounded Registry/pool pagination,
checks the exact reviewed ownership closure, and verifies signer/network plus
certified owner custody around the queries. Every selected child receives a final
custody check; redundant preliminary child reads are avoided. Shared reply decoding
bounds bytes, work, type count and header complexity. Pending registered assets
remain visible, and the returned inventory cannot claim quiescence. Unfinished
creation/import may own IDs outside the registered pool; reconcile those effects
before treating the census as complete destructive authority.

Twenty-three focused native tests pass against the concurrent release-credit
correction. The PocketIC wire fixture passes actual signed queries/certificates,
pending-reset membership, omitted-member/Registry/signer/custody refusal and
malformed replies (1.69 seconds). This qualifies the collector, not runtime role
quiescence or whole-Fleet release. Runner regressions and scoped shell checks also
pass, alongside final Host library/test all-feature Clippy with warnings denied
and scoped formatting/diff checks. Evidence:
`target/review-validation/fleet-release-inventory-*`. Another
session's separate activation assertion correction now passes its thirteen-test
rerun; those edits are preserved. FR1's account recovery, quiescence/handoff,
operation creation, execution/reuse proof and retirement contraction remain.

## Accepted simplification follow-up

The maintainer accepts all seven candidates from the later read-only audit into
the [0.110 CS1 tracker](../design/0.110-fleet-runtime-contraction/status.md#pre-blob-simplification--accepted-2026-10-01).
RD1 now takes priority; CS1 follows FR1 and coordinates with remaining owner
corrections. Shared Fleet transitions, chain-key decoding, nonroot retries,
bootstrap survey loops, role overviews, Backup staging and CI installers must
all complete with focused qualification, propagation and cleanup before final
B5/human 0.110 closeout and 0.111 blob-storage removal/extraction. All are pending;
this planning update implements no source cut and assigns no patch version.

## Separate complexity audit

The maintainer requested repository-wide complexity/obsolete-surface screening,
kept it separate from FR1, then stopped the other implementation and requested
completion. The [published-baseline report](../audits/reports/2026-10/2026-10-01/complexity-hard-cuts-and-feature-gaps.md)
retains the immutable `.49`/`.48` census and first cleanup's 102 focused tests.
The [follow-up report and manifest](../audits/reports/2026-10/2026-10-01/complexity-hard-cuts-and-feature-gaps-2.md)
close all three deferred Host families: inline durable-plan loading/compaction,
omitted empty import credits and omitted bootstrap recovery fields. Executable
plans also require retained reviewed input; their working-input fallback and
its error are removed. Active guides describe explicit current fields. The
generation guard now covers colon wire domains and journal/plan/state families.

All 241 follow-up native tests pass (165 Host, 76 Fleet CLI), plus warning-denied
Host/CLI library/test Clippy, scoped formatting, guard fixtures, audit catalog,
document semantics and diff hygiene. Two existing document-layout advisories
remain for the exact incident-design exception; two native-selected funding
cases remain ignored. Logs and isolated source diff:
`target/review-validation/complexity-finish-*`. Across both slices, 343 native
tests pass and 101 net Rust lines are removed; the follow-up removes 55 production
module lines while adding current authority/shape evidence. One predecessor-only
test struct and three standalone tests were removed by the first slice.

The named cleanup is complete and ready for review. No maintained Canic-owned
generation above `v1` or additional obsolete decoder was found in the repeated
screen and named traces. Exhaustive per-function reachability, semantic test
deduplication and full entropy scoring remain outside the audit's evidence.
The twelve feature/qualification limits remain: fresh backup preflight is
unimplemented, and FR1 is unfinished despite its admission/snapshot primitive.
The stopped task's source, dependency and runner edits are preserved; its later
design-feedback additions were also left intact. No retained paid-operation
files, reservations or frozen CANIC-188 bytes were rewritten. Current hashes
change through a hard cut; no migration or old-format reader is added. Release
scope is now the `0.110.50` corrective draft; newer FR1 work is set aside.
No version, Git, live effect or broad gate ran.

## Prior release qualification

An earlier maintainer release attempt failed only the embedded allocation-peer
lifecycle case: endpoint framework changes had left its checked-in Wasm stale.
The test stage took 3,382 seconds; the internal suite took 2,410 seconds because
independent cases continued after the early fixture failure. Native, documentation,
Host and runtime/blob/payload suites passed. There is no complete release
success receipt for this failed run; retained build caches are reusable, but the
current release owner does not reuse partial test results.

The fixture and structured provenance are now refreshed in unstaged changes.
Complete and PocketIC-only test runs now verify fixture freshness before starting
their suites or server. Narrow lanes retain their selection. The runner regression
passes both stale-fixture refusal and normal suite ordering. The exact previously
failed lifecycle case passes in 55.98 seconds (144-second invocation including
compilation and server exit). The read-only verifier and its warning-denied Clippy
check pass; shell lint, syntax and scoped formatting pass. Evidence:
`target/review-validation/embedded-root-release-*.log`
and `embedded-root-preflight-runner.log`. No full gate, version mutation, commit,
publication or Cargo cleanup ran for this correction. The open `.49` notes include
it. The complete urgent release batch is ready for the maintainer's release retry;
these focused results do not constitute a complete validation receipt.

The reported release Clippy failure and false `[CANIC-TEST:E001] ... FAIL failing`
line were repaired before that release attempt. The reinstall fixture now uses
the equivalent inclusive range. Progress uses libtest-aware stderr reporting:
passing self-tests retain expected failures in capture, while failed native tests
and uncaptured PocketIC runs still expose diagnostics. The two runner self-tests
and two rendering tests pass; the captured run contains no failure event, and an
explicit uncaptured run still emits it. The validation-runner shell proof and
`canic-testing-internal --all-targets --all-features` warning-denied Clippy pass.
Evidence: `target/review-validation/progress-capture-*.log`. This closes those
two reported issues. The later maintainer gate also passed the reinstall journey
described below. No version or Git action was performed by the agent.

The requested generic endpoint framework work for `ic-blob-storage` is complete
in this repository. Public `on_access_denied = "reject"` keeps
plain Candid replies with normal Fleet/custom guards, denial-only metrics and
synchronous handler dispatch. `decode = LIMITS` selects public `ArgumentLimits`
for bytes, decoding, skipping, type count and header complexity. Artifact-owned
`argument_limits = LIMITS` bounds initial lifecycle envelopes before restoration
and participants. Bounded proof predicates reuse decoded arguments. Adoption
examples and semantics are in [endpoint controls](../features/runtime/update-payload-limits.md).

Focused evidence passes 50 macro tests, 28 Core access tests, 2 decoder tests,
13 facade/invariant tests, 7 public API doctests (6 compile-fail), and all 6
`pic_ingress_payload_limits` PocketIC cases. Wasm proofs cover exact reply bytes
and declarations, denial metrics/short-circuiting, malformed and over-budget
query/inter-canister arguments, and init/post-upgrade refusal before participants.
A deliberately trapped participant proves the lifecycle log witness survives
failed installation; reinstall clears prior logs. Only typed install-rate-limit
responses are retried. Affected-package/target warning-denied Clippy and scoped
formatting pass. Evidence: `target/review-validation/endpoint-framework-*.log`.

This framework batch and the existing open `.49` changelog are ready for review;
the later maintainer gate qualified the separate Fleet regression below. The earlier
in-progress macro compile errors are resolved. The sibling repository was read
only; downstream adoption, composed-service/Caffeine qualification and paid
uploads are not qualified or authorized by this work. No version bump, broad
gate, commit, publication, deployment or Cargo cleanup was performed.

An earlier maintainer-selected gate exposed a fixture ordering defect in
`generated_reinstall_recovers_lost_install_and_reaches_working_fleet`: it assumed
a withdrawal must precede every installation, although targets advance in order.
The correction accepts either injected reply-loss order, requires
both applicable interruptions and checks exact withdrawal counts after recovery.
Its focused test compiled, but replacement-artifact compilation stopped on unused
`decode`/`reject_access` fields during concurrent macro/runtime edits, before the
corrected assertions ran. Formatting and the scoped diff check pass. Evidence:
`target/review-validation/generated-reinstall-recovery-order.log`.
The later release run qualified this journey in 5m 04s; see
`target/test-runs/20261001T113447Z-23922.mBMjjH/10.log`. Its earlier qualification
blocker is closed. The subsequent stale embedded-peer failure is repaired above.

The deployment-reliability and completed-Fleet cleanup implementation, prior
direct negative/recovery evidence, propagation and cleanup are complete. Package
versions remain `0.110.48`; both changelog views describe the existing open
`0.110.49` batch. The failed release tested the maintainer-created `c36a0edf8`
checkpoint; the embedded-fixture repair above is additional uncommitted work.
This diagnosis ran only focused checks; no broad validation, version transaction,
Git publication or live deployment was performed by the agent.

[Deployment reliability audit](../audits/reports/2026-09/2026-09-30/deployment-reliability.md)
findings own the current delivery work:

- Findings 1–8 are implemented and qualified: complete test failure feedback,
  native/gated/doctest selection, bootstrap budget admission, held-source funding
  credit, generated lock ownership and exact Cargo artifact capture.
- Findings 11–16 and 18 have implemented release/publication/runner corrections
  and focused shell/package evidence. Native macOS execution remains for CI.
- Finding 17 has native and installed-package evidence for build JSON and Fleet
  automation phases, exact argument-array next actions, approval versus resume,
  and successor review. Downstream adoption in Toko Miner remains separate work
  in its read-only repository.
- Finding 10 is implemented and qualified: structured embedded-Wasm provenance,
  an explicit refresh helper, unchanged-input reuse and changed-input byte
  qualification before the early public lifecycle test. The refreshed artifact
  is byte-identical to the checked-in peer. Two native/inventory tests and the
  exact public Component Group lifecycle pass (`embedded-root-*` logs).
- Finding 9's installed-package journey passes. The extracted and installed CLI
  builds a separate consumer, reuses the exact release, deploys through public
  JSON actions, recovers three injected interruptions, verifies installed
  infrastructure/application hashes and the application endpoint, and replays
  completion with ICP unavailable and no repeated effects. The case took 506.20
  seconds, including a 399.97-second cold build and 32.14-second reuse check;
  offline replay took 1.34 seconds. Evidence:
  `target/review-validation/packaged-consumer-eighth.log` and
  `target/test-runs/20261001T093639Z-27203.vu4IVl/1.log`. This expensive journey is
  explicit opt-in, outside ordinary tests.

Package qualification exposed four product defects that are now corrected:
human progress on JSON stdout; implicit `sccache` retaining deleted scratch;
fresh Root pool queries before Wasm installation; and ordinary terminal replay
repeating live IC observations. Host leaves compiler-wrapper selection to Cargo
and retains unavailable inventory for fresh uninitialized Roots. Ordinary Ensure
and clean reinstall now share durable terminal accounting, retaining the clean
reinstall selection binding. Exact replay returns historical completion evidence;
a new review observes current state. Missing or altered evidence rejects locally.
The old compiler-cache discovery/probe and live-replanning replay paths are removed.

Seven focused compiler-wrapper tests, the positive/negative fresh-Root observation
and retained creation-balance recovery regressions, all 497 native Fleet tests,
and Host/CLI/Testing all-target/all-feature warning-denied lint pass. Final journal
publication interruption and missing/altered receipt evidence are covered. The
small completed-Fleet reset/recovery/offline-replay PocketIC journey also passes
against the shared receipt implementation (`shared-terminal-*` logs). Source
hashes were unchanged throughout the final installed-package qualification.

Existing qualification logs live under `target/review-validation/`: `deployment-*`,
`bootstrap-admission-*`, `import-funding-*`, `generated-lock-*`, `artifact-drift-*`,
`cli-*`, `release-content-native.log` and `release-core-package.log`. These qualify
their recorded source states, not subsequent changes or the complete workspace.
Do not run the entire historical release-flow/remote-state test files: some create
real fixture commits. New release fixtures use fake Git.

## Completed-Fleet cleanup

The accepted cut is implemented: approximately 16,000 Rust lines of superseded
completed-source preparation, receipt/interface reconstruction, seal/publication,
reset paths and their tests are removed. Current clean reinstall is the sole
completed-Fleet reset route. Shared certified-controller observation belongs to
bootstrap/import; unfinished activation and paid-import reconciliation remain.
The retained CANIC-188 incident bundle is untouched.

Focused evidence passes 494 Host Fleet tests, 76 Fleet CLI tests and three exact
PocketIC journeys: completed-Fleet clean reinstall/recovery/replay, unfinished
activation recovery/replay, and running-application import reset/idempotency.
Formatting, document links/semantics, layering, hard-cut and scoped shell checks
pass, including warning-denied Host/CLI/Testing all-target/all-feature lint.
Evidence is retained as `target/review-validation/fleet-cleanup-*`; no broad
suite ran. The cleanup and complete deployment batch are ready for review; the
later shared-receipt evidence above covers the subsequent terminal replay change.

The [follow-up cleanup and usefulness audit](../audits/reports/2026-10/2026-10-01/surface-cleanup-and-subsystem-usefulness.md)
removes 100 net Rust lines and 381 lines from active design/operating docs;
historical plans remain archived. All 79 focused native tests and scoped
warning-denied lint pass. Standalone Root retirement and fixture-data delivery
are larger scope candidates. The [Root retirement follow-up](../audits/working/0.110-surface-contraction/root-retirement-usefulness.md)
informs accepted FR1 below. No Fleet-to-capacity route or feature cut is qualified;
preserve unfinished paid recovery.

## Retained incident and operating constraints

The maintainer prioritised CANIC-188 on October 1. Its existing source correction
passes the exact 24-source PocketIC regression against the completed-Fleet cleanup:
all imports, lost-response recovery, nine Workloads/fifteen Ready canisters,
conservation and effect-free replay. Import used 220 of 784 reviewed calls and
51,091,964,838 of 33,008,458,000,000 allowed Root debit cycles. The case passed in
615.09 seconds (706-second invocation including compilation and fresh artifacts).
Log: `target/review-validation/canic188-post-cleanup-pocketic.log`. All 17 retained
incident-bundle checksum checks pass. No additional runtime correction, live call,
sibling mutation or broad gate was needed or performed for this verification.

The CANIC-188 frozen-state repair was withdrawn on 2026-10-01. Its
[historical decision](../audits/release-lines/supporting/0.110-fleet-runtime-contraction/canic188-issued-import-recovery.md)
and retained `.canic/incident-repairs/canic188/` bundles are evidence only. Preserve
existing journal/status/artifact evidence without maintaining the old executable
owner or its continuation route. Current-build hard cut plus reinstall governs
replacement; observed controlled cycles and unfinished paid effects still need
cycle-safe disposition. Live repair/restoration is no longer planned work.

No live deployment, incident execution, sibling mutation, version transaction,
commit or Git publication is authorized by this cleanup. Toko Miner and other
repositories remain read-only. Keep release build artifacts and retained paid
operation evidence. Agents must never create commits. Check shared `target/`
ownership before each targeted build/test; use focused checks during implementation.

Fresh live backup execution remains unavailable because Coordinator-backed
Component Registry topology preflight is unimplemented. Preserve existing backup
and same-release restore/recovery machinery; see the
[backup availability guide](../features/backup-and-restore/README.md#current-availability).
Blob extraction remains separate accepted future work, not this cleanup's scope.

## Accepted follow-up and history

[FR1 Fleet release to reusable capacity](../design/0.110-fleet-runtime-contraction/0.110-design.md#fr1-fleet-release-to-reusable-capacity--accepted-2026-10-01)
is active at the maintainer's post-push request, before final 0.110 closeout/blob
extraction. It covers retained
Coordinator/Root/Store and child IDs, conservation, one existing Host journal,
controller/reset recovery and retirement contraction. Host admission is qualified
as described above; execution and contraction remain. It authorizes no live or
downstream effects. Toko reports its distinct exhausted import recovered; its
evidence review and preventive follow-ups are tracked above.

The [0.110 tracker](../design/0.110-fleet-runtime-contraction/status.md#accepted-code-review-corrections--2026-09-30)
owns remaining R2–R8 work. Open outcomes include exhausted/older-unknown imports,
remaining funding accounting, allocation-scoped caller/issuer/funding authority,
backup upload/capture/restore authority, background-driver trap recovery and
operation-specific convergence. Its conservative count is 31 of 401 original
findings; do not treat partial corrections as closed or the full queue as a gate
for every bounded corrective release.

The accepted deployment/cleanup outcomes and their direct evidence are complete;
the open changelog covers the whole batch. Broad validation, versioning and
publication retain the maintainer-selected release boundary.
Final qualification includes FR1. The 0.110 closeout audit must be explicitly
requested and accepted before 0.111 implementation; continuation does not cross
that boundary.

Earlier checkpoints, superseded next steps and detailed timings are retained in
[historical handoffs](archive/2026-10-01-prior-fleet-handoffs.md). They are evidence,
not current instructions. The
[0.110 design](../design/0.110-fleet-runtime-contraction/0.110-design.md), tracker,
audits and governance own their respective contracts; status grants no release authority.
