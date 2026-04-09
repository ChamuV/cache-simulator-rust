use sim::{low_mask, parse_address, parse_args_from, Cache, CacheLine, CacheSet};

#[test]
/// Unit test: verifies that low_mask(bits) sets the lowest `bits` bits to 1.
///
/// This checks the helper used to extract block offsets and set indices
/// from memory addresses.
fn low_mask_table_driven_cases() {
    let cases = vec![
        (0usize, 0u64),
        (1usize, 0b1u64),
        (2usize, 0b11u64),
        (3usize, 0b111u64),
        (4usize, 0b1111u64),
    ];

    for (bits, expected) in cases {
        assert_eq!(low_mask(bits), expected, "failed for bits = {bits}");
    }
}

#[test]
/// Unit test: verifies that parse_address correctly decomposes addresses
/// into (tag, set index, block offset) for several hand-worked examples.
///
/// This confirms the correctness of the simulator's address splitting logic
/// across different values of `s` and `b`.
fn parse_address_table_driven_cases() {
    let cases = vec![
        // (address, s, b, expected_tag, expected_set, expected_block)
        (0b110110u64, 2usize, 2usize, 0b11u64, 0b01usize, 0b10u64),
        (0b101101u64, 1usize, 2usize, 0b101u64, 0b01usize, 0b01u64),
        (0b111000u64, 2usize, 1usize, 0b111u64, 0b00usize, 0b0u64),
        (0b1001110u64, 3usize, 1usize, 0b100u64, 0b111usize, 0b0u64),
    ];

    for (address, s, b, exp_tag, exp_set, exp_block) in cases {
        let (tag, set_index, block_offset) = parse_address(address, s, b);

        assert_eq!(tag, exp_tag, "wrong tag for address {address:b}");
        assert_eq!(set_index, exp_set, "wrong set index for address {address:b}");
        assert_eq!(block_offset, exp_block, "wrong block offset for address {address:b}");
    }
}

#[test]
/// Unit test: verifies that CacheLine::empty creates an invalid cache line
/// with zeroed tag and timestamp fields.
///
/// This checks the default initial state used when constructing sets.
fn cache_line_empty_returns_zeroed_invalid_line() {
    let line = CacheLine::empty();

    assert!(!line.valid, "empty line should be invalid");
    assert_eq!(line.tag, 0, "empty line should have tag 0");
    assert_eq!(line.last_used, 0, "empty line should have last_used 0");
}

#[test]
/// Unit test: verifies that CacheSet::new creates the requested number of
/// empty cache lines.
///
/// This ensures that set construction works correctly for different
/// associativity values.
fn cache_set_new_table_driven_cases() {
    let cases = vec![1usize, 2usize, 4usize, 8usize, 16usize];

    for e in cases {
        let set = CacheSet::new(e);

        assert_eq!(set.lines.len(), e, "wrong number of lines for e = {e}");

        for (idx, line) in set.lines.iter().enumerate() {
            assert!(!line.valid, "line {idx} in set with e = {e} should be invalid");
            assert_eq!(line.tag, 0, "line {idx} in set with e = {e} should have tag 0");
            assert_eq!(
                line.last_used,
                0,
                "line {idx} in set with e = {e} should have last_used 0"
            );
        }
    }
}

#[test]
/// Unit test: verifies that Cache::new initialises the expected number of sets,
/// each with the correct number of lines, and resets all counters to zero.
///
/// This checks that the cache structure is built correctly for several
/// configurations.
fn cache_new_table_driven_cases() {
    let cases = vec![
        // (s, b, e, expected_sets)
        (0usize, 0usize, 1usize, 1usize),
        (1usize, 1usize, 1usize, 2usize),
        (2usize, 1usize, 2usize, 4usize),
        (3usize, 2usize, 4usize, 8usize),
        (4usize, 0usize, 2usize, 16usize),
    ];

    for (s, b, e, expected_sets) in cases {
        let cache = Cache::new(s, b, e);

        assert_eq!(cache.s, s, "wrong s value");
        assert_eq!(cache.b, b, "wrong b value");
        assert_eq!(cache.sets.len(), expected_sets, "wrong number of sets");
        assert_eq!(cache.time, 0, "time should start at 0");
        assert_eq!(cache.hits, 0, "hits should start at 0");
        assert_eq!(cache.misses, 0, "misses should start at 0");
        assert_eq!(cache.evictions, 0, "evictions should start at 0");

        for (set_index, set) in cache.sets.iter().enumerate() {
            assert_eq!(
                set.lines.len(),
                e,
                "set {set_index} has wrong number of lines for e = {e}"
            );
        }
    }
}

#[test]
/// Unit test: verifies basic cache access behaviour across several small scenarios.
///
/// The table includes:
/// - a cold miss,
/// - a repeated hit,
/// - a conflicting access causing eviction,
/// - a multi-line set without eviction,
/// - and a modify-style repeated access pattern.
fn cache_access_table_driven_cases() {
    let cases = vec![
        // description, s, b, e, addresses, expected_hits, expected_misses, expected_evictions
        (
            "first access to empty cache is a miss",
            1usize, 1usize, 1usize,
            vec![0x10u64],
            0u64, 1u64, 0u64,
        ),
        (
            "second access to same address becomes a hit",
            1usize, 1usize, 1usize,
            vec![0x10u64, 0x10u64],
            1u64, 1u64, 0u64,
        ),
        (
            "direct mapped conflicting addresses cause eviction",
            1usize, 1usize, 1usize,
            vec![0x0u64, 0x4u64],
            0u64, 2u64, 1u64,
        ),
        (
            "same set, two-line cache fills without eviction",
            0usize, 1usize, 2usize,
            vec![0x0u64, 0x2u64],
            0u64, 2u64, 0u64,
        ),
        (
            "modify-style repeated access pattern gives extra hits",
            1usize, 1usize, 1usize,
            vec![0x10u64, 0x10u64, 0x20u64, 0x20u64],
            2u64, 2u64, 1u64,
        ),
    ];

    for (description, s, b, e, addresses, exp_hits, exp_misses, exp_evictions) in cases {
        let mut cache = Cache::new(s, b, e);

        for address in addresses {
            cache.access(address);
        }

        assert_eq!(cache.hits, exp_hits, "{description}: wrong hit count");
        assert_eq!(cache.misses, exp_misses, "{description}: wrong miss count");
        assert_eq!(
            cache.evictions,
            exp_evictions,
            "{description}: wrong eviction count"
        );
    }
}

#[test]
/// Unit test: verifies that the Least Recently Used replacement policy is applied
/// correctly when a full set receives a conflicting access.
///
/// After making tag 0 the most recently used line, inserting a new tag should
/// evict tag 1 instead.
fn lru_replaces_least_recently_used_line() {
    let mut cache = Cache::new(0, 1, 2);

    cache.access(0x0); // miss, tag 0
    cache.access(0x2); // miss, tag 1
    cache.access(0x0); // hit, tag 0 becomes most recently used
    cache.access(0x4); // miss + eviction, tag 1 should be evicted

    assert_eq!(cache.hits, 1);
    assert_eq!(cache.misses, 3);
    assert_eq!(cache.evictions, 1);

    let tags: Vec<u64> = cache.sets[0]
        .lines
        .iter()
        .filter(|line| line.valid)
        .map(|line| line.tag)
        .collect();

    assert!(tags.contains(&0), "tag 0 should remain in cache");
    assert!(tags.contains(&2), "new tag should be inserted into cache");
    assert!(!tags.contains(&1), "least recently used tag should be evicted");
}

#[test]
/// Unit test: verifies that valid single-level command-line argument
/// combinations are parsed correctly into a Config structure.
///
/// This checks that the helper function used for argument parsing behaves
/// correctly across several input combinations, and that single-level mode
/// leaves all L2 fields unset.
fn parse_args_from_accepts_valid_single_level_argument_cases() {
    let cases = vec![
        (
            vec!["sim", "-s", "1", "-E", "1", "-b", "1", "-t", "trace1.txt"],
            1usize, 1usize, 1usize, "trace1.txt"
        ),
        (
            vec!["sim", "-s", "2", "-E", "4", "-b", "3", "-t", "trace2.txt"],
            2usize, 4usize, 3usize, "trace2.txt"
        ),
        (
            vec!["sim", "-s", "3", "-E", "2", "-b", "2", "-t", "trace3.txt"],
            3usize, 2usize, 2usize, "trace3.txt"
        ),
        (
            vec!["sim", "-s", "3", "-E", "3", "-b", "3", "-t", "trace4.txt"],
            3usize, 3usize, 3usize, "trace4.txt"
        ),
        (
            vec!["sim", "-s", "1", "-E", "10", "-b", "5", "-t", "trace5.txt"],
            1usize, 10usize, 5usize, "trace5.txt"
        ),
    ];

    for (raw_args, exp_s, exp_e, exp_b, exp_trace) in cases {
        let args: Vec<String> = raw_args.into_iter().map(String::from).collect();

        let cfg = parse_args_from(&args).expect("expected valid arguments to parse");

        assert!(!cfg.multi, "single-level mode should not enable multi");
        assert_eq!(cfg.s, exp_s, "wrong parsed s value");
        assert_eq!(cfg.e, exp_e, "wrong parsed E value");
        assert_eq!(cfg.b, exp_b, "wrong parsed b value");
        assert_eq!(cfg.trace_file, exp_trace, "wrong parsed trace path");
        assert_eq!(cfg.l2_s, None, "single-level mode should not set l2_s");
        assert_eq!(cfg.l2_e, None, "single-level mode should not set l2_e");
        assert_eq!(cfg.l2_b, None, "single-level mode should not set l2_b");
    }
}

#[test]
/// Unit test: verifies that valid multi-level command-line arguments
/// are parsed correctly into a Config structure.
fn parse_args_from_accepts_valid_multi_level_arguments() {
    let args: Vec<String> = vec![
        "sim",
        "--multi",
        "-s", "2",
        "-E", "1",
        "-b", "4",
        "--l2-s", "3",
        "--l2-E", "2",
        "--l2-b", "4",
        "-t", "trace.txt",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let cfg = parse_args_from(&args).expect("expected valid multi-level arguments to parse");

    assert!(cfg.multi, "multi-level mode should be enabled");
    assert_eq!(cfg.s, 2);
    assert_eq!(cfg.e, 1);
    assert_eq!(cfg.b, 4);
    assert_eq!(cfg.l2_s, Some(3));
    assert_eq!(cfg.l2_e, Some(2));
    assert_eq!(cfg.l2_b, Some(4));
    assert_eq!(cfg.trace_file, "trace.txt");
}

#[test]
/// Unit test: verifies that multi-level mode is rejected unless all
/// required L2 arguments are provided.
fn parse_args_from_rejects_multi_level_mode_without_all_l2_arguments() {
    let cases: Vec<Vec<&str>> = vec![
        vec![
            "sim", "--multi",
            "-s", "2", "-E", "1", "-b", "4",
            "--l2-s", "3",
            "-t", "trace.txt"
        ],
        vec![
            "sim", "--multi",
            "-s", "2", "-E", "1", "-b", "4",
            "--l2-E", "2",
            "-t", "trace.txt"
        ],
        vec![
            "sim", "--multi",
            "-s", "2", "-E", "1", "-b", "4",
            "--l2-b", "4",
            "-t", "trace.txt"
        ],
        vec![
            "sim", "--multi",
            "-s", "2", "-E", "1", "-b", "4",
            "--l2-s", "3", "--l2-E", "2",
            "-t", "trace.txt"
        ],
    ];

    for raw_args in cases {
        let args: Vec<String> = raw_args.into_iter().map(String::from).collect();

        assert!(
            parse_args_from(&args).is_err(),
            "expected parse failure for args: {:?}",
            args
        );
    }
}

#[test]
/// Unit test: verifies that missing required command-line arguments are rejected.
///
/// This confirms that the parser correctly fails when one or more of the
/// required flags are omitted.
fn parse_args_from_rejects_missing_required_argument_cases() {
    let cases: Vec<Vec<&str>> = vec![
        vec!["sim", "-E", "2", "-b", "4", "-t", "trace.txt"], // missing -s
        vec!["sim", "-s", "4", "-b", "4", "-t", "trace.txt"], // missing -E
        vec!["sim", "-s", "4", "-E", "4", "-t", "trace.txt"], // missing -b
        vec!["sim", "-s", "2", "-E", "4", "-b", "4"],         // missing -t
        vec!["sim"],                                           // all missing
    ];

    for raw_args in cases {
        let args: Vec<String> = raw_args.into_iter().map(String::from).collect();

        assert!(
            parse_args_from(&args).is_err(),
            "expected parse failure for args: {:?}",
            args
        );
    }
}