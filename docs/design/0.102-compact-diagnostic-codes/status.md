# Canic 0.102 Implementation Status

Date: 2026-08-14

## Status

- State: evidence-only B1 inventory is complete and awaiting allocation review.
  No public error shape, numeric assignment, stable record or diagnostic runtime
  behavior has changed.
- Baseline: clean `main` tag `v0.101.53` at
  `23c0328f78b215580d734ef01b52b35fa3e38ade`; current-candidate
  control-plane/core source is pinned at
  `0750c309104b111fa6f5a1b3355c04fcb38faf71`.
- Release boundary: 0.102 is reinstall-only. Every Canic-owned Fleet canister
  must use one admitted release set before activation. Same-release retry,
  backup, restore and interruption recovery remain required.
- Published checkpoint: `v0.102.0` at
  `e6dfd7d2d212f9fce4b1b16caba33d8062e3461d`.
- Open checkpoint draft: `0.102.1` records the completed whole-program
  inventory for review. It changes no runtime, Candid, stable state or package
  version.
- Mutation gate: B2-B6 remain blocked until the producer manifest, dense
  allocation rows, host catalogue and projection table receive maintainer
  approval.

Detailed source-by-source working evidence is kept outside this design
directory in the
[0.102 diagnostic inventory](../../audits/working/0.102-diagnostic-inventory/index.md).
The design directory retains only the normative design, this tracker, the
pending allocation proposal and the permanent allocation-ledger contract.

## Release-Batch Plan

| Batch | Outcome | Required evidence and cleanup | Status |
| --- | --- | --- | --- |
| B1 | Freeze current diagnostic authority and Wasm baseline | Complete producer, dynamic-context and durable-string inventories; allocation proposal; host catalogue; projection owners; representative Wasm baseline | Active: inventory complete; allocation/catalogue pending |
| B2 | Add prose-free runtime identity and host catalogue | Distinct raw/registered types, approved allocations, permanent current/retired ledger, exhaustive host metadata, lookup and Wasm absence proof | Pending |
| B3 | Hard-cut the Fleet-atomic public diagnostic contract | Replace enum-plus-message with `nat16`, update every owned endpoint and generated surface, reject mixed release sets | Pending |
| B4 | Make internal propagation code-first | Remove owned prose concatenation, map typed causes, preserve explicit projections and masked-code observability | Pending |
| B5 | Bound durable diagnostic ownership | Remove redundant prose, preserve recovery-significant state and prove changed lifecycle journeys | Pending |
| B6 | Whole-program cleanup and measured closeout | Remove residue and temporary inventory tooling, regenerate bindings, remeasure Wasm and publish downstream source guidance | Pending |

## B1 Completion Contract

B1 is complete only when all of these are reviewable together:

1. one exact current producer and consumer manifest;
2. complete dynamic public-value ownership and durable-string classifications;
3. one meaning, class, origin, host disposition and public projection for each
   proposed leaf;
4. operation-correlated numeric observability for every masked diagnostic;
5. nonzero unique allocation rows across permanent current and retired history;
6. a host catalogue and generated current registry bijective with active rows;
7. a reproducible representative-Wasm baseline; and
8. explicit maintainer approval before B2 creates runtime authority.

## Closed Inventory

- 2,208 mechanical `InternalError::*` references are classified as 2,514
  effective helper/call-site dispositions by 98 bounded source passes.
- The qualified semantic set contains 2,844 exact provisional identities and
  31 safe projections, for 2,875 symbolic identities after deducting known
  same-meaning reuse.
- All 656 dynamic public-message values are classified: 287 caller-derivable,
  67 sensitive operator-only, 234 authoritatively typed and 68 requiring
  narrow request/status owners.
- All 31 projections and five exact identities reused as projections have a
  proposed operation-correlated observability owner.
- Seventeen IC-call families map to their durable operation or guarded status.
  Store publication includes its previously missing attempt owner.
- The 105-row authentication formatter, the native configuration zero-row
  exclusion and the current Canister durable-string census are closed.
- Publication binding/release authority, all 56 GC/reclamation/deletion
  constructions and both management transports are fully expanded.
- No decision parses retained failure text. Four current Cashier identities
  remain allocated in 0.102 and retire without reuse in the 0.108 hard cut.

These are evidence results, not allocated code authority. The maintained public
error remains `ErrorCode + message`, `InternalError` remains string-first and
the host still consumes typed enum variants.

## Current Decisions

- Use dense, monotonic, nonzero numbers with compact unpadded `E<decimal>`
  rendering and no semantic bands.
- Keep lossless raw decoded identities distinct from registered producer
  identities.
- Keep class, origin, disposition, labels, summaries and remediation in
  host-only code.
- Retain every allocated number permanently as current or retired; never reuse
  a retired number.
- Install all Canic-owned Fleet canisters from one admitted release set before
  activating the new public contract.
- Do not add a dual protocol, compatibility decoder, diagnostic generation
  name, message fallback or string-based classification.

The dense allocation proposal is in
[allocation-proposal.md](allocation-proposal.md). The permanent allocation
authority contract is in
[code-allocation-ledger.md](code-allocation-ledger.md).

## Validation Evidence

- `CANIC-WASM-001/v3` passes at immutable tag `v0.101.53` over six Components,
  Fleet Subnet Root, Fleet Coordinator and Wasm Store in release and debug
  profiles at risk `5/10`.
- Source-count, semantic-expansion, dynamic-category and projection arithmetic
  reconcile in the working evidence.
- Qualified symbolic identities are collision-free after known reuse is
  deducted.
- No number, host catalogue, generated diagnostic registry, runtime code,
  Candid surface or stable schema has changed during B1 inventory work.

## Next Action

Build the mechanical producer manifest, dense allocation rows and host
catalogue from the closed inventory, then present that complete set for
maintainer approval. Rerun the stable failure-string census immediately before
B5 mutation. Do not begin B2 before the allocation review passes.
