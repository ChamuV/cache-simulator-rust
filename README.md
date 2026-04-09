# Cache memory simulator (Rust)

## Overview

This project implements a configurable cache simulator in Rust that processes memory trace files and reports cache performance statistics.

The simulator supports both:
- a **single-level cache** using a Least Recently Used (LRU) replacement policy
- an optional **two-level (L1/L2) exclusive cache hierarchy**

It records:
- cache hits
- cache misses
- cache evictions

The implementation is modular, separating address parsing, cache representation, and access logic, and is designed to be extensible to more advanced cache architectures.

## Features

- Configurable cache parameters:
  - `s`: number of set index bits
  - `E`: number of lines per set 
  - `b`: number of block offset bits
- LRU replacement policy using timestamp tracking
- Optional **two-level exclusive cache hierarchy (L1/L2)**
- Support for memory traces
- Handles operations:
  - `L` (load)
  - `S` (store)
  - `M` (modify = load + store)
  - `I` (ignored)
- Unit, integration and property-based testing

## Project Structure

```
.
|-- src/
|   |-- lib.rs
|   `-- main.rs
|-- tests/
|   |-- unit_tests.rs
|   |-- integration_tests.rs
|   `-- property_tests.rs
|-- README.md
`-- TESTING.md
```

## How to build

Ensure that Rust is installed, then run:

```bash
cargo build
```

## How to run

### Single-level cache

```bash
cargo run -- -s <s> -E <E> -b <b> -t <tracefile>
```

### Two-level cache (L1 + L2)

```bash
cargo run -- --multi \
  -s <l1_s> -E <l1_E> -b <l1_b> \
  --l2-s <l2_s> --l2-E <l2_E> --l2-b <l2_b> \
  -t <tracefile>
```

### Arguments

#### Common
- `-t`: path to the trace file

#### L1 (single-level or multi-level)
- `-s`: number of set index bits
- `-E`: number of lines per set
- `-b`: number of block offset bits

#### Multi-level only
- `--multi`: enable two-level cache simulation
- `--l2-s`: number of L2 set index bits
- `--l2-E`: number of lines per set in L2
- `--l2-b`: number of block offset bits in L2

### Examples

Single-level:
```bash
cargo run -- -s 2 -E 1 -b 4 -t ../traces/yi.trace
```

Two-level:
```bash
cargo run -- --multi \
  -s 2 -E 1 -b 4 \
  --l2-s 3 --l2-E 2 --l2-b 4 \
  -t ../traces/yi.trace
```

## Output

The simulator prints:

```text
hits:<num> misses:<num> evictions:<num>
```

## Testing
Run all tests with:
```bash
cargo test
```

The test suite includes:
  - Unit tests for individual components
  - Integration tests for full execution behaviour
  - Property-based tests for invariant checking

## Notes

- The simulator supports both single-level and two-level cache configurations
- The multi-level mode implements an **exclusive hierarchy**:
  - blocks reside in either L1 or L2, not both
  - L2 hits trigger promotion to L1
  - L1 evictions are demoted to L2
- Instruction loads (`I`) are ignored
- Memory accesses are assumed to be aligned within a block
- Read and write operations are treated identically
- Trace files may be located outside the `sim` directory, so paths may need `..`