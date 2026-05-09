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
- Removed persisted fleet/network defaults; fleet-scoped commands take the fleet
  name as a positional argument, and network selection is per-command via
  `--network <name>` with local replica behavior when omitted.
- Started the 0.33 hard cut toward `icp-cli`/`icp.yaml`; dev setup and CI
  install pinned `icp` and `ic-wasm` binaries.
- Confirmed the ICP-only local demo install smoke with `icp 0.2.5`, including
  `canic install demo`, `canic config demo`, `canic list demo`, and
  `canic medic demo`.
- Confirmed the auth-enabled `test` fleet install smoke after moving active
  threshold ECDSA key defaults from the old local key name to ICP CLI's `key_1`.
- Moved the public read-only/snapshot/restore-runner CLI surfaces toward ICP
  CLI: `list`, `config`, `medic`, `snapshot download`, and `restore run` now
  expose `--icp <path>` where a tool override is needed.
- Added `canic fleet delete <name>` for confirmed deletion of config-defined
  fleet directories.
- Hard-cut fleet scaffolds to top-level `fleets/`.
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
- `cargo run -q -p canic-cli --bin canic -- install demo --ready-timeout-seconds 60`
- `cargo run -q -p canic-cli --bin canic -- config demo`
- `cargo run -q -p canic-cli --bin canic -- list demo`
- `cargo run -q -p canic-cli --bin canic -- medic demo`
- `cargo run -q -p canic-cli --bin canic -- install test --ready-timeout-seconds 120`
- `cargo run -q -p canic-cli --bin canic -- list test`
- `cargo run -q -p canic-cli --bin canic -- medic test`
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
   `canic fleet create`, `canic install`, `canic fleet`, `canic config`,
   `canic list`, `canic backup`, `canic snapshot`, and `canic restore`.
3. Keep `canic-cli`, `canic-host`, and `canic-backup` boundaries sharp: CLI owns
   UX, host owns ICP CLI/filesystem/build/install mechanics, backup owns
   backup/restore domain logic.
4. Continue the ICP-only 0.33 hard cut across remaining backup/restore docs and
   any deeper CI smoke paths that still mention the old host-tool provider.
