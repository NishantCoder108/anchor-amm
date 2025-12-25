/// Virtual shares offset to prevent first depositor inflation attacks.
/// Adding this to share calculations creates an implicit exchange rate even when pool is empty.
pub const VIRTUAL_SHARES: u64 = 1_000_000_000; // 10^9 virtual shares
pub const VIRTUAL_ASSETS: u64 = 1;              // 1 virtual asset per token