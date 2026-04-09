use getopts::Options;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Returns a mask with the lowest `bits` bits set to 1.
///
/// This is used to isolate the block offset and set index
/// fields from a memory address.
fn low_mask(bits: usize) -> u64 {
    (1u64 << bits) - 1
}

/// Splits a memory address into `(tag, set_index, block_offset)`.
///
/// The address is interpreted as:
/// `[ tag | s set index bits | b block offset bits ]`
fn parse_address(address: u64, s: usize, b: usize) -> (u64, usize, u64) {
    let block_offset = address & low_mask(b);
    let set_index = ((address >> b) & low_mask(s)) as usize;
    let tag = address >> (s + b);

    (tag, set_index, block_offset)
}

#[derive(Clone, Copy)]
struct CacheLine {
    valid: bool,
    tag: u64,
    last_used: u64, // Timestamp for LRU replacement
}

impl CacheLine {
    fn empty() -> Self {
        Self {
            valid: false,
            tag: 0,
            last_used: 0,
        }
    }
}

struct CacheSet {
    lines: Vec<CacheLine>,
}

impl CacheSet {
    fn new(e: usize) -> Self {
        Self {
            lines: vec![CacheLine::empty(); e],
        }
    }
}

struct Cache {
    s: usize,
    b: usize,
    sets: Vec<CacheSet>,
    time: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl Cache {
    fn new(s: usize, b: usize, e: usize) -> Self {
        let num_sets: usize = 1usize << s; // S = 2^s
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

    /// Simulates a single cache access using a Least Recently Used (LRU) replacement policy.
    ///
    /// The selected set is searched for:
    /// 1. a matching valid line (hit),
    /// 2. an empty line (miss without eviction),
    /// 3. otherwise the least recently used line is replaced.
    fn access(&mut self, address: u64) {
        let (tag, set_index, _block_offset) = parse_address(address, self.s, self.b);
        let set: &mut CacheSet = &mut self.sets[set_index];

        for line in &mut set.lines {
            if line.valid && line.tag == tag {
                self.hits += 1;
                self.time += 1;
                line.last_used = self.time;
                return;
            }
        }

        self.misses += 1;

        // Use an empty line if the set is not yet full.
        for line in &mut set.lines {
            if !line.valid {
                line.valid = true;
                line.tag = tag;
                self.time += 1;
                line.last_used = self.time;
                return;
            }
        }

        // If the set is full, replace the LRU line.
        self.evictions += 1;

        let mut lru_idx: usize = 0usize;
        let mut lru_time: u64 = set.lines[0].last_used;
        for (i, line) in set.lines.iter().enumerate().skip(1) {
            if line.last_used < lru_time {
                lru_time = line.last_used;
                lru_idx = i;
            }
        }

        let victim = &mut set.lines[lru_idx];
        victim.tag = tag;
        self.time += 1;
        victim.last_used = self.time;
    }
}

struct Config {
    s: usize,
    e: usize,
    b: usize,
    trace_file: String,
}

/// Parses the required command-line arguments and returns the cache
/// configuration together with the trace file path.
fn parse_args() -> Result<Config, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let mut opts: Options = Options::new();
    opts.optopt("s", "", "Number of set index bits", "NUM");
    opts.optopt("E", "", "Associativity (lines per set)", "NUM");
    opts.optopt("b", "", "Number of block bits", "NUM");
    opts.optopt("t", "", "trace file", "FILE");

    let matches = opts.parse(&args[1..])?;

    let s: usize = matches.opt_str("s").ok_or("Missing -s")?.parse()?;
    let e: usize = matches.opt_str("E").ok_or("Missing -E")?.parse()?;
    let b: usize = matches.opt_str("b").ok_or("Missing -b")?.parse()?;
    let trace_file: String = matches.opt_str("t").ok_or("Missing -t")?;

    Ok(Config {
        s,
        e,
        b,
        trace_file,
    })
}

/// Reads the trace file and applies each memory operation to the cache.
///
/// Operation handling:
/// - `I` is ignored,
/// - `L` and `S` each perform one cache access,
/// - `M` performs two accesses because it represents a load followed by a store.
fn run_trace(cache: &mut Cache, filename: &str) -> Result<(), Box<dyn Error>> {
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
            continue; // Ignore instruction loads.
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
            _ => {} // Ignore unknown operations.
        }
    }
    Ok(())
}

/// Parses arguments, builds the cache, executes the trace,
/// and prints the final simulation statistics.
pub fn main() -> Result<(), Box<dyn Error>> {
    let cfg: Config = parse_args()?;
    let mut cache: Cache = Cache::new(cfg.s, cfg.b, cfg.e);

    run_trace(&mut cache, &cfg.trace_file)?;

    println!(
        "hits:{} misses:{} evictions:{}",
        cache.hits, cache.misses, cache.evictions
    );
    Ok(())
}
