# Deferred Idea: Host-First Fleet Observatory

Date: 2026-08-18
Deferred from former 0.112: 2026-08-30

## Status

- State: unnumbered and unscheduled.
- Implementation authority: none.
- Prior direction: a generic view/renderer package, revisioned cross-role Fleet
  projection and HTML/JSON endpoint on every installed role.
- Reason for deferral: 0.109 exposed excessive runtime, build, validation and
  recovery pressure, while Toko's endpoint-heavy Wasm evidence requires
  contraction rather than another every-role runtime surface.

The former 0.112 number is retired for this concept. 0.112 now owns bounded
multi-Fleet estates.

## Retained Product Need

Downstream applications still need a supported, truthful view of:

- local role and Fleet identity;
- Root/Subnet/Store placement;
- admission and release state;
- funding and estate counts;
- incomplete operations and typed blockers; and
- freshness and evidence provenance.

Toko Miner's `CANIC-002` feedback remains the primary real-application need.
`CANIC-008` separately owns product frontend/static-asset delivery. Neither
finding requires an HTML renderer inside every Canister.

## Reoriented Direction

Any future promotion starts host/downstream-first:

1. reuse existing bounded protected command/status and terminal Registry
   authority;
2. define passive data-only views with exact source, freshness and byte bounds;
3. aggregate through a read-only host/operator path before considering a new
   runtime projection;
4. keep HTML, CSS, product terminology and application sections downstream;
5. add no Canic endpoint per view, phase, role or recipient;
6. retain no runtime renderer in roles that do not explicitly consume it; and
7. measure Wasm, build and validation cost against the 0.110 budgets before
   accepting any on-canister adapter.

## Non-Goals

This idea does not currently authorize:

- a cross-role projection protocol;
- public raw Registry, funding, transition or application records;
- Fleet mutation or controller bypass;
- an on-canister HTML renderer;
- a central polling/metrics Canister;
- downstream application dependencies in canonical Canic packages; or
- implementation before a later explicit roadmap position and batch plan.

## Promotion Evidence

Promotion requires:

- accepted 0.110–0.112 closeout evidence;
- a current downstream need that existing status surfaces cannot satisfy;
- a read-only host prototype proving field ownership and bounds;
- measured canonical and endpoint-heavy Wasm/build cost;
- a validation plan inside the accepted resource envelope; and
- explicit maintainer acceptance of the exact release position.
