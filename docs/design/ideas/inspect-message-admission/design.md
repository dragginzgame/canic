# Idea: Revisit inspect-message admission

Date: 2026-09-15

## Status

Deferred at the maintainer's request for a future slice. This idea has no
scheduled release position or implementation authority. Scope is Canic only.

## Current behavior

Canic lifecycle macros generate `canister_inspect_message`. Ordinary ingress
updates have a 16 KiB encoded-argument limit unless an endpoint declares
`payload(max_bytes = ...)`. Control-plane inspectors also decode bounded
commands and, where applicable, enforce command-variant limits. Explicit
endpoint limits generate a separate check before decoding during update
execution, including inter-canister calls; the implicit ingress default alone
does not provide that endpoint-side check.

The generated hook currently exposes no application inspection callback.
See [hook generation](../../../../crates/canic/src/macros/start.rs),
[payload inspection](../../../../crates/canic-core/src/ingress/payload.rs)
and [endpoint expansion](../../../../crates/canic-macros/src/endpoint/expand/mod.rs).

## Future investigation

- Assess cheap caller and method filters, such as rejecting anonymous callers
  on selected methods or direct ingress to inter-canister-only methods.
- Consider bounded argument screening and advisory admission based on existing
  maintenance flags, allowlists or recorded quota usage.
- Review checking argument size before copying the payload: the current
  generic inspector copies argument bytes to obtain their length.
- Decide whether concrete application needs justify a composed inspection
  callback while retaining Canic ownership of the single generated hook.

## Constraints and ownership

Inspection is a non-replicated admission optimization, not authorization.
It runs on one node with potentially stale state and is bypassed by queries,
inter-canister calls and management-canister calls. Keep authoritative access,
payload and business checks at their executable endpoint boundaries.

Inspection cannot persist quota counters or reservations, make inter-canister
calls or return a normal application reply. Existing usage may inform an early
filter; update execution must own accounting. State-dependent filters must
consider false rejection after a recent state change and preserve recovery
access. See the [IC system API](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/#ingress-message-inspection).

Owners are `canic` lifecycle/facade generation, `canic-macros` endpoint adapters
and `canic-core` ingress handling; control-plane command policies remain with
their owning roles. Any extracted policy must remain pure.

## Evidence before promotion

Choose a concrete unwanted-call workload and a bounded release batch. Measure
admission cost and avoided execution, then use focused PocketIC cases to prove
accepted and rejected ingress, encoded-size boundaries, endpoint enforcement
on inter-canister calls and continued recovery access. Include stale-state
behavior for any state-dependent filter. Update generated surfaces and user
documentation together if the public configuration changes.
