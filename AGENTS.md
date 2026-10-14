# Project Overview
devpack-for-rust builds a snap package (https://snapcraft.io/docs/) containing the devpack-for-rust installer.  It ensures that the snap package builds successfully, passes all tests and lints, and can be published to the Snap store at the snapcraft.io website.  Snap package tests use the spread (https://github.com/canonical/spread) tool.  The devpack-for-rust installer is written in Rust and provides an easy-to-use terminal user interface (TUI) selector tool where the user can select and configure tools to install for Rust software development.

## Folder Structure
- `snapcraft.yaml`: snap package build configuration
- `spread.yaml`: snap package test configuration
- `/spread`: snap package tests
- `/src`: top-level Rust source code folder
- `/test`: top-level folder for integration tests

## Coding Standards
- Follow idiomatic Rust practices and community standards as defined in `rust-instructions.md`.

