use anchor_lang::prelude::*;
use anchor_spl::token_2022::{Token2022, spl_token_2022};
use anchor_spl::token_interface::{Mint, TokenAccount, transfer, Transfer};
use anchor_spl::token::Token;
use constant_product_curve::{ConstantProduct, LiquidityPair};
use crate::state::Config;
use crate::errors::AmmError;

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_from: Box<InterfaceAccount<'info, Mint>>,
    pub mint_to: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_from,
        associated_token::authority = user
    )]
    pub user_from: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_to,
        associated_token::authority = user
    )]
    pub user_to: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_from,
        associated_token::authority = config
    )]
    pub vault_from: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_to,
        associated_token::authority = config
    )]
    pub vault_to: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        seeds = [b"config", config.seed.to_le_bytes().as_ref(), config.mint_x.as_ref(), config.mint_y.as_ref()], 
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Option<Program<'info, Token2022>>,
}

impl<'info> Swap<'info> {
    pub fn checks(
        &self,
        amount: u64,
        min: u64,
        expiration: i64
    ) -> Result<()> {
        // Check the AMM is not locked
        require_eq!(self.config.locked, false, AmmError::PoolLocked);

        // Check the offer hasn't expired
        require_gt!(expiration, Clock::get()?.unix_timestamp, AmmError::OfferExpired);

        // Check all amounts have a valid number
        require_gt!(amount, 0, AmmError::InvalidAmount);
        require_gt!(min, 0, AmmError::InvalidAmount);

        Ok(())
    }

    pub fn calculate_swap_amounts(
        &self,
        amount: u64,
        min: u64,
    ) -> Result<(u64, u64, u64)> {
        let (x_amount, y_amount, pair) = if self.mint_from.key() == self.config.mint_x.key() {
            require_eq!(self.mint_to.key(), self.config.mint_y.key(), AmmError::InvalidMint);
            (self.vault_from.amount, self.vault_to.amount, LiquidityPair::X)
        } else if self.mint_from.key() == self.config.mint_y.key() {
            require_eq!(self.mint_to.key(), self.config.mint_x.key(), AmmError::InvalidMint);
            (self.vault_to.amount, self.vault_from.amount, LiquidityPair::Y)
        } else {
            return Err(AmmError::InvalidMint.into());
        };

        let mut curve = ConstantProduct::init(
            x_amount,
            y_amount,
            x_amount,
            self.config.fee,
            None
        ).map_err(AmmError::from)?;

        let amounts = curve.swap(pair, amount, min).map_err(AmmError::from)?;

        require_gt!(amounts.deposit, 0, AmmError::InvalidAmount);
        require_gt!(amounts.withdraw, 0, AmmError::InvalidAmount);

        Ok((amounts.deposit, amounts.withdraw, amounts.fee))
    }

    pub fn deposit_token(
        &self,
        amount: u64
    ) -> Result<()> {
        let cpi_accounts = Transfer {
            from: self.user_from.to_account_info(),
            to: self.vault_from.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let program = if *self.mint_from.to_account_info().owner == spl_token_2022::ID {
            self.token_2022_program.as_ref().ok_or(AmmError::InvalidToken)?.to_account_info()
        } else {
            self.token_program.to_account_info()
        };

        transfer(CpiContext::new(program, cpi_accounts), amount)
    }

    pub fn withdraw_token(
        &self,
        amount: u64
    ) -> Result<()> {
        let cpi_accounts = Transfer {
            from: self.vault_to.to_account_info(),
            to: self.user_to.to_account_info(),
            authority: self.config.to_account_info(),
        };

        let seed_binding = self.config.seed.to_le_bytes();
        let mint_x_binding = self.config.mint_x.key().to_bytes();
        let mint_y_binding = self.config.mint_y.key().to_bytes();

        let seeds: &[&[u8]] = &[
            b"config".as_ref(),
            seed_binding.as_ref(),
            mint_x_binding.as_ref(),
            mint_y_binding.as_ref(),
            &[self.config.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let program = if *self.mint_to.to_account_info().owner == spl_token_2022::ID {
            self.token_2022_program.as_ref().ok_or(AmmError::InvalidToken)?.to_account_info()
        } else {
            self.token_program.to_account_info()
        };

        transfer(CpiContext::new_with_signer(program, cpi_accounts, signer_seeds), amount)
    }

    pub fn pay_fee(
        &self,
        amount: u64
    ) -> Result<()> {
        let cpi_accounts = Transfer {
            from: self.user_from.to_account_info(),
            to: self.vault_from.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let program = if *self.mint_from.to_account_info().owner == spl_token_2022::ID {
            self.token_2022_program.as_ref().ok_or(AmmError::InvalidToken)?.to_account_info()
        } else {
            self.token_program.to_account_info()
        };

        transfer(CpiContext::new(program, cpi_accounts), amount)
    }
}

pub fn swap(
    ctx: Context<Swap>,
    amount: u64,
    min: u64,
    expiration: i64,
) -> Result<()> {
    ctx.accounts.checks(amount, min, expiration)?;
    let (deposit_amount, withdraw_amount, fee_amount) = ctx.accounts.calculate_swap_amounts(amount, min)?;

    ctx.accounts.deposit_token(deposit_amount)?;
    ctx.accounts.withdraw_token(withdraw_amount)?;
    ctx.accounts.pay_fee(fee_amount)
}
