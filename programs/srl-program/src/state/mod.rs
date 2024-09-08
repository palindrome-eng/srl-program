mod lending_market;
mod reserve;
mod last_update;

pub use {lending_market::*, reserve::*, last_update::*};

/// Collateral tokens are initially valued at a ratio of 5:1
/// (collateral:liquidity)
// @FIXME: restore to 5
pub const INITIAL_COLLATERAL_RATIO: u64 = 1;
const INITIAL_COLLATERAL_RATE: u64 = INITIAL_COLLATERAL_RATIO * WAD;

/// Current version of the program and all new accounts created
pub const PROGRAM_VERSION: u8 = 1;

/// Accounts are created with data zeroed out, so uninitialized state instances
/// will have the version set to 0.
pub const UNINITIALIZED_VERSION: u8 = 0;

//// Scale of precision
pub const SCALE: usize = 18;
/// Identity
pub const WAD: u64 = 1_000_000_000_000_000_000;
/// Half of identity
pub const HALF_WAD: u64 = 500_000_000_000_000_000;
/// Scale for percentages
pub const PERCENT_SCALER: u64 = 10_000_000_000_000_000;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixedPrecision(pub u128);

impl FixedPrecision {
    pub fn from_wad(wad: u64) -> Self {
        FixedPrecision(wad as u128)
    }

    pub fn to_wad(&self) -> Result<u64> {
        u64::try_from(self.0).map_err(|_| error!(LendingError::MathOverflow))
    }

    pub fn from_percent(percent: u8) -> Self {
        FixedPrecision(((percent as u128) * PERCENT_SCALER as u128) / 100)
    }

    pub fn checked_add(&self, other: &Self) -> Result<Self> {
        self.0.checked_add(other.0)
            .map(FixedPrecision)
            .ok_or_else(|| error!(LendingError::MathOverflow))
    }

    pub fn checked_sub(&self, other: &Self) -> Result<Self> {
        self.0.checked_sub(other.0)
            .map(FixedPrecision)
            .ok_or_else(|| error!(LendingError::MathOverflow))
    }

    pub fn checked_mul(&self, other: &Self) -> Result<Self> {
        self.0.checked_mul(other.0)
            .and_then(|r| r.checked_div(WAD as u128))
            .map(FixedPrecision)
            .ok_or_else(|| error!(LendingError::MathOverflow))
    }

    pub fn checked_div(&self, other: &Self) -> Result<Self> {
        if other.0 == 0 {
            return Err(error!(LendingError::DivideByZero));
        }
        self.0.checked_mul(WAD as u128)
            .and_then(|r| r.checked_div(other.0))
            .map(FixedPrecision)
            .ok_or_else(|| error!(LendingError::MathOverflow))
    }

    pub fn round_u64(&self) -> u64 {
        ((self.0 + (HALF_WAD as u128)) / (WAD as u128)) as u64
    }
}