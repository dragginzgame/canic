# Current handoff

## Isolated PR #596 source and scope

This contribution follows committed main `4caeec25d`, retaining shared branch
history, and was last pushed as `5bcdc7677`. The normal checkout's wider pending
work and newer dependency adoption are excluded. Own package versions remain
0.111.0, with the existing 0.112.0 changelog batch. No merge, version bump, release,
publication, lifecycle operation or normal-checkout mutation is included.

Canic keeps its recipes, source/config custody, transformation, release admission,
application protocols and retained-artifact policy. IC Build's unified engine
executes Cargo; Canic now retains successful runtime CargoBuildFacts through
configured single/batched and infrastructure artifacts, provenance, CLI/internal
fixture callers and embedded allocation-peer evidence. Provenance projects actual
selection, observed tool identities and captured Rust flags. Failed exits retain
original status and diagnostics without claiming successful facts. Fresh tool
probes and ambient recipe reconstruction are removed; evidence schema and UTF-8
refusal are preserved. Constructors must provide facts from the runtime build.

The maintainer's narrow authority-recovery repair from `8c2dda483` is carried with
attribution, using existing IcOps traps and identical messages. Its wider
unpublished parent is excluded. The layering guard remains unchanged.

## Published-owner CI completion

The branch retains published Build 0.19.0 and Host 0.27.6. Root and both private
Cargo graphs are fetched locked before offline CI admission. The private locks
align only Timers 0.28.0 with the root's existing 0.29.0 selection. ShellCheck follows
sourced helpers. Formatter failures retain/replay diagnostics through the shared
fixture lifecycle; preflight explicitly prepares and checks pinned Cargo tools.
Development CI selects Rust 1.99.1; consumer MSRV stays 1.91.

At `5bcdc7677`, hosted preflight and security pass. Subsequent jobs exposed fixture
imports from Memory's removed substrate re-export and hook qualification using
the installer's retired no-argument form. Six fixture modules now use published
IC Storage StableCell/StableBTreeMap over the same checked IC Memory leases.
Backing types and the Memory trait come from IC Memory. Constructor, iterator,
payload, key and allocation contracts are preserved; no storage migration, raw
manager or compatibility export is added. Three private packages inherit the
already-selected IC Storage 0.5.4 dependency. Only those root lock edges change;
all selected package identities remain unchanged. The Canic-owned hook fixture
passes --consumer explicitly; shared installer/hook bytes are unchanged.

The pending Build 0.23 observer API port is a separate held contribution. Do not
mix unpublished requirements, source overrides or its Host alignment into this PR.

## Validation and remaining qualification

Current focused Linux checks pass: real hook fixture, source-following ShellCheck,
grouped formatting, dependency inheritance/pins, strict Clippy and actual Rust
1.91 native library checks for Hub, Shard and the managed lifecycle probe.
Direct Cargo Wasm checking is refused by the existing authoritative-build guard;
do not bypass it or claim Wasm/installed acceptance. Hosted exact-head checks,
authoritative Canic builds and the owning managed tests remain necessary.

Prior facts qualification includes 41 focused build/provenance cases, twenty
enabled reuse cases, 26 provenance cases and four embedded-evidence cases.
One explicit real-Wasm reproduction remains ignored. Host/CLI/Internal caller
checks and strict Host Clippy passed; full suites, native macOS and complete
managed/PocketIC qualification are separate. Preserve the historical identities
and limits in the immutable
[5bcdc7677 handoff](https://github.com/dragginzgame/canic/blob/5bcdc7677f72917be600b5c8bfa62e0c3196f627/docs/status/current.md).

Current evidence/before-images: `/tmp/ic-build-hardcut-more.RkD1D3/`.
Earlier facts evidence: `/tmp/ic-build-fleet-followup.KGzFhf/` and
`/tmp/ic-build-followthrough.epj0I5/`. Leave review/integration to the owner;
consumer release and installed acceptance remain outside this contribution.
