use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount, Token};
use anchor_spl::associated_token::AssociatedToken;
use crate::errors::AmmError;
use crate::state::{Config, LazyConfig};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub mint_x: Box<Account<'info, Mint>>,
    pub mint_y: Box<Account<'info, Mint>>,
    #[account(
        init,
        seeds = [b"lp", config.key.as_ref()],
        payer = initializer,
        bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub mint_lp: Account<'info, Mint>,
    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Box<Account<'info, TokenAccount>>,
    #[account(
        init, 
        payer = initializer, 
        seeds = [b"config", seed.to_le_bytes().as_ref(), mint_x.key().as_ref(), mint_y.key().as_ref()], 
        bump,
        space =  Config::DISCRIMINATOR.len() + Config::INIT_SPACE
    )]
    pub config: LazyAccount<'info, Config>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Initialize<'info> {
    pub fn initialize_config(
        &mut self,
        seed: u64,
        authority: Pubkey,
        fee: u16,
        bumps: &InitializeBumps
    ) -> Result<()> {
        // Don't charge >100.00% as a fee
        require!(fee <= 10000, AmmError::InvalidFee);

        // Initialize the config using load_mut()
        let mut config = self.config.load_mut()?;
        *config = Config {
            seed,
            authority,
            mint_x: self.mint_x.key(),
            mint_y: self.mint_y.key(),
            fee,
            locked: false,
            lp_bump: bumps.mint_lp,
            bump: bumps.config
        };

        Ok(())
    }
}