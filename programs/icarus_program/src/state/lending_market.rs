use {
    super::*,
    anchor_lang::prelude::*,
};

/// Lending market state
#[account]
#[derive(Default)]
pub struct LendingMarket {
    /// Version of lending market
    pub version: u8,
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
    /// Bump seed for Lending Market
    pub bump: u8,
    /// Bump seed for derived authority address
    pub authority_bump: u8,
}

impl LendingMarket {
    /// Create a new lending market
    pub fn new(params: InitLendingMarketParams) -> Self {
        let mut lending_market = Self::default();

        lending_market.init(params);
        lending_market
    }

    /// Initialize a lending market
    pub fn init(&mut self, params: InitLendingMarketParams) {
        self.version = PROGRAM_VERSION;
        self.owner = params.owner;
        self.bump = params.bump;
        self.authority_bump = params.authority_bump;
    }

    pub fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

// Space for the account with 128 bytes of padding
impl Space for LendingMarket {
    const INIT_SPACE: usize = 8 + 1 + 32 + 1 + 1 + 1 + 128;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
/// Initialize a lending market
pub struct InitLendingMarketParams {
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
    /// Bump seed for Lending Market
    pub bump: u8,
    /// Bump seed for derived authority address
    pub authority_bump: u8,
}