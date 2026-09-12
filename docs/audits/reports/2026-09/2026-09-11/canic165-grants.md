# CANIC-165 initial grants and protected target assignments

Root derives initial fixture grants from verified Component and child
installation intent, and the installed target retains the exact source
assignment. This qualifies fresh source access and assignment delivery.
CANIC-165 remains open; fixture-bearing Fleet generation stays disabled.

## Maintained behavior

`RootStoreBootstrapResponse.fixtures` carries the protected manifest's selected
descriptors through bootstrap, live status and the host's expected receipt.
The installation owner derives the selected role's content identity and binds
it to the exact managed target, installation ID and release build.

The non-root payload carries the Store, descriptor and revision-1 grant. Ops
persists that assignment inside the existing protected Component runtime record.
Root checks installed module, controller, managed binding, runtime installation
and assignment before granting access. Directory preparation, activation and
synchronization projections preserve the assignment. Native status and protected
record admission reject substituted descriptors, target authority, installation,
release, content and grant revisions. Init also rejects another Principal's
assignment.

Publication and consumers share one core content/chunk verifier. Store admission
still checks its complete command-envelope bound. Payload/status assignments are
boxed to keep existing async futures below the warning threshold; the wire
contract is unchanged by boxing. The maintained current schema is a pre-1.0 hard
cut and adds no application cursor or mutable readiness flag.

The existing Installed phase owns grant continuation. After Store awaits, Root
rechecks retained allocation and installation effect plus Fleet authority,
subnet, Root, Store and release. Initial issuance uses the Store's revision-zero
precondition. Only the exact enabled revision-1 grant reconciles a lost response;
revoked, conflicting, zero or later revisions reject. Grant issuance requires
neither global Root activation nor application data readiness.

## Qualification

- [Native checks](canic165-grants-evidence/native-tests.log): 47 cases pass:
  eight control-plane, nineteen core and twenty host. They cover grant replay
  and refusal, assignment persistence for parent/child, substituted authority
  and descriptor rejection, activation regressions, bounded source compilation,
  retained-byte verification and deterministic protocol authority. The three
  assignment cases pass again after moving their unchanged test module into
  its final directory layout.
- Isolated [Root](canic165-grants-evidence/root-only.log) and
  [Store](canic165-grants-evidence/store-only.log) library compiles pass.
- [Scoped Clippy](canic165-grants-evidence/clippy.log) passes for core,
  control-plane, host and internal testing library/test targets, with warnings
  denied.
- The [actual initial-Shard journey](canic165-grants-evidence/pocketic.log)
  passes. Selected User Hub/Shard roles receive exact grants, and target-local
  status returns the same Store, complete descriptor and grant. Unselected roles
  receive no grants. Initial child creation, runtime activation and placement
  complete; terminal replay preserves grants and pool observations.
  The case takes 440.06s; the runner takes 525s, including native compilation
  and uncached Wasm builds. This is qualification timing, not a deployment
  benchmark or a measurement of application import.

[Commands](canic165-grants-evidence/commands.json) and the
[1,622-input source snapshot](canic165-grants-evidence/source.sha256) retain the
boundary. Every recorded Rust, Cargo/config and Candid input, and the input
inventory itself, stayed unchanged during final runtime qualification. The only
change after the complete native/Clippy checks was the test-module relocation;
its focused regression and the final runtime check cover that settled source.

## Remaining batch

Connect the registered synchronous importer and its application-owned durable
receipt to scheduling, placement, dispatch and deployment completion. The
combined parent/child proof must then demonstrate receipt ordering, rather than
only grant, assignment and runtime ordering. Qualify later-Shard delivery,
interrupted Root grant effects, funding/backoff and failure classification,
revocation before pool reuse, source reference release and terminal Store
retirement. Current fixture retention still conservatively blocks GC.

The combined worktree is not push-ready. The Unreleased note remains unassigned;
the separately qualified .15 operator batch is unchanged. No broad gate,
version transaction, Git publication, deployment or sibling-repository mutation ran.
