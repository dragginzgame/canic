# Internal Release Guide

Simple, internal-only release flow for ICU.

## Bump Version

Use the helper targets (shared workspace version):

```bash
make patch    # X.Y.Z -> X.Y.(Z+1)
make minor    # X.Y.Z -> X.(Y+1).0
make major    # X.Y.Z -> (X+1).0.0
```

Or manually set a specific version and tag:

```bash
sed -i 's/^version = ".*"/version = "1.2.3"/' Cargo.toml
git add Cargo.toml
git commit -m "Bump version to 1.2.3"
git tag -a v1.2.3 -m "Release 1.2.3"
```

## Prepare & Verify

```bash
make fmt-check
make clippy
make test
```

Update `CHANGELOG.md` under `[Unreleased]` before bumping.

## Tag & Push

The version script creates a tag. Push with:

```bash
git push --follow-tags
```

CI runs tests/builds on tags. No external publishing.

## Integration (Internal Git)

Pin to a tag from your internal host:

```toml
[dependencies]
icu = { git = "ssh://git@your.git.host/your-group/icu.git", tag = "vX.Y.Z" }
```

List tags:

```bash
git tag --sort=-version:refname
```

## Notes
- Tags are immutable; never modify existing tags.
- For help, contact maintainers in the internal channel.
