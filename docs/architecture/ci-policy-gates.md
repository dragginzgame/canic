# CI Policy Gates

Policy gates evaluate saved evidence without building artifacts or mutating a
Fleet:

```bash
canic evidence gate --policy <path> --envelope <path>
canic evidence gate --policy <path> --manifest <path>
```

Policies are strict TOML. Unknown fields reject. A conservative build policy:

```toml
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"
allowed_payload_schemas = ["canic.build_provenance.v1"]
allowed_payload_stability = ["stable"]

[exit_class]
allowed = ["success"]

[summary]
fail_on_evidence_conflicts = true
fail_on_blocked_actions = true
allow_missing_or_stale_evidence = false

[build_provenance]
require_clean_source = true
require_cargo_lock = true
require_wasm_gzip = true
require_sha256 = true
require_package_identity_matches_target = true
```

A minimal current pipeline builds provenance and gates that saved envelope:

```bash
canic build demo app \
  --provenance artifacts/canic/app-build-provenance.json

canic evidence gate \
  --policy ci/canic-policy.toml \
  --envelope artifacts/canic/app-build-provenance.json \
  --json \
  --output artifacts/canic/policy-gate-report.json
```

Success proves only that the saved evidence satisfied policy at evaluation
time. It does not prove live Fleet convergence; `canic fleet ensure` owns that
separate reviewed boundary.
