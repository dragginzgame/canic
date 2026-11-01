# Current handoff — Jobs qualification and CI repairs, 2026-11-01

## Branch and supplier scope

[PR #722](https://github.com/dragginzgame/canic/pull/722) retains the private
application-owned Jobs qualification from `4759beca60e0923cf91a16991541ab03c119d1ac`.
It incorporates delivered main through `2c2272b5ede0d9517a563ed63e841fd587304fd4`
(native export scoping and its metrics fixture repair) and the shared CI fixes
from [PR #726](https://github.com/dragginzgame/canic/pull/726), source
`4b333aae13165c0f49d66ce9667ff60264561724`.

The private probe adds Jobs 0.33.2 beside the already selected Timers 0.35.0.
Other supplier selections and package versions remain at the original PR graph;
this branch does not adopt the newer cohort in the separate busy Canic checkout.
Its root lock SHA-256 is
`7b593f09b93784bb08af3675723b478d9b137e93e8c21018c5b14355d61528a1`.
The probe manifest and its sole Jobs wakeup owner are explicit members of the
closed timer rosters; scanners and refusal rules remain strict. No public Jobs
mount or additional lifecycle, metadata/storage or timer authority is introduced.

## Peer and shared CI repair

Original artifacts from CI run `41294580215`, job `124816020858`, identify
standard-library panic locations under `/rust-toolchain/lib/rustlib/src/rust/`
in the embedded peer and `/rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/`
in the hosted rebuild. Rust source installation changes inlined metadata paths.
The recipe maps the physical Rust source directory to the compiler's virtual
commit path after the general sysroot mapping. Exact byte/evidence validation
and corruption refusal remain enforced.

The canonical peer from PR #726 is
`58fea38e4ae82f14d915d77b1b5b78c12b05f5128c37cefc380dfd950ab16844`,
byte-identical to that original hosted rebuild. Its provenance records PR #726's
root lock `7b68b03db743356b1a41f4c81ba0ad156d72a78c9f90b714ae8cf01f0e1fb5cb`
and normalized producer lock
`602fb81ccead06db647463374561b38e9af267d5dc255d008c4e328afc7958f1`.
Independent verification also passes against this combined Jobs branch's
selected producer. Unselected lock changes require fresh executable comparison
when the existing verifier's selected-input identity changes.

The interrupted-publication fixture gives direct owners the public workflow's
physical workspace identity through aliased parents. Tool downloads retry three
times within a 120-second retry window and 30-second connection timeout, with
HTTPS, pinned hashes and executable admission retained. The Testkit command owns
active server lifetime and cleanup; a two-hour idle timeout remains.

## Qualification and retained source

Evidence is `/tmp/canic-timer-ci-repair.Ioj4mAWX/`. The combined branch's 15 timer
inventory cases and independent canonical peer verification pass; logs are
`pr722-combined-inventory.log` and `pr722-combined-peer-verification.log`.
Inventory-target Clippy also passed at `568259c` before adding the shared repairs.

PR #726's graph separately passes all 28 selected Host and four provenance
cases, all five recovery cases under aliased TMPDIR, both runner fixtures,
strict Host/internal library/test Clippy, focused ShellCheck and formatting.
Explicit real-Wasm source-presence and producer path/release-version cases pass.
Canonical refresh and independent verification pass. The original CI ZIP and both
Wasms/provenance remain in `hosted-main/`; `peer-diagnosis.json` and
`repair-receipt.json` bind their identities. Valid red cases are
`attempt-recovery-ancestor-before.log` and `tls-retry-authorized-before.log`;
earlier setup failures keep their separate names. The original normalized
producer source is retained in `qualified-producer-source.tar`; reviewed PR #726
source is `reviewed-source.tar`. Its isolated checkout is `/tmp/ccfix.Smld3Y/`.

Original installed Jobs qualification is source `4759bec` and Wasm
`d44298e9531dff37116a6518a0ebf729530b5a1219f1c80d46ae2057e6455bfd`.
It covers intent rollback, lost-reply recovery and exact reconciliation; native
cases cover immutable payloads, independently committed post-await work and
bounded frames. Original source, Wasm, schema and server output remain at
`/tmp/ic-scheduling-improvements.VvCLTy/`, alongside this branch checkout. The
installed receipt predates these merges. The
[qualification contract](../features/jobs-qualification.md) retains the exact
governed selector and boundaries. No fresh installed Jobs result is claimed for
this combined head.

## Delivery and remaining acceptance

The branch is ready for PR review; pending 0.114.1 notes cover both outcomes.
Hosted CI qualifies the combined branch asynchronously. Keep #450/#551 open for
main delivery and applicable native macOS/long governed PocketIC acceptance;
#474 retains worker acceptance. No broad local suite, Canic main merge, release
or package publication is selected by this repair task. Historical source and
adoption details remain in the base commits' handoffs.
