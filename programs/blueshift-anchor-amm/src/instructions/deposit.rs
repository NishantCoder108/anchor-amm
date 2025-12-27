use anchor_lang::prelude::*;
use anchor_spl::token_2022::{Token2022, spl_token_2022};
use anchor_spl::token_interface::{Mint as InterfaceMint, TokenAccount as InterfaceTokenAccount, transfer, Transfer};
use anchor_spl::token::{Mint, TokenAccount, mint_to, MintTo, Token};
use crate::state::Config;
use crate::errors::AmmError;
use crate::constants::MINIMUM_LIQUIDITY;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Box<InterfaceAccount<'info, InterfaceMint>>,
    pub mint_y: Box<InterfaceAccount<'info, InterfaceMint>>,
    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump
    )]
    pub mint_lp: Box<Account<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
    )]
    pub user_lp: Box<Account<'info, TokenAccount>>,
    #[account(
        seeds = [b"config", config.seed.to_le_bytes().as_ref(), config.mint_x.as_ref(), config.mint_y.as_ref()], 
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Option<Program<'info, Token2022>>
}

impl<'info> Deposit<'info> {
    pub fn checks(
        &self,
        amount: u64,
        max_x: u64,
        max_y: u64,
        expiration: i64
    ) -> Result<()> {
        // Check the AMM is not locked
        require_eq!(self.config.locked, false, AmmError::PoolLocked);

        // Check the offer hasn't expired
        require_gt!(expiration, Clock::get()?.unix_timestamp, AmmError::OfferExpired);

        // Check all amounts have a valid number
        require_gt!(amount, 0, AmmError::InvalidAmount);
        require_gt!(max_x, 0, AmmError::InvalidAmount);
        require_gt!(max_y, 0, AmmError::InvalidAmount);

        // Pool must be initialized (first deposit happens in initialize)
        require_gt!(self.mint_lp.supply, 0, AmmError::PoolNotInitialized);

        Ok(())
    }

    pub fn calculate_amounts(
        &self,
        amount: u64,
    ) -> Result<(u64, u64)> {
        // MINIMUM_LIQUIDITY acts as virtual dead shares that were never minted.
        // This prevents inflation attacks by diluting all LP shares.
        // adjusted_supply = actual_minted + virtual_locked
        let adjusted_supply = (self.mint_lp.supply as u128)
            .checked_add(MINIMUM_LIQUIDITY as u128)
            .ok_or(AmmError::Overflow)?;

        // Calculate required deposits proportional to current pool state
        // x_required = vault_x * lp_amount / adjusted_supply
        // y_required = vault_y * lp_amount / adjusted_supply
        let x = (self.vault_x.amount as u128)
            .checked_mul(amount as u128)
            .ok_or(AmmError::Overflow)?
            .checked_div(adjusted_supply)
            .ok_or(AmmError::Overflow)?
            .checked_add(1) // Round up to prevent rounding exploits
            .ok_or(AmmError::Overflow)? as u64;

        let y = (self.vault_y.amount as u128)
            .checked_mul(amount as u128)
            .ok_or(AmmError::Overflow)?
            .checked_div(adjusted_supply)
            .ok_or(AmmError::Overflow)?
            .checked_add(1) // Round up to prevent rounding exploits
            .ok_or(AmmError::Overflow)? as u64;

        Ok((x, y))
    }

    pub fn deposit_tokens(
        &self,
        is_x: bool,
        amount: u64,
    ) -> Result<()> {
        let (mint, from, to) = match is_x {
            true => (
                self.mint_x.to_account_info(),
                self.user_x.to_account_info(),
                self.vault_x.to_account_info(),
            ),
            false => (
                self.mint_y.to_account_info(),
                self.user_y.to_account_info(),
                self.vault_y.to_account_info(),
            )
        };

        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.user.to_account_info(),
        };

        let program = if *mint.owner == spl_token_2022::ID {
            self.token_2022_program.as_ref().ok_or(AmmError::InvalidToken)?.to_account_info()
        } else {
            self.token_program.to_account_info()
        };

        transfer(CpiContext::new(program, cpi_accounts), amount)
    }

    pub fn mint_lp_tokens(
        &self,
        amount:u64
    ) -> Result<()> {    
        // Create the mint to accounts
        let accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            to: self.user_lp.to_account_info(),
            authority: self.config.to_account_info(),
        };

        // Create the signer seeds
        let seed_binding = self.config.seed.to_le_bytes();
        let mint_x_binding = self.mint_x.key().to_bytes();
        let mint_y_binding = self.mint_y.key().to_bytes();

        let seeds: &[&[u8]] = &[
            b"config".as_ref(),
            seed_binding.as_ref(),
            mint_x_binding.as_ref(),
            mint_y_binding.as_ref(),
            &[self.config.bump],
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

pub fn deposit(
    ctx: Context<Deposit>,
    amount: u64,
    max_x: u64,
    max_y: u64,
    expiration: i64,
) -> Result<()> {
    ctx.accounts.checks(amount, max_x, max_y, expiration)?;
    let (x, y) = ctx.accounts.calculate_amounts(amount)?;

    require_gte!(max_x, x, AmmError::SlippageExceeded);
    require_gte!(max_y, y, AmmError::SlippageExceeded);

    ctx.accounts.deposit_tokens(true, x)?;
    ctx.accounts.deposit_tokens(false, y)?;
    ctx.accounts.mint_lp_tokens(amount)
}