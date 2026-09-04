# Idea: Long-Running Multi-Subnet Local Fleet

Date: 2026-09-04

## Status

- Classification: deferred, unnumbered idea. It is not a scheduled release or
  implementation authority.
- Need: repository PocketIC suites prove multi-Subnet topology, but downstream
  development has no supported persistent Fleet process with browser discovery
  and lifecycle controls.
- Feedback owner: `CANIC-017`.
- Scope: developer tooling only. It must not claim mainnet latency, charging,
  consensus, Registry or boundary-node fidelity.

## Decision Direction

A future public host command or harness may own one bounded PocketIC-backed
development Fleet. It should:

1. start the declared Coordinator, at least two Roots and Stores on exact
   reported Subnets;
2. converge configured Components through the maintained Fleet Ensure owner;
3. expose one stable browser-compatible gateway;
4. emit a bounded structured role, Canister and Subnet discovery document;
5. support deterministic status, restart, time advance, reset and shutdown;
6. retain exact cross-Subnet message boundaries; and
7. reuse the public testing facade rather than exporting internal test crates.

The discovery document may contain public routing and explicit local root-key
trust material. It must not contain controller credentials, mutation authority,
private keys or an inference that a compiled artifact is live.

## Required Qualification Before Promotion

- clean packaged-downstream startup without copying Canic internals;
- two-Root placement and cross-Subnet call evidence;
- browser discovery for every installed role;
- restart, response-loss, time-control and clean-shutdown tests;
- bounded resource and artifact-cache behavior for a long-running process;
- explicit fidelity-limit output; and
- one application-neutral downstream qualification journey.

Until promotion, ordinary standalone-local development and repository-owned
PocketIC evidence remain separate. Downstreams must not hard-code test
Principals or present test topology as a deployed Fleet.
