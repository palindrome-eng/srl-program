pub use {
    anchor_lang::prelude::*,
    crate::{state::{LendingMarket, Obligation, InitObligationParams}, LENDING_MARKET_PREFIX, OBLIGATION_PREFIX},
};

#[derive(Accounts)]
pub struct InitializeObligation<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        seeds = [LENDING_MARKET_PREFIX],
        bump = lending_market.bump,
    )]
    pub lending_market: Account<'info, LendingMarket>,
    #[account(
        init,
        payer = owner,
        space = Obligation::INIT_SPACE,
        seeds = [OBLIGATION_PREFIX, owner.key().as_ref()],
        bump,
    )]
    pub obligation: Account<'info, Obligation>,
    pub system_program: Program<'info, System>
}

pub fn handler<'info>(ctx: Context<InitializeObligation>) -> Result<()> {
    // CHECKS: todo
    
    // Initialize Lending Market State
    ctx.accounts.obligation.init(InitObligationParams {
        owner: ctx.accounts.owner.key(),
        lending_market: ctx.accounts.lending_market.key(),
        bump: ctx.bumps.obligation
    });

    Ok(())
}
