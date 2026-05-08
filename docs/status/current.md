# Current Status

Last updated: 2026-05-08

## Purpose

This file is the compact handoff for new agent sessions. Read it first, then
inspect only the files needed for the current task.

## Current Line

- Active minor: `0.32.x`
- Theme: simplify the `canic` executable, reduce stale CLI surfaces, and lower
  repeated agent context cost.
- Last user-stated publish point in this thread: work is continuing after
  `0.32.4`-area cleanup; verify exact published tag with read-only git if it
  matters.

## Recent Work

- Shrunk `AGENTS.md` from a long embedded rulebook into a compact routing file.
- Added `docs/governance/ci-deployment.md` for command, git, versioning,
  release, network, and automation-language rules.
- Left detailed changelog policy in `docs/governance/changelog.md`.
- Removed the public `canic release-set` CLI surface.
- Continued CLI simplification around fleet/list/scaffold/install flows.
- Added `canic network`, `canic status`, and `canic fleet {list,use}` defaults;
  `canic install` now passes the selected network through dfx subprocesses and
  build scripts instead of relying on the caller's ambient `DFX_NETWORK`.
- Added `canic fleet delete <name>` for confirmed deletion of config-defined
  fleet directories.
- Hard-cut fleet scaffolds and implicit install defaults to top-level `fleets/`.
- Hard-cut role-attestation audience from optional to required.
- Added/updated recurring audit reports for audience target binding and token
  trust-chain invariants.

## Validation Recently Run

- `cargo fmt --all`
- `cargo check -p canic-cli`
- `cargo check -p canic-host`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo test -p canic-host --lib -- --nocapture`
- `cargo test -p canic-host --lib install_state_round_trips_from_project_state_dir -- --nocapture`
- targeted `canic-core` auth tests
- targeted PocketIC role-attestation/root-key tests
- `git diff --check` on touched files

## Known Worktree Notes

- The worktree is intentionally dirty during active slice work.
- Do not revert unrelated edits.
- Agents must not stage, commit, push, bump versions, or run release targets.

## Cost-Control Rules

- Prefer scoped searches over broad repo searches.
- Avoid searching `docs/changelog/**`, `docs/audits/reports/**`, and generated
  outputs unless the task is specifically about those files.
- Write detailed findings to files; summarize only the high-signal result in
  chat.
- Keep final responses concise and include validation commands actually run.

## Good Next Tasks

1. Continue reducing `AGENTS.md` by moving any remaining detailed architecture
   rules into focused governance/architecture docs if useful.
2. Keep removing stale public CLI surfaces and ensuring the operator flow is
   `canic scaffold`, `canic install`, `canic fleet`, `canic list`, `canic backup`,
   `canic snapshot`, and `canic restore`.
3. Keep `canic-cli`, `canic-host`, and `canic-backup` boundaries sharp: CLI owns
   UX, host owns local `dfx`/filesystem/build/install mechanics, backup owns
   backup/restore domain logic.
