pub use {
    anchor_lang::prelude::*,
    solana_program::{system_instruction, program::invoke},
    crate::{state::{LendingMarket, Reserve}, error::LendingError, LENDING_MARKET_PREFIX, LENDING_MARKET_AUTHORITY_PREFIX, RESERVE_PREFIX },
    anchor_spl::token::{Token, TokenAccount, mint_to, MintTo},
};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug, PartialEq)]
pub struct DepositLiquidityArgs {
    liquidity_amount: u64,
}

#[derive(Accounts)]
pub struct DepositLiquidity<'info> {
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
    #[account(mut, address = reserve.liquidity.vault_pubkey)]
    pub liquidity_vault: SystemAccount<'info>,
    #[account(
        init_if_needed,
        payer = user,
        token::mint = liquidity_mint,
        token::authority = user,
    )]
    pub user_liquidity_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

impl <'info> DepositLiquidity<'info> {
    pub fn deposit_liquidity(&self, amount: u64) -> Result<()> {
        invoke(
            &system_instruction::transfer(self.user.key, self.liquidity_vault.key, amount,),
            &[self.user.to_account_info()],
        )?;

        Ok(())
    }

    pub fn mint_pool_tokens(&self, amount: u64, bump_seed: u8) -> Result<()> {
        let lending_market_key = self.lending_market.key();
        let authority_seeds = &[LENDING_MARKET_AUTHORITY_PREFIX, lending_market_key.as_ref(), &[bump_seed]];
        let signers = &[&authority_seeds[..]];

        mint_to( 
            CpiContext::new_with_signer(
                self.token_program.to_account_info(), 
                MintTo {
                    mint: self.liquidity_mint.to_account_info(),
                    to: self.user_liquidity_token.to_account_info(),
                    authority: self.lending_market_authority.to_account_info(),
                },
                signers
            ),
            amount
        )?;

        Ok(())
    }
}

pub fn handler<'info>(ctx: Context<DepositLiquidity>, args: DepositLiquidityArgs) -> Result<()> {
    // CHECKS: todo
    require!(!ctx.accounts.reserve.last_update.is_stale(Clock::get()?.slot)?, LendingError::ReserveStale);

    // Deposit
    let token_amount = ctx.accounts.reserve.deposit(args.liquidity_amount)?;
    ctx.accounts.deposit_liquidity(args.liquidity_amount)?;

    // Mark Reserve as Stale to force refresh
    ctx.accounts.reserve.last_update.mark_stale();

    // Mint Pool Tokens
    ctx.accounts.mint_pool_tokens(token_amount, ctx.bumps.lending_market_authority)?;

    Ok(())
}
