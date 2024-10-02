use anchor_lang::prelude::*;

pub mod state;
pub mod error;
pub mod instructions;
pub use instructions::*;

pub const LENDING_MARKET_PREFIX: &[u8] = b"lending_market";
pub const LENDING_MARKET_AUTHORITY_PREFIX: &[u8] = b"authority";
pub const LIQUIDITY_VAULT_PREFIX: &[u8] = b"liquidity_vault";

pub const RESERVE_PREFIX: &[u8] = b"reserve";
pub const RESERVE_STAKE_PREFIX: &[u8] = b"stake";
pub const ACTIVATING_STAKE_PREFIX: &[u8] = b"activating_stake";
pub const DEACTIVATING_STAKE_PREFIX: &[u8] = b"deactivating_stake";
pub const COLLATERAL_MINT_PREFIX: &[u8] = b"collateral_mint";
pub const LIQUIDITY_MINT_PREFIX: &[u8] = b"liquidity_mint";

pub const OBLIGATION_PREFIX: &[u8] = b"obligation";

pub const MINT_DECIMALS: u8 = 9;

declare_id!("6CiDLjqtdxtbqC8oympZZdqxG2niyHaAUrmawGdoV16y");

#[program]
pub mod icarus_program {
    use super::*;

    /// Setup Instructions - owner always needs to sign

    /// Initialize a new lending market
    pub fn init_lending_market(ctx: Context<InitializeLendingMarket>) -> Result<()> {
        instructions::setup::init_lending_market::handler(ctx)
    }

    /// Set a new owner of the lending market
    pub fn set_lending_market_owner(ctx: Context<SetLendingMarketOwner>, args: SetLendingMarketOwnerArgs) -> Result<()> {
        instructions::setup::set_lending_market_owner::handler(ctx, args)
    }

    /// Initialize a new reserve
    pub fn init_reserve(ctx: Context<InitializeReserve>) -> Result<()> {
        instructions::setup::init_reserve::handler(ctx)
    }

    /// Crankless Setup Instructions - anyone can sign

    /// todo

    /// Actions Instructions - user always needs to sign

    /// 
    pub fn deposit_reserve_liquidity(ctx: Context<DepositLiquidity>, args: DepositLiquidityArgs) -> Result<()> {
        instructions::actions::deposit_reserve_liquidity::handler(ctx, args)
    }

    ///
    pub fn redeem_reserve_liquidity(ctx: Context<RedeemLiquidity>, args: RedeemLiquidityArgs) -> Result<()> {
        instructions::actions::reedem_reserve_liquidity::handler(ctx, args)
    }

}