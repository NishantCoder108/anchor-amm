use anchor_lang::prelude::*;

#[account(discriminator = 1)]
#[derive(InitSpace)]
pub struct Config {
    pub seed: u64,
    pub authority: Pubkey,
    pub mint_x: Pubkey,           // Token X Mint
    pub mint_y: Pubkey,           // Token Y Mint
    pub fee: u16,                 // Swap fee in basis points
    pub locked: bool,
    pub lp_bump: u8,
    pub bump: u8
}

impl Config {
    pub const INIT_SPACE: usize = 1 + 8 + 32 + 32 + 32 + 2 + 1 + 1 + 1;
}

/// Position NFT - represents LP tokens with optional vesting
#[account]
#[derive(InitSpace)]
pub struct Position {
    pub config: Pubkey,           // The Config account this position belongs to
    pub nft_mint: Pubkey,         // The NFT mint representing this position
    pub lp_amount: u64,           // Total LP tokens this position represents
    pub vesting_start: i64,       // When vesting begins (0 = no vesting)
    pub vesting_end: i64,         // When fully vested (0 = no vesting)
    pub cliff: i64,               // Nothing unlocks before this timestamp (0 = no cliff)
    pub bump: u8,
}

impl Position {
    /// Calculate how many LP tokens are currently unlocked based on vesting schedule
    pub fn unlocked_lp(&self, current_time: i64) -> u64 {
        // No vesting - everything unlocked
        if self.vesting_start == 0 && self.vesting_end == 0 {
            return self.lp_amount;
        }

        // Before cliff - nothing unlocked
        if current_time < self.cliff || current_time < self.vesting_start {
            return 0;
        }
        
        // After vesting ends - everything unlocked
        if current_time >= self.vesting_end {
            return self.lp_amount;
        }

        // Linear vesting: calculate proportion unlocked
        let total_vesting_time = self.vesting_end - self.vesting_start;
        let time_elapsed = current_time - self.vesting_start;
        
        // unlocked = lp_amount * time_elapsed / total_vesting_time
        ((self.lp_amount as u128) * (time_elapsed as u128) / (total_vesting_time as u128)) as u64
    }
}

