pub use {
    anchor_lang::prelude::*,
    solana_program::{
        stake::{self, state::StakeStateV2, config::ID as STAKE_CONFIG_ID, program::ID as STAKE_PROGRAM_ID},
        program::invoke_signed,
        borsh1::try_from_slice_unchecked,
        system_instruction,
        native_token::LAMPORTS_PER_SOL,
    },
    crate::{state::{Reserve, LendingMarket}, error::LendingError, LENDING_MARKET_AUTHORITY_PREFIX, RESERVE_STAKE_PREFIX, RESERVE_PREFIX, ACTIVATING_STAKE_PREFIX, DEACTIVATING_STAKE_PREFIX, LIQUIDITY_VAULT_PREFIX},
};

#[derive(Accounts)]
pub struct RefreshEpoch<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    pub lending_market: Account<'info, LendingMarket>,
    #[account(
        mut,
        has_one = lending_market,
        seeds = [RESERVE_PREFIX, reserve.lending_market.key().as_ref(), vote_account.key().as_ref()],
        bump = reserve.bump,
    )]
    pub reserve: Account<'info, Reserve>,
    /// CHECK: todo
    pub vote_account: UncheckedAccount<'info>,
    #[account(
        seeds = [LENDING_MARKET_AUTHORITY_PREFIX, reserve.lending_market.as_ref()],
        bump = lending_market.authority_bump,
    )]
    /// CHECK: todo
    pub lending_market_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [RESERVE_STAKE_PREFIX, reserve.key().as_ref()],
        bump = reserve.stake_bump,
    )]
    /// CHECK: todo
    pub reserve_stake: UncheckedAccount<'info>,
    #[account(
        seeds = [LIQUIDITY_VAULT_PREFIX, reserve.key().as_ref()],
        bump,
    )]
    /// CHECK: todo
    pub reserve_vault: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [ACTIVATING_STAKE_PREFIX, reserve.key().as_ref(), reserve.last_epoch.to_le_bytes().as_ref()],
        bump,
    )]
    /// CHECK: todo
    pub old_activating_reserve_stake: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [ACTIVATING_STAKE_PREFIX, reserve.key().as_ref(), (reserve.last_epoch + 1).to_le_bytes().as_ref()],
        bump,
    )]
    /// CHECK: todo
    pub new_activating_reserve_stake: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [DEACTIVATING_STAKE_PREFIX, reserve.key().as_ref(), reserve.last_epoch.to_le_bytes().as_ref()],
        bump,
    )]
    /// CHECK: todo
    pub old_deactivating_reserve_stake: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [DEACTIVATING_STAKE_PREFIX, reserve.key().as_ref(), (reserve.last_epoch + 1).to_le_bytes().as_ref()],
        bump,
    )]
    /// CHECK: todo
    pub new_deactivating_reserve_stake: UncheckedAccount<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
    pub stake_history: Sysvar<'info, StakeHistory>,
    #[account(address = STAKE_CONFIG_ID)]
    /// CHECK: checked by address constraint
    pub stake_config: UncheckedAccount<'info>,
    #[account(address = STAKE_PROGRAM_ID)]
    /// CHECK: checked by address constraint
    pub stake_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>
}

impl <'info> RefreshEpoch<'info> {
    /// Get the stake amount from the stake account
    pub fn get_stake_amount(&mut self) -> Result<u64> {
        let stake_state = try_from_slice_unchecked::<StakeStateV2>(&self.reserve_stake.data.borrow())?;

        match stake_state {
            StakeStateV2::Stake(_, stake, _) => Ok(stake.delegation.stake),
            _ => Err(LendingError::WrongStakeStake.into()),
        }
    }

    /// Merge the Stake account with active lamports into the Lending Market Stake account
    pub fn merge_activating_stake_account(&self) -> Result<()> {
        let lending_market_key = self.reserve.lending_market;
        let authority_seeds = &[LENDING_MARKET_AUTHORITY_PREFIX, lending_market_key.as_ref(), &[self.lending_market.authority_bump]];
        let signers = &[&authority_seeds[..]];

        invoke_signed(
            &stake::instruction::merge(&self.reserve_stake.key(), &self.old_activating_reserve_stake.key, &self.lending_market_authority.key)[0],
            &[
                self.reserve_stake.to_account_info(),
                self.old_activating_reserve_stake.to_account_info(),
                self.clock.to_account_info(),
                self.stake_history.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
            signers,
        )?;

        Ok(())
    }

    /// 
    pub fn split_and_deactivate_amount(&self, amount: u64) -> Result<()> {
        let lending_market_key = self.reserve.lending_market;
        let authority_seeds = &[LENDING_MARKET_AUTHORITY_PREFIX, lending_market_key.as_ref(), &[self.lending_market.authority_bump]];
        let signers = &[&authority_seeds[..]];

        invoke_signed(
            stake::instruction::split(&self.reserve_stake.key(), &self.lending_market_authority.key(), amount, &self.new_deactivating_reserve_stake.key()).last().unwrap(),
            &[
                self.reserve_stake.to_account_info(), 
                self.new_deactivating_reserve_stake.to_account_info(), 
                self.lending_market_authority.to_account_info(),
            ],
            signers,
        )?;

        invoke_signed(
            &stake::instruction::deactivate_stake(&self.new_deactivating_reserve_stake.key(), &self.lending_market_authority.key()),
            &[
                self.new_deactivating_reserve_stake.to_account_info(), 
                self.clock.to_account_info(), 
                self.lending_market_authority.to_account_info(),
            ],
            signers,
        )?;
        
        Ok(())
    }

    /// 
    pub fn claim_deactivated_stake_amount(&self) -> Result<()> {
        let lending_market_key = self.reserve.lending_market;
        let authority_seeds = &[LENDING_MARKET_AUTHORITY_PREFIX, lending_market_key.as_ref(), &[self.lending_market.authority_bump]];
        let signers = &[&authority_seeds[..]];

        invoke_signed(
            &stake::instruction::withdraw(&self.old_deactivating_reserve_stake.key(), &self.lending_market_authority.key(), &self.reserve_vault.key(), self.old_deactivating_reserve_stake.lamports(), None),
            &[
                self.old_deactivating_reserve_stake.to_account_info(), 
                self.reserve_vault.to_account_info(), 
                self.clock.to_account_info(),
                self.stake_history.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
            signers,
        )?;
        
        Ok(())
    }

    /// Initialize & Delegate a new stake account from the Lending Market Stake account that has inactive lamports
    pub fn initialize_stake_account(&mut self, amount: u64, stake_space: usize, stake_bump: u8) -> Result<()> {
        let reserve_key = self.reserve.key();
        let next_epoch = (self.reserve.last_epoch + 1).to_le_bytes();
        let stake_seeds = &[ACTIVATING_STAKE_PREFIX, reserve_key.as_ref(), next_epoch.as_ref(), &[stake_bump]];
        let stake_signers = &[&stake_seeds[..]];

        let lending_market_key = self.reserve.lending_market;
        let authority_seeds = &[LENDING_MARKET_AUTHORITY_PREFIX, lending_market_key.as_ref(), &[self.lending_market.authority_bump]];
        let stake_authority_signers = &[&authority_seeds[..]];

        **self.reserve_stake.to_account_info().try_borrow_mut_lamports()? -= amount;
        **self.new_activating_reserve_stake.to_account_info().try_borrow_mut_lamports()? += amount;

        let authorized = stake::state::Authorized::auto(self.lending_market_authority.key);

        invoke_signed(
            &system_instruction::allocate(&self.new_activating_reserve_stake.key, stake_space as u64),
            &[self.new_activating_reserve_stake.to_account_info()],
            stake_signers,
        )?;

        invoke_signed(
            &system_instruction::assign(&self.new_activating_reserve_stake.key, &self.stake_program.key),
            &[self.new_activating_reserve_stake.to_account_info()],
            stake_signers,
        )?;

        invoke_signed(
            &stake::instruction::initialize_checked(&self.new_activating_reserve_stake.key, &authorized),
            &[
                self.new_activating_reserve_stake.to_account_info(),
                self.rent.to_account_info(),
                self.lending_market_authority.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
            stake_authority_signers,
        )?;

        invoke_signed(
            &stake::instruction::delegate_stake(
                self.new_activating_reserve_stake.key,
                self.lending_market_authority.key,
                self.vote_account.key,
            ),
            &[
                self.new_activating_reserve_stake.to_account_info(),
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
        // Update the Reserve Epoch
        reserve.update_epoch(ctx.accounts.reserve.last_epoch + 1);

        let minimum_delegation = std::cmp::max(
            stake::tools::get_minimum_delegation()?,
            LAMPORTS_PER_SOL,
        );
        let stake_space = std::mem::size_of::<stake::state::StakeStateV2>();
        let stake_rent_plus_initial = ctx.accounts.rent
            .minimum_balance(stake_space)
            .saturating_add(minimum_delegation);

        let stake_amount = ctx.accounts.get_stake_amount()?;

        let inactive_stake_amount = ctx.accounts.reserve_stake.lamports()
            .checked_sub(stake_amount)
            .ok_or(LendingError::MathOverflow)?;

        if  inactive_stake_amount > stake_rent_plus_initial {
            ctx.accounts.initialize_stake_account(inactive_stake_amount, stake_space, ctx.bumps.new_activating_reserve_stake)?;
        }

        if let Ok(_) = try_from_slice_unchecked::<StakeStateV2>(&ctx.accounts.old_activating_reserve_stake.data.borrow()) {
            ctx.accounts.merge_activating_stake_account()?;
        }

        if let Ok(_) = try_from_slice_unchecked::<StakeStateV2>(&ctx.accounts.old_deactivating_reserve_stake.data.borrow()) {
            ctx.accounts.claim_deactivated_stake_amount()?;
        }

        if reserve.collateral.collateral_amount_to_claim > 0 {
            ctx.accounts.split_and_deactivate_amount(reserve.collateral.collateral_amount_to_claim)?;
            reserve.collateral.collateral_amount_to_claim = 0;
        }
    }
    
    Ok(())
}
