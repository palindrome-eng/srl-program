pub use {
    anchor_lang::prelude::*,
    crate::{state::Reserve, RESERVE_PREFIX},
};

#[derive(Accounts)]
pub struct RefreshReserve<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [RESERVE_PREFIX, reserve.lending_market.key().as_ref()],
        bump,
    )]
    pub reserve: Account<'info, Reserve>,
}

pub fn handler<'info>(ctx: Context<RefreshReserve>) -> Result<()> {
    // CHECKS: todo
    
    // Accrues Interest
    // ctx.accounts.reserve.accrue_interest(ctx.accounts.clock.epoch)?;

    // Update the Slot
    ctx.accounts.reserve.last_update.update_slot(Clock::get()?.slot);
    
    Ok(())
}
