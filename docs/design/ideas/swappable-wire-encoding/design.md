# Idea: Swappable IC Wire and Candid Encoding

Requested: 2026-10-17

## Status

Deferred and unnumbered. This records the maintainer's desire to keep Candid
available as an alternative if Canic adopts `ic-wire`. It does not authorize
implementation, select a release, or make the current integration batch depend
on this work. The proposed boundary needs qualification against the selected
upstream packages; this note does not claim they already supply interchangeable APIs.

## Objective

Make it possible to replace `ic-wire` with Candid, or select `ic-wire` instead
of Candid, without rewriting Canic's policy, workflows or authoritative models.
The choice should be localized to endpoint and client encoding adapters, generated
bindings and the build's declared protocol contract.

“Swappable” means producing a coherent build with a deliberately selected codec.
It does not mean changing the encoding of an installed Fleet in place, silently
retrying a request with another codec, or assuming the two byte formats match.

## Proposed ownership boundary

- `canic-contracts` owns passive logical request/response contracts and explicit
  representation rules. Codec-specific derives and conversions should be isolated
  where practical; do not invent a second copy of every domain model merely to
  hide a derive.
- `canic-macros` and runtime endpoint adapters own dispatch, bounded decoding and
  response encoding. Authentication and caller/subject/audience bindings remain
  enforced at the endpoint regardless of codec.
- `canic-host` and `canic-cli` select matching client adapters and artifacts from
  the exact build binding. Generic codec machinery belongs in its upstream owner,
  such as `ic-wire`; Canic retains its product contracts and admission rules.
- Policy and workflow consume logical typed values. They must not branch on the
  selected encoding or contain codec-specific serialization and parsing.

Prefer a small, explicit adapter boundary over a general serialization framework.
Build-time selection is the initial direction; the exact Cargo feature/package
shape remains open until an end-to-end prototype demonstrates the boundary.

## Requirements for a future design

1. Bind the selected codec and exact schema/contract identity into build artifacts,
   generated clients, protocol-profile evidence and installed identity checks.
   Reject a mismatched client before a paid or mutating effect where possible.
2. Specify exact mappings for principals, full-width integers, bytes, variants,
   optional values, errors and unknown fields. Equivalent logical values need not
   have identical bytes. Decode limits and malformed-input refusals must be explicit.
3. Keep signed bytes, action hashes, certified commitments and persisted journals
   under their existing canonical encoding owners. Changing endpoint encoding must
   not silently change those identities; if a future design changes one, review it
   as an explicit contract change rather than a codec implementation detail.
4. Qualify generated interfaces and tooling for each selection. A Candid build
   needs matching `.did` and client artifacts; an `ic-wire` build needs its declared
   equivalents. Do not advertise a Candid method that accepts different bytes.
5. Preserve same-release retries, idempotency, lost-response reconciliation and
   backup/restore safety. A decode failure must never trigger a second mutating
   request under another encoding.
6. Keep IC management calls and external services on their required protocols.
   Inventory these fixed boundaries instead of promising that every dependency
   can switch codecs. Stable-state encoding is a separate decision.

Pre-1.0 selection changes follow the existing hard-cut, reinstall-only policy.
This idea adds no cross-release migration, mixed-version Fleet operation, rollback
or compatibility lane. Canic-owned current contracts remain `v1`; cycle conservation
and same-operation interruption recovery remain required at a release boundary.

## Qualification before promotion

Prototype one query and one authenticated update through both complete builds:
endpoint, host client, generated interface and artifact identity. Demonstrate the
same logical results and typed refusals, malformed/oversized input rejection,
wrong-codec refusal, and no duplicate effect after an uncertain update response.
Measure Wasm size, compile time and encode/decode cost before generalizing.

Promotion requires an agreed upstream owner, evidence that Candid can be selected
without domain changes, and a complete maintainer-approved release batch. Until
then this remains a contingency idea, not a current release blocker.
