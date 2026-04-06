# Changelog

All notable, and occasionally less notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).


## [0.26.x] - 2026-04-06 - Metrics Baseline

- `0.26.3` makes delegated-auth config fail fast when the build is under-provisioned: root now traps immediately if delegated auth is configured without `auth-crypto`, signer canisters trap if they are built without threshold-ECDSA support, and verifier-only canisters still stay verifier-only.
- `0.26.2` keeps the first `0.26` runtime follow-through on the clean side by simplifying root replay/cycles routing, tightening delegation and verifier-cache paths, and lowering the retained instruction hotspots to `root::canic_response_capability_v1 = 489511` and `root::canic_request_delegation = 1682331` in the latest same-day rerun.
- `0.26.1` restores the supported public `ICRC-21` dispatcher facade at `canic::api::protocol::icrc21::Icrc21Dispatcher`, so downstream canisters no longer need hidden `canic-core` paths after the earlier facade narrowing.
- `0.26.0` establishes the first `0.26` metrics and performance baseline, refreshing the retained wasm and instruction audit reports so the next runtime work can measure drift against a clear starting point instead of the late `0.25` cleanup line.

See detailed breakdown:
[docs/changelog/0.26.md](docs/changelog/0.26.md)

---

## [0.25.x] - 2026-04-05 - Recurring Audit Refresh

- `0.25.11` moves `canic_metrics` off the internal-test build gate and onto a real `canic` `metrics` feature that is enabled by default, so ordinary facade users keep the metrics endpoint by default while still being able to opt out explicitly with Cargo features.
- `0.25.10` cleans up the public `canic-memory` facade by renaming the stable-memory bootstrap and lookup methods toward intent and by hiding the runtime summary type from the public return values, so downstreams use a smaller `MemoryApi` surface instead of substrate-shaped names.
- `0.25.9` extends `canic-memory` with small read-only registration queries, so downstreams can inspect registered memory ids by owner or label through the supported `MemoryApi` facade instead of reading registry/runtime snapshots directly.
- `0.25.8` adds a small read-only `canic-memory` inspection helper so downstreams can ask who owns one memory id, what reserved range it belongs to, and whether that slot already has a registered label, without reaching into registry/runtime internals.
- `0.25.7` adds a supported dynamic-memory API to `canic-memory`, so downstream crates can reserve ranges, register runtime-selected memory IDs, and open `VirtualMemory` handles without importing the hidden `MEMORY_MANAGER` internals directly, while also hardening shared `canic-testkit` PocketIC baseline recovery and continuing the `canic-testkit::pic` cleanup without changing downstream call sites.
- `0.25.6` adds the new recurring `module-structure` audit and uses its first retained pass to tighten structural visibility: `canic-core` now hides more support-only root modules, `canic-memory` no longer root-re-exports backend bootstrap state, and `canic-testkit::pic` is split by ownership so the public PocketIC seam is cleaner without changing downstream call sites.
- `0.25.5` keeps the `0.25` follow-through on the clean side by trimming more shared runtime weight from the default demo surface, removing leftover `wasm_store` carryover endpoints, centralizing the internal test/audit wasm-build path, and landing two small measured runtime cuts that lower sampled `root::canic_request_delegation` from `1768507` to `1726014` local instructions across the retained reruns.
- `0.25.4` finishes the internal canister-boundary cleanup by splitting correctness fixtures from audit probes, moving the `audit_*_probe` crates into a dedicated `audit-canisters` lane, and tightening the default instruction audit so it measures shared runtime and audit-only probe paths instead of demo `create_*` provisioning flows.
- `0.25.3` continues the post-audit runtime trim by cutting more avoidable work out of the delegated-auth and replay paths, including replay payload compaction, cheaper delegation cert hashing, a thinner root signing/cache path for `canic_request_delegation`, and compact cached cycles responses that cut sampled `canic_response_capability_v1` `cycles-request` from `1481137` to `601860` local instructions in the next retained audit rerun.
- `0.25.2` starts the runtime follow-through from the `0.25.0` audit sweep by tightening delegated-auth proof provisioning, threading shard key material through the root install path so verifier setup stops repeating avoidable key lookup work, and trimming repeated proof-install payload encoding in the `canic_request_delegation` hot path while keeping the auth/runtime checks green.
- `0.25.1` follows the audit sweep by splitting the auth/runtime complexity hotspots into smaller modules, moving the `test` role out of the default demo topology into internal test-only canisters, removing root debug helpers so the demo/reference canisters stay closer to real user-facing flows, and making public `canic-testkit` PocketIC setup more ergonomic with fallible startup/install helpers plus temp-root lock-parent creation for repo-local `TMPDIR` paths.
- `0.25.0` refreshes the recurring audit line with retained summary reruns across layering, capability surface, wasm footprint, instruction footprint, lifecycle/change-friction checks, and the auth invariants; the current result is that the invariants still hold while the main remaining pressure is complexity concentrated in the auth/runtime seams.

See detailed breakdown:
[docs/changelog/0.25.md](docs/changelog/0.25.md)

---

## [0.24.x] - 2026-04-04 - Shared Runtime Reduction and Test Boundary Cleanup

- `0.24.8` extends public `canic-testkit` with a generic prebuilt-wasm install path, so downstream PocketIC suites that do not use Canic canisters can still stay fully `canic-testkit`-backed instead of hand-rolling `create_canister` / `add_cycles` / `install_canister` adapters.
- `0.24.7` hardens the `pic_role_attestation` PocketIC suite by rebuilding dead cached baselines automatically after failed restore attempts and by aligning the role-attestation capability tests with the real `signer -> root` cycles caller path instead of the old `root -> root` shortcut.
- `0.24.6` makes `canic-testkit` more useful for downstreams by promoting the generic standalone non-root PocketIC fixture and PocketIC `install_code` retry helpers into the public crate, while keeping Canic-specific root, attestation, and delegation fixtures internal.
- `0.24.5` finishes another test-boundary cleanup pass by moving the local bogus-token auth guard onto the standalone PocketIC lane, sharing the internal `user_hub -> user_shard -> root delegation` fixture plumbing across auth-focused suites, and giving the reconcile tests their own named cached root profile so the remaining root hierarchy entrypoints are explicit instead of generic.
- `0.24.4` keeps hierarchy-heavy testing focused on the cases that really need `root` by moving standalone `app`, `test`, and `scale_hub` checks onto a shared internal PocketIC fixture, keeps heavy internal env/directory queries out of ordinary canister builds behind a test-only flag, and hardens the local tooling path by auto-recovering local `dfx` once and letting the wasm audit build artifacts without depending on a healthy replica first.
- `0.24.3` folds sharding back into `canic-core`, removes the standalone `canic-sharding-runtime` crate and the extra `xxhash-rust` dependency, keeps the `canic` `sharding` feature stable for facade users, switches HRW scoring to `sha2`, and narrows the internal root harness around explicit topology, scaling, and sharding profiles so hierarchy-heavy suites only pay for the roles they actually exercise.
- `0.24.2` follows the first `0.24` auth reductions by reusing cached root response attestations, carrying cycles authorization through replay/capability execution, trimming replay and registry work, and clarifying that query lanes are measured through same-call probe endpoints because query-side perf rows do not persist, while the next dated rerun cuts sampled `root::canic_request_delegation` from `3205866` in `instruction-footprint-20` to `2274445` in the `2026-04-05` instruction audit.
- `0.24.1` follows up the first `0.24` perf pass by warming root auth key material during setup, removing the redundant root-to-signer delegation proof push, and collapsing the root verifier cache path into one auth-state write, which cuts sampled `root::canic_request_delegation` from `4356980` in `instruction-footprint-17` to `3205866` in `instruction-footprint-20`.
- `0.24.0` continues the shared-runtime reduction line by trimming shipped `CandidType` doc bloat, separating the public `canic-testkit` surface from unpublished self-test support, cutting sampled root chunk publication from about `9.7M` to `390k` local instructions, cutting sampled `root::canic_request_delegation` from `5516827` in `instruction-footprint-15` to `4356980` in `instruction-footprint-17`, and hardening the audit and release surfaces around those reductions.

See detailed breakdown:
[docs/changelog/0.24.md](docs/changelog/0.24.md)

---

## [0.23.x] - 2026-04-03 - Deferred Follow-Through

- `0.23.2` removes the checked-in wasm budget layer from the recurring footprint audit, so follow-through work is driven by dated size deltas and hotspot evidence instead of static thresholds.
- `0.23.1` follows up the new parent-to-child cycles test helper with a small `scale` canister cleanup so the `request_cycles_from_parent` endpoint stays warning-free under `make clippy`.
- `0.23.0` starts the follow-through line with checked-in wasm budgets, a dated wasm-footprint rerun, a clearer split between the public `canic-testkit` PocketIC wrapper and the new unpublished `canic-testing-internal` self-test crate, a removal of the unused `*cycles_accept` compatibility endpoint so management-canister cycle deposit stays the only Canic-managed funding path, and a fix for the curlable setup script so its default `canic-installer` version stays aligned with the current Canic release.

See detailed breakdown:
[docs/changelog/0.23.md](docs/changelog/0.23.md)

---

## [0.22.x] - 2026-04-02 - Audits, Wasm Size, and Perf

- `0.22.10` fixes the narrowed local root-install build path so it issues one quiet `dfx build <canister>` call per selected target, matches the real DFX CLI contract, keeps the one-time Canic build context stable across the whole install, restores downstream `make test-canisters` flows after the `0.22.9` targeted-build change, adds a curlable `scripts/install.sh` setup path that bootstraps Rust when needed and installs the pinned Rust/Cargo/Canic toolchain plus `dfx` in one step, and removes the stale duplicate environment-update path so setup docs point at one shared flow.
- `0.22.9` tightens the local thin-root install path by fabricating cycles only when local root is actually short, building only `root` plus the configured release roles from the root-owning subnet, keeping the normal wait loop quieter, and removing the now-redundant DFX dependency edges from the reference `dfx.json`.
- `0.22.8` cleans up the repo-local/downstream output so both the shell wrapper and direct `canic-build-canister-artifact` calls print the workspace/DFX roots once per run, show the selected `debug|fast|release` build profile, add visible spacing between canister builds, log per-canister elapsed time with `0.01s` precision, and render the installer’s end-of-run timing summary as a readable table.
- `0.22.7` lets the installer auto-discover nested canister manifests from Cargo workspace metadata so downstreams no longer need flat alias directories just to match Canic role names.
- `0.22.6` improves local install diagnostics by exposing a typed `canic_bootstrap_status` query, lets the installer fail immediately on root bootstrap errors with phase-aware output and an end-of-install timing summary instead of waiting only on `canic_ready`, fixes the public visible-canister build path so it applies the same `ic-wasm shrink` pass as the hidden bootstrap `wasm_store` builder, and removes committed visible canister `.did` files so generated `.dfx/local/canisters/*/*.did` outputs are the only live source of truth apart from the canonical checked-in `crates/canic-wasm-store/wasm_store.did`.
- `0.22.5` continues the downstream `wasm_store` instruction-limit follow-through by removing a redundant init-time managed-store catalog import after publication, so root no longer snapshots the just-retired rollover store again before bootstrap can finish.
- `0.22.4` continues the downstream `wasm_store` instruction-limit follow-through by removing the managed-store chunk-store preflight during install-source resolution, so root no longer asks a freshly published store to enumerate its whole chunk-hash set again before `install_chunked_code`.
- `0.22.3` finishes the downstream `wasm_store` instruction-limit follow-through by replacing repeated full-store occupied-byte rescans with incremental counters, so each new chunk upload no longer re-serializes every already-stored chunk just to enforce capacity.
- `0.22.2` continues the `wasm_store` publication follow-through by streaming release chunks through the live root/store publication path instead of buffering full releases in memory and switching staged-release payload verification to incremental hashing, further reducing the cost of large downstream bootstrap publication.
- `0.22.1` follows up the audit/perf line by caching the expensive debug small-store reconcile baseline, adding a compact workspace timing summary table, recording the first dated `0.22` instruction-footprint report, hardening the wasm audit runner so missing local `dfx` fails fast, keeping `make publish` viable with the one intentional local `canic-core -> canic-testkit` test-only edge, and trimming managed `wasm_store` publication hot paths so large downstream release sets stop hitting instruction limits during bootstrap.
- `0.22.0` opens the audit/perf line by making `.dfx` artifact reuse aware of build env and profile, moving more reusable PocketIC root-baseline setup into `canic-testkit`, standardizing three wasm build lanes (`debug`, `fast`, `release`) across repo-local and downstream builders, and routing the special small-store reconcile build through the shared root harness so future audit work starts from reproducible inputs instead of stale artifact reuse.

See detailed breakdown:
[docs/changelog/0.22.md](docs/changelog/0.22.md)

---

## [0.21.x] - 2026-04-01 - Implicit Wasm Store and Managed Release Fleet

- `0.21.12` fixes the release lane so `make publish` can resume after partial crates.io uploads, skips already-published workspace crates instead of aborting at the first duplicate, keeps workspace manifest inheritance intact, and unblocks `canic-core` publish preparation by using a targeted `--no-verify` publish exception for its test-only `canic-testkit` edge.
- `0.21.11` stops the local installer from overriding caller-selected build profiles, keeps repo-local smoke installs on the optimized dev wasm path by default, hardcodes Canic wasm staging/install chunks to the IC-safe `1_048_576` bytes with no env or config override surface, adds visible installer plus root-side staging progress, moves reusable root PocketIC baseline setup into `canic-testkit`, front-loads root artifact builds once per workspace test run, and makes the normal `make test` path run with `--nocapture` plus explicit per-suite timings so long PocketIC phases stay visible live.
- `0.21.10` teaches the public `canic-installer` tools to separate Cargo/config discovery from DFX artifact output, so split repos like `backend/` + `frontend/` can keep one real repo-root `.dfx` while pointing Canic at a nested Rust workspace through `CANIC_WORKSPACE_ROOT` and `CANIC_DFX_ROOT`, and the repo-local `make demo-install` / `make test-canisters` smoke path now defaults to optimized dev wasm instead of slower release canister builds.
- `0.21.9` finishes productizing the downstream build/install boundary by publishing `canic-build-canister-artifact` and `canic-install-root`, shrinking the repo-local build/install scripts into thin wrappers, and adding an installed-binary `canic-installer` probe so downstream projects can rely on public Canic tools instead of copying more shell logic.
- `0.21.8` finishes the thin-root cleanup by moving GitHub Actions onto the shared Canic wasm build helper, preferring the public installer binaries in the repo-local wrappers, and publishing the hidden bootstrap `wasm_store` build behind `canic-build-wasm-store-artifact` so downstreams no longer need to re-own that shell logic.
- `0.21.7` hardens the new `canic-installer` path by fixing its false ready-timeout on successful thin-root installs, adding direct coverage for the accepted `canic_ready` JSON shapes, rejecting bad `.wasm.gz` release artifacts before any root staging work begins, opportunistically emitting `root.release-set.json` from the public installer path during normal custom builds, and proving the packaged installer can emit a downstream manifest from normalized package contents.
- `0.21.6` publishes `canic-installer` as the downstream thin-root installer surface, moves the manifest/staging binaries off workspace-private `canic-internal`, and hardens `root.release-set.json` so it only stages roles from the single subnet that actually owns `root`.
- `0.21.4` keeps `root.wasm` thin again by embedding only the bootstrap `wasm_store`, moving ordinary release staging back out to a manifest-driven Rust installer flow in `canic-internal`, removing the hidden `wasm_store` leak from downstream `dfx.json`, and restoring a manual `scripts/app/dfx_start.sh` convenience script without reintroducing auto-started `dfx` into the normal test or install gates.
- `0.21.3` hardens the managed `wasm_store` fleet again by adding root-facing live publication and retired-store status reads, proving the fixed-target and retire/finalize/delete flows under PocketIC, and making lifecycle-boundary tests resilient to PocketIC install throttling instead of failing on transient rate limits.
- `0.21.2` hardens the managed `wasm_store` fleet follow-through by clarifying the root-owned approved-state overview surface and adding PocketIC runtime proofs that exact releases are reused while conflicting duplicate `template_id@version` publications fail closed without mutating fleet state.
- `0.21.1` hardens the first managed-fleet release by scoping and pruning stale approved roles to the current config-driven release set, keeping the implicit `wasm_store` preset downstream-safe without const-only assumptions, tightening the root-owned overview semantics so its headroom flag is clearly approved-state-only, and removing the local `dfx` smoke path from `make test` / `make test-bump` so the normal test gate stays PocketIC/Cargo-driven while manual `dfx` installs still fail fast if the replica is not already running.
- `0.21.0` starts the new managed release-fleet line: `root` now owns the implicit `wasm_store` bootstrap, embeds the build-produced `.wasm.gz` bootstrap and ordinary release artifacts, manages a tracked multi-store fleet with exact-release reuse and post-upgrade reconcile, and lets downstreams build through `canic` without carrying a local `wasm_store` crate or a manual bootstrap script.

```bash
cargo install --locked canic-installer --version <same-version-as-canic>
dfx build --all
canic-install-root root
```

See detailed breakdown:
[docs/changelog/0.21.md](docs/changelog/0.21.md)

---

## [0.20.x] - 2026-03-31 - Cleanup and Optimization

- `0.20.10` turns root publication into a real `wasm_store` fleet manager: it now places releases from the full approved manifest set across the tracked store inventory, reuses exact existing releases instead of duplicating them, creates fresh stores proactively when no current store can accept a release, and stops assuming the current release set lives in one default store.
- `0.20.10` also hardens the fleet follow-through: root post-upgrade now reconciles approved manifests against the exact current release bytes instead of conflicting on older copies in older stores, the root store overview now clearly reports approved-release projections instead of pretending to know live occupancy, ordinary embedded release bundles are gzip-only, and the hidden `wasm_store` build path can synthesize its own wrapper so downstreams do not need to carry extra `wasm_store` config or source.
- `0.20.9` makes root publication multi-store aware by retrying individual releases on a newly promoted `wasm_store` when the current one runs out of capacity, and keeps later installs aligned by importing the catalog from the active publication store instead of assuming the configured default binding always won.
- `0.20.8` publishes the canonical `canic-wasm-store` crate so downstreams can stop carrying a local `wasm_store` canister crate, switches the embedded ordinary root release bundle to `.wasm.gz` payloads, and lets root roll publication across additional `wasm_store` canisters when one store cannot fit the whole bootstrap release set.
- `0.20.6` hardens the embedded `wasm_store` bootstrap contract by rejecting empty or non-wasm `.wasm.gz` artifacts during the root build itself, and expands the bootstrap provenance log to include both the original DFX source path and the copied embedded path so downstream artifact bugs fail early and read clearly.
- `0.20.5` fixes the embedded `wasm_store` bootstrap source so `root` now installs the current DFX-built `.wasm.gz` artifact instead of drifting back to a stale checked-in payload, and logs the exact embedded bootstrap provenance during root init so bootstrap mismatches are visible immediately.
- `0.20.4` makes ordinary child-role publication an internal root bootstrap detail by embedding the release bundle into `root` during the normal `dfx build --all` flow, so reinstalling `root` is sufficient again in local deployments and the old external release-staging scripts are gone.
- `0.20.3` stabilizes the `0.20` perf tooling by turning the instruction audit into a real repeated baseline instead of a one-off harness, adding production `perf!` checkpoints across the critical root/auth/replay/scaling/sharding flows, measuring root template-staging admin updates directly, and hardening the audit/build path so unrelated local `dfx` and Cargo state no longer invalidate the report runner.
- `0.20.2` makes `wasm_store` an internal root bootstrap detail instead of a user-managed reference canister, removes the old `shard` / `shard_hub` reference roles, consolidates the sharding demo and test lane on `user_hub` / `user_shard`, hardens root release staging so stale local `.dfx` artifacts cannot silently republish deleted roles, adds a generic host-side root bootstrap helper that downstream Canic projects can point at their own `canic.toml` and `.dfx` artifacts, and surfaces the staged `template_id@version` through staging, publication, and install logs so operators can see exactly which release root selected.
- `0.20.0` opens the cleanup and optimization line, using recurring wasm-footprint and instruction-footprint audits to drive shared wasm reduction, lower `perf!` and endpoint instruction counts, catch regressions before they spread across the runtime floor, keep publishable crates free of workspace-only integration-test baggage, and round out the `canic` control-plane facade so downstreams can keep dropping direct `canic-control-plane` imports.

See detailed breakdown:
[docs/changelog/0.20.md](docs/changelog/0.20.md)

---

## [0.19.x] - 2026-03-30 - Library Lane Cleanup and Crate Graph Simplification

- `0.19.6` cleans up stale automation by removing the unused `make release` / `check-versioning` paths and obsolete bootstrap helper scripts, fixes CI’s old `template_store` canister list to the current `wasm_store` topology, and adds a recurring instruction-footprint audit definition for `perf!` and endpoint instruction regression tracking.
- `0.19.5` rounds out the downstream facade story by adding a feature-gated `sharding` lane on `canic`, so sharding coordinator canisters can keep using `canic::api::canister::placement::ShardingApi` and `start!()` without depending on `canic-sharding-runtime` directly, while `root` and `wasm_store` continue to use the existing `control-plane` feature.
- `0.19.3` restores a feature-gated `canic` control-plane lane so downstream `root` and `wasm_store` crates can keep using the facade-owned root lifecycle and template/store API paths without making ordinary leaf canisters pull control-plane code by default.
- `0.19.2` simplifies the workspace crate graph by merging the temporary template helper crates into `canic-control-plane`, deleting the dead `canic-dsl` and `canic-utils` crates, and restoring an empty shared `SubnetState` so the generic state cascade shape is `[as ss ad sd]` again without reintroducing root-owned publication inventory into non-root sync.
- `0.19.1` finishes the library/reference split by moving template/store and sharding implementation lanes out of the default `canic` path, compiling `canic.toml` into the canister instead of parsing TOML at runtime, collapsing the temporary template helper crates back into `canic-control-plane`, removing the dead `canic::dsl` / `canic-utils` crates, standardizing debug-only Candid export on `canic::cdk::export_candid_debug!()`, and hardening the staged `wasm_store`/`root` reference install flow behind `make demo-install` once `dfx` is already running.
- `0.19.0` starts the `0.19` line with a clean post-`0.18` audit baseline, recording the release wasm footprint (`minimal`/`app`/`scale`/`shard` at `2489858` bytes, `root` at `3730865`, `wasm_store` at `2823075`) and the refreshed capability-surface baseline before the next reduction pass begins.

```toml
canic = { version = "0.19.5", features = ["control-plane", "sharding"] }
```

See detailed breakdown:
[docs/changelog/0.19.md](docs/changelog/0.19.md)

---

## [0.18.x] - 2026-03-27 - Template Store and Chunked Install Cutover

- `0.18.7` stops stale non-root canisters from spamming root with failed attestation-key refreshes after they fall out of the subnet registry, fixes cached `.did` invalidation so per-canister release builds stop retriggering whole-workspace rebuilds during `dfx build --all`, and compacts shared capability-proof wire payloads behind `CapabilityProofBlob` so non-root interfaces carry less proof-shape fan-out.
- `0.18.6` removes the remaining env-driven eager-init build split, keeps release builds single-pass while caching `.did` files independently of release wasm, stages the full config-defined release set into `root` before local smoke/bootstrap flows continue, adds root-owned bootstrap debug visibility with human-readable wasm sizes, and fixes the local smoke path so it calls the `test` canister that `root` actually created and registered.
- `0.18.5` keeps `ICRC-21` behind role-scoped compile-time gating, trims the shared generated surface by making `canic_app_state` and `canic_subnet_state` root-only, removes embedded release payloads from both `root` and `wasm_store`, and hardens bundle builds so profile-mismatched `.dfx/local` artifacts are no longer silently reused when the AA pipeline stages releases through `root`.
- `0.18.4` gives `root` a single controller-facing `canic_wasm_store_overview` read endpoint built entirely from root-owned state so operators can inspect all tracked wasm stores without direct store queries, consolidates the older split wasm-store status queries into that overview surface, and tightens the local release flow so `make patch` / `make minor` skip PocketIC-heavy tests, rely on an already-running `dfx`, and stop failing plain Cargo/clippy builds when `.dfx` release artifacts have not been generated yet.
- `0.18.3` makes `root` bootstrap its first `wasm_store` automatically again, updates the `canic-memory` eager-init contract so `canic::start!` consumes it seamlessly without extra user wiring, and hardens local `dfx` test flows by starting clean replicas and removing the now-stale manual bootstrap staging step from `make test` and `make patch`.
- `0.18.2` makes the `root` and `wasm_store` release flow fully config-driven from `canic.toml`, moves live wasm-store inventory into runtime subnet state so `root` can create and promote stores dynamically instead of relying on static bindings, and standardizes debug-only Candid export behind `canic::cdk::export_candid!()`.
- `0.18.1` completes the staged `wasm_store` bootstrap follow-up by fixing local `dfx` installs to stage the bootstrap payload before root becomes ready, restoring local compact-config compatibility, and trimming release-only exports so the raw `root` artifact drops further to `3554964` bytes.
- `0.18.0` starts the wasm-store cutover by moving ordinary child payload ownership out of `root`, requiring store-backed chunked install for every role except bootstrap `wasm_store`, reducing the raw release `root` artifact to `4151294` bytes (`delta -10366542` vs `0.17.3`), simplifying setup with one implicit per-subnet `wasm_store` on a fixed 40 MB / 4 MB IC preset, and refreshing the workspace baseline to Rust `1.94.1` with `ctor 0.8` and `sha2 0.11`.

```toml
[subnets.prime]
auto_create = ["app", "user_hub", "scale_hub", "shard_hub"]

[subnets.prime.canisters.app]
kind = "singleton"
```

See detailed breakdown:
[docs/changelog/0.18.md](docs/changelog/0.18.md)

---

## [0.17.x] - 2026-03-25 - Wasm Audit and Endpoint Surface Reduction

- `0.17.3` continues the wasm audit line by tightening `canic_metrics` and `canic_log`, completing the `0.17` root decomposition handoff to `0.18`, and reducing the `minimal` raw release artifact to `2433930` bytes (`delta -26446` vs `0.17.2`).
- `0.17.2` continues the wasm audit line by slimming shared runtime, metrics, and observability paths, bringing the `minimal` raw release artifact down to `2460376` bytes (`delta -100624` vs `0.17.1`) while keeping the intended operator-facing feature set intact.
- `0.17.1` cuts the shared wasm floor again by separating root-only capability verification from the non-root cycles path and by removing the old Canic standards canister-status endpoint, bringing the `minimal` raw release artifact down to `2561000` bytes while keeping the intended runtime feature set intact.
- `0.17.0` starts the wasm audit line with a measured per-canister footprint baseline, renames the canonical baseline canister from `blank` to `minimal`, and trims optional scaling, sharding, delegated-auth, and `ICRC-21` endpoint exports behind compile-time config so disabled features stop inflating every build.

See detailed breakdown:
[docs/changelog/0.17.md](docs/changelog/0.17.md)

---

## [0.16.x] - 2026-03-16 - Delegation Proof Evolution

- `0.16.2` hardens delegated-auth token handling by rejecting malformed or unusable lifetimes at both issuance and verification, making the zero-skew policy explicit, restoring ops-owned proof boundaries, and closing the `0.16` auth/proof line with remaining root/template architecture work handed off to `0.17` and `0.18`.
- `0.16.1` hardens delegated-auth audience binding so verifier proof installs and delegated-session bootstrap reject out-of-scope audiences, while typed auth rollout metrics make prewarm/repair failures easier to track during the `0.16` auth refactor.
- `0.16.0` is reserved as a placeholder minor-line entry for delegation proof evolution follow-up work (deferred from `0.15` Phase 3), with implementation details tracked in the `0.16` design docs.

See detailed breakdown:
[docs/changelog/0.16.md](docs/changelog/0.16.md)

---

## [0.15.x] - 2026-03-12 - Unified Auth Identity Foundation

- `0.15.6` bumps `pocket-ic` to `13.0`, refreshes supporting IC/Rust dependencies, and advances the workspace to `0.15.6` so local and integration tooling stay aligned with the current dependency baseline.
- `0.15.5` fixes CI flakiness in delegation/role-attestation integration builds by making cfg-gated test-material compilation reliably rebuild when `CANIC_TEST_DELEGATION_MATERIAL` changes between runs.
- `0.15.4` completes Tier 1 delegation provisioning guarantees by requiring required verifier fanout success at issuance, adding root-side verifier-target validation and role-labeled provisioning metrics, and validating issuance -> verifier verify -> bootstrap -> authenticated guard success end to end; Phase 3 follow-ups are explicitly deferred to the `0.16` design track.
- `0.15.3` removes unused legacy compatibility shims/fallbacks and records a follow-up `layer-violations` rerun (`3/10`, no hard layer violations).
- `0.15.2` fixes shard token issuance regression by routing non-root delegation requests to root over RPC, so shard-initiated proof refresh works again while root-only authorization stays enforced.
- `0.15.1` finalizes 0.15 release governance docs by recording explicit security sign-off scope/residual risks, freezing the auth-semantic boundary for 0.15, and clarifying canonical release-boundary tracking.
- `0.15.0` hardens delegated-caller behavior into token-gated delegated-session semantics with strict subject binding, TTL clamp, replay/session-binding controls, and auth observability, while keeping raw-caller infrastructure predicates unchanged.

```rust
DelegationApi::set_delegated_session_subject(delegated_subject, bootstrap_token, Some(300))?;
```

See detailed breakdown:
[docs/changelog/0.15.md](docs/changelog/0.15.md)

---

## [0.14.x] - 2026-03-09 - Parent-Funded Cycles Control Plane

- `0.14.4` upgrades recurring architecture/auth audits with normalized risk scoring, structural hotspot tracing, early-warning/fan-in detection, and stronger layer-drift checks so risks are easier to spot before regressions ship.
- `0.14.3` standardizes delegated-token issuance naming on `issue`, adds `DelegationApi::issue_token` as the single app-facing issuance path, and removes legacy `mint` naming from delegation endpoints and metrics labels.
- `0.14.2` consolidates metrics queries under `canic_metrics` (`MetricsRequest`/`MetricsResponse`) and removes the per-metric `canic_metrics_*` endpoint variants.
- `0.14.1` removes `funding_policy` config fields and keeps `topup_policy` as the only cycles config surface, while restoring unbounded request evaluation so oversized requests fail on actual parent balance checks instead of being clamped by config.
- `0.14.0` makes subtree funding parent-only with replay-safe RPC execution, adds an app-level global funding kill switch, and ships parent-emitted cycles funding metrics (totals, per-child, and denial reasons).

```text
canic_metrics(record { kind = variant { RootCapability }; page = record { limit = 100; offset = 0 } })
```

See detailed breakdown:
[docs/changelog/0.14.md](docs/changelog/0.14.md)

---

## [0.13.x] - 2026-03-07 - Distributed Capability Invocation

- `0.13.8` hardens cycles top-up safety validation with stronger config tests, restructures design/audit documentation layout for maintainability, and adds the `0.14` parent-funded cycles control-plane design/status documentation.
- `0.13.7` completed lifecycle boundary follow-up coverage (non-root repeated post-upgrade readiness plus non-root post-upgrade failure-phase checks), tightened root capability metric internals, refreshed replay/audit run guidance for constrained local environments, and fixed intent concurrency capacity checks so `max_in_flight` counts only pending reservations (preventing committed claim intents from permanently blocking later claims for the same caller-scoped key).
- `0.13.6` expanded auth/replay/capability test coverage and aligned root replay integration tests with current duplicate handling, while making the shared root test harness recover cleanly after a failed test.
- `0.13.5` further reduced branching pressure by moving replay commit fully into ops, switching built-in access predicates to evaluator-based dispatch, and replacing monolithic root capability metric events with structured `event_type`/`outcome`/`proof_mode` metrics.
- `0.13.4` simplified proof, replay, and auth internals with pluggable verifiers, a dedicated replay guard path, faster duplicate rejection, and clearer delegated-auth error grouping.
- `0.13.3` finished the auth/control-plane extraction, standardized directory modules with `mod.rs`, and refreshed complexity/velocity audit baselines.
- `0.13.2` continued the module split and moved request/auth helpers behind cleaner facades, reducing coupling between high-traffic code paths.
- `0.13.1` split large RPC/auth workflow files into smaller modules, making the control plane easier to read and change without altering behavior.
- `0.13.0` introduced signed capability envelopes for cross-canister root calls, with built-in replay protection and capability hashing to prevent request reuse/tampering.

```text
same request_id + same payload -> ReplayDuplicateSame (rejected)
same request_id + different payload -> ReplayDuplicateConflict (rejected)
```

See detailed breakdown:
[docs/changelog/0.13.md](docs/changelog/0.13.md)

---

## [0.12.x] - 2026-03-07 - Root Role Attestation Framework

- `0.12.0` adds root-signed role attestations and an attested root dispatch path, so services can authorize callers by signed proof instead of full directory sync.

See detailed breakdown:
[docs/changelog/0.12.md](docs/changelog/0.12.md)

---

## [0.11.x] - 2026-03-07 - Capabilities Arc and Replay Hardening

- `0.11.1` hardens root capability replay/dispatch behavior and improves auth diagnostics to make failure triage easier.
- `0.11.0` starts the capability-focused auth line with stronger scope checks and safer account/numeric behavior.

See detailed breakdown:
[docs/changelog/0.11.md](docs/changelog/0.11.md)

---

## [0.10.x] - 2026-02-24 - Delegated Auth Tightening and Runtime Guardrails

- `0.10.5` switched HTTP outcall APIs to raw response bytes, tightened memory-bootstrap safety, and reduced default wasm artifact size.
- `0.10.2` fixed lifecycle ordering so memory bootstrap is guaranteed before env restoration and runtime stable-memory access.
- `0.10.1` added optional scope syntax to `authenticated(...)` while preserving delegated-token verification semantics.
- `0.10.0` moved authenticated endpoints to direct delegated-token verification with explicit root/shard/audience binding and removed relay-style auth envelopes.

```rust
let raw: HttpRequestResult = HttpApi::get(url).await?;
```

See detailed breakdown:
[docs/changelog/0.10.md](docs/changelog/0.10.md)

---

## [0.9.x] - 2026-01-19 - Delegated Auth and Access Hardening

- `0.9.26` exported `SubnetRegistryApi` at the stable public path.
- `0.9.25` expanded network/pool bootstrap logging for clearer operational diagnostics.
- `0.9.24` added root top-up balance checks and safer pool-import bootstrap ordering.
- `0.9.23` renamed canister kinds and sharding query terminology to the current contract.
- `0.9.20` fixed multi-argument delegated-token ingress decoding and removed legacy dev bypass behavior.
- `0.9.18` enforced compile-time validation rules for authenticated endpoint argument shapes.
- `0.9.17` moved local bypass handling into delegated verification so auth paths stay consistent.
- `0.9.16` added a local/dev short-circuit path for delegated auth under controlled conditions.
- `0.9.14` removed delegation rotation/admin/status surfaces as part of shard lifecycle cleanup.
- `0.9.13` added signer-initiated delegation request support through root.
- `0.9.12` completed auth delegation audit follow-up and strengthened view-boundary usage.
- `0.9.11` added delegated-auth rejection counters for better operational visibility.
- `0.9.10` standardized the delegated-auth guard surface as `auth::authenticated()`.
- `0.9.7` cleaned up IC call builders so argument encoding/injection is consistently fallible and explicit.
- `0.9.6` hardened lifecycle/config semantics and normalized app config naming.
- `0.9.5` aligned access predicates into explicit families (`app`, `auth`, `env`) with a cleaner DSL surface.
- `0.9.4` made app init mode config-driven and aligned sync access behavior.
- `0.9.3` made app-state gating default-on for endpoints unless explicitly overridden.
- `0.9.2` moved endpoint authorization to a single `requires(...)` expression model with composable predicates.
- `0.9.1` ran consolidation audits to tighten layering boundaries and consistency rules.
- `0.9.0` established the delegated-auth baseline and runtime architecture for proof-driven endpoint authorization.

See detailed breakdown:
[docs/changelog/0.9.md](docs/changelog/0.9.md)

---

## [0.8.x] - 2026-01-13 - Intent System and API Consolidation

- `0.8.6` raised intent pending-entry storage bounds to safely handle large keys.
- `0.8.5` introduced the stable-memory intent system with reserve/commit/abort flows and contention coverage.
- `0.8.4` cleaned up docs and reduced redundant snapshot/view conversions.
- `0.8.3` exposed protocol surfaces through the public API layer.
- `0.8.1` exported `HttpApi` under `api::ic` alongside call utilities.
- `0.8.0` consolidated the public API surface and hardened error-model consistency.

See detailed breakdown:
[docs/changelog/0.8.md](docs/changelog/0.8.md)

---

## [0.7.x] - 2025-12-30 - Architecture Consolidation and Boundary Cleanup

- `0.7.28` moved macro entrypoints into the `canic` facade crate.
- `0.7.26` cleaned up stale docs and layering inconsistencies.
- `0.7.23` added a fail-fast root bootstrap guard for uninitialized embedded wasm registries.
- `0.7.22` unified internal topology state on authoritative `CanisterRecord`.
- `0.7.21` expanded IC call workflow helpers with argument-aware variants.
- `0.7.15` standardized endpoint-wrapper error conversion into downstream error types.
- `0.7.14` removed DTO usage from ops via ops-local command types.
- `0.7.13` standardized infra error bubbling and structure under ops.
- `0.7.12` switched signature internals to the `ic-certified-map` hash tree path.
- `0.7.11` moved sharding placement to a pure deterministic policy model.
- `0.7.10` moved API instrumentation ownership into `access`.
- `0.7.9` mirrored authentication helpers into `api::access`.
- `0.7.8` aligned topology policy modules under `policy::topology`.
- `0.7.7` split `api/topology` and filled missing surface functions.
- `0.7.6` resynced certified data from the signature map during post-upgrade.
- `0.7.4` expanded `canic-cdk` with additional ckToken support.
- `0.7.3` added a public `api::ic::call` wrapper routed through ops instrumentation.
- `0.7.2` tightened workflow/policy naming and topology lookup contracts.
- `0.7.1` tightened ops-layer boundaries through an explicit audit pass.
- `0.7.0` consolidated architecture/runtime discipline and clarified boundary ownership.

See detailed breakdown:
[docs/changelog/0.7.md](docs/changelog/0.7.md)

---

## [0.6.x] - 2025-12-18 - Runtime Hardening and Pool Evolution

- `0.6.20` added stricter canister-kind validation, typed endpoint identity, and registry/pool hardening.
- `0.6.19` switched endpoint perf accounting to an exclusive scoped stack model.
- `0.6.18` added log entry byte caps and fixed several lifecycle/http/sharding edge cases.
- `0.6.17` added bootstrap-time pool import support (`pool.import.local` / `pool.import.ic`).
- `0.6.16` hardened pool import/recycle/install failure handling and state cascade behavior.
- `0.6.13` made env/config access fallible with clearer lifecycle failure behavior and stronger directory/env semantics.
- `0.6.12` enforced build-time `DFX_NETWORK` validation across scripts and Cargo workflows.
- `0.6.10` improved ICRC-21 error propagation for idiomatic `?` handling.
- `0.6.9` renamed reserve configuration to pool and introduced status-aware import modes.
- `0.6.8` removed mutex-based randomness plumbing and introduced configurable reseed behavior.
- `0.6.7` replaced macro panics with compile errors for unsupported endpoint parameter patterns.
- `0.6.6` restored build-network access and aligned access-policy/runtime wrappers.
- `0.6.0` introduced a major endpoint-protection/runtime refactor and split metrics endpoints.

See detailed breakdown:
[docs/changelog/0.6.md](docs/changelog/0.6.md)

---

## [0.5.x] - 2025-12-05 - Metrics, Lifecycle, and Memory Foundations

- `0.5.22` aligned CI to build deterministic wasm artifacts before lint/test gates.
- `0.5.21` consolidated perf/type paths and improved timer metric labeling.
- `0.5.17` added ops-level HTTP metrics support.
- `0.5.16` fixed CMC top-up reply handling so failed top-ups are not reported as success.
- `0.5.15` simplified reserve-pool lifecycle orchestration.
- `0.5.14` split metrics into ICC and system categories.
- `0.5.13` centralized canister call metric recording through wrapped cross-canister construction.
- `0.5.12` made topology sync branch-targeted with safer fallback behavior.
- `0.5.10` added a wrapper around `performance_counter`.
- `0.5.8` reduced cascade complexity toward near-linear sync behavior.
- `0.5.7` improved create-flow bootstrap diagnostics with caller/parent context logs.
- `0.5.6` unified background timer startup through a single role-aware service entrypoint.
- `0.5.4` hardened reserve import/recycle sequencing and cascade safety.
- `0.5.2` split stable-memory infrastructure into `canic-memory` and re-exported runtime/macro support.
- `0.5.1` moved shared wrappers into `canic-core::types` and slimmed public type exports.
- `0.5.0` introduced the `canic-cdk` facade and stabilized a curated IC integration surface.

See detailed breakdown:
[docs/changelog/0.5.md](docs/changelog/0.5.md)

---

## [0.4.x] - 2025-12-01 - Registry and Signature Stability Passes

- `0.4.12` unified signature verification entrypoints and fixed root child-directory rebuild behavior.
- `0.4.8` tightened memory visibility and removed unused internals.
- `0.4.7` fixed signature verification panic behavior for short principal forms.
- `0.4.6` aligned directory rebuild behavior and added end-to-end consistency coverage.
- `0.4.1` fixed canister registration ordering to avoid phantom entries on install failure.
- `0.4.0` formalized the `endpoints -> ops -> model` layering contract.

See detailed breakdown:
[docs/changelog/0.4.md](docs/changelog/0.4.md)

---

## [0.3.x] - 2025-11-15 - Pagination and Logging Foundations

- `0.3.15` expanded app/subnet directory access across canisters with paginated DTO responses.
- `0.3.0` added paginated subnet-children APIs and introduced configurable bounded log retention.

See detailed breakdown:
[docs/changelog/0.3.md](docs/changelog/0.3.md)

---
## [0.2.x] - 2025-11-10 - PRIME Subnet and Topology Foundations

- `0.2.24` added `cfg(test)`-gated PocketIC helper support under `test/`.
- `0.2.21` fixed nested canister-role validation so invalid deep config is detected correctly.
- `0.2.17` removed the `icrc-ledger-types` dependency in favor of a local implementation.
- `0.2.10` switched sharding structures to string-based IDs and standardized scaling placement on HRW.
- `0.2.9` strengthened recursive config validation, including invalid subnet-directory detection.
- `0.2.7` moved `xxhash` utilities into `canic` for shared sharding usage.
- `0.2.6` continued layer cleanup by splitting memory/ops responsibilities and moving reserve config to per-subnet settings.
- `0.2.3` moved app/subnet directory projections to `SubnetCanisterRegistry` and included directory state in canister init payloads.
- `0.2.2` removed legacy delegation flow and added `ops::signature` for canister-signature creation/verification.
- `0.2.1` shipped early stabilization fixes after the initial topology rollout.
- `0.2.0` introduced prime-subnet topology foundations, including `SubnetRole`, `Env` identity context, and synchronized state+directory snapshots.

See detailed breakdown:
[docs/changelog/0.2.md](docs/changelog/0.2.md)

---

## [0.1.x] - 2025-10-08 - Initial Publish and Early Runtime Foundations

- `0.1.7` added subnet PID capture support with `dfx 0.30.2` for root subnet context tracking.
- `0.1.4` added delegation sync helpers and a more ergonomic `debug!` logging macro.
- `0.1.3` refreshed documentation, including a README rewrite and cleanup of outdated docs.
- `0.1.0` published `canic` to crates.io after the final rename from `icu`.

See detailed breakdown:
[docs/changelog/0.1.md](docs/changelog/0.1.md)
