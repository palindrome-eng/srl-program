pub use {
    anchor_lang::prelude::*,
    crate::{state::LendingMarket, LENDING_MARKET_PREFIX},
};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug, PartialEq)]
pub struct SetLendingMarketOwnerArgs {
    pub new_owner: Pubkey,
}

#[derive(Accounts)]
pub struct SetLendingMarketOwner<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    
    #[account(
        mut,
        seeds = [LENDING_MARKET_PREFIX, lending_market.vote_account.as_ref()],
        bump = lending_market.bump_seed,
    )]
    pub lending_market: Account<'info, LendingMarket>,
}

pub fn handler<'info>(ctx: Context<SetLendingMarketOwner>, args: SetLendingMarketOwnerArgs) -> Result<()> {
    // CHECKS: todo
    
    // Initialize Lending Market State
    ctx.accounts.lending_market.owner = args.new_owner;

    Ok(())
}
