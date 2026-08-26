# CANIC-050/051/053/054/055 Retained-Install Recovery Evidence

Status: CANIC-050/051 are published in immutable 0.109.9. The 0.109.10 source
batch closes CANIC-053/054 and adds the CANIC-055 no-effect installer
preflight; focused proof passes while the frozen-candidate downstream and
human-owned release gates remain.

## Failure

The former `root-repair-receipt.json` performed two incompatible jobs. It
authorized one exact successor Root module, but it could be compiled and
validated only after the local Root journal had reached
`component_registry_preparation_verified`. An exact post-infrastructure
checkpoint such as `store_bootstrapped` therefore continued to require the
predecessor module, while supplying terminal evidence immediately required
request/response fields that only the later canonical workflow could record.

The repair input also retained only artifact hashes and sizes. A later retry
could re-read caller-owned build paths, so loss of a disposable source checkout
could make exact artifact reuse unavailable even though all runtime authority
remained intact.

## Maintained correction

- `root-repair-authority.json` is provisional. It is accepted only for the
  typed `StoreBootstrapped` through
  `ComponentRegistryPreparationVerified` phase variants and binds session and
  journal schema, Fleet, plan, release build, install operation, Root, Store,
  placement, controller, expected/live predecessor and successor raw-Wasm
  hashes/sizes, all Candid hashes, pool and exact funding-policy digest.
- Provisional authority changes only the module hash accepted by ordinary
  infrastructure verification. It does not call Component Registry proof and
  cannot attest Store, Registry, pool, catalog or completion state.
- Store bootstrap, Registry join/sync/mirror and Component Registry
  preparation retain their original journals and deterministic operation IDs.
  They re-observe monotonic live state and record only their normal
  transitions.
- `root-repair-terminal-receipt.json` is terminal. It binds the provisional-authority
  digest, exact terminal journal digest and exact protected Component Registry
  request/response digests. It can be published only at
  `ComponentRegistryPreparationVerified`; only a durable terminal receipt can
  move the repair operation from `AssetReady` to `Adopted`.
- Exact repair Wasm and Candid bytes are copied before repair effects to
  content-addressed files beside the retained operation. Retry validates and
  uses that Canic-owned successor path.
- The historical predecessor's exact adjacent `.did` is mandatory and must
  match the manifest-bound Root journal Candid digest. The successor resolves
  its exact adjacent sidecar first; extraction is only a build-time fallback
  when that Wasm exports `get_candid_pointer`. Missing, symlinked, oversized,
  invalid or digest-mismatched sidecars reject before authority or effects.
- `root-repair-candidate.json` is retained evidence, not operational authority.
  It binds the exact transition while both Wasms and sidecars are checkpointed
  into a verified bundle. Only after that no-effect checkpoint may the same
  bytes be published as `root-repair-authority.json`; a second checkpoint then
  precedes the first upgrade, top-up or protected reinspection effect.
- `canic install ... --preflight` accepts only an existing incomplete session.
  It runs the production preparation path through finalized Coordinator/Root/
  Store artifact selection, existing journal inspection, exact predecessor and
  successor sidecars, candidate retention and the same verified bundle
  checkpoint, then returns before pre-root receipts, operational repair
  authority or any IC update. It performs read-only ICP observations and local
  candidate/bundle writes and is therefore deliberately not described as a
  generic dry-run.
- A separate schema-v1 recovery bundle checkpoints all exact Canic-owned
  install state, finalized artifacts and receipts before the first remote
  effect and after each governed phase. Every repair intent/observation is
  checkpointed immediately after its durable transition and before the
  upgrade, top-up or protected reinspection it authorizes. Its manifest is
  bound to network, Fleet, App, plan, release build and install operation.
  Root checkpoint entries bind the exact typed phase and derive its required
  journal, creation/install sidecars, repair authority/operation/receipt and
  retained Wasm/Candid objects. Every logical path is confined below `.canic`;
  each object is addressed and checked by SHA-256.
- The 0.109.10 completeness extension requires the exact finalized
  infrastructure manifest and application artifact union, derives every
  referenced raw Wasm, gzip Wasm and deterministic Candid sidecar, and binds
  their sizes and digests before import. The application union remains bound
  to the Fleet plan; every retained Root journal remains bound to the exact
  infrastructure manifest plus Root and Store entries.

## Negative boundary

The correction does not authorize a fresh Fleet, replacement or reinstalled
Root, manual journal edit, extra pool asset, additional funding, general
cross-release upgrade, version-pair allowlist or post-completion reopening.
Pre-infrastructure phases, phase regression, changed authority/artifact/
topology/funding fields, unrelated live modules and premature terminal
publication fail closed.

## Development-loop correction

Downstream measured the focused repair proof at 0.12 seconds, bundle proof at
0.68 seconds, the production-boundary PocketIC journey at 20.58 seconds, cold
Rust compilation at roughly two minutes per newly built target and the
39-target complete Canic gate at roughly 12 minutes. The preventable delay was
not recovery execution itself: `deploy plan` never entered the installer-only
Candid/candidate/bundle path, so the defect appeared only after publication.

The maintained loop is now:

1. keep one repository `target/` and use the pinned `sccache` when available;
2. run focused unit checks while editing;
3. run the one targeted retained-repair PocketIC journey before review;
4. freeze one immutable source candidate and run its exact downstream
   `canic install ... --preflight` before versioning; and
5. run the complete human-owned gate once on that unchanged candidate, then
   publish only if both gates identify it.

B9 owns extraction of dependency-light pure recovery-plan validation from the
heavy IC/PocketIC drivers. After this Fleet reaches terminal installation,
scheduled 0.111 remains the owner of the exact managed transition; CANIC-055
does not broaden arbitrary historical-version compatibility.

## Local evidence

Passing focused checks recorded during implementation:

```text
cargo check -p canic-host --lib
cargo check -p canic-host --all-targets
cargo check -p canic-cli --all-targets
cargo test -p canic-host fleet_install_recovery_bundle --lib
cargo test -p canic-host fleet_subnet_root_repair --lib
cargo test -p canic-host effect_equivalent_install_preflight --lib
cargo test -p canic-cli install --lib
cargo test -p canic-cli deploy --lib
```

The focused host proof covers typed phase bounds, phase regression, premature
terminal publication, immutable authority changes, content-addressed artifact
replay after caller-path deletion, terminal replay, phase-derived bundle
completeness and bundle tamper/incomplete/mixed/path/schema/import conflicts.
CANIC-054 adds direct coverage for no-export historical and successor Wasms,
exact sidecar resolution, the bounded build-output fallback and sidecar
missing/unsafe/oversized/invalid rejection. The ordinary focused repair,
bundle and CLI-help tests pass. The governed extended journey passes after the
CANIC-055 preflight change in 67 seconds with a 363,552 kB/20-thread PocketIC
high-water mark.

The governed production-boundary test starts from the canonical sequence-15
checkpoint with a historical Root that lacks `get_candid_pointer`, upgrades
the same live Root, funds and re-inspects the same pool
asset through PocketIC's Cycles Ledger, replays the already-live successor
without another debit, restarts between the production Store, Registry join/
sync/activation/mirror and Component Registry phase drivers, re-observes the
protected advanced Component Registry, publishes the terminal receipt only at
sequence 28, installs a Component, publishes the Fleet catalog and closes the
session. The final replay retains one funding attempt and an unchanged operator
balance; ordinary canister execution may consume a bounded amount from the
still-sufficient pool asset. It passed through the targeted repository runner
in 20 seconds; the shared PocketIC high-water mark was 353,556 kB and 20
threads in the published 0.109.9 proof. The 0.109.10 extension additionally
checkpoints before authority, closes the session, removes the original source
workspace and verifies/imports the terminal bundle; its governed result is
also exercised by the complete candidate gate's retained Root-repair row.

An earlier 0.109.10 complete-gate run ran every ordinary and PocketIC test
successfully, including the extended retained-repair journey, and then
reported only warning-denied `large_enum_variant` findings in the test-only
retained Root stub. The fixture response variants are now boxed and pass their
focused warning-denied Clippy and package-test checks. CANIC-055 changes the
candidate after that evidence; its exact targeted PocketIC journey now passes,
while the frozen-source downstream preflight and normal release gate own its
remaining validation.

A later cold host rebuild also exposed the 100-line warning limit in the
top-level install entrypoint. The correction groups the already-prepared
preflight/apply values into one named execution request and helper without
moving phase, receipt or effect ownership. Focused warning-denied host Clippy
and the install-order invariant test pass before the final complete gate.

## Published 0.109.9 validation and publication evidence

The unmodified maintainer command passed on final source
`37be941419c823cefa491539f3b50ee063571068`:

```text
make validate

PASS    1s  check
PASS    1s  clippy
PASS  429s  test
```

The complete ordinary and serial PocketIC graph, formatting,
layering/invariant guards, dependency risk, secret scan, ShellCheck,
role/feature compilation, release-integrity documentation and warning-denied
workspace Clippy passed in that same command. Annotated tag `v0.109.9` and all
five matching published packages identify governed release commit
`e949712969b1e9a7de875c51811a20c69b35f6ab`. A separate immutable-tag run of
the retained-repair journey reached Component installation, catalog
publication, permanent session closure and effect-free terminal replay in 72
seconds with one funding attempt. No audit-time remote IC operation was run.
