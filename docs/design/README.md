# Canic Design Authoring

Every new minor design must follow
[delivery cadence governance](../governance/delivery-cadence.md) and include a
release-batch plan before implementation begins.

## Release-Batch Plan Template

The whole minor line normally contains roughly 6–10 substantive release
batches, shared across every design document assigned to that minor.
Implementation slices may be smaller, but they must map into these batches;
they are not automatically patch releases.

Copy and complete this table in the design or its status tracker:

| Batch | Bounded outcome and owner | Included direct evidence and fallout | Focused validation | Surface impact | Status |
| --- | --- | --- | --- | --- | --- |
| B1 |  |  |  |  | Pending |
| B2 |  |  |  |  | Pending |
| B3 |  |  |  |  | Pending |
| B4 |  |  |  |  | Pending |
| B5 |  |  |  |  | Pending |
| B6 |  |  |  |  | Pending |

Add or remove rows to match the real dependency boundaries. If the line is
planned outside the normal range, explain why immediately below the table.
Use stable batch labels during design; the maintainer assigns version numbers
when a release is actually prepared.

Each batch should include its direct implementation, positive and adversarial
tests, interruption/retry evidence where applicable, documentation, generated
or fixture propagation and required cleanup. Do not create separate batches
for ordinary compile fallout or changelog maintenance.
