use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{constants::*, error::VaultError, state::VaultState};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        has_one = owner,
        close = owner,
        seeds = [STATE_SEED, owner.key().as_ref()],
        bump = state.state_bump
    )]
    pub state: Account<'info, VaultState>,
    #[account(mut, seeds = [VAULT_SEED, state.key().as_ref()], bump = state.vault_bump)]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct Withdrawn {
    pub owner: Pubkey,
    pub amount: u64,
}

pub fn handle_withdraw(ctx: Context<Withdraw>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    require!(now >= ctx.accounts.state.unlock_at, VaultError::StillLocked);

    let amount = ctx.accounts.vault.lamports();
    let state_key = ctx.accounts.state.key();
    let vault_bump = ctx.accounts.state.vault_bump;
    let signer_seeds: &[&[&[u8]]] = &[&[VAULT_SEED, state_key.as_ref(), &[vault_bump]]];
    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.owner.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.system_program.key(), cpi_accounts);
    transfer(cpi_ctx.with_signer(signer_seeds), amount)?;

    emit!(Withdrawn {
        owner: ctx.accounts.owner.key(),
        amount,
    });
    Ok(())
}
