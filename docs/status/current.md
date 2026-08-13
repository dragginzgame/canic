# Current Status

Last updated: 2026-08-13

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the active design, source, validation or changelog material needed for the
current task.

Historical handoffs are archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md);
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md); and
- [status through the 0.101.52 Q4 qualification](archive/2026-08-12-precompact.md).

## Current Release

- Workspace package version: `0.101.53`.
- Latest published release: `v0.101.53` at
  `23c0328f78b215580d734ef01b52b35fa3e38ade`.
- Root changelog: `0.102.0` is ready for maintainer-owned full validation as an
  explicitly requested checkpoint for the completed operator-performance/CLI-
  diagnostics outcomes and evidence-only B1 snapshot. Package versions remain
  `0.101.53`; no release version mutation or tag has occurred.
- Active design and checklist:
  [0.102 compact diagnostic codes](../design/0.102-compact-diagnostic-codes/status.md).
- Evidence-only adjacent design:
  [0.103 framework-neutral local application authorization](../design/0.103-framework-neutral-local-application-authorization/status.md).
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
has changed. The `0.102.0` changelog draft describes this evidence boundary and
the independent completed operator batch; it does not claim implementation of
the compact diagnostic protocol.

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

The approved 0.103 direction hard-cuts the subject-only delegated session into
bounded local Fleet/role/scope authority and exposes one synchronous
`caller + scope` decision for application-owned adapters. It derives from
read-only IcyDB requirements without adding a dependency. IcyDB remains
read-only. B1 evidence may begin; no runtime implementation is authorized.

Inserting that line moves every former provisional 0.103-0.111 Canic design to
0.104-0.112. Their intended order and implementation status are unchanged;
published package versions, historical changelogs, audit reports and archived
handoffs retain their original evidence identities.

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

The 0.103 B1 must propagate a required signed presenter derived from the
preparation caller, with no presenter-less or subject-equals-caller fallback,
and resolve scope, replay, ownership and purity gates. B2-B7 remain blocked.

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

Continue B1 with the direct-constructor frontier and the new
[dynamic public context inventory](../design/0.102-compact-diagnostic-codes/dynamic-public-context.md)
before assigning numbers. The typed and explicitly expanded family ledgers
began with a collision-free qualified subset of 685 symbolic identities. The
first seventy-three Component Registry/root-retirement/Component-provisioning/Coordinator/pool/Store passes classify 2,053 effective sites and add 1,678 exact
meanings plus one projection, bringing current qualified coverage to 2,364
identities: 2,333 exact and 31 projections. That is not whole-program coverage,
and the dynamic interpolation census is not yet complete. Its first nineteen
bounded slices classify 138 values from the Canic memory-ledger facade, Wasm
Store GC, shared manifest/capacity conversion, explicit Component Registry
denials, typed Store publication causes, delegated-session bootstrap, Store
publication binding/inventory and Store GC fence, reclamation, binding
finalization and deletion, the two publication management transports and the
Coordinator root-deletion closed labels, Coordinator initialization, pool asset
principals, import routing evidence, root Store bootstrap and adoption state:
66 are caller-derivable, sixteen are sensitive operator-only, thirty-one have
existing typed owners and twenty-five are caller-required but unowned. Those
twenty-five require request-scoped Store capacity/release inspection, guarded
delegated-session capacity status, exact closed-discriminator diagnostics or
root-proxied live GC inspection, operation-scoped Store deletion progress or a
narrow operation-scoped Store-publication attempt status.
Component RPC and Runtime Introspection have zero explicit dynamic-error rows.
Every dynamic publication GC invalid-state field is now
classified, as is the nested publication transport cause; their static
invariant/cause branches remain allocation work. The transitive auth formatter
remains open.
The current-candidate constructor scan is pinned to `0750c309104b111fa6f5a1b3355c04fcb38faf71`;
the post-baseline control-plane/core diff adds or removes no site. It finds 2,208
`InternalError::*` references in 101 files after excluding external and inline
test source. Component Registry ops and workflow alone contain 1,154. The Coordinator parent file reveals one
generic receipt-invariant constructor with 292 static calls across the parent,
root-deletion and deployment-ledger sources, expanding the effective frontier
to 2,499 sites. Two thousand fifty-three effective sites are classified, leaving
446 dispositions. Every remaining reference needs an existing/new exact meaning
or a transparent/native/sediment disposition. A fresh range-owner manifest assigns all 800 Component Registry ops sites exactly
once with zero gaps/overlaps, and the workflow source/table counts independently
agree at 354. The 177-site Component provisioning ops file is mechanically closed by four consecutive
range owners and its workflow by three. The Coordinator parent file's 154
direct and 235 parent-file hidden calls are closed, as are root deletion's 21
direct and 10 hidden sites and deployment-ledger's two direct and 47 hidden
sites. The 12-site Coordinator workflow, all 69 Canister pool ops and all 17
pool workflow/refill references and the 23-site root Store bootstrap workflow
are closed. Root bootstrap and Store adoption state are also closed. Continue
with remaining Wasm Store and Mirror/Directory synchronization owners.

In parallel, continue the dynamic-value ledger with remaining explicit runtime
constructions, the transitive auth formatter and transitive Component Registry
messages. Do not replace missing ownership with generic detail text or a global
last-error field.

All 31 currently known projections and five exact leaves reused as projections
have proposed observability owners. Seventeen IC call families are mapped to
their operation-specific durable authority or guarded runtime status, including
the missing Store-publication attempt owner; a masked code must be attached to
that same status/operation or correlated by an
existing retrievable operation ID. Four current Cashier leaves are included in
0.102 and retire without reuse in the 0.107 hard cut. The permanent ledger
contract is recorded, but no current or retired number is allocated. B2 remains
blocked until the complete site and dynamic-context manifests, allocation,
catalogue and projection table receive maintainer approval.
