pub use {
    super::*,
    anchor_lang::prelude::*,
    solana_program::{clock::{Epoch, Slot}, native_token::LAMPORTS_PER_SOL},
    crate::error::LendingError,
};

/// How does `Reserve` work for Icarus:
/// 
/// todo!()

#[account]
#[derive(Default)]
pub struct Reserve {
    /// Version of the struct
    pub version: u8,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Vote account address
    pub vote_account: Pubkey,
    /// Last updated epoch
    pub last_epoch: Epoch,
    /// Last slot where supply and rates got updated
    pub last_update: LastUpdate,
    /// Reserve liquidity
    pub liquidity: ReserveLiquidity,
    /// Reserve Collateral
    pub collateral: ReserveCollateral,
    /// Bump seed for Reserve
    pub bump: u8,
    /// Bump seed for the Stake Account
    pub stake_bump: u8,
    /// Bump seed for the Vault Account
    pub vault_bump: u8,
}

impl Space for Reserve {
    const INIT_SPACE: usize = 8 + 1 + 32 + 32 + 8 + LastUpdate::INIT_SPACE + ReserveLiquidity::INIT_SPACE + ReserveCollateral::INIT_SPACE + 1 + 1 + 1 + 128;
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
        self.vote_account = params.vote_account;
        self.liquidity = params.liquidity;
        self.collateral = params.collateral;
        self.bump = params.bump;
        self.stake_bump = params.stake_bump;
        self.vault_bump = params.vault_bump;
    }

    /// Record deposited liquidity and return amount of collateral tokens to mint
    pub fn deposit(&mut self, liquidity_amount: u64) -> Result<u64> {
        let total_liquidity = self.liquidity.total_liquidity()?;
        let token_amount = self.calculate_token_position(
            liquidity_amount,
            total_liquidity,
            self.liquidity.mint_total_supply,
        )?;

        self.liquidity.deposit(liquidity_amount)?;
        self.liquidity.mint(token_amount)?;

        Ok(token_amount)
    }


    /// Record reedeemed liquidity and return amount of collateral to withdraw
    pub fn reedem(&mut self, token_amount: u64) -> Result<u64> {
        let total_liquidity = self.liquidity.total_liquidity()?;
        let liquidity_amount = self.calculate_liquidity_position(
            token_amount,
            total_liquidity,
            self.liquidity.mint_total_supply,
        )?;

        self.liquidity.withdraw(liquidity_amount)?;
        self.liquidity.burn(token_amount)?;

        Ok(liquidity_amount)
    }

    /// Record deposited collateral and return amount of collateral tokens to mint
    pub fn deposit_collateral(&mut self, collateral_amount: u64) -> Result<u64> {
        let total_collateral = self.collateral.collateral_amount;
        let token_amount = self.calculate_token_position(
            collateral_amount,
            total_collateral,
            self.collateral.mint_total_supply,
        )?;

        self.collateral.deposit(collateral_amount)?;
        self.collateral.mint(token_amount)?;

        Ok(token_amount)
    }


    /// Record reedeemed liquidity and return amount of collateral to withdraw
    pub fn reedem_collateral(&mut self, token_amount: u64) -> Result<u64> {
        let total_collateral = self.collateral.collateral_amount;
        let liquidity_amount = self.calculate_liquidity_position(
            token_amount,
            total_collateral,
            self.collateral.mint_total_supply,
        )?;

        self.collateral.withdraw(liquidity_amount)?;
        self.collateral.burn(token_amount)?;

        Ok(liquidity_amount)
    }

    /// Calculate pool tokens to mint, given total token supply, total liquidity, liquidity deposit
    pub fn calculate_token_position(
        &self,
        liquidity_deposit: u64,
        total_liquidity: u64,
        total_token_supply: u64,
    ) -> Result<u64> {
        if total_liquidity == 0 || total_token_supply == 0 {
            return Ok(liquidity_deposit);
        }

        let numerator = (liquidity_deposit as u128).checked_mul(total_token_supply as u128)
            .ok_or(LendingError::MathOverflow)?;
        let result = numerator.checked_div(total_liquidity as u128)
            .ok_or(LendingError::MathOverflow)?;

        u64::try_from(result).map_err(|_| LendingError::MathOverflow.into())
    }

    /// Calculate liquidity to withdraw, given total token supply, total liquidity, pool tokens to burn
    pub fn calculate_liquidity_position(
        &self,
        token_to_burn: u64,
        total_liquidity: u64,
        total_token_supply: u64,
    ) -> Result<u64> {
        if total_liquidity == 0 || total_token_supply == 0 {
            return Ok(0);
        }

        let numerator = (token_to_burn as u128).checked_mul(total_liquidity as u128)
            .ok_or(LendingError::MathOverflow)?;
        let result = numerator.checked_div(total_token_supply as u128)
            .ok_or(LendingError::MathOverflow)?;

        u64::try_from(result).map_err(|_| LendingError::MathOverflow.into())
    }

    /// Return epoch elapsed since given epoch
    pub fn epoch_elapsed(&self, epoch: Epoch) -> Result<u64> {
        epoch.checked_sub(self.last_epoch)
            .ok_or(LendingError::MathOverflow.into())
    }

    /// Set last update epoch
    pub fn update_epoch(&mut self, epoch: Epoch) {
        self.last_epoch = epoch;
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
    /// Vote account address
    pub vote_account: Pubkey,
    /// Reserve liquidity
    pub liquidity: ReserveLiquidity,
    /// Reserve Collateral
    pub collateral: ReserveCollateral,
    /// Bump seed for Reserve
    pub bump: u8,
    /// Bump seed for the Stake Account
    pub stake_bump: u8,
    /// Bump seed for the Vault Account
    pub vault_bump: u8,
}

/// Reserve liquidity
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, InitSpace)]
pub struct ReserveLiquidity {
    pub mint_pubkey: Pubkey,
    pub mint_total_supply: u64,
    pub vault_pubkey: Pubkey,
    pub available_amount: u64,
    pub borrowed_amount: u64,
    pub cumulative_borrow_rate_wads: u128,
}

impl ReserveLiquidity {
    pub fn new(params: NewReserveLiquidityParams) -> Self {
        Self {
            mint_pubkey: params.mint_pubkey,
            mint_total_supply: 0,
            vault_pubkey: params.vault_pubkey,
            available_amount: 0,
            borrowed_amount: 0,
            cumulative_borrow_rate_wads: WAD as u128,
        }
    }

    /// Calculate total liquidity in the reserve
    pub fn total_liquidity(&self) -> Result<u64> {
        self.available_amount
            .checked_add(self.borrowed_amount)
            .ok_or_else(|| error!(LendingError::MathOverflow))
    }

    /// Add minted tokens to the total supply
    pub fn mint(&mut self, mint_amount: u64) -> Result<()> {
        self.mint_total_supply = self.mint_total_supply
            .checked_add(mint_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Substract burned tokens from the total supply
    pub fn burn(&mut self, burn_amount: u64) -> Result<()> {
        self.mint_total_supply = self.mint_total_supply
            .checked_sub(burn_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Deposit liquidity into the reserve
    pub fn deposit(&mut self, liquidity_amount: u64) -> Result<()> {
        self.available_amount = self.available_amount
            .checked_add(liquidity_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Withdraw liquidity from the reserve
    pub fn withdraw(&mut self, liquidity_amount: u64) -> Result<()> {
        if liquidity_amount > self.available_amount {
            return Err(error!(LendingError::InsufficientLiquidity));
        }
        self.available_amount = self.available_amount
            .checked_sub(liquidity_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Borrow liquidity from the reserve
    pub fn borrow(&mut self, borrow_amount: u64) -> Result<()> {
        if borrow_amount > self.available_amount {
            return Err(error!(LendingError::InsufficientLiquidity));
        }
        self.available_amount = self.available_amount
            .checked_sub(borrow_amount)
            .ok_or(LendingError::MathOverflow)?;
        self.borrowed_amount = self.borrowed_amount
            .checked_add(borrow_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Repay borrowed liquidity to the reserve
    pub fn repay(&mut self, repay_amount: u64, settle_amount: u64) -> Result<()> {
        self.available_amount = self.available_amount
            .checked_add(repay_amount)
            .ok_or(LendingError::MathOverflow)?;
        self.borrowed_amount = self.borrowed_amount
            .checked_sub(settle_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    pub fn liquidate(&mut self, liquidate_amount: u64) -> Result<()> {
        self.borrowed_amount = self.borrowed_amount
            .checked_sub(liquidate_amount)
            .ok_or(LendingError::MathOverflow)?;
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

/// Reserve Collateral
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, InitSpace)]
pub struct ReserveCollateral {
    pub mint_pubkey: Pubkey,
    pub mint_total_supply: u64,
    pub stake_account: Pubkey,
    pub collateral_amount: u64,
    pub collateral_amount_to_claim: u64
}

impl ReserveCollateral {
    pub fn new(params: NewReserveCollateralParams) -> Self {
        Self {
            mint_pubkey: params.mint_pubkey,
            mint_total_supply: 0,
            stake_account: params.stake_account,
            collateral_amount: 0,
            collateral_amount_to_claim: 0,
        }
    }

    pub fn mint(&mut self, mint_amount: u64) -> Result<()> {
        Ok(self.mint_total_supply = self.mint_total_supply
            .checked_add(mint_amount)
            .ok_or(LendingError::MathOverflow)?)
    }

    pub fn burn(&mut self, burn_amount: u64) -> Result<()> {
        Ok(self.mint_total_supply = self.mint_total_supply
            .checked_sub(burn_amount)
            .ok_or(LendingError::MathOverflow)?
        )
    }

    pub fn claim_interest(&mut self, interest_amount: u64) -> Result<()> {
        Ok(self.collateral_amount_to_claim = self.collateral_amount_to_claim
            .checked_add(interest_amount)
            .ok_or(LendingError::MathOverflow)?
        )
    }

    pub fn repay_or_liquidate(&mut self, amount: u64, interest_amount: u64, weighted_amount: u64,) -> Result<()> {
        self.withdraw(amount)?;
        self.claim_interest(interest_amount)?;
        self.burn(weighted_amount)?;

        Ok(())
    }

    pub fn deposit(&mut self, collateral_amount: u64) -> Result<()> {
        self.collateral_amount = self.collateral_amount
            .checked_add(collateral_amount)
            .ok_or(LendingError::MathOverflow)?;

        Ok(())
    }

    pub fn withdraw(&mut self, collateral_amount: u64) -> Result<()> {        
        require_gte!(collateral_amount, self.collateral_amount, LendingError::InsufficientLiquidity);
        Ok(self.collateral_amount = self.collateral_amount
            .checked_sub(collateral_amount)
            .ok_or(LendingError::MathOverflow)?
        )
    }
}

/// New reserve collateral parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct NewReserveCollateralParams {
    pub mint_pubkey: Pubkey,
    pub stake_account: Pubkey,
}

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


