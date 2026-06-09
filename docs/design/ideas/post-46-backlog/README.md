# Post-46 Backlog Ideas

These files preserve optional feature ideas that came from the historical
post-46 backlog.

The historical source material remains archived under:

```text
docs/design/archive/post-46-backlog/
```

The post-46 backlog is not an active numbered release line. These ideas should
not be implemented merely because they exist here.

## Already Covered

Later release lines already covered the major reusable foundations:

- 0.50 implemented passive adoption profiles and read-only adoption reports.
- 0.51 implemented stable evidence envelopes, exit classes, passive envelope
  emitters, and envelope comparison.
- 0.52 implemented source, build, and artifact provenance for build outputs.
- 0.53 implemented CI policy gates and project evidence manifests over saved
  evidence.
- 0.54 implemented the passive local deployment catalog.

These surfaces should not be rebuilt from this ideas directory.

## Optional Ideas

- [Deployment groups, lanes, and readiness](deployment-groups-readiness.md)
- [Active adoption and import](active-adoption-import.md)
- [Wasm-store artifact registry](wasm-store-artifact-registry.md)
- [CI locks, signing, and provider wrappers](ci-locks-signing-provider-wrappers.md)
- [DR clone verification](dr-clone-verification.md)
- [Teardown and test deployment lifecycle](teardown-test-deployment-lifecycle.md)
- [Deploy verification baselines](deploy-verification-baselines.md)

## Promotion Rule

An idea may move back into active design only when there is a named current
need. Historical backlog presence is not enough.
