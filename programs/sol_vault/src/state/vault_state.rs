use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub owner: Pubkey,
    pub unlock_at: i64,
    pub state_bump: u8,
    pub vault_bump: u8,
}
