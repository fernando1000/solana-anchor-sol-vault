pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("DhWRSVhphtMsyNAQjnQXc2FWaq4W2fnLJnn2v4sXMzRK");

#[program]
pub mod sol_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, unlock_at: i64) -> Result<()> {
        handle_initialize(ctx, unlock_at)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        handle_deposit(ctx, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        handle_withdraw(ctx)
    }

    pub fn extend_lock(ctx: Context<ExtendLock>, new_unlock_at: i64) -> Result<()> {
        handle_extend_lock(ctx, new_unlock_at)
    }
}
