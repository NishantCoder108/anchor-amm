use anchor_lang::prelude::*;

#[account]
pub struct Config {
    pub seed: u64,
    pub authority: Option<Pubkey>,
    pub mint_x: Pubkey,           // Token X Mint
    pub mint_y: Pubkey,           // Token Y Mint
    pub fee: u16,                 // Swap fee in basis points
    pub locked: bool,
    pub auth_bump: u8,
    pub config_bump: u8,
    pub lp_bump: u8
}

impl Config {
    pub const INIT_SPACE: usize = 8 + 8 + 1 + 32 + 32 + 32 + 2 + 1 + 1 + 1 + 1;
}