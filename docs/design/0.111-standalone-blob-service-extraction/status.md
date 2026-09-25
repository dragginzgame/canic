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
- Scope: Canic-only planning authority. No external repository creation or
  mutation, application adoption, versioning or deployment is authorized.

## Release-Batch Tracker

| Batch | Outcome | Status |
| --- | --- | --- |
| B1 | Refreshed provider contract, owner/consumer assignment and complete extraction inventory | Planned after accepted 0.110 closeout |
| B2 | Qualified and published independent service/client implementation | External dependency; owner/path and separate mutation authority required |
| B3 | Complete Canic hard cut and generic managed Component integration, evidence and propagation | Blocked on qualified service publication |
| B4 | Final artifact/consumer qualification and human minor closeout | Blocked on complete B3 |

## Next Action

Await human acceptance of the completed
[0.110 closeout audit](../../audits/release-lines/0.110-closeout-audit.md) before
beginning implementation. Then resolve B1's concrete consumer, external owner,
exact repository and provider evidence. The multi-Fleet Q0 capsule proof and
indexed-estate work are not prerequisites. Current blob features and commands
remain maintained until the complete B3 extraction ships.
