# Audit Method Fingerprints v1

- snapshot_status: `prepared_uncommitted`
- prepared_at: `2026-07-14`
- release_anchor: `v0.91.6`
- source_commit_full: `5f7a89f9b966ebf2755d5630ddcba0cdf968ebb1`
- source_tree_hash: `8170017a23bad302a87e4277050d720bfc3c1834`
- baseline_product_tree_hash: `8fce43e41ce430d9b505e19f8d596ed440b291d4c6ecb19c4a1cfdf71656a9b6`
- frozen_method_commit: `pending maintainer commit`

These SHA-256 identities describe the prepared Phase B method content in the
current worktree. They are not a frozen audit authority until the maintainer
commits the complete method-hardening slice. After that commit, record its full
identity here or in the 0.92 tracker before starting the product baseline.

A method change after freeze must increment the affected method version and
follow the defect/invalidation protocol in [AUDIT-HOWTO.md](AUDIT-HOWTO.md).

## Active Definition Identities

| Audit ID | Version | SHA-256 | Definition |
| --- | --- | --- | --- |
| `CANIC-AUTH-AUDIENCE-001` | `1` | `9d28324a6101e94ba964e8d8478909323e16e83bc0134975ab37f69030602448` | `docs/audits/recurring/invariants/audience-target-binding.md` |
| `CANIC-AUTH-EQUIVALENCE-001` | `1` | `3339c221ec11706c00b4dbe2d9d4be116441b1f0572b5e7060a23a74199f89d6` | `docs/audits/recurring/invariants/auth-abstraction-equivalence.md` |
| `CANIC-AUTH-BOUNDARY-001` | `1` | `f5383b05617c0f5c3df5f9a2e230e200925c4aa0a16586787d32ac6bab427b98` | `docs/audits/recurring/invariants/canonical-auth-boundary.md` |
| `CANIC-AUTH-CAPABILITY-001` | `1` | `4dd5127b43716dc8a89bef6148b76856794f7c5e158f536a7737ddceee0f1c55` | `docs/audits/recurring/invariants/capability-scope-enforcement.md` |
| `CANIC-AUTH-REPLAY-001` | `1` | `2a4726ca049194175f1230c9de54442746d462d460f2adea77b8b1df57f8868c` | `docs/audits/recurring/invariants/expiry-replay-single-use.md` |
| `CANIC-AUTH-SUBJECT-001` | `1` | `8af2c270ba89aae9715e6047afa908b1812865c5949f0f1da6441539fdee4475` | `docs/audits/recurring/invariants/subject-caller-binding.md` |
| `CANIC-AUTH-TRUST-001` | `1` | `8676d779cca173e3cb5cde61e29635eb14732dde6cf89fbd06b08c5ab19c5db7` | `docs/audits/recurring/invariants/token-trust-chain.md` |
| `CANIC-LIFECYCLE-001` | `1` | `c3b99716e67d6fc83bd33a9875ef93bb5eb7e9d9756fcfbce92c5030046c48cd` | `docs/audits/recurring/system/bootstrap-lifecycle-symmetry.md` |
| `CANIC-BUILD-INTEGRITY-001` | `1` | `57f0a380b1722927498ddd0f41b5490e8726cab943c2d3df02ecac73897a5311` | `docs/audits/recurring/system/build-integrity.md` |
| `CANIC-CAPABILITY-SURFACE-001` | `1` | `d7de4f8b7115b5e4861bde23aaebe9b2ddee3c83a07f7730b61122b3f3fff898` | `docs/audits/recurring/system/capability-surface.md` |
| `CANIC-CHANGE-FRICTION-001` | `1` | `00646b257428623f7ef4efce4dffdcd93f3bdc75cd7e2dbc02faad32cb2ce8d6` | `docs/audits/recurring/system/change-friction.md` |
| `CANIC-COMPLEXITY-001` | `1` | `47bc07614dae02c725cb288f8f8285ac4aa52a116e6480e3d5a8a9e5eefc4d20` | `docs/audits/recurring/system/complexity-accretion.md` |
| `CANIC-DEPENDENCY-001` | `1` | `71be0c1d68cc573bc7c17232709b3a576d9cba903eaa9062665ae9bc71a58194` | `docs/audits/recurring/system/dependency-hygiene.md` |
| `CANIC-DUPLICATION-001` | `1` | `c4b2b2828f551a5419de394d442ecb04932900d7b15665177a3c8529ee340262` | `docs/audits/recurring/system/dry-consolidation.md` |
| `CANIC-INSTRUCTION-001` | `1` | `f90bbd1443ac5acdcc69ad256eaef8877955a9219025f65c6255c6fdd7bf2805` | `docs/audits/recurring/system/instruction-footprint.md` |
| `CANIC-LAYERING-001` | `1` | `86270ae481556a8f5b544d71529d3b324cf5dbf7af7267100a6a74976eacfc49` | `docs/audits/recurring/system/layer-violations.md` |
| `CANIC-STRUCTURE-001` | `1` | `ca370a2c910c4d9d3755af74099c6d5715086d8b1ff226c29a40c77c5ee9f58e` | `docs/audits/recurring/system/module-structure.md` |
| `CANIC-PUBLISH-001` | `1` | `8e2eff6ac0c60c9903cd68f6354f7536636a987fd437306e851643464bdef884` | `docs/audits/recurring/system/publish-surface.md` |
| `CANIC-RELEASE-INTEGRITY-001` | `1` | `3f6b87b30a3c1f9c80803a8be5d45292e73217d260ea435a956bd05f10d63438` | `docs/audits/recurring/system/release-integrity.md` |
| `CANIC-AUTH-ORDERING-001` | `1` | `bf5e5a5bb0eb22f22cf30098bc881838c7908209f45c26d0296f546ac55e1802` | `docs/audits/recurring/system/security-boundary-ordering.md` |
| `CANIC-WASM-001` | `1` | `1ed32dd340d10135e899cda5794046d68e1e66ea89da9d6910aa4ca4e958a064` | `docs/audits/recurring/system/wasm-footprint.md` |
| `CANIC-MODULE-SURFACE-001` | `2.0` | `404a359b4448ea7288055f0444e3178ae972f4eb7e1a0814aa693ce67df59030` | `docs/audits/modular/module-surface-hardening.md` |

## Executable Composite Identities

- `CANIC-INSTRUCTION-001/v1`: `c79f7027f3629bcbe4dbf4680005d3a9b37104c7ba6d4956a5a3c789c5b5cfab`
- `CANIC-WASM-001/v1`: `e8c58213d9301d66d4adac4bd92e4aa702fd887b8adb55e2e602a70f29e9c505`

The runtime runners calculate these composites from their definition, script,
and executable fixture inputs and record the resulting value in each run.

## Governance And Executable Input Identities

| SHA-256 | Input |
| --- | --- |
| `113b55a44dda0e0780fc3dca5743eaec3949c99456594de5fe10e2a3469487c2` | `crates/canic-host/examples/build_artifact.rs` |
| `9bde640ebb6f70c0649a2ef862d32e29b0af20fb0a4a52e3c6a0fc74fac488c2` | `crates/canic-tests/tests/instruction_audit.rs` |
| `0989f0a8068efe462d293c325ca150183405795f3ee86462d59b7be9b84eb159` | `crates/canic-tests/tests/instruction_audit_support/estimates/mod.rs` |
| `1815e36925d0c643aa8c43694fa7f7725a9fb683ac76816b12906893999f7ef0` | `crates/canic-tests/tests/instruction_audit_support/execution.rs` |
| `dfd2a7df75ddbb6734ac4d8236d6feffca2621eff9c55a7340e909b1c4bc87f5` | `crates/canic-tests/tests/instruction_audit_support/mod.rs` |
| `768fbc005c555b601cc00a0f547a02b36d666f1dd23f7b09243e3f0a109ec6ff` | `crates/canic-tests/tests/instruction_audit_support/report.rs` |
| `c25f4aac1c8077fc1a820f963ebc309cb952827cae4c7130a7dd6ab6bae0804a` | `crates/canic-tests/tests/instruction_audit_support/scenarios.rs` |
| `3a4bd3c46d25fe3c4e16d7b4e1dba916920e12780bf569e5701c089a5c0fa053` | `docs/audits/AUDIT-HOWTO.md` |
| `5d7e2f15bdd195c63276d37beffce6a88fea2b593abfd7a1d6ede7274e6aa3c6` | `docs/audits/META-AUDIT.md` |
| `c87bd8fa6c1534c0302cad1d989d99ad9fa35db5a008904b65c646cf8895cdaa` | `docs/audits/METHODS.md` |
| `5fee9fc12be72d84a64137f4f3467833d895b611ea899dce91c34e89a56ee472` | `docs/audits/product-tree-scope-v1.md` |
| `a5eee1b85b1d54bfc23285e58360690b3bc09c0c1aece7e9440a8b029ec00475` | `docs/audits/retired-methods.md` |
| `ac7ab348d0e9a18df9def45f89f1c403f7c23e523eaf58da03b5099fb2634417` | `scripts/ci/audit-product-tree-hash.sh` |
| `24b7b8018e6d74813e947922a4e271481715b79e0a91f12fa86f4a6a8030df4f` | `scripts/ci/check-audit-method-catalog.sh` |
| `de2f59b074377f5f701e9e27d693465345918b8f46f40e8e986657f3d26947b2` | `scripts/ci/instruction-audit-report.sh` |
| `c7581d1805f8791766104d36422b8522c182d10fae906ec8c479bc4f63529969` | `scripts/ci/list-config-canisters.sh` |
| `61528bae974ed53e7b05a62626b3b086506bf70fb113006c38534f361a26e45a` | `scripts/ci/require_icp.sh` |
| `252ad7c3d0393f407b199d1c6e38eda9147ab72ecb4b43f64c81f8cc5c6c0b61` | `scripts/ci/wasm-audit-report.sh` |
