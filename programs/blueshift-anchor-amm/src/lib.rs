use anchor_lang::prelude::*;


mod state;
mod errors;
pub mod instructions;
pub use instructions::*;

declare_id!("22222222222222222222222222222222222222222222");

#[program]
pub mod blueshift_anchor_amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, seed: u64, authority: Pubkey, fee: u16) -> Result<()> {
        ctx.accounts.initialize_config(seed, authority, fee, &ctx.bumps)?;

        Ok(())
    }

    pub fn update_authority(ctx: Context<UpdateConfig>, authority: Pubkey) -> Result<()> {
        ctx.accounts.update_authority(authority)?;

        Ok(())
    }

    pub fn update_fee(ctx: Context<UpdateConfig>, fee: u16) -> Result<()> {
        ctx.accounts.update_fee(fee)?;

        Ok(())
    }

    pub fn update_lock(ctx: Context<UpdateConfig>) -> Result<()> {
        ctx.accounts.update_lock()?;

        Ok(())
    }

    pub fn remove_authority(ctx: Context<UpdateConfig>) -> Result<()> {
        ctx.accounts.remove_authority()?;

        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, max_x: u64, max_y: u64, expiration: i64) -> Result<()> {
        ctx.accounts.deposit(amount, max_x, max_y, expiration)?;

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, min_x: u64, min_y: u64, expiration: i64) -> Result<()> {
        ctx.accounts.withdraw(amount, min_x, min_y, expiration)?;

        Ok(())
    }

    pub fn swap(ctx: Context<Swap>, is_x: bool, amount: u64, min: u64, expiration: i64) -> Result<()> {
        ctx.accounts.swap(is_x, amount, min, expiration)?;

        Ok(())
    }    
}
