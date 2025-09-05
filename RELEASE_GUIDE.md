# Internal Release Guide

Simple, internal-only release flow for ICU.

## Bump Version

Use the helper targets (shared workspace version):

```bash
make patch    # X.Y.Z -> X.Y.(Z+1)
make minor    # X.Y.Z -> X.(Y+1).0
make major    # X.Y.Z -> (X+1).0.0
```

Or explicitly:

```bash
./scripts/app/version.sh release 1.2.3
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

