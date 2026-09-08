# Canonical Fleet Subnet Root

Date: 2026-09-07. Scope: the maintainer-selected CR1 hard cut after 0.110.9.
Package versions remain 0.110.9 during implementation. This report records
targeted working-tree qualification, not a published validation receipt.

## Contract

Canic owns `canic-fleet-coordinator`, `canic-fleet-root` and
`canic-fleet-wasm-store`. The last name replaces `canic-wasm-store` outright.
The maintained role identifiers remain `fleet_coordinator`, `root` and
`wasm_store`.

Apps declare `[roles.root]` with `kind = "root"` and no package path. Ordinary
application roles still require package paths. The host materializes the
canonical Root entrypoint and an exact dependency manifest, selecting the
required Canic features from the App configuration. It validates the real
Cargo graph before building. This also works for packaged consumers with no
Root source crate and for a standalone Cargo package without a lockfile.

The Root entrypoint contains only Canic lifecycle and endpoint composition.
Root application hooks and lifecycle participants are removed, as are the
demo/test application Root crates. Application lifecycle hooks remain in
their existing owner. Unpublished instrumented Root fixtures use the same
canonical lifecycle. Runtime orchestration stays in `canic-control-plane`.

Root configuration, capability selection, Candid, release identity and artifact
hashes remain build authority. Root provenance records the generated build's
lockfile. Passive capability/state reports derive Root's configured contract;
they do not certify an artifact. Artifact construction still validates its
actual package graph. Medic no longer asks for an application Root manifest.

Pre-1.0 release transitions remain explicit reviewed reinstall. Same-release
interruption recovery, intent-before-effect records, bounded paid effects,
cycle conservation and effect-free replay retain their existing owners. This
batch adds no runtime mode, compatibility path or custody mechanism.

## Focused evidence

| Boundary | Result |
| --- | --- |
| Config parsing, topology and role validation | 132 passed; the final package-authority assertion also passes |
| Host package, capability, provenance and config projections | 86 passed |
| Canonical/standalone Root selection and build grouping | 22 host tests plus the targeted config assertion passed |
| Scaffold | 23 passed |
| Medic, App and status projections | 42 passed |
| Build CLI and final help surface | 24 passed; the updated help check passes on its focused rerun |
| Lifecycle, managed endpoint, build cfg, Candid and workspace guards | 57 passed |
| Affected packages, all targets/features, warning-denied Clippy | Passed |
| Packaged consumer without a Root crate | Passed; Wasm, gzip and Candid produced from extracted packages |
| Crypto closure | Passed, including unsigned canonical Root and signed Root fixture |
| Modified shell scripts and current document semantics | Passed |
| Final changelog regression and diff checks | Passed |
| Canonical Root artifact-cache identity and final helper Clippy | Passed |
| Final Root selection tests, warning-denied host Clippy and changed-file formatting | Passed; two selection tests and 84 Rust files |
| Fresh generated Fleet, interruption and terminal replay | Passed in 720.46s, including a 4m03s cold artifact build; all 50 effects and both effect-free replays |
| Generated reinstall, lost response and terminal replay | Passed in 496.36s; exact reset recovery, working Fleet, conservation and both effect-free replays |

Local logs use the `/tmp/canic-canonical-root-` prefix, including
`fresh.log` and `packaged.log`. The packaged check uses the
`root` selector of `scripts/ci/verify-packaged-downstream-wasm-store.sh`; it
shares the maintained packaging flow and omits unrelated consumer scenarios.
The two runtime boundaries reuse existing application-neutral cases. The first
fresh run stopped before deployment because its cache expected a pre-existing
Root package. Both external artifact caches now capture the configured Root's
actual generated Cargo graph. The existing repeatable/distinct release-identity
cache regression passes, as does scoped Clippy. The batch build lock now covers
canonical manifest materialization as well as compilation and finalization.
After the fresh proof, only the Root entrypoint test's parsed macro comparison
and a macro documentation list changed. The focused selection tests and host
Clippy pass on those final edits; fresh runtime behavior is unchanged.

## Adoption and readiness

CR1 is complete and ready for release approval. No implementation or focused
qualification blocker remains in this batch. The complete batch is recorded
in the open 0.110.10 changelog; package versions remain 0.110.9. The
maintainer-selected release gate and publication remain separate. CANIC-141
remains deferred. Broader runtime contraction and further downstream feedback
are outside this batch.

After adoption, a downstream App must remove its Root crate/workspace member
and `roles.root.package`, update any explicit Store package reference, pin the
matching Canic/CLI release and rebuild its sealed artifacts. Application-owned
initialization belongs in application canisters. Review the actual estate,
controllers and funding before reinstall/Ensure, then validate application
readiness, descendants, reserve, recovery and cycle accounting in the intended
environment. Canic's synthetic proofs do not certify a downstream deployment.

No sibling repository edit, broad gate, version transaction, publication or
deployment is part of this working-tree qualification.
