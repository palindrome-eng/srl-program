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
    /// Bump seed for derived authority address
    pub bump_seed: u8,
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
    /// Vote Account of the validator that is running the lending market
    pub vote_account: Pubkey,
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
        self.bump_seed = params.bump_seed;
        self.owner = params.owner;
        self.vote_account = params.vote_account;
    }

    pub fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

// Space for the account with 128 bytes of padding
impl Space for LendingMarket {
    const INIT_SPACE: usize = 8 + 1 + 1 + 32 + 32 + 128;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
/// Initialize a lending market
pub struct InitLendingMarketParams {
    /// Bump seed for derived authority address
    pub bump_seed: u8,
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
    /// Vote Account where all the collateral is deposited
    pub vote_account: Pubkey, 
}