# PatMat

![PatMat](assets/patmat.gif)

PatMat is a build tool whose recipes are meant to be written with the ease of a Makefile but with the flexibility of a script

## Features

- **Target dependency graph**: Targets declare prerequisites and run asynchronously in dependency order.
- **Built-in tasks**: Git clone, branch checkout, pull, filesystem operations, and embedded terminal commands.
- **Live GUI status**: Displays target hierarchy, active state, task output, and error traces in real time.
- **YAML configurator**: Generates interactive form menus for editing YAML configurations.

## Architecture

- **Targets and Tasks**: A `Target` defines an execution node with dependencies and an associated `Task_trait`.
- **Reactive UI**: The `Builder` widget renders the dependency tree, selection details, and active widgets via Vizual stores.
- **Terminal Task**: Runs commands with live terminal output embedded in the UI panel.

## Technologies used

- [vizual](https://github.com/ElectricPulse/vizual) for UI components and reactive layout
- [gix](https://github.com/GitoxideLabs/gitoxide) for Git operations
- [tokio](https://github.com/tokio-rs/tokio) for asynchronous runtime
