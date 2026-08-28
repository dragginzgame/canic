# Build Provenance CI Policy

`canic build <app> <role> --provenance <path>` writes an
`EvidenceEnvelopeV1` with stable payload schema
`canic.build_provenance.v1`.

CI should validate:

- clean or explicitly reviewed source state;
- Cargo lock and package-manifest identities;
- package metadata App/role equality with the envelope target;
- Rust/Cargo toolchain and build profile;
- raw Wasm and deterministic gzip SHA-256 plus sizes;
- transform tool/version/executable-SHA/outcome consistency, including
  required Binaryen 108 optimization and its before/after structural metrics
  for release profiles;
- stable envelope and payload schema identities;
- a successful exit class without conflicting evidence.

Example:

```bash
canic build demo app \
  --profile release \
  --provenance artifacts/canic/app-build-provenance.json

canic evidence gate \
  --policy ci/canic-policy.toml \
  --envelope artifacts/canic/app-build-provenance.json
```

Timestamps are explanatory metadata, not provenance. Build evidence does not
authorize funding, canister identity, controllers, placement, replacement or
deletion; the reviewed current Fleet plan owns those decisions.
