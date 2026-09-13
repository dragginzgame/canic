# ICYDB-033: composed startup-driver outlining result

Measured 2026-09-13. **Discard `#[inline(never)]` on the generated
`startup_driver_attempt`: compiler and final Wasm are byte-identical.**
The annotation was removed from the isolated experimental copy. No production
source, dependency pin, recovery behavior or sibling checkout was changed.

This tests the existing Canic empty, metrics-enabled lifecycle participant
against IcyDB 0.257.11. It follows IcyDB's standalone result in
`docs/reports/investigations/2026/09/13/composed-wasm-preflight.md`, section
“Startup-driver outlining result”. The older 0.257.9
[host/participant measurement](report.md) is a separate experiment; no delta
against it is used here.

## Matched results

| Stage / metric | Baseline | Annotation candidate | Delta |
| --- | ---: | ---: | ---: |
| Final raw Wasm bytes | 3,820,852 | 3,820,852 | **0** |
| Final code-section bytes | 3,585,114 | 3,585,114 | **0** |
| Final defined functions | 7,164 | 7,164 | **0** |
| Compiler raw Wasm bytes | 6,143,407 | 6,143,407 | 0 |
| Compiler code-section bytes | 4,138,217 | 4,138,217 | 0 |
| Compiler defined functions | 8,194 | 8,194 | 0 |

The final data-section payload is 224,136 bytes in both. Section sizes include
their encoding; defined functions exclude the 38 imports. Final Wasm has no
custom sections. Direct binary comparison establishes equality, beyond equal
sizes/counts. Candid and gzip outputs are also byte-identical; compressed size
is not the deployable raw-byte measurement.

Identical baseline/candidate SHA-256 values:

- Compiler: `c4f97610615dbaae0979ff3784ba4369240dc09bfd03bf560470f4e41459d9e0`.
- Final: `5b1f9e385fbb2fc81ef555f937072d9dbfcfc96b3eef942664b6dfba523a490c`.
- Candid: `fb43514908eb5b8d889156733a781ec621a866b44f437b9449d85819b4c2df74`.

## Source and build controls

- Canic base commit: `8907442e028ebe44f47090783c39f64bcf940b04`
  (`v0.110.15`), with the current dirty work preserved. Its 1,791 recorded
  Rust, manifest, lock, Candid, shell, Makefile and toolchain inputs remained
  unchanged through both builds. Source manifest identity:
  `8803fafdc423f83798970a7ea715e7a50bb0fb5809f19e315d3b48bdcb8e0ca5`.
- IcyDB source is a Git archive of published 0.257.11 commit
  `8f830189fe2de079939ccc0264b87ca2eac31b1f`, tree
  `fe156344b2a619c832c08cf1278a3f2fb4515e23`, extracted into invocation-owned
  scratch. All six IcyDB packages resolve to that archive through local
  patches. The active IcyDB checkout was only read and stayed unchanged.
- Archived baseline source identity:
  `fb15c1f8c4e2b66644eb8db8cc3f06dfd809d590a3ffe67c0ef0ea90f786b516`.
  Candidate identity:
  `930c2ca2e6aa05bef5dbe9b8fbaf8eb7a314a76c19d3ff1cb90a82119af85369`.
  Across 3,193 archived files, only the annotation in
  `crates/icydb-model/src/build/actor/db/store.rs` differed.
- The existing Canic audit fixture's seven files were copied to scratch.
  Only its isolated manifests were prepared for absolute Canic paths and
  IcyDB 0.257.11. Fixture source identity:
  `1a255f9fe327a716a6f19890446f879078e33bc646f631aada7eec959a3fa8b9`.
  Its fresh lock has SHA-256
  `ab8f0051227b028b723f7011624a17c0f330ed2d88748f4a55923e4ffb9d13c3`
  and is identical for baseline/candidate. Root and maintained fixture locks
  were unchanged. The source path and target directory were reused between
  sequential builds, avoiding an additional path-identity difference.
- Both select `participant`, no default features, IcyDB `metrics`, ic-memory
  0.13.3, ic-timers 0.7.0, ic-cdk 0.20.2 and Candid 0.10.35. SQL,
  `test-admin-api` and Canic provisioning test features are absent. The fixture
  remains empty, uses memory IDs 100–106, and runs the existing synchronous
  participant hooks under Canic's lifecycle owner. This is not a Toko schema,
  dependency-lock or workload reproduction.
- Rust/Cargo 1.98.1; rustc commit
  `48a229ceaefd4985c50990b14116b6d856af0985`, LLVM 22.1.8;
  `wasm32-unknown-unknown`, release `opt-level=z`, LTO, one codegen unit,
  panic abort, no debug information, overflow checks or incremental builds.
  The common `CARGO_PROFILE_RELEASE_STRIP=none` preserves compiler names;
  canonical finalization strips them. No custom Rust flags or compiler
  wrapper. `CANIC_MEMORY_BUCKET_PAGES=16`, local network, sidecar-only Candid.
- Synthetic release-build identity is shared:
  `cf46982ebf1acbe39fbc1fb6c7cc43fb946b7baa094df79c9a9327a260cda32d`.
  The freshly rebuilt existing audit runner uses Canic's artifact builder
  and section parser. ic-wasm 0.11.1 `shrink`, then Binaryen 132 `-Oz`, with
  identical mutable-global, nontrapping-float-to-int, bulk-memory,
  sign-extension and bulk-memory-optimization features. Candid extractor is
  0.1.6. Exact executable hashes and build environment are retained below.

[Source evidence](outlining-source-manifest.json) defines the hashing
convention and records every input hash, isolated manifest adjustment and
exact one-line patch. [Measurements](outlining-measurements.json) retain the
full lock text, dependency feature tree, tools, transforms and comparisons.

## Generated-source and separate named evidence

Both declaration and runtime build outputs contain the candidate annotation
immediately on `startup_driver_attempt`. Removing just those generated tokens
restores each baseline `actor.rs` byte-for-byte. Generated source hashes are:

- Baseline, both stages:
  `0b574a4f2cf6985a303cdb21fbc5be8f3dc1d25a46746fa12f6d9e39b9507492`.
- Candidate, both stages:
  `28203f2031e63b67d81747d533b8096dd50b6b07163b04ef7d0562bcbe1a92f6`.

IcyDB's maintained participant macro includes that generated `actor.rs`.
Both artifact runs extracted current declaration Wasm after a cache miss;
candidate declaration and runtime generation used the annotated generator.
The result is not merely a comparison of two stale saved baseline outputs.

Because compiler inputs are identical, one shared named diagnostic was built
from the retained compiler Wasm using `ic-wasm shrink --keep-name-section`,
then the same Binaryen flags with `-g`. Its raw size is **5,016,634 bytes**,
with 3,585,114 code bytes and 7,164 defined functions. These are separate
diagnostic figures, not the installation-size result.

Twiggy 0.8.0 still identifies the timer `register_watchdog` closure instantiated
with the generated `startup_watchdog_callback`: **130,992 shallow bytes and
897,730 retained bytes**. This shows that the previously identified large
watchdog route exists in the current composed subject. Retained subtrees
include recovery and storage dependencies; they are not an additive savings
estimate. No exact mapping from named bytes to canonical function bodies is
claimed. The unchanged deployable artifacts decide this experiment.

## Validation, reproduction and limitations

Both canonical builds succeeded. Source and lock checks passed; the exact
one-line source difference and both generated outputs were verified. The final
Wasm passes `ic-wasm check-endpoints` against its matching Candid plus the
existing explicit lifecycle/timer hidden-endpoint list. One init, one
post-upgrade and the intended metrics endpoints remain present.

No watchdog IC instruction/cycle measurement or lifecycle/recovery execution
was performed: the candidate failed the size-improvement gate and was
discarded. No synchronous initialization counter or wall-clock duration is
offered as deferred recovery performance evidence. This result supports
discarding this annotation under these controls; it does not establish that
startup/recovery has no optimization opportunity or explain Toko's binaries.
Any different candidate that reduces size still needs actual watchdog
instruction/cycle and lifecycle/recovery qualification before adoption.

Scratch evidence lives under `/tmp/canic-icydb033-outline-a5cwqe0m/evidence`;
the isolated fixture/source are siblings there. The target directory is
`target/icydb033-outline/wasm`. Build logs are
`/tmp/canic-icydb033-outline-{runner,baseline,candidate}.log`. These locations
are disposable; structured evidence above is retained in the repository.
Reproduction uses the existing `icydb_composed_audit build <fixture> <evidence>
participant`, identical recorded environment and lock, saving each artifact
before changing only the archived generator annotation. The candidate patch
is retained as evidence; the scratch source has been restored to baseline.

No measurement framework, production optimization, root dependency update,
new fixture variant, broad test suite, version bump, commit or push was added.
The investigation is complete and recorded in the existing open 0.110.16 draft.
