# Mixed-Size Placement Database in Rust

This project provides a **database for Mixed-Size Placement (MSP)** written in Rust. It is designed to be integrated into other projects, offering a high-performance, Rust-native data structure for VLSI placement tasks.

It leverages [`reda-lefdef`](https://github.com/giammirove/reda-lefdef) to **parse LEF/DEF files**, ensuring accurate import of standard VLSI design layouts and netlists.

## Features

- **Rust-native DB** for Mixed-Size Placement
- Supports **standard LEF/DEF formats** via `reda-lefdef`
- Designed for **research and hobbyist use**, but structured to integrate with larger EDA pipelines
- Handles **mixed-size macros, standard cells, and IO pins**
- Easy access to instance positions, sizes, pin locations, and nets

## Installation

Add this project as a dependency in your `Cargo.toml`:

```toml
[dependencies]
reda-db = { git = "https://github.com/giammirove/reda-db.git" }
