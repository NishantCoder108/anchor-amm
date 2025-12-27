/// Minimum liquidity minted to dead address on pool initialization.
/// Prevents first depositor inflation attacks (Uniswap V2 style).
pub const MINIMUM_LIQUIDITY: u64 = 1_000;