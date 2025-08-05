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
