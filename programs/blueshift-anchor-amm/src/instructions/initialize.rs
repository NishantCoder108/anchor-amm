use anchor_lang::prelude::*;
use anchor_spl::token_2022::{Token2022, spl_token_2022};
use anchor_spl::token_interface::{Mint as InterfaceMint, TokenAccount as InterfaceTokenAccount, transfer, Transfer};
use anchor_spl::token::{Mint, TokenAccount, Token, mint_to, MintTo};
use anchor_spl::associated_token::AssociatedToken;
use crate::errors::AmmError;
use crate::state::Config;
use crate::constants::MINIMUM_LIQUIDITY;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub mint_x: Box<InterfaceAccount<'info, InterfaceMint>>,
    pub mint_y: Box<InterfaceAccount<'info, InterfaceMint>>,
    #[account(
        init,
        seeds = [b"lp", config.key.as_ref()],
        payer = initializer,
        bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub mint_lp: Box<Account<'info, Mint>>,
    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,
    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = initializer,
    )]
    pub initializer_x: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = initializer,
    )]
    pub initializer_y: Box<InterfaceAccount<'info, InterfaceTokenAccount>>,
    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_lp,
        associated_token::authority = initializer,
    )]
    pub initializer_lp: Box<Account<'info, TokenAccount>>,
    #[account(
        init, 
        payer = initializer, 
        seeds = [b"config", seed.to_le_bytes().as_ref(), mint_x.key().as_ref(), mint_y.key().as_ref()], 
        bump,
        space =  Config::DISCRIMINATOR.len() + Config::INIT_SPACE
    )]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Option<Program<'info, Token2022>>,
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
        self.config.set_inner(
            Config {
                seed,
                authority,
                mint_x: self.mint_x.key(),
                mint_y: self.mint_y.key(),
                fee,
                locked: false,
                lp_bump: bumps.mint_lp,
                bump: bumps.config
            }
        );

        Ok(())
    }

    pub fn deposit_tokens(
        &self,
        is_x: bool,
        amount: u64,
    ) -> Result<()> {
        let (mint, from, to) = match is_x {
            true => (
                self.mint_x.to_account_info(),
                self.initializer_x.to_account_info(),
                self.vault_x.to_account_info(),
            ),
            false => (
                self.mint_y.to_account_info(),
                self.initializer_y.to_account_info(),
                self.vault_y.to_account_info(),
            )
        };

        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.initializer.to_account_info(),
        };

        let program = if *mint.owner == spl_token_2022::ID {
            self.token_2022_program.as_ref().ok_or(AmmError::InvalidToken)?.to_account_info()
        } else {
            self.token_program.to_account_info()
        };

        transfer(CpiContext::new(program, cpi_accounts), amount)
    }

    pub fn mint_initial_lp(
        &self,
        liquidity: u64,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        let seed_binding = self.config.seed.to_le_bytes();
        let mint_x_binding = self.mint_x.key().to_bytes();
        let mint_y_binding = self.mint_y.key().to_bytes();

        let seeds: &[&[u8]] = &[
            b"config".as_ref(),
            seed_binding.as_ref(),
            mint_x_binding.as_ref(),
            mint_y_binding.as_ref(),
            &[bumps.config],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            to: self.initializer_lp.to_account_info(),
            authority: self.config.to_account_info(),
        };

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            ),
            liquidity,
        )
    }
}

/// Integer square root using Newton's method
fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

pub fn initialize(
    ctx: Context<Initialize>,
    seed: u64,
    authority: Pubkey,
    fee: u16,
    init_amount_x: u64,
    init_amount_y: u64,
) -> Result<()> {
    // Validate fee
    require!(fee <= 10000, AmmError::InvalidFee);

    // Validate initial amounts
    require!(init_amount_x > 0, AmmError::InvalidAmount);
    require!(init_amount_y > 0, AmmError::InvalidAmount);

    // Initialize config first (needed for signer seeds)
    ctx.accounts.initialize_config(seed, authority, fee, &ctx.bumps)?;

    // Transfer initial liquidity to vaults
    ctx.accounts.deposit_tokens(true, init_amount_x)?;
    ctx.accounts.deposit_tokens(false, init_amount_y)?;

    // Calculate LP tokens: sqrt(x * y) - MINIMUM_LIQUIDITY
    let product = (init_amount_x as u128)
        .checked_mul(init_amount_y as u128)
        .ok_or(AmmError::Overflow)?;
    let liquidity = integer_sqrt(product) as u64;

    // Ensure enough liquidity to lock MINIMUM_LIQUIDITY
    require!(liquidity > MINIMUM_LIQUIDITY, AmmError::InsufficientInitialLiquidity);

    let lp_to_mint = liquidity
        .checked_sub(MINIMUM_LIQUIDITY)
        .ok_or(AmmError::Overflow)?;

    // Mint LP tokens to initializer (minus MINIMUM_LIQUIDITY which stays virtual)
    ctx.accounts.mint_initial_lp(lp_to_mint, &ctx.bumps)?;

    msg!("Pool initialized: liquidity={}, minted={}, locked={}", 
         liquidity, lp_to_mint, MINIMUM_LIQUIDITY);

    Ok(())
}