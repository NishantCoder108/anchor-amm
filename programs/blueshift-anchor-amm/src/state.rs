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

