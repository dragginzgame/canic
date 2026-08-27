# Evidence Envelopes

`EvidenceEnvelopeV1` is the stable outer JSON contract for saved build
provenance and other current producers. It records command provenance, target,
input fingerprints, payload schema/identity, structured summary and exit class
without making every nested DTO a stable contract.

The maintained passive commands are:

```bash
canic evidence compare --left <path> --right <path>
canic evidence gate --policy <path> --envelope <path>
canic evidence gate --policy <path> --manifest <path>
```

Build one current envelope with:

```bash
canic build <app> <role> --provenance artifacts/build-provenance.json
```

Current stable schemas include:

```text
canic.evidence_envelope.v1
canic.build_provenance.v1
```

Historical adoption and deployment-check payloads may remain in archived
evidence, but no maintained command creates or interprets them as Fleet
authority.

Envelope generation, comparison and policy evaluation do not install Wasm,
fund canisters, mutate controllers/topology, or refresh stale live evidence.
Policy should branch on stable envelope fields and explicitly stable payloads,
not internal nested DTOs.
