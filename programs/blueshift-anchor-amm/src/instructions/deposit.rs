use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer, MintTo, transfer, mint_to};
use anchor_spl::associated_token::AssociatedToken;
use constant_product_curve::ConstantProduct;
use crate::state::Config;
use crate::errors::AmmError;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Box<Account<'info, Mint>>,
    pub mint_y: Box<Account<'info, Mint>>,
    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump
    )]
    pub mint_lp: Box<Account<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = auth,
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = auth,
    )]
    pub vault_y: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
    )]
    pub user_lp: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: just a pda for signing
    #[account(seeds = [b"auth"], bump = config.auth_bump)]
    pub auth: UncheckedAccount<'info>,
    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref(), mint_x.key().as_ref(), mint_y.key().as_ref()], 
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn deposit(
        &self,
        amount: u64,        // Amount of LP token to claim
        max_x: u64,         // Max amount of X we are willing to deposit
        max_y: u64,         // Max amount of Y we are willing to deposit
        expiration: i64     // Expiration of the offer
    ) -> Result<()> {
        // Check if the pool is locked
        require!(self.config.locked == false, AmmError::PoolLocked);

        // Check if the offer has expired
        require!(expiration > Clock::get()?.unix_timestamp, AmmError::OfferExpired);

        // Check if the amounts are valid
        require!(amount > 0, AmmError::InvalidAmount);
        require!(max_x > 0, AmmError::InvalidAmount);
        require!(max_y > 0, AmmError::InvalidAmount);

        let (x,y) = match self.mint_lp.supply == 0 && self.vault_x.amount == 0 && self.vault_y.amount == 0 {
            true => (max_x, max_y),
            false => {
                let amounts = ConstantProduct::xy_deposit_amounts_from_l(
                    self.vault_x.amount,
                    self.vault_y.amount,
                    self.mint_lp.supply,
                    amount,
                    6
                ).map_err(AmmError::from)?;
                (amounts.x, amounts.y)
            }
        };

        // Check for slippage
        require!(x <= max_x && y <= max_y, AmmError::SlippageExceeded);

        // Deposit the tokens
        self.deposit_tokens(true, x)?;
        self.deposit_tokens(false, y)?;

        // Mint the LP tokens
        self.mint_lp_tokens(amount)
    }

    pub fn deposit_tokens(
        &self,
        is_x: bool,
        amount:u64
    ) -> Result<()> {  
        let (from, to) = match is_x {
            true => (self.user_x.to_account_info(), self.vault_x.to_account_info()),
            false => (self.user_y.to_account_info(), self.vault_y.to_account_info())
        };      

        // Create the transfer accounts
        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.user.to_account_info(),
        };

        // Create the transfer context
        let ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts);

        // Execute the transfer
        transfer(ctx, amount)
    }

    pub fn mint_lp_tokens(
        &self,
        amount:u64
    ) -> Result<()> {    
        // Create the mint to accounts
        let accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            to: self.user_lp.to_account_info(),
            authority: self.auth.to_account_info(),
        };

        // Create the signer seeds
        let seeds = &[
            &b"auth"[..],
            &[self.config.auth_bump],
        ];

        let signer_seeds = &[&seeds[..]];

        // Create the mint to context
        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            accounts,
            signer_seeds
        );

        // Mint the LP tokens
        mint_to(ctx, amount)
    }
}