use anchor_lang::prelude::*;
use crate::errors::AmmError;
use crate::state::{Config, LazyConfig};

#[derive(Accounts)]
pub struct ConfigAction<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut, 
        seeds = [b"config", config.load_seed()?.to_le_bytes().as_ref(), config.load_mint_x()?.key().as_ref(), config.load_mint_y()?.key().as_ref()], 
        bump = *config.load_bump()?,
    )]
    pub config: LazyAccount<'info, Config>,
}

impl<'info> ConfigAction<'info> {
    pub fn check_authority(
        &self,
    ) -> Result<()> {
        require_eq!(self.authority.key(), *self.config.load_authority()?, AmmError::InvalidAuthority);

        Ok(())
    }

    pub fn update_authority(
        &mut self,
        authority: Pubkey,
    ) -> Result<()> {

        // Update the authority
        *self.config.load_mut_authority()? = authority;

        Ok(())
    }

    pub fn update_fee(
        &mut self,
        fee: u16,
    ) -> Result<()> {
        // Check if the fee is valid
        require_gte!(10000, fee, AmmError::InvalidFee);

        // Update the fee
        *self.config.load_mut_fee()? = fee;

        Ok(())
    }

    pub fn update_lock(
        &mut self,
    ) -> Result<()> {
        // Update the lock
        *self.config.load_mut_locked()? = !*self.config.load_locked()?;

        Ok(())
    }

    pub fn remove_authority(
        &mut self,
    ) -> Result<()> {
        // Remove the authority
        *self.config.load_mut_authority()? = Pubkey::default();

        Ok(())
    }
}