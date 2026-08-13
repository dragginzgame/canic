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
- Root changelog: `Unreleased` records the clarified agent repository-scope
  boundary, Clap-owned CLI parse diagnostics and proposed 0.103 design
  reservation; no 0.102 patch version has been assigned.
- Active design and checklist:
  [0.102 compact diagnostic codes](../design/0.102-compact-diagnostic-codes/status.md).
- Proposed adjacent design:
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

No public error shape, numeric code, stable state or runtime behavior has been
changed. No 0.102 version has been assigned.

An independent operator-maintenance slice hard-cuts top-level and build option
failures to their exact Clap diagnostics and adds explicit pre-parse argv
tracing for wrapper/executable investigation. It changes no canister runtime or
0.102 diagnostic-code contract. Its exact parser and trace tests, recursive
help ordering, package Clippy, current-document guard, changelog governance and
reference-surface checks pass.

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

The proposed 0.103 line generalizes a requirement from the read-only IcyDB
0.226 design without creating a repository dependency. It would hard-cut the
current subject-only delegated session into bounded verified local
Fleet/role/scope authority and expose one synchronous read-only
`caller + scope` decision for application-owned framework adapters. IcyDB
remains unchanged and read-only. No 0.103 implementation is authorized.

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

The independent 0.103 tracker is also planning-only. Its B1 inventory and
measurement work may begin only after maintainer approval; its mutating B2-B7
remain separately blocked.

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
first forty-nine Component Registry/root-retirement/Component-provisioning/
Coordinator passes classify 1,625 effective sites and add 1,145 exact meanings
plus one projection, bringing current qualified coverage to 1,831 identities: 1,800
exact and 31 projections. That is
not whole-program
coverage, and the dynamic interpolation census is not yet complete. Its first
twelve bounded slices now classify 117 values from the Canic memory-ledger
facade, Wasm Store GC, shared manifest/capacity conversion, explicit Component Registry
denials, typed Store publication causes, delegated-session bootstrap, Store
publication binding/inventory and Store GC fence, reclamation, binding
finalization and deletion plus the two publication management transports: 53
are caller-derivable, thirteen are sensitive operator-only, twenty-eight have
existing typed owners and twenty-three are caller-required but unowned. Those
twenty-three require request-scoped Store capacity/release inspection, guarded
delegated-session capacity status, exact closed-discriminator diagnostics or
root-proxied live GC inspection, operation-scoped Store deletion progress or a
narrow operation-scoped Store-publication attempt status.
Component RPC and Runtime Introspection are closed with zero explicit
dynamic-error rows. Every dynamic publication GC invalid-state field is now
classified, as is the nested publication transport cause; their static
invariant/cause branches remain allocation work. The transitive auth formatter
remains open.

The production-source scan finds 2,208 `InternalError::*` references in 101
files after excluding external and inline test source. Component Registry ops
and workflow alone contain 1,154. The Coordinator parent file reveals one
generic receipt-invariant constructor with 235 static calls, expanding the
effective frontier to 2,442 sites. One thousand six hundred twenty-five
effective sites are classified, leaving 817 dispositions.
Both Component Registry files
are fully classified. Every remaining reference must
be linked to an existing meaning, a newly justified meaning or a
transparent/native/sediment disposition. A fresh range-owner manifest assigns
all 800 Component Registry ops sites exactly once with zero gaps/overlaps, and
the workflow source/table counts independently agree at 354. The 177-site
Component provisioning ops file is mechanically closed by four consecutive
range owners and its workflow by three. The Coordinator parent file's 154
direct constructors are mechanically closed, while 219 of its 235 hidden
receipt calls remain open. Continue those, then its dedicated root-deletion
module and workflow before the pool/root/Store and runtime owners.

In parallel, continue the dynamic-value ledger with remaining explicit runtime
constructions, the transitive auth formatter and transitive Component Registry
messages. Do not replace missing
ownership with generic detail text or a global last-error field.

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
