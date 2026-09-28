# PR #32: Toko authentication integration

Date: 2026-09-28. Base: Canic 0.110.45. Open batch: 0.110.46.

## Source and review

Reviewed and incorporated Gabriel's [PR #32](https://github.com/dragginzgame/canic/pull/32),
head `84f696a573375ea57052f5c685277aeb0c8fe35f`, into the local working tree.
The PR was not merged or published by this session.

The role-contract defect is confirmed: attestation-cache verifier capability
required canister-signature verification but omitted the ECDSA feature needed to
validate configured chain-key material. The catalogue now requires both, and its
regression rejects the missing feature while accepting the complete feature set.
This changes no signing permission or storage allocation.

The new `GetChainKeyPublicKey` Root command accepts a key name and derivation
path. Endpoint controller authorization precedes execution. The request has no
canister-ID override: the management call explicitly targets the receiving Root.
It returns public-key bytes, calls no signing operation and changes no issuer
policy. Existing activation and authority-fence checks remain in force. Its
replay classification is read-only, and it is emitted for root-delegation-capable
Roots.

The original API handler invoked management operations directly. The incorporated
version instead follows API → workflow → ops, with request conversion and local
canister binding in ops. DTO and wire shapes match the submitted PR. The
[operator guide](../../../../operations/root-proof-provisioning.md) documents the
command and application caller boundaries.

## Evidence

The following PR-only evidence predates the offline-derivation extension below.

- Role-contract native tests: **29 passed** (`.tmp/pr32-role-contract.log`).
- Replay-policy native tests: **30 passed** (`.tmp/pr32-replay-policy.log`).
- Protocol-surface tests: **39 passed** (`.tmp/pr32-protocol.log`).
- Real PocketIC: `active_registry_issues_component_role_attestations` passes
  with the added key-discovery assertions. A non-controller receives the typed
  controller-required error; a controller derives a compressed public key;
  repetition returns identical bytes; another derivation path returns a different
  key. Existing registry-bound attestations still pass. Test: **267.28s**, including
  fixture artifact preparation; complete targeted runner: **484s**. Full log:
  `target/test-runs/20260928T100724Z-36817.hGHQUt/1.log`.
- Scoped warning-denied core/facade/internal library/test Clippy: **pass**,
  10.95s final run (`.tmp/pr32-clippy.log`). Initial lint findings were confined
  to the new test helper's missing test-only module gate and a `let...else`
  style correction. The final pass includes both library and test configurations.

No full workspace gate, Toko deployment or sibling mutation ran. Gabriel's
reported local Toko smoke result is external evidence, not a deployment reproduced
by this session.

## Offline derivation extension

The maintainer selected immutable build configuration and no public-key discovery
calls in deployment. The same open batch now supports
`auth.delegated_tokens.chain_key_root_proof.public_key_derivation = "ic"` or
`"pocketic"`. Host parsing resolves the public key and path hash before ordinary
configuration validation and role source generation. Root identity is explicit;
unknown master keys, network mismatch, invalid paths, and stale supplied values
produce typed configuration errors before compilation or live effects.

The implementation uses the pinned DFINITY `ic-secp256k1` 0.3.0 library's public
derivation APIs. It is a native-only dependency, with no new cryptographic code
or runtime key distribution in Wasm. The IC selection admits production
`key_1`; PocketIC selects its separate known test master keys. Existing explicit
public-key configuration remains available for other master-key environments.

The redundant public-key lookup before batch signing is removed. Signing still
calls `sign_with_ecdsa`; returned signatures are checked locally against the
configured key before being stored. The tests preserve wrong-key/corrupt-signature
rejection, high-S normalization, exact signing arguments, cached batch reuse,
failure persistence and retry delays. Explicit Root key inspection remains an
optional online diagnostic; no automatic discovery or fallback calls it.

For manually supplied keys without offline derivation, a mismatched but well-formed
key is now rejected when the signing response fails local verification, after
the signing call has been paid. Offline derivation checks stale supplied material
before building. Existing signing retry delays remain in force; no extra signing
retry loop was added.

Current extension evidence:

- **79 targeted native tests pass**, including five new offline-config cases;
  `.tmp/offline-key-regression.log`. Round-trip configuration, role projection,
  Root/path/network binding and typed malformed/stale-input rejection are covered.
- **Scoped warning-denied Clippy passes** for core, facade and internal tests
  with all features (75s); `.tmp/offline-key-clippy.log`.
- **PocketIC passes**: `active_registry_issues_component_role_attestations` proves
  exact equality of offline config derivation and management output, controller
  rejection, deterministic replay and path binding. Existing attestation checks
  pass with rebuilt Root/Coordinator/Component artifacts. Test: **277.47s**;
  complete runner: **473s** including native compilation and artifact preparation.
  `.tmp/offline-key-pocketic.log` retains the runner result; full case output is
  `target/test-runs/20260928T104302Z-37360.TO0RRM/1.log`.
- Scoped Rust formatting (24 files) and `git diff --check` pass.

The initial new unit fixture serialized cycle amounts in an output-only form;
it was corrected to serialize only auth over an authored minimal App fixture.
Initial lint findings were limited to redundant visibility and an `Option`
reference in the new module. Both focused reruns pass.

The [offline runbook](../../../../operations/root-proof-provisioning.md#offline-public-key-configuration)
contains the operator configuration. Downstream deployment scripts must select
this setting and stop requesting the diagnostic key command. No Toko files or
live canisters were changed. There is no claim of measured mainnet speedup.

## Remaining Toko work

| Reported caller or failure | Current Canic boundary / remaining work |
| --- | --- |
| `toko_active_component_role` | Canonical Root registry status queries are controller-only. The maintained attestation flow can authorize role-bound calls, with explicit subject, audience, subnet, TTL and epoch checks; it is not an instantaneous role lookup. The receiver must check its expected role. Gabriel must assess the original caller's freshness requirements before selecting this flow. |
| `toko_provision_issuer_delegation` | `GetOrCreateDelegationProof` binds the issuer to its caller. A user hub cannot invoke it for a user shard. Issuer lazy repair and controller-configured Root renewal are maintained. The provisioning Rust facade runs inside Root and is not a manager-callable wire command. Immediate manager-triggered provisioning would need a reviewed caller/issuer authorization contract. |
| 37 `project_instance` failures / `IcydbError("E210")` | Cause remains unverified. The affected Toko checkout is not available here; this session cannot classify those failures as unrelated. |
| Dependency/CLI selection | Adopt the matching released Canic runtime and CLI, or pin a reproducible source revision and build both from that selection. A sibling path and an older global CLI do not establish matching artifact provenance. |

These Canic fixes do not establish Toko staging readiness. No application-specific
Root aliases, permissive role guards or issuer-manager API were added.

## Release disposition

This is a necessary correction to the published 0.110 integration and remains on
that line despite the release-count guideline. The accepted Canic correction and
its changelog are ready for maintainer review and the selected release gate.
Package versions remain 0.110.45; 0.110.46 is an unreleased draft. All changes are
uncommitted. No new minor, version transaction or Git publication was started.
