use anchor_lang::error_code;

#[error_code]
pub enum Errors {
    #[msg("Zero deposit amount")]
    ZeroDeposit,
    #[msg("Insufficient base asset balance")]
    InsufficientBaseAssetBalance,
    #[msg("Math overflow occurred")]
    MathOverflow,
    #[msg("Maximum decimals exceeded")]
    MaxDecimalsExceeded,
    #[msg("Divided by zero error")]
    DivideByZero,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Deposits are currently paused")]
    DepositPaused,
    #[msg("Allocations are currently paused")]
    AllocatePaused,
}