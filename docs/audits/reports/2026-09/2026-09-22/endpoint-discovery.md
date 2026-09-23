# CANIC-181 selected-build endpoint discovery — 2026-09-22

## Published downstream acceptance — 2026-09-22

A subsequent read-only scan of Toko Miner's local feedback verifies CANIC-181
on published Canic/CLI .37. The application inspects fresh Game Shard and Root
declarations by exact build ID without an ICP sidecar or installed Fleet, with
an intentionally nonexistent ICP executable. The expected 49,152-byte explicit
action limit and 16,384-byte default are reported; a missing selected build
rejects. Real-Wasm payload-boundary qualification also passes.

Toko records every CI gate passing on frozen application commit
`75e727cef3c071bb67997f0dd25b5e6d32db7094` plus the Canic dependency update,
including all six managed scenarios. The proof uses IcyDB .261.5; current Toko
feedback selects .261.6 with separate composition evidence. Concurrent/later
gameplay and live deployment are outside the frozen Canic adoption proof.
An initial shared-target executable disappearance is retained and the remaining
gates pass using private outputs; this establishes no new Canic runtime defect.

Source: Toko's `docs/upstream/canic.md`, SHA-256
`2f3bd886f74dc80afded325e3b216400195575fa1ea16e9089215e0d051b05de`;
adoption receipt `docs/upstream/artifacts/canic-0.110.37-adoption-2026-09-22.json`,
SHA-256 `caf87c051266bc91f4cf881434c0b9502a1537ff36e87ee1de611cfc41e90a1e`.
The scan reads sibling files only. Representative deployment timing, matched
sampler/full-retention cost, exact live recovery, concurrent-operator and
release-profile/relocation acceptance remain open with their existing owners.
No new confirmed Canic defect is reported. The sections below retain the
original pre-publication correction evidence.

## Original correction context

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
