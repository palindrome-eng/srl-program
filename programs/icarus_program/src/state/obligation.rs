pub use {
    super::*,
    anchor_lang::prelude::*,
    solana_program::clock::Epoch,
    crate::error::LendingError,
};

/// How does `Obligation` work for Icarus:

/// Since the user in V1 can borrow SOL only from vault associated with
/// a certain vote_account (this is to outperform the underlying LST) we
/// created the concept of positions. `ObligationPosition` are singular 
/// for each vote account the user decide to borrow against, and contain:
/// starting deposit amount, the correspondant wighted amount, the 
/// borrowed amount and a `epoch_status` field.
///
/// `ObligationPosition`:
/// - The `LoanType` enum make sure that we take fees from the deposit amount 
/// if they are borrowing for more than the time limit. This get's enforced on
/// liquidation or on repayment using the `check_deposit_status` method.
/// - The collateral is deposited in the `deposited_amount` field and if follows
/// the same logic as an LSTs, the `weighted_deposited_amount` is used to calculate
/// the real position in SOL.

/// Lending market obligation state
#[account]
#[derive(Default)]
pub struct Obligation {
    /// Version of the struct
    pub version: u8,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Owner of the obligation
    pub owner: Pubkey,
    /// Current Active Position
    pub positions: Vec<ObligationPosition>,
    /// Bump seed for Obligation
    pub bump: u8,
}

// Maxing out the space because it could be custom rule on the resize and don't want to handle that manually.
impl Space for Obligation {
    const INIT_SPACE: usize = 1024; 
}

impl Obligation {
    /// Create a new obligation
    pub fn new(params: InitObligationParams) -> Self {
        let mut obligation = Self::default();
        Self::init(&mut obligation, params);
        obligation
    }
    
    /// Initialize an obligation
    pub fn init(&mut self, params: InitObligationParams) {
        self.version = PROGRAM_VERSION;
        self.lending_market = params.lending_market;
        self.owner = params.owner;
        self.positions = vec![];
        self.bump = params.bump
    }

    /// Get or create a new position
    pub fn add_or_create_position(&mut self, params: InitObligationPositionParams) -> Result<()> {
        match self.find_index(params.vote_account) {
            Some(index) => {
                // Position exists, update it
                let position = &mut self.positions[index];
                require!(position.loan_type == params.loan_type, LendingError::LoanTypeMismatch);
                position.deposit(params.deposited_amount, params.weighted_deposited_amount)?;
                position.borrow(params.borrowed_amount)?;
            },
            None => {
                // Position doesn't exist, create a new one
                let new_position = ObligationPosition::new(params);
                self.positions.push(new_position);
            }
        }
        
        Ok(())
    }

    /// Repay Loan
    pub fn repay_or_liquidate(&mut self, vote_account: Pubkey, current_epoch: Epoch) -> Result<(bool, u64)> {
        let index = self.find_index(vote_account).ok_or(LendingError::InvalidObligationPositionIndex)?;
        let (is_liquidatable, deposited_amount_after_fees) = self.positions[index].get_deposit_status(current_epoch)?;
        self.positions.remove(index);

        Ok((is_liquidatable, deposited_amount_after_fees)) 
    }

    /// Find epoch status by vote_account
    pub fn find_loan_type(&self, vote_account: Pubkey) -> Result<LoanType> {
        let position = self.find_position(vote_account)?.0;
        Ok(position.loan_type)
    }

    /// Find collateral by vote_account
    pub fn find_collaterals(&self, vote_account: Pubkey) -> Result<(u64, u64)> {
        let position = self.find_position(vote_account)?.0;
        Ok((position.deposited_amount, position.weighted_deposited_amount))
    }

    /// Find liquidity by vote_account
    pub fn find_liquidity(&self, vote_account: Pubkey) -> Result<u64> {
        let position = self.find_position(vote_account)?.0;
        Ok(position.borrowed_amount)
    }

    /// Find position by vote_account
    pub fn find_position(&self, vote_account: Pubkey) -> Result<(&ObligationPosition, usize)> {
        if self.positions.is_empty() {
            msg!("Obligation has no Collateral");
            return Err(LendingError::ObligationPositionEmpty.into());
        }
        let position_index = self
            .find_index(vote_account)
            .ok_or(LendingError::InvalidObligationPositionIndex)?;
        Ok((&self.positions[position_index], position_index))
    }

    /// Find index by vote_account
    pub fn find_index(&self, vote_account: Pubkey) -> Option<usize> {
        self.positions
            .iter()
            .position(|position| position.vote_account == vote_account)
    }
}

/// Initialize an obligation
pub struct InitObligationParams {
    /// Lending market address
    pub lending_market: Pubkey,
    /// Owner authority which can borrow liquidity
    pub owner: Pubkey,
    /// Bump seed for Obligation
    pub bump: u8,
}

/// Obligation Position
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ObligationPosition {
    /// Vote Account Address
    pub vote_account: Pubkey,
    /// Status on the Borrowing
    pub loan_type: LoanType,
    /// Amount of Sol deposited 
    pub deposited_amount: u64,
    /// Amount of collateral deposited (Weighted)
    pub weighted_deposited_amount: u64,
    /// Amount of Sol borrowed
    pub borrowed_amount: u64,
}

impl Space for ObligationPosition {
    const INIT_SPACE: usize = 8 + 32 + LoanType::INIT_SPACE + 8 + 8 + 8;
}

impl ObligationPosition {
    /// Create new obligation collateral
    pub fn new(params: InitObligationPositionParams) -> Self {
        Self {
            vote_account: params.vote_account,
            loan_type: params.loan_type,
            deposited_amount: params.deposited_amount,
            weighted_deposited_amount: params.weighted_deposited_amount,
            borrowed_amount: params.borrowed_amount,
        }
    }

    /// Increase deposited collateral
    pub fn deposit(&mut self, collateral_amount: u64, weighted_collateral_amount: u64) -> Result<()> {
        self.deposited_amount = self
            .deposited_amount
            .checked_add(collateral_amount)
            .ok_or(LendingError::MathOverflow)?;
        self.weighted_deposited_amount = self
            .weighted_deposited_amount
            .checked_add(weighted_collateral_amount)
            .ok_or(LendingError::MathOverflow)?;

        Ok(())
    }

    /// Decrease deposited collateral
    pub fn withdraw(&mut self, collateral_amount: u64, weighted_collateral_amount: u64) -> Result<()> {
        let new_deposited = self.deposited_amount.checked_sub(collateral_amount)
            .ok_or(LendingError::InsufficientCollateral)?;

        self.check_ltv(self.borrowed_amount, new_deposited)?;

        self.deposited_amount = new_deposited;
        self.weighted_deposited_amount = self.weighted_deposited_amount
            .checked_sub(weighted_collateral_amount)
            .ok_or(LendingError::MathOverflow)?;

        Ok(())
    }

    /// Increase borrowed liquidity
    pub fn borrow(&mut self, borrow_amount: u64) -> Result<()> {
        let new_borrowed = self.borrowed_amount.checked_add(borrow_amount)
            .ok_or(LendingError::MathOverflow)?;

        self.check_ltv(new_borrowed, self.deposited_amount)?;

        self.borrowed_amount = new_borrowed;
        Ok(())
    }

    /// Check the Loan to Value ratio of the position after a change
    fn check_ltv(&self, new_borrowed: u64, new_deposited: u64) -> Result<()> {
        let ltv = new_borrowed
            .checked_mul(100)
            .and_then(|v| v.checked_div(new_deposited))
            .ok_or(LendingError::MathOverflow)?;

        let max_ratio = match self.loan_type {
            LoanType::SHORT(_) => SHORT_MAX_RATIO,
            LoanType::MEDIUM(_) => MEDIUM_MAX_RATIO,
            LoanType::LONG(_) => LONG_MAX_RATIO,
        };

        if ltv > max_ratio {
            return Err(error!(LendingError::LoanToValueTooHigh));
        }

        Ok(())
    }

    /// Checks the health of the deposit and returns the amount after fees and if it's liquidatable
    pub fn get_deposit_status(&self, current_epoch: Epoch) -> Result<(bool, u64)> {
        let (start_epoch, loan_duration) = match self.loan_type {
            LoanType::SHORT(epoch) => (epoch, SHORT_LOAN_DURATION),
            LoanType::MEDIUM(epoch) => (epoch, MEDIUM_LOAN_DURATION),
            LoanType::LONG(epoch) => (epoch, LONG_LOAN_DURATION),
        };
    
        let epoch_elapsed = current_epoch.saturating_sub(start_epoch);
        
        if epoch_elapsed <= loan_duration {
            // Before loan duration: no late fees
            let is_liquidatable = self.borrowed_amount > self.deposited_amount;
            return Ok((is_liquidatable, if is_liquidatable { self.borrowed_amount } else { self.deposited_amount }));
        }
    
        // After loan duration: apply late fees
        let late_epochs = epoch_elapsed - loan_duration;
        let fee_percentage = 100u64.saturating_sub(late_epochs.saturating_mul(LATE_FEE));
        
        let deposited_amount_after_fees = self.deposited_amount
            .saturating_mul(fee_percentage)
            .checked_div(100)
            .ok_or(LendingError::MathOverflow)?;
    
        let is_liquidatable = self.borrowed_amount > deposited_amount_after_fees;
    
        Ok((is_liquidatable, if is_liquidatable { self.borrowed_amount } else { deposited_amount_after_fees }))
    }

    ///
    pub fn get_ltv_to_max_ratio(&self) -> Result<u64> {        
        let ltv = self.borrowed_amount
            .checked_mul(WAD) 
            .and_then(|v| v.checked_div(self.deposited_amount))  
            .ok_or(LendingError::MathOverflow)?;
    
        let max_ratio = match self.loan_type {
            LoanType::SHORT(_) => SHORT_MAX_RATIO,
            LoanType::MEDIUM(_) => MEDIUM_MAX_RATIO,
            LoanType::LONG(_) => LONG_MAX_RATIO,
        };
    
        let normalized_ltv = ltv
            .checked_mul(100) 
            .and_then(|v| v.checked_div(max_ratio)) 
            .ok_or(LendingError::MathOverflow)?;
    
        Ok(u64::from(normalized_ltv
            .checked_div(WAD)
            .ok_or(LendingError::MathOverflow)?
        ))
        
    }
}

/// Initialize an obligation
pub struct InitObligationPositionParams {
    /// Vote Account Address
    pub vote_account: Pubkey,
    /// Loan Type
    pub loan_type: LoanType,
    /// Deposit Amount
    pub deposited_amount: u64,
    /// Weighted Deposit Amount
    pub weighted_deposited_amount: u64,
    /// Borrowed Amount
    pub borrowed_amount: u64,
}

/// An enum representing the types of status Borrowing of the Stake Account can go trough
#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, PartialEq, Eq)]
pub enum LoanType {
    /// 15 Epochs Loan + Starting Epoch
    SHORT(Epoch),
    /// 45 Epochs Loan + Starting Epoch
    MEDIUM(Epoch),
    /// 90 Epochs Loan + Starting Epoch
    LONG(Epoch),
}

impl Space for LoanType {
    const INIT_SPACE: usize = 1 + 8;
}

impl Default for LoanType {
    fn default() -> Self {
        LoanType::SHORT(0)
    }
}

impl LoanType {
    /// Get the epoch of the Epoch Status
    pub fn epoch(&self) -> u64 {
        match self {
            Self::SHORT(epoch) => *epoch,
            Self::MEDIUM(epoch) => *epoch,
            Self::LONG(epoch) => *epoch,
        }
    }
}

// Note: we don't need to calculate the interest because it'100% of the rewards of 1/TVL * borrowed amount