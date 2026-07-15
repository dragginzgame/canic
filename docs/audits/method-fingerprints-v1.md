# Audit Method Fingerprints v1

- snapshot_status: `post_freeze_correction_in_progress`
- prepared_at: `2026-07-15`
- release_anchor: `v0.92.0`
- source_commit_full: `91736337fc1cfeb891f17d7d62affb5e671348e2`
- source_tree_hash: `fd31bb8289365a38f2bea7f8ebd6973908ee959f`
- baseline_product_tree_hash: `c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e`
- frozen_method_commit: `91736337fc1cfeb891f17d7d62affb5e671348e2`

These SHA-256 identities describe the current active method set. The initial
set was frozen at `v0.92.0`; a post-freeze correction changes only the affected
versioned method and preserves its superseded identity below. A correction in
the working tree is not a committed audit authority until the maintainer
commits it.

A method change after freeze must increment the affected method version and
follow the defect/invalidation protocol in [AUDIT-HOWTO.md](AUDIT-HOWTO.md).

## Active Definition Identities

| Audit ID | Version | SHA-256 | Definition |
| --- | --- | --- | --- |
| `CANIC-AUTH-AUDIENCE-001` | `2` | `bfe780a3e93f0511f9c7bbbbf7cf84dee40b23d1456ab68fe12122a671b30a5c` | `docs/audits/recurring/invariants/audience-target-binding.md` |
| `CANIC-AUTH-EQUIVALENCE-001` | `1` | `3339c221ec11706c00b4dbe2d9d4be116441b1f0572b5e7060a23a74199f89d6` | `docs/audits/recurring/invariants/auth-abstraction-equivalence.md` |
| `CANIC-AUTH-BOUNDARY-001` | `1` | `f5383b05617c0f5c3df5f9a2e230e200925c4aa0a16586787d32ac6bab427b98` | `docs/audits/recurring/invariants/canonical-auth-boundary.md` |
| `CANIC-AUTH-CAPABILITY-001` | `1` | `4dd5127b43716dc8a89bef6148b76856794f7c5e158f536a7737ddceee0f1c55` | `docs/audits/recurring/invariants/capability-scope-enforcement.md` |
| `CANIC-AUTH-REPLAY-001` | `2` | `743b9fcc18e37be029e12fa9db2a5fa5ffb8d4258c484739a5b0e73e43632b6d` | `docs/audits/recurring/invariants/expiry-replay-single-use.md` |
| `CANIC-AUTH-SUBJECT-001` | `1` | `8af2c270ba89aae9715e6047afa908b1812865c5949f0f1da6441539fdee4475` | `docs/audits/recurring/invariants/subject-caller-binding.md` |
| `CANIC-AUTH-TRUST-001` | `1` | `8676d779cca173e3cb5cde61e29635eb14732dde6cf89fbd06b08c5ab19c5db7` | `docs/audits/recurring/invariants/token-trust-chain.md` |
| `CANIC-LIFECYCLE-001` | `1` | `c3b99716e67d6fc83bd33a9875ef93bb5eb7e9d9756fcfbce92c5030046c48cd` | `docs/audits/recurring/system/bootstrap-lifecycle-symmetry.md` |
| `CANIC-BUILD-INTEGRITY-001` | `2` | `e75c8fdc54f090bd901482f50c88e2b6272830d1425f24d7165904c1b206a94b` | `docs/audits/recurring/system/build-integrity.md` |
| `CANIC-CAPABILITY-SURFACE-001` | `2` | `91e61f3385882d108b8541e31715b1ee1e126299f7ad64890c979043a9d7c759` | `docs/audits/recurring/system/capability-surface.md` |
| `CANIC-CHANGE-FRICTION-001` | `2` | `5f4377f00907f36f59388f797f210bdfed9398832f983529cdccd4bd747d2ab6` | `docs/audits/recurring/system/change-friction.md` |
| `CANIC-COMPLEXITY-001` | `2` | `76bb53a536f252348567d32fd0779a40347e54c254c5fd726207253dcd069fce` | `docs/audits/recurring/system/complexity-accretion.md` |
| `CANIC-DEPENDENCY-001` | `2` | `ad7b459667545ec5b3adfd33a614803e2c11fa77a28af873392a1d3344333f6f` | `docs/audits/recurring/system/dependency-hygiene.md` |
| `CANIC-DUPLICATION-001` | `1` | `c4b2b2828f551a5419de394d442ecb04932900d7b15665177a3c8529ee340262` | `docs/audits/recurring/system/dry-consolidation.md` |
| `CANIC-INSTRUCTION-001` | `2` | `610ee7acc1eb4675d19d1495ec8cccdf8132bc81411aa8d0196f6fe46308d243` | `docs/audits/recurring/system/instruction-footprint.md` |
| `CANIC-LAYERING-001` | `2` | `a4c71532e85f3ea0c5f1802478b15f444d78eae3540dc35b96b77b04231503bc` | `docs/audits/recurring/system/layer-violations.md` |
| `CANIC-STRUCTURE-001` | `1` | `ca370a2c910c4d9d3755af74099c6d5715086d8b1ff226c29a40c77c5ee9f58e` | `docs/audits/recurring/system/module-structure.md` |
| `CANIC-PUBLISH-001` | `1` | `8e2eff6ac0c60c9903cd68f6354f7536636a987fd437306e851643464bdef884` | `docs/audits/recurring/system/publish-surface.md` |
| `CANIC-RELEASE-INTEGRITY-001` | `1` | `3f6b87b30a3c1f9c80803a8be5d45292e73217d260ea435a956bd05f10d63438` | `docs/audits/recurring/system/release-integrity.md` |
| `CANIC-AUTH-ORDERING-001` | `1` | `bf5e5a5bb0eb22f22cf30098bc881838c7908209f45c26d0296f546ac55e1802` | `docs/audits/recurring/system/security-boundary-ordering.md` |
| `CANIC-WASM-001` | `2` | `e33fc36ee904fa6a9af8c7aa399a94b98c441e25fe6590ac1548c548ba2f3ffb` | `docs/audits/recurring/system/wasm-footprint.md` |
| `CANIC-MODULE-SURFACE-001` | `2.0` | `404a359b4448ea7288055f0444e3178ae972f4eb7e1a0814aa693ce67df59030` | `docs/audits/modular/module-surface-hardening.md` |

## Superseded Definition Identities

| Audit ID | Version | SHA-256 | Definition | Superseded by |
| --- | --- | --- | --- | --- |
| `CANIC-AUTH-AUDIENCE-001` | `1` | `9d28324a6101e94ba964e8d8478909323e16e83bc0134975ab37f69030602448` | `docs/audits/recurring/invariants/audience-target-binding.md` | `CANIC-AUTH-AUDIENCE-001/v2` |
| `CANIC-AUTH-REPLAY-001` | `1` | `2a4726ca049194175f1230c9de54442746d462d460f2adea77b8b1df57f8868c` | `docs/audits/recurring/invariants/expiry-replay-single-use.md` | `CANIC-AUTH-REPLAY-001/v2` |
| `CANIC-BUILD-INTEGRITY-001` | `1` | `57f0a380b1722927498ddd0f41b5490e8726cab943c2d3df02ecac73897a5311` | `docs/audits/recurring/system/build-integrity.md` | `CANIC-BUILD-INTEGRITY-001/v2` |
| `CANIC-CAPABILITY-SURFACE-001` | `1` | `d7de4f8b7115b5e4861bde23aaebe9b2ddee3c83a07f7730b61122b3f3fff898` | `docs/audits/recurring/system/capability-surface.md` | `CANIC-CAPABILITY-SURFACE-001/v2` |
| `CANIC-CHANGE-FRICTION-001` | `1` | `00646b257428623f7ef4efce4dffdcd93f3bdc75cd7e2dbc02faad32cb2ce8d6` | `docs/audits/recurring/system/change-friction.md` | `CANIC-CHANGE-FRICTION-001/v2` |
| `CANIC-DEPENDENCY-001` | `1` | `71be0c1d68cc573bc7c17232709b3a576d9cba903eaa9062665ae9bc71a58194` | `docs/audits/recurring/system/dependency-hygiene.md` | `CANIC-DEPENDENCY-001/v2` |
| `CANIC-INSTRUCTION-001` | `1` | `f90bbd1443ac5acdcc69ad256eaef8877955a9219025f65c6255c6fdd7bf2805` | `docs/audits/recurring/system/instruction-footprint.md` | `CANIC-INSTRUCTION-001/v2` |
| `CANIC-LAYERING-001` | `1` | `86270ae481556a8f5b544d71529d3b324cf5dbf7af7267100a6a74976eacfc49` | `docs/audits/recurring/system/layer-violations.md` | `CANIC-LAYERING-001/v2` |
| `CANIC-WASM-001` | `1` | `1ed32dd340d10135e899cda5794046d68e1e66ea89da9d6910aa4ca4e958a064` | `docs/audits/recurring/system/wasm-footprint.md` | `CANIC-WASM-001/v2` |

## Executable Composite Identities

- `CANIC-INSTRUCTION-001/v1`: `c79f7027f3629bcbe4dbf4680005d3a9b37104c7ba6d4956a5a3c789c5b5cfab`
- `CANIC-INSTRUCTION-001/v2`: `385ea065d337781828a10a9167948309d9bafb9e126434142aeb0104eacfc584`
- `CANIC-WASM-001/v1`: `e8c58213d9301d66d4adac4bd92e4aa702fd887b8adb55e2e602a70f29e9c505`
- `CANIC-WASM-001/v2`: `c88cdf0ef4e79b3c1702ef9c00777f5a7940c4771ab51dbf8442fabd71396bc5`

The runtime runners calculate these composites from their definition, script,
and executable fixture inputs and record the resulting value in each run.

## Governance And Executable Input Identities

| SHA-256 | Input |
| --- | --- |
| `113b55a44dda0e0780fc3dca5743eaec3949c99456594de5fe10e2a3469487c2` | `crates/canic-host/examples/build_artifact.rs` |
| `9bde640ebb6f70c0649a2ef862d32e29b0af20fb0a4a52e3c6a0fc74fac488c2` | `crates/canic-tests/tests/instruction_audit.rs` |
| `4b096319516ee6526ec4e97d2ccd58d6c36bc875cd9e4d91635ee9232dd2db5f` | `crates/canic-tests/tests/instruction_audit_support/estimates/mod.rs` |
| `f25fe2996fca7869c3a43bfaba08918f7137adea535b79b3831fcd4d84870b11` | `crates/canic-tests/tests/instruction_audit_support/execution.rs` |
| `f4fba7b003f14d952d6af5c7f9e8c0e50c707648188b5201275ede2e69cbdff3` | `crates/canic-tests/tests/instruction_audit_support/mod.rs` |
| `c6d2340247e17d83ae95c42d795339a8d9d7317dfad089008d41f78c2dab3483` | `crates/canic-tests/tests/instruction_audit_support/report.rs` |
| `039e5910c1d3235c98852bd9893ece9d80ba2e2ebdf75c0b7ecfc933b3658f9c` | `crates/canic-tests/tests/instruction_audit_support/scenarios.rs` |
| `759301e30336b4c26fa4cffd20e3807d7710b05aa55c965ad159564342905422` | `docs/audits/AUDIT-HOWTO.md` |
| `5d7e2f15bdd195c63276d37beffce6a88fea2b593abfd7a1d6ede7274e6aa3c6` | `docs/audits/META-AUDIT.md` |
| `26d16b0adc508176a938bcaeb05f54266b18ab4b8b1463ac20615c7ff5e482a4` | `docs/audits/METHODS.md` |
| `ea2c06b003464d6be8f458e07090082ac39f611b1c1907ff2d48ee7f9702e3c7` | `docs/audits/mandatory-trace-protocol.md` |
| `5fee9fc12be72d84a64137f4f3467833d895b611ea899dce91c34e89a56ee472` | `docs/audits/product-tree-scope-v1.md` |
| `a5eee1b85b1d54bfc23285e58360690b3bc09c0c1aece7e9440a8b029ec00475` | `docs/audits/retired-methods.md` |
| `7fda1aecc7a06d7d985b8ef62c338b75ce95ae1adc89d8d81f9a74a8e0377989` | `docs/audits/fixtures/layering/allowed-import.txt` |
| `9064f8aaf36c2f68626d28c98b389f5e8e7bc728281047f0435f26411638000a` | `docs/audits/fixtures/layering/forbidden-direct-import.txt` |
| `4135092a556dbcabfe895b27b6666f5d54f041526ee8e040c2777c9b875f0437` | `docs/audits/fixtures/layering/forbidden-grouped-import.txt` |
| `c4db862c85b44585f88562030560907a6ef1a8a5926fd4682445a10a8be41b6e` | `docs/audits/fixtures/layering/forbidden-nested-grouped-import.txt` |
| `42b440e2fc4d47394b7b8b99a69d1c05a91e79aa7eb3cd8872aa0520746078d3` | `docs/audits/fixtures/change-friction-v2-sample.tsv` |
| `7ffa84f792ef90208bbfcdd11d386ddc5e482521ba314e71a17f6070a4352c5c` | `docs/audits/scripts/measure-change-friction-v2.sh` |
| `4ff697d1ed68db19bca8810f609ea40547486a2174e81271312828ef034ca7c8` | `docs/audits/scripts/measure-complexity-v2.sh` |
| `8f4a46a26e56b845290c3adc4994826b8a10084c97a2c68579ca60038f8e1be8` | `docs/audits/scripts/run-nonempty-cargo-test.sh` |
| `ac7ab348d0e9a18df9def45f89f1c403f7c23e523eaf58da03b5099fb2634417` | `scripts/ci/audit-product-tree-hash.sh` |
| `08fd9421a891e118ace74392580cceb0c82a5c5c3aa194ac5011b497d3bba845` | `scripts/ci/check-audit-method-catalog.sh` |
| `960e8d64eef1b260f38ccc16fa65a517ecbcb2e026a00acb66282172f57af34d` | `scripts/ci/instruction-audit-report.sh` |
| `52df94a63cb469a29912bbd68ad9917a8bf0190645d5de5d9306e589e2aca10d` | `scripts/ci/run-layering-guards.sh` |
| `c7581d1805f8791766104d36422b8522c182d10fae906ec8c479bc4f63529969` | `scripts/ci/list-config-canisters.sh` |
| `61528bae974ed53e7b05a62626b3b086506bf70fb113006c38534f361a26e45a` | `scripts/ci/require_icp.sh` |
| `00e8aff1970563a39d9f21c52fc3d0bb80c72e2a3cea8e07ece685c8272069c6` | `scripts/ci/wasm-audit-report.sh` |
| `2500adbcaa4c0c8e3c53a2d9202caf5d44c38833b31b5c21a6f7db461a9e580a` | `tool-versions.env` |
