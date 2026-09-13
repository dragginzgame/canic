# ICYDB-033: first controlled Canic composition pair

Follow-up: the fresh 0.257.11 composed
[startup-driver outlining experiment](outlining.md) is complete. Its matched
baseline/candidate binaries are identical; the annotation was discarded.
The historical feature-cost pair below is not that experiment's baseline.

Measured 2026-09-13, after CANIC-168's build-cache qualification completed.
This completes the requested empty-participant investigation. No query, write
or per-entity variants have been started.

Adding one empty, metrics-enabled IcyDB participant increases the optimized
Canic host's code section by **1,387,426 bytes (63.2%)**. There are no entities
or application database operations in this fixture. Named diagnostics point
first to the startup watchdog and its recovery/schema/storage dependencies.
This identifies an investigation owner, not a proven removable byte budget.

## Controlled pair and exact measurements

Both variants compile the same
[audit package](../../../../canisters/audit/icydb_composed/Cargo.toml),
[host source](../../../../canisters/audit/icydb_composed/src/lib.rs) and
[Canic configuration](../../../../canisters/audit/icydb_composed/canic.toml).
Only the `participant` Cargo feature changes. The baseline does not select an
IcyDB runtime dependency. The participant selects IcyDB's `metrics` feature,
its metrics query and reset endpoints, and one journaled store with zero
entities, matching IcyDB's existing empty-metrics audit shape. Its memory IDs
are 100–106, as in the existing Canic/IcyDB lifecycle participant fixture.

The Canic role is `probe`, with the `runtime` capability and leaf metrics
(core, runtime, security). SQL is disabled. Neither variant selects
`internal-test-fixtures` or imports the provisioning lifecycle test machinery.
The existing synchronous participant hooks run under Canic's lifecycle owner.
There is no application query/write endpoint or test provisioning endpoint.

These are final stripped, uncompressed Wasm measurements from the existing
Canic section parser:

| Variant | Raw Wasm bytes | Code-section bytes | Data-section bytes | Defined functions |
| --- | ---: | ---: | ---: | ---: |
| Canic host | 2,396,348 | 2,195,334 | 193,257 | 3,989 |
| Host + empty metrics participant | 3,819,840 | 3,582,760 | 225,490 | 7,158 |
| Participant increment | **1,423,492** | **1,387,426** | **32,233** | **3,169** |

Code/data sizes are section payload sizes, including their vector/segment
encoding. Defined functions exclude imports. Both modules import 38 functions;
`ic-wasm info` therefore reports 4,027 and 7,196 total functions. Final gzip
sizes are 864,262 and 1,414,653 bytes, respectively.

The raw compiler inputs, before `ic-wasm` and Binaryen, are separately retained:

| Variant | Compiler Wasm bytes | Code-section bytes | Defined functions |
| --- | ---: | ---: | ---: |
| Host | 3,846,229 | 2,534,692 | 4,612 |
| Participant | 6,141,210 | 4,135,228 | 8,184 |

## Identity and build controls

- Canic base commit: `8907442e028ebe44f47090783c39f64bcf940b04`
  (`v0.110.15`), with the existing uncommitted memory and cache work preserved.
  Package versions remain 0.110.15. This is not a clean-tag measurement.
- Frozen source identity:
  `08c7cbd69df79c32ef4bc60f753d60d379284a47e9881af1a962df1ed1b2a58e`.
  All 1,654 recorded Rust/TOML/Candid/lock and measurement-tool inputs stayed
  unchanged across the final pair. [Source manifest](source-manifest.json)
  records every path/hash and the manifest's exact hashing convention.
- The independent [audit lock](../../../../canisters/audit/icydb_composed/Cargo.lock)
  selects published IcyDB 0.257.9 throughout its package family, one shared
  ic-memory 0.13.3, ic-timers 0.7.0, ic-cdk 0.20.2 and Candid 0.10.35. Every
  registry package identity/checksum in this lock matches the current Canic
  root lock. The audit did not change the root manifest or lock.
- IcyDB's published crate VCS record identifies
  `818d4350e310af304fcb41b36357c292075c40a2`. The read-only Toko reference instead
  uses IcyDB 0.257.10 at `82b0d71c9346024825c1f6404b91e10361516f8c`, released
  Canic 0.110.15 and ic-memory 0.13.2. This pair is explicitly synthetic; it
  does not reproduce Toko's source, lock, schema, profile or workload.
- Rust/Cargo 1.98.1, target `wasm32-unknown-unknown`; release `opt-level=z`,
  LTO enabled, one codegen unit, no debug information, panic abort, overflow
  checks and incremental compilation disabled. Both compiler passes retain
  symbols through the common `CARGO_PROFILE_RELEASE_STRIP=none` override;
  canonical finalization strips them. No custom Rust flags are set.
- Both variants use `local`, sidecar-only Candid, 16-page Canic memory buckets,
  an explicit empty compiler wrapper and the same synthetic release-build ID:
  `cf46982ebf1acbe39fbc1fb6c7cc43fb946b7baa094df79c9a9327a260cda32d`.
  The protocol profile digest can differ because the Candid surface differs.
- Canonical finalization uses ic-wasm 0.11.1 `shrink`, then Binaryen 132 `-Oz`.
  Both require the same five features: mutable globals, nontrapping float-to-int,
  bulk memory, sign extension and bulk-memory optimization. Candid extractor
  is 0.1.6; Twiggy is 0.8.0. Exact executable hashes, compiler commit/LLVM,
  environment, dependency identities and runtime feature trees are in
  [measurements.json](measurements.json).

Final canonical SHA-256 identities:

- Host: `c3cfc9d8a49f65d9f153359bb99e404c0f73e54259e3022f07e3cd286f5f1672`.
- Participant: `368021ada457ee05db9a3effb9265cc74fcf53fee622254e8939431dd1fcff77`.

## Separate named attribution

Each named artifact starts from that variant's retained compiler input and uses
`ic-wasm shrink --keep-name-section`, followed by the same Binaryen `-Oz` and
feature flags with `-g` added solely to preserve names. Named raw sizes are
3,060,954 and 5,015,297 bytes. They are diagnostic artifacts, not installation
size figures.

Their code/data sizes and defined-function counts match the canonical outputs.
Removing custom sections does **not** produce byte-identical canonical Wasm:
type, function and code sections differ, including internal ordering/indexing.
The remaining non-custom section payloads match exactly. Section hashes and
comparisons are retained; no exact canonical byte-to-symbol mapping or semantic
equivalence proof is claimed.

The existing capability report accounts for all bytes in each named artifact.
Its `application_and_upstream` category also includes the diagnostic name
subsections (function names alone occupy 662,710/1,192,949 bytes). Consequently,
its category totals and `named_code_fraction` must not be read as canonical
code-section attribution. The useful evidence here is the individual named
functions and Twiggy's dominator graph, with that limitation retained.

The leading IcyDB-related route is the `ic_timers::runtime::register_watchdog`
closure instantiated for `__icydb_generated::startup_watchdog_callback`:
**131,017 shallow bytes and 897,805 retained bytes** in the named participant.
Its dominated children include:

| Symbol, shortened for readability | Shallow bytes | Retained bytes |
| --- | ---: | ---: |
| `commit::recovery::apply_prepared_journal_batch` | 13,522 | 62,653 |
| `commit::recovery::recover_domain` | 20,101 | 48,537 |
| `commit::prepare::prepare_row_commit_for_entity_impl` | 11,691 | 45,646 |
| `commit::recovery::fold_oldest_journal_batch` | 24,500 | 43,055 |
| `AcceptedRuntimeEntity::prepare_commit_context` | 8,949 | 31,414 |
| `SchemaProposal::try_compose` | 18,257 | 22,948 |

The graph also retains generic index/data B-tree operations beneath recovery.
This is evidence of a substantial startup/recovery dependency path even with
zero entities. The first candidate owner is IcyDB's generated startup/watchdog
and recovery registration boundary, including its interaction with ic-timers.
It is not evidence that the timer library alone owns these bytes or that recovery
can safely be removed. Retained sizes include dominated code/data, overlap when
subtrees are nested, and are not an additive savings estimate.

## Qualification, reproduction and limits

The [audit runner](../../../../crates/canic-host/examples/icydb_composed_audit.rs)
reuses `CanisterArtifactBuilder` and `read_wasm_artifact_metrics`. It does not
install canisters, create a release plan or publish artifacts. The two synthetic
artifact outputs are copied before the next variant overwrites the role output.

After checking that no other Canic build/validation owns the shared target,
build the runner with `CARGO_NET_OFFLINE=true RUSTC_WRAPPER= cargo build --locked
-p canic-host --example icydb_composed_audit`. From the repository root:

```bash
export CARGO_NET_OFFLINE=true RUSTC_WRAPPER=
export CARGO_TARGET_DIR="$PWD/target/icydb033/wasm"
export CARGO_PROFILE_RELEASE_STRIP=none CANIC_MEMORY_BUCKET_PAGES=16
for variant in host participant; do
  target/debug/examples/icydb_composed_audit build \
    canisters/audit/icydb_composed target/icydb033/evidence "$variant"
done
```

Use the same source manifest and unset custom profile/Rust flag overrides for
an exact comparison; later dependency/source changes require a fresh recorded
experiment. Inspect other retained files with the runner's `measure <wasm>`
mode. Named analysis uses the existing
`scripts/ci/wasm-capability-size-report.sh`, `twiggy top --retained` and
`twiggy dominators`, not the full Fleet audit roster.

The runner passes all-feature example Clippy. Both feature selections and the
schema pass focused Wasm declaration-mode Clippy using the root Rust/Clippy
lint tables with warnings denied. The normal Canic artifact builder qualifies
both final runtime artifacts. `ic-wasm check-endpoints` matches each Candid
sidecar with the exact common hidden lifecycle/timer exports supplied. Both
have one init and one post-upgrade export; the only added exports are the metrics
query and metrics reset. Formatting, diff and current-document checks pass.
No PocketIC execution, instruction measurement, full validation, version bump,
commit or push is part of this experiment.

Raw compiler/final/named artifacts, Candid, feature reports, full Twiggy reports
and their checksums are retained under `target/icydb033/evidence`. Build and lint
logs are `/tmp/icydb033-{host,participant}-final-build.log`,
`/tmp/icydb033-runner-clippy.log` and `/tmp/icydb033-fixture-clippy.log`.
Those scratch locations are local and disposable; the committed-source
candidate contains the fixture, lock, report and structured evidence, not Wasm
binaries. No Git commit was made.

The measured increment combines lifecycle/schema/recovery, metrics endpoints
and shared dependency/generic compiler effects. It does not isolate the cost
of metrics alone, explain Toko's multi-entity binaries, or establish a safe
production optimization. This result is the requested stopping point before
any query/write/per-entity expansion.
