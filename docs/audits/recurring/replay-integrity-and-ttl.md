# Audit: Replay Integrity and TTL Semantics

## Purpose

Verify replay protection remains deterministic, bounded, and conflict-safe for
root capability requests.

Security invariant:

> Replay identity, payload binding, and TTL decisions must prevent duplicate
> side effects while rejecting mutated or expired replays.

## Canonical Contract

Primary references:
- `docs/design/0.11-root-capabilities.md`
- `docs/design/0.13-distributed-capability-invocation.md`
- `docs/status/0.13-distributed-capability-invocation.md`

Required replay properties:
1. slot identity is deterministic for caller/target/service/request identity
2. payload digest binds canonical request shape
3. identical replay returns cached response
4. conflicting replay is rejected and does not overwrite existing record
5. expired replay is rejected
6. replay store remains bounded

## Scope

Audit these modules first:
- `crates/canic-core/src/ops/replay/*`
- `crates/canic-core/src/ops/storage/replay.rs`
- `crates/canic-core/src/storage/stable/replay.rs`
- `crates/canic-core/src/workflow/rpc/request/handler/replay.rs`
- `crates/canic/tests/root_replay.rs`

## Run Context

Record in the result file:
- date
- auditor
- branch
- commit (`git rev-parse --short HEAD`)
- workspace state (`clean` or `dirty`)
- audited paths

## Checklist

Mark each item:
- `[x]` Pass
- `[ ]` Fail
- `[~]` Ambiguous or follow-up needed

### 1. Replay Slot Identity Is Stable and Scoped

Verify root replay slot key derivation includes expected context and has
legacy-read compatibility where intended.

Suggested scans:

```bash
rg -n 'slot_key|root_slot_key|legacy_root_slot_key|ReplayService::Root|REPLAY_SLOT_KEY_DOMAIN' \
  crates/canic-core/src/ops/replay/key.rs \
  crates/canic-core/src/ops/storage/replay.rs \
  crates/canic-core/src/ops/replay -g '*.rs'
```

- [ ] Canonical root slot key uses caller + target + service + request identity
- [ ] Legacy slot key path is read-only compatibility behavior
- [ ] Slot-key derivation is deterministic

Findings:
- (file, line, behavior)

### 2. TTL Validation Is Strict and Mechanical

Verify TTL bounds are pure checks and reject invalid values.

Suggested scans:

```bash
rg -n 'validate_replay_ttl|InvalidTtl|max_ttl_seconds|ttl_seconds' \
  crates/canic-core/src/ops/replay crates/canic-core/src/workflow/rpc/request/handler/replay.rs -g '*.rs'
```

- [ ] `ttl_seconds == 0` is rejected
- [ ] `ttl_seconds > max_ttl_seconds` is rejected
- [ ] TTL error mapping remains explicit and typed

Findings:
- (file, line, behavior)

### 3. Existing Slot Decisions Match Contract

Verify replay decision classification:
- `DuplicateSame`
- `DuplicateConflict`
- `Expired`
- `Fresh`

Suggested scans:

```bash
rg -n 'ReplayDecision|resolve_existing|DuplicateSame|DuplicateConflict|Expired|Fresh' \
  crates/canic-core/src/ops/replay/guard.rs crates/canic-core/src/workflow/rpc/request/handler/replay.rs -g '*.rs'
```

- [ ] Expired entry yields `Expired`
- [ ] Same payload hash yields `DuplicateSame`
- [ ] Different payload hash yields `DuplicateConflict`
- [ ] Missing slot yields `Fresh`

Findings:
- (file, line, behavior)

### 4. Payload Binding and Commit Semantics Are Correct

Verify replay payload digest and replay commit rules:
- payload hash comes from canonical capability command encoding
- commit persists payload hash and response bytes
- commit occurs only after successful execution

Suggested scans:

```bash
rg -n 'hash_capability_payload|payload_hash|commit_replay|RootReplayRecord|response_candid' \
  crates/canic-core/src/workflow/rpc/request/handler/replay.rs \
  crates/canic-core/src/storage/stable/replay.rs \
  crates/canic-core/src/ops/replay -g '*.rs'
```

- [ ] Replay payload hash excludes non-canonical metadata paths
- [ ] Conflicting replay does not overwrite accepted record
- [ ] Failed execution does not commit replay record

Findings:
- (file, line, behavior)

### 5. Store Boundedness and Expiry Purge Behavior

Verify bounded map and expiry purge behavior remain intact.

Suggested scans:

```bash
rg -n 'MAX_ROOT_REPLAY_ENTRIES|ReplayStoreCapacityReached|purge_expired|collect_expired|len\\(' \
  crates/canic-core/src/{workflow/rpc/request/handler/replay.rs,storage/stable/replay.rs,ops/storage/replay.rs,ops/replay} -g '*.rs'
```

- [ ] Capacity bound is enforced
- [ ] Expired entries are purged
- [ ] Expired records are not resurrected

Findings:
- (file, line, behavior)

### 6. Test Coverage for Replay Guarantees

Verify unit and integration coverage includes:
- duplicate-same
- duplicate-conflict
- expired replay
- invalid TTL
- conflict-shape mutation with same request id

Suggested scans:

```bash
rg -n 'duplicate|conflict|expired|ttl|ReplayDuplicateSame|ReplayConflict|ReplayExpired' \
  crates/canic-core/src/workflow/rpc/request/handler/tests.rs \
  crates/canic/tests/root_replay.rs
```

- [ ] Unit coverage exists for replay decision outcomes
- [ ] Integration coverage exists for root replay behavior
- [ ] Missing cases are listed explicitly

Findings:
- (test file, missing case)

## Severity Guide

- Critical: replay bypass allows repeated side effects
- High: conflict replay can overwrite or execute
- Medium: TTL/expiry drift causes non-deterministic replay behavior
- Low: boundedness or coverage drift without immediate bypass

## Audit Frequency

Run this audit:
- after replay module changes (`ops/replay`, `workflow/.../replay`)
- after capability DTO/replay metadata changes
- before each release cut
