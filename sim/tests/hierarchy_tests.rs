use sim::CacheHierarchy;

fn addr(x: u64) -> u64 {
    x
}

#[test]
fn test_hierarchy_first_access_is_miss() {
    let mut h = CacheHierarchy::new(0, 0, 1, 0, 0, 1);

    h.access(addr(1));

    assert_eq!(h.hits, 0);
    assert_eq!(h.misses, 1);
    assert_eq!(h.evictions, 0);
}

#[test]
fn test_hierarchy_l1_hit_on_second_access() {
    let mut h = CacheHierarchy::new(0, 0, 1, 0, 0, 1);

    h.access(addr(1)); // miss into L1
    h.access(addr(1)); // hit in L1

    assert_eq!(h.hits, 1);
    assert_eq!(h.misses, 1);
    assert_eq!(h.evictions, 0);
}

#[test]
fn test_hierarchy_l1_eviction_demotes_block_to_l2() {
    let mut h = CacheHierarchy::new(0, 0, 1, 0, 0, 2);

    h.access(addr(1)); // miss -> L1 has 1
    h.access(addr(2)); // miss -> 1 evicted from L1, demoted to L2

    assert!(!h.l1.probe(addr(1)));
    assert!(h.l2.probe(addr(1)));

    assert!(h.l1.probe(addr(2)));
}

#[test]
fn test_hierarchy_l2_hit_promotes_block_to_l1() {
    let mut h = CacheHierarchy::new(0, 0, 1, 0, 0, 2);

    h.access(addr(1)); // miss -> L1: 1
    h.access(addr(2)); // miss -> L1: 2, L2: 1
    h.access(addr(1)); // hit in L2 -> promote 1 to L1, demote 2 to L2

    assert_eq!(h.hits, 1);
    assert_eq!(h.misses, 2);
    assert_eq!(h.evictions, 0);

    assert!(h.l1.probe(addr(1)));
    assert!(!h.l2.probe(addr(1))); // exclusive: should no longer be in L2
    assert!(h.l2.probe(addr(2)));
}

#[test]
fn test_hierarchy_exclusive_property_no_duplicate_after_promotion() {
    let mut h = CacheHierarchy::new(0, 0, 1, 0, 0, 2);

    h.access(addr(10));
    h.access(addr(20)); // 10 demoted to L2
    h.access(addr(10)); // promote back to L1

    let in_l1 = h.l1.probe(addr(10));
    let in_l2 = h.l2.probe(addr(10));

    assert!(in_l1);
    assert!(!in_l2);
}

#[test]
fn test_hierarchy_l2_overflow_counts_eviction() {
    let mut h = CacheHierarchy::new(0, 0, 1, 0, 0, 1);

    h.access(addr(1)); // miss -> L1: 1
    h.access(addr(2)); // miss -> L1: 2, L2: 1
    h.access(addr(3)); // miss -> L1: 3, L2: 2, evict 1 from L2

    assert_eq!(h.hits, 0);
    assert_eq!(h.misses, 3);
    assert_eq!(h.evictions, 1);
}