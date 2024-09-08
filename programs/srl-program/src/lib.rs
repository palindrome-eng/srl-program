use anchor_lang::prelude::*;

pub mod state;
pub mod error;
pub mod instructions;

pub const LENDING_MARKET_PREFIX: &[u8] = b"lending_market";
pub const LENDING_MARKET_STAKE_PREFIX: &[u8] = b"stake";
pub const LENDING_MARKET_AUTHORITY_PREFIX: &[u8] = b"authority";

pub const RESERVE_PREFIX: &[u8] = b"reserve";
pub const COLLATERAL_MINT_PREFIX: &[u8] = b"collateral_mint";
pub const LIQUIDITY_MINT_PREFIX: &[u8] = b"liquidity_mint";

pub const LIQUIDITY_VAULT_PREFIX: &[u8] = b"liquidity_vault";

pub const MINT_DECIMALS: u8 = 9;

declare_id!("6CiDLjqtdxtbqC8oympZZdqxG2niyHaAUrmawGdoV16y");

#[program]
pub mod srl_program {
    use super::*;

}