#!/usr/bin/env bash
set -euo pipefail

# Shared by ordinary execution and compiled inventory reconciliation.
printf '%s\n' \
    'pic::governed_suite::' \
    'pic::workers::tests::' \
    'pic::fleet_registry::baseline::tests::release_artifacts::' \
    'pic::fleet_registry::baseline::tests::generated_release_cache_restores_fixture_authority_without_source_rebuild' \
    'pic::fleet_registry::baseline::tests::reinstall_fixture_release_cache_binds_distinct_repeatable_identities'
