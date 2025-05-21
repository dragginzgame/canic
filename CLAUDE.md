# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ICU (Internet Computer Utilities) is a Rust framework for simplifying the development and management of multi-canister systems on the DFINITY Internet Computer (IC). It provides utilities and macros to coordinate multiple canisters (smart contracts) working together, making it easier to create complex canister-based dapps that scale across canister boundaries and subnets.

## Common Commands

### Build Commands

```bash
# Build the project
cargo build

# Build with optimizations
cargo build --release

# Check if the code compiles without producing binaries
cargo check
```

### Run Commands

The framework is primarily meant to be used as a library in IC canister projects, so there are no specific run commands.

### Test Commands

```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test --package icu -- memory::registry

# Run a specific test
cargo test --package icu -- memory::registry::test_registry_init

# Run tests with output
cargo test -- --nocapture
```

### Lint Commands

```bash
# Check for linting issues
cargo clippy
```

## Architecture Overview

ICU is structured around several key concepts:

### Canister Hierarchy

ICU establishes a clear canister hierarchy with a **root** canister orchestrating child canisters. The root canister manages:
- Creation of child canisters
- Upgrades of child canisters
- Cross-canister state management

### Memory Management

ICU provides a structured approach to stable memory management:

1. **Registry**: Manages memory allocations with unique IDs, introduced in v0.1.3
2. **App State**: Stores application-level state that can be shared across canisters
3. **Canister State**: Stores individual canister state
4. **Child Index**: Maintains relationships between parent and child canisters
5. **Subnet Index**: Tracks subnets and their relationships

The `icu_register_memory!` macro handles registration of memory areas in the registry.

### Cascading Updates

The cascade system allows state changes to propagate through the canister hierarchy:

1. `app_state_cascade`: Propagates application state changes to all child canisters
2. `subnet_index_cascade`: Propagates subnet index updates to all child canisters

### Macros

ICU provides several macros that simplify canister development:

1. `icu_start`: Initializes a child canister with references to its parent and root
2. `icu_start_root`: Initializes a root canister
3. `icu_endpoints`: Adds standard ICU endpoints to a canister
4. `icu_endpoints_root`: Adds root-specific endpoints
5. `icu_register_memory`: Registers a memory area in the registry

### Cross-Canister Communication

ICU provides a structured approach to cross-canister calls:

1. **Request/Response Pattern**: Facilitates structured communication between canisters
2. **Call Interface**: Wrapper around IC inter-canister calls with retry logic and error handling

## Key Modules

- `auth`: Authentication and authorization mechanisms
- `config`: Configuration management
- `guard`: Guard functions for endpoint security
- `ic`: Wrappers and extensions to the IC CDK
- `interface`: Higher-level interfaces for memory, state, and inter-canister communication
- `macros`: Macros that simplify canister development
- `memory`: Stable memory management
- `state`: State management for canisters and the overall application
- `serialize`: Serialization utilities

## Development Workflow

1. Make changes to the codebase
2. Use `cargo check` to verify code compiles
3. Use `cargo clippy` to check for linting issues
4. Write/update tests as appropriate
5. Run tests with `cargo test`
6. Build with `cargo build --release` for optimized builds

## Versioning

ICU follows semantic versioning:
- MAJOR: Breaking changes to the API
- MINOR: Backwards-compatible feature additions
- PATCH: Backwards-compatible bug fixes

The changelog tracks version history and notable changes.