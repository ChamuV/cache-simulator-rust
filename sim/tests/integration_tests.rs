use sim::{run_trace, Cache};
use std::fs;

#[test]
/// Integration test: verifies correct handling of all operation types.
///
/// Checks that:
/// - `I` operations are ignored,
/// - `L` and `S` perform a single access,
/// - `M` performs two accesses,
/// and that the resulting hit/miss/eviction counts are correct.
fn run_trace_processes_load_store_modify_and_ignore_correctly() {
  let trace = "\
  I 0400d7,8
  L 10,1
  S 10,1
  M 20,1
  ";

  let path = "tests/test_trace.trace";
  fs::write(path, trace).expect("failed to write test trace");

  let mut cache = Cache::new(1, 1 ,1);
  run_trace(&mut cache, path).expect("run_trace failed");

  fs::remove_file(path).expect("failed to remove test trace");

  assert_eq!(cache.hits, 2);
  assert_eq!(cache.misses, 2);
  assert_eq!(cache.evictions, 1);
}

#[test]
/// Integration test: verifies eviction behaviour for conflicting addresses.
///
/// Two addresses mapping to the same set in a direct-mapped cache
/// should cause a miss followed by an eviction.
fn run_trace_counts_eviction_for_conflicting_blocks() {
  let trace = "\
  L 0,1
  L 4,1
  ";

  let path = "tests/evict_trace.trace";
  fs::write(path, trace).expect("failed to write test trace");

  let mut cache = Cache::new(1, 1, 1);
  run_trace(&mut cache, path).expect("run_trace failed");

  fs::remove_file(path).expect("failed to remove test trace");

  assert_eq!(cache.hits, 0);
  assert_eq!(cache.misses, 2);
  assert_eq!(cache.evictions, 1);
}

#[test]
/// Integration test: verifies robustness to whitespace in trace files.
///
/// Ensures that blank lines and leading spaces are ignored during parsing,
/// and do not affect correctness of cache behaviour.
fn run_trace_handles_blank_lines_and_leading_spaces() {
  let trace = "
  
  L 10,1
  L 10,1
  ";

  let path = "tests/whitespace_trace.trace";
  fs::write(path, trace).expect("failed to write test trace");

  let mut cache = Cache::new(1, 1, 1);
  run_trace(&mut cache, path).expect("run_trace failed");

  fs::remove_file(path).expect("failed to remove test trace");

  assert_eq!(cache.hits, 1);
  assert_eq!(cache.misses, 1);
  assert_eq!(cache.evictions, 0);
}

#[test]
/// Integration test: verifies parsing of hexadecimal addresses.
///
/// Confirms that addresses with the `0x` prefix are correctly parsed
/// and produce the same cache behaviour as standard hexadecimal input.
fn run_trace_accepts_hex_addresses_with_prefix() {
  let trace = "\
  L 0x10,1
  L 0x10,1
  ";

  let path = "tests/hex_trace.trace";
  fs::write(path, trace).expect("failed to write test trace");

  let mut cache = Cache::new(1, 1, 1);
  run_trace(&mut cache, path).expect("run_trace failed");

  fs::remove_file(path).expect("failed to remove test trace");

  assert_eq!(cache.hits, 1);
  assert_eq!(cache.misses, 1);
  assert_eq!(cache.evictions, 0);
}

#[test]
/// Integration test: verifies correct handling of repeated modify operations.
///
/// Each `M` operation corresponds to two accesses (load followed by store),
/// so repeated modify operations should accumulate hits accordingly.
fn run_trace_multiple_modify_operations() {
  let trace = "\
  M 10,1
  M 10,1
  ";

  let path = "tests/multiple_modify.trace";
  fs::write(path, trace).expect("failed to write test trace");

  let mut cache = Cache::new(1, 1, 1);
  run_trace(&mut cache, path).expect("run_trace failed");

  fs::remove_file(path).expect("failed to remove test trace");

  assert_eq!(cache.hits, 3);
  assert_eq!(cache.misses, 1);
  assert_eq!(cache.evictions, 0);
}