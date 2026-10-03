# Canic Documentation

Use this page as the front door. Current guides describe supported behavior;
designs explain decisions; contracts define exact boundaries; operations pages
tell an operator what to do.

## Get Started

| If you want to… | Read… |
| --- | --- |
| Install the matching CLI and runtime toolchain | [Install Canic](../INSTALLING.md) |
| Build the smallest complete managed shape | [Minimal managed Fleet](getting-started/minimal-managed-fleet.md) |
| Define roles, Specs, Groups, deployments and services | [Configuration](../CONFIG.md) |
| Explore checked-in application examples | [Reference Apps](../apps/README.md) |
| Understand one capability and its boundary | [Feature guides](features/README.md) |

## Operate A Fleet

| Task | Guide |
| --- | --- |
| Find a command or JSON surface | [CLI guide](../crates/canic-cli/README.md) |
| Generate, review, apply or resume desired state | [Fleet ensure](features/operations/fleet-ensure.md) |
| Diagnose funding or conversion recovery | [Fleet funding](operations/fleet-funding.md) |
| Hand verified bindings to a browser frontend | [Frontend handoff](features/operations/frontend-handoff.md) |
| Run a persistent local PocketIC Fleet | [Local Fleet](features/operations/local-development-fleet.md) |
| Find recovery and release runbooks | [Operations index](operations/README.md) |

## Develop And Review

| Need | Source |
| --- | --- |
| Repository rules and targeted-test policy | [Contributor rules](../AGENTS.md) and [testing guide](../TESTING.md) |
| System shape and exact ownership boundaries | [Architecture](architecture/README.md) and [contracts](contracts/ARCHITECTURE.md) |
| Latest implementation and qualification handoff | [Current status](status/current.md) |
| Accepted sequence and future design work | [Design roadmap](design/README.md) |
| Published history and the open patch draft | [Changelog](../CHANGELOG.md) |

## Reading Order And Authority

1. Start with a feature or operator guide for orientation.
2. Follow its configuration, contract or architecture links for exact rules.
3. Check current status for work-in-progress and qualification limits.
4. Treat archived designs, dated audits and older release notes as historical
   evidence, not current product support.

Use current guides for commands and schemas. Archived designs, dated audits and
older release notes describe their recorded checkpoints; they do not establish
support in the current release. Planned work is not an installed capability.
