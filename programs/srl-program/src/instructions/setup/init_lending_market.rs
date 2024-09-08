pub use {
    anchor_lang::prelude::*,
    solana_program::{
        stake::{self, config::ID as STAKE_CONFIG_ID, program::ID as STAKE_PROGRAM_ID},
        native_token::LAMPORTS_PER_SOL,
        system_instruction,
        program::{invoke, invoke_signed}
    },
    crate::{state::{LendingMarket, InitLendingMarketParams}, LENDING_MARKET_PREFIX, LENDING_MARKET_STAKE_PREFIX, LENDING_MARKET_AUTHORITY_PREFIX},
};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug, PartialEq)]
pub struct InitializeLendingMarketArgs {
    pub owner: Pubkey,
}

#[derive(Accounts)]
pub struct InitializeLendingMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        init,
        payer = admin,
        space = LendingMarket::INIT_SPACE,
        seeds = [LENDING_MARKET_PREFIX, vote_account.key.as_ref()],
        bump,
    )]
    pub lending_market: Account<'info, LendingMarket>,
    #[account(
        mut,
        seeds = [LENDING_MARKET_STAKE_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub lending_market_stake: UncheckedAccount<'info>,
    #[account(
        seeds = [LENDING_MARKET_AUTHORITY_PREFIX, lending_market.key().as_ref()],
        bump,
    )]
    pub lending_market_authority: UncheckedAccount<'info>,
    pub vote_account: UncheckedAccount<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
    pub stake_history: Sysvar<'info, StakeHistory>,
    #[account(address = STAKE_CONFIG_ID)]
    pub stake_config: UncheckedAccount<'info>,

    #[account(address = STAKE_PROGRAM_ID)]
    pub stake_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>
}

impl <'info> InitializeLendingMarket<'info> {
    pub fn initialize_stake_account(&mut self, stake_bump: u8, stake_authority_bump: u8) -> Result<()> {
        let lending_market_key = self.lending_market.key();
        
        let stake_seeds = &[LENDING_MARKET_STAKE_PREFIX, lending_market_key.as_ref(), &[stake_bump]];
        let stake_signers = &[&stake_seeds[..]];

        let stake_authority_seeds = &[LENDING_MARKET_AUTHORITY_PREFIX, lending_market_key.as_ref(), &[stake_authority_bump]];
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
            &system_instruction::transfer(&self.admin.key, &self.lending_market_stake.key, stake_rent_plus_initial),
            &[
                self.admin.to_account_info(),
                self.lending_market_stake.to_account_info(),
            ],
        )?;

        let authorized = stake::state::Authorized::auto(self.lending_market_authority.key);

        invoke_signed(
            &system_instruction::allocate(&self.lending_market_stake.key, stake_space as u64),
            &[self.lending_market_stake.to_account_info()],
            stake_signers,
        )?;

        invoke_signed(
            &system_instruction::assign(&self.lending_market_stake.key, &self.stake_program.key),
            &[self.lending_market_stake.to_account_info()],
            stake_signers,
        )?;

        invoke_signed(
            &stake::instruction::initialize_checked(&self.lending_market_stake.key, &authorized),
            &[
                self.lending_market_stake.to_account_info(),
                self.rent.to_account_info(),
                self.lending_market_authority.to_account_info(),
                self.lending_market_authority.to_account_info(),
            ],
            stake_authority_signers,
        )?;

        invoke_signed(
            &stake::instruction::delegate_stake(
                self.lending_market_stake.key,
                self.lending_market_authority.key,
                self.vote_account.key,
            ),
            &[
                self.lending_market_stake.to_account_info(),
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

pub fn handler<'info>(ctx: Context<InitializeLendingMarket>, args: InitializeLendingMarketArgs) -> Result<()> {
    // CHECKS: todo
    
    // Initialize Lending Market State
    ctx.accounts.lending_market.init(InitLendingMarketParams {
        bump_seed: ctx.bumps.lending_market,
        owner: args.owner,
        vote_account: *ctx.accounts.vote_account.key,
    });

    // Initialize & Delegate Stake Account
    ctx.accounts.initialize_stake_account(ctx.bumps.lending_market_stake, ctx.bumps.lending_market_authority)?;

    Ok(())
}
