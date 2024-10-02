pub use {
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount, mint_to, MintTo},
    solana_program::{system_instruction, program::{invoke, invoke_signed}, stake::{self, config::ID as STAKE_CONFIG_ID, program::ID as STAKE_PROGRAM_ID}, native_token::LAMPORTS_PER_SOL},
    crate::{state::{LendingMarket, Reserve, InitReserveParams, ReserveLiquidity, NewReserveLiquidityParams, NewReserveCollateralParams, ReserveCollateral}, error::LendingError, LIQUIDITY_VAULT_PREFIX, RESERVE_STAKE_PREFIX, LENDING_MARKET_AUTHORITY_PREFIX, RESERVE_PREFIX, COLLATERAL_MINT_PREFIX, LIQUIDITY_MINT_PREFIX, MINT_DECIMALS},
};

#[derive(Accounts)]
pub struct InitializeReserve<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(has_one = owner @ LendingError::OwnerMismatch)]
    pub lending_market: Account<'info, LendingMarket>,
    #[account(
        init,
        payer = owner,
        space = Reserve::INIT_SPACE,
        seeds = [RESERVE_PREFIX, lending_market.key().as_ref(), vote_account.key().as_ref()],
        bump,
    )]
    pub reserve: Account<'info, Reserve>,
    #[account(
        seeds = [LENDING_MARKET_AUTHORITY_PREFIX, lending_market.key().as_ref()],
        bump = lending_market.authority_bump,
    )]
    /// CHECK: todo
    pub lending_market_authority: UncheckedAccount<'info>,
    /// CHECK: todo
    pub vote_account: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [RESERVE_STAKE_PREFIX, reserve.key().as_ref()],
        bump,
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
        init,
        payer = owner,
        mint::decimals = MINT_DECIMALS,
        mint::authority = lending_market_authority,
    )]
    pub collateral_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        payer = owner,
        mint::decimals = MINT_DECIMALS,
        mint::authority = lending_market_authority,
    )]
    pub liquidity_mint: Box<Account<'info, Mint>>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
    pub stake_history: Sysvar<'info, StakeHistory>,
    #[account(address = STAKE_CONFIG_ID)]
    /// CHECK: checked by the address constraint
    pub stake_config: UncheckedAccount<'info>,
    #[account(address = STAKE_PROGRAM_ID)]
    /// CHECK: checked by the address constraint
    pub stake_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

impl <'info> InitializeReserve<'info> {
    pub fn initialize_stake_account(&mut self, stake_bump: u8) -> Result<()> {
        let reserve_key = self.reserve.key();
        let stake_seeds = &[RESERVE_STAKE_PREFIX, reserve_key.as_ref(), &[stake_bump]];
        let stake_signers = &[&stake_seeds[..]];

        let lending_market_key = self.lending_market.key();
        let stake_authority_seeds = &[LENDING_MARKET_AUTHORITY_PREFIX, lending_market_key.as_ref(), &[self.lending_market.authority_bump]];
        let stake_authority_signers = &[&stake_authority_seeds[..]];
            
        // Minimum delegation to create a pool:
        // We floor at 1sol to avoid over-minting tokens before the relevant feature is active
        let minimum_delegation = std::cmp::max(
            stake::tools::get_minimum_delegation()?,
            LAMPORTS_PER_SOL,
        );
        let stake_space = std::mem::size_of::<stake::state::StakeStateV2>();
        let stake_rent_plus_initial = self.rent
            .minimum_balance(stake_space)
            .saturating_add(minimum_delegation);

        invoke(
            &system_instruction::transfer(&self.owner.key, &self.reserve_stake.key, stake_rent_plus_initial),
            &[
                self.owner.to_account_info(),
                self.reserve_stake.to_account_info(),
            ],
        )?;

        let authorized = stake::state::Authorized::auto(self.lending_market_authority.key);

        invoke_signed(
            &system_instruction::allocate(&self.reserve_stake.key, stake_space as u64),
            &[self.reserve_stake.to_account_info()],
            stake_signers,
        )?;

        invoke_signed(
            &system_instruction::assign(&self.reserve_stake.key, &self.stake_program.key),
            &[self.reserve_stake.to_account_info()],
            stake_signers,
        )?;

        invoke_signed(
            &stake::instruction::initialize_checked(&self.reserve_stake.key, &authorized),
            &[
                self.reserve_stake.to_account_info(),
                self.rent.to_account_info(),
                self.lending_market_authority.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
            stake_authority_signers,
        )?;

        invoke_signed(
            &stake::instruction::delegate_stake(
                self.reserve_stake.key,
                self.lending_market_authority.key,
                self.vote_account.key,
            ),
            &[
                self.reserve_stake.to_account_info(),
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

pub fn handler<'info>(ctx: Context<InitializeReserve>) -> Result<()> {
    // CHECKS: todo
    
    // Initialize Reserve State
    ctx.accounts.reserve.init(InitReserveParams {
        current_epoch: Clock::get()?.epoch,
        current_slot: Clock::get()?.slot,
        lending_market: ctx.accounts.lending_market.key(),
        vote_account: ctx.accounts.vote_account.key(),
        liquidity: ReserveLiquidity::new(NewReserveLiquidityParams{
            mint_pubkey: ctx.accounts.liquidity_mint.key(), 
            vault_pubkey: ctx.accounts.reserve_vault.key()
        }),
        collateral: ReserveCollateral::new(NewReserveCollateralParams{
            mint_pubkey: ctx.accounts.collateral_mint.key(), 
            stake_account: ctx.accounts.reserve_stake.key()
        }),
        bump: ctx.bumps.reserve,
        stake_bump: ctx.bumps.reserve_stake,
        vault_bump: ctx.bumps.reserve_vault,
    });

    // Initialize Stake Account
    ctx.accounts.initialize_stake_account(ctx.bumps.reserve_stake)?;

    Ok(())
}
