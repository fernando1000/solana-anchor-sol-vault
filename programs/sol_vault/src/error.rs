use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Unlock time must be in the future")]
    UnlockInThePast,
    #[msg("Vault is still locked")]
    StillLocked,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("New unlock time must be later than the current one")]
    CannotShortenLock,
}
