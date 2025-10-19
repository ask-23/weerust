# WeeRust

A Rust implementation of WeeWX weather station software.

## Overview

This repository contains the Rust port of WeeWX, a weather station software package originally written in Python. The Rust implementation aims to provide better performance, memory safety, and modern async capabilities while maintaining compatibility with the original WeeWX ecosystem.

## Architecture

The project is organized as a Cargo workspace with the following crates:

- **weex-core**: Core data types, units, and common functionality
- **weex-db**: Database abstraction and operations
- **weex-ingest**: Data ingestion from weather stations and drivers
- **weex-archive**: Data archiving and aggregation
- **weex-daemon**: Main daemon process that orchestrates the system

## Building

To build the project:

```bash
cargo build
```

To run tests:

```bash
cargo test
```

To run the daemon:

```bash
cargo run --bin weex-daemon
```

## License

This project is licensed under the GPL-3.0 license, same as the original WeeWX project.

## Original Project

This is a Rust port of [WeeWX](https://weewx.com/), the original Python weather station software.
