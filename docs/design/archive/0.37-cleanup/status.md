# Canic 0.37 Agent Operating Notes

- **Version:** `0.37`
- **Status:** active cleanup-line operating note
- **Authority:** `AGENTS.md`, `docs/status/current.md`, and
  `docs/governance/*` remain normative
- **Purpose:** keep the release-local reminders for 0.37 audit and cleanup work

## Summary

The 0.37 line is a cleanup-only minor after the completed 0.36 backup/restore
operator flow. Work should stay small, audit-driven, and Canic-owned. Do not add
new backup/restore v1 scope unless real operator use exposes a bug or missing
workflow.

This note is not a replacement for `AGENTS.md`. It records the practical
operating boundaries that matter while doing 0.37 design, audit, and cleanup
work.

## Required Session Baseline

At the start of a new session:

1. Read `docs/status/current.md`.
2. Check `git status --short` before editing.
3. Preserve any dirty worktree changes unless explicitly asked to modify them.
4. Follow the governance docs for CI, deployment, changelog, and code hygiene.

## 0.37 Scope

Allowed work:

- recurring audit refreshes;
- small correctness fixes found by those audits;
- local simplification where it removes concrete risk or duplication;
- focused tests for fixed invariants;
- documentation that records current behavior or release decisions.

Avoid:

- broad architecture rewrites;
- formatting-only churn;
- new backup/restore features not proven by operator use;
- changing public behavior just because a cleaner shape is possible;
- moving files across ownership boundaries without a full migration plan.

## Ownership Boundaries

Keep changes inside Canic-owned code and docs unless a task explicitly says
otherwise:

- runtime/facade: `canic`, `canic-core`, `canic-cdk`, `canic-memory`,
  `canic-macros`;
- control plane/store: `canic-control-plane`, `canic-wasm-store`;
- host/operator: `canic-cli`, `canic-host`, `canic-backup`;
- testing: `canic-testkit`, `canic-testing-internal`, `canic-tests`.

Do not patch Toko paths during 0.37 cleanup work.

## Layering Reminders

Preserve the strict dependency direction:

```text
endpoints -> workflow -> policy -> ops -> model
```

Key rules:

- DTOs are passive boundary data.
- `policy/` stays pure: no mutation, async, timers, IC calls, storage, DTOs, or
  serialization.
- `ops/` owns deterministic state access, conversion, and approved single-step
  platform side effects.
- `workflow/` owns orchestration and may call ops and policy.
- endpoints and macros authenticate, marshal, and delegate immediately.
- conversions belong in ops, not workflow.

## Security Reminders

Auth is enforced at endpoints. Workflow and ops assume authenticated input.
Subnet, parent, subject, audience, and caller bindings must remain explicit.

For delegated auth work, pay particular attention to:

- public helpers that could expose partial "verified token" semantics;
- subject binding before endpoint authorization is accepted;
- required scopes;
- update-token replay consumption;
- keeping DTO auth types data-only.

## Changelog Reminder

Update changelogs only for user-facing changes. Governance-only or operating
note changes do not need release notes unless explicitly requested.

For the 0.37 line, keep the Latin title:

```text
Quis ipsos auditores audit?
```
