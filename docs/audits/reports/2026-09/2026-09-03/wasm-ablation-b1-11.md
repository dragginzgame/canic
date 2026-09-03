# Wasm Ablation B1-11 - Payload-Limited Async Adapter

## Verdict

- Run result: `pass`.
- Result validity: `valid`.
- Production decision: retain the raw payload-limited update adapter.

The adapter accounts for 967 optimized code-section bytes, 1,055 total Wasm
bytes, 316 gzip bytes and no replica-limited defined functions in the isolated
`payload_limit_probe`. That is a small, real footprint rather than a material
contraction opportunity. Removing it would drop the endpoint-local predecode
bound for canister-origin calls, which do not traverse inspect-message.

## Scope And Identity

- Experiment: `b1-11-payload-limited-async-adapters`.
- Immutable source: `v0.110.5` at
  `50f40171d6177c3d1e490b1fdb5f6163323b2cd5`.
- Source tree: `5a66988735c707b188d9d1fe03a3ed3b4ff7a273`.
- Artifact: Canic-owned `payload_limit_probe` fixture.
- Immediate baseline: `b1-01-current-baseline`.
- Switch: audit-only patch
  `scripts/ci/wasm-ablation-patches/b1-11-payload-limited-async-adapters.patch`.
- Switch SHA-256:
  `5807cfe5496f8b4ca4cf965475ad19777221311afd6f378ad3f2c51741bd4abd`.
- Runner SHA-256:
  `7ef5b737498a5a16571a26b3532b64b518cf1fcf4ff90275d9ef5f73e8ba1973`.
- Environment: local, offline Cargo, disabled incremental compilation, one
  fixed absolute target path removed and recreated before each repetition.

The switch retains the endpoint signature and body, payload-limit
registration, inspect-message lookup, ordinary endpoint dispatch, Candid and
exports. It replaces the generated raw update adapter with the ordinary IC CDK
adapter, making the raw size check, bounded copy, configured decode and manual
reply path unreachable. It is a destructive build-only attribution and makes
no complete payload-safety, canister-call or runtime-parity claim.

## Optimized Result

| Quantity | Baseline | Variant | Delta | Delta % |
| --- | ---: | ---: | ---: | ---: |
| release Wasm bytes | 1,987,304 | 1,986,249 | -1,055 | -0.053087% |
| release gzip bytes | 728,333 | 728,017 | -316 | -0.043387% |
| code-section bytes | 1,800,078 | 1,799,111 | -967 | -0.053720% |
| data-section bytes | 171,025 | 170,937 | -88 | -0.051454% |
| `ic-wasm` total functions | 3,541 | 3,541 | 0 | 0% |
| replica-limited defined functions | 3,504 | 3,504 | 0 | 0% |
| table minimum | 749 | 749 | 0 | 0% |
| element entries | 748 | 748 | 0 | 0% |
| Wasm export entries | 12 | 12 | 0 | 0% |
| Candid bytes | 9,225 | 9,225 | 0 | 0% |
| Candid service methods | 2 | 2 | 0 | 0% |

Before the governed optimizer, the switch removed 284 raw bytes and 284 code-
section bytes, changed gzip by -322 bytes, and changed neither data bytes nor
defined functions. The larger optimized code delta is the literal final
artifact result; no source-line proxy is used.

## Determinism And Harness Correction

Both clean baseline builds are byte-identical across Wasm, gzip and Candid and
have identical complete metric vectors. Both clean variant builds satisfy the
same checks. Baseline and variant Candid hashes are identical:
`fb5a55c930325f32d26ae91a49a6e47ebd3db4ea79290d07a21bb54d7ff6d0a9`.

A preliminary run failed closed because its two clean builds used differently
named target directories. The only Wasm difference was one embedded byte:
`baseline-a` versus `baseline-b`. That run is invalid as measurement evidence.
The corrected runner now removes and recreates the same fixed target path for
every repetition and condition; the complete governed rerun passes.

## Interpretation

The raw adapter is deliberate fault-containment, not architectural sediment.
Inspect-message protects ingress, while the raw adapter independently bounds
canister-origin update decoding. The optimized cost is below one KiB of code
and zero defined functions in the owning fixture, so B1-11 authorizes no
production deletion or consolidation. Future work should retain the behavior
unless a single replacement proves both caller classes under equivalent
predecode limits.

## Verification

| Check | Result |
| --- | --- |
| exact immutable source and lockfile | PASS |
| clean linked worktree before and after | PASS |
| authoritative `canic-host` release builder | PASS |
| two clean baseline builds | PASS |
| two clean variant builds | PASS |
| Wasm, gzip, Candid and metric determinism | PASS |
| `wasm-validate`, `gzip -t` and `didc check` | PASS |
| independent replica-limited function counter | PASS |
| exact one-switch path set | PASS |
| audit patch reversed | PASS |

## Retained Evidence

- [artifact metrics](artifacts/wasm-ablation-b1-11/artifact-metrics.tsv)
- [determinism](artifacts/wasm-ablation-b1-11/determinism.tsv)
- [run metadata](artifacts/wasm-ablation-b1-11/run-metadata.tsv)
- [evidence manifest](artifacts/wasm-ablation-b1-11/evidence-manifest.yml)
