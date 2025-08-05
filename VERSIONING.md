# Versioning Guide

This document explains how versioning works in ICU (Internet Computer Utilities) and how to manage releases.

## Overview

ICU uses [Semantic Versioning](https://semver.org/) (SemVer) for all releases. The version format is `MAJOR.MINOR.PATCH` (e.g., `1.2.3`).

- **MAJOR**: Breaking changes that require migration
- **MINOR**: New features in a backward-compatible manner
- **PATCH**: Backward-compatible bug fixes

## Current Version

The current version is managed in the workspace `Cargo.toml` file and is shared across all crates in the workspace.

## Version Management Script

We provide a convenient script for managing versions: `scripts/app/version.sh`

### Usage

```bash
# Show current version
./scripts/app/version.sh current

# Bump patch version (0.9.3 -> 0.9.4)
./scripts/app/version.sh patch

# Bump minor version (0.9.3 -> 0.10.0)
./scripts/app/version.sh minor

# Bump major version (0.9.3 -> 1.0.0)
./scripts/app/version.sh major

# Create a release with current version
./scripts/app/version.sh release

# Create a release with specific version
./scripts/app/version.sh release 1.0.0
```

### What the script does

1. **Version bumping**: Updates the version in `Cargo.toml`
2. **Changelog updates**: Adds a new version entry to `CHANGELOG.md`
3. **Git operations**: Creates a commit and git tag
4. **Validation**: Ensures working directory is clean before proceeding

## Release Process

### 1. Prepare for Release

Before creating a release:

1. Ensure all changes are committed
2. Update the changelog with your changes
3. Test thoroughly

### 2. Create Release

```bash
# For a patch release (bug fixes)
./scripts/app/version.sh patch

# For a minor release (new features)
./scripts/app/version.sh minor

# For a major release (breaking changes)
./scripts/app/version.sh major
```

### 3. Push Release

```bash
git push --follow-tags
```

This will:
- Push the version bump commit
- Push the git tag
- Trigger the GitHub Actions release workflow

## Automated Release Workflow

When you push a version tag (e.g., `v1.0.0`), the following happens automatically:

1. **Testing**: All tests run
2. **Building**: Release builds are created
3. **GitHub Release**: A GitHub release is created with notes from the changelog

## Changelog Management

The changelog follows the [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
## [Unreleased]
- New features in development

## [1.0.0] - 2024-01-15
- Breaking changes
- New features
- Bug fixes
```

### Adding Changes

When making changes, add them to the `[Unreleased]` section:

```markdown
## [Unreleased]
- Added new feature X
- Fixed bug in Y
- Breaking: Changed API for Z
```

## Security & Tag Immutability

### 🔒 Tag Immutability

**CRITICAL**: Once a version tag is pushed, the code at that version must NEVER change. This is essential for:

- **Reproducible builds** - Users can trust that `v1.0.0` always contains the same code
- **Security** - Prevents supply chain attacks through tag modification
- **Dependency integrity** - Git dependencies remain stable and predictable

### Security Checks

```bash
# Check tag immutability and version integrity
make security-check

# This will detect:
# - Modified tags
# - Unauthorized changes to tagged versions
# - Force-pushed tags
# - Broken or invalid tags
```

## Best Practices

1. **Always update the changelog** before creating a release
2. **Test thoroughly** before releasing
3. **Use semantic versioning** correctly
4. **Create meaningful commit messages** for version bumps
5. **Review the automated release** after pushing tags
6. **Never modify existing tags** - always bump to a new version
7. **Run security checks** regularly with `make security-check`
