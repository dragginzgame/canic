# Infrastructure build scheduling — CANIC-087 follow-up

Date: 2026-09-09. Candidate: open 0.110.14 batch, based on published
`v0.110.13` (`5250d69186047bc373db7cdc7b7680c3aad031d0`).

## Scope

Complete App builds now capture Coordinator and Store Cargo outputs before the
next compilation starts. Their existing qualification/finalization pipelines
run in scoped workers while subsequent Cargo work proceeds. Cargo remains
serial under one workspace artifact-build lock. At most two infrastructure
finalizers are added; configured-role finalization retains its existing bounds.
Single-role builds remain synchronous. The CLI seals release manifests only
after every requested artifact returns successfully.

This changes scheduling only. Runtime profiles, LTO, features, protocol hashes,
release identity, Candid/export validation, Binaryen admission, deterministic
gzip and code-size limits retain their existing owners and settings.

## Controlled comparison

The temporary native driver calls the current host's existing single-role
builder for Coordinator then Store in `serial` mode, and its new App builder
with no configured roles in `pipeline` mode. Both paths use the same candidate
code and finalizer; the comparison isolates scheduling rather than comparing
different runtime versions. The existing generated mixed-topology config and
one fixed release-build ID are shared by all runs.

One unmeasured serial pass warms a private Cargo target. Before each measured
pass, only the two generated wrapper source mtimes are advanced, forcing their
runtime relinks without changing any source bytes. No Cargo profile, optimizer
or cache policy is changed. Compiler wrapping is disabled; Cargo is offline.
The machine is not isolated and OS caches are not flushed. GNU time's maximum
RSS is the largest child-process measurement, not total concurrent RSS.

The measured serial pass took **112.893s**, compared with **85.032s** for the
pipeline: **27.861s (24.68%) less wall time**. All seven retained files (two
Wasm/gzip/Candid sets and the Store profile marker) have identical hashes.
Protocol and transform records also match exactly. The 1,507 recorded source
inputs and generated wrapper bytes remained unchanged throughout qualification.

| Phase | Serial | Pipeline |
| --- | ---: | ---: |
| Coordinator Cargo/link | 48.10s | 46.79s |
| Coordinator finalization | 35.58s | 37.37s |
| Store Cargo/link | 22.36s | 25.87s |
| Store finalization | 5.09s | 4.97s |
| Total wall, including admission/capture | 112.893s | 85.032s |
| Maximum child RSS | 1,664,492 KiB | 1,669,212 KiB |

Concurrent phases overlap; their durations are not additive. Store compilation
and Coordinator finalization individually slowed in this sample, while overlap
reduced total wall time. The cold cache warm-up (224.395s) is not an A/B result.
Rust is 1.98.1, ic-wasm 0.11.1 and Binaryen 132, using the unchanged release
profile. [Structured evidence](build-pipeline.json) retains exact source/tool
hashes, artifact hashes, driver source, commands, phase observations and limits.

## Failure and artifact evidence

Captured Cargo bytes survive source replacement and deletion. Capture failure
removes its staging directory and preserves existing published bytes. Existing
artifact tests cover optimizer/export/Candid rejection and preservation of prior
outputs on failed qualification. The real driver also requests an absent
configured role after both infrastructure compilations: the operation returns
an error only after its workers finish, leaves no captured staging directories,
and a following valid request with the same release-build identity reproduces
the artifacts. Failure handling took 38.728s; the valid retry took 34.271s.

All 53 focused host tests and 25 CLI build tests pass. Warning-denied Clippy
passes for both affected packages, including every target and feature. Commands:

```sh
RUSTC_WRAPPER= ICP_ENVIRONMENT=local CARGO_NET_OFFLINE=true cargo test --locked -p canic-host --lib --all-features -- artifact_io::tests canister_build:: bootstrap_coordinator::tests bootstrap_store::tests
RUSTC_WRAPPER= ICP_ENVIRONMENT=local CARGO_NET_OFFLINE=true cargo test --locked -p canic-cli --lib --all-features -- build::tests
RUSTC_WRAPPER= ICP_ENVIRONMENT=local CARGO_NET_OFFLINE=true cargo clippy --locked -p canic-host -p canic-cli --all-targets --all-features --keep-going -- -D warnings
```

Logs: `/tmp/canic114-pipeline-tests1.log`, `/tmp/canic114-cli-tests.log` and
`/tmp/canic114-pipeline-clippy2.log`. Captured-input and finalization rejection
cases assert file state; no IC management behavior is simulated.

## Limits and feedback disposition

This is a bounded two-infrastructure scheduling comparison, not a complete
Toko Miner cold/warm benchmark. It does not establish total concurrent peak
memory or downstream deployment behavior. No runtime source or compiler
setting changes; byte equality is the direct runtime drift check. No broad
workspace or PocketIC suite is claimed.

CANIC-087 remains partial pending its outstanding full-build qualification.
CANIC-139's existing exact whole-build reuse is unchanged. Changed inputs still
require runtimes carrying the new complete release identity; cross-identity
per-role reuse is not introduced. CANIC-141 remains deferred, and sibling
repositories remain read-only.
