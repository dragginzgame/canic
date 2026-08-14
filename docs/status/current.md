# Current Status

Last updated: 2026-08-14

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the active design, source, validation or changelog material needed for the
current task.

Historical handoffs are archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md);
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md); and
- [status through the 0.101.52 Q4 qualification](archive/2026-08-12-precompact.md).

## Current Release

- Workspace package version: `0.102.0`.
- Latest published release: `v0.102.0` at `e6dfd7d2d212f9fce4b1b16caba33d8062e3461d`.
- Open changelog checkpoint: `0.102.1` closes the B1 inventory phase for review;
  package/version mutation remains maintainer-owned.
- The published checkpoint contains the completed operator-performance/CLI-
  diagnostics outcomes and evidence-only B1 snapshot; B1 continues after it.
- Active design and checklist: [0.102 compact diagnostic codes](../design/0.102-compact-diagnostic-codes/status.md).
- Evidence-only adjacent designs: [0.103 local application authorization](../design/0.103-framework-neutral-local-application-authorization/status.md) and [0.104 synchronous lifecycle composition](../design/0.104-framework-neutral-synchronous-lifecycle-composition/status.md).
- Release boundary: 0.102 is reinstall-only and is not rolling-compatible with
  pre-0.102. Every Canic-owned canister in a Fleet must come from one admitted
  release set before activation. Same-release interruption recovery, exact
  retry, backup and restore remain required.

## Current Progress

0.101 is closed. Its Q5 whole-program cleanup, artifact classification, build
identity, formatting hook and responsibility/residue report are released in
0.101.53. Historical 0.101 design, status, closeout and changelog records remain
intact apart from correcting their release truth.

0.102 B1 is active as an evidence-only batch. The complete 0.102 design has
been reread against the current source rather than its historical 0.98
inventory. The present public error still carries a Candid `ErrorCode` plus
owned message text, the maintained enum has 20 leaves, `InternalError` remains
string-first, canonical infrastructure Candid retains the old shape and the
host still matches typed enum variants.

No public error shape, numeric code, stable state or diagnostic runtime behavior
has changed. The published `0.102.0` changelog describes this evidence boundary
and the independent completed operator batch; it does not claim implementation
of the compact diagnostic protocol.

An independent operator-maintenance slice hard-cuts top-level and build option
failures to their exact Clap diagnostics and adds explicit pre-parse argv
tracing for wrapper/executable investigation. It changes no canister runtime or
0.102 diagnostic-code contract. Its exact parser and trace tests, recursive
help ordering, package Clippy, current-document guard, changelog governance and
reference-surface checks pass.

A second independent operator-performance batch passes targeted validation. It
keeps the release-build ID out of shared runtime dependency compilation,
exports local Candid from the exact selected-profile leaf Wasm, finalizes role
artifacts concurrently, admits explicit reuse of one finalized release build
for another fresh Fleet and reports display-exact additive install phases.
Exact interrupted Fleet recovery selects the same finalized build
automatically. Reuse validates the recorded profile and matching Canic builder
version, plus current topology, role/package identities, canonical manifests
and artifact bytes before activation. A real default-fast-profile Demo App build
completed in 110.31 seconds
immediately after the code change and 22.97 seconds on the latest cache-hit
run, with one configured Cargo batch and one build each for Coordinator and
Store. Whole-App output is now infrastructure-first and reports instance scope,
install-time root placement and its shared-batch duration. No package version
or 0.102 diagnostic contract has changed.

The allocation-policy proposal uses dense monotonic numbers with no semantic
bands, compact unpadded `E<decimal>` rendering and nine host-only broad classes.
It is not code authority until the maintainer approves it and reviews the
complete producer-to-leaf table.

Maintainer review approved the architectural direction and required four P1
corrections before mutation: permanent current/retired allocation history, a
Fleet-atomic activation boundary, mechanically enforced registered producer
identities distinct from raw decoded numbers, and a complete ownership audit of
dynamic values currently embedded in public messages. The normative design and
B1 contracts now include all four. This is design/evidence progress only.

The approved 0.103 direction hard-cuts the subject-only delegated session into bounded local Fleet/role/scope authority with one synchronous `caller + scope` decision.
Peer review separates an at-most-60-second proof from a protected local session up to 1,800 seconds and requires native-agent acquisition evidence. A downstream IcyDB allowlist remains independent, read-only work and not a Canic release gate.
Function-level authorization composes, but combined runtime qualification requires a separate synchronous Canic lifecycle seam. B1 may run alongside 0.102 evidence; mutation waits for accepted, complete 0.102. No runtime implementation is authorized.

The approved 0.104 direction adds one paired, compile-time synchronous application participant to managed/local non-root lifecycle after Canic restoration and before activation checks, schedulers or deferred work. Prepared post-upgrade still invokes it; Root, Wasm Store and Coordinator remain inventory-only; B1 may begin alongside 0.102/0.103, but mutation waits for accepted, complete 0.103 and explicit promotion.

Inserting 0.103 and 0.104 moves every former provisional 0.103-0.111 Canic design to 0.105-0.113 without changing its order or status. Published versions, changelogs, audit reports and archived handoffs retain their historical identities.

## Current Decision

Follow the six release batches in the active tracker. Evidence-only B1 may
continue. It must produce the exact current producer/consumer,
dynamic-public-context and durable-string inventories, a reproducible
representative-Wasm baseline, a permanent current/retired allocation ledger,
typed host disposition and explicit public projections with retrievable
operation correlation. The design's numeric examples are not authority.

Do not begin mutating batches B2-B6 until the maintainer has reviewed the
complete B1 inventories, initial allocation, host catalogue and projections.
The public cut must install all Canic-owned Fleet canisters from one admitted
release set before activation, with matching host/CLI callers and regenerated
external bindings. Do not introduce a temporary dual protocol, generation
name, compatibility decoder, diagnostic protocol version or message fallback.

The 0.103 B1 must inventory and freeze complete signed-presenter propagation from the preparation caller with no legacy fallback; an authorized mutating batch performs it.
B1 must also resolve scope, separate proof/session lifetimes and capacities, canonical ownership, purity, the complete authority-generation transition table and native-client acquisition.
B2-B7 remain blocked until 0.102 is accepted and complete and the 0.103 promotion review explicitly authorizes mutation.

The 0.104 B1 may inventory lifecycle owners, macro expansions, symbols, ordering and costs. B2-B6 wait for accepted, complete 0.103, the frozen paired `fn() -> ()` grammar/exclusions and explicit promotion.

## Validation

Freshly observed baseline identity:

```text
branch: main
commit: 23c0328f78b215580d734ef01b52b35fa3e38ade
tag: v0.101.53
worktree: clean before 0.102 documentation work
```

The fresh retained `CANIC-WASM-001/v3` baseline passes at risk `5/10` over six
Components plus Fleet Subnet Root, Fleet Coordinator and Wasm Store in both
release and debug profiles. It uses immutable tag `v0.101.53`, a clean detached
worktree, isolated local/offline build state and the checksum-pinned toolchain.
Valid v2 evidence remains superseded and non-comparable. Previously recorded
0.101 test and release claims remain historical evidence only.

## Next Action

The current-candidate scan remains pinned to
`0750c309104b111fa6f5a1b3355c04fcb38faf71`. Ninety-eight site-level passes
classify all 2,208 mechanical references and 2,514 effective helper/call-site
dispositions. RPC, Template-manifest and complete publication-workflow source
expansion brings coverage to 2,875 identities: 2,844 exact plus 31 projections.

All 656 dynamic values are classified: 287 caller-derivable, 67 sensitive,
234 authoritatively typed and 68 caller-required but unowned. Each unowned value
has a narrow proposed request/status owner. The 105-row auth formatter, native
configuration zero-row exclusion and current Canister durable-string census
are closed. No decision parses retained failure text.

Every projection has a proposed numeric observability owner. Seventeen IC call
families map to their exact durable operation or guarded status; publication
now includes the missing attempt owner. Four current Cashier leaves allocate in
0.102 and retire without reuse in the 0.108 hard cut.

The inventory phase is ready for review. B1 still requires the mechanical
producer manifest, dense allocation rows and host catalogue before B2. No
number is allocated and no runtime, Candid or stable schema has changed.

Design-document hygiene now applies across current and archived numbered
lines: design roots retain their design/status authority, while necessary
working or historical supporting evidence lives under `docs/audits/`. This
changes document placement only and preserves historical findings.
