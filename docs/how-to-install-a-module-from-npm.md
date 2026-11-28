---
title: Install a module from npm
type: how-to
description: Install public module packages and inspect the modules in the local store.
---

# Install a module from npm

Install a public module package into the Quoin user store, then confirm that Quoin can list it.

## Before you start

Use Quoin 0.38.2. Run the install commands from any directory. Each module stays in the user configuration store, outside the project repository.

Follow these steps:

1. Install the dependency module.
2. Install the module that imports it.
3. Check the installed modules.

## Install the packages

One package imports types from the other, so install the dependency first:

```sh
quoin module install package:@agent-ix/spec-artifacts-iso@0.28.0
quoin module install package:@agent-ix/spec-artifacts-app@0.10.1
```

Quoin downloads each package from the public npm registry, validates its module, and records the install in the user store.

## Confirm the install

```sh
quoin module list
```

The list shows the installed module names and their sources. To repair default modules that differ from the CLI pins, follow the [module setup steps](regulated-baseline.md#step-1-prepare-the-module-store).

## Result

The list includes both installed packages and identifies their npm sources.
