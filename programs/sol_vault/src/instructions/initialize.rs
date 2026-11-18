use anchor_lang::prelude::*;

use crate::{constants::*, error::VaultError, state::VaultState};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        space = 8 + VaultState::INIT_SPACE,
        seeds = [STATE_SEED, owner.key().as_ref()],
        bump
    )]
    pub state: Account<'info, VaultState>,
    #[account(seeds = [VAULT_SEED, state.key().as_ref()], bump)]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(ctx: Context<Initialize>, unlock_at: i64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    require!(unlock_at > now, VaultError::UnlockInThePast);

    let state = &mut ctx.accounts.state;
    state.owner = ctx.accounts.owner.key();
    state.unlock_at = unlock_at;
    state.state_bump = ctx.bumps.state;
    state.vault_bump = ctx.bumps.vault;
    Ok(())
}
