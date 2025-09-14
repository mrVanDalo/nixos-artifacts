# NixOS Artifacts Store

[Documentation](https://mrvandalo.github.io/nixos-artifacts/)

## Overview

Nixos-artifacts is a framework to unify artifacts and secrets in NixOS flakes.

Inspired by:

- [Clan vars](https://docs.clan.lol/guides/vars-backend/)
- [NixOS PR #370444](https://github.com/NixOS/nixpkgs/pull/370444)

> **Note:** This project is currently in the design phase.

## Core Concept

NixOS-artifacts provides an abstraction layer over various secret management
backends, including:

- [agenix](https://github.com/ryantm/agenix)
- [sops-nix](https://github.com/Mic92/sops-nix) (not yet)
- [colmena](https://github.com/zhaofengli/colmena) (not yet)

### Key Features

- **Standardized Interface**: Common API for defining and managing secrets
- **Secret Rotation**: Built-in workflow for secret generation and rotation
- **Multi-Backend Support**: Mix different backends within the same
  configuration. You can choose different backends for each artifact.

### Limitations

To maintain compatibility across backends, some specialized features of
individual backends may not be accessible directly (e.g., public vars in
[clan](https://docs.clan.lol/concepts/generators/)).

### Implementation

Each backend is provided as a separate flake that you can add to your
configuration as needed.
