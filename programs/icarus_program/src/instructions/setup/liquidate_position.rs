use {
    anchor_lang::prelude::*,
    crate::{
        state::{LendingMarket, Reserve, Obligation},
        error::LendingError, 
        OBLIGATION_PREFIX
    },
};

#[derive(Accounts)]
pub struct LiquidatePosition<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    pub lending_market: Account<'info, LendingMarket>,
    #[account(
        mut,
        has_one = lending_market @LendingError::LendingMarketMismatch,
        seeds = [OBLIGATION_PREFIX, obligation.owner.as_ref()],
        bump,
    )]
    pub obligation: Account<'info, Obligation>,
    pub system_program: Program<'info, System>
}

impl<'info> LiquidatePosition<'info> {
    pub fn liquidate_position(&mut self, obligation: &mut Obligation, reserve_account: &AccountInfo<'info>) -> Result<()> {

        let reserve_data = reserve_account.try_borrow_data()?;
        let mut reserve = Reserve::try_deserialize(&mut &reserve_data[..])
            .map_err(|_| LendingError::InvalidReserveAccount)?;

        let vote_account = reserve.vote_account;

        let (position, _) = obligation.find_position(vote_account)?;

        require!(
            self.obligation.repay_or_liquidate(vote_account, Clock::get()?.epoch)?.0,
            LendingError::NotLiquidatable
        );

        reserve.collateral.repay_or_liquidate(position.deposited_amount, position.deposited_amount, position.weighted_deposited_amount)?;

        Ok(())
    }           
}

pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, LiquidatePosition<'info>>) -> Result<()> {
    // CHECKS: todo

    let remaining_accounts = ctx.remaining_accounts;
    let mut obligation = ctx.accounts.obligation.clone();

    require_eq!(
        remaining_accounts.len(), 
        obligation.positions.len(), 
        LendingError::WrongRemainingAccountSchema
    );

    for reserve_account in remaining_accounts {
        ctx.accounts.liquidate_position(&mut obligation, reserve_account)?;
    }
        
    Ok(())
}