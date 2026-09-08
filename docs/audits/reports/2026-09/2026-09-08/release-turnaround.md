# Release Turnaround Correction

Date: 2026-09-08. Baseline: maintainer-pushed `v0.110.10`. This work extends the
open 0.110.11 batch alongside CANIC-148; package versions remain 0.110.10.

## Outcome

The maintainer reported 67m20.980s for `make release-patch && make publish`.
The command runs the complete validation gate once, then publishes seven crates.
Publication does not repeat PocketIC. The preceding successful gate's raw log
was deleted, so its exact stage breakdown cannot be reconstructed.

The implemented correction removes publication's Rust test compilation for
manifest inspection, eliminates repeated exact registry observations, and retains
successful and failed stage logs automatically. The combined batch is ready for
release approval. These changes do not establish a new full-release duration.

## Changes

The release-integrity gate and publication share
`scripts/ci/check-publish-manifest-boundary.sh`. It reads locked offline Cargo
metadata and checks local normal/build dependencies against unpublished workspace
members. Cargo resolves workspace inheritance, dependency renames, optional flags
and target-specific tables. The check includes packages with default publication
permission and excludes development-only dependencies and unrelated registry
packages with the same name. Its focused fixtures replace the deleted Rust
manifest-boundary test and its parsing helpers. The remaining Rust manifest
tests continue running in the ordinary integration tier.

Publication records exact package/version observations in an invocation-local
set. A package already found on crates.io, or observed after upload, does not
need a second final lookup. A `PUBLISH_FROM` invocation still checks every
unobserved predecessor before declaring the package set complete. Observations
are never retained as authority for another invocation. Dependency order,
publication failure handling, dry-run behavior and package verification remain.

Validation writes raw per-target logs and `timings.tsv` beneath unique
`target/validation-runs/` directories. Existing latest-failure logs remain
independent and are not overwritten by success. Publication writes per-stage
logs and timings beneath `target/publication-runs/`, separating preflight,
registry lookup, package verification/upload and propagation. Directories are
printed before work starts and remain after completion or interruption. Nested
validation barriers retain separate directories; an interrupted target may have
a partial log without a completed timing row. These are diagnostic records,
not validation receipts. Normal release commands retain them; explicit Cargo
cleanup removes them.

## Evidence

| Check | Result |
| --- | --- |
| Shared guard on the current workspace | PASS, 0.06s wall time; no compilation |
| Manifest fixtures | PASS: renames, workspace inheritance, optional and target-specific edges, default publication, dev-only allowance and unrelated registry dependency |
| Publication fixtures | PASS: preflight stops before Cargo, failed upload stops later packages, retry skips published packages, incomplete predecessor rejection, complete resumed set and dry-run |
| Validation runner fixtures | PASS: mixed failure collection, raw passing logs, retained TSV results, complete-success retention and unchanged latest-failure evidence |
| Remaining `canic` workspace-manifest tests | 6 passed |
| `canic` Clippy, all targets/all features, warnings denied | PASS |
| Changed scripts, governed ShellCheck exclusions | PASS |
| Release-integrity contract guard | PASS |

With immediate propagation, a fresh seven-package publication now performs
14 registry probes instead of 21. Propagation retries can add probes. The
already-published retry fixture proves one lookup per package, reducing 14
probes to seven. These are operation counts, not measured registry wall-time
savings. The removed Rust preflight previously could compile after the release
version bump; no precise before/after cold-compile saving is claimed.

Logs are `/tmp/canic-release-speed-metadata.log`,
`/tmp/canic-release-speed-script-tests.log`,
`/tmp/canic-release-speed-manifest.log`,
`/tmp/canic-release-speed-clippy.log`, and
`/tmp/canic-release-speed-integrity.log`.

## Remaining Large Cost

Eight Fleet journeys remain serial. They cover different provisioning, funding,
reinstall, response-loss and conservation boundaries. Their current shared
`PIC_UNIT_TEST_SERIAL` lock and `canic-121-production-adapter` scratch path mean
raising the test thread count would either serialize again or introduce shared
fixture mutations. No concurrency switch was enabled and no case was deleted.

The next measured experiment should isolate fixture paths and mutable owners,
prepare shared immutable artifacts once, and compare two independent cases
serially versus with bounded concurrency. Existing timing evidence does not yet
qualify that change. The new retained normal-release logs will identify whether
Fleet waiting, fixture builds, native compilation or package verification is the
largest remaining contributor in the maintainer's environment.

No broad test gate, package upload, version transaction, Git publication,
deployment or sibling mutation was performed. CANIC-148's passing runtime
evidence remains applicable; this correction changes scripts, a removed ordinary
test and documentation only.
