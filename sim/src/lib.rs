// sim/src/lib.rs
//! Core cache simulator implementation.
//!
//! Provides data structures and logic for a single-level cache
//! using an LRU replacement policy, along with trace processing.
//!
//! Also includes an optional two-level exclusive cache hierarchy
//! built from two single-level caches.

use getopts::Options;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Returns a mask with the lowest `bits` bits set to 1.
///
/// This is used to isolate the block offset and set index
/// fields from a memory address.
pub fn low_mask(bits: usize) -> u64 {
    if bits == 0 {
        0
    } else {
        (1u64 << bits) - 1
    }
}

/// Splits a memory address into `(tag, set_index, block_offset)`.
///
/// The address is interpreted as:
/// `[ tag | s set index bits | b block offset bits ]`
pub fn parse_address(address: u64, s: usize, b: usize) -> (u64, usize, u64) {
    let block_offset = address & low_mask(b);
    let set_index = ((address >> b) & low_mask(s)) as usize;
    let tag = address >> (s + b);

    (tag, set_index, block_offset)
}

#[derive(Clone, Copy)]
pub struct CacheLine {
    pub valid: bool,
    pub tag: u64,
    pub last_used: u64, // Timestamp for LRU replacement
}

impl CacheLine {
    pub fn empty() -> Self {
        Self {
            valid: false,
            tag: 0,
            last_used: 0,
        }
    }
}

pub struct CacheSet {
    pub lines: Vec<CacheLine>,
}

impl CacheSet {
    pub fn new(e: usize) -> Self {
        Self {
            lines: vec![CacheLine::empty(); e],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvictedBlock {
    pub address: u64,
}

pub struct Cache {
    pub s: usize,
    pub b: usize,
    pub sets: Vec<CacheSet>,
    pub time: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl Cache {
    pub fn new(s: usize, b: usize, e: usize) -> Self {
        let num_sets: usize = 1usize << s;
        let sets: Vec<CacheSet> = (0..num_sets).map(|_| CacheSet::new(e)).collect();

        Self {
            s,
            b,
            sets,
            time: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Probes the cache for an address.
    ///
    /// Returns `true` if the block is found, updating the line timestamp
    /// for LRU ordering. Returns `false` otherwise.
    pub fn probe(&mut self, address: u64) -> bool {
        let (tag, set_index, _block_offset) = parse_address(address, self.s, self.b);
        let set: &mut CacheSet = &mut self.sets[set_index];

        for line in &mut set.lines {
            if line.valid && line.tag == tag {
                self.time += 1;
                line.last_used = self.time;
                return true;
            }
        }

        false
    }

    /// Inserts a block into the cache.
    ///
    /// If there is an empty line, the block is inserted and `None` is returned.
    /// If the set is full, the least recently used block is evicted and returned.
    pub fn insert_block(&mut self, address: u64) -> Option<EvictedBlock> {
        let s_bits = self.s;
        let b_bits = self.b;
        let (tag, set_index, _block_offset) = parse_address(address, s_bits, b_bits);
        let set: &mut CacheSet = &mut self.sets[set_index];

        for line in &mut set.lines {
            if !line.valid {
                line.valid = true;
                line.tag = tag;
                self.time += 1;
                line.last_used = self.time;
                return None;
            }
        }

        let mut lru_idx: usize = 0;
        let mut lru_time: u64 = set.lines[0].last_used;

        for (i, line) in set.lines.iter().enumerate().skip(1) {
            if line.last_used < lru_time {
                lru_time = line.last_used;
                lru_idx = i;
            }
        }

        let evicted_tag: u64 = set.lines[lru_idx].tag;
        let evicted_address: u64 =
            (evicted_tag << (s_bits + b_bits)) | ((set_index as u64) << b_bits);

        let victim = &mut set.lines[lru_idx];
        victim.valid = true;
        victim.tag = tag;
        self.time += 1;
        victim.last_used = self.time;

        Some(EvictedBlock {
            address: evicted_address,
        })
    }

    /// Removes a block from the cache if it is present.
    ///
    /// Returns `true` if the block was found and removed, and `false` otherwise.
    pub fn remove_block(&mut self, address: u64) -> bool {
        let (tag, set_index, _block_offset) = parse_address(address, self.s, self.b);
        let set: &mut CacheSet = &mut self.sets[set_index];

        for line in &mut set.lines {
            if line.valid && line.tag == tag {
                line.valid = false;
                return true;
            }
        }

        false
    }

    /// Simulates a single cache access using a Least Recently Used (LRU) replacement policy.
    pub fn access(&mut self, address: u64) {
        if self.probe(address) {
            self.hits += 1;
            return;
        }

        self.misses += 1;

        if self.insert_block(address).is_some() {
            self.evictions += 1;
        }
    }
}

/// Two-level exclusive cache hierarchy.
///
/// A hit is counted if the block is found in either L1 or L2.
/// A miss is counted only if the block is found in neither.
/// An eviction is counted only when a block leaves L2
/// and is removed from the hierarchy entirely.
pub struct CacheHierarchy {
    pub l1: Cache,
    pub l2: Cache,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl CacheHierarchy {
    pub fn new(
        l1_s: usize,
        l1_b: usize,
        l1_e: usize,
        l2_s: usize,
        l2_b: usize,
        l2_e: usize,
    ) -> Self {
        Self {
            l1: Cache::new(l1_s, l1_b, l1_e),
            l2: Cache::new(l2_s, l2_b, l2_e),
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Simulates a single access to an exclusive L1/L2 hierarchy.
    ///
    /// Policy:
    /// - Check L1 first.
    /// - On L1 miss, check L2.
    /// - On L2 hit, promote block to L1 and remove it from L2.
    /// - If promotion evicts a block from L1, demote that block to L2.
    /// - On miss in both levels, fetch into L1.
    /// - If L1 insertion evicts a block, demote it to L2.
    /// - If L2 insertion evicts a block, count one hierarchy eviction.
    pub fn access(&mut self, address: u64) {
        if self.l1.probe(address) {
            self.hits += 1;
            return;
        }

        if self.l2.probe(address) {
            self.hits += 1;

            let removed = self.l2.remove_block(address);
            debug_assert!(removed, "L2 hit block should be removable from L2");

            if let Some(evicted_from_l1) = self.l1.insert_block(address) {
                if self.l2.insert_block(evicted_from_l1.address).is_some() {
                    self.evictions += 1;
                }
            }

            return;
        }

        self.misses += 1;

        if let Some(evicted_from_l1) = self.l1.insert_block(address) {
            if self.l2.insert_block(evicted_from_l1.address).is_some() {
                self.evictions += 1;
            }
        }
    }
}

pub struct Config {
    pub multi: bool,
    pub s: usize,
    pub e: usize,
    pub b: usize,
    pub l2_s: Option<usize>,
    pub l2_e: Option<usize>,
    pub l2_b: Option<usize>,
    pub trace_file: String,
}

/// Parses the required command-line arguments and returns the cache
/// configuration together with the trace file path.
pub fn parse_args() -> Result<Config, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    parse_args_from(&args)
}

/// Helper that accepts an explicit argument list from CLI.
pub fn parse_args_from(args: &[String]) -> Result<Config, Box<dyn Error>> {
    let mut opts: Options = Options::new();
    opts.optflag("", "multi", "Run a two-level cache hierarchy");
    opts.optopt("s", "", "Number of L1 set index bits", "NUM");
    opts.optopt("E", "", "L1 associativity (lines per set)", "NUM");
    opts.optopt("b", "", "Number of L1 block bits", "NUM");
    opts.optopt("", "l2-s", "Number of L2 set index bits", "NUM");
    opts.optopt("", "l2-E", "L2 associativity (lines per set)", "NUM");
    opts.optopt("", "l2-b", "Number of L2 block bits", "NUM");
    opts.optopt("t", "", "Trace file", "FILE");

    let matches = opts.parse(&args[1..])?;

    let multi: bool = matches.opt_present("multi");

    let s: usize = matches.opt_str("s").ok_or("Missing -s")?.parse()?;
    let e: usize = matches.opt_str("E").ok_or("Missing -E")?.parse()?;
    let b: usize = matches.opt_str("b").ok_or("Missing -b")?.parse()?;
    let trace_file: String = matches.opt_str("t").ok_or("Missing -t")?;

    let l2_s: Option<usize> = matches.opt_str("l2-s").map(|v| v.parse()).transpose()?;
    let l2_e: Option<usize> = matches.opt_str("l2-E").map(|v| v.parse()).transpose()?;
    let l2_b: Option<usize> = matches.opt_str("l2-b").map(|v| v.parse()).transpose()?;

    if multi && (l2_s.is_none() || l2_e.is_none() || l2_b.is_none()) {
        return Err("Multi-level mode requires --l2-s, --l2-E, and --l2-b".into());
    }

    Ok(Config {
        multi,
        s,
        e,
        b,
        l2_s,
        l2_e,
        l2_b,
        trace_file,
    })
}

/// Reads the trace file and applies each memory operation to a single-level cache.
///
/// Operation handling:
/// - `I` is ignored,
/// - `L` and `S` each perform one cache access,
/// - `M` performs two accesses because it represents a load followed by a store.
pub fn run_trace(cache: &mut Cache, filename: &str) -> Result<(), Box<dyn Error>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line: String = line?;
        let trimmed: &str = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }

        let op: char = trimmed.chars().next().unwrap();

        if op == 'I' {
            continue;
        }

        let rest: &str = trimmed[1..].trim();
        let mut parts = rest.split(',');
        let add_str: &str = parts
            .next()
            .ok_or("Bad trace line: missing address")?
            .trim()
            .trim_start_matches("0x");

        let address: u64 =
            u64::from_str_radix(add_str, 16).map_err(|_| "Bad trace line: invalid hex address")?;

        match op {
            'L' | 'S' => cache.access(address),
            'M' => {
                cache.access(address);
                cache.access(address);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Reads the trace file and applies each memory operation to a two-level hierarchy.
///
/// Operation handling:
/// - `I` is ignored,
/// - `L` and `S` each perform one hierarchy access,
/// - `M` performs two accesses because it represents a load followed by a store.
pub fn run_trace_hierarchy(
    hierarchy: &mut CacheHierarchy,
    filename: &str,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line: String = line?;
        let trimmed: &str = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }

        let op: char = trimmed.chars().next().unwrap();

        if op == 'I' {
            continue;
        }

        let rest: &str = trimmed[1..].trim();
        let mut parts = rest.split(',');
        let add_str: &str = parts
            .next()
            .ok_or("Bad trace line: missing address")?
            .trim()
            .trim_start_matches("0x");

        let address: u64 =
            u64::from_str_radix(add_str, 16).map_err(|_| "Bad trace line: invalid hex address")?;

        match op {
            'L' | 'S' => hierarchy.access(address),
            'M' => {
                hierarchy.access(address);
                hierarchy.access(address);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Parses arguments, builds either a single-level cache or a two-level hierarchy,
/// executes the trace, and prints the final simulation statistics.
pub fn run() -> Result<(), Box<dyn Error>> {
    let cfg = parse_args()?;

    if cfg.multi {
        let l2_s = cfg.l2_s.ok_or("Missing --l2-s in multi-level mode")?;
        let l2_e = cfg.l2_e.ok_or("Missing --l2-E in multi-level mode")?;
        let l2_b = cfg.l2_b.ok_or("Missing --l2-b in multi-level mode")?;

        let mut hierarchy = CacheHierarchy::new(cfg.s, cfg.b, cfg.e, l2_s, l2_b, l2_e);
        run_trace_hierarchy(&mut hierarchy, &cfg.trace_file)?;

        println!(
            "hits:{} misses:{} evictions:{}",
            hierarchy.hits, hierarchy.misses, hierarchy.evictions
        );
    } else {
        let mut cache = Cache::new(cfg.s, cfg.b, cfg.e);
        run_trace(&mut cache, &cfg.trace_file)?;

        println!(
            "hits:{} misses:{} evictions:{}",
            cache.hits, cache.misses, cache.evictions
        );
    }

    Ok(())
}