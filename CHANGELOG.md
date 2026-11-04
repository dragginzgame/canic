# Changelog

Canic keeps one workspace with independently versioned release groups:

- [Framework](releases/framework/CHANGELOG.md): runtime, contracts,
  macros, control plane and framework qualification packages.
- [Host tooling](releases/host/CHANGELOG.md): `canic-host` and `canic-cli`.
- [IcyDB adapter](releases/icydb/CHANGELOG.md): `canic-icydb`.
- [Blob service adapter](releases/blob-service/CHANGELOG.md): `canic-blob-service`.

[Coordinated releases through 0.114.0](releases/history.md) retain their original
scope. Current versions live in member manifests and the
[root catalog](Cargo.toml). Release and publication explicitly select
`RELEASE_GROUP=framework`, `host`, `icydb` or `blob-service`.
