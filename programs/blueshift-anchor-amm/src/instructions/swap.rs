use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer, transfer};
use constant_product_curve::{ConstantProduct, LiquidityPair};
use crate::state::{Config, LazyConfig};
use crate::errors::AmmError;

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_from: Box<Account<'info, Mint>>,
    pub mint_to: Box<Account<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_from,
        associated_token::authority = user
    )]
    pub user_from: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_to,
        associated_token::authority = user
    )]
    pub user_to: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_from,
        associated_token::authority = config
    )]
    pub vault_from: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_to,
        associated_token::authority = config
    )]
    pub vault_to: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [b"config", config.load_seed()?.to_le_bytes().as_ref(), config.load_mint_x()?.key().as_ref(), config.load_mint_y()?.key().as_ref()], 
        bump = *config.load_bump()?,
    )]
    pub config: LazyAccount<'info, Config>,
    pub token_program: Program<'info, Token>,
}

impl<'info> Swap<'info> {
    pub fn checks(
        &self,
        amount: u64,
        min: u64,
        expiration: i64
    ) -> Result<()> {  
        // Check if the pool is locked
        require_eq!(*self.config.load_locked()?, false, AmmError::PoolLocked);

        // Check if the offer has expired
        require_gt!(expiration, Clock::get()?.unix_timestamp, AmmError::OfferExpired);

        // Check if the amount is valid
        require_gt!(amount, 0, AmmError::InvalidAmount);

        // Check if the min is valid
        require_gt!(min, 0, AmmError::InvalidAmount);

        Ok(())
    }

    pub fn swap(
        &mut self,
        amount: u64,
        min: u64,
    ) -> Result<()> {

       // Check if the mint are valid and decide the direction of the swap
       let (from_amount, to_amount, fee_amount) = if self.mint_from.key() == *self.config.load_mint_x()? {
            require_eq!(self.mint_to.key(), *self.config.load_mint_y()?, AmmError::InvalidMint);
            self.swap_x_to_y(amount, min)?
        } else if self.mint_from.key() == *self.config.load_mint_y()? {
            require_eq!(self.mint_to.key(), *self.config.load_mint_x()?, AmmError::InvalidMint);
            self.swap_y_to_x(amount, min)?
        } else {
            return Err(AmmError::InvalidMint.into());
        };

        // Deposit the tokens
        self.deposit_token(from_amount)?;

        // Withdraw the tokens
        self.withdraw_token(to_amount)?;

        // Pay the fee
        self.pay_fee(fee_amount)?;

        Ok(())
    }

    pub fn swap_x_to_y(
        &mut self,
        amount: u64,
        min: u64
    ) -> Result<(u64, u64, u64)> {
        // Calculate the amounts to swap
        let mut curve = ConstantProduct::init(
            self.vault_from.amount,
            self.vault_to.amount,
            self.vault_from.amount,
            *self.config.load_fee()?,
            None
        ).map_err(AmmError::from)?;

        let amounts = curve.swap(LiquidityPair::X, amount, min).map_err(AmmError::from)?;

        // Check if the amounts are valid
        require_gt!(amounts.deposit, 0, AmmError::InvalidAmount);
        require_gt!(amounts.withdraw, 0, AmmError::InvalidAmount);

        Ok((amounts.deposit, amounts.withdraw, amounts.fee))
    }

    pub fn swap_y_to_x(
        &mut self,
        amount: u64,
        min: u64
    ) -> Result<(u64, u64, u64)> {
        let mut curve = ConstantProduct::init(
            self.vault_to.amount,
            self.vault_from.amount,
            self.vault_to.amount,
            *self.config.load_fee()?,
            None
        ).map_err(AmmError::from)?;

        let amounts = curve.swap(LiquidityPair::Y, amount, min).map_err(AmmError::from)?;

        // Check if the amounts are valid
        require_gt!(amounts.deposit, 0, AmmError::InvalidAmount);
        require_gt!(amounts.withdraw, 0, AmmError::InvalidAmount);

        Ok((amounts.deposit, amounts.withdraw, amounts.fee))
    }

    pub fn deposit_token(
        &mut self,
        amount: u64
    ) -> Result<()> {
        // Create the transfer accounts
        let accounts = Transfer {
            from: self.user_from.to_account_info(),
            to: self.vault_from.to_account_info(),
            authority: self.user.to_account_info()
        };

        // Create the transfer context
        let ctx = CpiContext::new(
            self.token_program.to_account_info(),
            accounts
        );

        // Execute the transfer
        transfer(ctx, amount)
    }

    pub fn withdraw_token(
        &mut self,
        amount: u64
    ) -> Result<()> {
        // Create the transfer accounts
        let accounts = Transfer {
            from: self.vault_to.to_account_info(),
            to: self.user_to.to_account_info(),
            authority: self.config.to_account_info()
        };

        // Create the signer seeds
        let seed_binding = self.config.load_seed()?.to_le_bytes();
        let mint_x_binding = self.config.load_mint_x()?.key().to_bytes();
        let mint_y_binding = self.config.load_mint_y()?.key().to_bytes();

        let seeds = &[
            b"config".as_ref(),
            seed_binding.as_ref(),
            mint_x_binding.as_ref(),
            mint_y_binding.as_ref(),
        ];

        let signer_seeds = &[&seeds[..]];

        // Create the transfer context
        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            accounts,
            signer_seeds
        );

        // Execute the transfer
        transfer(ctx, amount)
    }

    pub fn pay_fee(
        &mut self,
        amount: u64
    ) -> Result<()> {
         // Create the transfer accounts
         let accounts = Transfer {
            from: self.user_from.to_account_info(),
            to: self.vault_from.to_account_info(),
            authority: self.user.to_account_info()
        };

        // Create the transfer context
        let ctx = CpiContext::new(
            self.token_program.to_account_info(),
            accounts
        );

        // Execute the transfer
        transfer(ctx, amount)
    }
}



