use proptest::prelude::*;
use sim::parse_address;

proptest! {
    #[test]
    /// Property: Splitting and reconstructing an address preserves the original value.
    ///
    /// This verifies that the decomposition into (tag, set index, block offset)
    /// is internally consistent for all valid bit configurations.
    fn parse_address_reconstructs_original_address(
        address in 0u64..(1u64 << 20),
        s in 0usize..6,
        b in 0usize..6,
    ) {
        let (tag, set_index, block_offset) = parse_address(address, s, b);

        // Reconstruct the address from its components
        let reconstructed =
            (tag << (s + b)) |
            ((set_index as u64) << b) |
            block_offset;

        prop_assert_eq!(reconstructed, address);
    }
}

proptest! {
    #[test]
    /// Property: The computed set index is always within the valid range [0, 2^s).
    ///
    /// This ensures that masking and shifting correctly isolate the set index bits.
    fn set_index_is_within_range(
        address in 0u64..1_000_000,
        s in 0usize..6,
        b in 0usize..6
    ) {
        let (_tag, set_index, _offset) = parse_address(address, s, b);

        // Upper bound for set index (number of sets = 2^s)
        let upper = if s == 0 { 1 } else { 1usize << s };

        prop_assert!(set_index < upper);
    }
}