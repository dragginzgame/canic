# Integration Guide (Internal)

This guide explains how to integrate ICU (Internet Computer Utilities) as an internal git dependency in your Rust projects.

## Git Dependency Integration

### Basic Integration

```toml
[dependencies]
# Replace with your internal Git host/namespace and desired tag
icu = { git = "ssh://git@your.git.host/your-group/icu.git", tag = "v0.1.12", features = [] }
```

### With Features

```toml
[dependencies]
icu = {
    git = "ssh://git@your.git.host/your-group/icu.git",
    tag = "v0.1.12",
    features = ["feature1", "feature2"]
}
```

### Development Version

```toml
[dependencies]
# Using a branch for development within the private repo
icu = { git = "ssh://git@your.git.host/your-group/icu.git", branch = "main", features = [] }
```

### Workspace Integration

```toml
[workspace.dependencies]
icu = { git = "ssh://git@your.git.host/your-group/icu.git", tag = "v0.1.12", features = [] }

[dependencies]
icu = { workspace = true }
```

## Available Versions

To see all available versions (private origin):

```bash
git ls-remote --tags ssh://git@your.git.host/your-group/icu.git | grep -o 'refs/tags/v.*' | sed 's/refs\/tags\///' | sort -V
```

## Security Benefits

Using git dependencies with specific tags provides several security benefits:

1. **Immutable versions** - Once tagged, the code cannot change
2. **Reproducible builds** - Same tag always produces same code
3. **Supply chain security** - No dependency on external registries
4. **Version pinning** - Exact version control

## Migration from Registry

If you're migrating from a registry dependency:

```toml
# Before (registry)
icu = "0.1.12"

# After (internal git dependency)
icu = { git = "ssh://git@your.git.host/your-group/icu.git", tag = "v0.1.12" }
```

## Troubleshooting

### Tag Not Found

If you get a "tag not found" error:

1. Check the tag exists: `git ls-remote --tags ssh://git@your.git.host/your-group/icu.git`
2. Ensure the tag format is correct: `v0.1.12` (with the 'v' prefix)
3. Verify your access and the repository URL

### Build Issues

If you encounter build issues:

1. Check the changelog for breaking changes
2. Verify you're using a compatible Rust version
3. Check the repository's CI status for the tag
