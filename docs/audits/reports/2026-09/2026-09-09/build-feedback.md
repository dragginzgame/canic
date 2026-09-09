# Build feedback CANIC-087 and CANIC-139

The maintainer accepted both items on 2026-09-09. This extends the open
0.110.13 source batch; package versions remain 0.110.12. Toko Miner is read-only.

## Scope and sequence

1. Separate declaration compilation from runtime linking, consume canonical
   infrastructure Candid, and batch configured runtime builds by workspace with
   exact package/role protocol authority.
2. Add exact build-input and output verification for unchanged complete builds
   through the existing host artifact and release-set owners.
3. Qualify invalidation, changed-role behavior, deterministic output and build
   cost with isolated Canic-owned fixtures. Evaluate production LTO from measured
   artifact and runtime evidence; do not introduce another release profile.

The initial inspection confirms release-profile declaration links and the serial
configured runtime loop. Coordinator already reads canonical Candid; Store still
compiles declarations on every ordinary build. A complete build allocates a fresh
nonce before compilation, and every runtime embeds that release identity. Exact
unchanged-build reuse must therefore select an existing finalized identity before
allocating a new one. Changed-role composition must account for the embedded
identity; copying an artifact into a new release directory is insufficient.

## Sanity and drift verdict

CANIC-087 identifies real redundant work. Safe batching needs a stronger
constraint than different package names: Cargo feature unification must preserve
each role's isolated dependency tree. The rejected first measurement demonstrates
why that admission is required. Declaration optimization does not justify
changing production runtime optimization without its separate size/behavior proof.

CANIC-139's unchanged complete-build path fits the existing authority model.
Its changed-role composition request does not: a successor release cannot contain
runtime bytes that attest another release ID. This batch preserves that contract
and does not create a second artifact identity, deployment owner or release mode.

## Implemented contract

- Declaration builds keep the selected profile/config/features/network but force
  optimization level zero, LTO explicitly off and 16 codegen units in an
  independent target. A real Cargo fixture confirms release cfg assertions
  stay disabled and declaration/runtime final outputs remain separate. They omit the release
  nonce so a new runtime identity does not invalidate declarations.
- The ordinary Coordinator and Store paths use canonical Candid. Configured
  packages compile in batches with one bounded, canonical package/role digest
  map. Batch admission compares Cargo's combined resolved dependency trees with
  each package's isolated tree and splits any dependency-feature change. Existing
  finalization and export/feature/Candid checks remain.
- Complete builds check the cache before allocating a release nonce. A hit
  verifies every retained output plus the finalized manifest hierarchy and
  returns the existing release identity without compilation or finalization.
- Inputs include complete Cargo package trees, explicit target files (including
  shared build scripts), Cargo-recorded external dependency paths, config/lock
  files, ambient environment, native tool bytes and Rust host/Wasm libraries and
  bundled tools. Omitted optional targets in published packages are fingerprinted
  as absent. Unknown input syntax or tool authority declines reuse.
- Every role reports its hit/miss. Timings distinguish declarations, runtime
  Cargo/link, Candid extraction, shrink, Candid embedding, Binaryen and
  gzip/validation. Runtime
  Cargo/link is an aggregate, not an isolated LLVM LTO measurement.

The current npm `ic-wasm` launcher is recognized by SHA-256
`ff4f9bd1d3734f7aa69078ecd4c5716dbfaf67d094c69d5082b00bac5b8bf936`
and resolved to its exact native payload before admission. The launcher was
verified against the integrity-checked
[`@icp-sdk/ic-wasm` 0.11.1 archive](https://registry.npmjs.org/@icp-sdk/ic-wasm/-/ic-wasm-0.11.1.tgz).
Unknown scripted tools remain usable for ordinary builds but cannot establish
exact reuse.

## Scope limits

CANIC-139's requested cross-role composition is not implemented. Every runtime
still embeds the complete release ID; changed inputs therefore allocate a new
identity and rebuild the runtime set. Reusing old bytes in that successor would
violate existing authority. Input collection is conservative over the complete
Cargo catalog rather than a precise per-role closure. Build scripts must declare
external inputs to Cargo, and a changed/discovered input set during compilation
cannot establish a reusable build. This work does not claim that all original
CANIC-139 completion criteria are closed.

Production LTO remains the maintained fat-LTO release policy with Binaryen 132.
The older upstream Binaryen 108 reference is historical, not the current tool
authority. No new release profile, release identity protocol or deployment owner
is introduced. Toko Miner's live/no-op adoption and downstream test results are
not inferred from Canic fixture evidence.

## Qualification

The initial 101 selected regressions passed across `canic`, `canic-core`,
`canic-host` and `canic-cli`. Subsequent host/CLI checks cover cache input and
output changes, exact npm payload selection and final completion rendering.
The final cache regressions include external build scripts/includes and omitted
registry examples; all six pass in `/tmp/canic87-139-cache-inputs3.log`.
Affected-package warning-denied all-target/all-feature Clippy passes in
`/tmp/canic87-139-clippy10.log`. Earlier macro/core Clippy retains its scope.

[Structured build measurements](build-feedback.json) retain tool hashes and the
eight-artifact fixture results. The installed 0.110.12 builder and candidate use
one frozen 3,768-file source snapshot, separate empty Cargo targets and disabled
compiler wrappers. The source SHA-256 inventory has zero mismatches. The baseline
took 794.55s, including 132.31s Coordinator, 105.71s Store and 556.09s configured
roles. Other repositories were compiling on the same host; the OS file cache was
not flushed. These are shared-machine observations, not isolated performance
guarantees. GNU time reports maximum child RSS, not aggregate concurrent RSS.
The first candidate attempt was stopped after cache preflight exposed omitted
registry example/test files. Its partial target and timing are excluded; the
corrected candidate starts from a different empty target.

That candidate completed in 547.88s and repeated with eight verified hits in
2.76s, preserving all 29 retained files. All eight Candid and protocol-profile
digests matched the baseline, but raw application Wasms grew by 27–320KB because
one workspace-wide Cargo batch unified the roles' dependency features. This is
rejected qualification evidence, not the accepted performance result. The fix
now admits a batch only when Cargo preserves each package's complete isolated
normal/build dependency tree. The next complete cold build took 800.15s with four admitted groups and
162.52s declaration compilation; its unchanged replay took 2.90s and preserved
all 29 retained files. All eight Candid/protocol digests match the baseline.
Raw sizes match except two eight-byte differences. This fixes the large feature
drift but does not establish a cold speedup. Distinct release identities mean
these runs do not establish byte determinism across clean builds.

The final declaration candidate explicitly sets LTO to `off` and optimization
level zero. [Cargo's profile contract](https://doc.rust-lang.org/cargo/reference/profiles.html#lto)
distinguishes `off` from `false`, which permits local Thin LTO. Thirty-one selected host
build tests pass, including real Cargo feature grouping and profile cfg/output
isolation; host/CLI warning-denied Clippy passes in
`/tmp/canic87-139-clippy13.log`. A tested attempt to share host intermediates
across profiles did not reuse the dependency and was removed.

The final complete candidate took 787.38s (baseline 794.55s), with
134.60s declaration compilation, 4.66s Candid extraction and 376.69s configured
runtime compilation. This is not a demonstrated material cold-build improvement
on this shared host. Maximum child RSS was 2,582,108 KiB versus 2,440,256 KiB.
All eight Candid and protocol digests match the baseline; raw sizes match except
two eight-byte differences. Its unchanged replay took 2.83s, returned the same
release ID and preserved all 29 output files byte-for-byte. Wrapper/cache deltas
are not claimed because compiler wrappers were explicitly disabled.

A separate cold fat-LTO Root build took 210.93s. Its Candid matches the
complete candidate, but Wasm/gzip differ despite the same release ID: both
artifacts contain their distinct target-directory paths to generated
`canic.compiled.rs`. Cargo target paths are fingerprinted reuse inputs. This
cross-target probe is not a fixed-input byte-determinism proof, and this batch
does not introduce path remapping.

The matching cold Thin-LTO Root build (16 codegen units) took 124.07s,
compared with 210.93s for fat LTO. Both pass final artifact admission and their
Candid matches. Thin increased final code from 7,128,048 to 7,812,428 bytes
(+9.6%) and gzip from 2,682,707 to 2,860,880 bytes (+6.6%). The maintained
production default stays fat LTO: this single-Root speed result trades away
footprint and does not provide the complete runtime-instruction and determinism
qualification required for a switch.

The one-role edit inserts `core::hint::black_box(1_u8)` into the empty setup
body of the owned snapshot's `apps/test/app/src/lib.rs`. With each builder's
warm target, the installed baseline took 477.10s and the candidate took 291.03s
(39% less wall time). Candidate declarations took 7.15s, Candid extraction 4.33s
and configured runtime compilation 140.31s. All eight Candid/protocol digests
match the edited baseline. The candidate emits eight misses, seals a new complete
release and replays it with eight hits in 3.09s, preserving all 29 output files.
Restoring the source returns the original complete release in 2.79s. All 3,768
snapshot files have been restored and verified. This measures conservative
complete-release rebuilding after an edit, not per-role artifact composition.

The selected mixed-topology PocketIC case built and admitted the complete Fast
artifact set, completed fresh infrastructure and full readiness, checked cycle
conservation and passed effect-free terminal replay assertions. It then entered
two additional same-release reset/recovery exercises. After the maintainer asked
about elapsed time, the extra portion was deliberately interrupted (exit 143);
the governed runner removed its scratch and stopped its owned server. This is
partial runtime evidence, not a full case pass. The smaller managed/standalone
lifecycle and same-release upgrade case passes: 147.23s test time, 267s total
runner time, with its server and scratch cleaned up. Its exact case is
`pic::lifecycle::tests::published_managed_app_support_drives_composed_lifecycle`;
log: `/tmp/canic87-139-lifecycle-pocketic1.log`. Earlier recovery and operator
feedback evidence retains its completed scope.

## Readiness and remaining upstream criteria

The scoped in-repository implementation, measured qualification and documentation
are complete. The combined accepted source batch and open 0.110.13 changelog are
ready for push/release review; package versions remain 0.110.12. Targeted formatting
and current-document semantics checks pass. No full workspace gate was run.

Neither original upstream checklist is fully closed. CANIC-087 still lacks a
demonstrated material cold-build improvement and a full fixed-input clean-build
determinism proof. CANIC-139 still lacks precise per-role invalidation and
composition across different embedded release IDs. Toko Miner adoption and its
downstream qualification remain separate work in the read-only sibling.
No broad validation, Git publication, version transaction, deployment or
downstream mutation is authorized by this task.
