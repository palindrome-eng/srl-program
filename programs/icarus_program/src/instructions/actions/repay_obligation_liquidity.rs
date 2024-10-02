pub use {
    anchor_lang::prelude::*,
    solana_program::{system_instruction, program::{invoke, invoke_signed}, stake::{self, program::ID as STAKE_PROGRAM_ID}},
    crate::{get_stake_amount, calculate_withdraw_amount, state::{LendingMarket, Reserve, Obligation}, error::LendingError, LENDING_MARKET_AUTHORITY_PREFIX, RESERVE_PREFIX, RESERVE_STAKE_PREFIX, OBLIGATION_PREFIX},
    anchor_spl::token::{Token, TokenAccount, mint_to, MintTo},
};

#[derive(Accounts)]
pub struct RepayLiquidity<'info> {
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
        seeds = [LENDING_MARKET_AUTHORITY_PREFIX, lending_market.key().as_ref()],
        bump = lending_market.authority_bump,
    )]
    pub lending_market_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        has_one = lending_market,
        seeds = [OBLIGATION_PREFIX, user.key().as_ref()],
        bump,
    )]
    pub obligation: Account<'info, Obligation>,
    pub clock: Sysvar<'info, Clock>,
    #[account(address = STAKE_PROGRAM_ID)]
    /// CHECK: checked by address constraint
    pub stake_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> RepayLiquidity<'info> {
    fn split_stake_account(&self, new_stake_account: &AccountInfo<'info>, split_amount: u64) -> Result<()> {
        let lending_market_key = self.reserve.lending_market;
        let authority_seeds = &[LENDING_MARKET_AUTHORITY_PREFIX, lending_market_key.as_ref(), &[self.lending_market.authority_bump]];
        let signers = &[&authority_seeds[..]];

        // Split stake account
        invoke_signed(
            stake::instruction::split(
                self.reserve_stake.key,
                self.user.key,
                split_amount,
                new_stake_account.key
            ).last().unwrap(),
            &[
                self.reserve_stake.to_account_info(),
                new_stake_account.clone(),
                self.user.to_account_info(),
            ],
            signers
        )?;

        // Authorize staker
        invoke_signed(
            &stake::instruction::authorize(
                new_stake_account.key,
                &self.lending_market_authority.key,
                &self.user.key,  
                stake::state::StakeAuthorize::Staker,
                None,
            ),
            &[
                new_stake_account.clone(),
                self.clock.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
            signers
        )?;

        invoke_signed(
            &stake::instruction::authorize(
                new_stake_account.key,
                &self.lending_market_authority.key,
                &self.user.key,  
                stake::state::StakeAuthorize::Withdrawer,
                None,
            ),
            &[
                new_stake_account.clone(),
                self.clock.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
            signers
        )?;
        
        Ok(())
    }
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, RepayLiquidity<'info>>) -> Result<()> {
    require!(!ctx.accounts.reserve.last_update.is_stale(Clock::get()?.slot)?, LendingError::ReserveStale);

    let vote_account = ctx.accounts.reserve.vote_account;
    let current_epoch = Clock::get()?.epoch;

    // Repay or liquidate
    let (is_liquidatable, deposited_amount_after_fee) = ctx.accounts.obligation.repay_or_liquidate(vote_account, current_epoch)?;
    let position = ctx.accounts.obligation.find_position(vote_account)?.0;

    if is_liquidatable {
        ctx.accounts.reserve.collateral.repay_or_liquidate(position.deposited_amount, position.deposited_amount, position.weighted_deposited_amount)?;
        return Ok(());
    }

    // Calculate fees and amounts
    let stake_amount = get_stake_amount(&ctx.accounts.reserve_stake.to_account_info())?;
    let stake_current_value = calculate_withdraw_amount(
        ctx.accounts.reserve.collateral.mint_total_supply,
        stake_amount,
        position.weighted_deposited_amount,
    )?;

    let ltv_to_max_ratio = position.get_ltv_to_max_ratio()?;

    let mut fee_to_collect = u64::try_from((stake_current_value as u128)
        .checked_sub(position.deposited_amount as u128)
        .and_then(|v| v.checked_mul(ltv_to_max_ratio as u128))
        .and_then(|v| v.checked_div(100))
        .ok_or(LendingError::MathOverflow)?
    )?;

    if position.deposited_amount != deposited_amount_after_fee {
        fee_to_collect = fee_to_collect.checked_add(
            position.deposited_amount
                .checked_sub(deposited_amount_after_fee)
                .ok_or(LendingError::MathOverflow)?
        ).ok_or(LendingError::MathOverflow)?;
    }

    ctx.accounts.reserve.collateral.repay_or_liquidate(position.deposited_amount, fee_to_collect, position.weighted_deposited_amount)?;

    // Split stake account
    require_eq!(ctx.remaining_accounts.len(), 1, LendingError::WrongRemainingAccountSchema);
    let split_stake_account = &ctx.remaining_accounts[0];
    let split_amount = stake_current_value.checked_sub(fee_to_collect).ok_or(LendingError::MathOverflow)?;
    ctx.accounts.split_stake_account(split_stake_account, split_amount)?;

    // Mark Reserve as stale
    ctx.accounts.reserve.last_update.mark_stale();

    Ok(())
}

fn calculate_fee(stake_current_value: u64, deposited_amount: u64, deposited_amount_after_fee: u64, ltv_to_max_ratio: u64) -> Result<u64> {
    let mut fee = u64::try_from((stake_current_value as u128)
        .checked_sub(deposited_amount as u128)
        .and_then(|v| v.checked_mul(ltv_to_max_ratio as u128))
        .and_then(|v| v.checked_div(100))
        .ok_or(LendingError::MathOverflow)?
    )?;

    if deposited_amount != deposited_amount_after_fee {
        fee = fee.checked_add(
            deposited_amount
                .checked_sub(deposited_amount_after_fee)
                .ok_or(LendingError::MathOverflow)?
        ).ok_or(LendingError::MathOverflow)?;
    }

    Ok(fee)
}