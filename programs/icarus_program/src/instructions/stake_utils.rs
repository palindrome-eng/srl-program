use {
    crate::error::LendingError, 
    anchor_lang::prelude::*, 
    solana_program::{
        borsh1::try_from_slice_unchecked, clock::Epoch, native_token::LAMPORTS_PER_SOL, stake::{self, state::{Meta, Stake, StakeStateV2}}
    }
};

/// Calculate weighted amount of token, given outstanding token supply, 
/// pool active stake, and deposit active stake
pub fn calculate_deposit_amount(
    token_supply: u64,
    pool_stake_amount: u64,
    user_stake_amount: u64,
) -> Result<u64> {
    let numerator = (user_stake_amount as u128)
        .checked_mul(token_supply as u128)
        .ok_or(LendingError::MathOverflow)?;
    let denominator = pool_stake_amount as u128;

    if pool_stake_amount == 0 || token_supply == 0 {
        Ok(user_stake_amount)
    } else {
        Ok(
            u64::try_from(numerator
                .checked_div(denominator)
                .ok_or(LendingError::MathOverflow)?
            )?
        )
    }
}

/// Calculate pool stake to return, given outstanding token supply, pool 
/// active stake, and tokens to redeem
pub fn calculate_withdraw_amount(
    token_supply: u64,
    pool_stake_amount: u64,
    user_token_amount: u64,
) -> Result<u64> {
    let numerator = (user_token_amount as u128)
        .checked_mul(pool_stake_amount as u128)
        .ok_or(LendingError::MathOverflow)?;
    let denominator = token_supply as u128;

    if numerator < denominator || denominator == 0 {
        Ok(0)
    } else {
        Ok(
            u64::try_from(numerator
                .checked_div(denominator)
                .ok_or(LendingError::MathOverflow)?
            )?
        )
    }
}

/// Deserialize the stake state from AccountInfo
pub fn is_stake_state_initialized(stake_account_info: &AccountInfo) -> Result<bool> {
    let stake_state = try_from_slice_unchecked::<StakeStateV2>(&stake_account_info.data.borrow())?;
    match stake_state {
        StakeStateV2::Stake(_, _, _) => Ok(true),
        _ => Ok(false)
    }
}

/// Deserialize the stake state from AccountInfo
pub fn get_stake_state(stake_account_info: &AccountInfo) -> Result<(Meta, Stake)> {
    let stake_state = try_from_slice_unchecked::<StakeStateV2>(&stake_account_info.data.borrow())?;

    match stake_state {
        StakeStateV2::Stake(meta, stake, _) => Ok((meta, stake)),
        _ => Err(LendingError::WrongStakeStake.into()),
    }
}

/// Deserialize the stake amount from AccountInfo
pub fn get_stake_amount(stake_account_info: &AccountInfo) -> Result<u64> {
    Ok(get_stake_state(stake_account_info)?.1.delegation.stake)
}

/// Determine if stake is active
pub fn is_stake_active_without_history(stake: &Stake, current_epoch: Epoch) -> bool {
    stake.delegation.activation_epoch < current_epoch
        && stake.delegation.deactivation_epoch == Epoch::MAX
}

/// Minimum delegation to create a pool
/// We floor at 1sol to avoid over-minting tokens before the relevant feature is
/// active
pub fn minimum_delegation() -> Result<u64> {
    Ok(std::cmp::max(
        stake::tools::get_minimum_delegation()?,
        LAMPORTS_PER_SOL,
    ))
}