# First-build reuse evidence — CANIC-139

Date: 2026-09-09. BF3 extends the existing open 0.110.14 draft after BF2.
Package versions remain 0.110.13. The initial diagnosis inspected siblings
read-only; the subsequently requested downstream qualification uses an isolated
Toko Miner source copy and does not change its dependency pins or installed CLI.

## Finding

Toko Miner's published-0.110.13 build compiled eight artifacts, then refused
cache recording with `build inputs changed during compilation`. Its no-edit
retry succeeded in 102.67s; the following complete reuse took 4.39s. The
historical log does not retain the before/after input inventory, so it cannot
identify every path responsible for that exact failure.

Read-only inspection of the current App Cargo catalog and aggregate runtime
and declaration dependency records finds 143 control-plane source files outside
the initial package/configuration roots. Generated infrastructure enables these
sources even though the App's ordinary Cargo graph omits that optional package.
The old fingerprint therefore depended on whether Cargo had already recorded
them. It also conflated replacement of stale dependency records with changes
to the source bytes those records named. Both mechanisms have focused
regressions; the historical per-path difference remains unavailable.

Inspection command:

```sh
cargo metadata --manifest-path ../toko-miner/Cargo.toml --locked --offline --format-version 1
```

The compared dependency records are the top-level `.d` files in the runtime
and declaration `wasm32-unknown-unknown/fast` directories. Generated and Cargo
output paths are excluded by the existing input-owner rules.

## Correction and boundaries

- The initial snapshot includes the exact Canic family roots selected by the
  existing generated-package patch resolver. Manifest names and versions retain
  their existing checks; there is no second package-selection list.
- File-level evidence and non-file identity remain available after compilation.
  If refreshed Cargo records stop naming an input, its bytes are rechecked;
  dropped directory observations are rescanned for additions and mutations.
- An admitted record uses the final verified inventory's digest, so replacing
  stale records does not force the next invocation to look under an obsolete key.
- Actual file mutations, missing files, new files within observed directories,
  changed configuration/environment identity and unsupported file types still
  refuse recording. Newly observed external inputs also refuse: the build has
  no evidence of their earlier bytes. Changed and unobserved inputs now have
  distinct typed errors carrying the affected path.

Cache schema, finalized-release verification, output hashes and release-build
identity remain with their existing owners. No runtime, compiler-profile,
optimizer, feature-selection or deployment change is introduced. This does not
implement per-role reuse across changed complete-release identities.

## Verification

The tiny Cargo fixture puts an optional control-plane package outside the App
configuration tree. It proves that the default App catalog omits that package,
then performs a real Wasm build enabling it. Initial and final fingerprints
match without preexisting dependency records. Replacing a stale external-source
record also succeeds after checking its bytes, and the refreshed fingerprint
matches the next lookup. Editing a control-plane source then fails with the
typed changed-input error.

Snapshot regressions cover dropped-record source edits and deletion, additions
beneath a dropped directory, symlink replacement, new external inputs and
non-file identity changes. Existing source/configuration/lock/tool/network,
output-corruption, package-version, feature-isolation and artifact-qualification
checks remain in the focused test selection.

```sh
RUSTC_WRAPPER= ICP_ENVIRONMENT=local CARGO_NET_OFFLINE=true cargo test --locked -p canic-host --lib --all-features -- artifact_io::tests canister_build:: bootstrap_coordinator::tests bootstrap_store::tests fleet_package::tests
RUSTC_WRAPPER= ICP_ENVIRONMENT=local CARGO_NET_OFFLINE=true cargo clippy --locked -p canic-host -p canic-cli --all-targets --all-features --keep-going -- -D warnings
```

All 58 host cases pass (33.61s compilation, 7.57s execution). Host/CLI Clippy
passes in 7.41s. The earlier 25 CLI build cases and controlled scheduling
comparison remain [BF2 evidence](build-pipeline.md); they are not a new complete
Toko Miner benchmark of this follow-up. No broad gate or PocketIC run is claimed.
The subsequent controlled downstream rerun is recorded below.

| Retained log | SHA-256 |
| --- | --- |
| `/tmp/toko-canic-011013-bindings.log` | `53c40e8c7e3cd30446bbb8bf6c8cbc4ce59ec2e0095f3caea1cb849a7f278f9a` |
| `/tmp/toko-canic-011013-bindings-retry.log` | `a2b0c0b284c4bbcf5b935f797bf614cab7bd169679222031a9a55d37c51dba5b` |
| `/tmp/toko-canic-011013-reuse.log` | `981112d7fa09862213cab0b2223e15d4e773219674cc83ae433f9550287a3cde` |
| `/tmp/canic139-first-build-final-tests.log` | `28aa588817db2d2cb54e32a9f82f032b59aaef47617b6b45fe539624aba52e4d` |
| `/tmp/canic139-first-build-clippy2.log` | `5f2eb1f234db7b3cc14c49b8fa267f8f56241af5d9330f98a264b61317927f2f` |

## Controlled Toko Miner first build

The maintainer subsequently requested downstream qualification. An isolated
source copy at `/tmp/toko-feedback-first-build` retained Toko Miner's exact
Canic 0.110.13 and IcyDB 0.257.3 pins. It began without `target/`, reuse records
or generated infrastructure. The candidate CLI was built from the BF2/BF3 source
in this checkout; the installed CLI and original Toko Miner target were unchanged.

Both invocations used:

```sh
RUSTC_WRAPPER= CARGO_NET_OFFLINE=true /tmp/canic139-candidate --environment toko_miner build toko_miner --profile fast
```

The first invocation completed all eight artifacts in **355.98 seconds**. The
next invocation verified and reused all eight in **3.40 seconds**, retaining
release build `43e2cf9b9258fea38f9c10d000621fe28a3f179a0baa2f72ddacd70c22e24b4f`.
This is a cold-first-build/reuse pair, not a controlled old/new speed comparison.

Independent before/after source inventories contain the same **26,768 entries**.
Cargo added **14 aggregate dependency records**. Their 143 control-plane source
paths outside ordinary App package/configuration roots were all present in the
initial inventory. The successful first invocation also crossed the production
host's complete input comparison and finalized-release verification.

The independent inventory records package/configuration source bytes and raw
Cargo records; it is not a dump of the host's private environment/tool digest.
That distinction prevents treating this audit script as a second cache owner.
The historical failed invocation still lacks its original inventory.

[Structured evidence](build-reuse-downstream.json) retains commands, source
inventory and log digests, candidate identity and the 143 source bindings. Full
inventories remain at `/tmp/canic139-toko-before.json` and
`/tmp/canic139-toko-after.json`; the script is `/tmp/canic139-inventory.py`.
This qualifies the candidate fix against a frozen downstream source, not its
publication or downstream adoption, and does not establish per-role changed-input
reuse or application runtime correctness.
