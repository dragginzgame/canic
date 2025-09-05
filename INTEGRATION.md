# Integration (Internal)

Private repository. Minimal integration notes for internal use.

## Option A: Workspace member (same repo)

Add the crate as a member and depend via workspace:

```toml
[workspace]
members = ["crates/icu", /* your crates */]

[workspace.dependencies]
icu = { path = "crates/icu" }

[dependencies]
icu = { workspace = true }
```

## Option B: Private git dependency (separate repo)

Pin to a tag on your internal Git host:

```toml
[dependencies]
icu = { git = "ssh://git@your.git.host/your-group/icu.git", tag = "v0.6.6" }
```

List tags (internal origin):

```bash
git ls-remote --tags ssh://git@your.git.host/your-group/icu.git | sed -n 's#.*refs/tags/##p' | sort -V
```

## Notes
- Use tags (vX.Y.Z), not branches, for builds.
- For help, ping the maintainers in the internal channel.

