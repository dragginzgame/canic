#!/bin/bash

# Mimic Versioning System Setup Script
# This script sets up the complete versioning and security system for any Rust repository

set -e

echo "🚀 Setting up Mimic Versioning System..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    print_error "Not in a git repository. Please run this script from the root of your git repository."
    exit 1
fi

# Check if we're in a Rust workspace
if [ ! -f "Cargo.toml" ]; then
    print_error "Cargo.toml not found. Please run this script from the root of your Rust workspace."
    exit 1
fi

print_status "Setting up versioning system for $(basename $(pwd))..."

# Create scripts directory structure
print_status "Creating scripts directory structure..."
mkdir -p scripts/app

# Create version management script
print_status "Creating version management script..."
cat > scripts/app/version.sh << 'EOF'
#!/bin/bash

# Version Management Script for Rust Workspaces
# Handles semantic versioning, changelog updates, and git operations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Get current version from Cargo.toml
get_current_version() {
    grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/'
}

# Bump version according to type
bump_version() {
    local current_version=$1
    local bump_type=$2
    
    IFS='.' read -ra VERSION_PARTS <<< "$current_version"
    local major=${VERSION_PARTS[0]}
    local minor=${VERSION_PARTS[1]}
    local patch=${VERSION_PARTS[2]}
    
    case $bump_type in
        "major")
            echo "$((major + 1)).0.0"
            ;;
        "minor")
            echo "$major.$((minor + 1)).0"
            ;;
        "patch")
            echo "$major.$minor.$((patch + 1))"
            ;;
        *)
            print_error "Invalid bump type: $bump_type"
            exit 1
            ;;
    esac
}

# Update version in Cargo.toml
update_cargo_version() {
    local new_version=$1
    sed -i.bak "s/^version = \".*\"/version = \"$new_version\"/" Cargo.toml
    rm Cargo.toml.bak
    print_success "Updated Cargo.toml to version $new_version"
}

# Update changelog
update_changelog() {
    local new_version=$1
    local current_date=$(date +%Y-%m-%d)
    
    # Create changelog if it doesn't exist
    if [ ! -f "CHANGELOG.md" ]; then
        cat > CHANGELOG.md << CHANGELOG_EOF
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [$new_version] - $current_date

CHANGELOG_EOF
        print_success "Created CHANGELOG.md"
    else
        # Add new version entry
        sed -i.bak "s/## \[Unreleased\]/## [$new_version] - $current_date\n\n## [Unreleased]/" CHANGELOG.md
        rm CHANGELOG.md.bak
        print_success "Added version $new_version to CHANGELOG.md"
    fi
}

# Create git tag
create_git_tag() {
    local version=$1
    local tag_name="v$version"
    
    # Check if tag already exists
    if git tag -l | grep -q "^$tag_name$"; then
        print_error "Tag $tag_name already exists!"
        exit 1
    fi
    
    git tag -a "$tag_name" -m "Release version $version"
    print_success "Created git tag $tag_name"
}

# Check if working directory is clean
check_working_directory() {
    if ! git diff-index --quiet HEAD --; then
        print_error "Working directory is not clean. Please commit or stash your changes first."
        exit 1
    fi
}

# Main command handler
case "${1:-help}" in
    "current")
        print_info "Current version: $(get_current_version)"
        ;;
    "major")
        check_working_directory
        local current_version=$(get_current_version)
        local new_version=$(bump_version "$current_version" "major")
        print_info "Bumping major version: $current_version -> $new_version"
        update_cargo_version "$new_version"
        update_changelog "$new_version"
        git add Cargo.toml CHANGELOG.md
        git commit -m "Bump version to $new_version"
        create_git_tag "$new_version"
        print_success "Version bumped to $new_version"
        ;;
    "minor")
        check_working_directory
        local current_version=$(get_current_version)
        local new_version=$(bump_version "$current_version" "minor")
        print_info "Bumping minor version: $current_version -> $new_version"
        update_cargo_version "$new_version"
        update_changelog "$new_version"
        git add Cargo.toml CHANGELOG.md
        git commit -m "Bump version to $new_version"
        create_git_tag "$new_version"
        print_success "Version bumped to $new_version"
        ;;
    "patch")
        check_working_directory
        local current_version=$(get_current_version)
        local new_version=$(bump_version "$current_version" "patch")
        print_info "Bumping patch version: $current_version -> $new_version"
        update_cargo_version "$new_version"
        update_changelog "$new_version"
        git add Cargo.toml CHANGELOG.md
        git commit -m "Bump version to $new_version"
        create_git_tag "$new_version"
        print_success "Version bumped to $new_version"
        ;;
    "release")
        local version=${2:-$(get_current_version)}
        check_working_directory
        print_info "Creating release for version $version"
        create_git_tag "$version"
        print_success "Release created for version $version"
        ;;
    "tags")
        print_info "Available git tags:"
        git tag --sort=-version:refname | head -10
        print_info "To see all tags: git tag --sort=-version:refname"
        ;;
    "help"|*)
        echo "Usage: $0 {current|major|minor|patch|release|tags}"
        echo ""
        echo "Commands:"
        echo "  current  - Show current version"
        echo "  major    - Bump major version (x.0.0)"
        echo "  minor    - Bump minor version (0.x.0)"
        echo "  patch    - Bump patch version (0.0.x)"
        echo "  release  - Create release with current version"
        echo "  tags     - List available git tags"
        echo "  help     - Show this help message"
        ;;
esac
EOF

chmod +x scripts/app/version.sh

# Create security check script
print_status "Creating security check script..."
cat > scripts/app/security-check.sh << 'EOF'
#!/bin/bash

# Security Check Script for Tag Immutability
# Ensures that tagged versions cannot be modified

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

echo "🔒 Security Check for Versioning"
echo "================================"
echo ""

# Check if working directory is clean
print_info "Checking repository status..."
if git diff-index --quiet HEAD --; then
    print_success "Working directory is clean"
else
    print_error "Working directory is not clean. Please commit or stash your changes first."
    exit 1
fi

# Get current version
current_version=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
print_info "Current version in Cargo.toml: $current_version"

# Check if current version is already tagged
if git tag -l | grep -q "^v$current_version$"; then
    print_warning "CRITICAL: Current version $current_version is already tagged!"
    print_warning "   This means the code at this version should NEVER change."
    
    # Check if HEAD is at the tagged commit
    tagged_commit=$(git rev-list -n 1 "v$current_version")
    head_commit=$(git rev-parse HEAD)
    
    if [ "$tagged_commit" != "$head_commit" ]; then
        print_error "🚨 SECURITY VIOLATION: HEAD is not at the tagged commit!"
        print_error "    This indicates the tag has been modified or HEAD has moved."
        print_error "    The code at version $current_version has changed!"
        print_info "    You MUST bump to a new version immediately."
        exit 1
    else
        print_success "HEAD is at the tagged commit - version integrity maintained"
    fi
else
    print_success "Current version $current_version is not yet tagged"
fi

print_success "Security check passed!"
EOF

chmod +x scripts/app/security-check.sh

# Create Makefile
print_status "Creating Makefile..."
cat > Makefile << 'EOF'
# Makefile for Rust Workspace with Versioning

.PHONY: help version current tags patch minor major release test build check clippy fmt fmt-check clean check-versioning git-versions security-check all

help:
	@echo "Available commands:"
	@echo ""
	@echo "Version Management:"
	@echo "  version          Show current version"
	@echo "  tags             List available git tags"
	@echo "  patch            Bump patch version (0.0.x)"
	@echo "  minor            Bump minor version (0.x.0)"
	@echo "  major            Bump major version (x.0.0)"
	@echo "  release          Create a release with current version"
	@echo ""
	@echo "Development:"
	@echo "  test             Run all tests"
	@echo "  build            Build all crates"
	@echo "  check            Run cargo check"
	@echo "  clippy           Run clippy checks"
	@echo "  fmt              Format code"
	@echo "  clean            Clean build artifacts"
	@echo ""
	@echo "Utilities:"
	@echo "  check-versioning Check versioning system setup"
	@echo "  git-versions     Check available git dependency versions"
	@echo "  security-check   Check tag immutability and version integrity"
	@echo ""
	@echo "Examples:"
	@echo "  make patch       # Bump patch version"
	@echo "  make test        # Run tests"
	@echo "  make build       # Build project"

# Version management
version:
	@./scripts/app/version.sh current

current:
	@./scripts/app/version.sh current

tags:
	@./scripts/app/version.sh tags

patch:
	@./scripts/app/version.sh patch

minor:
	@./scripts/app/version.sh minor

major:
	@./scripts/app/version.sh major

release:
	@./scripts/app/version.sh release

# Development commands
test:
	cargo test --workspace

build:
	cargo build --release --workspace

check:
	cargo check --workspace

clippy:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clean:
	cargo clean
	rm -rf target/

# Check versioning system
check-versioning:
	@echo "Checking versioning system setup..."
	@test -f scripts/app/version.sh || (echo "❌ version.sh not found" && exit 1)
	@test -f scripts/app/security-check.sh || (echo "❌ security-check.sh not found" && exit 1)
	@test -f Makefile || (echo "❌ Makefile not found" && exit 1)
	@test -f Cargo.toml || (echo "❌ Cargo.toml not found" && exit 1)
	@echo "✅ Versioning system setup complete"

# Check available git versions
git-versions:
	@echo "Available versions for git dependencies:"
	@git tag --sort=-version:refname | head -10
	@echo ""
	@echo "Example usage:"
	@echo '  mimic = { git = "https://github.com/your-repo/mimic", tag = "v0.9.4", features = [] }'

# Security check for tag immutability
security-check:
	@./scripts/app/security-check.sh

# Build and test everything
all: clean check fmt-check clippy test build
EOF

# Create GitHub Actions workflows directory
print_status "Creating GitHub Actions workflows..."
mkdir -p .github/workflows

# Create CI workflow
cat > .github/workflows/ci.yml << 'EOF'
name: CI

on:
  pull_request:
    branches: [ main ]
  push:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-
      
      - name: Run tests
        run: cargo test --workspace
      
      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings
      
      - name: Check formatting
        run: cargo fmt --all -- --check

  build:
    name: Build
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-
      
      - name: Build
        run: cargo build --release --workspace
EOF

# Create release workflow
cat > .github/workflows/release.yml << 'EOF'
name: Release

on:
  push:
    tags:
      - 'v*'

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-
      
      - name: Run tests
        run: cargo test --workspace
      
  build:
    name: Build
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-
      
      - name: Build
        run: cargo build --release --workspace

  create-release:
    name: Create GitHub Release
    runs-on: ubuntu-latest
    needs: [test, build]
    if: startsWith(github.ref, 'refs/tags/v')
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      
      - name: Generate release notes
        id: release_notes
        run: |
          # Extract version from tag
          VERSION=${GITHUB_REF#refs/tags/v}
          
          # Generate release notes from changelog
          if [ -f "CHANGELOG.md" ]; then
            RELEASE_NOTES=$(awk -v version="$VERSION" '
              /^## \[' version '\]/ { in_version=1; next }
              /^## \[/ { in_version=0 }
              in_version { print }
            ' CHANGELOG.md | sed '1d' | sed '/^$/d')
          else
            RELEASE_NOTES="Release version $VERSION"
          fi
          
          echo "notes<<EOF" >> $GITHUB_OUTPUT
          echo "$RELEASE_NOTES" >> $GITHUB_OUTPUT
          echo "EOF" >> $GITHUB_OUTPUT
      
      - name: Create Release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref_name }}
          body: ${{ steps.release_notes.outputs.notes }}
          draft: false
          prerelease: false
EOF

# Create initial changelog if it doesn't exist
if [ ! -f "CHANGELOG.md" ]; then
    print_status "Creating initial CHANGELOG.md..."
    current_version=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
    current_date=$(date +%Y-%m-%d)
    
    cat > CHANGELOG.md << EOF
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [$current_version] - $current_date

- Initial versioning system setup

EOF
    print_success "Created CHANGELOG.md"
fi

# Create README section for versioning
print_status "Creating versioning documentation..."

cat > VERSIONING.md << 'EOF'
# Versioning Guide

This document explains how versioning works in this project and how to manage releases.

## Overview

This project uses [Semantic Versioning](https://semver.org/) (SemVer) for all releases. The version format is `MAJOR.MINOR.PATCH` (e.g., `1.2.3`).

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
EOF

# Create integration guide
cat > INTEGRATION.md << 'EOF'
# Integration Guide

This guide explains how to integrate this project as a git dependency in your Rust projects.

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
EOF

echo -e "${GREEN}[SUCCESS]${NC} Versioning system setup complete!"
echo ""
echo -e "${BLUE}[INFO]${NC} Next steps:"
echo "1. Review the created files:"
echo "   - scripts/app/version.sh"
echo "   - scripts/app/security-check.sh"
echo "   - Makefile"
echo "   - .github/workflows/ci.yml"
echo "   - .github/workflows/release.yml"
echo "   - VERSIONING.md"
echo "   - INTEGRATION.md"
echo ""
echo "2. Test the system:"
echo "   make check-versioning"
echo "   make security-check"
echo ""
echo "3. Create your first release:"
echo "   make patch"
echo "   git push --follow-tags"
echo ""
echo -e "${GREEN}[SUCCESS]${NC} Your versioning system is ready to use!" 