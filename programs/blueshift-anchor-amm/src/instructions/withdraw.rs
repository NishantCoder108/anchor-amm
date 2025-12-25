use anchor_lang::prelude::*;
use anchor_spl::token_2022::{Token2022, spl_token_2022};
use anchor_spl::token_interface::{Mint, TokenAccount, transfer, Transfer, burn, Burn};
use anchor_spl::token::Token;
use constant_product_curve::ConstantProduct;
use crate::state::Config;
use crate::errors::AmmError;
use crate::constants::{VIRTUAL_SHARES, VIRTUAL_ASSETS};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Box<InterfaceAccount<'info, Mint>>,
    pub mint_y: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump
    )]
    pub mint_lp: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
    )]
    pub user_lp: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        seeds = [b"config", config.seed.to_le_bytes().as_ref(), config.mint_x.as_ref(), config.mint_y.as_ref()], 
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Option<Program<'info, Token2022>>
}

impl<'info> Withdraw<'info> {
    pub fn checks(
        &self,
        amount: u64,
        min_x: u64,
        min_y: u64,
        expiration: i64
    ) -> Result<()> {
        // Check the AMM is not locked
        require_eq!(self.config.locked, false, AmmError::PoolLocked);

        // Check the offer hasn't expired
        require_gt!(expiration, Clock::get()?.unix_timestamp, AmmError::OfferExpired);

        // Check all amounts have a valid number
        require_gt!(amount, 0, AmmError::InvalidAmount);
        require_gt!(min_x, 0, AmmError::InvalidAmount);
        require_gt!(min_y, 0, AmmError::InvalidAmount);

        Ok(())
    }

    pub fn calculate_amounts(&self, amount: u64) -> Result<(u64, u64)> {
        let adjusted_supply = (self.mint_lp.supply as u128)
            .checked_add(VIRTUAL_SHARES as u128)
            .ok_or(AmmError::Overflow)?;

        let adjusted_vault_x = (self.vault_x.amount as u128)
            .checked_add(VIRTUAL_ASSETS as u128)
            .ok_or(AmmError::Overflow)?;

        let adjusted_vault_y = (self.vault_y.amount as u128)
            .checked_add(VIRTUAL_ASSETS as u128)
            .ok_or(AmmError::Overflow)?;

        let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
            adjusted_vault_x as u64,
            adjusted_vault_y as u64,
            adjusted_supply as u64,
            amount,
            6
        ).map_err(AmmError::from)?;

        Ok((amounts.x, amounts.y))
    }

    pub fn withdraw_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
        let (mint, from, to) = match is_x {
            true => (
                self.mint_x.to_account_info(),
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            ),
            false => (
                self.mint_y.to_account_info(),
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            )
        };

        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.config.to_account_info(),
        };

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

        let program = if *mint.owner == spl_token_2022::ID {
            self.token_2022_program.as_ref().ok_or(AmmError::InvalidToken)?.to_account_info()
        } else {
            self.token_program.to_account_info()
        };

        transfer(CpiContext::new_with_signer(program, cpi_accounts, signer_seeds), amount)
    }

    pub fn burn_lp_tokens(&self, amount: u64) -> Result<()> {
        let cpi_accounts = Burn {
            mint: self.mint_lp.to_account_info(),
            from: self.user_lp.to_account_info(),
            authority: self.user.to_account_info(),
        };

        burn(CpiContext::new(self.token_program.to_account_info(), cpi_accounts), amount)
    }
}

pub fn withdraw(
    ctx: Context<Withdraw>,
    amount: u64,
    min_x: u64,
    min_y: u64,
    expiration: i64,
) -> Result<()> {
    ctx.accounts.checks(amount, min_x, min_y, expiration)?;
    let (x, y) = ctx.accounts.calculate_amounts(amount)?;

    require_gte!(x, min_x, AmmError::SlippageExceeded);
    require_gte!(y, min_y, AmmError::SlippageExceeded);

    ctx.accounts.withdraw_tokens(true, x)?;
    ctx.accounts.withdraw_tokens(false, y)?;
    ctx.accounts.burn_lp_tokens(amount)
}