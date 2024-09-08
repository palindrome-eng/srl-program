pub use {
    anchor_lang::prelude::*,
    solana_program::{system_instruction, program::invoke},
    crate::{state::{LendingMarket, Reserve, InitReserveParams, ReserveLiquidity, NewReserveLiquidityParams}, LENDING_MARKET_PREFIX, LIQUIDITY_VAULT_PREFIX, LENDING_MARKET_AUTHORITY_PREFIX, RESERVE_PREFIX, COLLATERAL_MINT_PREFIX, LIQUIDITY_MINT_PREFIX, MINT_DECIMALS},
    anchor_spl::token::{Mint, Token, TokenAccount, mint_to, MintTo},
};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug, PartialEq)]
pub struct InitializeReserveArgs {
    liquidity_amount: u64,
    // config: ReserveConfig,
}

#[derive(Accounts)]
pub struct InitializeReserve<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        seeds = [LENDING_MARKET_PREFIX, lending_market.vote_account.as_ref()],
        bump = lending_market.bump_seed,
    )]
    pub lending_market: Account<'info, LendingMarket>,
    #[account(
        mut,
        seeds = [LIQUIDITY_VAULT_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub liquidity_vault: SystemAccount<'info>,
    #[account(
        init,
        payer = admin,
        space = Reserve::INIT_SPACE,
        seeds = [RESERVE_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub reserve: Account<'info, Reserve>,
    #[account(
        seeds = [LENDING_MARKET_AUTHORITY_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub lending_market_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        seeds = [COLLATERAL_MINT_PREFIX, lending_market.key().as_ref()],
        bump,
        mint::decimals = MINT_DECIMALS,
        mint::authority = lending_market_authority,
    )]
    pub collateral_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = admin,
        seeds = [LIQUIDITY_MINT_PREFIX, lending_market.key().as_ref()],
        bump,
        mint::decimals = MINT_DECIMALS,
        mint::authority = lending_market_authority,
    )]
    pub liquidity_mint:Account<'info, Mint>,
    #[account(
        init,
        payer = admin,
        token::mint = liquidity_mint,
        token::authority = admin,
    )]
    pub admin_liquidity_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

impl <'info> InitializeReserve<'info> {
    pub fn deposit_liquidity(&self, amount: u64) -> Result<()> {
        invoke(
            &system_instruction::transfer(self.admin.key, self.liquidity_vault.key, amount,),
            &[self.admin.to_account_info()],
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
                    to: self.admin_liquidity_token.to_account_info(),
                    authority: self.lending_market_authority.to_account_info(),
                },
                signers
            ),
            amount
        )?;

        Ok(())
    }
}

pub fn handler<'info>(ctx: Context<InitializeReserve>, args: InitializeReserveArgs) -> Result<()> {
    // CHECKS: todo
    
    // Initialize Reserve State
    ctx.accounts.reserve.init(InitReserveParams {
        current_epoch: Clock::get()?.epoch,
        current_slot: Clock::get()?.slot,
        lending_market: ctx.accounts.lending_market.key(),
        liquidity: ReserveLiquidity::new(NewReserveLiquidityParams{mint_pubkey: ctx.accounts.liquidity_mint.key(), vault_pubkey: ctx.accounts.liquidity_vault.key()}),
    });

    // Deposit
    let token_amount = ctx.accounts.reserve.deposit(args.liquidity_amount)?;
    ctx.accounts.deposit_liquidity(args.liquidity_amount)?;

    // Mint Pool Tokens
    ctx.accounts.mint_pool_tokens(token_amount, ctx.bumps.lending_market_authority)?;

    Ok(())
}
