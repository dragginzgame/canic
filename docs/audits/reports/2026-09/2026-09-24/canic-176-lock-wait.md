# CANIC-176 build-lock wait correction

Date: 2026-09-24. Published base: 0.110.39. Local batch: 0.110.40.
Disposition: implemented and qualified locally; downstream adoption remains open.

## Confirmed feedback and scope

Toko Miner's CANIC-176 extension reports scrolling lock waits and insufficient
owner/recovery diagnostics. Its observed holder had Cargo and sccache children;
that establishes contention, not a deadlock or stalled compiler. The maintainer
explicitly authorized this bounded correction. Toko remains read-only.

The CLI now keeps interactive wait status on one width-bounded stderr line.
Redirected output has no terminal control sequences: owner/phase changes report
immediately and otherwise a summary appears at 30-second intervals. Acquisition,
failure and SIGINT/SIGTERM cancellation clear the live line and leave an outcome.
Normal signal termination resumes when the wait scope ends. A failed or cancelled
lock acquisition cannot fall through to an uncached build.

The existing locked inode carries advisory phase and process-birth metadata.
Phase time changes only on entry to input preparation, output verification,
artifact building, and final verification/publication. It is not a heartbeat or
progress claim within a long phase. A new host-only `signal-hook` dependency owns
safe signal registration; the prior lockfile changes are preserved exactly.

`canic diagnostic build-lock --lock <exact-path> [--json]` opens an existing regular
file read-only. It neither creates nor acquires a lock. Linux inspection compares
its device/inode with the visible kernel lock table, then binds PID, boot,
namespace and process birth before showing children. Traversal is bounded to 32
processes and 64 threads; individual reads are bounded. No command arguments or
environment values are read. Missing visibility, incomplete snapshots and PID
reuse do not establish an exited or stalled owner.

Human output includes escaped paths, UTC acquisition/phase times and ages,
whitelisted process kinds and scheduler/CPU observations, a quoted read-only
inspection command, and guidance for progressing owners, quiet children,
redundant waiters and independently confirmed owner exit. Control-containing or
non-UTF-8 paths omit the shell-copy command. No elapsed-time threshold authorizes
termination, and no path unlinks or replaces the inode. Exact cache admission and
artifact verification remain unchanged. Current advisory metadata and the
fallible host progress callback are a pre-1.0 contract hard cut.

## Qualification

Evidence is retained in `.tmp/canic-176-20260924/`:

- Final focused run: 34 host tests, nine CLI unit tests and two CLI executable
  integration tests pass. The existing opt-in relocation/performance matrix is
  not selected. Coverage includes real contention, owner crash/handoff, cancelled
  waiter exclusion, exact retained-release reuse and tampered-output rejection;
  typed process visibility/PID reuse, quiet/active child snapshots, malformed and
  bounded metadata, explicit nullable identity, process traversal limits, TTY
  repaint/resize, sparse redirected output, signal cancellation/default behavior,
  read-only executable inspection, and recursive concise/alphabetical help.
- Strict `cargo clippy --locked -p canic-host -p canic-cli --all-targets
  --all-features --keep-going -- -D warnings` passes on final source.
- A host-visible live test owner is inspected through the actual CLI: the kernel
  holder equals the recorded PID and birth/namespace binding is matched. The
  fixture releases normally through stdin and its metadata clears. The snapshot
  is retained as `live-inspection.json`.
- Dependency-risk inventory passes against the local advisory database: zero
  vulnerabilities, with the same two reviewed transitive warnings. Layering,
  scoped formatting, whitespace and current-document checks pass; document
  layout advisories remain confined to the parked metrics-history design.
- Prior CANIC-150/CANIC-182 source hashes remain unchanged. Removing only the
  new signal-hook lock entries reconstructs the prior lockfile hash
  `bd86d6698f65361891c00a5631fb083d39e854ecdaa521e553e8473211be89da`.
  `source.sha256` binds the resulting batch source and manifests.

The first inspection regression exposed a missing test fixture directory; its
setup was corrected before the final run. No full workspace or PocketIC suite
was run for this host/CLI correction. This is not an application build-speed
measurement or proof that Toko's previously observed owner was stalled.

## Delivery boundary

CANIC-176 joins CANIC-150 and CANIC-182 in the open .40 operator-correction batch.
The complete bounded batch and both changelog surfaces are ready for the
maintainer's release flow under the existing affected-line corrective-release
cadence exception. Package versions remain unchanged; all edits are uncommitted.
No release, commit, push, deployment or external repository mutation occurred.

Keep upstream acceptance open until the published CLI is adopted and real
contended/cancelled builds pass through Toko's ordinary launcher. Controlled
application-scale build measurements, per-target timing attribution and B3 remain
separate work.
