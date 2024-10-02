use anchor_lang::prelude::*;

#[error_code]
pub enum LendingError {
    #[msg("Divide by zero")]
    DivideByZero,
    #[msg("Wrong Stake Stake")]
    WrongStakeStake,
    #[msg("Reserve is stale and must be refreshed in the current slot")]
    ReserveStale,
    #[msg("Amount provided cannot be zero")]
    InvalidAmount,
    #[msg("This is not the Owner of the Lending Market")]
    OwnerMismatch,


    #[msg("The account you passed is not a Reserve Account")]
    InvalidReserveAccount,
    #[msg("You have the wrong number of remaining accounts")]
    WrongRemainingAccountSchema,
    #[msg("The Position used is uncorrect")]
    PositionNotFound,
    #[msg("The LastUpdated used is invalid")]
    InvalidLastUpdate,
    #[msg("The Position used is invalid")]
    InvalidPosition,

    /// Action Errors
    #[msg("The position is not liquidatable")]
    NotLiquidatable,
    #[msg("The Lending Market is not the same as the one in the Reserve Account")]
    LendingMarketMismatch,
    #[msg("The Loan type passed is invalid")]
    InvalidLoanType,
    #[msg("The collateral amount provided doesn't match with the amount in the stake account")]
    InvalidStakeAmount,

    /// Reserve Errors
    #[msg("Insufficient liquidity in the Reserve Account to perform this action")] 
    InsufficientLiquidity,

    /// Obligation Errors
    #[msg("There are no positions in this Obligation Account")]
    ObligationPositionEmpty,
    #[msg("There is no position related to this vote_account in this Obligation Account")]
    InvalidObligationPositionIndex,
    #[msg("Insufficient Collateral to decrease the deposit")]
    InsufficientCollateral,
    #[msg("The LTV of the Obligation after the change is too high")]
    LoanToValueTooHigh,
    #[msg("The Loan type passed for the loan creation is different from what is in the Obligation")]
    LoanTypeMismatch,

    /// General Errors
    #[msg("Math overflow")]
    MathOverflow,
}