# CANIC-186 unpaid reset review cancellation

Toko Miner's September 29 feedback reports that published .47 returns the unpaid
.16/.46 infrastructure review when .17 is selected. The retained desired input
and review remain authoritative even though no execution journal exists. This
prevents a fresh review from using the corrected current bootstrap reserve.

## Operator contract

`canic --environment <environment> fleet ensure <fleet> --cancel-reinstall <plan-sha256>`
now releases only an unapproved clean-reinstall infrastructure review. The command
uses the existing Fleet lock and local reinstall owner. It requires no old build,
application schema, signing identity or network call. JSON reports the cancelled
operation/plan and archive digests, with payment and deployment authority false.

Original files and shared content are archived through the existing byte-bound
snapshot owner before any removal. An external cancellation intent fences other
Fleet writers across interruption. Retry must supply the same digest; removal
checks the archived hashes and rejects changed or newly introduced files. The
lock inode stays in place. Completed cancellation can replay only while the
operation directory remains cleared; a new selected review prevents old receipt
replay from claiming that selection was cancelled.

The cancellation owner permits only known review and observation files. Any
execution journal, state, import, mint, publication or unknown side-owner file
rejects. Infrastructure inspection metadata must show no apply, terminal,
registration or effect observation attempt. Review observations remain archived
with their bounded attempt records. Cancellation never deletes paid-operation
reconciliation evidence or rebases its budgets.

After cancellation, current generation again requires complete explicit physical
inventory and qualified artifacts. The normal reinstall review performs fresh
custody and funding observation and selects the new build. Its new digest still
needs separate approval before effects. No second reset executor was added.

## Qualification

Native coverage exercises exact archival, stale-digest rejection, same-digest
receipt replay, new-selection rejection, every retained-file removal boundary,
other-writer fencing and changed/new-file rejection. A generated compiled review
also passes paired readiness without mutation, then cancellation releases its
old desired selection and restores current infrastructure forecast projection.

The targeted public-CLI PocketIC journey now starts with an actual unpaid review,
cancels it offline, regenerates and selects a different qualified release build,
and rejects a stale cancellation. After an actual install loses its reply,
cancellation must reject and preserve the execution journal before the original
digest resumes. The existing retained-ID, clearing, conservation and effect-free
terminal replay checks remain in that journey. The exact case passes on the final
source, including cancellation with a missing ICP executable and unchanged
operator Ledger balance.

Final targeted validation passes:

- `canic-host --lib`: 36 cancellation, operation-selection, readiness and generated
  retained-estate tests; three governed PocketIC cases ignored by the native filter.
- `canic-cli --lib`: three readiness and cancellation-option tests.
- `canic-cli --test subcommand_order`: both recursive help checks.
- Warning-denied Clippy for `canic-host`, `canic-cli` and
  `canic-testing-internal`, selecting all targets and all features.
- `make test-pocketic-case CASE=pic::fleet_registry::baseline::tests::completed_reset::completed_estate_reset_recovers_and_replays`:
  one exact case passed in 523.77 seconds; the governed run including compilation
  took 699 seconds.
- Scoped Rust formatting and diff whitespace checks.

Logs are retained at `/tmp/canic-186-host-final.log`,
`/tmp/canic-186-cli-final.log`, `/tmp/canic-186-help-final.log`,
`/tmp/canic-186-clippy-final.log` and `/tmp/canic-186-pocketic-final.log`.
The governed case log is
`target/test-runs/20260929T112956Z-48763.QVlD4G/1.log`.
Earlier compile attempts overlapped another session's Core/Control Plane cleanup
and were superseded after that session finished. PocketIC required local loopback
permission to start its pinned server. No broad workspace gate was run.

Toko's repository, retained review and live estate remain unchanged. Downstream
must adopt a matching published release, explicitly cancel its unpaid digest,
qualify the selected current application build, review fresh funding and obtain
approval before the live reset. This source work neither cancels that real review
nor approves its funding.
