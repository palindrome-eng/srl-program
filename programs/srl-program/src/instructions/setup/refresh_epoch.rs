pub use {
    anchor_lang::prelude::*,
    solana_program::{
        stake::{self, state::StakeStateV2, config::ID as STAKE_CONFIG_ID, program::ID as STAKE_PROGRAM_ID},
        program::invoke_signed,
        borsh1::try_from_slice_unchecked,
        system_instruction,
        native_token::LAMPORTS_PER_SOL,
    },
    crate::{state::{LendingMarket, Reserve}, error::LendingError, LENDING_MARKET_PREFIX, LENDING_MARKET_STAKE_PREFIX, LENDING_MARKET_AUTHORITY_PREFIX, RESERVE_PREFIX},
};

#[derive(Accounts)]
pub struct RefreshEpoch<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        seeds = [LENDING_MARKET_PREFIX, vote_account.key.as_ref()],
        bump = lending_market.bump_seed,
    )]
    pub lending_market: Account<'info, LendingMarket>,
    #[account(
        mut,
        seeds = [LENDING_MARKET_STAKE_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub lending_market_stake: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [LENDING_MARKET_STAKE_PREFIX, reserve.last_epoch.to_le_bytes().as_ref()],
        bump,
    )]
    pub merging_lending_market_stake: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [LENDING_MARKET_STAKE_PREFIX, (reserve.last_epoch + 1).to_le_bytes().as_ref()],
        bump,
    )]
    pub new_lending_market_stake: UncheckedAccount<'info>,
    #[account(
        seeds = [LENDING_MARKET_AUTHORITY_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub lending_market_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [RESERVE_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub reserve: Account<'info, Reserve>,
    pub vote_account: UncheckedAccount<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
    pub stake_history: Sysvar<'info, StakeHistory>,
    #[account(address = STAKE_CONFIG_ID)]
    pub stake_config: UncheckedAccount<'info>,

    #[account(address = STAKE_PROGRAM_ID)]
    pub stake_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>
}

impl <'info> RefreshEpoch<'info> {
    /// Merge the Stake account with active lamports into the Lending Market Stake account
    pub fn merge_stake_account(&self, bump_seed: u8) -> Result<()> {
        let lending_market_key = self.lending_market.key();
        let authority_seeds = &[LENDING_MARKET_AUTHORITY_PREFIX, lending_market_key.as_ref(), &[bump_seed]];
        let signers = &[&authority_seeds[..]];

        invoke_signed(
            &stake::instruction::merge(&self.lending_market_stake.key, &self.merging_lending_market_stake.key, &self.lending_market_authority.key)
                [0],
            &[
                self.lending_market_stake.to_account_info(),
                self.merging_lending_market_stake.to_account_info(),
                self.clock.to_account_info(),
                self.stake_history.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
            signers,
        )?;

        Ok(())
    }

    /// Get the stake amount from the stake account
    pub fn get_stake_amount(&mut self) -> Result<u64> {
        let stake_state = try_from_slice_unchecked::<StakeStateV2>(&self.lending_market_stake.data.borrow())?;

        match stake_state {
            StakeStateV2::Stake(_, stake, _) => Ok(stake.delegation.stake),
            _ => Err(LendingError::WrongStakeStake.into()),
        }
    }

    /// Initialize & Delegate a new stake account from the Lending Market Stake account that has inactive lamports
    pub fn initialize_stake_account(&mut self, amount: u64, stake_space: usize, stake_bump: u8, stake_authority_bump: u8) -> Result<()> {
        let next_epoch = (self.reserve.last_epoch + 1).to_le_bytes();

        let stake_seeds = &[LENDING_MARKET_STAKE_PREFIX, next_epoch.as_ref(), &[stake_bump]];
        let stake_signers = &[&stake_seeds[..]];

        let lending_market_key = self.lending_market.key();

        let stake_authority_seeds = &[LENDING_MARKET_AUTHORITY_PREFIX, lending_market_key.as_ref(), &[stake_authority_bump]];
        let stake_authority_signers = &[&stake_authority_seeds[..]];

        **self.lending_market_stake.to_account_info().try_borrow_mut_lamports()? -= amount;
        **self.new_lending_market_stake.to_account_info().try_borrow_mut_lamports()? += amount;

        let authorized = stake::state::Authorized::auto(self.lending_market_authority.key);

        invoke_signed(
            &system_instruction::allocate(&self.new_lending_market_stake.key, stake_space as u64),
            &[self.new_lending_market_stake.to_account_info()],
            stake_signers,
        )?;

        invoke_signed(
            &system_instruction::assign(&self.new_lending_market_stake.key, &self.stake_program.key),
            &[self.new_lending_market_stake.to_account_info()],
            stake_signers,
        )?;

        invoke_signed(
            &stake::instruction::initialize_checked(&self.new_lending_market_stake.key, &authorized),
            &[
                self.new_lending_market_stake.to_account_info(),
                self.rent.to_account_info(),
                self.lending_market_authority.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
            stake_authority_signers,
        )?;

        invoke_signed(
            &stake::instruction::delegate_stake(
                self.new_lending_market_stake.key,
                self.lending_market_authority.key,
                self.vote_account.key,
            ),
            &[
                self.new_lending_market_stake.to_account_info(),
                self.vote_account.to_account_info(),
                self.clock.to_account_info(),
                self.stake_history.to_account_info(),
                self.stake_config.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
            stake_authority_signers,
        )?;

        Ok(())
    }




}

pub fn handler<'info>(ctx: Context<RefreshEpoch>) -> Result<()> {
    // CHECKS: todo
    
    // Accrues Interest
    // ctx.accounts.reserve.accrue_interest(ctx.accounts.clock.epoch)?;

    // Update the Slot
    ctx.accounts.reserve.last_update.update_slot(Clock::get()?.slot);

    let mut reserve = ctx.accounts.reserve.clone();

    // Update the Epoch
    if reserve.epoch_elapsed(Clock::get()?.epoch)? != 0 {
        if reserve.epoch_elapsed(Clock::get()?.epoch)? == 1 {
            reserve.update_epoch(Clock::get()?.epoch);
        } else {
            reserve.update_epoch(ctx.accounts.reserve.last_epoch + 1);
        }

        // Minimum delegation to create a pool:
        // We floor at 1sol to avoid over-minting tokens before the relevant feature is active
        let minimum_delegation = std::cmp::max(
            stake::tools::get_minimum_delegation()?,
            LAMPORTS_PER_SOL,
        );
        let stake_space = std::mem::size_of::<stake::state::StakeStateV2>();
        let stake_rent_plus_initial = ctx.accounts.rent
            .minimum_balance(stake_space)
            .saturating_add(minimum_delegation);

        let stake_amount = ctx.accounts.get_stake_amount()?;

        if stake_amount > stake_rent_plus_initial {
            ctx.accounts.initialize_stake_account(stake_amount, stake_space, ctx.bumps.lending_market_stake, ctx.bumps.lending_market_authority)?;
        }

        if let Ok(_) = try_from_slice_unchecked::<StakeStateV2>(&ctx.accounts.merging_lending_market_stake.data.borrow()) {
            ctx.accounts.merge_stake_account(ctx.bumps.lending_market_authority)?;
        }
    }
    
    Ok(())
}
