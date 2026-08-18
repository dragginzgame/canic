# Idea: Product Frontend Delivery Handoff

Date: 2026-08-18

## Status

- Classification: deferred, unnumbered idea. It is not a scheduled release or
  implementation authority.
- Need: an external product frontend must consume exact Canic App/Fleet
  identity, generated role bindings and environment trust inputs without
  scraping incidental operator state.
- Sequence: this may be promoted independently after 0.105 freezes direct
  application authorization. It does not gate the infrastructure estate path.
- Downstream boundary: Prequel Wars is the first concrete consumer, but the
  contract must remain application-neutral and Canic must not depend on its
  frontend package.

## Decision Direction

Canic should qualify one read-only host workflow that emits an
integrity-checked frontend environment manifest from a terminal installed App
catalog. The manifest should bind:

- schema version 1, network/environment identity and Canic release-set digest;
- exact Fleet and App identity;
- each exported application role, Canister Principal, Candid input and
  generated-binding content hash;
- agent endpoint and explicit local-replica trust settings;
- Internet Identity Canister/origin plus any separately maintained
  alternative-origin artifact identity;
- an optional externally managed static-asset Canister Principal/origin; and
- one digest covering every field and referenced generated artifact.

The static frontend remains an ICP CLI/application delivery concern rather
than a Canic Component. Canic verifies and exports the interoperability
boundary but does not provision, upgrade or control the asset Canister.

The exporter must resolve identity from verified terminal Canic state, fail on
unknown or partially converged roles and never infer a Canister ID from a
source/package name. It may expose public routing and local trust material; it
must not expose controller credentials, signing keys, root proofs, bearer
tokens, provisioning authority or Fleet mutation capability.

## Required Qualification Before Promotion

1. freeze the canonical machine-readable format, size bounds and digest rules;
2. freeze the read-only CLI owner and keep its command/help ordering within the
   maintained lexicographic contract;
3. identify the one canonical Candid-to-JavaScript binding generator and prove
   stale generated bindings fail CI;
4. prove manifest role Principals match the selected installed environment and
   mismatches fail before a frontend build;
5. document agent and Internet Identity setup for one local replica and one IC
   environment, including alternative-origin handling;
6. qualify one independently built asset frontend without making it a Canic
   Component or granting it privileged infrastructure authority; and
7. prove a generic fixture can authenticate and call one App entry role and
   one 0.105-authorized managed Component through generated bindings.

Until promotion, downstream frontends remain independently built and hosted.
They must use explicit environment configuration and generated bindings and
must not scrape Canic workspace files or guess deployed identities.
