# CANIC-050/051 Retained-Install Recovery Evidence

Status: open 0.109.9 candidate; final targeted and governed proof passes. The
earlier complete maintainer gate predates the final durability correction and
must be rerun before publication.

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

## Negative boundary

The correction does not authorize a fresh Fleet, replacement or reinstalled
Root, manual journal edit, extra pool asset, additional funding, general
cross-release upgrade, version-pair allowlist or post-completion reopening.
Pre-infrastructure phases, phase regression, changed authority/artifact/
topology/funding fields, unrelated live modules and premature terminal
publication fail closed.

## Local evidence

Passing focused checks recorded during implementation:

```text
cargo check -p canic-host --lib
cargo check -p canic-host --all-targets
cargo check -p canic-cli --all-targets
cargo test -p canic-host fleet_install_recovery_bundle --lib
cargo test -p canic-host fleet_subnet_root_repair --lib
cargo test -p canic-cli deploy --lib
```

The focused host proof covers typed phase bounds, phase regression, premature
terminal publication, immutable authority changes, content-addressed artifact
replay after caller-path deletion, terminal replay, phase-derived bundle
completeness and bundle tamper/incomplete/mixed/path/schema/import conflicts.

The governed production-boundary test now starts from the canonical sequence-15
checkpoint, upgrades the same live Root, funds and re-inspects the same pool
asset through PocketIC's Cycles Ledger, replays the already-live successor
without another debit, restarts between the production Store, Registry join/
sync/activation/mirror and Component Registry phase drivers, re-observes the
protected advanced Component Registry, publishes the terminal receipt only at
sequence 28, installs a Component, publishes the Fleet catalog and closes the
session. The final replay retains one funding attempt and an unchanged operator
balance; ordinary canister execution may consume a bounded amount from the
still-sufficient pool asset. It passed through the targeted repository runner
in 20 seconds; the shared PocketIC high-water mark was 353,556 kB and 20
threads.

## Complete validation evidence pending refresh

The unmodified maintainer command passed on the prior source candidate:

```text
make validate

PASS    49s  check
PASS    69s  clippy
PASS  1540s  test
```

The test inventory contained 39 targets: 30 ordinary targets ran before nine
serial PocketIC targets. The ordered internal PocketIC suite passed in 684
seconds; the retained-Root repair journey passed in 137 seconds; the runtime,
blob-storage, payload-limit and instruction-audit PocketIC suites also passed.
The shared PocketIC server reported a 4,950,116 kB high-water mark and 257
threads. Formatting, layering/invariant guards, dependency risk, secret scan,
ShellCheck, role/feature compilation, release-integrity documentation and
warning-denied workspace Clippy all passed in that same command. That evidence
predates the final write-through checkpoint and phase-derived bundle
correction, so release governance requires the maintainer to rerun it on the
final immutable candidate. No remote IC operation was run.

Versioning, immutable publication and the downstream exact-session resume are
maintainer/downstream-owned and remain pending.
