pub use {
    anchor_lang::prelude::*,
    crate::{state::{LendingMarket, InitLendingMarketParams}, LENDING_MARKET_PREFIX, LENDING_MARKET_AUTHORITY_PREFIX},
};

#[derive(Accounts)]
pub struct InitializeLendingMarket<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        space = LendingMarket::INIT_SPACE,
        seeds = [LENDING_MARKET_PREFIX],
        bump,
    )]
    pub lending_market: Account<'info, LendingMarket>,
    #[account(
        seeds = [LENDING_MARKET_AUTHORITY_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    /// CHECK: todo
    pub lending_market_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>
}

pub fn handler<'info>(ctx: Context<InitializeLendingMarket>) -> Result<()> {
    // CHECKS: todo
    
    // Initialize Lending Market State
    ctx.accounts.lending_market.init(InitLendingMarketParams {
        owner: ctx.accounts.owner.key(),
        bump: ctx.bumps.lending_market_authority,
        authority_bump: ctx.bumps.lending_market_authority,
    });

    Ok(())
}
