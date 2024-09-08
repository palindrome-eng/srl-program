use anchor_lang::prelude::*;

#[error_code]
pub enum LendingError {
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Divide by zero")]
    DivideByZero,
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,
    #[msg("Wrong Stake Stake")]
    WrongStakeStake,
    #[msg("Reserve is stale and must be refreshed in the current slot")]
    ReserveStale,
    #[msg("Amount provided cannot be zero")]
    InvalidAmount
}

impl From<LendingError> for ProgramError {
    fn from(e: LendingError) -> Self {
        ProgramError::Custom(e as u32)
    }
}