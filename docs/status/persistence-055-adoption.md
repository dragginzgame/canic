# Persistence 0.55 adoption contribution

Prepared from published Canic main `5c236ab130c0d63892ef281b45bdc3fa353e012f` in
`/tmp/canic-persistence-adoption.kfhYIJ`. The normal checkout's active uncommitted
0.113 work remains untouched; none was copied into this contribution. The
maintainer explicitly selected a separate adoption PR. Own package versions
remain 0.112.0 and notes join the already accepted 0.113 batch.

## Selected graph and boundaries

Root and both private Blob graphs select Memory/Storage 0.55.1, Blob/Contracts
0.56.0, Metrics 0.17.3, Timers 0.32.0, Platform/macros 0.7.6 and Wire/derive 0.7.21.
Only these supplier identities change; the root retains its host-only Query
Metrics 0.16.3 edge. Native Query values cannot substitute for runtime 0.17 values.
The growth-refusal fixture delegates MemoryBacking identity to its VectorMemory.
No production runtime source, allocation key or collection encoding is changed.

The active owner must preserve its Blob relocation, newer dependencies, provider
metrics and heap Logging changes. #708's final stable Metrics dependency must
match the same Persistence family. Logging Stable is now published upstream but
has no caller in this published base; no speculative dependency/integration is
added. Blob's retained obligations still require explicit retirement disposition;
no deployment or stored data was changed here.

Related owner: [#527 — Qualify the selected Memory graph, composition and diagnostics](https://github.com/dragginzgame/canic/issues/527).
This contribution advances dependency alignment; its broader composition and
installed acceptance remain open. #381's custom-codec investigation is separate.

## Evidence and limits

Focused native Core/Blob library Clippy and runtime Wasm Clippy pass with warnings
denied. Both private consumer native all-target/all-feature Clippy selections
pass. Pins, production graph, formatting, documentation links and diff checks
pass. All 34 selected Core Memory regressions pass, including growth refusal
and committed allocation diagnostics. Dependency admission passes for all three
workspaces with one Memory/Storage/Platform identity in each deployed graph.

The initial Memory test compile correctly refused a wrapper lacking the new
backing custody trait; its delegation repair passes. Initial direct private-app
Wasm Cargo checks correctly stopped at Canic's canonical role-contract build
guard; that marker was not fabricated or bypassed. Supported native fixture
Clippy and runtime library Wasm checks are separate proofs. Managed app Wasm,
installed composition/recovery, artifact peer refresh, macOS and broad release
qualification retain their own scope and are not claimed here.

The first Make graph observation stopped because the isolated checkout had no
prepared sccache selection. The documented explicit empty RUSTC_WRAPPER selection
admits the same read-only graph without installing or inventing a cache binary.
The failed attempt remains retained.

Logs are retained at `/tmp/canic-persistence-055-*`. No old failed evidence,
normal checkout artifacts or pending release inputs were removed.
