# CANIC-181 selected-build endpoint discovery — 2026-09-22

Toko's published .36 adoption passes full CI and verifies same-checkout fast
build reuse in 2.00 seconds. It confirms a remaining Canic CLI defect: endpoint
inspection looks for a local ICP sidecar after a complete managed build retained
the declaration only under its release-build directory. Compiled payload metadata
and runtime enforcement already pass downstream acceptance.

## Correction

`canic info endpoints <fleet> <role> --release-build <sha256> --json` uses the
release-build ID emitted by `canic build`. Explicit selection bypasses live
lookup and resolves the role through the existing managed artifact owner. It
verifies finalization, canonical application/infrastructure manifest digests and
the exact declaration hash, then parses the bytes that were verified. There is
no newest-build heuristic or fallback to live/local data on a selected-build
failure. No current application topology is needed to inspect a sealed contract;
the normal deployment loader still validates against its supplied topology.

Both application and infrastructure roles work. JSON reports `source_kind`,
`source` and nullable `release_build_id`. Plain output identifies its source.
Without explicit build selection, live metadata remains preferred and local
sidecar lookup now uses the selected environment. Missing-sidecar guidance points
to the build selector. JSON additions may affect strict downstream decoders.
The Fleet positional is retained for the existing command and is unused during
explicit build inspection. No new state owner, protocol generation, alias file,
background process or publication step is introduced.

## Evidence

- 77 targeted host release-set tests pass, including finalized-build admission,
  changed manifests/declarations, missing/unknown roles, unsafe sidecars and
  preserved topology-bound loading.
- Four focused CLI library tests pass, including typed build-selection failures
  despite an available local sidecar and selected-environment lookup.
- Both recursive help/order tests pass.
- Scoped all-target/all-feature warning-denied Clippy passes for canic-host
  and canic-cli. Its initial visibility and test-fixture length findings were
  corrected without suppressions; all four affected fixture tests pass again.

The candidate executable also inspects Toko's exact .36 Game Shard and Root
artifacts from build
`8e60e7f14b3a091780f3eb0aa9260504fd4e9f78ac5c72d0f33a7bb39e837ab3`.
A Canic-owned diagnostic workspace contains byte-identical copies of 14 retained
input files, preserving the original managed directory layout. There are no ICP
sidecars or installed Fleet, and the command receives a nonexistent ICP executable.
Both commands succeed with `source_kind: built` and the exact build identity.
Game Shard reports `execute_actions` ingress and update-guard limits of 49,152
bytes with `explicit_override`; `bootstrap_user` reports 16,384 bytes.

The diagnostic copies isolate this check from concurrent downstream work; they
are not a proposed application workaround. Every original and copied input still
matches its recorded hash. Toko's repository and frozen qualification workspace
remain read-only. This proves candidate endpoint discovery over the exact
published build evidence, not downstream adoption of an unpublished CLI or a
live deployment. There is no new Wasm/runtime behavior to requalify with PocketIC.
The first CLI check selected the binary target and discovered zero tests; the
subsequent library-target command supplies the four actual unit-test results.

Logs and bounded receipts are retained under `.tmp/feedback37/`; the
[structured record](endpoint-discovery.json) binds source, input and result hashes.
The existing observation-performance qualification remains separate and unchanged.

## Feedback disposition and release scope

CANIC-181's new discovery defect is corrected locally in the same open .37 batch.
CANIC-176's fast-profile same-checkout reuse is verified by Toko; release-profile
and relocation acceptance remain open. CANIC-148 retains matched application
savings/full-retention acceptance. CANIC-166/172 retain downstream readiness
integration and separately authorized live recovery acceptance. CANIC-150/160/170
retain live timing, observation and concurrent-operator acceptance. None of those
remaining acceptance items establishes another new defect from this scan.

The correction extends the existing .37 performance batch and changelog drafts;
the complete accepted batch is ready for the maintainer's release flow and
package versions stay .36. No broad validation, Git mutation, release, deployment
or sibling mutation ran. B1 and the human minor-closeout gate are unchanged.
