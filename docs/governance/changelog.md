# Canic changelog governance

Apply the reviewed [shared changelog rules](../../rules/changelogs.md), including
release-group ledgers, retained coordinated history and impact classification.
[Delivery cadence](delivery-cadence.md) owns coherent batches and maintainer-owned
minor boundaries; [CI/deployment policy](ci-deployment.md) owns execution authority.

Canic has four explicit groups: `framework`, `host`, `icydb` and `blob-service`, declared
in [the release policy](../../ci/cargo-release-groups.json). Each owns one
`releases/<group>/CHANGELOG.md`. The root [changelog](../../CHANGELOG.md) is an
index. Earlier coordinated releases live once in
[shared history](../../releases/history.md), with their original scope and linked
historical detailed notes. The historical ledger receives no new releases.

Maintain one pending numbered entry for each affected group. Extend its existing
open entry until tagged; do not allocate a version per implementation slice.
An adapter change belongs in its adapter ledger. Host/CLI changes belong in the
Host ledger; framework changes and qualification belong in the framework ledger.
Dependent notes explain required
adoption and link to the supplying ledger instead of duplicating implementation
history. Current group versions come from member manifests and the root catalog;
status documents record evidence and remaining work without granting authority.

Each ledger identifies its group and covered packages. Keep root emoji sections
and concise outcome notes. New detailed notes, when useful, belong under
`releases/<group>/<major>.<minor>.md`. Existing linked historical detail paths
retain their original scope. A pending numbered heading may be undated or marked
`Unreleased`; the governed transaction seals the selected ledger with its date.
The native `changelog_governance` cases admit each declared ledger independently
and refuse ambiguous pending selections.

A group release changes only that group's notes and versions. It cannot finalize
another group's pending entry or the historical ledger. The adoption itself keeps
all numeric package identities unchanged; boundary approval is not permission to
bump, tag, push or publish. An authorized release explicitly selects its group:

```sh
make release-patch RELEASE_GROUP=icydb
make publish RELEASE_GROUP=icydb
```

The complete applicable release gate still applies, using the admitted group
plan or its full-roster fallback. See
[adapter release groups](../operations/adapter-release-groups.md) for exact source,
lock, receipt and supplier requirements. Changelog prose is descriptive: guards
must not make publication depend on explanatory wording or a heading inventory.
