# Canic 0.111 Implementation Status

Date: 2026-09-25

## Status

- Roadmap: standalone blob service extraction is the maintainer's accepted
  next major slice, replacing the deferred bounded multi-Fleet estate proposal.
- Design: [standalone blob service extraction](0.111-design.md).
- Implementation: not started; requires human acceptance of the completed 0.110 closeout audit.
- Canic owners: runtime/facade, host/CLI and testing owners.
- External owner, exact repository path, package names, concrete consuming
  application and publication plan remain to be assigned in B1.
- B1 must freeze the [service contract and B2 entry conditions](0.111-design.md#b1-service-contract-and-b2-entry-conditions):
  both deployments and adapter ownership, tenant authority, recovery identities
  and restore fencing, provider suitability, quota/cost accounting and existing
  installation retirement. Every guarantee needs an owner and acceptance
  test or evidence source; those decisions remain open.
- Scope: Canic-only planning authority. No external repository creation or
  mutation, application adoption, versioning or deployment is authorized.

## Release-Batch Tracker

| Batch | Outcome | Status |
| --- | --- | --- |
| B1 | Frozen service contract, actual-provider suitability evidence, owner/consumer assignment, behavior classification and complete removal/obligation inventories | Planned after accepted 0.110 closeout; contract decisions must close before B2 |
| B2 | Qualified and published shared service/client implementation and both adapters for the bounded B1 journey | External dependency; complete B1 contract, owner/path and separate mutation authority required |
| B3 | Complete Canic hard cut and generic managed Component integration, evidence and propagation | Blocked on qualified service publication |
| B4 | Final artifact/consumer qualification and human minor closeout | Blocked on complete B3 |

## Next Action

Await human acceptance of the completed
[0.110 closeout audit](../../audits/release-lines/0.110-closeout-audit.md) before
beginning implementation. Then resolve B1's concrete consumer, external owner,
exact repository, provider evidence and all service-contract entry conditions.
The acceptance journey is bounded upload/resume, verified read and authorized
release through confirmed deletion/billing cessation in both deployments.
Classify preserved behavior, required safety corrections and deferred new
capabilities to bound B2. Source allocation removal and safe retirement of
affected installations remain separately owned; no reset may erase the only
records of external obligations. The multi-Fleet Q0 capsule proof and
indexed-estate work are not prerequisites. Current blob features and commands
remain maintained until the complete B3 extraction ships.
