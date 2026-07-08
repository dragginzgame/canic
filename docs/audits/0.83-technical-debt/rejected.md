# Canic 0.83 Rejected Candidates

## REJECTED-001: Pure Policy Import Sweep

Decision: reject_with_rationale

Evidence:
- command/search: `rg -n -e "use crate::(dto|ops|workflow|storage|runtime)" -e "use ic_cdk" -e "use serde" -e "serde_json" -e "spawn" -e "timer" -e "management" crates/canic-core/src/domain/policy/pure -S`
- result: no forbidden active imports found. Matches were documentation text
  or pure decision field names such as `should_spawn`.

Rationale:

The `domain::policy::pure` namespace does not currently show policy-purity
debt under the scanned forbidden-import criteria.

## REJECTED-002: TODO/FIXME/HACK Sweep

Decision: reject_with_rationale

Evidence:
- command/search: `rg -n "TODO|FIXME|HACK|XXX" crates/canic-core/src crates/canic-cli/src crates/canic-host/src crates/canic-control-plane/src crates/canic-backup/src crates/canic-wasm-store/src -S`
- result: no active source matches.

Rationale:

The initial source TODO sweep did not produce debt candidates.

## REJECTED-003: Remaining Public Type Aliases

Decision: reject_with_rationale

Evidence:
- file: `crates/canic-core/src/cdk/types/string.rs`
- line or anchor: `BoundedString8` through `BoundedString256`
- command/search: `rg -n "pub type [A-Za-z0-9_]+\\s*=" crates/...`
- reachability: active public Rust helper surface
- exact issue: bounded-string aliases are semantic helper aliases over a
  const-generic bounded string owner, not compatibility wrappers.

Evidence:
- file: `crates/canic-core/src/api/timer.rs`
- line or anchor: `TimerSlot`
- reachability: active public macro/lifecycle helper surface
- exact issue: timer slot alias names a `LocalKey<RefCell<Option<TimerHandle>>>`
  shape for public lifecycle API ergonomics, not a legacy path.

Rationale:

These aliases are not debt under the 0.83 model because they do not preserve
removed surfaces, duplicate semantic ownership, or hide compatibility behavior.
