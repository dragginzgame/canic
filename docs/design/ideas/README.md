# Design Ideas

This directory holds unnumbered, deferred design concepts. Files here are not
release requirements, scheduled lines or implementation authority. A retired
working number has no current planning meaning. Historical review or
investigation approval does not authorize resuming an idea.

Each idea uses one descriptive topic directory with `design.md` and, only
where it preserves useful prior context, `status.md` or `exploration.md`.
Supporting implementation evidence does not belong here.

## Current Topics

- `cross-subnet-data-transport-groundwork/`
- `declarative-authentication-profiles/`
- `demand-driven-canister-pool-maintenance/`
- `estate-budget-replenishment/`
- `fleet-observatory/`
- `immutable-test-checkout-lease/`
- `long-running-multi-subnet-local-fleet/`
- `operator-funding-conversion-authority/`
- `operator-top-level-component-lifecycle/`
- `product-frontend-delivery-handoff/`
- `standalone-blob-service-extraction/`

## Maintainer Priorities

The two most pressing ideas, identified by the maintainer on 2026-09-06, are:

1. [Canonical infrastructure crates](../0.110-fleet-runtime-contraction/0.110-design.md#canonical-fleet-subnet-root-batch-cr1):
   give Root one Canic-owned entrypoint and consistent Fleet crate names.
2. [Standalone blob extraction](standalone-blob-service-extraction/design.md):
   remove application blob-storage semantics from Canic infrastructure.

Canonical infrastructure was promoted into the current 0.110 CR1 batch on
2026-09-07 at the maintainer’s request. Standalone blob extraction remains
deferred; the remaining ideas have no release position.

## Review Disposition

Reviewed against current Canic contracts on 2026-09-06:

- Keep the concrete product needs: canonical infrastructure crates, pool
  maintenance, operator Component lifecycle, local multi-Subnet development,
  frontend handoff, host-first Observatory and bounded estate replenishment.
- Retain transport measurement, authentication profiles and funding conversion
  as unscheduled proposals using current call, command/status, Fleet Ensure
  and application-authorization owners.
- Keep blob extraction conditional on a maintained external service and real
  consumer. The current Canic blob subsystem has not been removed.
- Re-measure checkout-lease benefits after current runner/cache improvements.

The oversized speculative plans and schemas have been replaced with bounded
idea notes. Earlier research remains in Git history. Retained topics confer no
release position; promotion still requires a current need and complete batch.

## Promotion

Move an idea to a top-level numbered directory only when it has a concrete
need and owner, an accepted release position, a complete release-batch plan
and explicit maintainer approval. Update this index and the document-semantics
guard in the same planning cut.

Archived historical source material remains under `docs/design/archive/`.
The former loose post-46 idea backlog was deleted; its durable historical
record remains under `docs/design/archive/post-46-backlog/`.
