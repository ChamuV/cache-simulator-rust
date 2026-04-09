# Testing Strategy

The cache simulator was tested using multiple testing approaches available in Rust.  
The design of the tests was informed by the University of London laboratory exercises on Rust as well as general testing practices described in the article:

https://zerotomastery.io/blog/complete-guide-to-testing-code-in-rust/

The test suite is organised as a layered strategy covering both the original single-level cache simulator and the extended two-level cache hierarchy.

## 1. Unit Tests

Unit tests were used to verify the correctness of individual components of the simulator.  
These tests check low-level functionality such as bit masking, address parsing, cache line and set construction, cache initialisation, basic cache access behaviour, LRU replacement, and command-line argument parsing.

The unit test suite was also extended to cover the new configuration options for multi-level execution, ensuring that both single-level and two-level command-line modes are parsed correctly.

## 2. Integration Tests

Integration tests verify that different parts of the simulator work together correctly.  
These tests simulate the processing of small memory traces and check that the resulting hit, miss, and eviction counts are correct.

They were used to confirm correct handling of load, store, modify, and ignored instruction operations, as well as whitespace robustness and hexadecimal input parsing.

## 3. Property-Based Tests

Property-based testing is used to check general properties of the simulator rather than only specific example inputs.  
The `proptest` crate generates random addresses and cache parameters to validate logical invariants of the shared address parsing process.

In particular, these tests verify that decomposed addresses reconstruct correctly and that computed set indices remain within valid bounds.

## 4. Multi-Level Cache Tests

Additional tests were introduced for the two-level exclusive cache hierarchy.  
These tests verify behaviour specific to the extended design, including:

- first access misses in the hierarchy
- repeated access hits in L1
- eviction from L1 causes demotion to L2
- an L2 hit promotes a block back to L1
- the hierarchy remains exclusive, so a block does not remain in both L1 and L2 after promotion
- overflow in L2 is counted as a hierarchy eviction

These tests provide confidence that the hierarchy coordination logic works correctly in addition to the original single-level simulator behaviour.

## Summary

The simulator therefore uses a layered testing strategy consisting of:

- Unit tests
- Integration tests
- Property-based tests
- Multi-level cache tests

This approach increases confidence that the simulator behaves correctly across a range of inputs and configurations, including both single-level and extended multi-level modes.