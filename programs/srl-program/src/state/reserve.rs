pub use {
    super::*,
    crate::error::LendingError,
    anchor_lang::prelude::*,
    solana_program::{clock::{Epoch, Slot}, native_token::LAMPORTS_PER_SOL},
};

#[account]
#[derive(Default)]
pub struct Reserve {
    /// Version of the struct
    pub version: u8,
    /// Last Epoch when updated
    pub last_epoch: Epoch,
    /// Last slot when supply and rates updated
    pub last_update: LastUpdate,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Reserve liquidity
    pub liquidity: ReserveLiquidity,
}

impl Space for Reserve {
    const INIT_SPACE: usize = 8 + 1 + LastUpdate::INIT_SPACE + 32 + ReserveLiquidity::INIT_SPACE + 128;
}

impl Reserve {
    /// Create a new reserve
    pub fn new(params: InitReserveParams) -> Self {
        let mut reserve = Self::default();
        Self::init(&mut reserve, params);
        reserve
    }

    /// Initialize a reserve
    pub fn init(&mut self, params: InitReserveParams) {
        self.version = PROGRAM_VERSION;
        self.last_epoch = params.current_epoch;
        self.last_update = LastUpdate::new(params.current_slot);
        self.lending_market = params.lending_market;
        self.liquidity = params.liquidity;
    }

    /// Return epoch elapsed since given epoch
    pub fn epoch_elapsed(&self, epoch: Epoch) -> Result<u64> {
        let epoch_elapsed = epoch
            .checked_sub(self.last_epoch)
            .ok_or(LendingError::MathOverflow)?;
        Ok(epoch_elapsed)
    }

    /// Set last update epoch
    pub fn update_epoch(&mut self, epoch: Epoch) {
        self.last_epoch = epoch;
    }

    /// Record deposited liquidity and return amount of collateral tokens to mint
    pub fn deposit(&mut self, liquidity_amount: u64) -> Result<u64> {
        let token_amount = self
            .calculate_token_position(
                liquidity_amount,
                self.liquidity.total_liquidity()?,
                self.liquidity.mint_total_supply,
            )?;

        self.liquidity.deposit(liquidity_amount)?;
        self.liquidity.mint(token_amount)?;

        Ok(token_amount)
    }

    pub fn reedem(&mut self, token_amount: u64) -> Result<u64> {
        let liquidity_amount = self
            .calculate_liquidity_position(
                token_amount,
                self.liquidity.total_liquidity()?,
                self.liquidity.mint_total_supply,
            )?;

        self.liquidity.withdraw(liquidity_amount)?;
        self.liquidity.burn(token_amount)?;

        Ok(liquidity_amount)
    }

    /// Calculate pool tokens to mint, given total token supply, total liquidity, liquidity deposit
    pub fn calculate_token_position(
        &self,
        liquidity_deposit: u64,
        total_liquidity: u128,
        total_token_supply: u64,
    ) -> Result<u64> {
        if total_liquidity == 0 || total_token_supply == 0 {
            return Ok(liquidity_deposit);
        } else {
            Ok(u64::try_from(
                (liquidity_deposit as u128)
                    .checked_mul(total_token_supply as u128)
                    .ok_or(LendingError::MathOverflow)?
                    .checked_div(total_liquidity)
                    .ok_or(LendingError::MathOverflow)?,
            ).map_err(|_| LendingError::MathOverflow)?)
        }
    }

    /// Calculate liquidity to withdraw, given total token supply, total liquidity, pool tokens to burn
    pub fn calculate_liquidity_position(
        &self,
        token_to_burn: u64,
        total_liquidity: u128,
        total_token_supply: u64,
    ) -> Result<u64> {
        if total_liquidity == 0 || total_token_supply == 0 {
            return Ok(0);
        } else {
            Ok(u64::try_from(
                (token_to_burn as u128)
                    .checked_mul(total_liquidity)
                    .ok_or(LendingError::MathOverflow)?
                    .checked_div(total_token_supply as u128)
                    .ok_or(LendingError::MathOverflow)?,
            ).map_err(|_| LendingError::MathOverflow)?)
        }
    }

    // pub fn current_borrow_rate -- To be implemented

    // pub fn accrue_interest -- To be implemented

    // pub fn calculate_borrow -- To be implemented

    // pub fn calculate_repay -- To be implemented

    // pub fn calculate_liquidation -- To be implemented

}

/// Initialize a reserve
pub struct InitReserveParams {
    /// Last epoch when supply and rates updated
    pub current_epoch: Epoch,
    /// Last slot when supply and rates updated
    pub current_slot: Slot,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Reserve liquidity
    pub liquidity: ReserveLiquidity,
}

/// Reserve liquidity
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, InitSpace)]
pub struct ReserveLiquidity {
    pub mint_pubkey: Pubkey,
    pub mint_total_supply: u64,
    pub vault_pubkey: Pubkey,
    pub available_amount: u64,
    pub borrowed_amount_wads: u128,
    pub cumulative_borrow_rate_wads: u128,
}

impl ReserveLiquidity {
    pub fn new(params: NewReserveLiquidityParams) -> Self {
        Self {
            mint_pubkey: params.mint_pubkey,
            mint_total_supply: 0,
            vault_pubkey: params.vault_pubkey,
            available_amount: 0,
            borrowed_amount_wads: 0,
            cumulative_borrow_rate_wads: WAD as u128,
        }
    }

    pub fn total_liquidity(&self) -> Result<u128> {
        (self.available_amount as u128)
            .checked_add(self.borrowed_amount_wads)
            .ok_or_else(|| error!(LendingError::MathOverflow))
    }

    pub fn mint(&mut self, mint_amount: u64) -> Result<()> {
        self.mint_total_supply = self.mint_total_supply
            .checked_add(mint_amount)
            .ok_or_else(|| error!(LendingError::MathOverflow))?;
        Ok(())
    }

    pub fn burn(&mut self, burn_amount: u64) -> Result<()> {
        self.mint_total_supply = self.mint_total_supply
            .checked_sub(burn_amount)
            .ok_or_else(|| error!(LendingError::MathOverflow))?;
        Ok(())
    }

    pub fn deposit(&mut self, liquidity_amount: u64) -> Result<()> {
        self.available_amount = self.available_amount
            .checked_add(liquidity_amount)
            .ok_or_else(|| error!(LendingError::MathOverflow))?;
        Ok(())
    }

    pub fn withdraw(&mut self, liquidity_amount: u64) -> Result<()> {
        if liquidity_amount > self.available_amount {
            return Err(error!(LendingError::InsufficientLiquidity));
        }
        self.available_amount = self.available_amount
            .checked_sub(liquidity_amount)
            .ok_or_else(|| error!(LendingError::MathOverflow))?;
        Ok(())
    }

    pub fn borrow(&mut self, borrow_amount: u64) -> Result<()> {
        if borrow_amount > self.available_amount {
            return Err(error!(LendingError::InsufficientLiquidity));
        }
        self.available_amount = self.available_amount
            .checked_sub(borrow_amount)
            .ok_or_else(|| error!(LendingError::MathOverflow))?;
        self.borrowed_amount_wads = self.borrowed_amount_wads
            .checked_add((borrow_amount as u128).checked_mul(WAD as u128)
                .ok_or_else(|| error!(LendingError::MathOverflow))?)
            .ok_or_else(|| error!(LendingError::MathOverflow))?;
        Ok(())
    }

    pub fn repay(&mut self, repay_amount: u64, settle_amount_wads: u128) -> Result<()> {
        self.available_amount = self.available_amount
            .checked_add(repay_amount)
            .ok_or_else(|| error!(LendingError::MathOverflow))?;
        self.borrowed_amount_wads = self.borrowed_amount_wads
            .checked_sub(settle_amount_wads)
            .ok_or_else(|| error!(LendingError::MathOverflow))?;
        Ok(())
    }

    // Calculate the liquidity utilization rate of the reserve
    // pub fn utilization_rate -- To be implemented

    // Compound current borrow rate over elapsed slots
    // fn compound_interest -- To be implemented
}

/// New reserve liquidity parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct NewReserveLiquidityParams {
    pub mint_pubkey: Pubkey,
    pub vault_pubkey: Pubkey,
}

// /// Reserve mints
// #[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
// pub struct ReserveMints {
//     pub liquidity_mint_pubkey: Pubkey,
//     pub liquidity_supply: u64,
//     pub collateral_mint_pubkey: Pubkey,
//     pub collateral_supply: u64,
// }

// impl ReserveMints {
//     pub fn mint_liquidity(&mut self, amount: u64) -> Result<()> {
//         self.liquidity_supply = self.liquidity_supply
//             .checked_add(amount)
//             .ok_or_else(|| error!(LendingError::MathOverflow))?;
//         Ok(())
//     }

//     pub fn burn_liquidity(&mut self, amount: u64) -> Result<()> {
//         self.liquidity_supply = self.liquidity_supply
//             .checked_sub(amount)
//             .ok_or_else(|| error!(LendingError::MathOverflow))?;
//         Ok(())
//     }

//     pub fn mint_collateral(&mut self, amount: u64) -> Result<()> {
//         self.collateral_supply = self.collateral_supply
//             .checked_add(amount)
//             .ok_or_else(|| error!(LendingError::MathOverflow))?;
//         Ok(())
//     }

//     pub fn burn_collateral(&mut self, amount: u64) -> Result<()> {
//         self.collateral_supply = self.collateral_supply
//             .checked_sub(amount)
//             .ok_or_else(|| error!(LendingError::MathOverflow))?;
//         Ok(())
//     }
// }

// // ToDo: Run Calculation for ReserveConfig
// #[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
// pub struct ReserveConfig {
//     /// Optimal utilization rate, as a percentage
//     pub optimal_utilization_rate: u8,
//     /// Target ratio of the value of borrows to deposits, as a percentage
//     /// 0 if use as collateral is disabled
//     pub loan_to_value_ratio: u8,
//     /// Min borrow APY
//     pub min_borrow_rate: u8,
//     /// Optimal (utilization) borrow APY
//     pub optimal_borrow_rate: u8,
//     /// Max borrow APY
//     pub max_borrow_rate: u8,
//     /// Program owner fees assessed, separate from gains due to interest accrual
//     pub fees: ReserveFees,
// }

// // impl ReserveConfig {
// //     /// Validate the reserve configs, when initializing or modifying the reserve
// //     /// configs
// //     pub fn validate(&self) -> ProgramResult {
// //         if self.optimal_utilization_rate > 100 {
// //             msg!("Optimal utilization rate must be in range [0, 100]");
// //             return Err(LendingError::InvalidConfig.into());
// //         }
// //         if self.loan_to_value_ratio >= 100 {
// //             msg!("Loan to value ratio must be in range [0, 100)");
// //             return Err(LendingError::InvalidConfig.into());
// //         }
// //         if self.liquidation_bonus > 100 {
// //             msg!("Liquidation bonus must be in range [0, 100]");
// //             return Err(LendingError::InvalidConfig.into());
// //         }
// //         if self.liquidation_threshold <= self.loan_to_value_ratio
// //             || self.liquidation_threshold > 100
// //         {
// //             msg!("Liquidation threshold must be in range (LTV, 100]");
// //             return Err(LendingError::InvalidConfig.into());
// //         }
// //         if self.optimal_borrow_rate < self.min_borrow_rate {
// //             msg!("Optimal borrow rate must be >= min borrow rate");
// //             return Err(LendingError::InvalidConfig.into());
// //         }
// //         if self.optimal_borrow_rate > self.max_borrow_rate {
// //             msg!("Optimal borrow rate must be <= max borrow rate");
// //             return Err(LendingError::InvalidConfig.into());
// //         }
// //         if self.fees.borrow_fee_wad >= WAD {
// //             msg!("Borrow fee must be in range [0, 1_000_000_000_000_000_000)");
// //             return Err(LendingError::InvalidConfig.into());
// //         }
// //         if self.fees.flash_loan_fee_wad >= WAD {
// //             msg!("Flash loan fee must be in range [0, 1_000_000_000_000_000_000)");
// //             return Err(LendingError::InvalidConfig.into());
// //         }
// //         if self.fees.host_fee_percentage > 100 {
// //             msg!("Host fee percentage must be in range [0, 100]");
// //             return Err(LendingError::InvalidConfig.into());
// //         }
// //         Ok(())
// //     }
// // }

// /// Additional fee information on a reserve
// ///
// /// These exist separately from interest accrual fees, and are specifically for
// /// the program owner and frontend host. The fees are paid out as a percentage
// /// of liquidity token amounts during repayments and liquidations.
// #[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug, Default, PartialEq)]
// pub struct ReserveFees {
//     /// Fee assessed on `BorrowObligationLiquidity`, expressed as a Wad.
//     /// Must be between 0 and 10^18, such that 10^18 = 1.  A few examples for
//     /// clarity:
//     /// 1% = 10_000_000_000_000_000
//     /// 0.01% (1 basis point) = 100_000_000_000_000
//     /// 0.00001% (Aave borrow fee) = 100_000_000_000
//     pub borrow_fee_wad: u64,
//     /// Amount of fee going to host account, if provided in liquidate and repay
//     pub host_fee_percentage: u8,
// }


// impl ReserveFees {
//     /// Calculate the owner and host fees on borrow
//     pub fn calculate_borrow_fees(
//         &self,
//         borrow_amount: Decimal,
//         fee_calculation: FeeCalculation,
//     ) -> Result<(u64, u64), ProgramError> {
//         self.calculate_fees(borrow_amount, self.borrow_fee_wad, fee_calculation)
//     }

//     /// Calculate the owner and host fees on flash loan
//     pub fn calculate_flash_loan_fees(
//         &self,
//         flash_loan_amount: Decimal,
//     ) -> Result<(u64, u64), ProgramError> {
//         self.calculate_fees(
//             flash_loan_amount,
//             self.flash_loan_fee_wad,
//             FeeCalculation::Exclusive,
//         )
//     }

//     fn calculate_fees(
//         &self,
//         amount: Decimal,
//         fee_wad: u64,
//         fee_calculation: FeeCalculation,
//     ) -> Result<(u64, u64), ProgramError> {
//         let borrow_fee_rate = Rate::from_scaled_val(fee_wad);
//         let host_fee_rate = Rate::from_percent(self.host_fee_percentage);
//         if borrow_fee_rate > Rate::zero() && amount > Decimal::zero() {
//             let need_to_assess_host_fee = host_fee_rate > Rate::zero();
//             let minimum_fee = if need_to_assess_host_fee {
//                 2u64 // 1 token to owner, 1 to host
//             } else {
//                 1u64 // 1 token to owner, nothing else
//             };

//             let borrow_fee_amount = match fee_calculation {
//                 // Calculate fee to be added to borrow: fee = amount * rate
//                 FeeCalculation::Exclusive => amount.try_mul(borrow_fee_rate)?,
//                 // Calculate fee to be subtracted from borrow: fee = amount * (rate / (rate + 1))
//                 FeeCalculation::Inclusive => {
//                     let borrow_fee_rate =
//                         borrow_fee_rate.try_div(borrow_fee_rate.try_add(Rate::one())?)?;
//                     amount.try_mul(borrow_fee_rate)?
//                 }
//             };

//             let borrow_fee_decimal = borrow_fee_amount.max(minimum_fee.into());
//             if borrow_fee_decimal >= amount {
//                 msg!("Borrow amount is too small to receive liquidity after fees");
//                 return Err(LendingError::BorrowTooSmall.into());
//             }

//             let borrow_fee = borrow_fee_decimal.try_round_u64()?;
//             let host_fee = if need_to_assess_host_fee {
//                 borrow_fee_decimal
//                     .try_mul(host_fee_rate)?
//                     .try_round_u64()?
//                     .max(1u64)
//             } else {
//                 0
//             };

//             Ok((borrow_fee, host_fee))
//         } else {
//             Ok((0, 0))
//         }
//     }
// }

// /// Calculate fees exlusive or inclusive of an amount
// pub enum FeeCalculation {
//     /// Fee added to amount: fee = rate * amount
//     Exclusive,
//     /// Fee included in amount: fee = (rate / (1 + rate)) * amount
//     Inclusive,
// }


