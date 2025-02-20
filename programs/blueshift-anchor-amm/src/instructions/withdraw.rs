use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer, transfer, burn, Burn};
use anchor_spl::associated_token::AssociatedToken;
use constant_product_curve::{ConstantProduct, XYAmounts};
use crate::state::Config;
use crate::errors::AmmError;

#[derive(Accounts)]
pub struct Withdraw<'info> {
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
        init_if_needed,
        payer = user,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
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

impl<'info> Withdraw<'info> {
    pub fn withdraw(
        &self,
        amount: u64,        // Amount of LP token to burn
        min_x: u64,         // Min amount of X we are willing to withdraw
        min_y: u64,         // Min amount of Y we are willing to withdraw
        expiration: i64     // Expiration of the offer
    ) -> Result<()> {
        // Check if the pool is locked
        require!(self.config.locked == false, AmmError::PoolLocked);

        // Check if the offer has expired
        require!(expiration > Clock::get()?.unix_timestamp, AmmError::OfferExpired);

        // Check if the amounts are valid
        require!(amount > 0, AmmError::InvalidAmount);
        require!(min_x > 0, AmmError::InvalidAmount);
        require!(min_y > 0, AmmError::InvalidAmount);

        let amounts = if self.mint_lp.supply != amount {
            ConstantProduct::xy_withdraw_amounts_from_l(
                self.vault_x.amount,
                self.vault_y.amount,
                self.mint_lp.supply,
                amount,
                6
            ).map_err(AmmError::from)?
        } else {
            XYAmounts {
                x: self.vault_x.amount,
                y: self.vault_y.amount
            }
        };

        // Check for slippage
        require!(min_x <= amounts.x && min_y <= amounts.y, AmmError::SlippageExceeded);
        
        // Withdraw the tokens
        self.withdraw_tokens(true, amounts.x)?;
        self.withdraw_tokens(false, amounts.y)?;

        // Burn the LP tokens
        self.burn_lp_tokens(amount)
    }

    pub fn withdraw_tokens(
        &self,
        is_x: bool,
        amount:u64
    ) -> Result<()> {  
        // Create the transfer accounts
        let (from, to) = match is_x {
            true => (self.vault_x.to_account_info(), self.user_x.to_account_info()),
            false => (self.vault_y.to_account_info(), self.user_y.to_account_info())
        };

        // Create the transfer accounts
        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.auth.to_account_info(),
        };
        
        // Create the signer seeds
        let seeds = &[
            &b"auth"[..],
            &[self.config.auth_bump],
        ];

        let signer_seeds = &[&seeds[..]];

        // Create the transfer context
        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            cpi_accounts,
            signer_seeds
        );

        // Execute the transfer
        transfer(ctx, amount)
    }

    pub fn burn_lp_tokens(
        &self,
        amount:u64
    ) -> Result<()> {        
        // Create the burn accounts
        let cpi_accounts = Burn {
            mint: self.mint_lp.to_account_info(),
            from: self.user_lp.to_account_info(),
            authority: self.user.to_account_info(),
        };

        // Create the burn context
        let ctx = CpiContext::new(
            self.token_program.to_account_info(), 
            cpi_accounts,
        );

        // Burn the LP tokens
        burn(ctx, amount)
    }
}