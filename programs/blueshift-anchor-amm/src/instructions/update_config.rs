use anchor_lang::prelude::*;
use crate::errors::AmmError;
use crate::state::Config;

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut, 
        seeds = [b"config", config.seed.to_le_bytes().as_ref(), config.mint_x.key().as_ref(), config.mint_y.key().as_ref()], 
        bump,
    )]
    pub config: Account<'info, Config>,
}

impl<'info> UpdateConfig<'info> {
    pub fn update_authority(
        &mut self,
        authority: Pubkey,
    ) -> Result<()> {
        // Check if the authority is the same as the current authority
        if let Some(current_authority) = self.config.authority {
            require!(current_authority == self.authority.key(), AmmError::InvalidAuthority);
        } else {
            return Err(AmmError::AmmIsImmutable.into());
        }

        // Update the authority
        self.config.authority = Some(authority);

        Ok(())
    }

    pub fn update_fee(
        &mut self,
        fee: u16,
    ) -> Result<()> {
        // Check if the fee is valid
        require!(fee <= 10000, AmmError::InvalidFee);

        // Check if the authority is the same as the current authority
        if let Some(current_authority) = self.config.authority {
            require!(current_authority == self.authority.key(), AmmError::InvalidAuthority);
        } else {
            return Err(AmmError::AmmIsImmutable.into());
        }

        // Update the fee
        self.config.fee = fee;

        Ok(())
    }

    pub fn update_lock(
        &mut self,
    ) -> Result<()> {
        // Check if the authority is the same as the current authority
        if let Some(current_authority) = self.config.authority {
            require!(current_authority == self.authority.key(), AmmError::InvalidAuthority);
        } else {
            return Err(AmmError::AmmIsImmutable.into());
        }

        // Update the lock
        self.config.locked = !self.config.locked;

        Ok(())
    }

    pub fn remove_authority(
        &mut self,
    ) -> Result<()> {
        // Check if the authority is the same as the current authority
        if let Some(current_authority) = self.config.authority {
            require!(current_authority == self.authority.key(), AmmError::InvalidAuthority);
        } else {
            return Err(AmmError::AmmIsImmutable.into());
        }

        // Remove the authority
        self.config.authority = None;

        Ok(())
    }
}