# Application-owned Jobs qualification

The private `managed_lifecycle_probe` composes published IC Jobs 0.33.2 with
Canic's existing managed lifecycle, allocation pool, Fleet admission and shared
Timers runtime. It is a bounded application queue, not a public Jobs mount API.
Canic's paid-operation leases and journals retain their existing authority;
[#502](https://github.com/dragginzgame/canic/issues/502) still requires a separate
assessment before replacing any existing domain recovery owner.

The application admits at most eight immutable one-shot deltas, each from 1 to
1000, with a maximum one-hour scheduling delay and no automatic retry. It stores
JobRecord values beside their payloads and exact destination receipts in one
bounded Wire frame (`CJQ\x01`): at most 16,384 bytes, a 262,144-byte conservative
decode allowance, 2048 nodes and depth 32. Every retained job passes through `Job::restore`
before dispatch authority is published. Corrupt, oversized or unsupported state
refuses restoration; there is no empty-state fallback. Jobs supplies metadata
transitions only and remains independent of storage, platform and timer code.

The existing Canic memory participant grants one queue key. Its synchronous
restore classifies retained Running attempts as Uncertain and reconstructs a
volatile wakeup through the existing Timers runtime. Application-owned job IDs
bind their local sequence to the canister version at initial installation;
that epoch is retained across same-release upgrades. A dispatch commits its
Running intent before submitting the self-call. Completion reloads current
state after awaiting, matches the immutable attempt and requires the exact
stored receipt. Missing evidence never grants retry authority.

Endpoints require both Fleet admission and the fixture's application owner.
The destination separately requires the canister itself as caller; an Attempt
is correlation data, not authentication. The counted self-call destination is
an installed IC message boundary, not a production external-effect adapter.
Private fault endpoints can trap after intent write before transport, trap
after a committed destination reply, or corrupt the retained frame. They belong
only to this private test canister.

Run the installed proof with:

```sh
make test-pocketic-case CASE=pic::jobs::managed_jobs_queue_preserves_attempts_across_upgrade_and_lost_reply
```

The canonical runner owns selected tools, server lifetime, serial execution and
nonempty test admission. The case covers Fleet/application refusal, message-wide
intent rollback, same-release reconstruction, lost-response recovery without
redelivery, stale and duplicate disposition, destination caller refusal and
invalid restoration. Native model tests cover payload mismatch, independently
committed work across an await boundary and full-capacity frame admission.
These focused checks do not establish production deployment or the complete
Canic release matrix.
