use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{constants::*, error::VaultError, state::VaultState};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        has_one = owner,
        seeds = [STATE_SEED, owner.key().as_ref()],
        bump = state.state_bump
    )]
    pub state: Account<'info, VaultState>,
    #[account(mut, seeds = [VAULT_SEED, state.key().as_ref()], bump = state.vault_bump)]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct Deposited {
    pub owner: Pubkey,
    pub amount: u64,
    pub total: u64,
}

pub fn handle_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::ZeroAmount);

    let cpi_accounts = Transfer {
        from: ctx.accounts.owner.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.system_program.key(), cpi_accounts);
    transfer(cpi_ctx, amount)?;

    emit!(Deposited {
        owner: ctx.accounts.owner.key(),
        amount,
        total: ctx.accounts.vault.lamports(),
    });
    Ok(())
}
