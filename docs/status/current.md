# Current handoff — CI fixture repairs, 2026-11-01

## Source and scope

This isolated contribution starts at delivered main `2c2272b5ede0d9517a563ed63e841fd587304fd4`
(Canic 0.114.0, including #723/#725). It prepares the remaining #450/#551
repairs for PR review. Package versions, all three dependency catalogs/locks and
Shared Tooling 0.27.23 are unchanged. The separate normal Canic checkout and its
newer dependency adoption remain untouched. Prior source/adoption history is in
[the base handoff](https://github.com/dragginzgame/canic/blob/2c2272b5ede0d9517a563ed63e841fd587304fd4/docs/status/current.md).

## Repairs and exact peer diagnosis

The original artifacts from CI run `41294580215`, job `124816020858`, show
standard-library panic locations under `/rust-toolchain/lib/rustlib/src/rust/`
in the embedded peer and `/rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/`
in the hosted rebuild. Rust source installation changes inlined metadata paths.
The recipe now maps the physical Rust source directory to the compiler's virtual
commit path after the general sysroot mapping. The verifier still compares exact
bytes and rejects corrupt evidence or artifacts.

Canonical regeneration produces SHA-256
`58fea38e4ae82f14d915d77b1b5b78c12b05f5128c37cefc380dfd950ab16844`,
identical byte for byte to that original hosted rebuild. Root lock SHA-256 is
`7b68b03db743356b1a41f4c81ba0ad156d72a78c9f90b714ae8cf01f0e1fb5cb`;
the normalized producer lock remains
`602fb81ccead06db647463374561b38e9af267d5dc255d008c4e328afc7958f1`.
Structured provenance and the Wasm are regenerated together.

The interrupted-publication fixture gives direct owners the same physical
workspace identity as the public workflow, including through aliased parents.
Tool downloads retry three times within a 120-second retry window and a
30-second connection timeout, retaining HTTPS, hashes and executable admission.
The workspace runner gives its command active server custody and cleanup,
retaining a two-hour idle timeout. Both runner fixtures check actual server
arguments, failure capture, continuation, cancellation and cleanup.

## Focused evidence and remaining acceptance

Evidence/source: `/tmp/canic-timer-ci-repair.Ioj4mAWX/` and `/tmp/ccfix.Smld3Y/`.
The original CI ZIP and both original Wasms/provenance remain in `hosted-main/`;
`peer-diagnosis.json` records their identities. Valid red reproductions are
`attempt-recovery-ancestor-before.log` (the exact Integrity refusal) and
`tls-retry-authorized-before.log` (one connection instead of four). Earlier
setup failures remain separately named and are not those reproductions.

All 28 selected Host cases and four native provenance cases pass. All five
recovery cases also pass under aliased TMPDIR. The explicit Rust-source presence
and real peer path/release-version reproduction cases pass. Canonical refresh
and independent verification pass; strict Host/internal library/test Clippy,
focused ShellCheck, formatting and both runner fixture suites pass. Original
normalized producer source is retained in `qualified-producer-source.tar`.

The scoped contribution is ready for PR review. Required hosted native macOS and
long governed PocketIC acceptance remain with #450/#551 and #474. No broad local
suite, Canic main merge, release or package publication is claimed. The numbered
0.114.1 notes describe this coherent CI/tooling batch; release authority remains
separate. PR #722 separately registers its private Jobs probe in the closed timer
rosters and retains its original installed qualification.
