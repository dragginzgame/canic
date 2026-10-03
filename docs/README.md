# Canic Documentation

This is the front door to Canic's documentation. You do not need to understand
Canic's internal architecture before getting started.

- **Guides** explain a task or feature in practical terms.
- **Operations pages** give step-by-step procedures for people running Canic.
- **Architecture documents** explain why the system is designed as it is.
- **Contracts** define exact technical and security rules for implementers.
- **Designs and audits** record planned work and evidence; they are not beginner
  tutorials.

## Get Started

| If you want to… | Read… |
| --- | --- |
| Understand why Canic exists and how its pieces fit together | [How Canic works](getting-started/how-canic-works.md) |
| Install the matching CLI and runtime toolchain | [Install Canic](../INSTALLING.md) |
| Build a small application and its management canisters | [First managed application](getting-started/minimal-managed-fleet.md) |
| Describe an application's canisters and layout | [Configuration](../CONFIG.md) |
| Explore checked-in application examples | [Reference Apps](../apps/README.md) |
| Understand one capability and its boundary | [Feature guides](features/README.md) |

## Operate A Deployed Application

Canic calls one running copy of an application a **Fleet**. These guides are
for people who build, inspect, deploy, recover, or remove a Fleet.

| Task | Guide |
| --- | --- |
| Find a command or JSON surface | [CLI guide](../crates/canic-cli/README.md) |
| Generate, review, apply or resume desired state | [Fleet ensure](features/operations/fleet-ensure.md) |
| Diagnose funding or conversion recovery | [Fleet funding](operations/fleet-funding.md) |
| Hand verified bindings to a browser frontend | [Frontend handoff](features/operations/frontend-handoff.md) |
| Run a persistent local test network with PocketIC | [Local Fleet](features/operations/local-development-fleet.md) |
| Find recovery and release runbooks | [Operations index](operations/README.md) |

## Understand Or Contribute To Canic

| Need | Source |
| --- | --- |
| Repository rules and targeted-test policy | [Contributor rules](../AGENTS.md) and [testing guide](../TESTING.md) |
| System shape and exact ownership boundaries | [Architecture](architecture/README.md) and [contracts](contracts/ARCHITECTURE.md) |
| Latest implementation and qualification handoff | [Current status](status/current.md) |
| Accepted sequence and future design work | [Design roadmap](design/README.md) |
| Published history and the open patch draft | [Changelog](../CHANGELOG.md) |

## How To Read These Docs

1. Start with a feature guide or task guide for an overview.
2. Follow its configuration or operations links when you are ready to use it.
3. Use contracts and architecture pages when you need exact implementation
   rules.
4. Check current status for work in progress and known limits.
5. Treat archived designs, dated audits and older release notes as historical
   evidence, not current product support.

Use current guides for commands and schemas. Archived designs, dated audits and
older release notes describe their recorded checkpoints; they do not establish
support in the current release. Planned work is not an installed capability.
