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
| `CANIC-AUTH-AUDIENCE-001` | `3` | `10a790fd838059feea550533775f84fdb6e77fdc133a0c0306cc492ced321fe2` | `docs/audits/recurring/invariants/audience-target-binding.md` |
| `CANIC-AUTH-EQUIVALENCE-001` | `2` | `de18b672622fdc8ec41882921ca6a11d3d7872e5d2df7754b855a45d6f639942` | `docs/audits/recurring/invariants/auth-abstraction-equivalence.md` |
| `CANIC-AUTH-BOUNDARY-001` | `1` | `3fdc297ab141ff1bddd26981a4a25aaf7e727ff63146054b626e0e054500d822` | `docs/audits/recurring/invariants/canonical-auth-boundary.md` |
| `CANIC-AUTH-CAPABILITY-001` | `1` | `1d6f51308f6c5d250f6931b3f3d98eb321aac2c24bbc794bfa3cd4742bbfffe4` | `docs/audits/recurring/invariants/capability-scope-enforcement.md` |
| `CANIC-AUTH-REPLAY-001` | `2` | `743b9fcc18e37be029e12fa9db2a5fa5ffb8d4258c484739a5b0e73e43632b6d` | `docs/audits/recurring/invariants/expiry-replay-single-use.md` |
| `CANIC-AUTH-SUBJECT-001` | `1` | `8af2c270ba89aae9715e6047afa908b1812865c5949f0f1da6441539fdee4475` | `docs/audits/recurring/invariants/subject-caller-binding.md` |
| `CANIC-AUTH-TRUST-001` | `2` | `150d075dd9749d6c2b849837ce422753fa1a438a8656e50eac0370f555e769c5` | `docs/audits/recurring/invariants/token-trust-chain.md` |
| `CANIC-LIFECYCLE-001` | `4` | `1e5cf3dbc8efe09b69fb8a8722cae85b5cbcb40547c747388cbef6165a955b4f` | `docs/audits/recurring/system/bootstrap-lifecycle-symmetry.md` |
| `CANIC-BUILD-INTEGRITY-001` | `2` | `e75c8fdc54f090bd901482f50c88e2b6272830d1425f24d7165904c1b206a94b` | `docs/audits/recurring/system/build-integrity.md` |
| `CANIC-CAPABILITY-SURFACE-001` | `2` | `91e61f3385882d108b8541e31715b1ee1e126299f7ad64890c979043a9d7c759` | `docs/audits/recurring/system/capability-surface.md` |
| `CANIC-CHANGE-FRICTION-001` | `4` | `72b063c432d9521bdbb565b3176dcb06909d7586b9a5dc7b578b22c437ec6725` | `docs/audits/recurring/system/change-friction.md` |
| `CANIC-COMPLEXITY-001` | `4` | `e3ea1d265170a7cfffe185c135d7a8b7e36a1a268cdca4380258dfa5b9e854db` | `docs/audits/recurring/system/complexity-accretion.md` |
| `CANIC-DEPENDENCY-001` | `3` | `e0e4474467664e46a56243ccf41c5b29b9786a0052a9ccf9117e499226384d40` | `docs/audits/recurring/system/dependency-hygiene.md` |
| `CANIC-DUPLICATION-001` | `3` | `5a417c934b44b2d1e3f31980aebcfa9c6b396cda5440ee64c4f917c964418021` | `docs/audits/recurring/system/dry-consolidation.md` |
| `CANIC-INSTRUCTION-001` | `3` | `515dbe0d4957dd4705f3e21ce88cca1a34b8c4cc86ccd34483479abedf5c2ab8` | `docs/audits/recurring/system/instruction-footprint.md` |
| `CANIC-LAYERING-001` | `2` | `a4c71532e85f3ea0c5f1802478b15f444d78eae3540dc35b96b77b04231503bc` | `docs/audits/recurring/system/layer-violations.md` |
| `CANIC-STRUCTURE-001` | `3` | `635df6921e39cb0a780252248f4d6961d4f33d2a72e5b60535a872c2aa9846e8` | `docs/audits/recurring/system/module-structure.md` |
| `CANIC-PUBLISH-001` | `2` | `5c6f38395c02454e861115c6a06638944d7841b331f1758c247601b69675e341` | `docs/audits/recurring/system/publish-surface.md` |
| `CANIC-RELEASE-INTEGRITY-001` | `2` | `4eb3d6f45df9ee9b64df901dc891ebfd37e8aaa1ede45a02eadbc3018b8a6609` | `docs/audits/recurring/system/release-integrity.md` |
| `CANIC-AUTH-ORDERING-001` | `1` | `2619b50394d35381cb2be0d124868f8249218bf41591fad2713730e20f266b87` | `docs/audits/recurring/system/security-boundary-ordering.md` |
| `CANIC-WASM-001` | `6` | `0c8487a989dadaae03cba0437545a2ea236274607622ff3d74623696c1ebe797` | `docs/audits/recurring/system/wasm-footprint.md` |
| `CANIC-MODULE-SURFACE-001` | `2.4` | `55490f62bd93b81d93d60168c05ea8de9797d081e9dbf8f7154ba0787db4e422` | `docs/audits/modular/module-surface-hardening.md` |

The [September 29 correction report](reports/2026-09/2026-09-29/audit-method-correction.md)
records the DRY and module-hardening revision scope and affected-result limits.

The October 2 maintainer-directed removal of dedicated secret scanning advances
release integrity to revision 2. Earlier scanner results remain historical
evidence; revision 2 reviews credential handling without requiring a scanner.

## Superseded Definition Identities


| Audit ID | Version | SHA-256 | Definition | Superseded by |
| --- | --- | --- | --- | --- |
| `CANIC-DUPLICATION-001` | `2` | `7d2369e2cafa2f10128c97044c0853d56dc9a58dd912277ba5dcdcc4e82169dc` | `docs/audits/historical/shared-adoption-20261006/dry-consolidation.md` | `CANIC-DUPLICATION-001/v3` |
| `CANIC-COMPLEXITY-001` | `3` | `4e20ddfbf9b33e5cc9ca2b6535aaeab0b5f30ffbf053b06b55f4d9a3c0831a2a` | `docs/audits/historical/shared-adoption-20261006/complexity-accretion.md` | `CANIC-COMPLEXITY-001/v4` |
| `CANIC-CHANGE-FRICTION-001` | `3` | `c23af3f87879a3223893b11ef4568340789a291d4c98b33f83d7fab5efa67cda` | `docs/audits/historical/shared-adoption-20261006/change-friction.md` | `CANIC-CHANGE-FRICTION-001/v4` |
| `CANIC-STRUCTURE-001` | `2` | `d8ad8f06492d8a37e4f9b9632b83714b4a88125989d44ed7e825e93c2957dd49` | `docs/audits/historical/shared-adoption-20261006/module-structure.md` | `CANIC-STRUCTURE-001/v3` |
| `CANIC-MODULE-SURFACE-001` | `2.3` | `b4b6e300b70ae8f2899b36b99b287aa2332ab1c8775514a99954f991c472a3f4` | `docs/audits/historical/shared-adoption-20261006/module-surface-hardening.md` | `CANIC-MODULE-SURFACE-001/v2.4` |
| `CANIC-RELEASE-INTEGRITY-001` | `1` | `3f6b87b30a3c1f9c80803a8be5d45292e73217d260ea435a956bd05f10d63438` | `docs/audits/recurring/system/release-integrity.md` | `CANIC-RELEASE-INTEGRITY-001/v2` |
| `CANIC-MODULE-SURFACE-001` | `2.2` | `982c79eeeb88f32002b47810280109bf273bf39a2d80b1cd5205010929b129ff` | `docs/audits/modular/module-surface-hardening.md` | `CANIC-MODULE-SURFACE-001/v2.3` |
| `CANIC-MODULE-SURFACE-001` | `2.1` | `e3cb15bba0909fff96075206d6a4780b2a74f31a98c95ebd143023d0e73e9835` | `docs/audits/modular/module-surface-hardening.md` | `CANIC-MODULE-SURFACE-001/v2.2` |
| `CANIC-DUPLICATION-001` | `1` | `c4b2b2828f551a5419de394d442ecb04932900d7b15665177a3c8529ee340262` | `docs/audits/recurring/system/dry-consolidation.md` | `CANIC-DUPLICATION-001/v2` |
| `CANIC-MODULE-SURFACE-001` | `2.0` | `404a359b4448ea7288055f0444e3178ae972f4eb7e1a0814aa693ce67df59030` | `docs/audits/modular/module-surface-hardening.md` | `CANIC-MODULE-SURFACE-001/v2.1` |
| `CANIC-PUBLISH-001` | `1` | `8e2eff6ac0c60c9903cd68f6354f7536636a987fd437306e851643464bdef884` | `docs/audits/recurring/system/publish-surface.md` | `CANIC-PUBLISH-001/v2` |
| `CANIC-STRUCTURE-001` | `1` | `ca370a2c910c4d9d3755af74099c6d5715086d8b1ff226c29a40c77c5ee9f58e` | `docs/audits/recurring/system/module-structure.md` | `CANIC-STRUCTURE-001/v2` |
| `CANIC-DEPENDENCY-001` | `2` | `ad7b459667545ec5b3adfd33a614803e2c11fa77a28af873392a1d3344333f6f` | `docs/audits/recurring/system/dependency-hygiene.md` | `CANIC-DEPENDENCY-001/v3` |
| `CANIC-LIFECYCLE-001` | `3` | `f35101e6f877dadcba678d2b2db5695b3ffdecd23d6f6eb4e737200b2b1bf405` | `docs/audits/recurring/system/bootstrap-lifecycle-symmetry.md` | `CANIC-LIFECYCLE-001/v4` |
| `CANIC-WASM-001` | `5` | `a51d17f9e744bf6452005e543ba6f6e77a3c2257d385aa2ad58f1acad2439c19` | `docs/audits/recurring/system/wasm-footprint.md` | `CANIC-WASM-001/v6` |
| `CANIC-WASM-001` | `4` | `85dd9bd380f0c7b8efe1a50ab574f144ca86ce9c71a24406b29a708ff522e3f5` | `docs/audits/recurring/system/wasm-footprint.md` | `CANIC-WASM-001/v5` |
| `CANIC-CHANGE-FRICTION-001` | `2` | `5f4377f00907f36f59388f797f210bdfed9398832f983529cdccd4bd747d2ab6` | `docs/audits/recurring/system/change-friction.md` | `CANIC-CHANGE-FRICTION-001/v3` |
| `CANIC-COMPLEXITY-001` | `2` | `76bb53a536f252348567d32fd0779a40347e54c254c5fd726207253dcd069fce` | `docs/audits/recurring/system/complexity-accretion.md` | `CANIC-COMPLEXITY-001/v3` |
| `CANIC-WASM-001` | `3` | `9747666aeee64eff0af26b92f90bd1ccf12f318023780aa7148fdd55fc29d745` | `docs/audits/recurring/system/wasm-footprint.md` | `CANIC-WASM-001/v4` |
| `CANIC-LIFECYCLE-001` | `2` | `89202cda4ed08ced7d9f70fd98ff1e68e04bc1877a528a489060242f5acd8059` | `docs/audits/recurring/system/bootstrap-lifecycle-symmetry.md` | `CANIC-LIFECYCLE-001/v3` |
| `CANIC-WASM-001` | `2` | `e33fc36ee904fa6a9af8c7aa399a94b98c441e25fe6590ac1548c548ba2f3ffb` | `docs/audits/recurring/system/wasm-footprint.md` | `CANIC-WASM-001/v3` |
| `CANIC-AUTH-AUDIENCE-001` | `2` | `bfe780a3e93f0511f9c7bbbbf7cf84dee40b23d1456ab68fe12122a671b30a5c` | `docs/audits/recurring/invariants/audience-target-binding.md` | `CANIC-AUTH-AUDIENCE-001/v3` |
| `CANIC-AUTH-AUDIENCE-001` | `1` | `9d28324a6101e94ba964e8d8478909323e16e83bc0134975ab37f69030602448` | `docs/audits/recurring/invariants/audience-target-binding.md` | `CANIC-AUTH-AUDIENCE-001/v2` |
| `CANIC-AUTH-EQUIVALENCE-001` | `1` | `7784678597a51e59b521aaefc15806f20e2c03bc2ac49ba91b8c824f88d2461b` | `docs/audits/recurring/invariants/auth-abstraction-equivalence.md` | `CANIC-AUTH-EQUIVALENCE-001/v2` |
| `CANIC-AUTH-REPLAY-001` | `1` | `2a4726ca049194175f1230c9de54442746d462d460f2adea77b8b1df57f8868c` | `docs/audits/recurring/invariants/expiry-replay-single-use.md` | `CANIC-AUTH-REPLAY-001/v2` |
| `CANIC-AUTH-TRUST-001` | `1` | `5cb6143aee5fec2e4ff5f7c8649fc70e120f5233be19b913e90cff9ae38341d6` | `docs/audits/recurring/invariants/token-trust-chain.md` | `CANIC-AUTH-TRUST-001/v2` |
| `CANIC-LIFECYCLE-001` | `1` | `df4fd68b78e1fab92bf17f85ac6adec235e978c52783f5151b2c57411b0e4913` | `docs/audits/recurring/system/bootstrap-lifecycle-symmetry.md` | `CANIC-LIFECYCLE-001/v2` |
| `CANIC-BUILD-INTEGRITY-001` | `1` | `57f0a380b1722927498ddd0f41b5490e8726cab943c2d3df02ecac73897a5311` | `docs/audits/recurring/system/build-integrity.md` | `CANIC-BUILD-INTEGRITY-001/v2` |
| `CANIC-CAPABILITY-SURFACE-001` | `1` | `d7de4f8b7115b5e4861bde23aaebe9b2ddee3c83a07f7730b61122b3f3fff898` | `docs/audits/recurring/system/capability-surface.md` | `CANIC-CAPABILITY-SURFACE-001/v2` |
| `CANIC-CHANGE-FRICTION-001` | `1` | `00646b257428623f7ef4efce4dffdcd93f3bdc75cd7e2dbc02faad32cb2ce8d6` | `docs/audits/recurring/system/change-friction.md` | `CANIC-CHANGE-FRICTION-001/v2` |
| `CANIC-DEPENDENCY-001` | `1` | `71be0c1d68cc573bc7c17232709b3a576d9cba903eaa9062665ae9bc71a58194` | `docs/audits/recurring/system/dependency-hygiene.md` | `CANIC-DEPENDENCY-001/v2` |
| `CANIC-INSTRUCTION-001` | `2` | `610ee7acc1eb4675d19d1495ec8cccdf8132bc81411aa8d0196f6fe46308d243` | `docs/audits/recurring/system/instruction-footprint.md` | `CANIC-INSTRUCTION-001/v3` |
| `CANIC-INSTRUCTION-001` | `1` | `f90bbd1443ac5acdcc69ad256eaef8877955a9219025f65c6255c6fdd7bf2805` | `docs/audits/recurring/system/instruction-footprint.md` | `CANIC-INSTRUCTION-001/v2` |
| `CANIC-LAYERING-001` | `1` | `86270ae481556a8f5b544d71529d3b324cf5dbf7af7267100a6a74976eacfc49` | `docs/audits/recurring/system/layer-violations.md` | `CANIC-LAYERING-001/v2` |
| `CANIC-WASM-001` | `1` | `1ed32dd340d10135e899cda5794046d68e1e66ea89da9d6910aa4ca4e958a064` | `docs/audits/recurring/system/wasm-footprint.md` | `CANIC-WASM-001/v2` |

## Executable Composite Identities

- `CANIC-INSTRUCTION-001/v1`: `c79f7027f3629bcbe4dbf4680005d3a9b37104c7ba6d4956a5a3c789c5b5cfab`
- `CANIC-INSTRUCTION-001/v2`: `f48e99ac68c74e5c967b65be26df9fe93d4147725002696b7947281c194dc6b1`
- `CANIC-INSTRUCTION-001/v3`: `f0abfa64b51c076d55fb105825267be8e18fe881709c28c7c0d7c36bec2c6626`
- `CANIC-WASM-001/v1`: `e8c58213d9301d66d4adac4bd92e4aa702fd887b8adb55e2e602a70f29e9c505`
- `CANIC-WASM-001/v2`: `7e6bd9d57f7791ff2a6fa4b3a2d2cbd153f56d1c9a5c4269a869ff72a7675f18`
- `CANIC-WASM-001/v3`: `0eb95225272ca531e4104454d24c2e2fa26d4fc26cfefbe4d151383ad0c6b6a6`
- `CANIC-WASM-001/v4`: `25fa2b50f14838c8debfd77d274b55ce9e0edb1befc4e14cea64c665908f7ce4`
- `CANIC-WASM-001/v5`: `e5fea20658708141f9ec95545536c73306fe725f5410567a045cb8ce5df8cc27`
- `CANIC-WASM-001/v6`: `726e629bd67983acd2a7e4d42c275c282953b1ffef0e3236435c3c0817ef0d01`

The runtime runners calculate these composites from their definition, script,
and executable fixture inputs and record the resulting value in each run.

## Governance And Executable Input Identities

| SHA-256 | Input |
| --- | --- |
| `c4a158532e59c17d16b0fe161eb30bc15c4bacd3a6f41561f6c8ae88b678682c` | `crates/canic-host/examples/build_artifact.rs` |
| `9bde640ebb6f70c0649a2ef862d32e29b0af20fb0a4a52e3c6a0fc74fac488c2` | `crates/canic-tests/tests/instruction_audit.rs` |
| `f5f307560315beed0afc1f4eabc7257bbbb1bc0e1d84dfb417565d2f893967fa` | `crates/canic-tests/tests/instruction_audit_support/estimates/mod.rs` |
| `55c03bb123d4b6e85eb94f36ad8cad658199d1500ca6423ea27ad17943d23bf9` | `crates/canic-tests/tests/instruction_audit_support/execution.rs` |
| `a52d4f1c1466c9705a5fee8bdc09317d3d06d4b25890bb4dd26e54d08de0fa66` | `crates/canic-tests/tests/instruction_audit_support/mod.rs` |
| `9951d8bd46dcd151fb21462429c6874a1dbc2cce295df35aff2ed9838478fd71` | `crates/canic-tests/tests/instruction_audit_support/report.rs` |
| `f831745b25e82adbe5a231ba622c17f4717ce79f427f94734b47fc9f421959a9` | `crates/canic-tests/tests/instruction_audit_support/scenarios.rs` |
| `bff7fe8449b05cca312ce81e9628fdf6277c00f346a42573df426fc628e610ec` | `docs/audits/AUDIT-HOWTO.md` |
| `140cf49f47755c7a036662e88fbbdc67e7de64feaf88ef548fd43a5ded100dea` | `docs/audits/META-AUDIT.md` |
| `e9333a68cb8dd8eb19178249828348e0f38e057a7ef42add350697dec00b0257` | `docs/audits/METHODS.md` |
| `ea2c06b003464d6be8f458e07090082ac39f611b1c1907ff2d48ee7f9702e3c7` | `docs/audits/mandatory-trace-protocol.md` |
| `5fee9fc12be72d84a64137f4f3467833d895b611ea899dce91c34e89a56ee472` | `docs/audits/product-tree-scope-v1.md` |
| `a5eee1b85b1d54bfc23285e58360690b3bc09c0c1aece7e9440a8b029ec00475` | `docs/audits/retired-methods.md` |
| `7fda1aecc7a06d7d985b8ef62c338b75ce95ae1adc89d8d81f9a74a8e0377989` | `docs/audits/fixtures/layering/allowed-import.txt` |
| `9064f8aaf36c2f68626d28c98b389f5e8e7bc728281047f0435f26411638000a` | `docs/audits/fixtures/layering/forbidden-direct-import.txt` |
| `4135092a556dbcabfe895b27b6666f5d54f041526ee8e040c2777c9b875f0437` | `docs/audits/fixtures/layering/forbidden-grouped-import.txt` |
| `c4db862c85b44585f88562030560907a6ef1a8a5926fd4682445a10a8be41b6e` | `docs/audits/fixtures/layering/forbidden-nested-grouped-import.txt` |
| `42b440e2fc4d47394b7b8b99a69d1c05a91e79aa7eb3cd8872aa0520746078d3` | `docs/audits/fixtures/change-friction-v2-sample.tsv` |
| `77ceab2352ccce278aba0c698bc16eabb7f3dadf66256c8bf6db957d64863371` | `docs/audits/scripts/measure-change-friction-v3.sh` |
| `7ffa84f792ef90208bbfcdd11d386ddc5e482521ba314e71a17f6070a4352c5c` | `docs/audits/scripts/measure-change-friction-v2.sh` |
| `f08e975e3b68e483d736a62b376e2ec83c9038d0901c688c2a2f1d69d0f55aaa` | `docs/audits/scripts/measure-complexity-v3.sh` |
| `4ff697d1ed68db19bca8810f609ea40547486a2174e81271312828ef034ca7c8` | `docs/audits/scripts/measure-complexity-v2.sh` |
| `8f4a46a26e56b845290c3adc4994826b8a10084c97a2c68579ca60038f8e1be8` | `docs/audits/scripts/run-nonempty-cargo-test.sh` |
| `ac7ab348d0e9a18df9def45f89f1c403f7c23e523eaf58da03b5099fb2634417` | `scripts/ci/audit-product-tree-hash.sh` |
| `8d15916d94674d4fef698ea21d1d5dab3e770c84fae709964fb8ca341dc3931a` | `scripts/ci/check-audit-method-catalog.sh` |
| `d6bc808dd8ef0b590dad612ca38a8b11ccdd18737165cc4b847368c4e187fd57` | `scripts/ci/check-release-integrity-contract.sh` |
| `b99c45e6033b7537a62af5175ef874f35709dc7af12cae25f3f494921705f72b` | `scripts/ci/instruction-audit-report.sh` |
| `93ac1f3c77d9d58009cae84c4570e51306a077a207262c9316aea6f2fdae122c` | `scripts/ci/run-layering-guards.sh` |
| `d4923f84ce221736e71619a148069f7e77b42e1270689224716fa28ede7e4542` | `scripts/ci/list-config-canisters.sh` |
| `f2d37cdd60f85a3e7be5d4f57694cf62122f01c6576a3397e76c73b4f93cc216` | `scripts/ci/require_icp.sh` |
| `c8ac347d3c8d7ecc0beb8f2d3a65dc7a2ae56bceaee547b4e94f464940f85933` | `scripts/ci/wasm-audit-report.sh` |
| `a85ca9acd295bc795bdaa773ac2d25267e5f22e5d88fb9bdc752a9fdd04960ce` | `tool-versions.env` |
| `4c981184847462ab3c01cac087e28309f8e054253fd95c7bbe56f643877aa708` | `scripts/ci/verify-file-checksum.sh` |
