pub use {
    anchor_lang::prelude::*,
    solana_program::{system_instruction, program::invoke, stake},
    crate::{get_stake_amount, state::{LendingMarket, Reserve, Obligation, LoanType, InitObligationPositionParams}, error::LendingError, LENDING_MARKET_AUTHORITY_PREFIX, RESERVE_PREFIX, OBLIGATION_PREFIX, RESERVE_STAKE_PREFIX},
};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug, PartialEq)]
pub struct BorrowLiquidityArgs {
    loan_type: u8,
    collateral_amount: u64,
    borrowed_amount: u64,
}

#[derive(Accounts)]
pub struct BorrowLiquidity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub lending_market: Account<'info, LendingMarket>,
    #[account(
        mut,
        has_one = lending_market,
        seeds = [RESERVE_PREFIX, lending_market.key().as_ref(), reserve.vote_account.as_ref()],
        bump = reserve.bump,
    )]
    pub reserve: Account<'info, Reserve>,
    #[account(
        mut,
        seeds = [RESERVE_STAKE_PREFIX, reserve.key().as_ref()],
        bump = reserve.stake_bump,
    )]
    /// CHECK: Validated in business logic
    pub reserve_stake: UncheckedAccount<'info>,
    #[account(
        mut,
        has_one = lending_market,
        seeds = [OBLIGATION_PREFIX, obligation.owner.as_ref()],
        bump,
    )]
    pub obligation: Account<'info, Obligation>,
    #[account(
        seeds = [LENDING_MARKET_AUTHORITY_PREFIX, lending_market.key().as_ref()],
        bump = lending_market.authority_bump,
    )]
    /// CHECK: PDA account, no need to validate
    pub lending_market_authority: UncheckedAccount<'info>,
    /// CHECK: Validated in business logic
    pub stake_account: UncheckedAccount<'info>,
    pub clock: Sysvar<'info, Clock>,
    pub stake_history: Sysvar<'info, StakeHistory>,
    pub system_program: Program<'info, System>,
}

impl<'info> BorrowLiquidity<'info> {
    fn merge_stake_account(&self, deposited_amount: u64) -> Result<()> {
        require_eq!(
            get_stake_amount(&self.stake_account.to_account_info())?,
            deposited_amount,
            LendingError::InvalidStakeAmount
        );

        invoke(
            &stake::instruction::authorize(
                self.stake_account.key,
                self.user.key,
                self.lending_market_authority.key,
                stake::state::StakeAuthorize::Staker,
                None,
            ),
            &[
                self.stake_account.to_account_info(),
                self.clock.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
        )?;

        invoke(
            &stake::instruction::authorize(
                self.stake_account.key,
                self.user.key,
                self.lending_market_authority.key,
                stake::state::StakeAuthorize::Withdrawer,
                None,
            ),
            &[
                self.stake_account.to_account_info(),
                self.clock.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
        )?;

        invoke(
            &stake::instruction::merge(
                self.reserve_stake.key,
                self.stake_account.key,
                self.user.key
            )[0],
            &[
                self.reserve_stake.to_account_info(),
                self.stake_account.to_account_info(),
                self.clock.to_account_info(),
                self.stake_history.to_account_info(),
                self.user.to_account_info(),
            ],
        )?;

        Ok(())
    }

    fn split_stake_account(&self, new_stake_account: &AccountInfo<'info>, split_amount: u64) -> Result<()> {
        invoke(
            stake::instruction::split(
                self.stake_account.key,
                self.user.key,
                split_amount,
                new_stake_account.key
            ).last().unwrap(),
            &[
                self.stake_account.to_account_info(),
                new_stake_account.clone(),
                self.user.to_account_info(),
            ],
        )?;

        Ok(())
    }
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, BorrowLiquidity<'info>>, args: BorrowLiquidityArgs) -> Result<()> {
    // Validate loan type and get current epoch
    let current_epoch = Clock::get()?.epoch;
    let loan_type = match args.loan_type {
        0 => LoanType::SHORT(current_epoch),
        1 => LoanType::MEDIUM(current_epoch),
        2 => LoanType::LONG(current_epoch),
        _ => return Err(LendingError::InvalidLoanType.into()),
    };

    let deposited_amount = args.collateral_amount;

    // Deposit collateral and update obligation
    let weighted_deposited_amount = ctx.accounts.reserve.deposit_collateral(deposited_amount)?;
    ctx.accounts.obligation.add_or_create_position(InitObligationPositionParams {
        vote_account: ctx.accounts.reserve.vote_account,
        loan_type,
        deposited_amount,
        weighted_deposited_amount,
        borrowed_amount: args.borrowed_amount,
    })?;

    // Validate stake amount
    let stake_amount = get_stake_amount(&ctx.accounts.stake_account.to_account_info())?;
    require_gte!(stake_amount, deposited_amount, LendingError::InsufficientCollateral);

    // Handle stake account operations
    if stake_amount == deposited_amount {
        ctx.accounts.merge_stake_account(deposited_amount)?;
    } else {
        require_eq!(ctx.remaining_accounts.len(), 1, LendingError::WrongRemainingAccountSchema);
        let split_stake_account = &ctx.remaining_accounts[0];
        let split_amount = stake_amount.checked_sub(deposited_amount).ok_or(LendingError::MathOverflow)?;

        ctx.accounts.split_stake_account(split_stake_account, split_amount)?;
        ctx.accounts.merge_stake_account(deposited_amount)?;
    }

    // Mark Reserve as stale
    ctx.accounts.reserve.last_update.mark_stale();

    Ok(())
}