# Wasm Footprint Audit - 2026-07-14

## Verdict

- Run result: `blocked`.
- Result validity: `invalid` under the post-freeze method-defect protocol.
- Comparability: `non-comparable: first frozen CANIC-WASM-001/v1 attempt
  produced no artifact metrics`.
- Authoritative risk score: `blocked`.

The canonical runner fails on the first `app` artifact. Frozen v1 defines
direct Cargo Wasm as its required pre-processing baseline and executes
`cargo build --target wasm32-unknown-unknown`; the published product correctly
rejects that obsolete path at `crates/canic/src/build_support/config.rs:25` and
requires `canic build <fleet> <role>`. No raw, shrunk, gzip, debug, structure,
or `twiggy` measurement was produced, so no Wasm growth or improvement can be
claimed.

The runner's composite fingerprint is also repository-root dependent. The
frozen relative-path composite is `e8c58213...`, while the required linked
worktree would emit `8c8e2248...` because absolute path text enters the hash.
The obsolete artifact model and invalid executable identity are recorded as
P1 `CANIC-092-AUDIT-016`. No product or frozen-method file was changed.

## Report Preamble

- Scope: default attached role roster from `fleets/test/canic.toml`: `app`,
  `user_hub`, `user_shard`, `scale_hub`, `scale_replica`, and `root`.
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`.
- Compared baseline report path: `N/A`; first run of the UTC day.
- Code snapshot: `v0.92.0` /
  `91736337fc1cfeb891f17d7d62affb5e671348e2`; source tree
  `fd31bb8289365a38f2bea7f8ebd6973908ee959f`; product tree
  `c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e`.
- Method: `CANIC-WASM-001/v1`, definition fingerprint
  `1ed32dd340d10135e899cda5794046d68e1e66ea89da9d6910aa4ca4e958a064`;
  expected executable composite
  `e8c58213d9301d66d4adac4bd92e4aa702fd887b8adb55e2e602a70f29e9c505`.
- Linked-worktree executable composite:
  `8c8e2248cf96ce00e3b4eccceeb542e5eed92db6e40a815779517dfb3aa1caf5`.
- Auditor: Codex.
- Run timestamp: `2026-07-14T17:54:43Z` to `2026-07-14T17:55:48Z`.
- Branch: detached `v0.92.0` linked worktree.
- Worktree: clean before and after the failed execution.
- Profile: `release`, with required `wasm-debug` comparison planned but not
  reached.
- Execution environment: isolated `CARGO_TARGET_DIR` in clean linked worktree
  `/tmp/canic-wasm-audit-092`; local environment only.

## Run Identity

```text
release_anchor: v0.92.0
source_commit_full: 91736337fc1cfeb891f17d7d62affb5e671348e2
source_tree_hash: fd31bb8289365a38f2bea7f8ebd6973908ee959f
product_tree_hash: c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e
clean_worktree: true; disposable detached worktree remained clean
cargo_lock_hash: 6cd75f146077bbf3f254fda608f1265531d1065ce0cd9c1bb56d67118f3de5cc
rust_toolchain: rustc 1.97.0; cargo 1.97.0
target_triple: wasm32-unknown-unknown release attempt
feature_set: default fleets/test attached role roster
audit_method_id: CANIC-WASM-001
audit_method_version: 1
audit_method_fingerprint: 1ed32dd340d10135e899cda5794046d68e1e66ea89da9d6910aa4ca4e958a064
expected_executable_composite: e8c58213d9301d66d4adac4bd92e4aa702fd887b8adb55e2e602a70f29e9c505
runner_executable_composite: 8c8e2248cf96ce00e3b4eccceeb542e5eed92db6e40a815779517dfb3aa1caf5
external_tool_versions: icp 1.0.2; ic-wasm 0.9.11; twiggy-opt 0.8.0
fixture_or_seed: fleets/test/canic.toml attached six-role roster
environment_class: isolated local linked-worktree execution trace
started_at: 2026-07-14T17:54:43Z
completed_at: 2026-07-14T17:55:48Z
```

## Execution Trace

The exact preferred command was run from a clean detached linked worktree:

```text
bash scripts/ci/wasm-audit-report.sh
```

The runner:

1. passed its linked-worktree, clean-state, local-environment, Cargo, and ICP
   prerequisites;
2. resolved the default attached role roster;
3. created an isolated temporary Cargo target;
4. entered `ensure_raw_canister app` at
   `scripts/ci/wasm-audit-report.sh:310-337`;
5. called direct Cargo Wasm compilation; and
6. failed with exit 101 when `canister_app` enforced the authoritative Canic
   build boundary.

Because `set -e` exits before report finalization, the runner retained no size
artifact or generated evidence manifest. The linked worktree remained clean.

## Artifact Size Matrix

No byte measurement is available. `root` remains classified as a
`bundle-canister`; it is not compared with leaf roles.

| Canister | Kind | Built raw | Shrunk raw | Built gzip | Shrunk gzip | Debug raw | Delta |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| `app` | leaf | N/A | N/A | N/A | N/A | N/A | blocked at build |
| `user_hub` | leaf | N/A | N/A | N/A | N/A | N/A | not reached |
| `user_shard` | leaf | N/A | N/A | N/A | N/A | N/A | not reached |
| `scale_hub` | leaf | N/A | N/A | N/A | N/A | N/A | not reached |
| `scale_replica` | leaf | N/A | N/A | N/A | N/A | N/A | not reached |
| `root` | bundle | N/A | N/A | N/A | N/A | N/A | not reached |

There is no attached explicitly minimal baseline role. V1 correctly forbids
treating missing baseline evidence as a size signal; repeated leaf hotspots
also cannot be inferred without artifacts.

## Debug/Profile Comparison

`BLOCKED`. The release build fails before `capture_debug_artifacts`; no
release-versus-`wasm-debug` comparison exists.

## Shrink and Structure Evidence

- `icp 1.0.2`, `ic-wasm 0.9.11`, and `twiggy-opt 0.8.0` are available.
- The host `build_artifact` path, deterministic gzip, `ic-wasm info`, and all
  `twiggy` commands were not reached.
- No function count, export count, data-section size, shrink delta, or gzip
  continuity value is retained.
- No Candid documentation-attribute growth claim is made because shared data
  section growth was not observed.

## Structural Hotspots

No measured retained-size hotspot table can be produced. Static candidates
from the independent complexity baseline are not substituted for `twiggy`
evidence. In particular, the root control plane and delegated-auth modules may
be source hubs, but that does not prove their retained Wasm contribution.

| Required attribution | Result | Reason |
| --- | --- | --- |
| artifact outliers by canister | blocked | no artifact sizes |
| retained-size hotspot | blocked | `twiggy` not reached |
| root child-bundle contribution | blocked | root not built |
| leaf/baseline comparison | blocked | no minimal role and no artifacts |
| debug/release comparison | blocked | debug capture not reached |

## Dependency Fan-In Pressure

`BLOCKED` for retained-size attribution. Shared `canic-core`, Candid/DTO,
auth/crypto, logging/metrics, and lifecycle/macro code are known fan-in
candidates, but no current `twiggy` or section evidence quantifies their cost.

## Early Warning Signals

- The measurement artifact model resurrects a direct build surface that the
  product intentionally hard-cut.
- The runner cannot reach the supported `build_artifact` call because it first
  requires the rejected direct Cargo baseline.
- Executable identity changes with the linked-worktree path, so two otherwise
  identical runs do not share the frozen composite.
- Missing raw/shrunk/debug evidence prevents detection of leaf spread, root
  bundle growth, shrink collapse, generic monomorphization, or function-count
  drift.

## Risk Score

Authoritative score: `blocked`.

Missing required size evidence is not scored as stable zero-byte drift. No
release artifact risk value can be derived until the method is executable.
The independently confirmed root reproducibility defect remains owned by
`CANIC-092-BUILD-001`; this blocked size run neither clears nor duplicates it.

## Finding

### P1 - frozen Wasm method depends on a removed build path

Canonical finding: `CANIC-092-AUDIT-016` (`audit_method_defect`, P1,
confirmed, open).

Evidence:

- The method contract declares direct Cargo output a required primary built
  artifact.
- `ensure_raw_canister` and `ensure_debug_raw_canister` execute direct Cargo
  Wasm builds before the supported host artifact builder.
- The current build contract rejects that path deterministically.
- Absolute filenames enter the executable composite, producing
  `8c8e2248...` instead of frozen `e8c58213...` in the required linked
  worktree.

Required correction:

1. Preserve this blocked result and do not add a compatibility bypass to the
   product build guard.
2. Define supported pre/post-transform artifacts inside the authoritative
   `canic build`/host builder, or remove the unsupported direct-Cargo metric.
3. Route release and debug comparison through the current authoritative build
   contract with explicit provenance and transform identity.
4. Make the executable composite independent of repository root.
5. Version/fingerprint v2 and rerun immutable `v0.92.0` in a clean linked
   worktree.

## Required Checklist

| Requirement | Result | Evidence |
| --- | --- | --- |
| all target artifacts built/loaded | `FAIL` | first `app` build rejected |
| machine-readable sizes | `BLOCKED` | no artifact generated |
| `twiggy top` | `BLOCKED` | build did not reach analysis |
| `twiggy dominators` | `BLOCKED` | build did not reach analysis |
| `twiggy monos` | `BLOCKED` | build did not reach analysis |
| baseline selection | `PASS` | first run; `N/A` |
| `wasm-debug` artifacts | `BLOCKED` | release build failed first |
| debug/profile deltas | `BLOCKED` | no artifacts |
| per-canister current sizes | `BLOCKED` | no artifacts |
| baseline deltas | `BLOCKED` | first invalid run |
| verification readout | `PASS` | this table records normalized outcomes |
| Candid doc-comment regression check | `N/A` | no shared data-section growth under review |

## Verification Readout

| Command/check | Result | Notes |
| --- | --- | --- |
| clean linked worktree | `PASS` | detached exact `v0.92.0` |
| local environment guard | `PASS` | no IC deployment target |
| tool prerequisites | `PASS` | Cargo, ICP, `ic-wasm`, and `twiggy` available |
| definition SHA-256 | `PASS` | `1ed32dd3...` |
| relative executable composite | `PASS` | equals frozen `e8c58213...` |
| runner executable composite | `FAIL` | root-dependent `8c8e2248...` |
| `cargo build --target wasm32-unknown-unknown -p canister_app --release --locked` | `FAIL` | authoritative guard rejected direct build |
| supported host artifact build | `BLOCKED` | not reached |
| raw/shrunk/debug metrics | `BLOCKED` | no artifact generated |
| `ic-wasm` / `twiggy` analysis | `BLOCKED` | no input artifact |
| tracked source mutation | `PASS` | linked worktree clean after failure |
| canonical runner | `FAIL` | exit 101 |

## Retained Evidence

The compact evidence manifest for this failed execution is retained at
[evidence-manifest.yml](artifacts/wasm-footprint/evidence-manifest.yml).

No product or frozen-method change was made during this run.
