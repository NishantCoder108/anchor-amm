use anchor_lang::prelude::*;
use crate::errors::AmmError;
use crate::state::Config;

#[derive(Accounts)]
pub struct ConfigAction<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut, 
        seeds = [b"config", config.seed.to_le_bytes().as_ref(), config.mint_x.as_ref(), config.mint_y.as_ref()], 
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
}

impl<'info> ConfigAction<'info> {
    pub fn check_authority(
        &self,
    ) -> Result<()> {
        require_eq!(self.authority.key(), self.config.authority, AmmError::InvalidAuthority);

        Ok(())
    }

    pub fn update_authority(
        &mut self,
        authority: Pubkey,
    ) -> Result<()> {

        // Update the authority
        self.config.authority = authority;

        Ok(())
    }

    pub fn update_fee(
        &mut self,
        fee: u16,
    ) -> Result<()> {
        // Check if the fee is valid
        require_gte!(10000, fee, AmmError::InvalidFee);

        // Update the fee
        self.config.fee = fee;

        Ok(())
    }

    pub fn update_lock(
        &mut self,
    ) -> Result<()> {
        // Update the lock
        self.config.locked = !self.config.locked;

        Ok(())
    }

    pub fn remove_authority(
        &mut self,
    ) -> Result<()> {
        // Remove the authority
        self.config.authority = Pubkey::default();

        Ok(())
    }
}