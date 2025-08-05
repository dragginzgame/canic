# Integration Guide

This guide explains how to integrate ICU (Internet Computer Utilities) as a git dependency in your Rust projects.

## Git Dependency Integration

### Basic Integration

```toml
[dependencies]
icu = { git = "https://github.com/dragginzgame/icu", tag = "v0.1.10", features = [] }
```

### With Features

```toml
[dependencies]
icu = { 
    git = "https://github.com/dragginzgame/icu", 
    tag = "v0.1.10", 
    features = ["feature1", "feature2"] 
}
```

### Development Version

```toml
[dependencies]
icu = { git = "https://github.com/dragginzgame/icu", branch = "main", features = [] }
```

### Workspace Integration

```toml
[workspace.dependencies]
icu = { git = "https://github.com/dragginzgame/icu", tag = "v0.1.10", features = [] }

[dependencies]
icu = { workspace = true }
```

## Available Versions

To see all available versions:

```bash
git ls-remote --tags https://github.com/dragginzgame/icu | grep -o 'refs/tags/v.*' | sed 's/refs\/tags\///' | sort -V
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
icu = "0.1.10"

# After (git dependency)
icu = { git = "https://github.com/dragginzgame/icu", tag = "v0.1.10" }
```

## Troubleshooting

### Tag Not Found

If you get a "tag not found" error:

1. Check the tag exists: `git ls-remote --tags https://github.com/dragginzgame/icu`
2. Ensure the tag format is correct: `v0.1.10` (with the 'v' prefix)
3. Verify the repository URL is correct

### Build Issues

If you encounter build issues:

1. Check the changelog for breaking changes
2. Verify you're using a compatible Rust version
3. Check the repository's CI status for the tag
