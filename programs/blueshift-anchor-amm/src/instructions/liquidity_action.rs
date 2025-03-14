use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer, MintTo, Burn, transfer, mint_to, burn};
use constant_product_curve::ConstantProduct;
use crate::state::Config;
use crate::errors::AmmError;

#[derive(Accounts)]
pub struct LiquidityAction<'info> {
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
        associated_token::authority = config,
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
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
}

impl<'info> LiquidityAction<'info> {
    pub fn checks(
        &self,
        amount: u64,
        max_x: u64,
        max_y: u64,
        expiration: i64
    ) -> Result<()> {
        // Check if the pool is locked
        require_eq!(self.config.locked, false, AmmError::PoolLocked);
        
        // Check if the offer has expired
        require_gt!(expiration, Clock::get()?.unix_timestamp, AmmError::OfferExpired);

        // Check if the amounts are valid
        require_gt!(amount, 0, AmmError::InvalidAmount);
        require_gt!(max_x, 0, AmmError::InvalidAmount);
        require_gt!(max_y, 0, AmmError::InvalidAmount);

        Ok(())
    }

    pub fn deposit(
        &self,
        amount: u64,        // Amount of LP token to claim
        max_x: u64,         // Max amount of X we are willing to deposit
        max_y: u64,         // Max amount of Y we are willing to deposit
    ) -> Result<()> {
        // Calculate the amount of tokens to deposit
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
        require_gte!(max_x, x, AmmError::SlippageExceeded);
        require_gte!(max_y, y, AmmError::SlippageExceeded);

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

    pub fn withdraw(
        &self,
        amount: u64,
        min_x: u64,
        min_y: u64,
    ) -> Result<()> {
        // Calculate the amount of tokens to deposit
        let (x,y) = match self.mint_lp.supply == amount {
            true => (self.vault_x.amount, self.vault_y.amount),
            false => {
                let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
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
        require_gte!(x, min_x, AmmError::SlippageExceeded);
        require_gte!(y, min_y, AmmError::SlippageExceeded);

        // Withdraw the tokens
        self.withdraw_tokens(true, x)?;
        self.withdraw_tokens(false, y)?;

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