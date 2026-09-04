# Idea: Operator Top-Level Component Lifecycle

Date: 2026-09-04

## Status

- Classification: deferred, unnumbered idea. It is not a scheduled release or
  implementation authority.
- Need: Fleet Ensure converges declared initial Component Groups, but the
  maintained CLI does not yet own ordinary post-install allocation and
  activation of one admitted top-level Component.
- Feedback owner: `CANIC-010`.
- Safety boundary: this must drive the existing Root-owned typed lifecycle; it
  must not expose raw Candid choreography or create a second lifecycle owner.

## Decision Direction

A future CLI leaf should plan and reconcile one top-level Component from its
current Component Spec and placement authority. It should:

1. bind Fleet, Root, Component Spec, release set and placement before mutation;
2. persist one bounded operation identity before the allocation request;
3. reconcile lost responses through the existing Root operation status;
4. wait through allocation, installation, Directory publication and activation;
5. return the exact terminal role, Component identity and Canister Principal;
6. fail closed on pool capacity, admission, authority or release drift; and
7. replay terminally without another allocation, install or debit.

The command must distinguish a role selected automatically for a Component
Spec from an instance that is explicitly requested or declared as an initial
placement. It must provide concise structured output suitable for environment
linking without making downstream scripts construct protected protocol DTOs.

## Required Qualification Before Promotion

- exact retry and lost-response recovery at every lifecycle phase;
- conflicting operation, placement and release authority rejection;
- insufficient pool capacity and denied-caller evidence;
- machine-readable terminal role, Component and Principal output;
- one packaged downstream consumer using only the public CLI; and
- proof that initial Fleet Ensure placements retain their existing owner.

Until promotion, initial instances belong in checked-in Component Group
placements. Later allocation must use the documented protected Root protocol
deliberately rather than an application-owned convenience wrapper.
