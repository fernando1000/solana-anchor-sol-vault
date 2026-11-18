use anchor_lang::prelude::*;

use crate::{constants::*, error::VaultError, state::VaultState};

#[derive(Accounts)]
pub struct ExtendLock<'info> {
    pub owner: Signer<'info>,
    #[account(
        mut,
        has_one = owner,
        seeds = [STATE_SEED, owner.key().as_ref()],
        bump = state.state_bump
    )]
    pub state: Account<'info, VaultState>,
}

pub fn handle_extend_lock(ctx: Context<ExtendLock>, new_unlock_at: i64) -> Result<()> {
    let state = &mut ctx.accounts.state;
    require!(
        new_unlock_at > state.unlock_at,
        VaultError::CannotShortenLock
    );
    state.unlock_at = new_unlock_at;
    Ok(())
}
