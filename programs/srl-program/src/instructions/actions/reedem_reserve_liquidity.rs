pub use {
    anchor_lang::prelude::*,
    solana_program::{system_instruction, program::invoke_signed},
    crate::{state::{LendingMarket, Reserve}, error::LendingError, LENDING_MARKET_PREFIX, LENDING_MARKET_AUTHORITY_PREFIX, RESERVE_PREFIX, LIQUIDITY_VAULT_PREFIX },
    anchor_spl::token::{Token, TokenAccount, burn, Burn},
};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug, PartialEq)]
pub struct RedeemLiquidityArgs {
    token_amount: u64,
}

#[derive(Accounts)]
pub struct RedeemLiquidity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        seeds = [LENDING_MARKET_PREFIX, lending_market.vote_account.as_ref()],
        bump = lending_market.bump_seed,
    )]
    pub lending_market: Account<'info, LendingMarket>,
    #[account(
        mut,
        seeds = [RESERVE_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub reserve: Account<'info, Reserve>,
    #[account(
        seeds = [LENDING_MARKET_AUTHORITY_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub lending_market_authority: UncheckedAccount<'info>,
    #[account(mut, address = reserve.liquidity.mint_pubkey)]
    pub liquidity_mint: UncheckedAccount<'info>,
    #[account(
        mut, 
        seeds = [LIQUIDITY_VAULT_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub liquidity_vault: SystemAccount<'info>,
    #[account(
        mut,
        token::mint = liquidity_mint,
        token::authority = user,
    )]
    pub user_liquidity_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

impl <'info> RedeemLiquidity<'info> {
    pub fn reedem_liquidity(&self, amount: u64, bump_seed: u8) -> Result<()> {
        let lending_market_key = self.lending_market.key();
        let authority_seeds = &[LIQUIDITY_VAULT_PREFIX, lending_market_key.as_ref(), &[bump_seed]];
        let signers = &[&authority_seeds[..]]; 

        invoke_signed(
            &system_instruction::transfer(self.liquidity_vault.key, self.user.key, amount,),
            &[self.user.to_account_info()],
            signers,
        )?;

        Ok(())
    }

    pub fn burn_pool_tokens(&self, amount: u64) -> Result<()> {
        burn( 
            CpiContext::new(
                self.token_program.to_account_info(), 
                Burn {
                    mint: self.liquidity_mint.to_account_info(),
                    from: self.user_liquidity_token.to_account_info(),
                    authority: self.lending_market_authority.to_account_info(),
                }
            ),
            amount
        )?;

        Ok(())
    }
}

pub fn handler<'info>(ctx: Context<RedeemLiquidity>, args: RedeemLiquidityArgs) -> Result<()> {
    // CHECKS: todo
    require!(args.token_amount > 0, LendingError::InvalidAmount);
    require!(!ctx.accounts.reserve.last_update.is_stale(Clock::get()?.slot)?, LendingError::ReserveStale);

    // Deposit
    let liquidity_amount = ctx.accounts.reserve.reedem(args.token_amount)?;
    ctx.accounts.reedem_liquidity(liquidity_amount, ctx.bumps.liquidity_vault)?;

    // Mark Reserve as Stale to force refresh
    ctx.accounts.reserve.last_update.mark_stale();

    // Mint Pool Tokens
    ctx.accounts.burn_pool_tokens(args.token_amount)?;

    Ok(())
}
