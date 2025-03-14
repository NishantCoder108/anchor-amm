use anchor_lang::prelude::*;


mod state;
mod errors;
pub mod instructions;
pub use instructions::*;

declare_id!("22222222222222222222222222222222222222222222");

#[program]
pub mod blueshift_anchor_amm {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn initialize(ctx: Context<Initialize>, seed: u64, authority: Pubkey, fee: u16) -> Result<()> {
        ctx.accounts.initialize_config(seed, authority, fee, &ctx.bumps)?;

        Ok(())
    }

    #[instruction(discriminator = 1)]
    pub fn deposit(ctx: Context<LiquidityAction>, amount: u64, max_x: u64, max_y: u64, expiration: i64) -> Result<()> {
        ctx.accounts.checks(amount, max_x, max_y, expiration)?;
        ctx.accounts.deposit(amount, max_x, max_y)?;

        Ok(())
    }

    #[instruction(discriminator = 2)]
    pub fn withdraw(ctx: Context<LiquidityAction>, amount: u64, min_x: u64, min_y: u64, expiration: i64) -> Result<()> {
        ctx.accounts.checks(amount, min_x, min_y, expiration)?;
        ctx.accounts.withdraw(amount, min_x, min_y)?;

        Ok(())
    }

    #[instruction(discriminator = 3)]
    pub fn swap(ctx: Context<Swap>, amount: u64, min: u64) -> Result<()> {
        ctx.accounts.swap(amount, min)?;

        Ok(())
    }

    #[instruction(discriminator = 4)]
    pub fn update_authority(ctx: Context<ConfigAction>, authority: Pubkey) -> Result<()> {
        ctx.accounts.check_authority()?;
        ctx.accounts.update_authority(authority)?;

        Ok(())
    }

    #[instruction(discriminator = 5)]
    pub fn update_fee(ctx: Context<ConfigAction>, fee: u16) -> Result<()> {
        ctx.accounts.check_authority()?;
        ctx.accounts.update_fee(fee)?;

        Ok(())
    }

    #[instruction(discriminator = 6)]
    pub fn update_lock(ctx: Context<ConfigAction>) -> Result<()> {
        ctx.accounts.check_authority()?;
        ctx.accounts.update_lock()?;

        Ok(())
    }

    #[instruction(discriminator = 7)]
    pub fn remove_authority(ctx: Context<ConfigAction>) -> Result<()> {
        ctx.accounts.check_authority()?;
        ctx.accounts.remove_authority()?;

        Ok(())
    }
}
